//! Predefined job implementations for common tasks.
//!
//! This module provides ready-to-use job definitions that can be
//! registered with the scheduler for periodic execution.
//!
//! # Available Jobs
//!
//! - **rollup**: TOC rollup jobs for day/week/month aggregation
//! - **compaction**: RocksDB compaction for storage optimization
//! - **search**: Search index commit job for making documents searchable
//! - **indexing**: Outbox indexing job for processing new entries into indexes
//! - **vector_prune**: Vector index lifecycle pruning (FR-08)
//! - **bm25_prune**: BM25 index lifecycle pruning (FR-09)

pub mod compaction;
pub mod rollup;

#[cfg(feature = "jobs")]
pub mod bm25_prune;
#[cfg(feature = "jobs")]
pub mod indexing;
#[cfg(feature = "jobs")]
pub mod search;
#[cfg(feature = "jobs")]
pub mod vector_prune;

pub use compaction::{create_compaction_job, CompactionJobConfig};
pub use rollup::{create_rollup_jobs, RollupJobConfig};

#[cfg(feature = "jobs")]
pub use bm25_prune::{create_bm25_prune_job, Bm25PruneJob, Bm25PruneJobConfig};
#[cfg(feature = "jobs")]
pub use indexing::{create_indexing_job, IndexingJobConfig};
#[cfg(feature = "jobs")]
pub use search::{create_index_commit_job, IndexCommitJobConfig};
#[cfg(feature = "jobs")]
pub use vector_prune::{create_vector_prune_job, VectorPruneJob, VectorPruneJobConfig};
