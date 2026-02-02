//! Embedding error types.

use thiserror::Error;

/// Errors that can occur during embedding operations.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    /// Candle model error
    #[error("Candle error: {0}")]
    Candle(#[from] candle_core::Error),

    /// Tokenizer error
    #[error("Tokenizer error: {0}")]
    Tokenizer(String),

    /// Model file not found
    #[error("Model file not found: {0}")]
    ModelNotFound(String),

    /// Download error
    #[error("Failed to download model: {0}")]
    Download(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Dimension mismatch
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}
