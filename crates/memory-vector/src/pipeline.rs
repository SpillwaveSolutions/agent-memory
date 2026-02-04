//! Outbox-driven vector indexing pipeline.
//!
//! Consumes TOC nodes and grips, generates embeddings, and adds to HNSW index
//! with checkpoint tracking for crash recovery.
//!
//! Requirements: FR-09 (Outbox-driven indexing), FR-10 (Checkpoint-based recovery)

use std::sync::{Arc, RwLock};

use chrono::Utc;
use tracing::{debug, error, info, warn};

use memory_embeddings::EmbeddingModel;
use memory_types::TocNode;

use crate::error::VectorError;
use crate::hnsw::HnswIndex;
use crate::index::VectorIndex;
use crate::metadata::{DocType, VectorEntry, VectorMetadata};

/// Checkpoint key for vector indexing
pub const VECTOR_INDEX_CHECKPOINT: &str = "vector_index_last_processed";

/// Statistics from indexing run
#[derive(Debug, Default, Clone)]
pub struct IndexingStats {
    /// Number of entries processed
    pub entries_processed: usize,
    /// Number of vectors successfully added to index
    pub vectors_added: usize,
    /// Number of entries skipped (already indexed or empty)
    pub vectors_skipped: usize,
    /// Number of errors encountered
    pub errors: usize,
}

impl IndexingStats {
    /// Merge another stats into this one
    pub fn merge(&mut self, other: &IndexingStats) {
        self.entries_processed += other.entries_processed;
        self.vectors_added += other.vectors_added;
        self.vectors_skipped += other.vectors_skipped;
        self.errors += other.errors;
    }
}

/// Vector indexing pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Batch size for processing entries
    pub batch_size: usize,
    /// Maximum entries to process per run (0 = unlimited)
    pub max_entries_per_run: usize,
    /// Whether to continue on individual entry errors
    pub continue_on_error: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            max_entries_per_run: 1000,
            continue_on_error: true,
        }
    }
}

/// Item to be indexed by the pipeline
#[derive(Debug, Clone)]
pub enum IndexableItem {
    /// A TOC node to index
    TocNode {
        /// Node ID
        node_id: String,
        /// Node data
        node: TocNode,
    },
    /// A grip to index
    Grip {
        /// Grip ID
        grip_id: String,
        /// Excerpt text to embed
        excerpt: String,
        /// Timestamp when grip was created
        created_at: i64,
    },
}

impl IndexableItem {
    /// Get the document ID for this item
    pub fn doc_id(&self) -> &str {
        match self {
            IndexableItem::TocNode { node_id, .. } => node_id,
            IndexableItem::Grip { grip_id, .. } => grip_id,
        }
    }

    /// Get the document type for this item
    pub fn doc_type(&self) -> DocType {
        match self {
            IndexableItem::TocNode { .. } => DocType::TocNode,
            IndexableItem::Grip { .. } => DocType::Grip,
        }
    }

    /// Get the text to embed for this item
    pub fn text(&self) -> String {
        match self {
            IndexableItem::TocNode { node, .. } => extract_node_text(node),
            IndexableItem::Grip { excerpt, .. } => excerpt.clone(),
        }
    }

    /// Get the created_at timestamp for this item
    pub fn created_at(&self) -> i64 {
        match self {
            IndexableItem::TocNode { node, .. } => node.created_at.timestamp_millis(),
            IndexableItem::Grip { created_at, .. } => *created_at,
        }
    }
}

/// Extract searchable text from a TOC node.
fn extract_node_text(node: &TocNode) -> String {
    let mut parts = Vec::new();

    // Include title
    if !node.title.is_empty() {
        parts.push(node.title.clone());
    }

    // Include bullets
    for bullet in &node.bullets {
        parts.push(bullet.text.clone());
    }

    // Include keywords
    if !node.keywords.is_empty() {
        parts.push(node.keywords.join(" "));
    }

    parts.join(". ")
}

/// Vector indexing pipeline.
///
/// Processes items (TOC nodes and grips), generates embeddings, and adds to HNSW index.
/// Uses checkpoint tracking for crash recovery.
pub struct VectorIndexPipeline<E: EmbeddingModel> {
    embedder: Arc<E>,
    index: Arc<RwLock<HnswIndex>>,
    metadata: Arc<VectorMetadata>,
    config: PipelineConfig,
}

impl<E: EmbeddingModel> VectorIndexPipeline<E> {
    /// Create a new pipeline.
    pub fn new(
        embedder: Arc<E>,
        index: Arc<RwLock<HnswIndex>>,
        metadata: Arc<VectorMetadata>,
        config: PipelineConfig,
    ) -> Self {
        Self {
            embedder,
            index,
            metadata,
            config,
        }
    }

