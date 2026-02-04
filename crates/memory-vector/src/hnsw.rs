//! HNSW index implementation using usearch.
//!
//! Parameters tuned for quality over speed:
//! - M = 16 (connections per layer)
//! - ef_construction = 200 (build-time quality)
//! - ef_search = 100 (search-time quality)

use std::path::PathBuf;
use std::sync::RwLock;

use memory_embeddings::Embedding;
use tracing::{debug, info};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use crate::error::VectorError;
use crate::index::{IndexStats, SearchResult, VectorIndex};

/// HNSW index configuration
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Embedding dimension (must match model)
    pub dimension: usize,
    /// Number of connections per layer (M parameter)
    pub connectivity: usize,
    /// Build-time search depth (ef_construction)
    pub expansion_add: usize,
    /// Query-time search depth (ef_search)
    pub expansion_search: usize,
    /// Index file path
    pub index_path: PathBuf,
    /// Maximum capacity (for pre-allocation)
    pub capacity: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            dimension: 384, // all-MiniLM-L6-v2
            connectivity: 16,
            expansion_add: 200,
            expansion_search: 100,
            index_path: PathBuf::from("./vector-index"),
            capacity: 1_000_000,
        }
    }
}

impl HnswConfig {
    pub fn new(dimension: usize, index_path: impl Into<PathBuf>) -> Self {
        Self {
            dimension,
            index_path: index_path.into(),
            ..Default::default()
        }
    }

    pub fn with_connectivity(mut self, m: usize) -> Self {
        self.connectivity = m;
        self
    }

    pub fn with_expansion(mut self, ef_add: usize, ef_search: usize) -> Self {
        self.expansion_add = ef_add;
        self.expansion_search = ef_search;
        self
    }

    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }
}

/// HNSW index wrapper around usearch.
pub struct HnswIndex {
    index: RwLock<Index>,
    config: HnswConfig,
}

impl HnswIndex {
    /// Create a new HNSW index or open existing one.
    pub fn open_or_create(config: HnswConfig) -> Result<Self, VectorError> {
        let index_file = config.index_path.join("hnsw.usearch");

        let options = IndexOptions {
            dimensions: config.dimension,
            metric: MetricKind::Cos, // Cosine similarity
            quantization: ScalarKind::F32,
            connectivity: config.connectivity,
            expansion_add: config.expansion_add,
            expansion_search: config.expansion_search,
            multi: false, // Single vector per key
        };

        let index = if index_file.exists() {
            info!(path = ?index_file, "Opening existing vector index");
            let idx = Index::new(&options).map_err(|e| VectorError::Index(e.to_string()))?;
            idx.load(
                index_file
                    .to_str()
                    .ok_or_else(|| VectorError::Index("Invalid path encoding".to_string()))?,
            )
            .map_err(|e| VectorError::Index(format!("Failed to load: {}", e)))?;
            idx
        } else {
            info!(path = ?index_file, dim = config.dimension, "Creating new vector index");
            std::fs::create_dir_all(&config.index_path)?;
            let idx = Index::new(&options).map_err(|e| VectorError::Index(e.to_string()))?;
            idx.reserve(config.capacity)
                .map_err(|e| VectorError::Index(e.to_string()))?;
            idx
        };

        Ok(Self {
            index: RwLock::new(index),
            config,
        })
    }

    /// Get the index file path
    pub fn index_file(&self) -> PathBuf {
        self.config.index_path.join("hnsw.usearch")
    }
}

impl VectorIndex for HnswIndex {
    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn len(&self) -> usize {
        self.index.read().unwrap().size()
    }

