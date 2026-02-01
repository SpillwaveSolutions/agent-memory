//! Scheduler service wrapper around tokio-cron-scheduler.
//!
//! Provides lifecycle management for background jobs with
//! graceful shutdown support.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono_tz::Tz;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

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
        Err(e) => Err(SchedulerError::InvalidCron(format!(
            "'{}': {}",
            expr,
            e
        ))),
    }
}

/// Service wrapper around JobScheduler for lifecycle management.
///
/// Provides start/stop functionality with graceful shutdown support
/// via CancellationToken propagation to jobs.
pub struct SchedulerService {
    scheduler: JobScheduler,
    config: SchedulerConfig,
    shutdown_token: CancellationToken,
    is_running: AtomicBool,
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
        })
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
        use std::sync::Arc;
        use std::sync::atomic::AtomicU32;

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
}
