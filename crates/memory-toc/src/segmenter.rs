//! Event segmentation engine.
//!
//! Per TOC-03: Creates segments on time threshold (30 min) or token threshold (4K).
//! Per TOC-04: Includes overlap for context continuity.

use chrono::{DateTime, Utc};
use tracing::{debug, trace};

use memory_types::{Event, EventType, Segment};

use crate::config::SegmentationConfig;

/// Token counter for events.
pub struct TokenCounter {
    /// Maximum chars for tool results
    max_tool_result_chars: usize,
}

impl TokenCounter {
    pub fn new(max_tool_result_chars: usize) -> Self {
        Self {
            max_tool_result_chars,
        }
    }

    /// Count tokens in event text.
    ///
    /// Uses tiktoken for accurate OpenAI token counting.
    /// Truncates tool results to avoid token explosion.
    pub fn count_event(&self, event: &Event) -> usize {
        let text = if event.event_type == EventType::ToolResult {
            // Truncate tool results to avoid token explosion
            let len = event.text.len().min(self.max_tool_result_chars);
            &event.text[..len]
        } else {
            &event.text
        };

        // Use tiktoken for accurate counting
        // Fall back to estimate if tiktoken unavailable
        match tiktoken_rs::cl100k_base() {
            Ok(bpe) => bpe.encode_with_special_tokens(text).len(),
            Err(_) => {
                // Rough estimate: ~4 chars per token
                (text.len() / 4).max(1)
            }
        }
    }

    /// Count tokens in multiple events.
    pub fn count_events(&self, events: &[Event]) -> usize {
        events.iter().map(|e| self.count_event(e)).sum()
    }
}

/// Builder for creating segments from a stream of events.
///
/// Detects segment boundaries based on:
/// - Time gaps (TOC-03: 30 min default)
/// - Token thresholds (TOC-03: 4K default)
///
/// Includes overlap from previous segment (TOC-04).
pub struct SegmentBuilder {
    config: SegmentationConfig,
    token_counter: TokenCounter,

    /// Events in current segment being built
    current_events: Vec<Event>,
    /// Token count of current segment
    current_tokens: usize,
    /// Time of last event
    last_event_time: Option<DateTime<Utc>>,

    /// Events to include as overlap in next segment
    overlap_buffer: Vec<Event>,
    /// Tokens in overlap buffer
    overlap_tokens: usize,
}

impl SegmentBuilder {
    /// Create a new segment builder with the given configuration.
    pub fn new(config: SegmentationConfig) -> Self {
        let token_counter = TokenCounter::new(config.max_tool_result_chars);
        Self {
            config,
            token_counter,
            current_events: Vec::new(),
            current_tokens: 0,
            last_event_time: None,
            overlap_buffer: Vec::new(),
            overlap_tokens: 0,
        }
    }

    /// Add an event to the builder.
    ///
    /// Returns Some(Segment) if a boundary was detected and segment completed.
    pub fn add_event(&mut self, event: Event) -> Option<Segment> {
        let event_tokens = self.token_counter.count_event(&event);

        trace!(
            event_id = %event.event_id,
            tokens = event_tokens,
            "Processing event"
        );

        // Check for time gap boundary
        if let Some(last_time) = self.last_event_time {
            let gap_ms = event.timestamp.timestamp_millis() - last_time.timestamp_millis();
            if gap_ms > self.config.time_threshold_ms && !self.current_events.is_empty() {
                debug!(
                    gap_ms = gap_ms,
                    threshold = self.config.time_threshold_ms,
                    "Time gap boundary detected"
                );
                let segment = self.flush_segment();
                self.add_event_internal(event, event_tokens);
                return Some(segment);
            }
        }

        // Check for token threshold boundary
        if self.current_tokens + event_tokens > self.config.token_threshold
            && !self.current_events.is_empty()
        {
            debug!(
                current_tokens = self.current_tokens,
                event_tokens = event_tokens,
                threshold = self.config.token_threshold,
                "Token threshold boundary detected"
            );
            let segment = self.flush_segment();
            self.add_event_internal(event, event_tokens);
            return Some(segment);
        }

        // No boundary, add to current segment
        self.add_event_internal(event, event_tokens);
        None
    }

    /// Internal method to add event to current segment.
    fn add_event_internal(&mut self, event: Event, event_tokens: usize) {
        self.last_event_time = Some(event.timestamp);
        self.current_events.push(event);
        self.current_tokens += event_tokens;
    }

    /// Flush current events as a completed segment.
    fn flush_segment(&mut self) -> Segment {
        let events = std::mem::take(&mut self.current_events);
        let tokens = self.current_tokens;
        self.current_tokens = 0;

        let start_time = events.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let end_time = events.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);

        // Create segment with overlap from previous
        let overlap = std::mem::take(&mut self.overlap_buffer);
        let segment_id = format!("seg:{}", ulid::Ulid::new());

        debug!(
            segment_id = %segment_id,
            events = events.len(),
            overlap = overlap.len(),
            tokens = tokens,
            "Created segment"
        );

        // Build overlap buffer for next segment
        self.build_overlap_buffer(&events);

