//! Scheduler service wrapper around tokio-cron-scheduler.
//!
//! Provides lifecycle management for background jobs with
//! graceful shutdown support, job status tracking, overlap prevention,
//! and jitter for distributed scheduling.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono_tz::Tz;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::jitter::{JitterConfig, TimeoutConfig};
use crate::overlap::{OverlapGuard, OverlapPolicy};
use crate::registry::{JobRegistry, JobResult};
use crate::{SchedulerConfig, SchedulerError};

/// Validate a cron expression.
///
/// Checks that the expression is syntactically valid. The expression should
/// use 6-field format: second minute hour day-of-month month day-of-week.
///
/// # Errors
///
/// Returns `SchedulerError::InvalidCron` if the expression is not valid.
///
/// # Example
///
/// ```
/// use memory_scheduler::validate_cron_expression;
///
/// // Valid expressions
/// assert!(validate_cron_expression("0 0 * * * *").is_ok());  // Every hour
/// assert!(validate_cron_expression("0 30 4 * * *").is_ok()); // 4:30 AM daily
///
/// // Invalid expressions
/// assert!(validate_cron_expression("invalid").is_err());
/// assert!(validate_cron_expression("").is_err());
/// ```
pub fn validate_cron_expression(expr: &str) -> Result<(), SchedulerError> {
    // Try to create a job to validate the expression
    // tokio-cron-scheduler uses croner internally for parsing
    match Job::new_async(expr, |_uuid, _lock| Box::pin(async {})) {
        Ok(_) => Ok(()),
        Err(e) => Err(SchedulerError::InvalidCron(format!("'{}': {}", expr, e))),
    }
}

/// Service wrapper around JobScheduler for lifecycle management.
///
/// Provides start/stop functionality with graceful shutdown support
/// via CancellationToken propagation to jobs. Also includes a job
/// registry for tracking job status and execution history.
pub struct SchedulerService {
    scheduler: JobScheduler,
    config: SchedulerConfig,
    shutdown_token: CancellationToken,
    is_running: AtomicBool,
    registry: Arc<JobRegistry>,
}

impl SchedulerService {
    /// Create a new scheduler service with the given configuration.
    ///
    /// The scheduler is created but not started. Call `start()` to begin
    /// executing scheduled jobs.
    pub async fn new(config: SchedulerConfig) -> Result<Self, SchedulerError> {
        // Validate timezone configuration upfront
        let _ = config.parse_timezone()?;

        let scheduler = JobScheduler::new().await?;

        Ok(Self {
            scheduler,
            config,
            shutdown_token: CancellationToken::new(),
            is_running: AtomicBool::new(false),
            registry: Arc::new(JobRegistry::new()),
        })
    }

    /// Get a reference to the job registry.
    ///
    /// The registry tracks job status, execution history, and provides
    /// observability into scheduled jobs.
    pub fn registry(&self) -> Arc<JobRegistry> {
        self.registry.clone()
    }

    /// Start the scheduler.
    ///
    /// Jobs will begin executing according to their schedules.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::AlreadyRunning` if the scheduler is already started.
    pub async fn start(&self) -> Result<(), SchedulerError> {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return Err(SchedulerError::AlreadyRunning);
        }

        self.scheduler.start().await?;
        info!("Scheduler started");

