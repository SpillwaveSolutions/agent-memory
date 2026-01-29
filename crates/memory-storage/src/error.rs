//! Storage layer error types.

use thiserror::Error;

/// Errors that can occur in the storage layer
#[derive(Error, Debug)]
pub enum StorageError {
    /// RocksDB operation failed
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    /// Column family not found
    #[error("Column family not found: {0}")]
    ColumnFamilyNotFound(String),

    /// Key encoding/decoding error
    #[error("Key error: {0}")]
    Key(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Event not found
    #[error("Event not found: {0}")]
    NotFound(String),
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}
