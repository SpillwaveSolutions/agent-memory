//! Vector index updater for HNSW-based semantic search.
//!
//! Wraps HnswIndex and CandleEmbedder to handle outbox-driven vector indexing.
//! Generates embeddings from text content and stores vectors with metadata.

use std::sync::{Arc, RwLock};

use tracing::{debug, warn};

use memory_embeddings::{CandleEmbedder, EmbeddingModel};
use memory_storage::Storage;
use memory_types::{Grip, OutboxAction, OutboxEntry, TocNode};
use memory_vector::{DocType, HnswIndex, VectorEntry, VectorIndex, VectorMetadata};

use crate::checkpoint::IndexType;
use crate::error::IndexingError;
use crate::updater::{IndexUpdater, UpdateResult};

/// Vector index updater using HNSW and Candle embeddings.
///
/// Generates embeddings for TOC nodes and grips, then stores
/// them in the HNSW index with metadata for retrieval.
pub struct VectorIndexUpdater<E: EmbeddingModel = CandleEmbedder> {
    index: Arc<RwLock<HnswIndex>>,
    embedder: Arc<E>,
    metadata: Arc<VectorMetadata>,
    storage: Arc<Storage>,
}

impl<E: EmbeddingModel> VectorIndexUpdater<E> {
    /// Create a new vector updater.
    pub fn new(
        index: Arc<RwLock<HnswIndex>>,
        embedder: Arc<E>,
        metadata: Arc<VectorMetadata>,
        storage: Arc<Storage>,
    ) -> Self {
        Self {
            index,
            embedder,
            metadata,
            storage,
        }
    }

    /// Extract text content from a TOC node for embedding.
    fn extract_toc_text(node: &TocNode) -> String {
        let mut parts = vec![node.title.clone()];

        for bullet in &node.bullets {
            parts.push(bullet.text.clone());
        }

        if !node.keywords.is_empty() {
            parts.push(node.keywords.join(" "));
        }

        parts.join(". ")
    }

    /// Index a TOC node.
    fn index_toc_node(&self, node: &TocNode) -> Result<bool, IndexingError> {
        let doc_id = &node.node_id;

        // Check if already indexed
        if self
            .metadata
            .find_by_doc_id(doc_id)
            .map_err(|e| IndexingError::Index(format!("Metadata lookup error: {}", e)))?
            .is_some()
        {
            debug!(doc_id = %doc_id, "TOC node already indexed, skipping");
            return Ok(false);
        }

        let text = Self::extract_toc_text(node);
        if text.trim().is_empty() {
            debug!(doc_id = %doc_id, "Empty text, skipping");
            return Ok(false);
        }

        // Generate embedding
        let embedding = self
            .embedder
            .embed(&text)
            .map_err(|e| IndexingError::Index(format!("Embedding error: {}", e)))?;

        // Get next vector ID
        let vector_id = self
            .metadata
            .next_vector_id()
            .map_err(|e| IndexingError::Index(format!("Metadata error: {}", e)))?;

        // Add to HNSW index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| IndexingError::Index(format!("Index lock error: {}", e)))?;
            index
                .add(vector_id, &embedding)
                .map_err(|e| IndexingError::Index(format!("HNSW add error: {}", e)))?;
        }

        // Store metadata
        let entry = VectorEntry::new(
            vector_id,
            DocType::TocNode,
            doc_id.to_string(),
            node.created_at.timestamp_millis(),
            &text,
        );
        self.metadata
            .put(&entry)
            .map_err(|e| IndexingError::Index(format!("Metadata put error: {}", e)))?;

