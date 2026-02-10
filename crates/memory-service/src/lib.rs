//! gRPC service implementation for agent-memory.
//!
//! Provides:
//! - IngestEvent RPC for event ingestion (ING-01)
//! - Query RPCs for TOC navigation (QRY-01 through QRY-05)
//! - Scheduler RPCs for job status and control (SCHED-05)
//! - Teleport search RPC for BM25 keyword search (TEL-01 through TEL-04)
//! - Vector search RPCs for semantic search (VEC-01 through VEC-03)
//! - Topic graph RPCs for topic navigation (TOPIC-08)
//! - Health check endpoint (GRPC-03)
//! - Reflection endpoint for debugging (GRPC-04)

pub mod agents;
pub mod hybrid;
pub mod ingest;
pub mod novelty;
pub mod query;
pub mod retrieval;
pub mod scheduler_service;
pub mod search_service;
pub mod server;
pub mod teleport_service;
pub mod topics;
pub mod vector;

pub mod pb {
    tonic::include_proto!("memory");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("memory_descriptor");
}

pub use agents::AgentDiscoveryHandler;
pub use hybrid::HybridSearchHandler;
pub use ingest::MemoryServiceImpl;
pub use novelty::{NoveltyChecker, NoveltyMetrics, NoveltyMetricsSnapshot};
pub use retrieval::RetrievalHandler;
pub use scheduler_service::SchedulerGrpcService;
pub use server::{run_server, run_server_with_scheduler, run_server_with_shutdown};
pub use topics::{TopicGraphHandler, TopicGraphStatus, TopicSearchResult};
pub use vector::{VectorSearchResult, VectorTeleportHandler};
