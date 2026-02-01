# State Machines

This document describes the state machines that govern component lifecycles and data transitions in the agent-memory system. Understanding these states is essential for debugging, monitoring, and extending the system.

## Overview

The agent-memory system uses several interconnected state machines:

| Component | States | Purpose |
|-----------|--------|---------|
| Daemon | Starting, Running, Stopping, Stopped | Process lifecycle |
| Scheduler | Idle, Running, Stopping | Job execution control |
| Job Execution | Pending, Running, Completed/Failed/Skipped | Individual job runs |
| TOC Node | Draft, Building, Complete | Summary generation |
| Outbox Entry | Pending, Processing, Completed | Async task queue |
| gRPC Connection | Connecting, Connected, Streaming, Disconnected | Client connectivity |

---

## 1. Daemon Lifecycle

The memory daemon manages the entire system lifecycle, coordinating storage, scheduler, and gRPC server components.

### State Diagram

```
                    ┌─────────────┐
                    │   STOPPED   │
                    └──────┬──────┘
                           │
                           │ start_daemon()
                           ▼
                    ┌─────────────┐
                    │  STARTING   │
                    │             │
                    │ - Load config
                    │ - Open storage
                    │ - Init scheduler
                    │ - Register jobs
                    │ - Write PID file
                    └──────┬──────┘
                           │
                           │ scheduler.start() + server.serve()
                           ▼
                    ┌─────────────┐
        SIGINT ────▶│   RUNNING   │◀──── Normal operation
       SIGTERM      │             │
                    │ - gRPC serving
                    │ - Jobs executing
                    │ - Health: SERVING
                    └──────┬──────┘
                           │
                           │ shutdown_signal received
                           ▼
                    ┌─────────────┐
                    │  STOPPING   │
                    │             │
                    │ - Cancel jobs
                    │ - Drain requests
                    │ - Flush storage
                    │ - Remove PID file
                    └──────┬──────┘
                           │
                           │ cleanup complete
                           ▼
                    ┌─────────────┐
                    │   STOPPED   │
                    └─────────────┘
```

### States

| State | Description | Key Activities |
|-------|-------------|----------------|
| **STOPPED** | Daemon not running | No processes, no resources held |
| **STARTING** | Initialization phase | Config loading, storage open, scheduler setup |
| **RUNNING** | Normal operation | Serving gRPC requests, executing scheduled jobs |
| **STOPPING** | Graceful shutdown | Canceling jobs, draining connections |

### Transitions

| Transition | Trigger | Guard Conditions | Actions |
|------------|---------|------------------|---------|
| STOPPED → STARTING | `start_daemon()` called | Valid config path | Load settings, create tracing subscriber |
| STARTING → RUNNING | Initialization success | Storage opened, scheduler started | Begin serving gRPC, write PID file |
| STARTING → STOPPED | Initialization failure | Any error | Log error, exit with non-zero code |
| RUNNING → STOPPING | Shutdown signal | SIGINT or SIGTERM received | Cancel shutdown token, set health to NOT_SERVING |
| STOPPING → STOPPED | Cleanup complete | All jobs finished, server drained | Remove PID file, drop resources |

### Signal Handling

```rust
// From crates/memory-daemon/src/commands.rs
let shutdown_signal = async {
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down...");
        }
    }
};
```

The daemon handles two Unix signals:

- **SIGINT (Ctrl+C)**: Interactive shutdown, typically from terminal
- **SIGTERM**: Programmatic shutdown, used by process managers

Both signals trigger the same graceful shutdown sequence. The `CancellationToken` propagates the shutdown request to all running jobs.

### Graceful Shutdown Flow

1. **Signal received**: Shutdown signal future completes
2. **Health update**: Health reporter marks service as NOT_SERVING
3. **Server drain**: `serve_with_shutdown` stops accepting new connections
4. **Scheduler stop**: Jobs receive cancellation, wait for timeout
5. **Storage flush**: Pending writes are flushed to RocksDB
6. **PID cleanup**: PID file removed from filesystem

