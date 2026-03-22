//! gRPC client helper for connecting to the memory daemon.

use anyhow::Context;
use memory_client::MemoryClient;

/// Connect to the memory daemon, returning an actionable error if it is not running.
///
/// Wraps `MemoryClient::connect` with a user-friendly error message that
/// includes the endpoint and instructions for starting the daemon.
#[allow(dead_code)] // Used by command implementations (added in subsequent plans)
pub async fn connect_client(endpoint: &str) -> anyhow::Result<MemoryClient> {
    MemoryClient::connect(endpoint).await.context(format!(
        "memory daemon not running -- start with: memory-daemon start (endpoint: {endpoint})"
    ))
}
