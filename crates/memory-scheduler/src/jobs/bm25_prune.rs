//! BM25 prune scheduler job (FR-09).
//!
//! Prunes old documents from the Tantivy BM25 index based on retention config.
//! DISABLED by default per PRD "append-only, no eviction" philosophy.
//! Runs according to cron schedule and respects per-level retention config.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use memory_search::lifecycle::{
    is_protected_level, retention_map, Bm25LifecycleConfig, Bm25MaintenanceConfig, Bm25PruneStats,
};
use tokio_util::sync::CancellationToken;
use tracing;

/// Prune function type for BM25 pruning.
/// Takes (age_days, level_filter, dry_run) and returns prune stats.
pub type Bm25PruneFn = Arc<
    dyn Fn(
            u64,
            Option<String>,
            bool,
        ) -> Pin<Box<dyn Future<Output = Result<Bm25PruneStats, String>> + Send>>
        + Send
        + Sync,
>;

/// Configuration for BM25 prune job.
#[derive(Clone)]
pub struct Bm25PruneJobConfig {
    /// Lifecycle config (includes enabled flag).
    pub lifecycle: Bm25LifecycleConfig,
    /// Maintenance config (includes schedule).
    pub maintenance: Bm25MaintenanceConfig,
    /// Optional prune callback - if None, job logs but doesn't prune.
    pub prune_fn: Option<Bm25PruneFn>,
}

impl std::fmt::Debug for Bm25PruneJobConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bm25PruneJobConfig")
            .field("lifecycle", &self.lifecycle)
            .field("maintenance", &self.maintenance)
            .field("prune_fn", &self.prune_fn.is_some())
            .finish()
    }
}

impl Default for Bm25PruneJobConfig {
    fn default() -> Self {
        Self {
            lifecycle: Bm25LifecycleConfig::default(), // enabled: false by default
            maintenance: Bm25MaintenanceConfig::default(),
            prune_fn: None,
        }
    }
}

/// BM25 prune job - prunes old documents from Tantivy index.
pub struct Bm25PruneJob {
    config: Bm25PruneJobConfig,
}

impl Bm25PruneJob {
    pub fn new(config: Bm25PruneJobConfig) -> Self {
        Self { config }
    }

    /// Create a job with a prune callback.
    ///
    /// The callback should call `SearchIndexer::prune_and_commit()` and return
    /// the prune statistics.
    pub fn with_prune_fn<F, Fut>(mut config: Bm25PruneJobConfig, prune_fn: F) -> Self
    where
        F: Fn(u64, Option<String>, bool) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Bm25PruneStats, String>> + Send + 'static,
    {
        config.prune_fn = Some(Arc::new(move |age_days, level, dry_run| {
            Box::pin(prune_fn(age_days, level, dry_run))
        }));
        Self { config }
    }