---

## 2. Scheduler States

The scheduler manages cron-based job execution with lifecycle control.

### State Diagram

```
                    ┌─────────────┐
                    │    IDLE     │◀─────────────────┐
                    │             │                  │
                    │ is_running: │                  │
                    │   false     │                  │
                    └──────┬──────┘                  │
                           │                         │
                           │ start()                 │
                           ▼                         │
                    ┌─────────────┐                  │
                    │   RUNNING   │                  │
                    │             │                  │
    AlreadyRunning◀─│ is_running: │                  │
     (error)        │   true      │                  │
                    │             │                  │
                    │ Jobs execute│                  │
                    │ per cron    │                  │
                    └──────┬──────┘                  │
                           │                         │
                           │ shutdown()              │
                           ▼                         │
                    ┌─────────────┐                  │
                    │  STOPPING   │                  │
                    │             │                  │
                    │ - Token     │                  │
                    │   cancelled │                  │
                    │ - Wait      │                  │
                    │   timeout   │                  │
                    └──────┬──────┘                  │
                           │                         │
                           │ shutdown complete       │
                           └─────────────────────────┘
```

### States

| State | `is_running` | Description |
|-------|--------------|-------------|
| **IDLE** | `false` | Scheduler created but not started |
| **RUNNING** | `true` | Actively executing jobs per schedule |
| **STOPPING** | `true` → `false` | Shutdown in progress |

### Error Conditions

```rust
// From crates/memory-scheduler/src/scheduler.rs
pub async fn start(&self) -> Result<(), SchedulerError> {
    if self.is_running.swap(true, Ordering::SeqCst) {
        return Err(SchedulerError::AlreadyRunning);  // Can't start twice
    }
    // ...
}

pub async fn shutdown(&mut self) -> Result<(), SchedulerError> {
    if !self.is_running.load(Ordering::SeqCst) {
        return Err(SchedulerError::NotRunning);  // Can't stop if not running
    }
    // ...
}
```

---

## 3. Job Execution States

Individual job executions follow a state machine that handles overlap, jitter, and error conditions.

### State Diagram

```
                    ┌─────────────┐
                    │   PENDING   │
                    │             │
                    │ Cron time   │
                    │ arrived     │
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
       ┌─────────────┐           ┌─────────────┐
       │   PAUSED    │           │ CHECK       │
       │             │           │ OVERLAP     │
       │ is_paused:  │           │             │
       │   true      │           │ Guard       │
       └──────┬──────┘           │ acquisition │
              │                  └──────┬──────┘
              │                         │
              │           ┌─────────────┴─────────────┐
              │           │                           │
              │           ▼                           ▼
              │    ┌─────────────┐            ┌─────────────┐
              │    │  SKIPPED    │            │ APPLY       │
              │    │             │            │ JITTER      │
              │    │ Overlap     │            │             │
              │    │ detected    │            │ Random      │
              │    └──────┬──────┘            │ delay       │
              │           │                   └──────┬──────┘
              │           │                          │
              │           │                          ▼
              │           │                   ┌─────────────┐
              │           │                   │   RUNNING   │
              │           │                   │             │
              │           │                   │ - Start     │
              │           │                   │   recorded  │
              │           │                   │ - Job fn    │
              │           │                   │   executing │
              │           │                   └──────┬──────┘
              │           │                          │
              │           │           ┌──────────────┴──────────────┐
              │           │           │                             │
              │           │           ▼                             ▼
              │           │    ┌─────────────┐              ┌─────────────┐
              │           │    │   SUCCESS   │              │   FAILED    │
              │           │    │             │              │             │
              │           │    │ Ok(())      │              │ Err(msg)    │
              │           │    └──────┬──────┘              └──────┬──────┘
              │           │           │                            │
              └───────────┴───────────┴────────────────────────────┘
                                      │
                                      ▼
                               ┌─────────────┐
                               │  COMPLETED  │
                               │             │
                               │ Registry    │
                               │ updated     │
                               │ Guard       │
                               │ released    │
                               └─────────────┘
```

