//! Job registry for tracking job metadata and execution status.
//!
//! The `JobRegistry` provides a thread-safe registry for tracking the execution
//! status of scheduled jobs, including last/next run times, durations, and error counts.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result of a job execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobResult {
    /// Job completed successfully
    Success,
    /// Job failed with an error message
    Failed(String),
    /// Job was skipped (e.g., due to overlap policy)
    Skipped(String),
}

/// Extended job output with optional metadata.
///
/// Use this when your job needs to report stats back to the registry
/// (e.g., prune count, items processed).
#[derive(Debug, Clone, Default)]
pub struct JobOutput {
    /// Arbitrary key-value metadata from the job run.
    pub metadata: HashMap<String, String>,
}

impl JobOutput {
    /// Create a new empty job output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a metadata entry.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add prune count metadata (convenience method for prune jobs).
    pub fn with_prune_count(self, count: u32) -> Self {
        self.with_metadata("prune_count", count.to_string())
    }

    /// Add items processed metadata (convenience method).
    pub fn with_items_processed(self, count: usize) -> Self {
        self.with_metadata("items_processed", count.to_string())
    }
}

/// Status of a registered job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    /// Name of the job
    pub job_name: String,
    /// Cron expression for the job schedule
    pub cron_expr: String,
    /// When the job last started (if ever)
    pub last_run: Option<DateTime<Utc>>,
    /// Duration of the last run in milliseconds
    pub last_duration_ms: Option<u64>,
    /// Result of the last execution
    pub last_result: Option<JobResult>,
    /// When the job is next scheduled to run
    pub next_run: Option<DateTime<Utc>>,
    /// Total number of times the job has run
    pub run_count: u64,
    /// Total number of times the job has failed
    pub error_count: u64,
    /// Whether the job is currently executing
    pub is_running: bool,
    /// Whether the job is paused
    pub is_paused: bool,
    /// Optional metadata from last run (e.g., prune count, items processed)
    /// Maps arbitrary keys to string values for extensibility.
    #[serde(default)]
    pub last_run_metadata: HashMap<String, String>,
}

impl JobStatus {
    /// Create a new job status with the given name and cron expression.
    pub fn new(job_name: String, cron_expr: String) -> Self {
        Self {
            job_name,
            cron_expr,
            last_run: None,
            last_duration_ms: None,
            last_result: None,
            next_run: None,
            run_count: 0,
            error_count: 0,
            is_running: false,
            is_paused: false,
            last_run_metadata: HashMap::new(),
        }
    }
}

/// Registry for tracking job metadata and execution status.
///
/// The registry provides thread-safe access to job status information,
/// allowing multiple jobs to update their status concurrently.
///
/// # Example
///
/// ```
/// use memory_scheduler::JobRegistry;
///
/// let registry = JobRegistry::new();
/// registry.register("hourly-rollup", "0 0 * * * *");
///
/// // Job starts
/// registry.record_start("hourly-rollup");
/// assert!(registry.is_running("hourly-rollup"));
///
/// // Job completes
/// use memory_scheduler::JobResult;
/// registry.record_complete("hourly-rollup", JobResult::Success, 1500);
/// assert!(!registry.is_running("hourly-rollup"));
/// ```
pub struct JobRegistry {
    jobs: RwLock<HashMap<String, JobStatus>>,
}

impl JobRegistry {
    /// Create a new empty job registry.
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new job in the registry.
    ///
    /// If a job with the same name already exists, it will be replaced.
    pub fn register(&self, job_name: &str, cron_expr: &str) {
        let mut jobs = self.jobs.write().unwrap();
        jobs.insert(
            job_name.to_string(),
            JobStatus::new(job_name.to_string(), cron_expr.to_string()),
        );
    }

