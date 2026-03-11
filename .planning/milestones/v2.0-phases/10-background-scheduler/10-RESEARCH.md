# Phase 10: Background Scheduler - Research

**Researched:** 2026-01-31
**Domain:** In-process async job scheduling with Tokio
**Confidence:** HIGH

## Summary

This phase adds a Tokio-based background scheduler for periodic jobs like TOC rollups, compaction, and future index maintenance. Research focused on the `tokio-cron-scheduler` crate (v0.15.x) which is the most mature and feature-complete option for async cron scheduling in Rust.

The standard approach is to embed `tokio-cron-scheduler` into the daemon process, configure timezone-aware scheduling via `chrono-tz`, implement overlap policies at the application level (the crate does not provide built-in overlap handling), add jitter through sleep delays before job execution, and leverage Tokio's cancellation tokens for graceful shutdown.

**Primary recommendation:** Use `tokio-cron-scheduler` 0.15.x with in-memory storage (no external persistence needed), implement a custom `JobRegistry` to track job metadata (last run, next run, status), and add overlap/jitter logic as wrapper functions around job execution.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio-cron-scheduler | 0.15.x | Cron scheduling + job management | Most complete async cron scheduler for Tokio |
| croner | 3.0.x | Cron expression parsing | Used by tokio-cron-scheduler, robust DST handling |
| chrono | 0.4.x | DateTime operations | Already in project, widely used |
| chrono-tz | 0.10.x | Timezone handling | DST-aware scheduling, IANA database |
| tokio-util | 0.7.x | Cancellation tokens, task tracker | Official Tokio utilities for graceful shutdown |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1.x | Job execution logging | Already in project |
| rand | 0.9.x | Jitter randomization | Already in project |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio-cron-scheduler | clokwerk | Simpler API but less async-native, no cron syntax |
| tokio-cron-scheduler | SACS | Lighter weight but less mature, fewer features |
| tokio-cron-scheduler | tokio-cron | Simpler but no job management, just scheduling |
| External persistence | PostgreSQL/Nats store | Overkill for single-daemon; adds complexity |

**Installation:**
```toml
[dependencies]
tokio-cron-scheduler = "0.15"
chrono-tz = "0.10"
tokio-util = { version = "0.7", features = ["rt"] }
```

## Architecture Patterns

### Recommended Project Structure
```
crates/memory-scheduler/
├── src/
│   ├── lib.rs              # Module exports
│   ├── scheduler.rs        # SchedulerService wrapper around JobScheduler
│   ├── job_registry.rs     # Custom registry for job metadata tracking
│   ├── jobs/
│   │   ├── mod.rs
│   │   ├── rollup.rs       # TOC rollup job definitions
│   │   └── compaction.rs   # RocksDB compaction job
│   ├── overlap.rs          # Overlap policy implementations
│   └── jitter.rs           # Jitter utilities
├── Cargo.toml
└── tests/
```

### Pattern 1: Scheduler Service Wrapper
**What:** Wrap `JobScheduler` in a custom `SchedulerService` that manages lifecycle and provides observability.
**When to use:** Always - provides clean abstraction over raw scheduler.
**Example:**
```rust
// Source: tokio-cron-scheduler docs + custom patterns
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use std::sync::Arc;

pub struct SchedulerService {
    scheduler: JobScheduler,
    registry: Arc<JobRegistry>,
    shutdown_token: CancellationToken,
}

impl SchedulerService {
    pub async fn new() -> Result<Self, SchedulerError> {
        let scheduler = JobScheduler::new().await?;
        Ok(Self {
            scheduler,
            registry: Arc::new(JobRegistry::new()),
            shutdown_token: CancellationToken::new(),
        })
    }

    pub async fn start(&self) -> Result<(), SchedulerError> {
        self.scheduler.start().await?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        self.shutdown_token.cancel();
        self.scheduler.shutdown().await.ok();
    }
}
```

### Pattern 2: Job Registry for Observability
**What:** Custom registry that tracks job metadata (last run, next run, status, duration).
**When to use:** Required for job status observability via CLI/gRPC.
**Example:**
```rust
use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct JobStatus {
    pub job_name: String,
    pub last_run: Option<DateTime<Utc>>,
    pub last_duration_ms: Option<u64>,
    pub last_result: Option<JobResult>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub error_count: u64,
    pub is_running: bool,
}

#[derive(Debug, Clone)]
pub enum JobResult {
    Success,
    Failed(String),
    Skipped(String),
}

pub struct JobRegistry {
    jobs: RwLock<HashMap<String, JobStatus>>,
}

impl JobRegistry {
    pub fn record_start(&self, job_name: &str) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(status) = jobs.get_mut(job_name) {
            status.is_running = true;
        }
    }

    pub fn record_complete(&self, job_name: &str, result: JobResult, duration_ms: u64) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(status) = jobs.get_mut(job_name) {
            status.is_running = false;
            status.last_run = Some(Utc::now());
            status.last_duration_ms = Some(duration_ms);
            status.last_result = Some(result.clone());
            status.run_count += 1;
            if matches!(result, JobResult::Failed(_)) {
                status.error_count += 1;
            }
        }
    }

    pub fn get_all_status(&self) -> Vec<JobStatus> {
        self.jobs.read().unwrap().values().cloned().collect()
    }
}
```

