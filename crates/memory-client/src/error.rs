//! Error types for the memory client.

use thiserror::Error;

/// Errors that can occur when using the memory client.
#[derive(Error, Debug)]
pub enum ClientError {
    /// Failed to connect to the daemon
    #[error("Connection failed: {0}")]
    Connection(#[from] tonic::transport::Error),

    /// RPC call failed
    #[error("RPC failed: {0}")]
    Rpc(#[from] tonic::Status),

    /// Serialization/deserialization failed
    #[error("Serialization failed: {0}")]
    Serialization(String),

    /// Invalid endpoint URL
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
}
