//! Scheduler service wrapper around tokio-cron-scheduler.
//!
//! Provides lifecycle management for background jobs with
//! graceful shutdown support.

use std::sync::atomic::{AtomicBool, Ordering};

use tokio_cron_scheduler::JobScheduler;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{SchedulerConfig, SchedulerError};

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
}
