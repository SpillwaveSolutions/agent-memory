//! BM25 index updater for Tantivy-based full-text search.
//!
//! Wraps SearchIndexer from memory-search to handle outbox-driven updates.
//! Converts OutboxEntry references to searchable documents.

use std::sync::Arc;

use tracing::{debug, warn};

use memory_search::SearchIndexer;
use memory_storage::Storage;
use memory_types::{Grip, OutboxAction, OutboxEntry, TocNode};

use crate::checkpoint::IndexType;
use crate::error::IndexingError;
use crate::updater::{IndexUpdater, UpdateResult};

/// BM25 index updater using Tantivy.
///
/// Indexes TOC nodes and grips for full-text BM25 search.
/// Consumes outbox entries and fetches the corresponding
/// data from storage for indexing.
pub struct Bm25IndexUpdater {
    indexer: Arc<SearchIndexer>,
    storage: Arc<Storage>,
}

impl Bm25IndexUpdater {
    /// Create a new BM25 updater.
    pub fn new(indexer: Arc<SearchIndexer>, storage: Arc<Storage>) -> Self {
        Self { indexer, storage }
    }

    /// Index a TOC node.
    fn index_toc_node(&self, node: &TocNode) -> Result<(), IndexingError> {
        self.indexer
            .index_toc_node(node)
            .map_err(|e| IndexingError::Index(format!("BM25 index error: {}", e)))
    }

    /// Index a grip.
    fn index_grip(&self, grip: &Grip) -> Result<(), IndexingError> {
        self.indexer
            .index_grip(grip)
            .map_err(|e| IndexingError::Index(format!("BM25 index error: {}", e)))
    }

    /// Process an outbox entry by fetching the event and related data.
    ///
    /// For IndexEvent actions, we need to determine if this event
    /// is associated with a TOC node or grip and index accordingly.
    fn process_entry(&self, entry: &OutboxEntry) -> Result<bool, IndexingError> {
        match entry.action {
            OutboxAction::IndexEvent => {
                // The event_id in the outbox entry points to the event
                // We need to check if there's a corresponding TOC node or grip
                // For now, we'll try to find and index any related content

                // Check if there's a TOC node associated with this timestamp
                // This is a simplified approach - in practice, you might have
                // more sophisticated event-to-document mapping
                debug!(event_id = %entry.event_id, "Processing index event for BM25");

                // Try to find TOC nodes that might reference this event
                // The event_id format is typically a ULID
                // We could look up grips that span this event
                if let Some(grip) = self.find_grip_for_event(&entry.event_id)? {
                    self.index_grip(&grip)?;
                    return Ok(true);
                }

                // If no direct match, the event will be indexed when
                // the summarizer creates TOC nodes/grips
                debug!(event_id = %entry.event_id, "No grip found for event, skipping");
                Ok(false)
            }
            OutboxAction::UpdateToc => {
                // For TOC updates, we'd need additional context about which
                // TOC node was updated. For now, skip these as they're
                // typically handled by the TOC expansion logic.
                debug!(event_id = %entry.event_id, "Skipping TOC update action");
                Ok(false)
            }
        }
    }

    /// Find a grip that references this event.
    fn find_grip_for_event(&self, event_id: &str) -> Result<Option<Grip>, IndexingError> {
        // This is a simplified lookup - in a full implementation,
        // you might have an index from event_id to grip_id
        // For now, we'll return None and rely on explicit grip indexing
        debug!(event_id = %event_id, "Looking up grip for event");
        Ok(None)
    }

    /// Process a batch of outbox entries.
    pub fn process_batch(
        &self,
        entries: &[(u64, OutboxEntry)],
    ) -> Result<UpdateResult, IndexingError> {
        let mut result = UpdateResult::new();

        for (sequence, entry) in entries {
            match self.process_entry(entry) {
                Ok(true) => {
                    result.record_success();
                }
                Ok(false) => {
                    result.record_skip();
                }
                Err(e) => {
                    warn!(
                        sequence = sequence,
                        event_id = %entry.event_id,
                        error = %e,
                        "Failed to process entry for BM25"
                    );
                    result.record_error();
                }
            }
            result.set_sequence(*sequence);
        }

        Ok(result)
    }

