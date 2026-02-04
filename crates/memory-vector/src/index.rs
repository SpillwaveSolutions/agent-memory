//! Vector index trait and types.
//!
//! Defines the interface for vector similarity search.

use crate::error::VectorError;
use memory_embeddings::Embedding;

/// Result of a vector search
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Internal vector ID
    pub vector_id: u64,
    /// Distance/similarity score (lower = more similar for L2, higher = more similar for cosine)
    pub score: f32,
}

impl SearchResult {
    pub fn new(vector_id: u64, score: f32) -> Self {
        Self { vector_id, score }
    }
}

/// Index statistics
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Number of vectors in the index
    pub vector_count: usize,
    /// Embedding dimension
    pub dimension: usize,
    /// Index file size in bytes
    pub size_bytes: u64,
    /// Whether index is available for search
    pub available: bool,
}

/// Trait for vector indexes.
///
/// Implementations must be thread-safe for concurrent read access.
pub trait VectorIndex: Send + Sync {
    /// Get the embedding dimension
    fn dimension(&self) -> usize;

    /// Get the number of vectors in the index
    fn len(&self) -> usize;

    /// Check if the index is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Add a vector with the given ID.
    /// Returns error if ID already exists.
    fn add(&mut self, id: u64, embedding: &Embedding) -> Result<(), VectorError>;

    /// Add multiple vectors in batch.
    fn add_batch(&mut self, vectors: &[(u64, Embedding)]) -> Result<(), VectorError> {
        for (id, emb) in vectors {
            self.add(*id, emb)?;
        }
        Ok(())
    }

    /// Search for k nearest neighbors.
    /// Returns results sorted by similarity (best first).
    fn search(&self, query: &Embedding, k: usize) -> Result<Vec<SearchResult>, VectorError>;

    /// Remove a vector by ID.
    fn remove(&mut self, id: u64) -> Result<bool, VectorError>;

    /// Check if a vector ID exists
    fn contains(&self, id: u64) -> bool;

    /// Get index statistics
    fn stats(&self) -> IndexStats;

    /// Save index to disk
    fn save(&self) -> Result<(), VectorError>;

    /// Clear all vectors from the index
    fn clear(&mut self) -> Result<(), VectorError>;
}
