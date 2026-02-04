//! Outbox indexing scheduled job.
//!
//! Processes outbox entries in batches and updates search indexes
//! (BM25 and vector) with checkpoint tracking for crash recovery.
//!
//! # Architecture
//!
//! The indexing job follows the outbox pattern:
//! 1. Events are written with outbox entries atomically
//! 2. This job periodically consumes outbox entries
//! 3. Each index updater (BM25, vector) processes entries
//! 4. Checkpoints track progress for crash recovery
//! 5. Processed entries are cleaned up after all indexes catch up
//!
//! # Default Schedule
//!
//! By default, the job runs every minute to minimize latency between
//! writes and searchability, while keeping checkpoint overhead low.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use memory_indexing::{IndexingPipeline, PipelineConfig};

use crate::{JitterConfig, OverlapPolicy, SchedulerError, SchedulerService, TimeoutConfig};

/// Configuration for the indexing job.
///
/// Controls the job schedule, batch processing, and checkpoint behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingJobConfig {
    /// Cron expression (default: "0 * * * * *" = every minute)
    ///
    /// The default one-minute interval balances latency with efficiency.
    /// For higher throughput systems, consider "*/30 * * * * *" (every 30s).
    pub cron: String,

    /// Timezone for scheduling (default: "UTC")
    pub timezone: String,

    /// Max jitter in seconds (default: 10)
    ///
    /// Jitter spreads job execution to avoid thundering herd when
    /// multiple daemon instances run on the same schedule.
    pub jitter_secs: u64,

    /// Maximum entries to process per batch (default: 100)
    ///
    /// Larger batches are more efficient but increase memory usage
    /// and latency between checkpoint commits.
    pub batch_size: usize,

    /// Maximum iterations per job run (default: 10)
    ///
    /// Limits how many batches are processed in a single job execution.
    /// Prevents long-running jobs from blocking other scheduled work.
    pub max_iterations: usize,

    /// Whether to cleanup processed outbox entries (default: true)
    ///
    /// When enabled, entries that all indexes have processed are
    /// deleted to reclaim storage space.
    pub cleanup_after_processing: bool,

    /// Whether to continue processing on individual entry errors (default: true)
    ///
    /// When enabled, errors on individual entries are logged but don't
    /// stop the batch. When disabled, any error fails the entire batch.
    pub continue_on_error: bool,

    /// Whether to commit after each batch (default: true)
    ///
    /// When enabled, checkpoints are saved after each batch for crash
    /// recovery. When disabled, commits only happen at job completion.
    pub commit_after_batch: bool,

    /// Timeout in seconds for job execution (default: 300 = 5 minutes)
    ///
    /// Prevents runaway jobs from blocking the scheduler. Set to 0 for
    /// no timeout (not recommended in production).
    pub timeout_secs: u64,
}

impl Default for IndexingJobConfig {
    fn default() -> Self {
        Self {
            cron: "0 * * * * *".to_string(), // Every minute
            timezone: "UTC".to_string(),
            jitter_secs: 10,
            batch_size: 100,
            max_iterations: 10,
            cleanup_after_processing: true,
            continue_on_error: true,
            commit_after_batch: true,
            timeout_secs: 300, // 5 minutes
        }
    }
}

impl IndexingJobConfig {
    /// Create a new config with the given cron expression.
    pub fn with_cron(mut self, cron: impl Into<String>) -> Self {
        self.cron = cron.into();
        self
    }

    /// Set the timezone.
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = timezone.into();
        self
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Set the max iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set whether to cleanup after processing.
    pub fn with_cleanup(mut self, cleanup: bool) -> Self {
        self.cleanup_after_processing = cleanup;
        self
    }

    /// Set the timeout in seconds.
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Convert to PipelineConfig for the indexing pipeline.
    pub fn to_pipeline_config(&self) -> PipelineConfig {
        PipelineConfig::default()
            .with_batch_size(self.batch_size)
            .with_continue_on_error(self.continue_on_error)
            .with_commit_after_batch(self.commit_after_batch)
    }
}

