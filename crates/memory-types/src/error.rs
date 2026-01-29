//! Error types for the agent-memory system.

use thiserror::Error;

/// Unified error type for memory operations.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