### States

| State | Description | Registry Update |
|-------|-------------|-----------------|
| **PENDING** | Scheduled time arrived | None |
| **PAUSED** | Job disabled by operator | `JobResult::Skipped("paused")` |
| **SKIPPED** | Overlap policy prevented run | `JobResult::Skipped("overlap")` |
| **RUNNING** | Job function executing | `record_start()` called |
| **SUCCESS** | Job completed without error | `JobResult::Success` |
| **FAILED** | Job returned error | `JobResult::Failed(msg)` |
| **COMPLETED** | Final state, resources released | `record_complete()` called |

### Overlap Policy Effects

```rust
// From crates/memory-scheduler/src/overlap.rs
pub enum OverlapPolicy {
    /// Skip execution if previous run is still active
    Skip,
    /// Allow concurrent executions
    Concurrent,
}
```

| Policy | Previous Running | Behavior |
|--------|------------------|----------|
| **Skip** | Yes | New execution skipped, recorded as `Skipped("overlap")` |
| **Skip** | No | New execution proceeds normally |
| **Concurrent** | Yes or No | All executions proceed (may cause contention) |

The `OverlapGuard` uses atomic operations to ensure thread-safe acquisition:

```rust
// From crates/memory-scheduler/src/overlap.rs
pub fn try_acquire(&self) -> Option<RunGuard> {
    match self.policy {
        OverlapPolicy::Skip => {
            // Atomic compare-exchange: false → true
            if self.is_running.compare_exchange(
                false, true,
                Ordering::SeqCst, Ordering::SeqCst
            ).is_ok() {
                Some(RunGuard { flag: self.is_running.clone() })
            } else {
                None  // Already running, skip
            }
        }
        OverlapPolicy::Concurrent => {
            Some(RunGuard { flag: Arc::new(AtomicBool::new(true)) })
        }
    }
}
```

### Jitter Delay

Jitter adds a random delay before execution to prevent thundering herd:

```rust
// From crates/memory-scheduler/src/jitter.rs
if max_jitter_secs > 0 {
    let jitter_duration = jitter_config.generate_jitter();
    if !jitter_duration.is_zero() {
        tokio::time::sleep(jitter_duration).await;
    }
}
```

| Configuration | Effect |
|---------------|--------|
| `JitterConfig::none()` | No delay, immediate execution |
| `JitterConfig::new(30)` | Random delay 0-30 seconds |

### Pause and Resume

Jobs can be paused/resumed via the scheduler:

```rust
// From crates/memory-scheduler/src/scheduler.rs
pub fn pause_job(&self, job_name: &str) -> Result<(), SchedulerError> {
    self.registry.set_paused(job_name, true);
}

pub fn resume_job(&self, job_name: &str) -> Result<(), SchedulerError> {
    self.registry.set_paused(job_name, false);
}
```

Paused jobs check their status at execution time:

```rust
if registry.is_paused(&name) {
    registry.record_complete(&name, JobResult::Skipped("paused".into()), 0);
    return;
}
```

---

## 4. TOC Node States

TOC (Table of Contents) nodes track the lifecycle of time-based summaries.

### State Diagram

```
                    ┌─────────────┐
                    │   DRAFT     │
                    │             │
                    │ Placeholder │
                    │ created     │
                    │ version: 1  │
                    └──────┬──────┘
                           │
                           │ Children added
                           │ Rollup triggered
                           ▼
                    ┌─────────────┐
                    │  BUILDING   │
                    │             │
                    │ Summarizer  │
                    │ processing  │
                    │ children    │
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
       ┌─────────────┐           ┌─────────────┐
       │  COMPLETE   │           │   FAILED    │
       │             │           │             │
       │ Summary     │           │ Summarizer  │
       │ generated   │           │ error       │
       │ version: N  │           │ Retryable   │
       └─────────────┘           └─────────────┘
              │
              │ New children arrive
              │ (triggers re-rollup)
              ▼
       ┌─────────────┐
       │  COMPLETE   │
       │             │
       │ version: N+1│
       │ Appended    │
       └─────────────┘
```