        debug!(vector_id = vector_id, doc_id = %doc_id, "Indexed TOC node vector");
        Ok(true)
    }

    /// Index a grip.
    fn index_grip(&self, grip: &Grip) -> Result<bool, IndexingError> {
        let doc_id = &grip.grip_id;

        // Check if already indexed
        if self
            .metadata
            .find_by_doc_id(doc_id)
            .map_err(|e| IndexingError::Index(format!("Metadata lookup error: {}", e)))?
            .is_some()
        {
            debug!(doc_id = %doc_id, "Grip already indexed, skipping");
            return Ok(false);
        }

        let text = &grip.excerpt;
        if text.trim().is_empty() {
            debug!(doc_id = %doc_id, "Empty excerpt, skipping");
            return Ok(false);
        }

        // Generate embedding
        let embedding = self
            .embedder
            .embed(text)
            .map_err(|e| IndexingError::Index(format!("Embedding error: {}", e)))?;

        // Get next vector ID
        let vector_id = self
            .metadata
            .next_vector_id()
            .map_err(|e| IndexingError::Index(format!("Metadata error: {}", e)))?;

        // Add to HNSW index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| IndexingError::Index(format!("Index lock error: {}", e)))?;
            index
                .add(vector_id, &embedding)
                .map_err(|e| IndexingError::Index(format!("HNSW add error: {}", e)))?;
        }

        // Store metadata
        let entry = VectorEntry::new(
            vector_id,
            DocType::Grip,
            doc_id.to_string(),
            grip.timestamp.timestamp_millis(),
            text,
        );
        self.metadata
            .put(&entry)
            .map_err(|e| IndexingError::Index(format!("Metadata put error: {}", e)))?;

        debug!(vector_id = vector_id, doc_id = %doc_id, "Indexed grip vector");
        Ok(true)
    }

    /// Process an outbox entry.
    fn process_entry(&self, entry: &OutboxEntry) -> Result<bool, IndexingError> {
        match entry.action {
            OutboxAction::IndexEvent => {
                debug!(event_id = %entry.event_id, "Processing index event for vector");

                // Try to find a grip for this event
                if let Some(grip) = self.find_grip_for_event(&entry.event_id)? {
                    return self.index_grip(&grip);
                }

                debug!(event_id = %entry.event_id, "No grip found for event, skipping");
                Ok(false)
            }
            OutboxAction::UpdateToc => {
                debug!(event_id = %entry.event_id, "Skipping TOC update action");
                Ok(false)
            }
        }
    }

    /// Find a grip that references this event.
    fn find_grip_for_event(&self, event_id: &str) -> Result<Option<Grip>, IndexingError> {
        // Simplified lookup - return None for now
        // In a full implementation, this would query storage
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
                        "Failed to process entry for vector index"
                    );
                    result.record_error();
                }
            }
            result.set_sequence(*sequence);
        }

        Ok(result)
    }

    /// Index a TOC node directly (for bulk indexing).
    pub fn index_node(&self, node: &TocNode) -> Result<bool, IndexingError> {
        self.index_toc_node(node)
    }

    /// Index a grip directly (for bulk indexing).
    pub fn index_grip_direct(&self, grip: &Grip) -> Result<bool, IndexingError> {
        self.index_grip(grip)
    }

    /// Remove a vector by document ID.
    pub fn remove_by_doc_id(&self, doc_id: &str) -> Result<bool, IndexingError> {
        // Find the vector ID for this document
        let entry = self
            .metadata
            .find_by_doc_id(doc_id)
            .map_err(|e| IndexingError::Index(format!("Metadata lookup error: {}", e)))?;

        if let Some(entry) = entry {
            // Remove from HNSW
            {
                let mut index = self
                    .index
                    .write()
                    .map_err(|e| IndexingError::Index(format!("Index lock error: {}", e)))?;
                index
                    .remove(entry.vector_id)
                    .map_err(|e| IndexingError::Index(format!("HNSW remove error: {}", e)))?;
            }

            // Remove metadata
            self.metadata
                .delete(entry.vector_id)
                .map_err(|e| IndexingError::Index(format!("Metadata delete error: {}", e)))?;

            debug!(vector_id = entry.vector_id, doc_id = %doc_id, "Removed vector");
            Ok(true)
        } else {
            debug!(doc_id = %doc_id, "Vector not found for removal");
            Ok(false)
        }
    }

    /// Get the underlying storage reference.
    pub fn storage(&self) -> &Arc<Storage> {
        &self.storage
    }

    /// Get the embedding dimension.
    pub fn dimension(&self) -> usize {
        self.embedder.info().dimension
    }
}

impl<E: EmbeddingModel> IndexUpdater for VectorIndexUpdater<E> {
    fn index_document(&self, entry: &OutboxEntry) -> Result<(), IndexingError> {
        let _ = self.process_entry(entry)?;
        Ok(())
    }

    fn remove_document(&self, doc_id: &str) -> Result<(), IndexingError> {
        let _ = self.remove_by_doc_id(doc_id)?;
        Ok(())
    }

    fn commit(&self) -> Result<(), IndexingError> {
        // Save the HNSW index to disk
        let index = self
            .index
            .read()
            .map_err(|e| IndexingError::Index(format!("Index lock error: {}", e)))?;
        index
            .save()
            .map_err(|e| IndexingError::Index(format!("HNSW save error: {}", e)))?;
        Ok(())
    }

    fn index_type(&self) -> IndexType {
        IndexType::Vector
    }

