//! TOC rollup job definitions.
//!
//! Wraps memory_toc::rollup to schedule periodic rollups of
//! TOC nodes at different time granularities.
//!
//! # Job Schedule
//!
//! By default:
//! - Day rollup: 1 AM daily
//! - Week rollup: 2 AM Sunday
//! - Month rollup: 3 AM 1st of month
//!
//! All jobs use OverlapPolicy::Skip to prevent concurrent execution
//! of the same rollup level.

use std::sync::Arc;

use chrono::Duration;
use serde::{Deserialize, Serialize};
use tracing::info;

use memory_storage::Storage;
use memory_toc::rollup::RollupJob;
use memory_toc::summarizer::Summarizer;
use memory_types::TocLevel;

use crate::{JitterConfig, OverlapPolicy, SchedulerError, SchedulerService};

/// Configuration for TOC rollup jobs.
///
/// Defines cron schedules for day, week, and month rollups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollupJobConfig {
    /// Cron expression for day rollup (default: "0 0 1 * * *" = 1 AM daily)
    pub day_cron: String,

    /// Cron expression for week rollup (default: "0 0 2 * * 0" = 2 AM Sunday)
    pub week_cron: String,

    /// Cron expression for month rollup (default: "0 0 3 1 * *" = 3 AM 1st of month)
    pub month_cron: String,

    /// Timezone for scheduling (default: "UTC")
    pub timezone: String,

    /// Max jitter in seconds (default: 300 = 5 min)
    pub jitter_secs: u64,
}

impl Default for RollupJobConfig {
    fn default() -> Self {
        Self {
            day_cron: "0 0 1 * * *".to_string(),
            week_cron: "0 0 2 * * 0".to_string(),
            month_cron: "0 0 3 1 * *".to_string(),
            timezone: "UTC".to_string(),
            jitter_secs: 300,
        }
    }
}

/// Register all rollup jobs with the scheduler.
///
/// Creates jobs for day, week, and month rollups using the existing
/// memory_toc::rollup implementation. Each job:
/// - Uses OverlapPolicy::Skip to prevent concurrent execution
/// - Applies jitter to spread load across time
/// - Checkpoints progress for crash recovery
///
/// # Arguments
///
/// * `scheduler` - The scheduler service to register jobs with
/// * `storage` - Storage instance for TOC operations
/// * `summarizer` - Summarizer for generating rollup summaries
/// * `config` - Configuration for job schedules
///
/// # Errors
///
/// Returns error if any job registration fails (invalid cron, invalid timezone).
pub async fn create_rollup_jobs(
    scheduler: &SchedulerService,
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
    config: RollupJobConfig,
) -> Result<(), SchedulerError> {
    // Day rollup job
    let storage_day = storage.clone();
    let summarizer_day = summarizer.clone();
    scheduler
        .register_job(
            "toc_rollup_day",
            &config.day_cron,
            Some(&config.timezone),
            OverlapPolicy::Skip,
            JitterConfig::new(config.jitter_secs),
            move || {
                let storage = storage_day.clone();
                let summarizer = summarizer_day.clone();
                async move { run_day_rollup(storage, summarizer).await }
            },
        )
        .await?;

    // Week rollup job
    let storage_week = storage.clone();
    let summarizer_week = summarizer.clone();
    scheduler
        .register_job(
            "toc_rollup_week",
            &config.week_cron,
            Some(&config.timezone),
            OverlapPolicy::Skip,
            JitterConfig::new(config.jitter_secs),
            move || {
                let storage = storage_week.clone();
                let summarizer = summarizer_week.clone();
                async move { run_week_rollup(storage, summarizer).await }
            },
        )
        .await?;

    // Month rollup job
    let storage_month = storage.clone();
    let summarizer_month = summarizer.clone();
    scheduler
        .register_job(
            "toc_rollup_month",
            &config.month_cron,
            Some(&config.timezone),
            OverlapPolicy::Skip,
            JitterConfig::new(config.jitter_secs),
            move || {
                let storage = storage_month.clone();
                let summarizer = summarizer_month.clone();
                async move { run_month_rollup(storage, summarizer).await }
            },
        )
        .await?;

    info!("Registered TOC rollup jobs (day, week, month)");
    Ok(())
}

/// Run day-level rollup.
///
/// Aggregates segment nodes into day nodes. Uses 1 hour min_age
/// to avoid rolling up incomplete hours.
async fn run_day_rollup(
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
) -> Result<(), String> {
    let job = RollupJob::new(storage, summarizer, TocLevel::Day, Duration::hours(1));
    job.run()
        .await
        .map(|count| info!(count, "Day rollup complete"))
        .map_err(|e| e.to_string())
}

/// Run week-level rollup.
///
/// Aggregates day nodes into week nodes. Uses 24 hour min_age
/// to avoid rolling up incomplete days.
async fn run_week_rollup(
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
) -> Result<(), String> {
    let job = RollupJob::new(storage, summarizer, TocLevel::Week, Duration::hours(24));
    job.run()
        .await
        .map(|count| info!(count, "Week rollup complete"))
        .map_err(|e| e.to_string())
}

/// Run month-level rollup.
///
/// Aggregates week nodes into month nodes. Uses 24 hour min_age
/// to avoid rolling up incomplete weeks.
async fn run_month_rollup(
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
) -> Result<(), String> {
    let job = RollupJob::new(storage, summarizer, TocLevel::Month, Duration::hours(24));
    job.run()
        .await
        .map(|count| info!(count, "Month rollup complete"))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollup_config_default() {
        let config = RollupJobConfig::default();

        assert_eq!(config.day_cron, "0 0 1 * * *");
        assert_eq!(config.week_cron, "0 0 2 * * 0");
        assert_eq!(config.month_cron, "0 0 3 1 * *");
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.jitter_secs, 300);
    }

    #[test]
    fn test_rollup_config_serialization() {
        let config = RollupJobConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: RollupJobConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.day_cron, decoded.day_cron);
        assert_eq!(config.week_cron, decoded.week_cron);
        assert_eq!(config.month_cron, decoded.month_cron);
    }
}