    /// Execute the prune job.
    ///
    /// Prunes documents per level according to retention config.
    pub async fn run(&self, cancel: CancellationToken) -> Result<Bm25PruneStats, String> {
        if cancel.is_cancelled() {
            return Ok(Bm25PruneStats::new());
        }

        if !self.config.lifecycle.enabled {
            tracing::debug!("BM25 lifecycle disabled, skipping prune job");
            return Ok(Bm25PruneStats::new());
        }

        tracing::info!("Starting BM25 prune job");

        let mut total_stats = Bm25PruneStats::new();

        // Get retention map for all levels
        let retentions = retention_map(&self.config.lifecycle);

        // Process each level
        for (level, retention_days) in retentions {
            if is_protected_level(level) {
                tracing::debug!(level, "Skipping protected level");
                continue;
            }

            if cancel.is_cancelled() {
                tracing::info!("BM25 prune job cancelled");
                break;
            }

            tracing::info!(
                level = level,
                retention_days = retention_days,
                "Processing level for BM25 pruning"
            );

            // Call prune callback if available
            if let Some(ref prune_fn) = self.config.prune_fn {
                match prune_fn(retention_days as u64, Some(level.to_string()), false).await {
                    Ok(level_stats) => {
                        // Merge level stats into total
                        total_stats.segments_pruned += level_stats.segments_pruned;
                        total_stats.grips_pruned += level_stats.grips_pruned;
                        total_stats.days_pruned += level_stats.days_pruned;
                        total_stats.weeks_pruned += level_stats.weeks_pruned;
                        tracing::info!(
                            level,
                            count = level_stats.total(),
                            "Pruned documents for level"
                        );
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
                    "Would prune documents older than {} days (no prune_fn configured)",
                    retention_days
                );
            }
        }

        // Mark if optimization was requested
        if self.config.maintenance.optimize_after_prune && total_stats.total() > 0 {
            total_stats.optimized = true;
            tracing::info!("Index optimization would be triggered after prune");
        }

        tracing::info!(
            total_pruned = total_stats.total(),
            errors = total_stats.errors.len(),
            optimized = total_stats.optimized,
            "BM25 prune job completed"
        );

        Ok(total_stats)
    }

    /// Get job name.
    pub fn name(&self) -> &str {
        "bm25_prune"
    }

    /// Get cron schedule.
    pub fn cron_schedule(&self) -> &str {
        &self.config.maintenance.prune_schedule
    }

    /// Get configuration.
    pub fn config(&self) -> &Bm25PruneJobConfig {
        &self.config
    }
}

/// Create BM25 prune job for registration with scheduler.
pub fn create_bm25_prune_job(config: Bm25PruneJobConfig) -> Bm25PruneJob {
    Bm25PruneJob::new(config)
}

/// Register the BM25 prune job with the scheduler.
///
/// This function registers a BM25 prune job that will:
/// 1. Run according to the maintenance schedule (default: daily at 3 AM)
/// 2. Iterate through each TOC level (segment, grip, day, week)
/// 3. Call the prune callback for each level with appropriate retention
/// 4. Skip protected levels (month, year) that should never be pruned
///
/// # Arguments
///
/// * `scheduler` - The scheduler service to register the job with
/// * `job` - Pre-configured Bm25PruneJob with prune callback
///
/// # Returns
///
/// Returns `Ok(())` if the job was registered successfully.
///
/// # Example
///
/// ```ignore
/// use memory_scheduler::{SchedulerService, Bm25PruneJob, Bm25PruneJobConfig};
/// use memory_search::SearchIndexer;
///
/// let indexer = Arc::new(SearchIndexer::new(&index)?);
/// let job = Bm25PruneJob::with_prune_fn(
///     Bm25PruneJobConfig::default(),
///     move |age_days, level, dry_run| {
///         let idx = Arc::clone(&indexer);
///         async move {
///             idx.prune_and_commit(age_days, level.as_deref(), dry_run)
///                 .map_err(|e| e.to_string())
///         }
///     },
/// );
///
/// register_bm25_prune_job(&scheduler, job).await?;
/// ```
pub async fn register_bm25_prune_job(
    scheduler: &crate::SchedulerService,
    job: Bm25PruneJob,
) -> Result<(), crate::SchedulerError> {
    use crate::{JitterConfig, JobOutput, OverlapPolicy, TimeoutConfig};

    let config = job.config().clone();
    let cron = convert_5field_to_6field(&config.maintenance.prune_schedule);
    let job = Arc::new(job);

    scheduler
        .register_job_with_metadata(
            "bm25_prune",
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
                                "BM25 prune job completed"
                            );
                            JobOutput::new()
                                .with_prune_count(stats.total())
                                .with_metadata("segments_pruned", stats.segments_pruned.to_string())
                                .with_metadata("grips_pruned", stats.grips_pruned.to_string())
                                .with_metadata("days_pruned", stats.days_pruned.to_string())
                                .with_metadata("weeks_pruned", stats.weeks_pruned.to_string())
                                .with_metadata("error_count", stats.errors.len().to_string())
                        })
                        .map_err(|e| format!("BM25 prune failed: {}", e))
                }
            },
        )
        .await?;

    tracing::info!(
        enabled = config.lifecycle.enabled,
        schedule = %config.maintenance.prune_schedule,
        "Registered BM25 prune job"
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
    async fn test_job_disabled_by_default() {
        let config = Bm25PruneJobConfig::default();
        assert!(!config.lifecycle.enabled); // MUST be disabled by default

        let job = Bm25PruneJob::new(config);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total(), 0);
    }

    #[tokio::test]
    async fn test_job_respects_cancel() {
        let config = Bm25PruneJobConfig {
            lifecycle: Bm25LifecycleConfig::enabled(),
            ..Default::default()
        };
        let job = Bm25PruneJob::new(config);
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total(), 0);
    }

    #[tokio::test]
    async fn test_job_calls_prune_fn() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let prune_fn = move |_age_days: u64, _level: Option<String>, _dry_run: bool| {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                let mut stats = Bm25PruneStats::new();
                stats.add("segment", 3);
                Ok(stats)
            }
        };

        let config = Bm25PruneJobConfig {
            lifecycle: Bm25LifecycleConfig::enabled(),
            ..Default::default()
        };
        let job = Bm25PruneJob::with_prune_fn(config, prune_fn);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());

        // Should have called prune_fn for each non-protected level
        // (segment, grip, day, week = 4 levels)
        assert_eq!(call_count.load(Ordering::SeqCst), 4);

        // Each call adds 3 to segments_pruned
        let stats = result.unwrap();
        assert_eq!(stats.segments_pruned, 12); // 4 * 3
    }

    #[tokio::test]
    async fn test_job_handles_prune_error() {
        let prune_fn = |_age_days: u64, _level: Option<String>, _dry_run: bool| async {
            Err("test error".to_string())
        };

        let config = Bm25PruneJobConfig {
            lifecycle: Bm25LifecycleConfig::enabled(),
            ..Default::default()
        };
        let job = Bm25PruneJob::with_prune_fn(config, prune_fn);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert!(!stats.errors.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = Bm25PruneJobConfig::default();
        assert!(!config.lifecycle.enabled);
        assert_eq!(config.maintenance.prune_schedule, "0 3 * * *");
        assert!(config.maintenance.optimize_after_prune);
        assert!(config.prune_fn.is_none());
    }

    #[test]
    fn test_job_name() {
        let job = Bm25PruneJob::new(Bm25PruneJobConfig::default());
        assert_eq!(job.name(), "bm25_prune");
    }

    #[test]
    fn test_job_cron_schedule() {
        let job = Bm25PruneJob::new(Bm25PruneJobConfig::default());
        assert_eq!(job.cron_schedule(), "0 3 * * *");
    }

    #[test]
    fn test_config_debug() {
        let config = Bm25PruneJobConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Bm25PruneJobConfig"));
        assert!(debug_str.contains("prune_fn: false"));
    }
}