### Pattern 3: Timezone-Aware Job Creation
**What:** Use `_tz` variants of job creation methods for timezone-aware scheduling.
**When to use:** Always - ensures correct DST handling.
**Example:**
```rust
use tokio_cron_scheduler::Job;
use chrono_tz::Tz;

// Create timezone-aware job using Job::new_async_tz
let timezone: Tz = "America/New_York".parse().unwrap();

let job = Job::new_async_tz(
    "0 0 2 * * *",  // 2 AM daily
    timezone,
    move |uuid, _lock| {
        Box::pin(async move {
            // Job logic
            tracing::info!("Running daily rollup job");
        })
    }
)?;
```

### Pattern 4: Overlap Policy Wrapper
**What:** Implement overlap policies (skip, queue, concurrent) as wrapper functions.
**When to use:** For long-running jobs that might overlap.
**Example:**
```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum OverlapPolicy {
    Skip,       // Skip if already running
    Queue,      // Wait for previous to finish (not recommended for cron)
    Concurrent, // Allow concurrent execution
}

pub struct OverlapGuard {
    is_running: Arc<AtomicBool>,
    policy: OverlapPolicy,
}

impl OverlapGuard {
    pub fn new(policy: OverlapPolicy) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            policy,
        }
    }

    /// Returns Some(guard) if job should run, None if should skip
    pub fn try_acquire(&self) -> Option<RunGuard> {
        match self.policy {
            OverlapPolicy::Skip => {
                if self.is_running.compare_exchange(
                    false, true,
                    Ordering::SeqCst, Ordering::SeqCst
                ).is_ok() {
                    Some(RunGuard { flag: self.is_running.clone() })
                } else {
                    tracing::info!("Skipping job - previous run still in progress");
                    None
                }
            }
            OverlapPolicy::Concurrent => {
                Some(RunGuard { flag: Arc::new(AtomicBool::new(true)) })
            }
            OverlapPolicy::Queue => {
                // Spin-wait (not recommended for cron jobs)
                while self.is_running.compare_exchange(
                    false, true,
                    Ordering::SeqCst, Ordering::SeqCst
                ).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Some(RunGuard { flag: self.is_running.clone() })
            }
        }
    }
}

pub struct RunGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}
```

### Pattern 5: Jitter Implementation
**What:** Add random delay before job execution to spread load.
**When to use:** When multiple instances might run the same schedule.
**Example:**
```rust
use rand::Rng;
use std::time::Duration;

/// Add jitter delay before job execution
/// max_jitter_secs: Maximum random delay in seconds
pub async fn with_jitter<F, T>(max_jitter_secs: u64, job_fn: F) -> T
where
    F: std::future::Future<Output = T>,
{
    if max_jitter_secs > 0 {
        let jitter_ms = rand::thread_rng().gen_range(0..max_jitter_secs * 1000);
        tokio::time::sleep(Duration::from_millis(jitter_ms)).await;
    }
    job_fn.await
}

// Usage in job:
let job = Job::new_async_tz(
    "0 0 * * * *",  // Every hour
    timezone,
    move |uuid, _lock| {
        Box::pin(async move {
            // Add up to 5 minutes of jitter
            with_jitter(300, async {
                do_actual_work().await;
            }).await;
        })
    }
)?;
```

### Anti-Patterns to Avoid
- **Blocking in async jobs:** Never use `std::thread::sleep()` in async job callbacks. Use `tokio::time::sleep()`.
- **Single-threaded test runtime:** Tests hang if using `#[tokio::test]` without `flavor = "multi_thread"`.
- **Ignoring DST:** Always use `_tz` job variants and avoid scheduling between 1-3 AM on DST transition dates.
- **No overlap handling:** Jobs without overlap guards can pile up if execution exceeds interval.
- **Hardcoded schedules:** Put cron expressions in configuration, not code.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cron parsing | Custom parser | croner (via tokio-cron-scheduler) | Edge cases: day-of-week, month boundaries, leap years |
| DST handling | Manual offset calculation | chrono-tz | IANA database updates, historical timezone changes |
| Graceful shutdown | Manual task tracking | tokio_util::sync::CancellationToken | Race conditions, missed cancellation signals |
| Job scheduling | Manual tokio::spawn + sleep | tokio-cron-scheduler | Next-run calculation, missed execution handling |
| Timezone database | Static offset mappings | chrono-tz | Timezone rules change (North Korea 2015, Russia 2011) |