### States

| State | Version | Summary | Description |
|-------|---------|---------|-------------|
| **DRAFT** | 1 | Placeholder | Initial node with pending summary |
| **BUILDING** | N | Processing | Rollup job aggregating children |
| **COMPLETE** | N | Generated | Full summary with bullets/keywords |
| **FAILED** | N | Unchanged | Summarization error, will retry |

### Version Transitions (TOC-06)

TOC nodes use append-only versioning:

```rust
// From crates/memory-storage/src/db.rs
pub fn put_toc_node(&self, node: &TocNode) -> Result<(), StorageError> {
    // Get current version
    let current_version = self.db.get_cf(&latest_cf, &latest_key)?
        .map(|b| { /* parse version */ });

    // Append new version
    let versioned_key = format!("{}:v{}", node.node_id, node.version);
    batch.put_cf(&nodes_cf, versioned_key, node_bytes);

    // Update latest pointer
    batch.put_cf(&latest_cf, latest_key, version_bytes);
}
```

### Rollup Triggers

The rollup job processes nodes based on:

1. **Time-based**: Cron schedule (hourly for day, daily for week/month)
2. **Minimum age**: Period must be closed (e.g., 1 hour old for day rollup)
3. **Children present**: Node must have child nodes to aggregate

```rust
// From crates/memory-toc/src/rollup.rs
let cutoff_time = Utc::now() - self.min_age;
if node.end_time > cutoff_time {
    continue;  // Skip - period not yet closed
}
let children = self.storage.get_child_nodes(&node.node_id)?;
if children.is_empty() {
    continue;  // Skip - no children to roll up
}
```

---

## 5. Outbox Entry States

The outbox implements at-least-once delivery for async processing tasks.

### State Diagram

```
                    ┌─────────────┐
        Event ────▶│   PENDING   │
        ingested   │             │
                   │ Written     │
                   │ atomically  │
                   │ with event  │
                   └──────┬──────┘
                          │
                          │ Background worker
                          │ polls outbox
                          ▼
                   ┌─────────────┐
                   │ PROCESSING  │
                   │             │
                   │ Worker      │
                   │ executing   │
                   │ action      │
                   └──────┬──────┘
                          │
             ┌────────────┴────────────┐
             │                         │
             ▼                         ▼
      ┌─────────────┐          ┌─────────────┐
      │  COMPLETED  │          │   FAILED    │
      │             │          │             │
      │ Entry       │          │ Remains in  │
      │ deleted     │          │ outbox for  │
      │             │          │ retry       │
      └─────────────┘          └─────────────┘
                                      │
                                      │ Retry on next
                                      │ worker poll
                                      ▼
                               ┌─────────────┐
                               │ PROCESSING  │
                               │ (retry)     │
                               └─────────────┘
```

### States

| State | Location | Description |
|-------|----------|-------------|
| **PENDING** | Outbox CF | Entry written, awaiting worker |
| **PROCESSING** | In-memory | Worker executing action |
| **COMPLETED** | Deleted | Successfully processed |
| **FAILED** | Outbox CF | Error occurred, will retry |

### Outbox Actions

```rust
// From crates/memory-types/src/outbox.rs
pub enum OutboxAction {
    /// Index this event for BM25/vector search
    IndexEvent,
    /// Update TOC node with new event
    UpdateToc,
}
```

### Atomic Write Guarantee (ING-05)

Events and outbox entries are written atomically:

```rust
// From crates/memory-storage/src/db.rs
let mut batch = WriteBatch::default();
batch.put_cf(&events_cf, event_key.to_bytes(), event_bytes);
batch.put_cf(&outbox_cf, outbox_key.to_bytes(), outbox_bytes);
self.db.write(batch)?;  // Atomic commit
```

