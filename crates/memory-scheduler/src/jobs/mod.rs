//! Predefined job implementations for common tasks.
//!
//! This module provides ready-to-use job definitions that can be
//! registered with the scheduler for periodic execution.
//!
//! # Available Jobs
//!
//! - **rollup**: TOC rollup jobs for day/week/month aggregation
//! - **compaction**: RocksDB compaction for storage optimization

pub mod compaction;
pub mod rollup;

pub use compaction::{create_compaction_job, CompactionJobConfig};
pub use rollup::{create_rollup_jobs, RollupJobConfig};
