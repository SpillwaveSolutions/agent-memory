//! Indexing pipeline for agent-memory system.
//!
//! This crate provides the infrastructure for consuming outbox entries
//! and updating search indexes (BM25 and vector).
//!
//! ## Key Components
//!
//! - [`IndexCheckpoint`]: Tracks indexing progress for crash recovery
//! - [`IndexType`]: Distinguishes between BM25, vector, and combined indexes
//! - [`IndexingError`]: Error types for indexing operations
//! - [`IndexUpdater`]: Trait for index-specific update operations
//! - [`Bm25IndexUpdater`]: BM25 full-text search updater using Tantivy
//! - [`VectorIndexUpdater`]: Vector similarity search updater using HNSW
//! - [`IndexingPipeline`]: Coordinates multiple updaters with checkpointing
//!
//! ## Architecture
//!
//! The indexing pipeline follows the outbox pattern:
//! 1. Events are written with outbox entries atomically
//! 2. This crate consumes outbox entries in sequence order
//! 3. Each [`IndexUpdater`] processes entries for its index type
//! 4. Checkpoints track progress for crash recovery
//! 5. After processing, outbox entries can be cleaned up
//!
//! ## Example
//!
//! ```ignore
//! use memory_indexing::{IndexingPipeline, PipelineConfig, Bm25IndexUpdater, VectorIndexUpdater};
//!
//! let mut pipeline = IndexingPipeline::new(storage, PipelineConfig::default());
//! pipeline.add_updater(Box::new(bm25_updater));
//! pipeline.add_updater(Box::new(vector_updater));
//! pipeline.load_checkpoints()?;
//!
//! // Process until caught up
//! let result = pipeline.process_until_caught_up(100)?;
//!
//! // Clean up processed entries
//! pipeline.cleanup_outbox()?;
//! ```

pub mod bm25_updater;
pub mod checkpoint;
pub mod error;
pub mod pipeline;
pub mod rebuild;
pub mod updater;
pub mod vector_updater;

pub use bm25_updater::Bm25IndexUpdater;
pub use checkpoint::{IndexCheckpoint, IndexType};
pub use error::IndexingError;
pub use pipeline::{IndexingPipeline, PipelineConfig, ProcessResult};
pub use rebuild::{
    iter_all_grips, iter_all_toc_nodes, rebuild_bm25_index, rebuild_vector_index,
    LoggingProgressCallback, NoOpProgressCallback, ProgressCallback, RebuildConfig,
    RebuildProgress, RebuildResult,
};
pub use updater::{IndexUpdater, UpdateResult};
pub use vector_updater::VectorIndexUpdater;
