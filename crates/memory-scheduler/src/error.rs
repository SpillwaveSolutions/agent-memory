//! Error types for the scheduler crate.
//!
//! Provides unified error handling for scheduler operations including
//! cron expression validation, timezone parsing, and job lifecycle.

use thiserror::Error;
use tokio_cron_scheduler::JobSchedulerError;

/// Errors that can occur during scheduler operations.
#[derive(Debug, Error)]
pub enum SchedulerError {
    /// Error from the underlying tokio-cron-scheduler
    #[error("Scheduler error: {0}")]
    Scheduler(String),

    /// Invalid cron expression
    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    /// Invalid timezone string
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    /// Job not found in scheduler
    #[error("Job not found: {0}")]
    JobNotFound(String),

    /// Scheduler is already running
    #[error("Scheduler is already running")]
    AlreadyRunning,

    /// Scheduler is not running
    #[error("Scheduler is not running")]
    NotRunning,
}

impl From<JobSchedulerError> for SchedulerError {
    fn from(err: JobSchedulerError) -> Self {
        SchedulerError::Scheduler(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SchedulerError::InvalidCron("bad expression".to_string());
        assert!(err.to_string().contains("Invalid cron expression"));

        let err = SchedulerError::InvalidTimezone("Bad/Zone".to_string());
        assert!(err.to_string().contains("Invalid timezone"));

        let err = SchedulerError::JobNotFound("job-123".to_string());
        assert!(err.to_string().contains("Job not found"));

        let err = SchedulerError::AlreadyRunning;
        assert!(err.to_string().contains("already running"));

        let err = SchedulerError::NotRunning;
        assert!(err.to_string().contains("not running"));
    }
}