/// Register the indexing job with the scheduler.
///
/// Creates a job that processes outbox entries and updates search indexes.
/// The pipeline must already have index updaters registered.
///
/// # Arguments
///
/// * `scheduler` - The scheduler service to register the job with
/// * `pipeline` - Pre-configured indexing pipeline with updaters
/// * `config` - Configuration for job schedule and batch processing
///
/// # Errors
///
/// Returns error if job registration fails (invalid cron, invalid timezone).
///
/// # Example
///
/// ```ignore
/// use memory_indexing::{IndexingPipeline, PipelineConfig, Bm25IndexUpdater};
/// use memory_scheduler::{SchedulerService, create_indexing_job, IndexingJobConfig};
///
/// let mut pipeline = IndexingPipeline::new(storage.clone(), PipelineConfig::default());
/// pipeline.add_updater(Box::new(bm25_updater));
/// pipeline.load_checkpoints()?;
///
/// let pipeline = Arc::new(Mutex::new(pipeline));
///
/// create_indexing_job(&scheduler, pipeline, IndexingJobConfig::default()).await?;
/// ```
pub async fn create_indexing_job(
    scheduler: &SchedulerService,
    pipeline: Arc<Mutex<IndexingPipeline>>,
    config: IndexingJobConfig,
) -> Result<(), SchedulerError> {
    let max_iterations = config.max_iterations;
    let cleanup_after = config.cleanup_after_processing;

    scheduler
        .register_job(
            "outbox_indexing",
            &config.cron,
            Some(&config.timezone),
            OverlapPolicy::Skip,
            JitterConfig::new(config.jitter_secs),
            TimeoutConfig::new(config.timeout_secs),
            move || {
                let pipeline = pipeline.clone();
                async move { run_indexing_job(pipeline, max_iterations, cleanup_after).await }
            },
        )
        .await?;

    info!("Registered outbox indexing job");
    Ok(())
}

/// Execute the indexing job.
///
/// Processes outbox entries in batches until caught up or max iterations reached.
async fn run_indexing_job(
    pipeline: Arc<Mutex<IndexingPipeline>>,
    max_iterations: usize,
    cleanup_after: bool,
) -> Result<(), String> {
    // Acquire pipeline lock (tokio::sync::Mutex for async-friendly locking)
    let mut pipeline = pipeline.lock().await;

    debug!(max_iterations = max_iterations, "Starting indexing job run");

    // Process until caught up or max iterations
    let result = pipeline
        .process_until_caught_up(max_iterations)
        .map_err(|e| format!("Indexing failed: {}", e))?;

    if result.has_updates() {
        info!(
            total_processed = result.total_processed,
            last_sequence = ?result.last_sequence,
            "Indexing job processed entries"
        );
    } else {
        debug!("Indexing job: no entries to process");
    }

    // Cleanup processed entries if enabled
    if cleanup_after && result.has_updates() {
        match pipeline.cleanup_outbox() {
            Ok(deleted) => {
                if deleted > 0 {
                    info!(deleted = deleted, "Cleaned up processed outbox entries");
                }
            }
            Err(e) => {
                // Log but don't fail the job - cleanup can be retried
                warn!(error = %e, "Failed to cleanup outbox entries");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IndexingJobConfig::default();

        assert_eq!(config.cron, "0 * * * * *");
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.jitter_secs, 10);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_iterations, 10);
        assert!(config.cleanup_after_processing);
        assert!(config.continue_on_error);
        assert!(config.commit_after_batch);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_config_builder() {
        let config = IndexingJobConfig::default()
            .with_cron("*/30 * * * * *")
            .with_timezone("America/New_York")
            .with_batch_size(50)
            .with_max_iterations(5)
            .with_cleanup(false);

        assert_eq!(config.cron, "*/30 * * * * *");
        assert_eq!(config.timezone, "America/New_York");
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.max_iterations, 5);
        assert!(!config.cleanup_after_processing);
    }

    #[test]
    fn test_config_serialization() {
        let config = IndexingJobConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: IndexingJobConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.cron, decoded.cron);
        assert_eq!(config.timezone, decoded.timezone);
        assert_eq!(config.jitter_secs, decoded.jitter_secs);
        assert_eq!(config.batch_size, decoded.batch_size);
        assert_eq!(config.max_iterations, decoded.max_iterations);
        assert_eq!(
            config.cleanup_after_processing,
            decoded.cleanup_after_processing
        );
        assert_eq!(config.continue_on_error, decoded.continue_on_error);
        assert_eq!(config.commit_after_batch, decoded.commit_after_batch);
    }

    #[test]
    fn test_to_pipeline_config() {
        let config = IndexingJobConfig::default().with_batch_size(50);

        let pipeline_config = config.to_pipeline_config();

        assert_eq!(pipeline_config.batch_size, 50);
        assert!(pipeline_config.continue_on_error);
        assert!(pipeline_config.commit_after_batch);
    }

    #[test]
    fn test_config_json_format() {
        let config = IndexingJobConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();

        // Verify expected fields are present in JSON
        assert!(json.contains("\"cron\""));
        assert!(json.contains("\"timezone\""));
        assert!(json.contains("\"jitter_secs\""));
        assert!(json.contains("\"batch_size\""));
        assert!(json.contains("\"max_iterations\""));
        assert!(json.contains("\"cleanup_after_processing\""));
        assert!(json.contains("\"continue_on_error\""));
        assert!(json.contains("\"commit_after_batch\""));
    }
}
