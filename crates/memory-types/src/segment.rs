//! Segment type for conversation segmentation.
//!
//! Segments group related events for summarization.
//! Per TOC-03: Created on time threshold (30 min) or token threshold (4K).
//! Per TOC-04: Include overlap for context continuity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Event;

/// A segment of conversation events.
///
/// Segments are the leaf nodes of the TOC hierarchy, containing
/// actual events that will be summarized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Unique segment identifier
    pub segment_id: String,

    /// Events in the overlap window (from previous segment for context)
    /// Per TOC-04: Provides context continuity
    #[serde(default)]
    pub overlap_events: Vec<Event>,

    /// Events in this segment (excluding overlap)
    pub events: Vec<Event>,

    /// Start time of the segment (first event, excluding overlap)
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub start_time: DateTime<Utc>,

    /// End time of the segment (last event)
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub end_time: DateTime<Utc>,

    /// Token count of events (excluding overlap)
    pub token_count: usize,
}

impl Segment {
    /// Create a new segment
    pub fn new(
        segment_id: String,
        events: Vec<Event>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        token_count: usize,
    ) -> Self {
        Self {
            segment_id,
            overlap_events: Vec::new(),
            events,
            start_time,
            end_time,
            token_count,
        }
    }

    /// Add overlap events from previous segment
    pub fn with_overlap(mut self, overlap_events: Vec<Event>) -> Self {
        self.overlap_events = overlap_events;
        self
    }

    /// Get all events (overlap + main) for summarization
    pub fn all_events(&self) -> Vec<&Event> {
        self.overlap_events
            .iter()
            .chain(self.events.iter())
            .collect()
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventRole, EventType};

    fn create_test_event(text: &str) -> Event {
        Event::new(
            ulid::Ulid::new().to_string(),
            "session-123".to_string(),
            Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[test]
    fn test_segment_creation() {
        let events = vec![create_test_event("Hello"), create_test_event("World")];
        let start = events[0].timestamp;
        let end = events[1].timestamp;

        let segment = Segment::new("seg-123".to_string(), events.clone(), start, end, 100);

        assert_eq!(segment.events.len(), 2);
        assert_eq!(segment.token_count, 100);
    }

    #[test]
    fn test_segment_with_overlap() {
        let overlap = vec![create_test_event("Context")];
        let events = vec![create_test_event("Main")];
        let start = events[0].timestamp;
        let end = events[0].timestamp;

        let segment =
            Segment::new("seg-123".to_string(), events, start, end, 50).with_overlap(overlap);

        assert_eq!(segment.overlap_events.len(), 1);
        assert_eq!(segment.all_events().len(), 2);
    }

    #[test]
    fn test_segment_serialization() {
        let events = vec![create_test_event("Test")];
        let start = events[0].timestamp;

        let segment = Segment::new("seg-123".to_string(), events, start, start, 25);
        let bytes = segment.to_bytes().unwrap();
        let decoded = Segment::from_bytes(&bytes).unwrap();

        assert_eq!(segment.segment_id, decoded.segment_id);
        assert_eq!(segment.token_count, decoded.token_count);
    }
}