    fn name(&self) -> &str {
        "vector"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use memory_embeddings::{Embedding, EmbeddingError, ModelInfo};
    use memory_types::TocLevel;
    use memory_vector::HnswConfig;
    use tempfile::TempDir;

    // Mock embedder for testing
    struct MockEmbedder {
        dimension: usize,
        info: ModelInfo,
    }

    impl MockEmbedder {
        fn new(dimension: usize) -> Self {
            Self {
                dimension,
                info: ModelInfo {
                    name: "mock".to_string(),
                    dimension,
                    max_sequence_length: 512,
                },
            }
        }
    }

    impl EmbeddingModel for MockEmbedder {
        fn info(&self) -> &ModelInfo {
            &self.info
        }

        fn embed(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
            // Return a simple embedding of the correct dimension
            let values: Vec<f32> = (0..self.dimension)
                .map(|i| (i as f32) / (self.dimension as f32))
                .collect();
            Ok(Embedding::new(values))
        }
    }

    fn create_test_components(
        temp_dir: &TempDir,
    ) -> (
        Arc<RwLock<HnswIndex>>,
        Arc<MockEmbedder>,
        Arc<VectorMetadata>,
        Arc<Storage>,
    ) {
        let storage_path = temp_dir.path().join("storage");
        std::fs::create_dir_all(&storage_path).unwrap();
        let storage = Arc::new(Storage::open(&storage_path).unwrap());

        let vector_path = temp_dir.path().join("vector");
        std::fs::create_dir_all(&vector_path).unwrap();

        let config = HnswConfig::new(64, &vector_path).with_capacity(1000);
        let index = Arc::new(RwLock::new(HnswIndex::open_or_create(config).unwrap()));

        let embedder = Arc::new(MockEmbedder::new(64));

        let metadata_path = temp_dir.path().join("metadata");
        std::fs::create_dir_all(&metadata_path).unwrap();
        let metadata = Arc::new(VectorMetadata::open(&metadata_path).unwrap());

        (index, embedder, metadata, storage)
    }

    #[test]
    fn test_vector_updater_creation() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index, embedder, metadata, storage);
        assert_eq!(updater.index_type(), IndexType::Vector);
        assert_eq!(updater.name(), "vector");
        assert_eq!(updater.dimension(), 64);
    }

    #[test]
    fn test_extract_toc_text() {
        use memory_types::TocBullet;

        let mut node = TocNode::new(
            "test-node".to_string(),
            TocLevel::Day,
            "Test Title".to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.bullets.push(TocBullet::new("First bullet"));
        node.bullets.push(TocBullet::new("Second bullet"));
        node.keywords = vec!["key1".to_string(), "key2".to_string()];

        let text = VectorIndexUpdater::<MockEmbedder>::extract_toc_text(&node);

        assert!(text.contains("Test Title"));
        assert!(text.contains("First bullet"));
        assert!(text.contains("Second bullet"));
        assert!(text.contains("key1"));
        assert!(text.contains("key2"));
    }

    #[test]
    fn test_index_toc_node() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index.clone(), embedder, metadata.clone(), storage);

        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Monday, January 15".to_string(),
            Utc::now(),
            Utc::now(),
        );

        let indexed = updater.index_node(&node).unwrap();
        assert!(indexed);

        // Should find in metadata
        let found = metadata.find_by_doc_id("toc:day:2024-01-15").unwrap();
        assert!(found.is_some());

        // Index should have one vector
        let idx = index.read().unwrap();
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn test_index_toc_node_duplicate() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index, embedder, metadata, storage);

        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Monday, January 15".to_string(),
            Utc::now(),
            Utc::now(),
        );

        // First index
        let indexed1 = updater.index_node(&node).unwrap();
        assert!(indexed1);

        // Second index should skip
        let indexed2 = updater.index_node(&node).unwrap();
        assert!(!indexed2);
    }

    #[test]
    fn test_index_grip() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index.clone(), embedder, metadata.clone(), storage);

        let grip = Grip::new(
            "grip:12345".to_string(),
            "User asked about authentication".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "test".to_string(),
        );

        let indexed = updater.index_grip_direct(&grip).unwrap();
        assert!(indexed);

        // Should find in metadata
        let found = metadata.find_by_doc_id("grip:12345").unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn test_remove_by_doc_id() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index.clone(), embedder, metadata.clone(), storage);

        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Monday, January 15".to_string(),
            Utc::now(),
            Utc::now(),
        );

        // Index first
        updater.index_node(&node).unwrap();
        assert_eq!(index.read().unwrap().len(), 1);

        // Remove
        let removed = updater.remove_by_doc_id("toc:day:2024-01-15").unwrap();
        assert!(removed);

        // Should be gone from metadata
        let found = metadata.find_by_doc_id("toc:day:2024-01-15").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index, embedder, metadata, storage);

        let removed = updater.remove_by_doc_id("nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_process_batch_empty() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index, embedder, metadata, storage);

        let entries: Vec<(u64, OutboxEntry)> = vec![];
        let result = updater.process_batch(&entries).unwrap();

        assert_eq!(result.total(), 0);
    }

    #[test]
    fn test_process_batch_with_entries() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index, embedder, metadata, storage);

        let entries = vec![
            (0, OutboxEntry::for_index("event-1".to_string(), 1000)),
            (1, OutboxEntry::for_index("event-2".to_string(), 2000)),
            (2, OutboxEntry::for_toc("event-3".to_string(), 3000)),
        ];

        let result = updater.process_batch(&entries).unwrap();

        // All should be skipped (no grips found)
        assert_eq!(result.skipped, 3);
        assert_eq!(result.last_sequence, 2);
    }

    #[test]
    fn test_commit() {
        let temp_dir = TempDir::new().unwrap();
        let (index, embedder, metadata, storage) = create_test_components(&temp_dir);

        let updater = VectorIndexUpdater::new(index, embedder, metadata, storage);

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
}
