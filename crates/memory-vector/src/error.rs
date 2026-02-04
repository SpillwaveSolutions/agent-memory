//! Vector index error types.

use thiserror::Error;

/// Errors that can occur during vector operations.
#[derive(Debug, Error)]
pub enum VectorError {
    /// usearch index error
    #[error("Index error: {0}")]
    Index(String),

    /// Dimension mismatch
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Vector not found
    #[error("Vector not found: {0}")]
    NotFound(u64),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// RocksDB error
    #[error("Database error: {0}")]
    Database(#[from] rocksdb::Error),

    /// Index is full
    #[error("Index capacity reached: {0}")]
    CapacityReached(usize),

    /// Index not initialized
    #[error("Index not initialized")]
    NotInitialized,

    /// Embedding error
    #[error("Embedding error: {0}")]
    Embedding(#[from] memory_embeddings::EmbeddingError),
}