    #[allow(clippy::readonly_write_lock)] // usearch::Index uses interior mutability
    fn add(&mut self, id: u64, embedding: &Embedding) -> Result<(), VectorError> {
        if embedding.dimension() != self.config.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.config.dimension,
                actual: embedding.dimension(),
            });
        }

        let index = self.index.write().unwrap();
        index
            .add(id, &embedding.values)
            .map_err(|e| VectorError::Index(e.to_string()))?;

        debug!(id = id, "Added vector");
        Ok(())
    }

    fn search(&self, query: &Embedding, k: usize) -> Result<Vec<SearchResult>, VectorError> {
        if query.dimension() != self.config.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.config.dimension,
                actual: query.dimension(),
            });
        }

        let index = self.index.read().unwrap();
        let results = index
            .search(&query.values, k)
            .map_err(|e| VectorError::Index(e.to_string()))?;

        let search_results: Vec<SearchResult> = results
            .keys
            .iter()
            .zip(results.distances.iter())
            .map(|(&id, &dist)| SearchResult::new(id, 1.0 - dist)) // Convert distance to similarity
            .collect();

        debug!(k = k, found = search_results.len(), "Search complete");
        Ok(search_results)
    }

    #[allow(clippy::readonly_write_lock)] // usearch::Index uses interior mutability
    fn remove(&mut self, id: u64) -> Result<bool, VectorError> {
        let index = self.index.write().unwrap();
        let result = index
            .remove(id)
            .map_err(|e| VectorError::Index(e.to_string()))?;

        if result > 0 {
            debug!(id = id, "Removed vector");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn contains(&self, id: u64) -> bool {
        let index = self.index.read().unwrap();
        index.contains(id)
    }

    fn stats(&self) -> IndexStats {
        let index = self.index.read().unwrap();
        let size_bytes = std::fs::metadata(self.index_file())
            .map(|m| m.len())
            .unwrap_or(0);

        IndexStats {
            vector_count: index.size(),
            dimension: self.config.dimension,
            size_bytes,
            available: true,
        }
    }

    fn save(&self) -> Result<(), VectorError> {
        let index = self.index.read().unwrap();
        let path = self.index_file();
        let path_str = path
            .to_str()
            .ok_or_else(|| VectorError::Index("Invalid path encoding".to_string()))?;
        index
            .save(path_str)
            .map_err(|e| VectorError::Index(format!("Failed to save: {}", e)))?;

        info!(path = ?path, vectors = index.size(), "Saved vector index");
        Ok(())
    }

    fn clear(&mut self) -> Result<(), VectorError> {
        // Recreate empty index
        let options = IndexOptions {
            dimensions: self.config.dimension,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            connectivity: self.config.connectivity,
            expansion_add: self.config.expansion_add,
            expansion_search: self.config.expansion_search,
            multi: false,
        };

        let new_index = Index::new(&options).map_err(|e| VectorError::Index(e.to_string()))?;
        new_index
            .reserve(self.config.capacity)
            .map_err(|e| VectorError::Index(e.to_string()))?;

        *self.index.write().unwrap() = new_index;
        info!("Cleared vector index");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn random_embedding(dim: usize) -> Embedding {
        use rand::Rng;
        let mut rng = rand::rng();
        let values: Vec<f32> = (0..dim).map(|_| rng.random()).collect();
        Embedding::new(values)
    }

    #[test]
    fn test_create_index() {
        let temp = TempDir::new().unwrap();
        let config = HnswConfig::new(384, temp.path());
        let index = HnswIndex::open_or_create(config).unwrap();
        assert_eq!(index.dimension(), 384);
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_add_and_search() {
        let temp = TempDir::new().unwrap();
        let config = HnswConfig::new(64, temp.path()).with_capacity(100);
        let mut index = HnswIndex::open_or_create(config).unwrap();

        // Add some vectors
        for i in 0..10 {
            let emb = random_embedding(64);
            index.add(i, &emb).unwrap();
        }

        assert_eq!(index.len(), 10);

        // Search
        let query = random_embedding(64);
        let results = index.search(&query, 5).unwrap();
        assert_eq!(results.len(), 5);

        // Results should be sorted by score (descending)
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[test]
    fn test_save_and_load() {
        let temp = TempDir::new().unwrap();
        let config = HnswConfig::new(64, temp.path()).with_capacity(100);

        // Create and populate
        {
            let mut index = HnswIndex::open_or_create(config.clone()).unwrap();
            for i in 0..5 {
                index.add(i, &random_embedding(64)).unwrap();
            }
            index.save().unwrap();
        }

        // Reopen
        let index = HnswIndex::open_or_create(config).unwrap();
        assert_eq!(index.len(), 5);
    }

    #[test]
    fn test_dimension_mismatch() {
        let temp = TempDir::new().unwrap();
        let config = HnswConfig::new(64, temp.path());
        let mut index = HnswIndex::open_or_create(config).unwrap();

        let wrong_dim = random_embedding(32);
        let result = index.add(0, &wrong_dim);
        assert!(matches!(result, Err(VectorError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_remove() {
        let temp = TempDir::new().unwrap();
        let config = HnswConfig::new(64, temp.path()).with_capacity(100);
        let mut index = HnswIndex::open_or_create(config).unwrap();

        index.add(42, &random_embedding(64)).unwrap();
        assert!(index.contains(42));

        let removed = index.remove(42).unwrap();
        assert!(removed);
        assert!(!index.contains(42));
    }
}
