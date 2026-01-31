//! gRPC service implementation for agent-memory.
//!
//! Provides:
//! - IngestEvent RPC for event ingestion (ING-01)
//! - TOC navigation RPCs (QRY-01, QRY-02, QRY-03)
//! - Event retrieval RPCs (QRY-04, QRY-05)
//! - Health check endpoint (GRPC-03)
//! - Reflection endpoint for debugging (GRPC-04)

pub mod ingest;
pub mod query;
pub mod server;

pub mod pb {
    tonic::include_proto!("memory");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("memory_descriptor");
}

pub use ingest::MemoryServiceImpl;
pub use server::{run_server, run_server_with_shutdown};
