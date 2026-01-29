//! Outbox entry type for async index updates.
//!
//! Per ING-05: Outbox entries are written atomically with events.
//! Background workers consume outbox entries to update indexes.

use serde::{Deserialize, Serialize};

/// Type of outbox action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxAction {
    /// Index this event for BM25/vector search
    IndexEvent,
    /// Update TOC node with new event
    UpdateToc,
}

/// An outbox entry for async processing.
///
/// Written atomically with events to ensure index updates are not lost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    /// Reference to the event that triggered this entry
    pub event_id: String,

    /// Timestamp of the source event (for ordering)
    pub timestamp_ms: i64,

    /// What action should be performed
    pub action: OutboxAction,
}

impl OutboxEntry {
    /// Create a new outbox entry for event indexing
    pub fn for_index(event_id: String, timestamp_ms: i64) -> Self {
        Self {
            event_id,
            timestamp_ms,
            action: OutboxAction::IndexEvent,
        }
    }

    /// Create a new outbox entry for TOC update
    pub fn for_toc(event_id: String, timestamp_ms: i64) -> Self {
        Self {
            event_id,
            timestamp_ms,
            action: OutboxAction::UpdateToc,
        }
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
    fn test_outbox_entry_roundtrip() {
        let entry = OutboxEntry::for_index("event-123".to_string(), 1706540400000);
        let bytes = entry.to_bytes().unwrap();
        let decoded = OutboxEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.event_id, decoded.event_id);
        assert_eq!(entry.timestamp_ms, decoded.timestamp_ms);
        assert_eq!(entry.action, decoded.action);
    }
}
