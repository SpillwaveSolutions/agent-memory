---
phase: 10-background-scheduler
plan: 02
subsystem: infra
tags: [job-registry, overlap-policy, jitter, status-tracking, concurrency]

# Dependency graph
requires:
  - phase: 10-01-scheduler-infrastructure
    provides: SchedulerService, tokio-cron-scheduler wrapper
provides:
  - JobRegistry for job status tracking
  - OverlapPolicy with Skip and Concurrent modes
  - JitterConfig for random delays
  - Integrated register_job method with full lifecycle management
  - Pause/resume job control
affects: [10-03-rollup-jobs, 10-04-job-observability]

# Tech tracking
tech-stack:
  added: [rand 0.8]
  patterns: [job-registry, overlap-guard, jitter-delay, raii-run-guard]

key-files:
  created:
    - crates/memory-scheduler/src/registry.rs
    - crates/memory-scheduler/src/overlap.rs
    - crates/memory-scheduler/src/jitter.rs
  modified:
    - crates/memory-scheduler/src/lib.rs
    - crates/memory-scheduler/src/scheduler.rs
    - crates/memory-scheduler/Cargo.toml

key-decisions:
  - "JobRegistry uses RwLock<HashMap> for thread-safe status tracking"
  - "OverlapPolicy::Skip is the default - prevents job pileup"
  - "OverlapGuard uses AtomicBool for lock-free running state"
  - "RunGuard RAII pattern ensures running flag is released on drop/panic"
  - "JitterConfig generates random delay in milliseconds for fine-grained control"
  - "register_job() checks is_paused before attempting overlap guard acquisition"

patterns-established:
  - "RAII RunGuard: Automatic release of overlap lock when dropped"
  - "Job execution tracking: record_start -> job execution -> record_complete"
  - "Pause check first: Check registry.is_paused() before acquiring overlap guard"
  - "Jitter in-job: Apply jitter delay after acquiring guard, not before"

# Metrics
duration: 10min
completed: 2026-01-31
---

# Phase 10 Plan 02: Job Registry and Lifecycle Summary

**JobRegistry for status tracking with OverlapPolicy (Skip/Concurrent) and JitterConfig for random delays, integrated into SchedulerService.register_job()**

## Performance

- **Duration:** 10 min
- **Started:** 2026-01-31
- **Completed:** 2026-01-31
- **Tasks:** 3
- **Files created:** 3
- **Files modified:** 3
- **Total tests:** 54

## Accomplishments
- Implemented JobRegistry with thread-safe RwLock<HashMap> storage
- Created JobStatus struct tracking last_run, next_run, duration, run_count, error_count
- Added JobResult enum: Success, Failed, Skipped for execution outcomes
- Implemented OverlapPolicy with Skip (default) and Concurrent modes
- Created OverlapGuard with atomic lock-free running state
- Added RunGuard RAII type for automatic release on drop
- Implemented JitterConfig for configurable random delays
- Added with_jitter async helper for delayed execution
- Integrated registry, overlap, and jitter into register_job() method
- Added pause_job() and resume_job() for job lifecycle control

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement JobRegistry for status tracking** - `aa27631` (feat)
   - registry.rs with JobRegistry, JobStatus, JobResult
   - 11 tests for thread safety and all operations

2. **Task 2: Implement OverlapPolicy and jitter utilities** - `47a5cf9` (feat)
   - overlap.rs with OverlapPolicy, OverlapGuard, RunGuard
   - jitter.rs with JitterConfig and with_jitter helper
   - 16 tests for overlap and jitter behavior

3. **Task 3: Integrate registry with SchedulerService** - `bf33349` (feat)
   - Added Arc<JobRegistry> to SchedulerService
   - Added register_job() with full lifecycle support
   - Added pause_job() and resume_job() methods
   - 11 integration tests

## Files Created/Modified
- `crates/memory-scheduler/src/registry.rs` - JobRegistry, JobStatus, JobResult (NEW)
- `crates/memory-scheduler/src/overlap.rs` - OverlapPolicy, OverlapGuard, RunGuard (NEW)
- `crates/memory-scheduler/src/jitter.rs` - JitterConfig, with_jitter (NEW)
- `crates/memory-scheduler/src/lib.rs` - Added module declarations and exports
- `crates/memory-scheduler/src/scheduler.rs` - Added registry field and register_job method
- `crates/memory-scheduler/Cargo.toml` - Added rand dependency

## Decisions Made
- **RwLock<HashMap> for registry:** Allows multiple concurrent readers for status queries while serializing writes
- **OverlapPolicy::Skip as default:** Safer default - prevents resource exhaustion from job pileup
- **AtomicBool for overlap guard:** Lock-free performance for checking running state
- **RAII RunGuard:** Ensures running flag is cleared even if job panics
- **Jitter in milliseconds:** Allows fine-grained control (0-N*1000ms range)
- **Pause check before overlap:** Paused jobs don't acquire the overlap guard at all

## Deviations from Plan

**[Rule 1 - Bug] Fixed clippy warning**
- **Found during:** Task 3 verification
- **Issue:** clippy complained about derivable_impls for JitterConfig::Default
- **Fix:** Changed manual impl Default to #[derive(Default)]
- **Files modified:** jitter.rs
- **Commit:** bf33349

## Issues Encountered
- Clippy warning for derivable Default impl - resolved by using derive macro

## API Summary

```rust
// Create scheduler
let scheduler = SchedulerService::new(config).await?;

// Register job with full lifecycle management
scheduler.register_job(
    "my-job",
    "0 0 * * * *",      // Cron expression
    None,               // Use default timezone
    OverlapPolicy::Skip, // Skip if already running
    JitterConfig::new(30), // Up to 30s random delay
    || async { do_work().await },
).await?;

// Check job status
let registry = scheduler.registry();
let status = registry.get_status("my-job").unwrap();
println!("Run count: {}", status.run_count);

// Pause/resume jobs
scheduler.pause_job("my-job")?;
scheduler.resume_job("my-job")?;
```

## Next Phase Readiness
- Job registry and lifecycle management complete
- Ready for TOC rollup jobs (Plan 03) to wire existing rollups to scheduler
- register_job() provides full observability via registry
- Overlap policy prevents rollup job pileup

---
*Phase: 10-background-scheduler*
*Completed: 2026-01-31*