    /// Record that a job has started executing.
    pub fn record_start(&self, job_name: &str) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(status) = jobs.get_mut(job_name) {
            status.is_running = true;
        }
    }

    /// Record that a job has completed.
    ///
    /// Updates the last run time, duration, result, and run/error counts.
    pub fn record_complete(&self, job_name: &str, result: JobResult, duration_ms: u64) {
        self.record_complete_with_metadata(job_name, result, duration_ms, HashMap::new());
    }

    /// Record that a job has completed with optional metadata.
    ///
    /// Updates the last run time, duration, result, run/error counts, and metadata.
    /// Metadata can include job-specific stats like prune count, items processed, etc.
    pub fn record_complete_with_metadata(
        &self,
        job_name: &str,
        result: JobResult,
        duration_ms: u64,
        metadata: HashMap<String, String>,
    ) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(status) = jobs.get_mut(job_name) {
            status.is_running = false;
            status.last_run = Some(Utc::now());
            status.last_duration_ms = Some(duration_ms);
            status.run_count += 1;
            if matches!(result, JobResult::Failed(_)) {
                status.error_count += 1;
            }
            status.last_result = Some(result);
            status.last_run_metadata = metadata;
        }
    }

    /// Update the next scheduled run time for a job.
    pub fn set_next_run(&self, job_name: &str, next: DateTime<Utc>) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(status) = jobs.get_mut(job_name) {
            status.next_run = Some(next);
        }
    }

    /// Set the paused state of a job.
    pub fn set_paused(&self, job_name: &str, paused: bool) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(status) = jobs.get_mut(job_name) {
            status.is_paused = paused;
        }
    }

    /// Get the status of a specific job.
    ///
    /// Returns `None` if the job is not registered.
    pub fn get_status(&self, job_name: &str) -> Option<JobStatus> {
        self.jobs.read().unwrap().get(job_name).cloned()
    }

    /// Get the status of all registered jobs.
    pub fn get_all_status(&self) -> Vec<JobStatus> {
        self.jobs.read().unwrap().values().cloned().collect()
    }

    /// Check if a job is currently running.
    ///
    /// Returns `false` if the job is not registered.
    pub fn is_running(&self, job_name: &str) -> bool {
        self.jobs
            .read()
            .unwrap()
            .get(job_name)
            .map(|s| s.is_running)
            .unwrap_or(false)
    }

    /// Check if a job is registered.
    pub fn is_registered(&self, job_name: &str) -> bool {
        self.jobs.read().unwrap().contains_key(job_name)
    }

    /// Check if a job is paused.
    ///
    /// Returns `false` if the job is not registered.
    pub fn is_paused(&self, job_name: &str) -> bool {
        self.jobs
            .read()
            .unwrap()
            .get(job_name)
            .map(|s| s.is_paused)
            .unwrap_or(false)
    }

    /// Get the number of registered jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.read().unwrap().len()
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_register_and_get() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");

        let status = registry.get_status("test-job").unwrap();
        assert_eq!(status.job_name, "test-job");
        assert_eq!(status.cron_expr, "0 0 * * * *");
        assert_eq!(status.run_count, 0);
        assert!(!status.is_running);
        assert!(!status.is_paused);
    }

    #[test]
    fn test_registry_record_start() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");

        assert!(!registry.is_running("test-job"));
        registry.record_start("test-job");
        assert!(registry.is_running("test-job"));
    }

    #[test]
    fn test_registry_record_complete_success() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");
        registry.record_start("test-job");

        registry.record_complete("test-job", JobResult::Success, 1500);

        let status = registry.get_status("test-job").unwrap();
        assert!(!status.is_running);
        assert!(status.last_run.is_some());
        assert_eq!(status.last_duration_ms, Some(1500));
        assert_eq!(status.run_count, 1);
        assert_eq!(status.error_count, 0);
        assert_eq!(status.last_result, Some(JobResult::Success));
    }

    #[test]
    fn test_registry_record_complete_failure() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");
        registry.record_start("test-job");

        registry.record_complete("test-job", JobResult::Failed("timeout".into()), 5000);

        let status = registry.get_status("test-job").unwrap();
        assert_eq!(status.run_count, 1);
        assert_eq!(status.error_count, 1);
        assert_eq!(
            status.last_result,
            Some(JobResult::Failed("timeout".into()))
        );
    }

    #[test]
    fn test_registry_record_complete_skipped() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");

        registry.record_complete("test-job", JobResult::Skipped("overlap".into()), 0);

        let status = registry.get_status("test-job").unwrap();
        assert_eq!(status.run_count, 1);
        assert_eq!(status.error_count, 0); // Skipped doesn't count as error
        assert_eq!(
            status.last_result,
            Some(JobResult::Skipped("overlap".into()))
        );
    }

    #[test]
    fn test_registry_pause_resume() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");

        assert!(!registry.is_paused("test-job"));

        registry.set_paused("test-job", true);
        assert!(registry.is_paused("test-job"));

        registry.set_paused("test-job", false);
        assert!(!registry.is_paused("test-job"));
    }

    #[test]
    fn test_registry_get_all_status() {
        let registry = JobRegistry::new();
        registry.register("job-1", "0 0 * * * *");
        registry.register("job-2", "0 30 * * * *");
        registry.register("job-3", "0 0 0 * * *");

        let all = registry.get_all_status();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_registry_set_next_run() {
        let registry = JobRegistry::new();
        registry.register("test-job", "0 0 * * * *");

        let next = Utc::now();
        registry.set_next_run("test-job", next);

        let status = registry.get_status("test-job").unwrap();
        assert_eq!(status.next_run, Some(next));
    }

    #[test]
    fn test_registry_unknown_job() {
        let registry = JobRegistry::new();

        assert!(registry.get_status("unknown").is_none());
        assert!(!registry.is_running("unknown"));
        assert!(!registry.is_paused("unknown"));
        assert!(!registry.is_registered("unknown"));

        // These should not panic for unknown jobs
        registry.record_start("unknown");
        registry.record_complete("unknown", JobResult::Success, 100);
        registry.set_paused("unknown", true);
    }

    #[test]
    fn test_registry_job_count() {
        let registry = JobRegistry::new();
        assert_eq!(registry.job_count(), 0);

        registry.register("job-1", "0 0 * * * *");
        assert_eq!(registry.job_count(), 1);

        registry.register("job-2", "0 30 * * * *");
        assert_eq!(registry.job_count(), 2);
    }

    #[test]
    fn test_registry_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let registry = Arc::new(JobRegistry::new());

        // Register jobs from multiple threads
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let registry = registry.clone();
                thread::spawn(move || {
                    registry.register(&format!("job-{}", i), "0 0 * * * *");
                    registry.record_start(&format!("job-{}", i));
                    registry.record_complete(&format!("job-{}", i), JobResult::Success, 100);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(registry.job_count(), 10);
    }
}
