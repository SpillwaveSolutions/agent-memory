//! gRPC service implementation for agent-memory.
//!
//! Provides:
//! - IngestEvent RPC for event ingestion (ING-01)
//! - Query RPCs for TOC navigation (QRY-01 through QRY-05)
//! - Scheduler RPCs for job status and control (SCHED-05)
//! - Health check endpoint (GRPC-03)
//! - Reflection endpoint for debugging (GRPC-04)

pub mod ingest;
pub mod query;
pub mod scheduler_service;
pub mod server;

pub mod pb {
    tonic::include_proto!("memory");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("memory_descriptor");
}

pub use ingest::MemoryServiceImpl;
pub use scheduler_service::SchedulerGrpcService;
pub use server::{run_server, run_server_with_shutdown};