**Key insight:** Cron scheduling has decades of edge cases. The croner library explicitly handles DST gaps (spring forward) and overlaps (fall back) per the Open Cron Pattern Specification. Don't reinvent this wheel.

## Common Pitfalls

### Pitfall 1: Test Hangs on scheduler.add()
**What goes wrong:** Tests hang indefinitely when adding jobs to scheduler.
**Why it happens:** Default `#[tokio::test]` uses single-threaded runtime; scheduler requires multi-threaded.
**How to avoid:** Use `#[tokio::test(flavor = "multi_thread")]` for all scheduler tests.
**Warning signs:** Test appears to hang after calling `scheduler.add()`.

### Pitfall 2: DST "Lost Hour" Jobs
**What goes wrong:** Jobs scheduled between 2-3 AM don't run on spring-forward DST dates.
**Why it happens:** The hour 2:00-2:59 doesn't exist when clocks jump from 2 AM to 3 AM.
**How to avoid:** Avoid scheduling critical jobs in the 1-3 AM window; use UTC for critical jobs.
**Warning signs:** Job didn't run on a specific Sunday in March/November.

### Pitfall 3: Overlapping Long-Running Jobs
**What goes wrong:** Multiple instances of the same job run concurrently, causing resource exhaustion.
**Why it happens:** If job takes longer than interval, next run starts before previous finishes.
**How to avoid:** Implement `OverlapPolicy::Skip` for all rollup jobs.
**Warning signs:** Growing memory usage, duplicate processing, database contention.

### Pitfall 4: Shutdown Losing Work
**What goes wrong:** In-progress jobs are cancelled abruptly, leaving partial work.
**Why it happens:** `scheduler.shutdown()` cancels running jobs immediately.
**How to avoid:** Use `CancellationToken` to signal jobs to checkpoint and finish cleanly.
**Warning signs:** Partial rollups, checkpoint inconsistency after restart.

### Pitfall 5: Scheduler Not Starting
**What goes wrong:** Jobs are added but never execute.
**Why it happens:** Forgot to call `scheduler.start()` after adding jobs.
**How to avoid:** Always call `.start()` before expecting jobs to run.
**Warning signs:** Jobs show "next_run" time but never execute.

## Code Examples

Verified patterns from official sources:

### Basic Async Job with Timezone
```rust
// Source: https://github.com/mvniekerk/tokio-cron-scheduler
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono_tz::Tz;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;

    let timezone: Tz = "America/New_York".parse()?;

    // Run every 10 seconds
    sched.add(
        Job::new_async_tz("1/10 * * * * *", timezone, |uuid, _lock| {
            Box::pin(async move {
                println!("Job {:?} running", uuid);
            })
        })?
    ).await?;

    sched.start().await?;

    // Keep running
    tokio::time::sleep(std::time::Duration::from_secs(100)).await;

    sched.shutdown().await?;
    Ok(())
}
```

### Job with Notification Callbacks
```rust
// Source: https://github.com/mvniekerk/tokio-cron-scheduler
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

async fn create_job_with_notifications() -> Result<Job, JobSchedulerError> {
    let mut job = Job::new_async("0 * * * * *", |uuid, _lock| {
        Box::pin(async move {
            println!("Running job {}", uuid);
        })
    })?;

    // Notification when job starts
    job.on_start_notification_add(
        &JobScheduler::new().await?,
        Box::new(|job_id, notification_id, type_of_notification| {
            Box::pin(async move {
                println!("Job {:?} started", job_id);
            })
        }),
    ).await?;

    Ok(job)
}
```

### Graceful Shutdown with CancellationToken
```rust
// Source: https://tokio.rs/tokio/topics/shutdown
use tokio_util::sync::CancellationToken;
use tokio::signal;

async fn run_with_graceful_shutdown(
    scheduler: JobScheduler,
    shutdown_token: CancellationToken,
) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating shutdown");
        }
    }

    // Signal all jobs to stop gracefully
    shutdown_token.cancel();

    // Give jobs time to finish current work
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Stop scheduler
    scheduler.shutdown().await.ok();
}
```