This ensures:
- If the event is written, the outbox entry is also written
- If the process crashes, recovery sees both or neither
- No "orphan" events that are never indexed

### Checkpoint Recovery (STOR-03)

Rollup jobs use checkpoints for crash recovery:

```rust
// From crates/memory-toc/src/rollup.rs
pub struct RollupCheckpoint {
    pub job_name: String,
    pub level: TocLevel,
    pub last_processed_time: DateTime<Utc>,
    pub processed_count: usize,
    pub created_at: DateTime<Utc>,
}
```

Recovery flow:
1. Job starts, loads checkpoint from storage
2. Uses `last_processed_time` to skip already-processed items
3. After each item, saves new checkpoint
4. If crash occurs, restart continues from checkpoint

### At-Least-Once Delivery

The outbox pattern guarantees at-least-once processing:

| Scenario | Behavior |
|----------|----------|
| Normal completion | Entry deleted after successful processing |
| Worker crash mid-processing | Entry remains, reprocessed on restart |
| Duplicate processing | Actions must be idempotent |

---

## 6. gRPC Connection States

Client connections follow a state machine for reliability.

### State Diagram

```
                    ┌─────────────┐
                    │DISCONNECTED │◀──────────────────┐
                    │             │                   │
                    │ No channel  │                   │
                    └──────┬──────┘                   │
                           │                          │
                           │ connect()                │
                           ▼                          │
                    ┌─────────────┐                   │
                    │ CONNECTING  │                   │
                    │             │                   │
                    │ Channel     │                   │
                    │ establishing│                   │
                    └──────┬──────┘                   │
                           │                          │
              ┌────────────┴────────────┐             │
              │                         │             │
              ▼                         ▼             │
       ┌─────────────┐           ┌─────────────┐      │
       │  CONNECTED  │           │ CONN_FAILED │──────┘
       │             │           │             │
       │ Ready for   │           │ Transport   │
       │ requests    │           │ error       │
       └──────┬──────┘           └─────────────┘
              │
              │ Request initiated
              ▼
       ┌─────────────┐
       │  STREAMING  │
       │             │
       │ Active RPC  │
       │ in progress │
       └──────┬──────┘
              │
              │ Response received or error
              ▼
       ┌─────────────┐
       │  CONNECTED  │◀─── Ready for next request
       └─────────────┘
```

### States

| State | Channel | Description |
|-------|---------|-------------|
| **DISCONNECTED** | None | No connection established |
| **CONNECTING** | Creating | TCP handshake in progress |
| **CONNECTED** | Active | Ready to send requests |
| **STREAMING** | Active + RPC | Request/response in flight |
| **CONN_FAILED** | None | Connection attempt failed |

### Connection Flow

```rust
// From crates/memory-client/src/client.rs
pub async fn connect(endpoint: &str) -> Result<Self, ClientError> {
    let inner = MemoryServiceClient::connect(endpoint.to_string())
        .await
        .map_err(ClientError::Connection)?;
    Ok(Self { inner })
}
```

### Error Handling

| Error Type | State Transition | Recovery |
|------------|------------------|----------|
| DNS failure | CONNECTING → CONN_FAILED | Retry with backoff |
| TCP timeout | CONNECTING → CONN_FAILED | Retry with backoff |
| TLS error | CONNECTING → CONN_FAILED | Check certificates |
| Server unavailable | CONNECTED → DISCONNECTED | Reconnect |
| Request timeout | STREAMING → CONNECTED | Retry request |

### Health Check Integration

The server reports health via tonic-health:

```rust
// From crates/memory-service/src/server.rs
let (mut health_reporter, health_service) = health_reporter();
health_reporter
    .set_serving::<MemoryServiceServer<MemoryServiceImpl>>()
    .await;
```

Clients can use health checks to verify connectivity:

| Health Status | Meaning |
|---------------|---------|
| `SERVING` | Server ready for requests |
| `NOT_SERVING` | Server shutting down or unhealthy |
| `UNKNOWN` | Health service not configured |

