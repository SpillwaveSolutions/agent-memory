# Background Scheduler Design

This document describes the architecture and implementation of the background scheduler system in agent-memory. The scheduler handles periodic tasks such as TOC rollups, RocksDB compaction, and index commits.

## Table of Contents

1. [Scheduler Overview](#scheduler-overview)
2. [Architecture](#architecture)
3. [Job Lifecycle](#job-lifecycle)
4. [Overlap Policies](#overlap-policies)
5. [Jitter Implementation](#jitter-implementation)
6. [Built-in Jobs](#built-in-jobs)
7. [Job Registry](#job-registry)
8. [Graceful Shutdown](#graceful-shutdown)
9. [Observability](#observability)

---

## Scheduler Overview

### Why In-Process Scheduling

The agent-memory daemon uses an in-process scheduler rather than external scheduling mechanisms (cron, systemd timers) for several important reasons:

1. **Tight Integration**: Jobs have direct access to storage handles, summarizers, and indexes without IPC overhead
2. **Graceful Shutdown**: The scheduler can signal jobs to checkpoint and complete cleanly during shutdown
3. **Single Deployment Unit**: No external dependencies to configure, simplifying deployment
4. **Resource Awareness**: Jobs can share the tokio runtime and respect backpressure signals
5. **State Consistency**: Job status is tracked alongside the data they operate on

### tokio-cron-scheduler Integration

The scheduler is built on `tokio-cron-scheduler` (v0.15.x), the most mature async-native cron scheduling library for Rust. Key features used:

- **Async-native execution**: Jobs run as tokio tasks, not blocking threads
- **Standard cron syntax**: 6-field expressions (second minute hour day month weekday)
- **Timezone support**: Via `_tz` variants for DST-aware scheduling
- **Job management**: Add, remove, and lifecycle hooks

```rust
use tokio_cron_scheduler::{Job, JobScheduler};

// Create scheduler with default in-memory storage
let scheduler = JobScheduler::new().await?;

// Add timezone-aware job
scheduler.add(
    Job::new_async_tz("0 0 * * * *", timezone, |_uuid, _lock| {
        Box::pin(async move {
            // Job logic
        })
    })?
).await?;

scheduler.start().await?;
```

### Timezone Handling with chrono-tz

All scheduling uses IANA timezone identifiers via the `chrono-tz` crate:

- **IANA Database**: Automatic handling of timezone rule changes
- **DST Transitions**: croner (internal parser) handles spring-forward gaps and fall-back overlaps
- **Configuration**: Timezone specified per-job or defaults to `SchedulerConfig.default_timezone`

```rust
use chrono_tz::Tz;

// Parse timezone string to chrono_tz::Tz
let tz: Tz = "America/New_York".parse()?;

// Jobs scheduled between 2-3 AM may not run on spring-forward days
// Recommendation: Avoid 1-3 AM window for critical jobs, or use UTC
```

---

## Architecture

### Component Diagram

```
                            +-------------------+
                            |    Daemon Main    |
                            +--------+----------+
                                     |
                        +------------+------------+
                        |                         |
                        v                         v
              +-------------------+     +-------------------+
              | SchedulerService  |     |   gRPC Server     |
              | (memory-scheduler)|     |                   |
              +--------+----------+     +--------+----------+
                       |                         |
         +-------------+-------------+           |
         |             |             |           |
         v             v             v           |
  +-----------+ +-----------+ +------------+    |
  | JobRegistry| |OverlapGuard| |JitterConfig|   |
  +-----------+ +-----------+ +------------+    |
         |                                       |
         +-------------------+-------------------+
                             |
              +--------------+--------------+
              |              |              |
              v              v              v
       +----------+   +----------+   +----------+
       | Rollup   |   |Compaction|   |  Index   |
       |  Jobs    |   |   Job    |   |  Commit  |
       +----+-----+   +----+-----+   +----+-----+
            |              |              |
            v              v              v
    +-------------------+-----------+
    |              Storage              |
    | (RocksDB + Tantivy + TOC)       |
    +----------------------------------+
```

### SchedulerService

The central coordinator that wraps `tokio_cron_scheduler::JobScheduler`:

```rust
pub struct SchedulerService {
    scheduler: JobScheduler,
    config: SchedulerConfig,
    shutdown_token: CancellationToken,
    is_running: AtomicBool,
    registry: Arc<JobRegistry>,
}
```

**Responsibilities**:
- Lifecycle management (start/stop)
- Job registration with validation
- Shutdown token distribution
- Registry access for observability

### JobRegistry

Thread-safe storage for job metadata and execution history:

```rust
pub struct JobRegistry {
    jobs: RwLock<HashMap<String, JobStatus>>,
}
```

**Responsibilities**:
- Track job registration and cron expressions
- Record execution start/complete/failure
- Maintain run counts and error counts
- Support pause/resume operations

### OverlapGuard

Lock-free mechanism to prevent concurrent execution of the same job:

```rust
pub struct OverlapGuard {
    is_running: Arc<AtomicBool>,
    policy: OverlapPolicy,
}
```

**Responsibilities**:
- Atomic state tracking via `compare_exchange`
- RAII guard for automatic release
- Policy-based skip/concurrent behavior

### Integration with Daemon

The scheduler integrates with the daemon's lifecycle:

```
Daemon Start
    |
    v
+-- SchedulerService::new() ----------+
|   - Validate config                 |
|   - Create JobScheduler             |
|   - Initialize JobRegistry          |
|   - Create CancellationToken        |
+---------+---------------------------+
          |
          v
+-- Register Jobs ------------------------+
|   - create_rollup_jobs()               |
|   - create_compaction_job()            |
|   - (future) create_index_commit_job() |
+---------+------------------------------+
          |
          v
+-- scheduler.start() ---------+
|   Jobs begin executing       |
+------------------------------+
```

---

## Job Lifecycle

### State Diagram

```
                    +-------------+
                    |  Registered |
                    +------+------+
                           |
          +----------------+----------------+
          |                                 |
          v                                 v
   +------+------+                   +------+------+
   |   Paused    |<----------------->|   Active    |
   +------+------+   pause/resume    +------+------+
          |                                 |
          |                                 |
          |     +---------------------------+
          |     |       Cron Trigger
          |     v
          | +---+-------------+
          | | Check Overlap   |
          | +---+-------------+
          |     |
          |     +-------+-------+
          |             |       |
          |    overlap  |       | acquired
          |             v       v
          |      +------+---+ +-+------------+
          |      | Skipped  | |  Executing   |
          |      +----------+ +------+-------+
          |                          |
          |              +-----------+-----------+
          |              |                       |
          |              v                       v
          |       +------+------+        +------+------+
          |       |   Success   |        |   Failed    |
          |       +------+------+        +------+------+
          |              |                       |
          |              +-----------+-----------+
          |                          |
          |                          v
          |                   +------+------+
          |                   |   Waiting   |
          |                   | (next cron) |
          |                   +------+------+
          |                          |
          +--------------------------+
```

### Registration Phase

Jobs are registered with full configuration:

```rust
scheduler.register_job(
    "toc_rollup_day",           // Unique name
    "0 0 1 * * *",              // Cron expression
    Some("America/New_York"),   // Timezone (or None for default)
    OverlapPolicy::Skip,        // Overlap handling
    JitterConfig::new(300),     // Up to 5 min jitter
    || async {                  // Job function
        do_work().await
    },
).await?;
```

**Validation performed**:
1. Cron expression syntax validation via croner
2. Timezone string validation via chrono-tz
3. Registration in JobRegistry with initial status

### Scheduling Phase

When the cron trigger fires:

```rust
// Inside the job wrapper
Box::pin(async move {
    // 1. Check pause state
    if registry.is_paused(&name) {
        registry.record_complete(&name, JobResult::Skipped("paused".into()), 0);
        return;
    }

    // 2. Try to acquire overlap guard
    let run_guard = match guard.try_acquire() {
        Some(g) => g,
        None => {
            registry.record_complete(&name, JobResult::Skipped("overlap".into()), 0);
            return;
        }
    };

    // 3. Record start
    registry.record_start(&name);

    // 4. Apply jitter delay
    if max_jitter_secs > 0 {
        tokio::time::sleep(jitter_duration).await;
    }

    // 5. Execute job function
    let result = job_fn().await;

    // 6. Record completion
    registry.record_complete(&name, result, duration_ms);

    // 7. RunGuard dropped here - releases overlap lock
})
```

### Pause/Resume Flow

Jobs can be paused without unregistering:

```rust
// Pause - job remains registered but skips execution
scheduler.pause_job("toc_rollup_day")?;

// Check status
assert!(scheduler.registry().is_paused("toc_rollup_day"));

// Resume - job will execute at next scheduled time
scheduler.resume_job("toc_rollup_day")?;
```

### Error Handling

Errors are captured and recorded without crashing the scheduler:

```rust
pub enum JobResult {
    Success,
    Failed(String),        // Error message captured
    Skipped(String),       // Reason: "overlap", "paused"
}
```

The registry maintains separate counters:
- `run_count`: Total executions (including skipped)
- `error_count`: Only incremented for `Failed` results

---

## Overlap Policies

### Policy Options

```rust
pub enum OverlapPolicy {
    Skip,       // Skip if previous run is still active (default)
    Concurrent, // Allow concurrent executions
}
```

### Overlap Policy Diagram

```
                          Cron Trigger
                               |
                               v
                     +--------------------+
                     | Previous Running?  |
                     +--------------------+
                         |           |
                        Yes         No
                         |           |
         +---------------+           +---------------+
         |                                           |
         v                                           v
+--------+--------+                         +--------+--------+
|  Check Policy   |                         | Acquire Guard   |
+-----------------+                         | (always works)  |
    |         |                             +-----------------+
  Skip    Concurrent                                 |
    |         |                                      |
    v         v                                      v
+-------+ +----------+                        +-----------+
| Skip  | | Allow    |                        | Execute   |
| Exec  | | Parallel |                        |   Job     |
+-------+ +----------+                        +-----------+
              |                                      |
              v                                      v
        +-----------+                         +-----------+
        | Execute   |                         | Release   |
        | Another   |                         | Guard     |
        | Instance  |                         +-----------+
        +-----------+
```

### Skip Policy (Default)

The recommended policy for most jobs. Uses atomic `compare_exchange` for lock-free acquisition:

```rust
impl OverlapGuard {
    pub fn try_acquire(&self) -> Option<RunGuard> {
        match self.policy {
            OverlapPolicy::Skip => {
                // Atomically set from false to true
                if self.is_running.compare_exchange(
                    false, true,
                    Ordering::SeqCst, Ordering::SeqCst
                ).is_ok() {
                    Some(RunGuard { flag: self.is_running.clone() })
                } else {
                    None  // Already running, skip
                }
            }
            // ...
        }
    }
}
```

**When to use**:
- Long-running jobs (rollups, compaction)
- Jobs that access shared resources
- Any job where parallel execution would cause contention

### Concurrent Policy

Allows multiple instances to run simultaneously:

```rust
OverlapPolicy::Concurrent => {
    // Always returns a guard with a dummy flag
    Some(RunGuard {
        flag: Arc::new(AtomicBool::new(true)),
    })
}
```

**When to use**:
- Idempotent operations
- Jobs that process independent data
- Read-only monitoring jobs

### Why Queue Was Deferred

A `Queue` policy (wait for previous to finish) was considered but deferred:

1. **Async complexity**: Implementing async waiting without blocking tokio requires careful design
2. **Unbounded queuing risk**: Long jobs could cause unbounded queue growth
3. **Rarely needed**: Most cron use cases prefer skip-if-running semantics
4. **Simple alternative**: Increase job interval if overlap is frequent

---

## Jitter Implementation

### Why Jitter Matters

Jitter prevents the "thundering herd" problem when multiple instances schedule jobs at the same time:

```
Without Jitter:                    With Jitter:

  00:00:00  Job A --+              00:00:00  Job A ---------+
  00:00:00  Job B --+              00:00:45  Job B ----+    |
  00:00:00  Job C --+              00:02:30  Job C --+ |    |
            |                                       | |    |
            v                                       v v    v
      All hit DB                             Spread over time
      simultaneously                         Reduced contention
```

### JitterConfig

Configuration for random delay generation:

```rust
#[derive(Debug, Clone, Default)]
pub struct JitterConfig {
    /// Maximum jitter in seconds (0 = no jitter)
    pub max_jitter_secs: u64,
}

impl JitterConfig {
    /// Create jitter config with maximum delay
    pub fn new(max_jitter_secs: u64) -> Self {
        Self { max_jitter_secs }
    }

    /// No jitter (immediate execution)
    pub fn none() -> Self {
        Self { max_jitter_secs: 0 }
    }

    /// Generate random duration between 0 and max_jitter_secs
    pub fn generate_jitter(&self) -> Duration {
        if self.max_jitter_secs == 0 {
            return Duration::ZERO;
        }
        let jitter_ms = rand::thread_rng().gen_range(0..self.max_jitter_secs * 1000);
        Duration::from_millis(jitter_ms)
    }
}
```

### Usage in Jobs

Jitter is applied after the overlap check but before execution:

```rust
// After acquiring overlap guard
if max_jitter_secs > 0 {
    let jitter_duration = jitter_config.generate_jitter();
    if !jitter_duration.is_zero() {
        debug!(jitter_ms = jitter_duration.as_millis(), "Applying jitter delay");
        tokio::time::sleep(jitter_duration).await;
    }
}

// Then execute job
job_fn().await;
```

### Standalone Function

For use outside the registry:

```rust
pub async fn with_jitter<F, T>(max_jitter_secs: u64, job_fn: F) -> T
where
    F: std::future::Future<Output = T>,
{
    if max_jitter_secs > 0 {
        let config = JitterConfig::new(max_jitter_secs);
        let jitter = config.generate_jitter();
        tokio::time::sleep(jitter).await;
    }
    job_fn.await
}
```

### Configuration Recommendations

| Job Type | Recommended Jitter |
|----------|-------------------|
| Day rollup | 300s (5 min) |
| Week rollup | 300s (5 min) |
| Month rollup | 300s (5 min) |
| Compaction | 600s (10 min) |
| Index commit | 60s (1 min) |

---

## Built-in Jobs

### TOC Rollup Jobs

The scheduler includes three rollup jobs that aggregate TOC nodes at different time granularities:

```rust
pub struct RollupJobConfig {
    pub day_cron: String,    // Default: "0 0 1 * * *"  (1 AM daily)
    pub week_cron: String,   // Default: "0 0 2 * * 0" (2 AM Sunday)
    pub month_cron: String,  // Default: "0 0 3 1 * *" (3 AM 1st of month)
    pub timezone: String,    // Default: "UTC"
    pub jitter_secs: u64,    // Default: 300
}
```

**Job Schedule Visualization**:

```
        Daily         Weekly          Monthly
        (1 AM)        (2 AM Sun)      (3 AM 1st)
          |              |               |
          v              v               v
    +----------+   +----------+   +----------+
    | Segments |-->|   Days   |-->|  Weeks   |
    | -> Days  |   | -> Weeks |   | -> Months|
    +----------+   +----------+   +----------+
         |              |              |
         v              v              v
    +------------------------------------------+
    |           TOC Hierarchical Index          |
    +------------------------------------------+
```

Each rollup job:
- Uses `OverlapPolicy::Skip` to prevent concurrent execution
- Applies jitter to spread load
- Uses `min_age` to avoid rolling up incomplete periods

```rust
// Day rollup - 1 hour min_age for incomplete hours
let job = RollupJob::new(storage, summarizer, TocLevel::Day, Duration::hours(1));

// Week rollup - 24 hour min_age for incomplete days
let job = RollupJob::new(storage, summarizer, TocLevel::Week, Duration::hours(24));

// Month rollup - 24 hour min_age for incomplete weeks
let job = RollupJob::new(storage, summarizer, TocLevel::Month, Duration::hours(24));
```

### RocksDB Compaction

Manual compaction to optimize storage:

```rust
pub struct CompactionJobConfig {
    pub cron: String,        // Default: "0 0 4 * * 0" (4 AM Sunday)
    pub timezone: String,    // Default: "UTC"
    pub jitter_secs: u64,    // Default: 600 (10 min)
}
```

**Why manual compaction**:
- Reclaims deleted space from tombstones
- Merges SST files to reduce read amplification
- Scheduled during low-traffic periods (early Sunday morning)

```rust
async fn run_compaction(storage: Arc<Storage>) -> Result<(), String> {
    storage
        .compact()
        .map(|_| info!("Compaction complete"))
        .map_err(|e| e.to_string())
}
```

### Index Commit (Tantivy)

Future job for committing Tantivy index changes:

```rust
// Planned - not yet implemented
pub struct IndexCommitJobConfig {
    pub cron: String,        // Proposed: "0 */5 * * * *" (every 5 min)
    pub timezone: String,
    pub jitter_secs: u64,    // Proposed: 60
}
```

**Purpose**: Ensure index changes are persisted to disk periodically, balancing durability with performance.

### Cron Expression Reference

The scheduler uses 6-field cron expressions:

```
 ┌────────────── second (0-59)
 │ ┌──────────── minute (0-59)
 │ │ ┌────────── hour (0-23)
 │ │ │ ┌──────── day of month (1-31)
 │ │ │ │ ┌────── month (1-12 or JAN-DEC)
 │ │ │ │ │ ┌──── day of week (0-6 or SUN-SAT)
 │ │ │ │ │ │
 * * * * * *
```

| Expression | Meaning |
|------------|---------|
| `0 0 * * * *` | Every hour at :00 |
| `0 0 1 * * *` | 1 AM daily |
| `0 0 2 * * 0` | 2 AM every Sunday |
| `0 0 3 1 * *` | 3 AM on the 1st of each month |
| `*/10 * * * * *` | Every 10 seconds |
| `0 */15 * * * *` | Every 15 minutes |

---

## Job Registry

### Class Diagram

```
+--------------------------------------------------+
|                   JobRegistry                     |
+--------------------------------------------------+
| - jobs: RwLock<HashMap<String, JobStatus>>       |
+--------------------------------------------------+
| + new() -> Self                                   |
| + register(name, cron_expr)                      |
| + record_start(name)                             |
| + record_complete(name, result, duration_ms)     |
| + set_next_run(name, datetime)                   |
| + set_paused(name, paused)                       |
| + get_status(name) -> Option<JobStatus>          |
| + get_all_status() -> Vec<JobStatus>             |
| + is_running(name) -> bool                       |
| + is_registered(name) -> bool                    |
| + is_paused(name) -> bool                        |
| + job_count() -> usize                           |
+--------------------------------------------------+
                        |
                        | contains
                        v
+--------------------------------------------------+
|                    JobStatus                      |
+--------------------------------------------------+
| + job_name: String                               |
| + cron_expr: String                              |
| + last_run: Option<DateTime<Utc>>                |
| + last_duration_ms: Option<u64>                  |
| + last_result: Option<JobResult>                 |
| + next_run: Option<DateTime<Utc>>                |
| + run_count: u64                                 |
| + error_count: u64                               |
| + is_running: bool                               |
| + is_paused: bool                                |
+--------------------------------------------------+
                        |
                        | uses
                        v
+--------------------------------------------------+
|                    JobResult                      |
+--------------------------------------------------+
| Success                                          |
| Failed(String)                                   |
| Skipped(String)                                  |
+--------------------------------------------------+
```

### Thread-Safe Access

The registry uses `RwLock` for concurrent access:

```rust
// Multiple readers can check status simultaneously
pub fn get_all_status(&self) -> Vec<JobStatus> {
    self.jobs.read().unwrap().values().cloned().collect()
}

// Writers get exclusive access
pub fn record_complete(&self, job_name: &str, result: JobResult, duration_ms: u64) {
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
    }
}
```

### Usage Example

```rust
// Get registry from scheduler
let registry = scheduler.registry();

// Check individual job status
if let Some(status) = registry.get_status("toc_rollup_day") {
    println!("Job: {}", status.job_name);
    println!("Last run: {:?}", status.last_run);
    println!("Run count: {}", status.run_count);
    println!("Error count: {}", status.error_count);
}

// List all jobs
for status in registry.get_all_status() {
    println!("{}: {} runs, {} errors",
        status.job_name,
        status.run_count,
        status.error_count
    );
}
```

---

## Graceful Shutdown

### CancellationToken Pattern

The scheduler uses tokio-util's `CancellationToken` to coordinate shutdown:

```
    SIGTERM/SIGINT
          |
          v
+-------------------+
| shutdown_token    |
|    .cancel()      |
+-------------------+
          |
          +-----------+-----------+-----------+
          |           |           |           |
          v           v           v           v
     +-------+   +-------+   +-------+   +-------+
     | Job A |   | Job B |   | Job C |   | Job D |
     +-------+   +-------+   +-------+   +-------+
          |           |           |           |
          v           v           v           v
     token.is_     token.is_   Complete   Complete
     cancelled()   cancelled()  current    current
          |           |          work       work
          v           v
       Checkpoint   Checkpoint
       and exit    and exit
```

### Shutdown Implementation

```rust
impl SchedulerService {
    pub async fn shutdown(&mut self) -> Result<(), SchedulerError> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(SchedulerError::NotRunning);
        }

        info!("Initiating scheduler shutdown");

        // 1. Signal all jobs to stop
        self.shutdown_token.cancel();

        // 2. Give jobs time to finish current work
        tokio::time::sleep(std::time::Duration::from_secs(
            self.config.shutdown_timeout_secs.min(5),
        )).await;

        // 3. Stop the scheduler (cancels any remaining jobs)
        if let Err(e) = self.scheduler.shutdown().await {
            warn!("Error during scheduler shutdown: {}", e);
        }

        self.is_running.store(false, Ordering::SeqCst);
        info!("Scheduler shutdown complete");

        Ok(())
    }
}
```

### Job Cancellation Response

Jobs receive the token and should check it periodically:

```rust
scheduler.add_cron_job(
    "long-running-job",
    "0 0 * * * *",
    None,
    move |token| async move {
        for chunk in data_chunks {
            // Check if we should stop
            if token.is_cancelled() {
                info!("Shutdown requested, checkpointing...");
                save_checkpoint(&chunk).await;
                return;
            }

            process_chunk(chunk).await;
        }
    },
).await?;
```

### Job Completion vs Cancellation

| Scenario | Behavior |
|----------|----------|
| Job completes before shutdown | Normal completion recorded |
| Job in progress at shutdown | Given `shutdown_timeout_secs` to finish |
| Job doesn't check token | Forcibly cancelled after timeout |
| Job panics | RunGuard dropped, overlap lock released |

### Daemon Integration

The daemon coordinates scheduler shutdown with other subsystems:

```rust
// In daemon shutdown handler
async fn shutdown(mut scheduler: SchedulerService, storage: Arc<Storage>) {
    // 1. Stop accepting new requests
    // 2. Shutdown scheduler (waits for jobs)
    scheduler.shutdown().await.ok();

    // 3. Flush storage
    storage.flush().ok();

    // 4. Close connections
}
```

---

## Observability

### GetSchedulerStatus RPC

The gRPC API exposes scheduler status (planned integration):

```protobuf
message GetSchedulerStatusRequest {}

message JobStatusProto {
    string job_name = 1;
    string cron_expr = 2;
    google.protobuf.Timestamp last_run = 3;
    uint64 last_duration_ms = 4;
    string last_result = 5;
    google.protobuf.Timestamp next_run = 6;
    uint64 run_count = 7;
    uint64 error_count = 8;
    bool is_running = 9;
    bool is_paused = 10;
}

message GetSchedulerStatusResponse {
    bool scheduler_running = 1;
    repeated JobStatusProto jobs = 2;
}
```

### CLI Commands

The CLI provides scheduler management commands:

```bash
# List all scheduled jobs
agent-memory scheduler list

# Show detailed job status
agent-memory scheduler status toc_rollup_day

# Pause a job
agent-memory scheduler pause toc_rollup_day

# Resume a paused job
agent-memory scheduler resume toc_rollup_day

# Trigger immediate execution (bypasses schedule)
agent-memory scheduler run toc_rollup_day
```

### Metrics and Logging

The scheduler emits structured logs via tracing:

```rust
// Job registration
info!(
    job = %name,
    uuid = %uuid,
    cron = %cron_expr,
    timezone = %tz.name(),
    overlap = ?overlap_policy,
    jitter_secs = max_jitter_secs,
    "Job registered"
);

// Job execution
info!(job = %name, "Job started");
debug!(job = %name, jitter_ms = jitter_duration.as_millis(), "Applying jitter delay");
warn!(job = %name, error = %e, "Job failed");
info!(job = %name, duration_ms = duration_ms, "Job completed");
```

**Key metrics available via registry**:

| Metric | Source |
|--------|--------|
| Job run count | `JobStatus.run_count` |
| Job error rate | `error_count / run_count` |
| Last execution time | `JobStatus.last_duration_ms` |
| Time since last run | `now - JobStatus.last_run` |
| Jobs currently running | Count where `is_running == true` |
| Paused jobs | Count where `is_paused == true` |

---

## Appendix: Alternatives Considered

### External Schedulers

| Option | Why Rejected |
|--------|--------------|
| System cron | No graceful shutdown, no status tracking |
| systemd timers | Platform-specific, adds deployment complexity |
| Kubernetes CronJob | Overkill for single-daemon use case |
| External queue (NATS) | Adds operational dependency |

### Other Rust Crates

| Crate | Why Not Chosen |
|-------|----------------|
| clokwerk | Simpler API but not async-native |
| SACS | Less mature, fewer features |
| tokio-cron | No job management, just scheduling |

### Persistence

Job state persistence was considered but deferred:

- **Current**: In-memory registry, reconstructed on restart
- **Future option**: Store job state in RocksDB if needed
- **Rationale**: Checkpointing in individual jobs handles recovery; scheduler-level persistence adds complexity without clear benefit

---

## Summary

The agent-memory background scheduler provides:

1. **In-process cron scheduling** via tokio-cron-scheduler
2. **Timezone-aware execution** via chrono-tz
3. **Overlap prevention** via atomic guards
4. **Load distribution** via configurable jitter
5. **Observability** via JobRegistry
6. **Graceful shutdown** via CancellationToken

This design balances simplicity with the operational requirements of a production system, avoiding external dependencies while providing the hooks needed for monitoring and management.
