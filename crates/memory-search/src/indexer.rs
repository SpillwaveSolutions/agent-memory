//! Search indexer for adding documents to the Tantivy index.
//!
//! The indexer wraps IndexWriter with shared access via Arc<Mutex>.
//! Documents are not visible until commit() is called.

use std::sync::{Arc, Mutex};

use tantivy::{IndexWriter, Term};
use tracing::{debug, info, warn};

use memory_types::{Grip, TocNode};

use crate::document::{grip_to_doc, toc_node_to_doc};
use crate::error::SearchError;
use crate::index::SearchIndex;
use crate::schema::SearchSchema;

/// Manages document indexing operations.
///
/// Wraps IndexWriter for shared access across components.
/// Commit batches documents for visibility.
pub struct SearchIndexer {
    writer: Arc<Mutex<IndexWriter>>,
    schema: SearchSchema,
}

impl SearchIndexer {
    /// Create a new indexer from a SearchIndex.
    pub fn new(index: &SearchIndex) -> Result<Self, SearchError> {
        let writer = index.writer()?;
        let schema = index.schema().clone();

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            schema,
        })
    }

    /// Create from an existing writer (for testing or shared use).
    pub fn from_writer(writer: IndexWriter, schema: SearchSchema) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
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
}