    /// Index a batch of items.
    ///
    /// Returns statistics about the indexing operation.
    pub fn index_items(&self, items: &[IndexableItem]) -> Result<IndexingStats, VectorError> {
        let mut stats = IndexingStats::default();

        if items.is_empty() {
            debug!("No items to index");
            return Ok(stats);
        }

        info!(count = items.len(), "Processing items for vector indexing");

        // Process in batches
        for batch in items.chunks(self.config.batch_size) {
            match self.process_batch(batch) {
                Ok(batch_stats) => {
                    stats.merge(&batch_stats);
                }
                Err(e) => {
                    error!(error = %e, "Batch processing failed");
                    if !self.config.continue_on_error {
                        return Err(e);
                    }
                    stats.errors += batch.len();
                }
            }
        }

        // Save index after processing
        {
            let index = self
                .index
                .read()
                .map_err(|e| VectorError::Index(format!("Failed to acquire read lock: {}", e)))?;
            index.save()?;
        }

        info!(
            processed = stats.entries_processed,
            added = stats.vectors_added,
            skipped = stats.vectors_skipped,
            errors = stats.errors,
            "Vector indexing complete"
        );

        Ok(stats)
    }

    /// Process a batch of items.
    fn process_batch(&self, items: &[IndexableItem]) -> Result<IndexingStats, VectorError> {
        let mut stats = IndexingStats::default();

        for item in items {
            stats.entries_processed += 1;

            match self.process_item(item) {
                Ok(true) => stats.vectors_added += 1,
                Ok(false) => stats.vectors_skipped += 1,
                Err(e) => {
                    warn!(doc_id = %item.doc_id(), error = %e, "Failed to process item");
                    if self.config.continue_on_error {
                        stats.errors += 1;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Process a single item.
    ///
    /// Returns true if vector was added, false if skipped.
    fn process_item(&self, item: &IndexableItem) -> Result<bool, VectorError> {
        let doc_id = item.doc_id();

        // Skip if already indexed
        if self.metadata.find_by_doc_id(doc_id)?.is_some() {
            debug!(doc_id = %doc_id, "Already indexed, skipping");
            return Ok(false);
        }

        // Get text to embed
        let text = item.text();

        // Skip empty text
        if text.trim().is_empty() {
            debug!(doc_id = %doc_id, "Empty text, skipping");
            return Ok(false);
        }

        // Generate embedding
        let embedding = self.embedder.embed(&text)?;

        // Get next vector ID
        let vector_id = self.metadata.next_vector_id()?;

        // Add to index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| VectorError::Index(format!("Failed to acquire write lock: {}", e)))?;
            index.add(vector_id, &embedding)?;
        }

        // Store metadata
        let meta_entry = VectorEntry::new(
            vector_id,
            item.doc_type(),
            doc_id.to_string(),
            item.created_at(),
            &text,
        );
        self.metadata.put(&meta_entry)?;

        debug!(vector_id = vector_id, doc_id = %doc_id, "Indexed vector");
        Ok(true)
    }

    /// Index a single TOC node.
    pub fn index_toc_node(&self, node: &TocNode) -> Result<bool, VectorError> {
        let item = IndexableItem::TocNode {
            node_id: node.node_id.clone(),
            node: node.clone(),
        };
        self.process_item(&item)
    }

    /// Index a single grip.
    pub fn index_grip(
        &self,
        grip_id: &str,
        excerpt: &str,
        created_at: i64,
    ) -> Result<bool, VectorError> {
        let item = IndexableItem::Grip {
            grip_id: grip_id.to_string(),
            excerpt: excerpt.to_string(),
            created_at,
        };
        self.process_item(&item)
    }

    /// Rebuild entire vector index from scratch.
    ///
    /// Clears existing index and re-indexes all provided items.
    pub fn rebuild(&self, items: &[IndexableItem]) -> Result<IndexingStats, VectorError> {
        info!("Starting full vector index rebuild");

        // Clear index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| VectorError::Index(format!("Failed to acquire write lock: {}", e)))?;
            index.clear()?;
        }

        // Clear metadata
        self.metadata.clear()?;

        // Re-index all items
        self.index_items(items)
    }

    /// Prune old vectors based on age.
    ///
    /// Removes vectors older than age_days from the HNSW index.
    /// Does NOT delete primary data (TOC nodes, grips remain in RocksDB).
    pub fn prune(&self, age_days: u64) -> Result<usize, VectorError> {
        let cutoff_ms = Utc::now().timestamp_millis() - (age_days as i64 * 24 * 60 * 60 * 1000);

        info!(
            age_days = age_days,
            cutoff_ms = cutoff_ms,
            "Pruning old vectors"
        );

        let all_entries = self.metadata.get_all()?;
        let mut pruned = 0;

        for entry in all_entries {
            if entry.created_at < cutoff_ms {
                // Remove from HNSW index
                {
                    let mut index = self.index.write().map_err(|e| {
                        VectorError::Index(format!("Failed to acquire write lock: {}", e))
                    })?;
                    index.remove(entry.vector_id)?;
                }
                // Remove metadata
                self.metadata.delete(entry.vector_id)?;
                pruned += 1;
            }
        }

        if pruned > 0 {
            let index = self
                .index
                .read()
                .map_err(|e| VectorError::Index(format!("Failed to acquire read lock: {}", e)))?;
            index.save()?;
        }

        info!(pruned = pruned, "Prune complete");
        Ok(pruned)
    }

    /// Get the current index statistics.
    pub fn stats(&self) -> Result<crate::index::IndexStats, VectorError> {
        let index = self
            .index
            .read()
            .map_err(|e| VectorError::Index(format!("Failed to acquire read lock: {}", e)))?;
        Ok(index.stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_embeddings::{Embedding, EmbeddingError};

    // Mock embedder for testing
    struct MockEmbedder {
        dimension: usize,
    }

    impl MockEmbedder {
        #[allow(dead_code)]
        fn new(dimension: usize) -> Self {
            Self { dimension }
        }
    }

    impl EmbeddingModel for MockEmbedder {
        fn info(&self) -> &memory_embeddings::ModelInfo {
            // Return a static reference - in real code you'd store this
            // For testing, we'll panic if this is called
            unimplemented!("info() not needed for tests")
        }

        fn embed(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
            // Return a simple embedding of the correct dimension
            let values: Vec<f32> = (0..self.dimension).map(|i| (i as f32) / 100.0).collect();
            Ok(Embedding::new(values))
        }
    }

    #[test]
    fn test_extract_node_text() {
        use memory_types::{TocBullet, TocLevel};

        let mut node = TocNode::new(
            "test-node".to_string(),
            TocLevel::Day,
            "Test Day".to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.bullets.push(TocBullet::new("First bullet"));
        node.bullets.push(TocBullet::new("Second bullet"));
        node.keywords = vec!["keyword1".to_string(), "keyword2".to_string()];

        let text = extract_node_text(&node);

        assert!(text.contains("Test Day"));
        assert!(text.contains("First bullet"));
        assert!(text.contains("Second bullet"));
        assert!(text.contains("keyword1"));
        assert!(text.contains("keyword2"));
    }

    #[test]
    fn test_indexable_item_toc_node() {
        use memory_types::TocLevel;

        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Test Day".to_string(),
            Utc::now(),
            Utc::now(),
        );

        let item = IndexableItem::TocNode {
            node_id: node.node_id.clone(),
            node: node.clone(),
        };

        assert_eq!(item.doc_id(), "toc:day:2024-01-15");
        assert_eq!(item.doc_type(), DocType::TocNode);
        assert!(!item.text().is_empty());
    }

    #[test]
    fn test_indexable_item_grip() {
        let item = IndexableItem::Grip {
            grip_id: "grip:123".to_string(),
            excerpt: "Test excerpt content".to_string(),
            created_at: 1705320000000,
        };

        assert_eq!(item.doc_id(), "grip:123");
        assert_eq!(item.doc_type(), DocType::Grip);
        assert_eq!(item.text(), "Test excerpt content");
        assert_eq!(item.created_at(), 1705320000000);
    }

    #[test]
    fn test_indexing_stats_merge() {
        let mut stats1 = IndexingStats {
            entries_processed: 10,
            vectors_added: 8,
            vectors_skipped: 1,
            errors: 1,
        };

        let stats2 = IndexingStats {
            entries_processed: 5,
            vectors_added: 4,
            vectors_skipped: 1,
            errors: 0,
        };

        stats1.merge(&stats2);

        assert_eq!(stats1.entries_processed, 15);
        assert_eq!(stats1.vectors_added, 12);
        assert_eq!(stats1.vectors_skipped, 2);
        assert_eq!(stats1.errors, 1);
    }

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.batch_size, 32);
        assert_eq!(config.max_entries_per_run, 1000);
        assert!(config.continue_on_error);
    }
}
