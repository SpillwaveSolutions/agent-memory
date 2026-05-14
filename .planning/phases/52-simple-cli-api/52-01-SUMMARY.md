---
phase: 52-simple-cli-api
plan: 01
subsystem: cli
tags: [clap, grpc, json-envelope, tty-detection, memory-cli]

requires:
  - phase: 51-retrieval-orchestrator
    provides: MemoryOrchestrator, RouteQuery RPC definition in proto

provides:
  - memory-cli crate with memory binary (6 subcommands)
  - JsonEnvelope output formatting with TTY-aware printing
  - route_query() method on MemoryClient
  - connect_client helper with actionable daemon-not-running error
  - CLI parsing infrastructure (Cli, GlobalArgs, Commands enum)

affects: [52-02, 52-03, memory-cli commands]

tech-stack:
  added: [memory-cli crate]
  patterns: [clap derive subcommands, JsonEnvelope output envelope, TTY detection via IsTerminal]

key-files:
  created:
    - crates/memory-cli/Cargo.toml
    - crates/memory-cli/src/main.rs
    - crates/memory-cli/src/cli.rs
    - crates/memory-cli/src/output.rs
    - crates/memory-cli/src/client.rs
    - crates/memory-cli/src/commands/mod.rs
    - crates/memory-cli/src/commands/search.rs
    - crates/memory-cli/src/commands/context.rs
    - crates/memory-cli/src/commands/recall.rs
    - crates/memory-cli/src/commands/add.rs
    - crates/memory-cli/src/commands/timeline.rs
    - crates/memory-cli/src/commands/summary.rs
  modified:
    - Cargo.toml
    - crates/memory-client/src/client.rs
    - crates/memory-client/src/lib.rs

key-decisions:
  - "All CLI commands route through gRPC (no direct RocksDB access) to avoid lock conflicts"
  - "Errors printed as JSON to stderr for programmatic consumption"
  - "dead_code suppressed on scaffold functions pending command implementation in 52-02/52-03"

patterns-established:
  - "JsonEnvelope: ok/error/context_ok constructors with builder pattern for meta"
  - "should_force_json checks global and command format args"
  - "print_output uses IsTerminal for TTY detection"

requirements-completed: [CLI-01, CLI-05, CLI-09, CLI-10]

duration: 6min
completed: 2026-03-22
---

# Phase 52 Plan 01: CLI Scaffold Summary

**memory-cli crate with clap derive parsing, JsonEnvelope output, TTY detection, and route_query gRPC method**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-22T05:03:02Z
- **Completed:** 2026-03-22T05:09:04Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments
- Scaffolded memory-cli crate producing `memory` binary with all 6 subcommands (search, context, add, timeline, summary, recall)
- Added route_query() method to MemoryClient for orchestrated retrieval via gRPC
- Implemented JsonEnvelope with ok/error/context_ok constructors, skip_serializing_if, TTY-aware print_output
- 25 unit tests passing (11 CLI parsing + 14 output/serialization)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add route_query() to MemoryClient and scaffold memory-cli crate** - `4060b27` (feat)
2. **Task 2: Implement JsonEnvelope, Meta, TTY-aware output, and client helper** - `649ed37` (feat)

## Files Created/Modified
- `crates/memory-cli/Cargo.toml` - Crate manifest with [[bin]] name = "memory"
- `crates/memory-cli/src/cli.rs` - Clap derive structs for all 6 subcommands
- `crates/memory-cli/src/output.rs` - JsonEnvelope, Meta, print_output with TTY detection
- `crates/memory-cli/src/client.rs` - connect_client helper with actionable error
- `crates/memory-cli/src/main.rs` - Binary entrypoint with tracing and error handling
- `crates/memory-cli/src/commands/*.rs` - Stub files for 6 command implementations
- `crates/memory-client/src/client.rs` - Added route_query() method
- `crates/memory-client/src/lib.rs` - Re-exported RouteQueryResponse
- `Cargo.toml` - Added memory-cli to workspace members

## Decisions Made
- All CLI commands route through gRPC to avoid RocksDB lock conflicts with daemon
- Errors printed as JSON to stderr (not stdout) for programmatic consumption
- dead_code suppressed on scaffold functions that will be used by command implementations in plans 02 and 03

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CLI scaffold complete with all types and helpers ready for command implementations
- Plans 52-02 and 52-03 can implement search/context/recall and add/timeline/summary commands respectively

---
*Phase: 52-simple-cli-api*
*Completed: 2026-03-22*
