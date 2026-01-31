//! Grip expansion for context retrieval.
//!
//! Per GRIP-04: ExpandGrip returns context events around excerpt.

use std::sync::Arc;
use chrono::Duration;
use tracing::debug;

use memory_storage::Storage;
use memory_types::{Event, Grip};

/// Configuration for grip expansion.
#[derive(Debug, Clone)]
pub struct ExpandConfig {
    /// Number of events to include before the excerpt range
    pub events_before: usize,
    /// Number of events to include after the excerpt range
    pub events_after: usize,
    /// Maximum time window before excerpt (in minutes)
    pub max_time_before_mins: i64,
    /// Maximum time window after excerpt (in minutes)
    pub max_time_after_mins: i64,
}

impl Default for ExpandConfig {
    fn default() -> Self {
        Self {
            events_before: 3,
            events_after: 3,
            max_time_before_mins: 30,
            max_time_after_mins: 30,
        }
    }
}

/// Result of grip expansion.
#[derive(Debug, Clone)]
pub struct ExpandedGrip {
    /// The original grip
    pub grip: Grip,
    /// Events before the excerpt range
    pub events_before: Vec<Event>,
    /// Events in the excerpt range (start to end)
    pub excerpt_events: Vec<Event>,
    /// Events after the excerpt range
    pub events_after: Vec<Event>,
}

impl ExpandedGrip {
    /// Get all events in order.
    pub fn all_events(&self) -> Vec<&Event> {
        self.events_before
            .iter()
            .chain(self.excerpt_events.iter())
            .chain(self.events_after.iter())
            .collect()
    }

    /// Get the total event count.
    pub fn total_events(&self) -> usize {
        self.events_before.len() + self.excerpt_events.len() + self.events_after.len()
    }
}

