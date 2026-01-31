//! Grip extraction from events.
//!
//! Per SUMM-03: Extracts key excerpts and creates grips during summarization.

use memory_types::{Event, Grip};

use crate::grip_id::generate_grip_id;

/// Configuration for grip extraction.
#[derive(Debug, Clone)]
pub struct GripExtractorConfig {
    /// Maximum excerpt length in characters
    pub max_excerpt_length: usize,
    /// Minimum text length to consider for extraction
    pub min_text_length: usize,
}

impl Default for GripExtractorConfig {
    fn default() -> Self {
        Self {
            max_excerpt_length: 200,
            min_text_length: 20,
        }
    }
}

/// Extracted grip with bullet association.
#[derive(Debug, Clone)]
pub struct ExtractedGrip {
    /// The grip
    pub grip: Grip,
    /// Index of the bullet this grip supports (if known)
    pub bullet_index: Option<usize>,
}

/// Extracts grips from events based on bullet points.
pub struct GripExtractor {
    config: GripExtractorConfig,
}

impl GripExtractor {
    /// Create a new grip extractor with default config.
    pub fn new() -> Self {
        Self {
            config: GripExtractorConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: GripExtractorConfig) -> Self {
        Self { config }
    }

    /// Extract grips from events based on bullet points.
    ///
    /// For each bullet, finds events that best support it and creates a grip.
    pub fn extract_grips(
        &self,
        events: &[Event],
        bullets: &[String],
        source: &str,
    ) -> Vec<ExtractedGrip> {
        if events.is_empty() || bullets.is_empty() {
            return Vec::new();
        }

        let mut grips = Vec::new();

        for (bullet_idx, bullet) in bullets.iter().enumerate() {
            if let Some(grip) = self.find_best_match(events, bullet, source) {
                grips.push(ExtractedGrip {
                    grip,
                    bullet_index: Some(bullet_idx),
                });
            }
        }

        grips
    }

    /// Find the best matching events for a bullet point.
    fn find_best_match(&self, events: &[Event], bullet: &str, source: &str) -> Option<Grip> {
        // Extract key terms from bullet
        let key_terms: Vec<&str> = bullet
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        if key_terms.is_empty() {
            return None;
        }

        // Score each event based on term overlap
        let mut best_match: Option<(usize, usize, f32)> = None; // (start_idx, end_idx, score)

        for (idx, event) in events.iter().enumerate() {
            if event.text.len() < self.config.min_text_length {
                continue;
            }

            let text_lower = event.text.to_lowercase();
            let score: f32 = key_terms
                .iter()
                .filter(|term| text_lower.contains(&term.to_lowercase()))
                .count() as f32
                / key_terms.len() as f32;

            if score > 0.3 {
                // At least 30% term match
                match &best_match {
                    Some((start, _, best_score)) if score > *best_score => {
                        best_match = Some((*start, idx, score));
                    }
                    Some((start, end, best_score)) if score >= *best_score * 0.8 => {
                        // Extend range for similar scores
                        best_match = Some((*start, idx.max(*end), best_score.max(score)));
                    }
                    None => {
                        best_match = Some((idx, idx, score));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(start_idx, end_idx, _)| {
            let start_event = &events[start_idx];
            let end_event = &events[end_idx];

            // Create excerpt from the matching event(s)
            let excerpt = self.create_excerpt(&events[start_idx..=end_idx]);

            Grip::new(
                generate_grip_id(start_event.timestamp),
                excerpt,
                start_event.event_id.clone(),
                end_event.event_id.clone(),
                start_event.timestamp,
                source.to_string(),
            )
        })
    }

    /// Create an excerpt from a range of events.
    fn create_excerpt(&self, events: &[Event]) -> String {
        let combined: String = events
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if combined.len() <= self.config.max_excerpt_length {
            combined
        } else {
            format!("{}...", &combined[..self.config.max_excerpt_length - 3])
        }
    }
}

impl Default for GripExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to extract grips from events.
pub fn extract_grips(
    events: &[Event],
    bullets: &[String],
    source: &str,
) -> Vec<ExtractedGrip> {
    GripExtractor::new().extract_grips(events, bullets, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_types::{EventRole, EventType};

    fn create_test_event(text: &str, timestamp_ms: i64) -> Event {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        Event::new(
            ulid.to_string(),
            "session-123".to_string(),
            chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[test]
    fn test_extract_grips_basic() {
        let events = vec![
            create_test_event("How do I implement authentication?", 1706540400000),
            create_test_event("You can use JWT tokens for stateless authentication", 1706540500000),
            create_test_event("That sounds good, let me try it", 1706540600000),
        ];

        let bullets = vec![
            "Discussed JWT authentication implementation".to_string(),
        ];

        let grips = extract_grips(&events, &bullets, "test");

        assert_eq!(grips.len(), 1);
        assert_eq!(grips[0].bullet_index, Some(0));
        assert!(grips[0].grip.excerpt.contains("authentication"));
    }

    #[test]
    fn test_extract_grips_empty_events() {
        let bullets = vec!["Some bullet".to_string()];
        let grips = extract_grips(&[], &bullets, "test");
        assert!(grips.is_empty());
    }

    #[test]
    fn test_extract_grips_empty_bullets() {
        let events = vec![create_test_event("Some text", 1706540400000)];
        let grips = extract_grips(&events, &[], "test");
        assert!(grips.is_empty());
    }

    #[test]
    fn test_excerpt_truncation() {
        let extractor = GripExtractor::with_config(GripExtractorConfig {
            max_excerpt_length: 50,
            min_text_length: 10,
        });

        let events = vec![
            create_test_event("This is a very long text that should be truncated when creating an excerpt because it exceeds the maximum length", 1706540400000),
        ];

        let bullets = vec!["Long text truncation".to_string()];
        let grips = extractor.extract_grips(&events, &bullets, "test");

        assert_eq!(grips.len(), 1);
        assert!(grips[0].grip.excerpt.len() <= 50);
        assert!(grips[0].grip.excerpt.ends_with("..."));
    }
}
