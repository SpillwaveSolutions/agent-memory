//! Topic error types.

use thiserror::Error;

/// Errors that can occur during topic operations.
#[derive(Debug, Error)]
pub enum TopicsError {
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(#[from] memory_storage::StorageError),

    /// Clustering error
    #[error("Clustering error: {0}")]
    Clustering(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Topic not found
    #[error("Topic not found: {0}")]
    NotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Feature disabled
    #[error("Topic graph is disabled")]
    Disabled,

    /// Embedding error
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Cycle detected in relationships
    #[error("Cycle detected in topic relationships")]
    CycleDetected,
}