---

## State Machine Interactions

### Daemon Shutdown Cascade

When the daemon receives a shutdown signal, states cascade:

```
SIGTERM received
      │
      ▼
Daemon: RUNNING → STOPPING
      │
      ▼
Scheduler: RUNNING → STOPPING
      │
      ├─── Job 1: RUNNING → wait → COMPLETED
      ├─── Job 2: RUNNING → wait → COMPLETED
      └─── Job 3: PENDING → cancelled
      │
      ▼
Scheduler: STOPPING → IDLE
      │
      ▼
gRPC: drain connections
      │
      ▼
Daemon: STOPPING → STOPPED
```

### Event Ingestion Flow

When a new event arrives:

```
Event received via gRPC
      │
      ▼
Event: stored in events CF
Outbox: PENDING
      │
      ├──────────────────────────────────┐
      │                                  │
      ▼ (async)                          │
Background worker                        │
polls outbox                             │
      │                                  │
      ▼                                  │
Outbox: PROCESSING                       │
      │                                  │
      ├─── TOC update                    │
      │         │                        │
      │         ▼                        │
      │    TOC: DRAFT → BUILDING         │
      │         │                        │
      │         ▼                        │
      │    TOC: BUILDING → COMPLETE      │
      │                                  │
      ▼                                  │
Outbox: COMPLETED (deleted)              │
                                         │
gRPC response ◀──────────────────────────┘
```

---

## Monitoring State Machines

### Job Status via CLI

```bash
memory-daemon scheduler status
```

Output shows current job states:

```
JOB                  STATUS       LAST RUN             NEXT RUN             RUNS       ERRORS
────────────────────────────────────────────────────────────────────────────────────────────
rollup-day           IDLE         2024-01-29 15:00     2024-01-29 16:00     24         0
rollup-week          PAUSED       2024-01-28 00:00     -                    4          0
compaction           RUNNING      2024-01-29 15:30     2024-01-29 16:30     12         1
```

### Observability Points

| Component | Observable State | How to Check |
|-----------|------------------|--------------|
| Daemon | PID file exists | `cat ~/.cache/agent-memory/daemon.pid` |
| Scheduler | `is_running()` | `GetSchedulerStatus` RPC |
| Job | `JobStatus` struct | `GetSchedulerStatus` RPC |
| gRPC | Health check | `grpc_health_probe` tool |
| Storage | Stats | `memory-daemon admin stats` |

---

## Error States and Recovery

### Scheduler Errors

| Error | Recovery |
|-------|----------|
| `AlreadyRunning` | Check if start was already called |
| `NotRunning` | Call start before shutdown |
| `InvalidCron` | Fix cron expression syntax |
| `InvalidTimezone` | Use valid IANA timezone |
| `JobNotFound` | Verify job was registered |

### Storage Errors

| Error | Recovery |
|-------|----------|
| `DatabaseError` | Check disk space, permissions |
| `ColumnFamilyNotFound` | Database may be corrupted |
| `SerializationError` | Data format mismatch |

### Connection Errors

| Error | Recovery |
|-------|----------|
| `Connection` (tonic) | Verify daemon is running |
| `Timeout` | Increase timeout, check network |
| `Cancelled` | Request was cancelled |

---

## Summary

The agent-memory system uses state machines to manage:

1. **Daemon lifecycle**: Clean startup and graceful shutdown with signal handling
2. **Scheduler states**: Control over background job execution
3. **Job execution**: Overlap prevention, jitter, pause/resume
4. **TOC nodes**: Versioned, append-only summary generation
5. **Outbox entries**: At-least-once async task processing
6. **gRPC connections**: Reliable client connectivity

Understanding these states helps with:
- **Debugging**: Identify where failures occur
- **Monitoring**: Build dashboards around state transitions
- **Operations**: Know what to expect during shutdown/restart
- **Extension**: Add new components following established patterns