    /// Index a TOC node directly (for bulk indexing).
    pub fn index_node(&self, node: &TocNode) -> Result<(), IndexingError> {
        self.index_toc_node(node)
    }

    /// Index a grip directly (for bulk indexing).
    pub fn index_grip_direct(&self, grip: &Grip) -> Result<(), IndexingError> {
        self.index_grip(grip)
    }

    /// Get the underlying storage reference.
    pub fn storage(&self) -> &Arc<Storage> {
        &self.storage
    }
}

impl IndexUpdater for Bm25IndexUpdater {
    fn index_document(&self, entry: &OutboxEntry) -> Result<(), IndexingError> {
        match self.process_entry(entry)? {
            true => Ok(()),
            false => Ok(()), // Skipped entries are not errors
        }
    }

    fn remove_document(&self, doc_id: &str) -> Result<(), IndexingError> {
        self.indexer
            .delete_document(doc_id)
            .map_err(|e| IndexingError::Index(format!("BM25 delete error: {}", e)))
    }

    fn commit(&self) -> Result<(), IndexingError> {
        self.indexer
            .commit()
            .map_err(|e| IndexingError::Index(format!("BM25 commit error: {}", e)))?;
        Ok(())
    }

    fn index_type(&self) -> IndexType {
        IndexType::Bm25
    }

    fn name(&self) -> &str {
        "bm25"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_search::{SearchIndex, SearchIndexConfig};
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path()).unwrap();
        (Arc::new(storage), temp_dir)
    }

    fn create_test_indexer(path: &std::path::Path) -> Arc<SearchIndexer> {
        let config = SearchIndexConfig::new(path);
        let index = SearchIndex::open_or_create(config).unwrap();
        Arc::new(SearchIndexer::new(&index).unwrap())
    }

    #[test]
    fn test_bm25_updater_creation() {
        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);
        assert_eq!(updater.index_type(), IndexType::Bm25);
        assert_eq!(updater.name(), "bm25");
    }

    #[test]
    fn test_process_index_event_no_grip() {
        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);

        let entry = OutboxEntry::for_index("event-123".to_string(), 1706540400000);
        let result = updater.process_entry(&entry).unwrap();

        // Should return false since no grip found
        assert!(!result);
    }

    #[test]
    fn test_process_batch_empty() {
        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);

        let entries: Vec<(u64, OutboxEntry)> = vec![];
        let result = updater.process_batch(&entries).unwrap();

        assert_eq!(result.processed, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors, 0);
    }

    #[test]
    fn test_process_batch_with_entries() {
        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);

        let entries = vec![
            (0, OutboxEntry::for_index("event-1".to_string(), 1000)),
            (1, OutboxEntry::for_index("event-2".to_string(), 2000)),
            (2, OutboxEntry::for_toc("event-3".to_string(), 3000)),
        ];

        let result = updater.process_batch(&entries).unwrap();

        // All should be skipped (no grips found, TOC updates skipped)
        assert_eq!(result.skipped, 3);
        assert_eq!(result.last_sequence, 2);
    }

    #[test]
    fn test_index_toc_node_direct() {
        use chrono::Utc;
        use memory_types::TocLevel;

        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);

        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Monday, January 15".to_string(),
            Utc::now(),
            Utc::now(),
        );

        updater.index_node(&node).unwrap();
        updater.commit().unwrap();
    }

    #[test]
    fn test_index_grip_direct() {
        use chrono::Utc;

        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);

        let grip = Grip::new(
            "grip:12345".to_string(),
            "User asked about authentication".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "test".to_string(),
        );

        updater.index_grip_direct(&grip).unwrap();
        updater.commit().unwrap();
    }

    #[test]
    fn test_remove_document() {
        use chrono::Utc;
        use memory_types::TocLevel;

        let (storage, temp_dir) = create_test_storage();
        let search_path = temp_dir.path().join("search");
        std::fs::create_dir_all(&search_path).unwrap();
        let indexer = create_test_indexer(&search_path);

        let updater = Bm25IndexUpdater::new(indexer, storage);

        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Monday, January 15".to_string(),
            Utc::now(),
            Utc::now(),
        );

        updater.index_node(&node).unwrap();
        updater.commit().unwrap();

        updater.remove_document("toc:day:2024-01-15").unwrap();
        updater.commit().unwrap();
    }
}
