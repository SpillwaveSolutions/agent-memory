//! Search indexer for adding documents to the Tantivy index.
//!
//! The indexer wraps IndexWriter with shared access via `Arc<Mutex>`.
//! Documents are not visible until commit() is called.

use std::sync::{Arc, Mutex};

use chrono::Utc;
use tantivy::collector::DocSetCollector;
use tantivy::query::AllQuery;
use tantivy::schema::Value;
use tantivy::{IndexReader, IndexWriter, ReloadPolicy, Term};
use tracing::{debug, info, warn};

use memory_types::{Grip, TocNode};

use crate::document::{grip_to_doc, toc_node_to_doc};
use crate::error::SearchError;
use crate::index::SearchIndex;
use crate::lifecycle::Bm25PruneStats;
use crate::schema::SearchSchema;

/// Manages document indexing operations.
///
/// Wraps IndexWriter for shared access across components.
/// Commit batches documents for visibility.
pub struct SearchIndexer {
    writer: Arc<Mutex<IndexWriter>>,
    reader: IndexReader,
    schema: SearchSchema,
}

impl SearchIndexer {
    /// Create a new indexer from a SearchIndex.
    pub fn new(index: &SearchIndex) -> Result<Self, SearchError> {
        let writer = index.writer()?;
        let reader = index
            .index()
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let schema = index.schema().clone();

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            reader,
            schema,
        })
    }

    /// Create from an existing writer (for testing or shared use).
    pub fn from_writer(writer: IndexWriter, reader: IndexReader, schema: SearchSchema) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            reader,
            schema,
        }
    }

    /// Get a clone of the writer Arc for sharing.
    pub fn writer_handle(&self) -> Arc<Mutex<IndexWriter>> {
        self.writer.clone()
    }

    /// Index a TOC node.
    ///
    /// If a document with the same node_id exists, it will be replaced.
    pub fn index_toc_node(&self, node: &TocNode) -> Result<(), SearchError> {
        let doc = toc_node_to_doc(&self.schema, node);

        let writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        // Delete existing document with same ID (for update)
        let term = Term::from_field_text(self.schema.doc_id, &node.node_id);
        writer.delete_term(term);

        // Add new document
        writer.add_document(doc)?;

        debug!(node_id = %node.node_id, level = %node.level, "Indexed TOC node");
        Ok(())
    }

    /// Index a grip.
    ///
    /// If a document with the same grip_id exists, it will be replaced.
    pub fn index_grip(&self, grip: &Grip) -> Result<(), SearchError> {
        let doc = grip_to_doc(&self.schema, grip);

        let writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        // Delete existing document with same ID (for update)
        let term = Term::from_field_text(self.schema.doc_id, &grip.grip_id);
        writer.delete_term(term);

        // Add new document
        writer.add_document(doc)?;

        debug!(grip_id = %grip.grip_id, "Indexed grip");
        Ok(())
    }

    /// Index multiple TOC nodes in batch.
    pub fn index_toc_nodes(&self, nodes: &[TocNode]) -> Result<usize, SearchError> {
        let writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        let mut count = 0;
        for node in nodes {
            let doc = toc_node_to_doc(&self.schema, node);

            // Delete existing
            let term = Term::from_field_text(self.schema.doc_id, &node.node_id);
            writer.delete_term(term);

            // Add new
            writer.add_document(doc)?;
            count += 1;
        }

        debug!(count, "Indexed TOC nodes batch");
        Ok(count)
    }

    /// Index multiple grips in batch.
    pub fn index_grips(&self, grips: &[Grip]) -> Result<usize, SearchError> {
        let writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        let mut count = 0;
        for grip in grips {
            let doc = grip_to_doc(&self.schema, grip);

            // Delete existing
            let term = Term::from_field_text(self.schema.doc_id, &grip.grip_id);
            writer.delete_term(term);

            // Add new
            writer.add_document(doc)?;
            count += 1;
        }

        debug!(count, "Indexed grips batch");
        Ok(count)
    }

    /// Delete a document by ID.
    pub fn delete_document(&self, doc_id: &str) -> Result<(), SearchError> {
        let writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        let term = Term::from_field_text(self.schema.doc_id, doc_id);
        writer.delete_term(term);

        debug!(doc_id, "Deleted document");
        Ok(())
    }

    /// Commit pending changes to make them searchable.
    ///
    /// This is expensive - batch document adds and commit periodically.
    pub fn commit(&self) -> Result<u64, SearchError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        let opstamp = writer.commit()?;
        info!(opstamp, "Committed index changes");
        Ok(opstamp)
    }

    /// Rollback uncommitted changes.
    pub fn rollback(&self) -> Result<u64, SearchError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        let opstamp = writer.rollback()?;
        warn!(opstamp, "Rolled back index changes");
        Ok(opstamp)
    }

    /// Get the current commit opstamp.
    pub fn pending_ops(&self) -> Result<u64, SearchError> {
        let writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

        Ok(writer.commit_opstamp())
    }

    /// Reload the reader to see recent commits.
    pub fn reload_reader(&self) -> Result<(), SearchError> {
        self.reader.reload()?;
        debug!("Reloaded indexer reader");
        Ok(())
    }

    /// Prune documents older than the specified age.
    ///
    /// Scans all documents and deletes those with timestamp_ms older than
    /// (now - age_days). Does NOT commit - caller must commit() after pruning.
    ///
    /// # Arguments
    /// * `age_days` - Documents older than this many days will be deleted
    /// * `level_filter` - Optional level filter (e.g., "segment", "grip", "day")
    /// * `dry_run` - If true, counts but doesn't delete
    ///
    /// Returns statistics about pruned documents.
    pub fn prune(
        &self,
        age_days: u64,
        level_filter: Option<&str>,
        dry_run: bool,
    ) -> Result<Bm25PruneStats, SearchError> {
        let cutoff_ms = Utc::now().timestamp_millis() - (age_days as i64 * 24 * 60 * 60 * 1000);

        info!(
            age_days = age_days,
            cutoff_ms = cutoff_ms,
            level = ?level_filter,
            dry_run = dry_run,
            "Starting BM25 prune"
        );

        // Reload reader to see latest commits
        self.reader.reload()?;

        let searcher = self.reader.searcher();
        let mut stats = Bm25PruneStats::new();
        let mut docs_to_delete: Vec<String> = Vec::new();

        // Collect all documents using AllQuery
        let all_docs = searcher.search(&AllQuery, &DocSetCollector)?;

        debug!(
            all_docs_count = all_docs.len(),
            "Found documents to scan for pruning"
        );

        for doc_address in all_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            // Get timestamp
            let timestamp_ms = doc
                .get_first(self.schema.timestamp_ms)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(i64::MAX); // Don't delete if timestamp missing

            // Get level for filtering and stats
            let level = doc
                .get_first(self.schema.level)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Get doc type for grips (which have empty level)
            let doc_type = doc
                .get_first(self.schema.doc_type)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Apply level filter if specified
            let effective_level = if level.is_empty() && doc_type == "grip" {
                "grip"
            } else {
                level
            };

            if let Some(filter) = level_filter {
                if effective_level != filter {
                    continue;
                }
            }

            // Check if older than cutoff
            if timestamp_ms < cutoff_ms {
                let doc_id = doc
                    .get_first(self.schema.doc_id)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Update stats by level
                stats.add(effective_level, 1);

                debug!(
                    doc_id = %doc_id,
                    level = effective_level,
                    timestamp_ms = timestamp_ms,
                    "Document marked for pruning"
                );

                if !dry_run {
                    docs_to_delete.push(doc_id);
                }
            }
        }

        // Delete documents if not dry run
        if !dry_run && !docs_to_delete.is_empty() {
            let writer = self
                .writer
                .lock()
                .map_err(|e| SearchError::IndexLocked(e.to_string()))?;

            for doc_id in &docs_to_delete {
                let term = Term::from_field_text(self.schema.doc_id, doc_id);
                writer.delete_term(term);
            }

            info!(
                count = docs_to_delete.len(),
                dry_run = dry_run,
                "Deleted documents (uncommitted)"
            );
        }

        info!(
            total = stats.total(),
            segments = stats.segments_pruned,
            grips = stats.grips_pruned,
            days = stats.days_pruned,
            weeks = stats.weeks_pruned,
            dry_run = dry_run,
            "BM25 prune complete"
        );

        Ok(stats)
    }

    /// Prune and commit in one operation.
    ///
    /// Convenience method that calls prune() followed by commit().
    pub fn prune_and_commit(
        &self,
        age_days: u64,
        level_filter: Option<&str>,
        dry_run: bool,
    ) -> Result<Bm25PruneStats, SearchError> {
        let stats = self.prune(age_days, level_filter, dry_run)?;

        if !dry_run && stats.total() > 0 {
            self.commit()?;
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{SearchIndex, SearchIndexConfig};
    use chrono::Utc;
    use memory_types::{TocBullet, TocLevel};
    use tempfile::TempDir;

    fn sample_toc_node(id: &str) -> TocNode {
        let mut node = TocNode::new(
            id.to_string(),
            TocLevel::Day,
            format!("Test Node {}", id),
            Utc::now(),
            Utc::now(),
        );
        node.bullets = vec![TocBullet::new("Test bullet content")];
        node.keywords = vec!["test".to_string()];
        node
    }

    fn sample_grip(id: &str) -> Grip {
        Grip::new(
            id.to_string(),
            "Test excerpt content".to_string(),
            "event-001".to_string(),
            "event-002".to_string(),
            Utc::now(),
            "test".to_string(),
        )
    }

    #[test]
    fn test_index_toc_node() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        let node = sample_toc_node("node-1");
        indexer.index_toc_node(&node).unwrap();
        indexer.commit().unwrap();
    }

    #[test]
    fn test_index_grip() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        let grip = sample_grip("grip-1");
        indexer.index_grip(&grip).unwrap();
        indexer.commit().unwrap();
    }

    #[test]
    fn test_index_batch() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        let nodes: Vec<TocNode> = (0..5)
            .map(|i| sample_toc_node(&format!("node-{}", i)))
            .collect();

        let count = indexer.index_toc_nodes(&nodes).unwrap();
        assert_eq!(count, 5);
        indexer.commit().unwrap();
    }

    #[test]
    fn test_index_grips_batch() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        let grips: Vec<Grip> = (0..3)
            .map(|i| sample_grip(&format!("grip-{}", i)))
            .collect();

        let count = indexer.index_grips(&grips).unwrap();
        assert_eq!(count, 3);
        indexer.commit().unwrap();
    }

    #[test]
    fn test_update_existing_document() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Index initial version
        let mut node = sample_toc_node("node-1");
        node.title = "Version 1".to_string();
        indexer.index_toc_node(&node).unwrap();
        indexer.commit().unwrap();

        // Index updated version (same ID)
        node.title = "Version 2".to_string();
        node.version = 2;
        indexer.index_toc_node(&node).unwrap();
        indexer.commit().unwrap();

        // Should only have one document
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs, 1);
    }

    #[test]
    fn test_delete_document() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Index a node
        let node = sample_toc_node("node-to-delete");
        indexer.index_toc_node(&node).unwrap();
        indexer.commit().unwrap();

        // Verify it exists
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs_before: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs_before, 1);

        // Delete it
        indexer.delete_document("node-to-delete").unwrap();
        indexer.commit().unwrap();

        // Verify it's gone (need new reader after commit)
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs_after: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs_after, 0);
    }

    #[test]
    fn test_writer_handle() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Get handle and verify it's shareable
        let handle1 = indexer.writer_handle();
        let handle2 = indexer.writer_handle();

        // Both handles should point to the same Arc
        assert!(Arc::ptr_eq(&handle1, &handle2));
    }

    #[test]
    fn test_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Index a node but don't commit
        let node = sample_toc_node("node-to-rollback");
        indexer.index_toc_node(&node).unwrap();

        // Rollback
        indexer.rollback().unwrap();

        // Commit (after rollback, writer is reset)
        indexer.commit().unwrap();

        // Verify no documents exist
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs, 0);
    }

    #[test]
    fn test_pending_ops() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Check we can get the opstamp
        let opstamp = indexer.pending_ops().unwrap();
        // Initial opstamp should be 0
        assert_eq!(opstamp, 0);
    }

    fn sample_old_toc_node(id: &str, days_old: i64) -> TocNode {
        use chrono::Duration;
        let old_time = Utc::now() - Duration::days(days_old);
        let mut node = TocNode::new(
            id.to_string(),
            TocLevel::Day,
            format!("Old Node {}", id),
            old_time,
            old_time,
        );
        node.bullets = vec![TocBullet::new("Old content")];
        node.keywords = vec!["old".to_string()];
        node
    }

    fn sample_old_grip(id: &str, days_old: i64) -> Grip {
        use chrono::Duration;
        let old_time = Utc::now() - Duration::days(days_old);
        Grip::new(
            id.to_string(),
            "Old excerpt content".to_string(),
            "event-001".to_string(),
            "event-002".to_string(),
            old_time,
            "test".to_string(),
        )
    }

    #[test]
    fn test_prune_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Prune empty index should succeed with zero pruned
        let stats = indexer.prune(30, None, false).unwrap();
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_prune_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Add old documents
        let old_node = sample_old_toc_node("old-node-1", 60);
        indexer.index_toc_node(&old_node).unwrap();
        indexer.commit().unwrap();

        // Dry run should report but not delete
        let stats = indexer.prune(30, None, true).unwrap();
        assert_eq!(stats.total(), 1);

        // Verify document still exists
        indexer.reload_reader().unwrap();
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs, 1);
    }

    #[test]
    fn test_prune_deletes_old_documents() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Add old and new documents
        let old_node = sample_old_toc_node("old-node-1", 60);
        let new_node = sample_toc_node("new-node-1");

        indexer.index_toc_node(&old_node).unwrap();
        indexer.index_toc_node(&new_node).unwrap();
        indexer.commit().unwrap();

        // Prune documents older than 30 days
        let stats = indexer.prune_and_commit(30, None, false).unwrap();
        assert_eq!(stats.total(), 1);
        assert_eq!(stats.days_pruned, 1); // TocLevel::Day

        // Verify only new document remains
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs, 1);
    }

    #[test]
    fn test_prune_with_level_filter() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Add old TOC node and old grip
        let old_node = sample_old_toc_node("old-node-1", 60);
        let old_grip = sample_old_grip("old-grip-1", 60);

        indexer.index_toc_node(&old_node).unwrap();
        indexer.index_grip(&old_grip).unwrap();
        indexer.commit().unwrap();

        // Prune only grips
        let stats = indexer.prune_and_commit(30, Some("grip"), false).unwrap();
        assert_eq!(stats.total(), 1);
        assert_eq!(stats.grips_pruned, 1);
        assert_eq!(stats.days_pruned, 0); // TOC node should not be pruned

        // Verify TOC node still exists
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs, 1);
    }

    #[test]
    fn test_prune_keeps_recent_documents() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Add only recent documents
        let new_node1 = sample_toc_node("new-node-1");
        let new_node2 = sample_toc_node("new-node-2");
        let new_grip = sample_grip("new-grip-1");

        indexer.index_toc_node(&new_node1).unwrap();
        indexer.index_toc_node(&new_node2).unwrap();
        indexer.index_grip(&new_grip).unwrap();
        indexer.commit().unwrap();

        // Prune documents older than 30 days - should prune nothing
        let stats = indexer.prune_and_commit(30, None, false).unwrap();
        assert_eq!(stats.total(), 0);

        // Verify all documents still exist
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();
        assert_eq!(num_docs, 3);
    }

    #[test]
    fn test_reload_reader() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Reload should succeed
        indexer.reload_reader().unwrap();
    }
}
