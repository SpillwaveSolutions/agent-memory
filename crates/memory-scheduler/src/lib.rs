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
//! - **SCHED-05**: Job status observability
//!
//! # Example
//!
//! ```ignore
//! use memory_scheduler::{SchedulerService, SchedulerConfig};
//!
//! let config = SchedulerConfig::default();
//! let scheduler = SchedulerService::new(config).await?;
//!
//! scheduler.add_cron_job(
//!     "hourly-rollup",
//!     "0 0 * * * *",
//!     None, // Use default timezone
//!     || async { do_rollup().await },
//! ).await?;
//!
//! scheduler.start().await?;
//! ```

mod config;
mod error;
mod scheduler;

pub use config::SchedulerConfig;
pub use error::SchedulerError;
pub use scheduler::SchedulerService;
