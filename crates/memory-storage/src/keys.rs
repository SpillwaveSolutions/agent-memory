//! Key encoding and decoding for storage layer.
//!
//! Key format: `{prefix}:{timestamp_ms}:{ulid}`
//! - prefix: identifies the key type (evt, outbox, etc.)
//! - timestamp_ms: milliseconds since Unix epoch, zero-padded to 13 digits
//! - ulid: 26-character ULID for uniqueness within same millisecond
//!
//! This format enables efficient time-range scans via RocksDB prefix iteration.

use ulid::Ulid;
use crate::error::StorageError;

/// Key for event storage
/// Format: evt:{timestamp_ms:013}:{ulid}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventKey {
    /// Source timestamp in milliseconds
    pub timestamp_ms: i64,
    /// Unique identifier (also serves as event_id)
    pub ulid: Ulid,
}

impl EventKey {
    /// Create a new event key with given timestamp and fresh ULID
    pub fn new(timestamp_ms: i64) -> Self {
        Self {
            timestamp_ms,
            ulid: Ulid::new(),
        }
    }

    /// Create an event key from existing timestamp and ULID
    pub fn from_parts(timestamp_ms: i64, ulid: Ulid) -> Self {
        Self { timestamp_ms, ulid }
    }

    /// Create an event key from an event_id string (the ULID portion)
    /// Uses the ULID's embedded timestamp
    pub fn from_event_id(event_id: &str) -> Result<Self, StorageError> {
        let ulid: Ulid = event_id.parse()
            .map_err(|e| StorageError::Key(format!("Invalid event_id ULID: {}", e)))?;
        // ULID contains timestamp - extract it
        let timestamp_ms = ulid.timestamp_ms() as i64;
        Ok(Self { timestamp_ms, ulid })
    }

    /// Encode key to bytes for storage
    /// Format: "evt:{timestamp_ms:013}:{ulid}"
    pub fn to_bytes(&self) -> Vec<u8> {
        // Zero-pad timestamp to 13 digits for lexicographic sorting
        format!("evt:{:013}:{}", self.timestamp_ms, self.ulid).into_bytes()
    }

    /// Decode key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StorageError> {
        let s = std::str::from_utf8(bytes)
            .map_err(|e| StorageError::Key(format!("Invalid UTF-8: {}", e)))?;
        Self::from_str(s)
    }

    /// Parse from string format
    pub fn from_str(s: &str) -> Result<Self, StorageError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 || parts[0] != "evt" {
            return Err(StorageError::Key(format!("Invalid event key format: {}", s)));
        }

        let timestamp_ms: i64 = parts[1].parse()
            .map_err(|e| StorageError::Key(format!("Invalid timestamp: {}", e)))?;
        let ulid: Ulid = parts[2].parse()
            .map_err(|e| StorageError::Key(format!("Invalid ULID: {}", e)))?;

        Ok(Self { timestamp_ms, ulid })
    }

    /// Get the event_id (ULID string) for this key
    pub fn event_id(&self) -> String {
        self.ulid.to_string()
    }

    /// Generate prefix for time range scan start
    pub fn prefix_start(start_ms: i64) -> Vec<u8> {
        format!("evt:{:013}:", start_ms).into_bytes()
    }

    /// Generate prefix for time range scan end (exclusive)
    pub fn prefix_end(end_ms: i64) -> Vec<u8> {
        format!("evt:{:013}:", end_ms).into_bytes()
    }
}

/// Key for outbox entries (async index updates)
/// Format: outbox:{sequence:020}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxKey {
    /// Monotonic sequence number
    pub sequence: u64,
}

impl OutboxKey {
    /// Create a new outbox key with given sequence
    pub fn new(sequence: u64) -> Self {
        Self { sequence }
    }

    /// Encode key to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        format!("outbox:{:020}", self.sequence).into_bytes()
    }

    /// Decode key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StorageError> {
        let s = std::str::from_utf8(bytes)
            .map_err(|e| StorageError::Key(format!("Invalid UTF-8: {}", e)))?;

        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 || parts[0] != "outbox" {
            return Err(StorageError::Key(format!("Invalid outbox key format: {}", s)));
        }

        let sequence: u64 = parts[1].parse()
            .map_err(|e| StorageError::Key(format!("Invalid sequence: {}", e)))?;

        Ok(Self { sequence })
    }
}

/// Key for checkpoint entries
/// Format: checkpoint:{job_name}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointKey {
    /// Job name (e.g., "segmenter", "day_rollup")
    pub job_name: String,
}

impl CheckpointKey {
    pub fn new(job_name: impl Into<String>) -> Self {
        Self { job_name: job_name.into() }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        format!("checkpoint:{}", self.job_name).into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_key_roundtrip() {
        let key = EventKey::new(1706540400000);
        let bytes = key.to_bytes();
        let decoded = EventKey::from_bytes(&bytes).unwrap();
        assert_eq!(key.timestamp_ms, decoded.timestamp_ms);
        assert_eq!(key.ulid, decoded.ulid);
    }

    #[test]
    fn test_event_key_lexicographic_order() {
        let key1 = EventKey::from_parts(1000, Ulid::new());
        let key2 = EventKey::from_parts(2000, Ulid::new());
        assert!(key1.to_bytes() < key2.to_bytes());
    }

    #[test]
    fn test_event_key_from_event_id() {
        let original = EventKey::new(1706540400000);
        let event_id = original.event_id();
        let reconstructed = EventKey::from_event_id(&event_id).unwrap();
        assert_eq!(original.ulid, reconstructed.ulid);
    }

    #[test]
    fn test_outbox_key_roundtrip() {
        let key = OutboxKey::new(12345);
        let bytes = key.to_bytes();
        let decoded = OutboxKey::from_bytes(&bytes).unwrap();
        assert_eq!(key.sequence, decoded.sequence);
    }
}