        Segment::new(segment_id, events, start_time, end_time, tokens).with_overlap(overlap)
    }

    /// Build overlap buffer for next segment from current events.
    fn build_overlap_buffer(&mut self, events: &[Event]) {
        if events.is_empty() {
            return;
        }

        let end_time = events.last().unwrap().timestamp;
        let overlap_start_ms = end_time.timestamp_millis() - self.config.overlap_time_ms;

        let mut overlap_events = Vec::new();
        let mut overlap_tokens = 0;

        // Collect events within overlap time window, up to token limit
        for event in events.iter().rev() {
            if event.timestamp.timestamp_millis() < overlap_start_ms {
                break;
            }

            let tokens = self.token_counter.count_event(event);
            if overlap_tokens + tokens > self.config.overlap_tokens {
                break;
            }

            overlap_events.push(event.clone());
            overlap_tokens += tokens;
        }

        // Reverse to maintain chronological order
        overlap_events.reverse();

        self.overlap_buffer = overlap_events;
        self.overlap_tokens = overlap_tokens;

        trace!(
            overlap_events = self.overlap_buffer.len(),
            overlap_tokens = self.overlap_tokens,
            "Built overlap buffer"
        );
    }

    /// Flush any remaining events as a final segment.
    ///
    /// Call this when processing is complete to get any remaining events.
    pub fn flush(&mut self) -> Option<Segment> {
        if self.current_events.is_empty() {
            return None;
        }
        Some(self.flush_segment())
    }

    /// Check if builder has pending events.
    pub fn has_pending(&self) -> bool {
        !self.current_events.is_empty()
    }

    /// Get current token count.
    pub fn current_token_count(&self) -> usize {
        self.current_tokens
    }

    /// Get current event count.
    pub fn current_event_count(&self) -> usize {
        self.current_events.len()
    }
}

/// Process a batch of events into segments.
pub fn segment_events(events: Vec<Event>, config: SegmentationConfig) -> Vec<Segment> {
    let mut builder = SegmentBuilder::new(config);
    let mut segments = Vec::new();

    for event in events {
        if let Some(segment) = builder.add_event(event) {
            segments.push(segment);
        }
    }

    // Flush any remaining events
    if let Some(segment) = builder.flush() {
        segments.push(segment);
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use memory_types::{EventRole, EventType};

    fn create_event_at(text: &str, timestamp_ms: i64) -> Event {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        Event::new(
            ulid.to_string(),
            "session-123".to_string(),
            Utc.timestamp_millis_opt(timestamp_ms).unwrap(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[test]
    fn test_token_counter_basic() {
        let counter = TokenCounter::new(1000);
        let event = create_event_at("Hello, world!", 1000);
        let tokens = counter.count_event(&event);
        assert!(tokens > 0);
        assert!(tokens < 10); // "Hello, world!" should be ~4 tokens
    }

    #[test]
    fn test_token_counter_truncates_tool_results() {
        let counter = TokenCounter::new(100);
        let mut event = create_event_at(&"x".repeat(1000), 1000);
        event.event_type = EventType::ToolResult;

        let tokens = counter.count_event(&event);
        // Should be based on truncated text (100 chars), not full 1000
        assert!(tokens < 50);
    }

    #[test]
    fn test_segment_builder_time_boundary() {
        let config = SegmentationConfig {
            time_threshold_ms: 1000, // 1 second for testing
            token_threshold: 10000,
            overlap_time_ms: 500,
            overlap_tokens: 100,
            max_tool_result_chars: 1000,
        };

        let mut builder = SegmentBuilder::new(config);

        // Events within 1 second - no boundary
        assert!(builder.add_event(create_event_at("First", 1000)).is_none());
        assert!(builder.add_event(create_event_at("Second", 1500)).is_none());

        // Event after 2 second gap - boundary
        let segment = builder.add_event(create_event_at("After gap", 4000));
        assert!(segment.is_some());

        let seg = segment.unwrap();
        assert_eq!(seg.events.len(), 2);
    }

    #[test]
    fn test_segment_builder_token_boundary() {
        let config = SegmentationConfig {
            time_threshold_ms: 1000000, // Very high to not trigger
            token_threshold: 10,        // Very low to trigger
            overlap_time_ms: 500,
            overlap_tokens: 5,
            max_tool_result_chars: 1000,
        };

        let mut builder = SegmentBuilder::new(config);

        // First event
        assert!(builder.add_event(create_event_at("Short", 1000)).is_none());

        // Long event should trigger boundary
        let segment = builder.add_event(create_event_at(
            "This is a much longer message that should exceed the token threshold",
            2000,
        ));
        assert!(segment.is_some());
    }

    #[test]
    fn test_segment_builder_overlap() {
        let config = SegmentationConfig {
            time_threshold_ms: 1000,
            token_threshold: 10000,
            overlap_time_ms: 500,
            overlap_tokens: 1000,
            max_tool_result_chars: 1000,
        };

        let mut builder = SegmentBuilder::new(config);

        // Add events
        builder.add_event(create_event_at("Early", 1000));
        builder.add_event(create_event_at("Middle", 1200));
        builder.add_event(create_event_at("Late", 1400));

        // Trigger boundary
        let segment1 = builder
            .add_event(create_event_at("After gap", 5000))
            .unwrap();
        assert_eq!(segment1.events.len(), 3);

        // Add more events and flush
        builder.add_event(create_event_at("New event", 5500));
        let segment2 = builder.flush().unwrap();

        // Second segment should have overlap from first
        assert!(!segment2.overlap_events.is_empty());
    }

    #[test]
    fn test_segment_events_batch() {
        let config = SegmentationConfig {
            time_threshold_ms: 1000,
            token_threshold: 10000,
            overlap_time_ms: 100,
            overlap_tokens: 50,
            max_tool_result_chars: 1000,
        };

        let events = vec![
            create_event_at("Event 1", 1000),
            create_event_at("Event 2", 1500),
            create_event_at("Event 3", 5000), // Gap
            create_event_at("Event 4", 5500),
        ];

        let segments = segment_events(events, config);
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_flush_empty_builder() {
        let mut builder = SegmentBuilder::new(SegmentationConfig::default());
        assert!(builder.flush().is_none());
    }
}
