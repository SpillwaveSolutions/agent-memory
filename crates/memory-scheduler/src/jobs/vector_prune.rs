//! Vector prune scheduler job (FR-08).
//!
//! Prunes old vectors from the HNSW index based on retention config.
//! Runs according to cron schedule and respects per-level retention config.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use memory_vector::lifecycle::{
    is_protected_level, retention_map, PruneStats, VectorLifecycleConfig,
};
use tokio_util::sync::CancellationToken;
use tracing;

/// Prune function type for vector pruning.
/// Takes (age_days, level_filter) and returns count of pruned vectors.
pub type VectorPruneFn = Arc<
    dyn Fn(u64, Option<String>) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send>>
        + Send
        + Sync,
>;

/// Legacy prune function type (age_days only, no level filter).
/// Deprecated: Use VectorPruneFn instead.
pub type PruneFn =
    Arc<dyn Fn(u64) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send>> + Send + Sync>;

/// Configuration for vector prune job.
#[derive(Clone)]
pub struct VectorPruneJobConfig {
    /// Cron schedule (default: "0 3 * * *" - daily at 3 AM).
    pub cron_schedule: String,
    /// Lifecycle config.
    pub lifecycle: VectorLifecycleConfig,
    /// Whether to run dry-run first.
    pub dry_run_first: bool,
    /// Optional prune callback with level filter support.
    /// The callback receives (age_days, level_filter) and returns count of pruned vectors.
    pub prune_fn: Option<VectorPruneFn>,
}

impl std::fmt::Debug for VectorPruneJobConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorPruneJobConfig")
            .field("cron_schedule", &self.cron_schedule)
            .field("lifecycle", &self.lifecycle)
            .field("dry_run_first", &self.dry_run_first)
            .field("prune_fn", &self.prune_fn.is_some())
            .finish()
    }
}

impl Default for VectorPruneJobConfig {
    fn default() -> Self {
        Self {
            cron_schedule: "0 3 * * *".to_string(),
            lifecycle: VectorLifecycleConfig::default(),
            dry_run_first: false,
            prune_fn: None,
        }
    }
}

/// Vector prune job - prunes old vectors from HNSW index.
pub struct VectorPruneJob {
    config: VectorPruneJobConfig,
}

impl VectorPruneJob {
    pub fn new(config: VectorPruneJobConfig) -> Self {
        Self { config }
    }