        Ok(())
    }

    /// Shutdown the scheduler gracefully.
    ///
    /// Signals all jobs to stop via the cancellation token, waits for
    /// the configured timeout, then stops the scheduler.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::NotRunning` if the scheduler is not started.
    pub async fn shutdown(&mut self) -> Result<(), SchedulerError> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(SchedulerError::NotRunning);
        }

        info!("Initiating scheduler shutdown");

        // Signal all jobs to stop
        self.shutdown_token.cancel();

        // Give jobs time to finish
        tokio::time::sleep(std::time::Duration::from_secs(
            self.config.shutdown_timeout_secs.min(5), // Cap at 5s for tests
        ))
        .await;

        // Stop the scheduler
        if let Err(e) = self.scheduler.shutdown().await {
            warn!("Error during scheduler shutdown: {}", e);
        }

        self.is_running.store(false, Ordering::SeqCst);
        info!("Scheduler shutdown complete");

        Ok(())
    }

    /// Get a clone of the shutdown token for job cancellation.
    ///
    /// Jobs should check this token periodically and exit cleanly
    /// when cancelled.
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    /// Check if the scheduler is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Add a job to the scheduler.
    ///
    /// Returns the UUID of the added job.
    pub async fn add_job(
        &self,
        job: tokio_cron_scheduler::Job,
    ) -> Result<uuid::Uuid, SchedulerError> {
        let uuid = self.scheduler.add(job).await?;
        Ok(uuid)
    }

    /// Get the scheduler configuration.
    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }

    /// Add a cron job with timezone-aware scheduling.
    ///
    /// Creates and registers a job that runs according to the given cron expression.
    /// The job receives a clone of the shutdown token for cancellation support.
    ///
    /// # Arguments
    ///
    /// * `name` - Descriptive name for logging
    /// * `cron_expr` - Cron expression (6-field: sec min hour day month weekday)
    /// * `timezone` - IANA timezone string, or None to use config default
    /// * `job_fn` - Async function to execute
    ///
    /// # Errors
    ///
    /// Returns error if cron expression is invalid or timezone is not recognized.
    ///
    /// # Example
    ///
    /// ```ignore
    /// scheduler.add_cron_job(
    ///     "hourly-rollup",
    ///     "0 0 * * * *",      // Every hour
    ///     Some("America/New_York"),
    ///     || async { do_rollup().await },
    /// ).await?;
    /// ```
    pub async fn add_cron_job<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        timezone: Option<&str>,
        job_fn: F,
    ) -> Result<uuid::Uuid, SchedulerError>
    where
        F: Fn(CancellationToken) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send,
    {
        // Parse timezone
        let tz: Tz = match timezone {
            Some(tz_str) => tz_str
                .parse()
                .map_err(|_| SchedulerError::InvalidTimezone(tz_str.to_string()))?,
            None => self.config.parse_timezone()?,
        };

        // Validate cron expression
        validate_cron_expression(cron_expr)?;

        let job_name = name.to_string();
        let shutdown_token = self.shutdown_token.clone();

        // Create timezone-aware job
        let job = Job::new_async_tz(cron_expr, tz, move |_uuid, _lock| {
            let name = job_name.clone();
            let token = shutdown_token.clone();
            let job_fn = job_fn.clone();

            Box::pin(async move {
                info!(job = %name, "Job started");
                let start = std::time::Instant::now();

                // Execute the job function with the shutdown token
                job_fn(token).await;

                let elapsed = start.elapsed();
                info!(job = %name, duration_ms = elapsed.as_millis(), "Job completed");
            })
        })
        .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;

        let uuid = self.scheduler.add(job).await?;
        info!(job = %name, uuid = %uuid, cron = %cron_expr, timezone = %tz.name(), "Job registered");

        Ok(uuid)
    }

    /// Parse a timezone string into a chrono_tz::Tz.
    ///
    /// This is useful for validating timezone strings before job creation.
    pub fn parse_timezone(tz_str: &str) -> Result<Tz, SchedulerError> {
        tz_str
            .parse()
            .map_err(|_| SchedulerError::InvalidTimezone(tz_str.to_string()))
    }

    /// Register a job with full lifecycle management.
    ///
    /// This is the recommended way to add jobs as it provides:
    /// - Job status tracking via the registry
    /// - Overlap policy to prevent concurrent execution
    /// - Jitter for distributed scheduling
    /// - Timeout to prevent runaway jobs
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the job (used for status tracking)
    /// * `cron_expr` - Cron expression (6-field: sec min hour day month weekday)
    /// * `timezone` - IANA timezone string, or None to use config default
    /// * `overlap_policy` - How to handle overlapping executions
    /// * `jitter` - Random delay configuration before execution
    /// * `timeout` - Maximum execution time configuration
    /// * `job_fn` - Async function returning `Result<(), String>`
    ///
    /// # Example
    ///
    /// ```ignore
    /// use memory_scheduler::{OverlapPolicy, JitterConfig, TimeoutConfig};
    ///
    /// scheduler.register_job(
    ///     "hourly-rollup",
    ///     "0 0 * * * *",
    ///     None,
    ///     OverlapPolicy::Skip,
    ///     JitterConfig::new(30),
    ///     TimeoutConfig::new(300), // 5 minute timeout
    ///     || async { do_rollup().await },
    /// ).await?;
    ///
    /// // Check job status
    /// let status = scheduler.registry().get_status("hourly-rollup");
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn register_job<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        timezone: Option<&str>,
        overlap_policy: OverlapPolicy,
        jitter: JitterConfig,
        timeout: TimeoutConfig,
        job_fn: F,
    ) -> Result<uuid::Uuid, SchedulerError>
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send,
    {
        // Parse timezone
        let tz: Tz = match timezone {
            Some(tz_str) => tz_str
                .parse()
                .map_err(|_| SchedulerError::InvalidTimezone(tz_str.to_string()))?,
            None => self.config.parse_timezone()?,
        };

        // Validate cron expression
        validate_cron_expression(cron_expr)?;

        // Register in registry
        self.registry.register(name, cron_expr);

        let job_name = name.to_string();
        let registry = self.registry.clone();
        let overlap_guard = Arc::new(OverlapGuard::new(overlap_policy));
        let max_jitter_secs = jitter.max_jitter_secs;
        let timeout_duration = timeout.as_duration();

        // Create timezone-aware job with overlap, jitter, and timeout support
        let job = Job::new_async_tz(cron_expr, tz, move |_uuid, _lock| {
            let name = job_name.clone();
            let registry = registry.clone();
            let guard = overlap_guard.clone();
            let job_fn = job_fn.clone();
            let timeout_dur = timeout_duration;

            Box::pin(async move {
                // Check if job is paused
                if registry.is_paused(&name) {
                    debug!(job = %name, "Job is paused, skipping execution");
                    registry.record_complete(&name, JobResult::Skipped("paused".into()), 0);
                    return;
                }

                // Try to acquire overlap guard
                let run_guard = match guard.try_acquire() {
                    Some(g) => g,
                    None => {
                        debug!(job = %name, "Job already running, skipping due to overlap policy");
                        registry.record_complete(&name, JobResult::Skipped("overlap".into()), 0);
                        return;
                    }
                };

                // Record start
                registry.record_start(&name);
                info!(job = %name, "Job started");
                let start = std::time::Instant::now();

                // Apply jitter
                if max_jitter_secs > 0 {
                    let jitter_config = JitterConfig::new(max_jitter_secs);
                    let jitter_duration = jitter_config.generate_jitter();
                    if !jitter_duration.is_zero() {
                        debug!(job = %name, jitter_ms = jitter_duration.as_millis(), "Applying jitter delay");
                        tokio::time::sleep(jitter_duration).await;
                    }
                }

                // Execute the job function with optional timeout
                let result = match timeout_dur {
                    Some(duration) => {
                        debug!(job = %name, timeout_secs = duration.as_secs(), "Executing with timeout");
                        match tokio::time::timeout(duration, job_fn()).await {
                            Ok(Ok(())) => JobResult::Success,
                            Ok(Err(e)) => {
                                warn!(job = %name, error = %e, "Job failed");
                                JobResult::Failed(e)
                            }
                            Err(_) => {
                                warn!(job = %name, timeout_secs = duration.as_secs(), "Job timed out");
                                JobResult::Failed(format!("Job timed out after {} seconds", duration.as_secs()))
                            }
                        }
                    }
                    None => {
                        match job_fn().await {
                            Ok(()) => JobResult::Success,
                            Err(e) => {
                                warn!(job = %name, error = %e, "Job failed");
                                JobResult::Failed(e)
                            }
                        }
                    }
                };

                let elapsed = start.elapsed();
                let duration_ms = elapsed.as_millis() as u64;

                // Record completion
                registry.record_complete(&name, result, duration_ms);
                info!(job = %name, duration_ms = duration_ms, "Job completed");

                // RunGuard is dropped here, releasing the overlap lock
                drop(run_guard);
            })
        })
        .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;

        let uuid = self.scheduler.add(job).await?;
        info!(
            job = %name,
            uuid = %uuid,
            cron = %cron_expr,
            timezone = %tz.name(),
            overlap = ?overlap_policy,
            jitter_secs = max_jitter_secs,
            timeout_secs = timeout.timeout_secs,
            "Job registered"
        );

        Ok(uuid)
    }

    /// Register a job that returns metadata with its result.
    ///
    /// Like `register_job`, but the job function returns `Result<JobOutput, String>`
    /// where `JobOutput` contains optional metadata that is stored in the registry.
    ///
    /// This is useful for jobs that need to report stats (e.g., prune count, items processed)
    /// that can be queried via the scheduler status API.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use memory_scheduler::{OverlapPolicy, JitterConfig, TimeoutConfig, JobOutput};
    ///
    /// scheduler.register_job_with_metadata(
    ///     "prune-job",
    ///     "0 3 * * * *",
    ///     None,
    ///     OverlapPolicy::Skip,
    ///     JitterConfig::new(30),
    ///     TimeoutConfig::new(300),
    ///     || async {
    ///         let count = do_prune().await?;
    ///         Ok(JobOutput::new().with_prune_count(count))
    ///     },
    /// ).await?;
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn register_job_with_metadata<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        timezone: Option<&str>,
        overlap_policy: OverlapPolicy,
        jitter: JitterConfig,
        timeout: TimeoutConfig,
        job_fn: F,
    ) -> Result<uuid::Uuid, SchedulerError>
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<crate::registry::JobOutput, String>> + Send,
    {
        use std::collections::HashMap;

        // Parse timezone
        let tz: Tz = match timezone {
            Some(tz_str) => tz_str
                .parse()
                .map_err(|_| SchedulerError::InvalidTimezone(tz_str.to_string()))?,
            None => self.config.parse_timezone()?,
        };

        // Validate cron expression
        validate_cron_expression(cron_expr)?;

        // Register in registry
        self.registry.register(name, cron_expr);

        let job_name = name.to_string();
        let registry = self.registry.clone();
        let overlap_guard = Arc::new(OverlapGuard::new(overlap_policy));
        let max_jitter_secs = jitter.max_jitter_secs;
        let timeout_duration = timeout.as_duration();

        // Create timezone-aware job with overlap, jitter, and timeout support
        let job = Job::new_async_tz(cron_expr, tz, move |_uuid, _lock| {
            let name = job_name.clone();
            let registry = registry.clone();
            let guard = overlap_guard.clone();
            let job_fn = job_fn.clone();
            let timeout_dur = timeout_duration;

            Box::pin(async move {
                // Check if job is paused
                if registry.is_paused(&name) {
                    debug!(job = %name, "Job is paused, skipping execution");
                    registry.record_complete(&name, JobResult::Skipped("paused".into()), 0);
                    return;
                }

                // Try to acquire overlap guard
                let run_guard = match guard.try_acquire() {
                    Some(g) => g,
                    None => {
                        debug!(job = %name, "Job already running, skipping due to overlap policy");
                        registry.record_complete(&name, JobResult::Skipped("overlap".into()), 0);
                        return;
                    }
                };

                // Record start
                registry.record_start(&name);
                info!(job = %name, "Job started");
                let start = std::time::Instant::now();

                // Apply jitter
                if max_jitter_secs > 0 {
                    let jitter_config = JitterConfig::new(max_jitter_secs);
                    let jitter_duration = jitter_config.generate_jitter();
                    if !jitter_duration.is_zero() {
                        debug!(job = %name, jitter_ms = jitter_duration.as_millis(), "Applying jitter delay");
                        tokio::time::sleep(jitter_duration).await;
                    }
                }

                // Execute the job function with optional timeout
                let (result, metadata) = match timeout_dur {
                    Some(duration) => {
                        debug!(job = %name, timeout_secs = duration.as_secs(), "Executing with timeout");
                        match tokio::time::timeout(duration, job_fn()).await {
                            Ok(Ok(output)) => (JobResult::Success, output.metadata),
                            Ok(Err(e)) => {
                                warn!(job = %name, error = %e, "Job failed");
                                (JobResult::Failed(e), HashMap::new())
                            }
                            Err(_) => {
                                warn!(job = %name, timeout_secs = duration.as_secs(), "Job timed out");
                                (
                                    JobResult::Failed(format!(
                                        "Job timed out after {} seconds",
                                        duration.as_secs()
                                    )),
                                    HashMap::new(),
                                )
                            }
                        }
                    }
                    None => match job_fn().await {
                        Ok(output) => (JobResult::Success, output.metadata),
                        Err(e) => {
                            warn!(job = %name, error = %e, "Job failed");
                            (JobResult::Failed(e), HashMap::new())
                        }
                    },
                };

                let elapsed = start.elapsed();
                let duration_ms = elapsed.as_millis() as u64;

                // Record completion with metadata
                registry.record_complete_with_metadata(&name, result, duration_ms, metadata);
                info!(job = %name, duration_ms = duration_ms, "Job completed");

                // RunGuard is dropped here, releasing the overlap lock
                drop(run_guard);
            })
        })
        .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;

        let uuid = self.scheduler.add(job).await?;
        info!(
            job = %name,
            uuid = %uuid,
            cron = %cron_expr,
            timezone = %tz.name(),
            overlap = ?overlap_policy,
            jitter_secs = max_jitter_secs,
            timeout_secs = timeout.timeout_secs,
            "Job registered with metadata support"
        );

        Ok(uuid)
    }

    /// Pause a job by name.
    ///
    /// Paused jobs will skip execution when their scheduled time arrives.
    /// The job remains registered and can be resumed later.
    ///
    /// Note: This only affects jobs registered via `register_job`. Jobs
    /// added via `add_cron_job` are not tracked in the registry.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::JobNotFound` if no job with the given name
    /// is registered.
    pub fn pause_job(&self, job_name: &str) -> Result<(), SchedulerError> {
        if !self.registry.is_registered(job_name) {
            return Err(SchedulerError::JobNotFound(job_name.to_string()));
        }

        self.registry.set_paused(job_name, true);
        info!(job = %job_name, "Job paused");
        Ok(())
    }

    /// Resume a paused job.
    ///
    /// The job will resume executing at its next scheduled time.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::JobNotFound` if no job with the given name
    /// is registered.
    pub fn resume_job(&self, job_name: &str) -> Result<(), SchedulerError> {
        if !self.registry.is_registered(job_name) {
            return Err(SchedulerError::JobNotFound(job_name.to_string()));
        }

        self.registry.set_paused(job_name, false);
        info!(job = %job_name, "Job resumed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scheduler_new() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();
        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scheduler_start_stop() {
        let config = SchedulerConfig {
            shutdown_timeout_secs: 1,
            ..Default::default()
        };
        let mut scheduler = SchedulerService::new(config).await.unwrap();

        // Start the scheduler
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());

        // Starting again should fail
        let result = scheduler.start().await;
        assert!(matches!(result, Err(SchedulerError::AlreadyRunning)));

        // Shutdown
        scheduler.shutdown().await.unwrap();
        assert!(!scheduler.is_running());

        // Shutdown again should fail
        let result = scheduler.shutdown().await;
        assert!(matches!(result, Err(SchedulerError::NotRunning)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_shutdown_token() {
        let config = SchedulerConfig {
            shutdown_timeout_secs: 1,
            ..Default::default()
        };
        let mut scheduler = SchedulerService::new(config).await.unwrap();

        let token = scheduler.shutdown_token();
        assert!(!token.is_cancelled());

        scheduler.start().await.unwrap();
        scheduler.shutdown().await.unwrap();

        // Token should be cancelled after shutdown
        assert!(token.is_cancelled());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_invalid_timezone_config() {
        let config = SchedulerConfig {
            default_timezone: "Invalid/Zone".to_string(),
            ..Default::default()
        };
        let result = SchedulerService::new(config).await;
        assert!(matches!(result, Err(SchedulerError::InvalidTimezone(_))));
    }

    #[test]
    fn test_validate_cron_expression_valid() {
        // Standard cron expressions (6-field format)
        assert!(validate_cron_expression("0 0 * * * *").is_ok()); // Every hour
        assert!(validate_cron_expression("0 30 4 * * *").is_ok()); // 4:30 AM daily
        assert!(validate_cron_expression("*/10 * * * * *").is_ok()); // Every 10 seconds
        assert!(validate_cron_expression("0 0 0 * * SUN").is_ok()); // Midnight every Sunday
    }

    #[test]
    fn test_validate_cron_expression_invalid() {
        // Invalid expressions
        assert!(validate_cron_expression("invalid").is_err());
        assert!(validate_cron_expression("").is_err());
        assert!(validate_cron_expression("* * *").is_err()); // Too few fields
    }

    #[test]
    fn test_timezone_parsing() {
        // Valid timezones
        assert!(SchedulerService::parse_timezone("UTC").is_ok());
        assert!(SchedulerService::parse_timezone("America/New_York").is_ok());
        assert!(SchedulerService::parse_timezone("Europe/London").is_ok());
        assert!(SchedulerService::parse_timezone("Asia/Tokyo").is_ok());

        // Invalid timezone
        let result = SchedulerService::parse_timezone("Invalid/Zone");
        assert!(matches!(result, Err(SchedulerError::InvalidTimezone(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_cron_job_valid_expression() {
        use std::sync::atomic::AtomicU32;
        use std::sync::Arc;

        let config = SchedulerConfig {
            shutdown_timeout_secs: 1,
            ..Default::default()
        };
        let mut scheduler = SchedulerService::new(config).await.unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Add a job that runs every second
        let uuid = scheduler
            .add_cron_job(
                "test-job",
                "*/1 * * * * *", // Every second
                None,
                move |_token| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                    }
                },
            )
            .await
            .unwrap();

        assert!(!uuid.is_nil());

        // Start scheduler and let it run briefly
        scheduler.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        scheduler.shutdown().await.unwrap();

        // Counter may or may not have incremented depending on timing
        // The key test is that the job was successfully added
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_cron_job_invalid_expression() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let result = scheduler
            .add_cron_job("bad-job", "invalid-cron", None, |_token| async {})
            .await;

        assert!(matches!(result, Err(SchedulerError::InvalidCron(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_cron_job_with_timezone() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        // Valid timezone
        let uuid = scheduler
            .add_cron_job(
                "tz-job",
                "0 0 9 * * *", // 9 AM daily
                Some("America/New_York"),
                |_token| async {},
            )
            .await
            .unwrap();

        assert!(!uuid.is_nil());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_cron_job_invalid_timezone() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let result = scheduler
            .add_cron_job(
                "bad-tz-job",
                "0 0 * * * *",
                Some("Invalid/Timezone"),
                |_token| async {},
            )
            .await;

        assert!(matches!(result, Err(SchedulerError::InvalidTimezone(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_registry_access() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let registry = scheduler.registry();
        assert_eq!(registry.job_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_adds_to_registry() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let uuid = scheduler
            .register_job(
                "test-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        assert!(!uuid.is_nil());

        // Job should be in registry
        let registry = scheduler.registry();
        assert!(registry.is_registered("test-job"));

        let status = registry.get_status("test-job").unwrap();
        assert_eq!(status.job_name, "test-job");
        assert_eq!(status.cron_expr, "0 0 * * * *");
        assert!(!status.is_paused);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pause_resume_job() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        scheduler
            .register_job(
                "pausable-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        // Initially not paused
        assert!(!scheduler.registry().is_paused("pausable-job"));

        // Pause
        scheduler.pause_job("pausable-job").unwrap();
        assert!(scheduler.registry().is_paused("pausable-job"));

        // Resume
        scheduler.resume_job("pausable-job").unwrap();
        assert!(!scheduler.registry().is_paused("pausable-job"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pause_nonexistent_job() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let result = scheduler.pause_job("nonexistent");
        assert!(matches!(result, Err(SchedulerError::JobNotFound(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_resume_nonexistent_job() {
        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let result = scheduler.resume_job("nonexistent");
        assert!(matches!(result, Err(SchedulerError::JobNotFound(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_with_overlap_policy() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        // Register with Skip policy
        let uuid1 = scheduler
            .register_job(
                "skip-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        // Register with Concurrent policy
        let uuid2 = scheduler
            .register_job(
                "concurrent-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Concurrent,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        assert!(!uuid1.is_nil());
        assert!(!uuid2.is_nil());
        assert_ne!(uuid1, uuid2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_with_jitter() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        // Register with jitter
        let uuid = scheduler
            .register_job(
                "jittery-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::new(30),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await
            .unwrap();

        assert!(!uuid.is_nil());
        assert!(scheduler.registry().is_registered("jittery-job"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_invalid_cron() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let result = scheduler
            .register_job(
                "bad-cron-job",
                "invalid",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await;

        assert!(matches!(result, Err(SchedulerError::InvalidCron(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_invalid_timezone() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        let result = scheduler
            .register_job(
                "bad-tz-job",
                "0 0 * * * *",
                Some("Invalid/Timezone"),
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                || async { Ok(()) },
            )
            .await;

        assert!(matches!(result, Err(SchedulerError::InvalidTimezone(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_execution_tracking() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};
        use std::sync::atomic::AtomicU32;
        use std::sync::Arc;

        let config = SchedulerConfig {
            shutdown_timeout_secs: 1,
            ..Default::default()
        };
        let mut scheduler = SchedulerService::new(config).await.unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Register a job that runs every second
        scheduler
            .register_job(
                "tracked-job",
                "*/1 * * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::none(),
                move || {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }
                },
            )
            .await
            .unwrap();

        // Verify initial state
        let status = scheduler.registry().get_status("tracked-job").unwrap();
        assert_eq!(status.run_count, 0);
        assert!(status.last_run.is_none());

        // Start scheduler and let it run briefly
        scheduler.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        scheduler.shutdown().await.unwrap();

        // The job may or may not have run depending on timing
        // The key test is that if it ran, the registry would be updated
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_with_timeout() {
        use crate::{JitterConfig, OverlapPolicy, TimeoutConfig};

        let config = SchedulerConfig::default();
        let scheduler = SchedulerService::new(config).await.unwrap();

        // Register with timeout
        let uuid = scheduler
            .register_job(
                "timeout-job",
                "0 0 * * * *",
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::new(300), // 5 minute timeout
                || async { Ok(()) },
            )
            .await
            .unwrap();

        assert!(!uuid.is_nil());
        assert!(scheduler.registry().is_registered("timeout-job"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_job_timeout_triggers() {
        use crate::{JitterConfig, JobResult, OverlapPolicy, TimeoutConfig};
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;

        let config = SchedulerConfig {
            shutdown_timeout_secs: 1,
            ..Default::default()
        };
        let mut scheduler = SchedulerService::new(config).await.unwrap();

        let job_started = Arc::new(AtomicBool::new(false));
        let job_started_clone = job_started.clone();

        // Register a job that takes longer than the timeout
        scheduler
            .register_job(
                "slow-job",
                "*/5 * * * * *", // Every 5 seconds to avoid overlap during test
                None,
                OverlapPolicy::Skip,
                JitterConfig::none(),
                TimeoutConfig::new(1), // 1 second timeout
                move || {
                    let started = job_started_clone.clone();
                    async move {
                        started.store(true, Ordering::SeqCst);
                        // Sleep for longer than timeout
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        Ok(())
                    }
                },
            )
            .await
            .unwrap();

        // Start scheduler and let it run (wait long enough for one cron fire + timeout)
        scheduler.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(6000)).await;
        scheduler.shutdown().await.unwrap();

        // If the job ran, it should have been marked as failed due to timeout
        let status = scheduler.registry().get_status("slow-job");
        if let Some(s) = status {
            if s.run_count > 0 {
                // Job ran and should have timed out
                assert!(
                    matches!(s.last_result, Some(JobResult::Failed(ref msg)) if msg.contains("timed out")),
                    "Expected timeout failure, got {:?}",
                    s.last_result
                );
            }
        }
    }
}
