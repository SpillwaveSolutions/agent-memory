//! # memory-service
//!
//! gRPC service implementation for the Agent Memory system.
//!
//! This crate implements the MemoryService gRPC interface:
//! - IngestEvent: Accept new events from agent hooks
//! - GetTocRoot: Return top-level time periods
//! - GetNode: Drill into specific TOC nodes
//! - ExpandGrip: Get source events for a grip
//! - GetEvents: Retrieve raw events by time range
//!
//! ## Usage
//!
//! ```rust,ignore
//! use memory_service::MemoryServiceImpl;
//!
//! let service = MemoryServiceImpl::new(storage);
//! Server::builder()
//!     .add_service(MemoryServiceServer::new(service))
//!     .serve(addr)
//!     .await?;
//! ```

use memory_storage::Storage;

/// Placeholder gRPC service implementation.
/// Will be fully implemented in Phase 1, Plan 03.
pub struct MemoryServiceImpl {
    #[allow(dead_code)]
    storage: Storage,
}

impl MemoryServiceImpl {
    /// Create a new service instance with the given storage backend.
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_test() {
        // Placeholder test to verify crate compiles
        let storage = Storage::new();
        let _service = MemoryServiceImpl::new(storage);
        assert!(true);
    }
}