    /// Create a job with a prune callback that supports per-level filtering.
    ///
    /// The callback should call `VectorIndexPipeline::prune_level(age_days, level)` and return
    /// the count of pruned vectors.
    pub fn with_prune_fn<F, Fut>(mut config: VectorPruneJobConfig, prune_fn: F) -> Self
    where
        F: Fn(u64, Option<String>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<usize, String>> + Send + 'static,
    {
        config.prune_fn = Some(Arc::new(move |age_days, level| {
            Box::pin(prune_fn(age_days, level))
        }));
        Self { config }
    }

    /// Execute the prune job.
    ///
    /// Prunes vectors per level according to retention config.
    /// Uses the shortest retention period to prune all vectors older than that age.
    pub async fn run(&self, cancel: CancellationToken) -> Result<PruneStats, String> {
        if cancel.is_cancelled() {
            return Ok(PruneStats::new());
        }

        if !self.config.lifecycle.enabled {
            tracing::debug!("Vector lifecycle disabled, skipping prune job");
            return Ok(PruneStats::new());
        }

        tracing::info!("Starting vector prune job");

        let mut total_stats = PruneStats::new();

        // Get retention map for all levels
        let retentions = retention_map(&self.config.lifecycle);

        // Process each level
        for (level, retention_days) in retentions {
            if is_protected_level(level) {
                tracing::debug!(level, "Skipping protected level");
                continue;
            }

            if cancel.is_cancelled() {
                tracing::info!("Vector prune job cancelled");
                break;
            }

            tracing::info!(
                level = level,
                retention_days = retention_days,
                "Processing level for pruning"
            );

            // Call prune callback if available
            if let Some(ref prune_fn) = self.config.prune_fn {
                match prune_fn(retention_days as u64, Some(level.to_string())).await {
                    Ok(count) => {
                        total_stats.add(level, count as u32);
                        tracing::info!(level, count, "Pruned vectors for level");
                    }
                    Err(e) => {
                        tracing::error!(level, error = %e, "Failed to prune level");
                        total_stats.errors.push(format!("{}: {}", level, e));
                    }
                }
            } else {
                // No prune function - just log what would happen
                tracing::info!(
                    level = level,
                    retention_days = retention_days,
                    "Would prune vectors older than {} days (no prune_fn configured)",
                    retention_days
                );
            }
        }

        tracing::info!(
            total_pruned = total_stats.total(),
            errors = total_stats.errors.len(),
            "Vector prune job completed"
        );

        Ok(total_stats)
    }

    /// Get job name.
    pub fn name(&self) -> &str {
        "vector_prune"
    }

    /// Get cron schedule.
    pub fn cron_schedule(&self) -> &str {
        &self.config.cron_schedule
    }

    /// Get configuration.
    pub fn config(&self) -> &VectorPruneJobConfig {
        &self.config
    }
}

/// Create vector prune job for registration with scheduler.
pub fn create_vector_prune_job(config: VectorPruneJobConfig) -> VectorPruneJob {
    VectorPruneJob::new(config)
}

/// Register the vector prune job with the scheduler.
///
/// This function registers a vector prune job that will:
/// 1. Run according to the configured schedule (default: daily at 3 AM)
/// 2. Iterate through each TOC level (segment, grip, day, week)
/// 3. Call the prune callback for each level with appropriate retention
/// 4. Skip protected levels (month, year) that should never be pruned
///
/// # Arguments
///
/// * `scheduler` - The scheduler service to register the job with
/// * `job` - Pre-configured VectorPruneJob with prune callback
///
/// # Returns
///
/// Returns `Ok(())` if the job was registered successfully.
///
/// # Example
///
/// ```ignore
/// use memory_scheduler::{SchedulerService, VectorPruneJob, VectorPruneJobConfig};
/// use memory_vector::VectorIndexPipeline;
///
/// let pipeline = Arc::new(VectorIndexPipeline::new(...));
/// let job = VectorPruneJob::with_prune_fn(
///     VectorPruneJobConfig::default(),
///     move |age_days, level| {
///         let p = Arc::clone(&pipeline);
///         async move {
///             p.prune_level(age_days, level.as_deref())
///                 .map_err(|e| e.to_string())
///         }
///     },
/// );
///
/// register_vector_prune_job(&scheduler, job).await?;
/// ```
pub async fn register_vector_prune_job(
    scheduler: &crate::SchedulerService,
    job: VectorPruneJob,
) -> Result<(), crate::SchedulerError> {
    use crate::{JitterConfig, JobOutput, OverlapPolicy, TimeoutConfig};

    let config = job.config().clone();
    let cron = convert_5field_to_6field(&config.cron_schedule);
    let job = Arc::new(job);

    scheduler
        .register_job_with_metadata(
            "vector_prune",
            &cron,
            Some("UTC"),
            OverlapPolicy::Skip,
            JitterConfig::new(60),    // Up to 60 seconds jitter
            TimeoutConfig::new(3600), // 1 hour timeout
            move || {
                let job = Arc::clone(&job);
                async move {
                    let cancel = CancellationToken::new();
                    job.run(cancel)
                        .await
                        .map(|stats| {
                            tracing::info!(
                                total = stats.total(),
                                segments = stats.segments_pruned,
                                grips = stats.grips_pruned,
                                days = stats.days_pruned,
                                weeks = stats.weeks_pruned,
                                errors = stats.errors.len(),
                                "Vector prune job completed"
                            );
                            JobOutput::new()
                                .with_prune_count(stats.total())
                                .with_metadata("segments_pruned", stats.segments_pruned.to_string())
                                .with_metadata("grips_pruned", stats.grips_pruned.to_string())
                                .with_metadata("days_pruned", stats.days_pruned.to_string())
                                .with_metadata("weeks_pruned", stats.weeks_pruned.to_string())
                                .with_metadata("error_count", stats.errors.len().to_string())
                        })
                        .map_err(|e| format!("Vector prune failed: {}", e))
                }
            },
        )
        .await?;

    tracing::info!(
        enabled = config.lifecycle.enabled,
        schedule = %config.cron_schedule,
        "Registered vector prune job"
    );
    Ok(())
}

/// Convert 5-field cron (minute hour day month weekday) to 6-field (second minute hour day month weekday).
fn convert_5field_to_6field(cron_5field: &str) -> String {
    let parts: Vec<&str> = cron_5field.split_whitespace().collect();
    if parts.len() == 5 {
        // Add "0" for seconds
        format!("0 {}", cron_5field)
    } else {
        // Already 6 fields or invalid - return as-is
        cron_5field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_job_respects_cancel() {
        let job = VectorPruneJob::new(VectorPruneJobConfig::default());
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total(), 0);
    }

    #[tokio::test]
    async fn test_job_skips_when_disabled() {
        let config = VectorPruneJobConfig {
            lifecycle: VectorLifecycleConfig::disabled(),
            ..Default::default()
        };
        let job = VectorPruneJob::new(config);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total(), 0);
    }

    #[tokio::test]
    async fn test_job_calls_prune_fn() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let prune_fn = move |_age_days: u64, _level: Option<String>| {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(5usize) // Pretend we pruned 5 vectors
            }
        };

        let config = VectorPruneJobConfig::default();
        let job = VectorPruneJob::with_prune_fn(config, prune_fn);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());

        // Should have called prune_fn for each non-protected level
        // (segment, grip, day, week = 4 levels)
        assert_eq!(call_count.load(Ordering::SeqCst), 4);

        // Total should be 4 * 5 = 20
        let stats = result.unwrap();
        assert_eq!(stats.total(), 20);
    }

    #[tokio::test]
    async fn test_job_handles_prune_error() {
        let prune_fn =
            |_age_days: u64, _level: Option<String>| async { Err("test error".to_string()) };

        let config = VectorPruneJobConfig::default();
        let job = VectorPruneJob::with_prune_fn(config, prune_fn);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert!(!stats.errors.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = VectorPruneJobConfig::default();
        assert_eq!(config.cron_schedule, "0 3 * * *");
        assert!(config.lifecycle.enabled);
        assert!(!config.dry_run_first);
        assert!(config.prune_fn.is_none());
    }

    #[test]
    fn test_job_name() {
        let job = VectorPruneJob::new(VectorPruneJobConfig::default());
        assert_eq!(job.name(), "vector_prune");
    }

    #[test]
    fn test_config_debug() {
        let config = VectorPruneJobConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("VectorPruneJobConfig"));
        assert!(debug_str.contains("prune_fn: false"));
    }
}
