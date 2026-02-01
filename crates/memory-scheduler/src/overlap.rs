//! Overlap policy for controlling concurrent job execution.
//!
//! When a job is scheduled to run but a previous instance is still running,
//! the overlap policy determines whether to skip the new execution or allow
//! concurrent runs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Policy for handling overlapping job executions.
///
/// When a job's scheduled time arrives but a previous instance is still running,
/// this policy determines the behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OverlapPolicy {
    /// Skip execution if previous run is still active (recommended).
    ///
    /// This prevents resource exhaustion when jobs take longer than their
    /// scheduling interval. The skipped execution is recorded in the registry.
    #[default]
    Skip,

    /// Allow concurrent executions.
    ///
    /// Multiple instances of the same job can run simultaneously. Use with
    /// caution as this can lead to resource contention and race conditions.
    Concurrent,
}

/// Guard for tracking whether a job is currently running.
///
/// The `OverlapGuard` uses an `AtomicBool` to track running state and provides
/// lock-free acquisition of a `RunGuard` that automatically releases the lock
/// when dropped.
pub struct OverlapGuard {
    is_running: Arc<AtomicBool>,
    policy: OverlapPolicy,
}

impl OverlapGuard {
    /// Create a new overlap guard with the given policy.
    pub fn new(policy: OverlapPolicy) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            policy,
        }
    }

    /// Attempt to acquire the guard for execution.
    ///
    /// Returns `Some(RunGuard)` if the job should proceed, or `None` if it
    /// should be skipped due to the overlap policy.
    ///
    /// # Behavior by Policy
    ///
    /// - `Skip`: Returns `None` if the job is already running, otherwise
    ///   returns a `RunGuard` and marks the job as running.
    /// - `Concurrent`: Always returns a `RunGuard`, allowing multiple
    ///   concurrent executions.
    pub fn try_acquire(&self) -> Option<RunGuard> {
        match self.policy {
            OverlapPolicy::Skip => {
                // Try to atomically set is_running from false to true
                if self
                    .is_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    Some(RunGuard {
                        flag: self.is_running.clone(),
                    })
                } else {
                    // Already running, skip this execution
                    None
                }
            }
            OverlapPolicy::Concurrent => {
                // Always allow concurrent execution, use a dummy flag
                Some(RunGuard {
                    flag: Arc::new(AtomicBool::new(true)),
                })
            }
        }
    }

    /// Check if the job is currently running.
    ///
    /// For `Concurrent` policy, this only reflects the state of the shared
    /// flag, which may not accurately represent all running instances.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Get the overlap policy.
    pub fn policy(&self) -> OverlapPolicy {
        self.policy
    }
}

/// RAII guard that releases the running flag when dropped.
///
/// This ensures that even if a job panics, the running flag is properly
/// cleared to allow subsequent executions.
pub struct RunGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_overlap_skip_prevents_concurrent() {
        let guard = OverlapGuard::new(OverlapPolicy::Skip);

        // First acquisition should succeed
        let run1 = guard.try_acquire();
        assert!(run1.is_some());
        assert!(guard.is_running());

        // Second acquisition should fail while first is held
        let run2 = guard.try_acquire();
        assert!(run2.is_none());

        // Drop the first guard
        drop(run1);
        assert!(!guard.is_running());

        // Now acquisition should succeed again
        let run3 = guard.try_acquire();
        assert!(run3.is_some());
    }

    #[test]
    fn test_overlap_concurrent_allows_multiple() {
        let guard = OverlapGuard::new(OverlapPolicy::Concurrent);

        // First acquisition should succeed
        let run1 = guard.try_acquire();
        assert!(run1.is_some());

        // Second acquisition should also succeed
        let run2 = guard.try_acquire();
        assert!(run2.is_some());

        // Third acquisition should also succeed
        let run3 = guard.try_acquire();
        assert!(run3.is_some());

        // All three should be valid
        assert!(run1.is_some() && run2.is_some() && run3.is_some());
    }

    #[test]
    fn test_overlap_skip_thread_safety() {
        let guard = Arc::new(OverlapGuard::new(OverlapPolicy::Skip));
        let successful_acquisitions = Arc::new(std::sync::atomic::AtomicU32::new(0));

        // Spawn multiple threads trying to acquire the guard
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let guard = guard.clone();
                let counter = successful_acquisitions.clone();
                thread::spawn(move || {
                    if let Some(_run_guard) = guard.try_acquire() {
                        counter.fetch_add(1, Ordering::SeqCst);
                        // Hold the guard briefly
                        thread::sleep(Duration::from_millis(10));
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // With Skip policy, only one thread at a time can hold the guard
        // But since threads may finish before others try, count can vary
        // The key invariant is that is_running is false after all complete
        assert!(!guard.is_running());
    }

    #[test]
    fn test_run_guard_releases_on_drop() {
        let guard = OverlapGuard::new(OverlapPolicy::Skip);

        {
            let _run = guard.try_acquire().unwrap();
            assert!(guard.is_running());
        } // RunGuard dropped here

        assert!(!guard.is_running());
    }

    #[test]
    fn test_overlap_policy_default() {
        let policy = OverlapPolicy::default();
        assert_eq!(policy, OverlapPolicy::Skip);
    }

    #[test]
    fn test_overlap_guard_policy() {
        let guard = OverlapGuard::new(OverlapPolicy::Concurrent);
        assert_eq!(guard.policy(), OverlapPolicy::Concurrent);
    }

    #[test]
    fn test_overlap_policy_serialization() {
        let skip = OverlapPolicy::Skip;
        let concurrent = OverlapPolicy::Concurrent;

        // Test serialization round-trip
        let skip_json = serde_json::to_string(&skip).unwrap();
        let concurrent_json = serde_json::to_string(&concurrent).unwrap();

        let skip_back: OverlapPolicy = serde_json::from_str(&skip_json).unwrap();
        let concurrent_back: OverlapPolicy = serde_json::from_str(&concurrent_json).unwrap();

        assert_eq!(skip, skip_back);
        assert_eq!(concurrent, concurrent_back);
    }
}
