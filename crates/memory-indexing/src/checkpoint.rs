//! Checkpoint tracking for indexing pipelines.
//!
//! Checkpoints track the last processed outbox sequence number for each
//! index type, enabling crash recovery and resumable indexing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::IndexingError;

/// Type of index being tracked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexType {
    /// BM25 full-text search index
    Bm25,
    /// Vector similarity search index
    Vector,
    /// Combined index (both BM25 and vector)
    Combined,
}

impl IndexType {
    /// Get the checkpoint key for this index type
    pub fn checkpoint_key(&self) -> &'static str {
        match self {
            IndexType::Bm25 => "index_bm25",
            IndexType::Vector => "index_vector",
            IndexType::Combined => "index_combined",
        }
    }
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::Bm25 => write!(f, "bm25"),
            IndexType::Vector => write!(f, "vector"),
            IndexType::Combined => write!(f, "combined"),
        }
    }
}

/// Checkpoint for tracking indexing progress.
///
/// Persisted to storage to enable crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCheckpoint {
    /// Type of index this checkpoint is for
    pub index_type: IndexType,

    /// Last outbox sequence number processed
    pub last_sequence: u64,

    /// Timestamp of last processing (milliseconds since epoch for JSON compatibility)
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub last_processed_time: DateTime<Utc>,

    /// Total items processed since checkpoint creation
    pub processed_count: u64,

    /// When this checkpoint was first created (milliseconds since epoch)
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl IndexCheckpoint {
    /// Create a new checkpoint for the given index type
    pub fn new(index_type: IndexType) -> Self {
        let now = Utc::now();
        Self {
            index_type,
            last_sequence: 0,
            last_processed_time: now,
            processed_count: 0,
            created_at: now,
        }
    }

    /// Create a checkpoint with a specific starting sequence
    pub fn with_sequence(index_type: IndexType, sequence: u64) -> Self {
        let now = Utc::now();
        Self {
            index_type,
            last_sequence: sequence,
            last_processed_time: now,
            processed_count: 0,
            created_at: now,
        }
    }

    /// Get the checkpoint key for storage
    pub fn checkpoint_key(&self) -> &'static str {
        self.index_type.checkpoint_key()
    }

    /// Update checkpoint after processing entries
    pub fn update(&mut self, new_sequence: u64, items_processed: u64) {
        self.last_sequence = new_sequence;
        self.last_processed_time = Utc::now();
        self.processed_count += items_processed;
    }

    /// Serialize to JSON bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>, IndexingError> {
        serde_json::to_vec(self).map_err(IndexingError::from)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IndexingError> {
        serde_json::from_slice(bytes).map_err(IndexingError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_type_checkpoint_keys() {
        assert_eq!(IndexType::Bm25.checkpoint_key(), "index_bm25");
        assert_eq!(IndexType::Vector.checkpoint_key(), "index_vector");
        assert_eq!(IndexType::Combined.checkpoint_key(), "index_combined");
    }

    #[test]
    fn test_index_type_display() {
        assert_eq!(IndexType::Bm25.to_string(), "bm25");
        assert_eq!(IndexType::Vector.to_string(), "vector");
        assert_eq!(IndexType::Combined.to_string(), "combined");
    }

    #[test]
    fn test_checkpoint_new() {
        let checkpoint = IndexCheckpoint::new(IndexType::Bm25);
        assert_eq!(checkpoint.index_type, IndexType::Bm25);
        assert_eq!(checkpoint.last_sequence, 0);
        assert_eq!(checkpoint.processed_count, 0);
        assert_eq!(checkpoint.checkpoint_key(), "index_bm25");
    }

    #[test]
    fn test_checkpoint_with_sequence() {
        let checkpoint = IndexCheckpoint::with_sequence(IndexType::Vector, 100);
        assert_eq!(checkpoint.index_type, IndexType::Vector);
        assert_eq!(checkpoint.last_sequence, 100);
        assert_eq!(checkpoint.processed_count, 0);
    }

    #[test]
    fn test_checkpoint_update() {
        let mut checkpoint = IndexCheckpoint::new(IndexType::Bm25);
        let original_time = checkpoint.last_processed_time;

        // Small sleep to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        checkpoint.update(50, 10);
        assert_eq!(checkpoint.last_sequence, 50);
        assert_eq!(checkpoint.processed_count, 10);
        assert!(checkpoint.last_processed_time >= original_time);

        checkpoint.update(100, 5);
        assert_eq!(checkpoint.last_sequence, 100);
        assert_eq!(checkpoint.processed_count, 15);
    }

    #[test]
    fn test_checkpoint_serialization_roundtrip() {
        let checkpoint = IndexCheckpoint::with_sequence(IndexType::Combined, 42);
        let bytes = checkpoint.to_bytes().unwrap();
        let decoded = IndexCheckpoint::from_bytes(&bytes).unwrap();

        assert_eq!(checkpoint.index_type, decoded.index_type);
        assert_eq!(checkpoint.last_sequence, decoded.last_sequence);
        assert_eq!(checkpoint.processed_count, decoded.processed_count);
        // Compare timestamps at millisecond precision (JSON serializes to ms)
        assert_eq!(
            checkpoint.created_at.timestamp_millis(),
            decoded.created_at.timestamp_millis()
        );
        assert_eq!(
            checkpoint.last_processed_time.timestamp_millis(),
            decoded.last_processed_time.timestamp_millis()
        );
    }

    #[test]
    fn test_checkpoint_json_format() {
        let checkpoint = IndexCheckpoint::new(IndexType::Bm25);
        let bytes = checkpoint.to_bytes().unwrap();
        let json_str = String::from_utf8(bytes).unwrap();

        // Verify JSON contains expected fields
        assert!(json_str.contains("\"index_type\":\"bm25\""));
        assert!(json_str.contains("\"last_sequence\":0"));
        assert!(json_str.contains("\"processed_count\":0"));
        assert!(json_str.contains("\"last_processed_time\":"));
        assert!(json_str.contains("\"created_at\":"));
    }

    #[test]
    fn test_index_type_serialization() {
        // Test all variants serialize correctly
        let bm25 = serde_json::to_string(&IndexType::Bm25).unwrap();
        let vector = serde_json::to_string(&IndexType::Vector).unwrap();
        let combined = serde_json::to_string(&IndexType::Combined).unwrap();

        assert_eq!(bm25, "\"bm25\"");
        assert_eq!(vector, "\"vector\"");
        assert_eq!(combined, "\"combined\"");

        // Test deserialization
        let bm25: IndexType = serde_json::from_str("\"bm25\"").unwrap();
        let vector: IndexType = serde_json::from_str("\"vector\"").unwrap();
        let combined: IndexType = serde_json::from_str("\"combined\"").unwrap();

        assert_eq!(bm25, IndexType::Bm25);
        assert_eq!(vector, IndexType::Vector);
        assert_eq!(combined, IndexType::Combined);
    }
}
