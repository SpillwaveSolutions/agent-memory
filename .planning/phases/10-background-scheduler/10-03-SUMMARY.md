---
phase: 10-background-scheduler
plan: 03
subsystem: scheduler
tags: [scheduler, rollup, compaction, daemon, jobs]
dependencies:
  requires: ["10-01", "10-02"]
  provides: ["scheduled-rollup-jobs", "scheduled-compaction-job", "daemon-scheduler-integration"]
  affects: ["10-04"]
tech-stack:
  added: []
  patterns: ["job-factory-pattern", "scheduler-wiring"]
key-files:
  created:
    - crates/memory-scheduler/src/jobs/mod.rs
    - crates/memory-scheduler/src/jobs/rollup.rs
    - crates/memory-scheduler/src/jobs/compaction.rs
  modified:
    - crates/memory-scheduler/Cargo.toml
    - crates/memory-scheduler/src/lib.rs
    - crates/memory-daemon/Cargo.toml
    - crates/memory-daemon/src/commands.rs
    - crates/memory-service/src/server.rs
    - crates/memory-service/src/ingest.rs
    - crates/memory-service/src/lib.rs
decisions:
  - id: "JOBS-01"
    choice: "Optional jobs feature with default enabled"
    reason: "Allows scheduler crate to be used without heavy memory-toc dependency if needed"
  - id: "JOBS-02"
    choice: "MockSummarizer for rollup jobs by default"
    reason: "Production should load ApiSummarizer from config; allows daemon to start without API keys"
  - id: "JOBS-03"
    choice: "run_server_with_scheduler as new server function"
    reason: "Preserves backward compatibility with existing run_server_with_shutdown"
metrics:
  duration: "~19min"
  completed: "2026-01-31"
---

# Phase 10 Plan 03: TOC Rollup Jobs Summary

Jobs module wiring existing memory-toc rollups to scheduler, integrated into daemon startup.

## One-Liner

TOC day/week/month rollup jobs plus compaction job wired to scheduler with daemon startup integration.

## What Was Built

### Task 1: Rollup Job Definitions
- Created `jobs` module in memory-scheduler with rollup and compaction submodules
- `RollupJobConfig` with configurable cron schedules:
  - Day rollup: 1 AM daily (`0 0 1 * * *`)
  - Week rollup: 2 AM Sunday (`0 0 2 * * 0`)
  - Month rollup: 3 AM 1st of month (`0 0 3 1 * *`)
- `create_rollup_jobs()` registers three jobs using existing `memory_toc::rollup::RollupJob`
- All jobs use `OverlapPolicy::Skip` and configurable jitter (default 5 min)

### Task 2: Compaction Job
- `CompactionJobConfig` with weekly schedule (4 AM Sunday)
- `create_compaction_job()` registers RocksDB compaction via `Storage::compact()`
- Uses 10 minute jitter to spread load

### Task 3: Daemon Integration
- Updated `start_daemon()` to create and start `SchedulerService`
- Registers rollup and compaction jobs on startup
- Added `run_server_with_scheduler()` function to memory-service
- Updated `MemoryServiceImpl` with `with_scheduler()` constructor
- Wired scheduler gRPC service handlers (GetSchedulerStatus, PauseJob, ResumeJob)
- Scheduler starts before gRPC server, graceful shutdown when server stops

## Key Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| JOBS-01 | Optional "jobs" feature (enabled by default) | Allows scheduler without heavy memory-toc dependency |
| JOBS-02 | MockSummarizer for rollup jobs | Production should configure ApiSummarizer; allows daemon to start without API keys |
| JOBS-03 | New run_server_with_scheduler function | Backward compatible with existing shutdown-only function |

## Technical Details

### Job Registration Pattern
```rust
create_rollup_jobs(
    &scheduler,
    storage.clone(),
    summarizer,
    RollupJobConfig::default(),
).await?;

create_compaction_job(
    &scheduler,
    storage.clone(),
    CompactionJobConfig::default(),
).await?;
```

### Server Integration Pattern
```rust
let scheduler = SchedulerService::new(SchedulerConfig::default()).await?;
// ... register jobs ...
let result = run_server_with_scheduler(addr, storage, scheduler, shutdown_signal).await;
```

### Feature Flag Structure
```toml
[features]
default = ["jobs"]
jobs = ["memory-toc", "memory-storage", "memory-types"]
```

## File Changes

| File | Change Type | Description |
|------|-------------|-------------|
| memory-scheduler/Cargo.toml | Modified | Added jobs feature and dependencies |
| memory-scheduler/src/lib.rs | Modified | Export jobs module and job functions |
| memory-scheduler/src/jobs/mod.rs | Created | Jobs module root with rollup and compaction |
| memory-scheduler/src/jobs/rollup.rs | Created | TOC rollup job definitions |
| memory-scheduler/src/jobs/compaction.rs | Created | RocksDB compaction job definition |
| memory-daemon/Cargo.toml | Modified | Added memory-scheduler and memory-toc deps |
| memory-daemon/src/commands.rs | Modified | Scheduler initialization in start_daemon |
| memory-service/src/server.rs | Modified | Added run_server_with_scheduler |
| memory-service/src/ingest.rs | Modified | Added with_scheduler constructor |
| memory-service/src/lib.rs | Modified | Export run_server_with_scheduler |

## Commits

| Hash | Message |
|------|---------|
| 580f2cb | feat(10-03): add rollup and compaction job definitions |
| 64b0ed0 | feat(10-03): integrate scheduler into daemon startup |

## Verification

- All 58 memory-scheduler tests pass
- Daemon builds successfully
- CLI shows scheduler subcommand: `memory-daemon scheduler status`
- Job registration logs on startup: "Scheduler initialized with 4 jobs"

## Deviations from Plan

None - plan executed exactly as written.

## Next Phase Readiness

Phase 10-04 (Job Observability) can proceed:
- SchedulerGrpcService already wired into MemoryServiceImpl
- Registry provides get_all_status() for job status queries
- gRPC handlers (GetSchedulerStatus, PauseJob, ResumeJob) are functional
- CLI scheduler subcommand exists with status/pause/resume commands
