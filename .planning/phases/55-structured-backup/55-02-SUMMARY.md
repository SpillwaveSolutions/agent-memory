---
phase: 55-structured-backup
plan: 02
subsystem: cli
tags: [grpc, streaming, jsonl, backup, cli, clap]

# Dependency graph
requires:
  - phase: 55-structured-backup plan 01
    provides: ExportBackup streaming RPC, BackupChunk/BackupOptions proto messages, server-side backup handler
provides:
  - memory backup CLI command with --events-only, --since, --until, --dir flags
  - Streaming gRPC client method (export_backup) for backup consumption
  - Per-day event JSONL routing with overwrite semantics
  - BackupChunkType re-export from memory-client crate
affects: [56-import-bootstrap, cli-testing, documentation]

# Tech tracking
tech-stack:
  added: []
  patterns: [tonic Streaming client consumption, chunk-type routing to filesystem, time-spec parsing (24h/7d/YYYY-MM-DD)]

key-files:
  created:
    - crates/memory-cli/src/commands/backup.rs
  modified:
    - crates/memory-client/src/client.rs
    - crates/memory-client/src/lib.rs
    - crates/memory-cli/src/cli.rs
    - crates/memory-cli/src/commands/mod.rs
    - crates/memory-cli/src/main.rs

key-decisions:
  - "BackupChunkType re-exported from memory-client lib.rs for CLI access"
  - "Per-day event files use overwrite (not append) for incremental backup correctness"
  - "Time spec parser supports 24h, 7d, and YYYY-MM-DD formats"

patterns-established:
  - "Streaming RPC client consumption: export_backup returns Streaming<BackupChunk>, caller iterates with stream.message().await"
  - "Chunk-type routing: match on proto enum to dispatch JSONL data to correct filesystem paths"

requirements-completed: [BACKUP-01, BACKUP-02, BACKUP-03, BACKUP-04, BACKUP-05]

# Metrics
duration: 6min
completed: 2026-03-24
---

# Phase 55 Plan 02: Backup CLI Command Summary

**`memory backup` CLI command consuming ExportBackup streaming RPC with per-day event JSONL routing, --events-only/--since/--until/--dir flags, and overwrite semantics**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-24T02:41:06Z
- **Completed:** 2026-03-24T02:47:29Z
- **Tasks:** 1
- **Files modified:** 8

## Accomplishments
- Streaming client method `export_backup` added to memory-client crate returning `Streaming<BackupChunk>`
- Full `memory backup` CLI command with --events-only, --since, --until, --dir flags
- Chunk-type routing: events to per-day JSONL files, TOC levels to toc/*.jsonl, grips/episodes to top-level JSONL
- Per-day event files use overwrite semantics for safe incremental backups
- manifest.json pretty-printed from stream
- 81 CLI tests passing (including new backup parser and helper function tests)

## Task Commits

Each task was committed atomically:

1. **Task 1: Streaming client method + CLI Backup command** - `bf51772` (feat)

## Files Created/Modified
- `crates/memory-cli/src/commands/backup.rs` - New backup command implementation with chunk routing, time parsing, file writing
- `crates/memory-client/src/client.rs` - Added export_backup streaming client method
- `crates/memory-client/src/lib.rs` - Re-exported BackupChunkType for CLI access
- `crates/memory-cli/src/cli.rs` - Added BackupArgs struct and Backup variant to Commands enum
- `crates/memory-cli/src/commands/mod.rs` - Registered backup module
- `crates/memory-cli/src/main.rs` - Wired Backup command dispatch

## Decisions Made
- Re-exported BackupChunkType from memory-client lib.rs so CLI can match on chunk types without reaching into memory-service::pb directly
- Per-day event files use overwrite semantics (fs::write, not append) to prevent duplicate JSONL lines on incremental backups
- Time spec parser supports three formats: relative hours (24h), relative days (7d), and absolute dates (YYYY-MM-DD)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Included pre-existing formatting changes from Plan 01**
- **Found during:** Task 1 (git status showed modified files not in plan)
- **Issue:** backup.rs and episodes.rs had uncommitted `cargo fmt` changes from Plan 01 execution
- **Fix:** Included formatting-only changes in the task commit
- **Files modified:** crates/memory-service/src/backup.rs, crates/memory-storage/src/episodes.rs
- **Verification:** cargo fmt --all -- --check passes

---

**Total deviations:** 1 auto-fixed (1 bug/formatting)
**Impact on plan:** Trivial formatting cleanup from prior plan. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 55 (structured-backup) fully complete: server-side streaming RPC + client CLI command
- Ready for Phase 56 (import-bootstrap) which will consume the backup directory format
- Backup directory structure: events/*.jsonl, toc/*.jsonl, grips.jsonl, episodes.jsonl, manifest.json

---
*Phase: 55-structured-backup*
*Completed: 2026-03-24*
