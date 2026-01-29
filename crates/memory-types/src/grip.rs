//! Grip type for provenance anchoring.
//!
//! Grips link TOC summaries to source events, providing evidence
//! for claims made in bullet points.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A grip anchors a summary excerpt to source events.
///
/// Per GRIP-01: Contains excerpt, event_id_start, event_id_end, timestamp, source.
/// Per GRIP-02: TOC node bullets link to supporting grips.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grip {
    /// Unique identifier for this grip
    pub grip_id: String,

    /// The excerpt text that this grip anchors
    pub excerpt: String,

    /// First event in the range that supports this excerpt
    pub event_id_start: String,

    /// Last event in the range that supports this excerpt
    pub event_id_end: String,

    /// Timestamp of the excerpt (typically the start event's timestamp)
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,

    /// Source context (e.g., which summarization produced this)
    pub source: String,

    /// Optional: The TOC node ID that uses this grip
    #[serde(default)]
    pub toc_node_id: Option<String>,
}

impl Grip {
    /// Create a new grip
    pub fn new(
        grip_id: String,
        excerpt: String,
        event_id_start: String,
        event_id_end: String,
        timestamp: DateTime<Utc>,
        source: String,
    ) -> Self {
        Self {
            grip_id,
            excerpt,
            event_id_start,
            event_id_end,
            timestamp,
            source,
            toc_node_id: None,
        }
    }

    /// Link this grip to a TOC node
    pub fn with_toc_node(mut self, toc_node_id: String) -> Self {
        self.toc_node_id = Some(toc_node_id);
        self
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

    #[test]
    fn test_grip_serialization() {
        let grip = Grip::new(
            "grip-123".to_string(),
            "User asked about Rust memory safety".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "segment_summarizer".to_string(),
        ).with_toc_node("toc-day-20240115".to_string());

        let bytes = grip.to_bytes().unwrap();
        let decoded = Grip::from_bytes(&bytes).unwrap();

        assert_eq!(grip.grip_id, decoded.grip_id);
        assert_eq!(grip.excerpt, decoded.excerpt);
        assert_eq!(grip.toc_node_id, decoded.toc_node_id);
    }
}
