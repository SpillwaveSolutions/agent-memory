//! Jitter utilities for distributed scheduling.
//!
//! Jitter adds a random delay before job execution to prevent thundering herd
//! problems when many instances are scheduled at the same time.

use std::time::Duration;

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Configuration for job execution jitter.
///
/// Jitter adds a random delay before job execution to spread out load when
/// multiple instances are scheduled at the same time.
///
/// # Example
///
/// ```
/// use memory_scheduler::JitterConfig;
///
/// // Create jitter config with up to 30 seconds delay
/// let config = JitterConfig::new(30);
///
/// // Generate a random jitter duration
/// let delay = config.generate_jitter();
/// assert!(delay <= std::time::Duration::from_secs(30));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JitterConfig {
    /// Maximum jitter in seconds (0 = no jitter).
    pub max_jitter_secs: u64,
}

impl Default for JitterConfig {
    fn default() -> Self {
        Self { max_jitter_secs: 0 }
    }
}

impl JitterConfig {
    /// Create a new jitter configuration with the given maximum delay.
    ///
    /// # Arguments
    ///
    /// * `max_jitter_secs` - Maximum jitter in seconds. Set to 0 for no jitter.
    pub fn new(max_jitter_secs: u64) -> Self {
        Self { max_jitter_secs }
    }

    /// Create a jitter configuration with no delay.
    pub fn none() -> Self {
        Self { max_jitter_secs: 0 }
    }

    /// Generate a random jitter duration.
    ///
    /// Returns a duration between 0 and `max_jitter_secs` (exclusive).
    /// If `max_jitter_secs` is 0, returns `Duration::ZERO`.
    pub fn generate_jitter(&self) -> Duration {
        if self.max_jitter_secs == 0 {
            return Duration::ZERO;
        }
        let jitter_ms = rand::thread_rng().gen_range(0..self.max_jitter_secs * 1000);
        Duration::from_millis(jitter_ms)
    }

    /// Check if jitter is enabled.
    pub fn is_enabled(&self) -> bool {
        self.max_jitter_secs > 0
    }
}

/// Execute a future with jitter delay.
///
/// Applies a random delay (up to `max_jitter_secs`) before executing the
/// provided future. This is useful for spreading out load when many jobs
/// are scheduled at the same time.
///
/// # Arguments
///
/// * `max_jitter_secs` - Maximum jitter in seconds. Set to 0 for no delay.
/// * `job_fn` - The async function to execute after the delay.
///
/// # Example
///
/// ```ignore
/// use memory_scheduler::with_jitter;
///
/// // Execute with up to 10 seconds random delay
/// with_jitter(10, async {
///     do_work().await
/// }).await;
/// ```
pub async fn with_jitter<F, T>(max_jitter_secs: u64, job_fn: F) -> T
where
    F: std::future::Future<Output = T>,
{
    if max_jitter_secs > 0 {
        let config = JitterConfig::new(max_jitter_secs);
        let jitter = config.generate_jitter();
        if !jitter.is_zero() {
            tracing::debug!(jitter_ms = jitter.as_millis(), "Applying jitter delay");
            tokio::time::sleep(jitter).await;
        }
    }
    job_fn.await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_zero_is_immediate() {
        let config = JitterConfig::new(0);
        let jitter = config.generate_jitter();
        assert_eq!(jitter, Duration::ZERO);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_jitter_within_bounds() {
        let config = JitterConfig::new(10);
        assert!(config.is_enabled());

        // Generate many jitter values and verify they're within bounds
        for _ in 0..100 {
            let jitter = config.generate_jitter();
            assert!(jitter < Duration::from_secs(10));
        }
    }

    #[test]
    fn test_jitter_default() {
        let config = JitterConfig::default();
        assert_eq!(config.max_jitter_secs, 0);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_jitter_none() {
        let config = JitterConfig::none();
        assert_eq!(config.max_jitter_secs, 0);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_jitter_serialization() {
        let config = JitterConfig::new(30);
        let json = serde_json::to_string(&config).unwrap();
        let config_back: JitterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, config_back);
    }

    #[tokio::test]
    async fn test_with_jitter_zero_no_delay() {
        let start = std::time::Instant::now();
        let result = with_jitter(0, async { 42 }).await;
        let elapsed = start.elapsed();

        assert_eq!(result, 42);
        // With zero jitter, should complete almost instantly
        assert!(elapsed < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_with_jitter_returns_value() {
        let result = with_jitter(0, async { "hello" }).await;
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_with_jitter_small_delay() {
        // Use a small jitter value to keep tests fast
        let result = with_jitter(1, async { 123 }).await;
        assert_eq!(result, 123);
    }

    #[test]
    fn test_jitter_distribution() {
        let config = JitterConfig::new(10);

        // Collect many samples
        let samples: Vec<Duration> = (0..1000).map(|_| config.generate_jitter()).collect();

        // Verify all are within bounds
        for s in &samples {
            assert!(*s < Duration::from_secs(10));
        }

        // Check we get some variation (not all the same value)
        let unique: std::collections::HashSet<_> = samples.iter().map(|d| d.as_millis()).collect();
        assert!(unique.len() > 1, "Jitter should produce varied values");
    }
}
