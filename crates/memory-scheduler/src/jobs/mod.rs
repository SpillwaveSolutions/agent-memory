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

pub mod compaction;
pub mod rollup;

#[cfg(feature = "jobs")]
pub mod indexing;
#[cfg(feature = "jobs")]
pub mod search;

pub use compaction::{create_compaction_job, CompactionJobConfig};
pub use rollup::{create_rollup_jobs, RollupJobConfig};

#[cfg(feature = "jobs")]
pub use indexing::{create_indexing_job, IndexingJobConfig};
#[cfg(feature = "jobs")]
pub use search::{create_index_commit_job, IndexCommitJobConfig};
