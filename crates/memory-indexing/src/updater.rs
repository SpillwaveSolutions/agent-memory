//! Index updater trait for incremental index updates.
//!
//! Defines the interface for index-specific update operations.
//! Each index type (BM25, vector) implements this trait.

use crate::checkpoint::IndexType;
use crate::error::IndexingError;
use memory_types::OutboxEntry;

/// Trait for index-specific update operations.
///
/// Implementations handle the details of converting outbox entries
/// to index-specific documents and managing commits.
pub trait IndexUpdater: Send + Sync {
    /// Index a new or updated document from an outbox entry.
    ///
    /// The entry contains an event_id that can be used to fetch
    /// the full event data from storage if needed.
    fn index_document(&self, entry: &OutboxEntry) -> Result<(), IndexingError>;

    /// Remove a document from the index by its ID.
    ///
    /// The doc_id could be an event_id, node_id, or grip_id depending
    /// on what was indexed.
    fn remove_document(&self, doc_id: &str) -> Result<(), IndexingError>;

    /// Commit pending changes to make them visible.
    ///
    /// This may be expensive - batch updates before calling.
    fn commit(&self) -> Result<(), IndexingError>;

    /// Get the index type this updater manages.
    fn index_type(&self) -> IndexType;

    /// Get the name of this updater for logging.
    fn name(&self) -> &str;
}

/// Result of processing a batch of outbox entries.
#[derive(Debug, Default, Clone)]
pub struct UpdateResult {
    /// Number of entries successfully processed
    pub processed: usize,
    /// Number of entries skipped (already indexed, invalid, etc.)
    pub skipped: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// The highest sequence number processed
    pub last_sequence: u64,
}

impl UpdateResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful processing.
    pub fn record_success(&mut self) {
        self.processed += 1;
    }

    /// Record a skipped entry.
    pub fn record_skip(&mut self) {
        self.skipped += 1;
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    /// Set the last sequence number processed.
    pub fn set_sequence(&mut self, seq: u64) {
        self.last_sequence = seq;
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: &UpdateResult) {
        self.processed += other.processed;
        self.skipped += other.skipped;
        self.errors += other.errors;
        if other.last_sequence > self.last_sequence {
            self.last_sequence = other.last_sequence;
        }
    }

    /// Check if any entries were processed successfully.
    pub fn has_updates(&self) -> bool {
        self.processed > 0
    }

    /// Total number of entries handled (success + skip + error).
    pub fn total(&self) -> usize {
        self.processed + self.skipped + self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_result_new() {
        let result = UpdateResult::new();
        assert_eq!(result.processed, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors, 0);
        assert_eq!(result.last_sequence, 0);
    }

    #[test]
    fn test_update_result_record() {
        let mut result = UpdateResult::new();
        result.record_success();
        result.record_success();
        result.record_skip();
        result.record_error();
        result.set_sequence(42);

        assert_eq!(result.processed, 2);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.errors, 1);
        assert_eq!(result.last_sequence, 42);
        assert_eq!(result.total(), 4);
        assert!(result.has_updates());
    }

    #[test]
    fn test_update_result_merge() {
        let mut result1 = UpdateResult {
            processed: 5,
            skipped: 2,
            errors: 1,
            last_sequence: 10,
        };

        let result2 = UpdateResult {
            processed: 3,
            skipped: 1,
            errors: 0,
            last_sequence: 15,
        };

        result1.merge(&result2);

        assert_eq!(result1.processed, 8);
        assert_eq!(result1.skipped, 3);
        assert_eq!(result1.errors, 1);
        assert_eq!(result1.last_sequence, 15);
    }

    #[test]
    fn test_update_result_no_updates() {
        let mut result = UpdateResult::new();
        result.record_skip();
        result.record_error();

        assert!(!result.has_updates());
    }
}