/// Error type for grip expansion.
#[derive(Debug, thiserror::Error)]
pub enum ExpandError {
    #[error("Storage error: {0}")]
    Storage(#[from] memory_storage::StorageError),

    #[error("Grip not found: {0}")]
    GripNotFound(String),

    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

/// Expander for retrieving context around grips.
pub struct GripExpander {
    storage: Arc<Storage>,
    config: ExpandConfig,
}

impl GripExpander {
    /// Create a new grip expander.
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            config: ExpandConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(storage: Arc<Storage>, config: ExpandConfig) -> Self {
        Self { storage, config }
    }

    /// Expand a grip by ID, retrieving context events.
    pub fn expand(&self, grip_id: &str) -> Result<ExpandedGrip, ExpandError> {
        // Get the grip
        let grip = self.storage.get_grip(grip_id)?
            .ok_or_else(|| ExpandError::GripNotFound(grip_id.to_string()))?;

        self.expand_grip(&grip)
    }

    /// Expand a grip, retrieving context events.
    pub fn expand_grip(&self, grip: &Grip) -> Result<ExpandedGrip, ExpandError> {
        debug!(
            grip_id = %grip.grip_id,
            event_start = %grip.event_id_start,
            event_end = %grip.event_id_end,
            "Expanding grip"
        );

        // Parse timestamps from event IDs (ULIDs contain timestamp)
        let start_ts = parse_ulid_timestamp(&grip.event_id_start)
            .ok_or_else(|| ExpandError::EventNotFound(grip.event_id_start.clone()))?;
        let end_ts = parse_ulid_timestamp(&grip.event_id_end)
            .ok_or_else(|| ExpandError::EventNotFound(grip.event_id_end.clone()))?;

        // Calculate time range for context
        let context_start = start_ts - Duration::minutes(self.config.max_time_before_mins);
        let context_end = end_ts + Duration::minutes(self.config.max_time_after_mins);

        // Get all events in the extended range
        let all_events = self.storage.get_events_in_range(
            context_start.timestamp_millis(),
            context_end.timestamp_millis(),
        )?;

        // Deserialize and partition events
        let mut events_before = Vec::new();
        let mut excerpt_events = Vec::new();
        let mut events_after = Vec::new();

        for (_key, bytes) in all_events {
            let event: Event = serde_json::from_slice(&bytes)
                .map_err(|e| ExpandError::Deserialization(e.to_string()))?;

            if event.timestamp < start_ts {
                events_before.push(event);
            } else if event.timestamp <= end_ts {
                excerpt_events.push(event);
            } else {
                events_after.push(event);
            }
        }

        // Limit context events
        let events_before: Vec<_> = events_before
            .into_iter()
            .rev()
            .take(self.config.events_before)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let events_after: Vec<_> = events_after
            .into_iter()
            .take(self.config.events_after)
            .collect();

        debug!(
            grip_id = %grip.grip_id,
            before = events_before.len(),
            excerpt = excerpt_events.len(),
            after = events_after.len(),
            "Expanded grip"
        );

        Ok(ExpandedGrip {
            grip: grip.clone(),
            events_before,
            excerpt_events,
            events_after,
        })
    }
}

/// Parse timestamp from ULID event ID.
fn parse_ulid_timestamp(event_id: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    ulid::Ulid::from_string(event_id)
        .ok()
        .and_then(|u| {
            let ms = u.timestamp_ms();
            chrono::DateTime::from_timestamp_millis(ms as i64)
        })
}

/// Convenience function to expand a grip.
pub fn expand_grip(
    storage: Arc<Storage>,
    grip_id: &str,
) -> Result<ExpandedGrip, ExpandError> {
    GripExpander::new(storage).expand(grip_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_types::{EventRole, EventType};
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        (storage, temp_dir)
    }

    fn create_and_store_event(storage: &Storage, text: &str, timestamp_ms: i64) -> Event {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        let event = Event::new(
            ulid.to_string(),
            "session-123".to_string(),
            chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        );

        let event_bytes = serde_json::to_vec(&event).unwrap();
        let outbox_bytes = b"outbox";
        storage.put_event(&event.event_id, &event_bytes, outbox_bytes).unwrap();

        event
    }

    #[test]
    fn test_expand_grip_basic() {
        let (storage, _temp) = create_test_storage();

        // Create events
        let _event1 = create_and_store_event(&storage, "Context before", 1706540300000);
        let event2 = create_and_store_event(&storage, "Start of excerpt", 1706540400000);
        let event3 = create_and_store_event(&storage, "End of excerpt", 1706540500000);
        let _event4 = create_and_store_event(&storage, "Context after", 1706540600000);

        // Create and store grip
        let grip = Grip::new(
            "grip:1706540400000:test".to_string(),
            "Start of excerpt".to_string(),
            event2.event_id.clone(),
            event3.event_id.clone(),
            event2.timestamp,
            "test".to_string(),
        );
        storage.put_grip(&grip).unwrap();

        // Expand grip
        let expander = GripExpander::new(storage);
        let expanded = expander.expand(&grip.grip_id).unwrap();

        assert_eq!(expanded.excerpt_events.len(), 2);
        assert!(expanded.events_before.len() >= 1);
        assert!(expanded.events_after.len() >= 1);
    }

    #[test]
    fn test_expand_grip_not_found() {
        let (storage, _temp) = create_test_storage();

        let expander = GripExpander::new(storage);
        let result = expander.expand("grip:nonexistent");

        assert!(matches!(result, Err(ExpandError::GripNotFound(_))));
    }

    #[test]
    fn test_expanded_grip_all_events() {
        let (storage, _temp) = create_test_storage();

        let _event1 = create_and_store_event(&storage, "Before", 1706540300000);
        let event2 = create_and_store_event(&storage, "Excerpt", 1706540400000);
        let _event3 = create_and_store_event(&storage, "After", 1706540500000);

        let grip = Grip::new(
            "grip:1706540400000:test2".to_string(),
            "Excerpt".to_string(),
            event2.event_id.clone(),
            event2.event_id.clone(),
            event2.timestamp,
            "test".to_string(),
        );
        storage.put_grip(&grip).unwrap();

        let expander = GripExpander::new(storage);
        let expanded = expander.expand(&grip.grip_id).unwrap();

        let all = expanded.all_events();
        assert!(all.len() >= 1); // At least the excerpt event
    }

    #[test]
    fn test_expand_config_limits() {
        let (storage, _temp) = create_test_storage();

        // Create many events
        for i in 0..10 {
            create_and_store_event(&storage, &format!("Event {}", i), 1706540000000 + i * 100000);
        }

        let event = create_and_store_event(&storage, "Target", 1706540500000);

        let grip = Grip::new(
            "grip:test".to_string(),
            "Target".to_string(),
            event.event_id.clone(),
            event.event_id.clone(),
            event.timestamp,
            "test".to_string(),
        );
        storage.put_grip(&grip).unwrap();

        let config = ExpandConfig {
            events_before: 2,
            events_after: 2,
            ..Default::default()
        };

        let expander = GripExpander::with_config(storage, config);
        let expanded = expander.expand(&grip.grip_id).unwrap();

        assert!(expanded.events_before.len() <= 2);
        assert!(expanded.events_after.len() <= 2);
    }
}
