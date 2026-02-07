//! # memory-vector
//!
//! Vector index for Agent Memory using HNSW algorithm.
//!
//! This crate provides semantic similarity search by storing embeddings
//! in an HNSW (Hierarchical Navigable Small World) index via usearch.
//!
//! ## Features
//! - usearch-powered HNSW index with mmap persistence
//! - O(log n) approximate nearest neighbor search
//! - Metadata storage linking vector IDs to document IDs
//! - Configurable HNSW parameters (M, ef_construction, ef_search)
//!
//! ## Requirements
//! - FR-02: HNSW index via usearch
//! - FR-03: VectorTeleport RPC support
//! - Index lifecycle: prune/rebuild operations

pub mod error;
pub mod hnsw;
pub mod index;
pub mod lifecycle;
pub mod metadata;
pub mod pipeline;

pub use error::VectorError;
pub use hnsw::{HnswConfig, HnswIndex};
pub use index::{IndexStats, SearchResult, VectorIndex};
pub use lifecycle::{is_protected_level, PruneStats, VectorLifecycleConfig};
pub use metadata::{DocType, VectorEntry, VectorMetadata, CF_VECTOR_META};
pub use pipeline::{
    IndexableItem, IndexingStats, PipelineConfig, VectorIndexPipeline, VECTOR_INDEX_CHECKPOINT,
};
