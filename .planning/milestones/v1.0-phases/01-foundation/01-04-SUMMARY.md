---
phase: 01-foundation
plan: 04
subsystem: infra
tags: [cli, clap, tokio, daemon, pid-file, graceful-shutdown]

# Dependency graph
requires:
  - phase: 01-02
    provides: Settings configuration with load() and expanded_db_path()
  - phase: 01-03
    provides: run_server_with_shutdown for graceful daemon operation
provides:
  - memory-daemon binary with start/stop/status commands
  - CLI argument parsing with clap
  - Configuration precedence (defaults -> file -> env -> CLI)
  - PID file management for daemon lifecycle
  - Graceful shutdown on SIGINT/SIGTERM
affects:
  - 05-integration (daemon testing)
  - 06-demo (end-to-end usage)

# Tech tracking
tech-stack:
  added: [libc (unix signal handling)]
  patterns: [CLI command dispatch, PID file lifecycle, signal handling]

key-files:
  created:
    - crates/memory-daemon/src/cli.rs
    - crates/memory-daemon/src/commands.rs
    - crates/memory-daemon/src/lib.rs
  modified:
    - crates/memory-daemon/src/main.rs
    - crates/memory-daemon/Cargo.toml
    - crates/memory-service/src/lib.rs

key-decisions:
  - "Use directories crate for cross-platform PID file location"
  - "libc::kill for process checking and SIGTERM on Unix"
  - "Background daemonization deferred to Phase 5 (use process manager)"

patterns-established:
  - "CLI structure: global flags -> subcommand -> subcommand options"
  - "Command handlers: async start_daemon, sync stop_daemon/show_status"
  - "PID file: write on start, remove on shutdown, check for status"

# Metrics
duration: 4min
completed: 2026-01-29
---

# Phase 1 Plan 04: CLI Daemon Binary Summary

**Complete daemon binary with clap CLI, configuration loading, gRPC server startup, PID file management, and graceful shutdown**

## Performance

- **Duration:** 4 min
- **Started:** 2026-01-29T22:16:23Z
- **Completed:** 2026-01-29T22:20:13Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- CLI with start/stop/status subcommands (CLI-01)
- Configuration precedence: defaults -> file -> env -> CLI (CFG-01)
- Graceful shutdown on SIGINT/SIGTERM with PID file cleanup
- Working daemon: `memory-daemon start --foreground` serves gRPC

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement CLI argument parsing with clap** - `f7dceb9` (feat)
2. **Task 2: Implement command handlers (start, stop, status)** - `8015140` (feat)
3. **Task 3: Wire up main entry point** - `ac2b760` (feat)

## Files Created/Modified

- `crates/memory-daemon/src/cli.rs` - CLI struct with Parser/Subcommand for start/stop/status
- `crates/memory-daemon/src/commands.rs` - Command implementations with PID file management
- `crates/memory-daemon/src/lib.rs` - Library exports for Cli, Commands, and handlers
- `crates/memory-daemon/src/main.rs` - Main entry point with tokio runtime
- `crates/memory-daemon/Cargo.toml` - Added serde and libc dependencies
- `crates/memory-service/src/lib.rs` - Export run_server_with_shutdown

## Decisions Made

1. **PID file location:** Use `directories::BaseDirs::runtime_dir()` with fallback to cache_dir for cross-platform support
2. **Process checking:** Use `libc::kill(pid, 0)` on Unix to check if process exists without actually killing
3. **Background mode deferred:** Background daemonization (double-fork) deferred to Phase 5; recommend process managers (systemd, launchd) for now

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all tasks completed successfully.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 1 Foundation is now **COMPLETE**:
- Workspace scaffolding (01-00)
- RocksDB storage layer (01-01)
- Domain types (01-02)
- gRPC service with IngestEvent (01-03)
- CLI daemon binary (01-04)

Ready for Phase 2: TOC Building (semantic table of contents generation).

---
*Phase: 01-foundation*
*Completed: 2026-01-29*
