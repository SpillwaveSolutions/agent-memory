//! Error types for the indexing pipeline.

use memory_embeddings::EmbeddingError;
use memory_search::SearchError;
use memory_storage::StorageError;
use memory_vector::VectorError;
use thiserror::Error;

/// Errors that can occur in the indexing pipeline
#[derive(Error, Debug)]
pub enum IndexingError {
    /// Storage operation failed
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Checkpoint load/save issues
    #[error("Checkpoint error: {0}")]
    Checkpoint(String),

    /// JSON encoding/decoding errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Generic index operation error
    #[error("Index error: {0}")]
    Index(String),

    /// BM25 search index error
    #[error("Search error: {0}")]
    Search(#[from] SearchError),

    /// Vector index error
    #[error("Vector error: {0}")]
    Vector(#[from] VectorError),

    /// Embedding generation error
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
}

impl From<serde_json::Error> for IndexingError {
    fn from(err: serde_json::Error) -> Self {
        IndexingError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = IndexingError::Checkpoint("failed to load".to_string());
        assert_eq!(err.to_string(), "Checkpoint error: failed to load");

        let err = IndexingError::Serialization("invalid json".to_string());
        assert_eq!(err.to_string(), "Serialization error: invalid json");

        let err = IndexingError::Index("index corrupted".to_string());
        assert_eq!(err.to_string(), "Index error: index corrupted");
    }

    #[test]
    fn test_from_serde_error() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let indexing_err: IndexingError = json_err.into();
        assert!(matches!(indexing_err, IndexingError::Serialization(_)));
    }
}
