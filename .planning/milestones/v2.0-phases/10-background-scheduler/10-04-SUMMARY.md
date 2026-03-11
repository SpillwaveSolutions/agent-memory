---
phase: 10-background-scheduler
plan: 04
subsystem: infra
tags: [grpc, cli, scheduler, observability, job-status]

# Dependency graph
requires:
  - phase: 10-02-job-registry
    provides: JobRegistry, JobStatus, JobResult
provides:
  - GetSchedulerStatus gRPC RPC for job status query
  - PauseJob/ResumeJob gRPC RPCs for job control
  - CLI scheduler subcommand (status/pause/resume)
  - SchedulerGrpcService for gRPC handler delegation
affects: [10-03-rollup-jobs, memory-setup-plugin]

# Tech tracking
tech-stack:
  added: []
  patterns: [grpc-scheduler-service, cli-subcommand-handler]

key-files:
  created:
    - crates/memory-service/src/scheduler_service.rs
  modified:
    - proto/memory.proto
    - crates/memory-service/src/ingest.rs
    - crates/memory-service/src/lib.rs
    - crates/memory-daemon/src/cli.rs
    - crates/memory-daemon/src/lib.rs
    - crates/memory-daemon/src/main.rs

key-decisions:
  - "JobStatusProto uses Proto suffix to avoid name conflict with domain JobStatus"
  - "Scheduler RPCs return success/error response rather than gRPC errors for pause/resume"
  - "CLI uses gRPC client to query daemon rather than direct storage access"
  - "Timestamps formatted as local time for human readability in CLI"

patterns-established:
  - "SchedulerGrpcService delegation: MemoryServiceImpl delegates to SchedulerGrpcService when scheduler is configured"
  - "CLI scheduler subcommand: status shows table, pause/resume take job_name argument"

# Metrics
duration: 24min
completed: 2026-01-31
---

# Phase 10 Plan 04: Job Observability Summary

**Scheduler status gRPC RPC and CLI commands for job monitoring, pause, and resume**

## Performance

- **Duration:** 24 min
- **Started:** 2026-02-01T02:02:31Z
- **Completed:** 2026-02-01T03:26:42Z
- **Tasks:** 3
- **Files created:** 1
- **Files modified:** 7
- **Total tests:** 57 (22 memory-service + 15 memory-daemon + 13 integration + 7 scheduler-service)

## Accomplishments
- Added GetSchedulerStatus, PauseJob, ResumeJob RPCs to proto with JobStatusProto message
- Implemented SchedulerGrpcService with all scheduler RPC handlers
- Added CLI scheduler subcommand with status, pause, resume commands
- Integrated scheduler service into MemoryServiceImpl with optional scheduler support
- Added formatted job status table output with RUNNING/PAUSED/IDLE states

## Task Commits

Each task was committed atomically:

1. **Task 1: Add scheduler proto messages and RPC** - `39fa8c7` (feat)
   - JobResultStatus enum, JobStatusProto message
   - GetSchedulerStatus, PauseJob, ResumeJob request/response messages
   - Placeholder implementations in MemoryServiceImpl

2. **Task 2: Implement gRPC scheduler service** - `e53a7f5` (feat)
   - scheduler_service.rs with SchedulerGrpcService
   - job_result_to_proto conversion helper
   - 7 unit tests for scheduler gRPC operations

3. **Task 3: Add CLI scheduler commands** - `20b1492` (feat)
   - SchedulerCommands enum (Status, Pause, Resume)
   - handle_scheduler function with formatted table output
   - 4 CLI tests for scheduler commands

## Files Created/Modified
- `proto/memory.proto` - Added scheduler messages and RPCs (SCHED-05)
- `crates/memory-service/src/scheduler_service.rs` - SchedulerGrpcService implementation (NEW)
- `crates/memory-service/src/ingest.rs` - Added scheduler RPC delegation to SchedulerGrpcService
- `crates/memory-service/src/lib.rs` - Added scheduler_service module and exports
- `crates/memory-service/Cargo.toml` - Added memory-scheduler dependency
- `crates/memory-daemon/src/cli.rs` - Added SchedulerCommands and Scheduler command
- `crates/memory-daemon/src/lib.rs` - Exported SchedulerCommands and handle_scheduler
- `crates/memory-daemon/src/main.rs` - Handle Scheduler command in main match

## Decisions Made
- **JobStatusProto name:** Used Proto suffix to avoid conflict with domain JobStatus type
- **Response-level errors:** Pause/Resume return success=false with error message rather than gRPC errors for non-existent jobs
- **CLI table format:** 92-character wide table with JOB, STATUS, LAST RUN, NEXT RUN, RUNS, ERRORS columns
- **Local time in CLI:** Timestamps formatted as local time (YYYY-MM-DD HH:MM) for human readability

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Binary was in architecture-specific target directory (target/aarch64-apple-darwin/debug/) rather than target/debug/ - adjusted verification commands accordingly

## API Summary

```rust
// CLI usage
memory-daemon scheduler status                 # Show all job statuses
memory-daemon scheduler pause hourly-rollup    # Pause a job
memory-daemon scheduler resume hourly-rollup   # Resume a job

// gRPC usage
let response = client.get_scheduler_status(GetSchedulerStatusRequest {}).await?;
println!("Scheduler running: {}", response.scheduler_running);
for job in response.jobs {
    println!("{}: {}", job.job_name, if job.is_paused { "PAUSED" } else { "IDLE" });
}
```

## Next Phase Readiness
- Job observability complete - can monitor and control scheduler via CLI and gRPC
- All Phase 10 plans now complete (infrastructure, registry, rollup jobs, observability)
- Ready for Phase 11 (Teleport - BM25 search)

---
*Phase: 10-background-scheduler*
*Completed: 2026-01-31*
