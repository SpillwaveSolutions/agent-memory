//! # memory-orchestrator
//!
//! Retrieval orchestration layer for agent-memory.
//! Adds query expansion, RRF fusion across all indexes,
//! and optional LLM reranking on top of `memory-retrieval`.

pub mod context_builder;
pub mod expand;
pub mod fusion;
pub mod orchestrator;
pub mod rerank;
pub mod types;

pub use fusion::{fuse, fuse_weighted, FusedResult};
pub use orchestrator::{MemoryOrchestrator, OrchestratorOutput};
pub use rerank::{
    Completer, CrossEncoderReranker, HeuristicReranker, LlmReranker, RerankError, RerankedResult,
    Reranker,
};
pub use types::{MemoryContext, OrchestratorConfig, RankedResult, RerankMode};
