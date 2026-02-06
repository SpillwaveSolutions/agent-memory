//! Background job scheduler for agent-memory daemon.
//!
//! This crate provides async job scheduling using `tokio-cron-scheduler`
//! with timezone support and graceful shutdown capabilities.
//!
//! # Features
//!
//! - **SCHED-01**: Cron-based scheduling with standard cron expression syntax
//! - **SCHED-02**: Timezone-aware scheduling via chrono-tz
//! - **SCHED-03**: Graceful shutdown via CancellationToken
//! - **SCHED-04**: Job lifecycle management (add, remove, pause)
//! - **SCHED-05**: Job status observability via JobRegistry
//! - **SCHED-06**: Overlap policy (skip/concurrent) for job execution
//! - **SCHED-07**: Jitter support for distributed scheduling
//!
//! # Example
//!
//! ```ignore
//! use memory_scheduler::{SchedulerService, SchedulerConfig, OverlapPolicy, JitterConfig};
//!
//! let config = SchedulerConfig::default();
//! let scheduler = SchedulerService::new(config).await?;
//!
//! // Register a job with overlap prevention and jitter
//! scheduler.register_job(
//!     "hourly-rollup",
//!     "0 0 * * * *",
//!     None, // Use default timezone
//!     OverlapPolicy::Skip,
//!     JitterConfig::new(30), // Up to 30 seconds jitter
//!     || async { do_rollup().await },
//! ).await?;
//!
//! // Check job status
//! let registry = scheduler.registry();
//! let status = registry.get_status("hourly-rollup");
//!
//! scheduler.start().await?;
//! ```

mod config;
mod error;
mod jitter;
mod overlap;
mod registry;
mod scheduler;

#[cfg(feature = "jobs")]
pub mod jobs;

pub use config::SchedulerConfig;
pub use error::SchedulerError;
pub use jitter::{with_jitter, JitterConfig, TimeoutConfig};
pub use overlap::{OverlapGuard, OverlapPolicy, RunGuard};
pub use registry::{JobOutput, JobRegistry, JobResult, JobStatus};
pub use scheduler::{validate_cron_expression, SchedulerService};

#[cfg(feature = "jobs")]
pub use jobs::bm25_prune::{
    create_bm25_prune_job, register_bm25_prune_job, Bm25PruneJob, Bm25PruneJobConfig,
};
#[cfg(feature = "jobs")]
pub use jobs::compaction::{create_compaction_job, CompactionJobConfig};
#[cfg(feature = "jobs")]
pub use jobs::indexing::{create_indexing_job, IndexingJobConfig};
#[cfg(feature = "jobs")]
pub use jobs::rollup::{create_rollup_jobs, RollupJobConfig};
#[cfg(feature = "jobs")]
pub use jobs::search::{create_index_commit_job, IndexCommitJobConfig};
#[cfg(feature = "jobs")]
pub use jobs::vector_prune::{
    create_vector_prune_job, register_vector_prune_job, VectorPruneJob, VectorPruneJobConfig,
};
