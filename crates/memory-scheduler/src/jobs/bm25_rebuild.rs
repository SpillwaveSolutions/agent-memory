//! BM25 rebuild scheduler job for lifecycle automation.
//!
//! Rebuilds the BM25 index with level filtering, removing fine-grained
//! segment/grip docs after rollup has created day+ level summaries.
//! DISABLED by default - opt-in via `[lifecycle.bm25]` config section.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tracing;

/// Rebuild function type for BM25 rebuild.
/// Takes min_level filter and returns count of documents removed.
pub type Bm25RebuildFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<u32, String>> + Send>> + Send + Sync>;

/// Configuration for BM25 rebuild job.
#[derive(Clone)]
pub struct Bm25RebuildJobConfig {
    /// Cron schedule (default: "0 4 * * 0" - weekly Sunday 4 AM).
    pub cron_schedule: String,
    /// Minimum level to keep (default: "day").
    pub min_level: String,
    /// Whether the job is enabled (default: false).
    pub enabled: bool,
    /// Optional rebuild callback.
    pub rebuild_fn: Option<Bm25RebuildFn>,
}

impl std::fmt::Debug for Bm25RebuildJobConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bm25RebuildJobConfig")
            .field("cron_schedule", &self.cron_schedule)
            .field("min_level", &self.min_level)
            .field("enabled", &self.enabled)
            .field("rebuild_fn", &self.rebuild_fn.is_some())
            .finish()
    }
}

impl Default for Bm25RebuildJobConfig {
    fn default() -> Self {
        Self {
            cron_schedule: "0 4 * * 0".to_string(),
            min_level: "day".to_string(),
            enabled: false,
            rebuild_fn: None,
        }
    }
}

/// BM25 rebuild job - rebuilds BM25 index with level filtering.
pub struct Bm25RebuildJob {
    config: Bm25RebuildJobConfig,
}

impl Bm25RebuildJob {
    pub fn new(config: Bm25RebuildJobConfig) -> Self {
        Self { config }
    }

    /// Create a job with a rebuild callback.
    ///
    /// The callback should call `SearchIndexer::rebuild_with_filter()` and return
    /// the count of removed documents.
    pub fn with_rebuild_fn<F, Fut>(mut config: Bm25RebuildJobConfig, rebuild_fn: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<u32, String>> + Send + 'static,
    {
        config.rebuild_fn = Some(Arc::new(move |min_level| Box::pin(rebuild_fn(min_level))));
        Self { config }
    }

    /// Execute the rebuild job.
    pub async fn run(&self, cancel: CancellationToken) -> Result<u32, String> {
        if cancel.is_cancelled() {
            return Ok(0);
        }

        if !self.config.enabled {
            tracing::debug!("BM25 rebuild job disabled, skipping");
            return Ok(0);
        }

        tracing::info!(
            min_level = %self.config.min_level,
            "Starting BM25 rebuild job"
        );

        if let Some(ref rebuild_fn) = self.config.rebuild_fn {
            let result = rebuild_fn(self.config.min_level.clone()).await;
            match &result {
                Ok(count) => {
                    tracing::info!(removed = count, "BM25 rebuild job completed");
                }
                Err(e) => {
                    tracing::error!(error = %e, "BM25 rebuild job failed");
                }
            }
            result
        } else {
            tracing::info!(
                min_level = %self.config.min_level,
                "Would rebuild BM25 index (no rebuild_fn configured)"
            );
            Ok(0)
        }
    }

    /// Get job name.
    pub fn name(&self) -> &str {
        "bm25_rebuild"
    }

    /// Get cron schedule.
    pub fn cron_schedule(&self) -> &str {
        &self.config.cron_schedule
    }

    /// Get configuration.
    pub fn config(&self) -> &Bm25RebuildJobConfig {
        &self.config
    }
}

/// Create BM25 rebuild job for registration with scheduler.
pub fn create_bm25_rebuild_job(config: Bm25RebuildJobConfig) -> Bm25RebuildJob {
    Bm25RebuildJob::new(config)
}

/// Register the BM25 rebuild job with the scheduler.
pub async fn register_bm25_rebuild_job(
    scheduler: &crate::SchedulerService,
    job: Bm25RebuildJob,
) -> Result<(), crate::SchedulerError> {
    use crate::{JitterConfig, JobOutput, OverlapPolicy, TimeoutConfig};

    let config = job.config().clone();

    // Convert 5-field cron to 6-field
    let cron = convert_5field_to_6field(&config.cron_schedule);
    let job = Arc::new(job);

    scheduler
        .register_job_with_metadata(
            "bm25_rebuild",
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
                        .map(|count| {
                            tracing::info!(removed = count, "BM25 rebuild job completed");
                            JobOutput::new()
                                .with_prune_count(count)
                                .with_metadata("documents_removed", count.to_string())
                        })
                        .map_err(|e| format!("BM25 rebuild failed: {}", e))
                }
            },
        )
        .await?;

    tracing::info!(
        enabled = config.enabled,
        schedule = %config.cron_schedule,
        min_level = %config.min_level,
        "Registered BM25 rebuild job"
    );
    Ok(())
}

/// Convert 5-field cron to 6-field (add seconds).
fn convert_5field_to_6field(cron_5field: &str) -> String {
    let parts: Vec<&str> = cron_5field.split_whitespace().collect();
    if parts.len() == 5 {
        format!("0 {}", cron_5field)
    } else {
        cron_5field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_job_disabled_by_default() {
        let config = Bm25RebuildJobConfig::default();
        assert!(!config.enabled);

        let job = Bm25RebuildJob::new(config);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_job_respects_cancel() {
        let config = Bm25RebuildJobConfig {
            enabled: true,
            ..Default::default()
        };
        let job = Bm25RebuildJob::new(config);
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_job_calls_rebuild_fn() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let rebuild_fn = move |_min_level: String| {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(42u32)
            }
        };

        let config = Bm25RebuildJobConfig {
            enabled: true,
            ..Default::default()
        };
        let job = Bm25RebuildJob::with_rebuild_fn(config, rebuild_fn);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_job_handles_rebuild_error() {
        let rebuild_fn = |_min_level: String| async { Err("test error".to_string()) };

        let config = Bm25RebuildJobConfig {
            enabled: true,
            ..Default::default()
        };
        let job = Bm25RebuildJob::with_rebuild_fn(config, rebuild_fn);
        let cancel = CancellationToken::new();

        let result = job.run(cancel).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_default_config() {
        let config = Bm25RebuildJobConfig::default();
        assert_eq!(config.cron_schedule, "0 4 * * 0");
        assert_eq!(config.min_level, "day");
        assert!(!config.enabled);
        assert!(config.rebuild_fn.is_none());
    }

    #[test]
    fn test_job_name() {
        let job = Bm25RebuildJob::new(Bm25RebuildJobConfig::default());
        assert_eq!(job.name(), "bm25_rebuild");
    }

    #[test]
    fn test_config_debug() {
        let config = Bm25RebuildJobConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Bm25RebuildJobConfig"));
        assert!(debug_str.contains("rebuild_fn: false"));
    }
}