### TOC Rollup Job Definition
```rust
// Pattern for integrating existing RollupJob with scheduler
use memory_toc::rollup::{RollupJob, RollupError};
use memory_storage::Storage;
use std::sync::Arc;

pub fn create_rollup_job(
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
    cron_expr: &str,
    timezone: Tz,
    overlap_guard: Arc<OverlapGuard>,
    registry: Arc<JobRegistry>,
) -> Result<Job, JobSchedulerError> {
    let job_name = "toc_rollup".to_string();

    Job::new_async_tz(cron_expr, timezone, move |uuid, _lock| {
        let storage = storage.clone();
        let summarizer = summarizer.clone();
        let overlap_guard = overlap_guard.clone();
        let registry = registry.clone();
        let job_name = job_name.clone();

        Box::pin(async move {
            // Check overlap policy
            let Some(_guard) = overlap_guard.try_acquire() else {
                registry.record_complete(
                    &job_name,
                    JobResult::Skipped("Previous run still active".into()),
                    0
                );
                return;
            };

            registry.record_start(&job_name);
            let start = std::time::Instant::now();

            // Run the actual rollup
            match memory_toc::rollup::run_all_rollups(storage, summarizer).await {
                Ok(count) => {
                    tracing::info!(count, "TOC rollup completed");
                    registry.record_complete(
                        &job_name,
                        JobResult::Success,
                        start.elapsed().as_millis() as u64
                    );
                }
                Err(e) => {
                    tracing::error!(error = %e, "TOC rollup failed");
                    registry.record_complete(
                        &job_name,
                        JobResult::Failed(e.to_string()),
                        start.elapsed().as_millis() as u64
                    );
                }
            }
        })
    })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| job-scheduler (sync) | tokio-cron-scheduler (async) | 2022+ | Native async support, no blocking threads |
| cron crate | croner | 2024 | Better DST handling per OCPS spec |
| Manual UTC offset | chrono-tz | Ongoing | IANA database updates automatically |
| External scheduler (cron, systemd) | In-process scheduler | Architecture choice | Better integration, no external dependencies |

**Deprecated/outdated:**
- **job-scheduler crate:** Synchronous, blocks threads. Use tokio-cron-scheduler instead.
- **Manual UTC offsets:** Timezone rules change. Always use chrono-tz with IANA database.

## Open Questions

Things that couldn't be fully resolved:

1. **Queue overlap policy implementation**
   - What we know: Skip and Concurrent are straightforward; Queue requires blocking/waiting
   - What's unclear: Best async pattern for queue-style waiting without blocking tokio runtime
   - Recommendation: Start with Skip policy only; Queue rarely needed for cron jobs

2. **Job persistence across daemon restarts**
   - What we know: tokio-cron-scheduler supports PostgreSQL/Nats persistence
   - What's unclear: Whether persistence is needed when checkpoints already exist in RocksDB
   - Recommendation: Use in-memory scheduler; rely on existing checkpoint system for job recovery

3. **Metrics integration**
   - What we know: JobRegistry provides status data; need to expose via gRPC/CLI
   - What's unclear: Best format for metrics (Prometheus? Custom RPC?)
   - Recommendation: Defer metrics format decision; implement JobRegistry now, expose via existing gRPC

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| tokio-cron-scheduler abandonment | LOW | HIGH | Mature crate (700+ stars), fallback to manual implementation |
| DST edge case bugs | MEDIUM | LOW | Use UTC for critical jobs, extensive testing around DST dates |
| Job pileup under load | MEDIUM | MEDIUM | Overlap policy (Skip) + monitoring via registry |
| Shutdown data loss | LOW | HIGH | Cancellation tokens + checkpoint system |
| Cron expression misconfiguration | MEDIUM | LOW | Validation at config load time, test with mocked time |

## Sources

### Primary (HIGH confidence)
- [tokio-cron-scheduler GitHub](https://github.com/mvniekerk/tokio-cron-scheduler) - Full README, examples
- [tokio-cron-scheduler docs.rs](https://docs.rs/tokio-cron-scheduler/latest/tokio_cron_scheduler/) - API reference
- [Tokio Graceful Shutdown Guide](https://tokio.rs/tokio/topics/shutdown) - Official patterns
- [croner-rust GitHub](https://github.com/Hexagon/croner-rust) - DST handling specification
- [chrono-tz GitHub](https://github.com/chronotope/chrono-tz) - Timezone handling

### Secondary (MEDIUM confidence)
- [Platform.sh Cron Jitter](https://platform.sh/blog/increasing-cron-jitter) - Jitter patterns
- [Spring Boot Concurrent Job Execution Control](https://alexanderobregon.substack.com/p/concurrent-job-execution-control) - Overlap policy patterns
- [JobRunr Documentation](https://www.jobrunr.io/en/documentation/background-methods/recurring-jobs/) - Concurrent job handling

### Tertiary (LOW confidence)
- [Rust Users Forum on scheduling](https://users.rust-lang.org/t/what-is-the-best-way-to-run-scheduled-concurrent-tasks-in-rust/43931) - Community discussion

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - tokio-cron-scheduler is well-documented, actively maintained
- Architecture patterns: HIGH - Patterns derived from official docs and existing codebase
- Overlap/jitter patterns: MEDIUM - Application-level patterns, not crate-provided
- Pitfalls: HIGH - Documented in crate issues and README

**Research date:** 2026-01-31
**Valid until:** 2026-03-01 (30 days - stable crate, unlikely major changes)
