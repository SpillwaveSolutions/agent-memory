//! gRPC client helper for connecting to the memory daemon.
//!
//! Placeholder -- will be fully implemented in Task 2.

use anyhow::Context;
use memory_client::MemoryClient;

/// Connect to the memory daemon, returning an actionable error if it is not running.
#[allow(dead_code)] // Used by command implementations (added in subsequent plans)
pub async fn connect_client(endpoint: &str) -> anyhow::Result<MemoryClient> {
    MemoryClient::connect(endpoint).await.context(format!(
        "memory daemon not running -- start with: memory-daemon start (endpoint: {})",
        endpoint
    ))
}
