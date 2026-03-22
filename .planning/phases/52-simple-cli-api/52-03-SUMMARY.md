---
phase: 52-simple-cli-api
plan: 03
subsystem: cli
tags: [grpc, cli, toc, timeline, ingest, events]

requires:
  - phase: 52-simple-cli-api/01
    provides: CLI scaffold, GlobalArgs, JsonEnvelope, connect_client helper
provides:
  - memory add command (gRPC ingest)
  - memory timeline command (get_events RPC)
  - memory summary command (get_toc_root + browse_toc RPCs)
affects: [52-simple-cli-api, benchmarks, integration-testing]

tech-stack:
  added: [memory-service dependency for ProtoTocNode]
  patterns: [kind_to_event_type mapping, parse_range time parsing, node_overlaps filtering]

key-files:
  created: []
  modified:
    - crates/memory-cli/src/commands/add.rs
    - crates/memory-cli/src/commands/timeline.rs
    - crates/memory-cli/src/commands/summary.rs
    - crates/memory-cli/src/main.rs
    - crates/memory-cli/src/client.rs
    - crates/memory-cli/Cargo.toml

key-decisions:
  - "ProtoEvent accessed via memory_client re-export, ProtoTocNode via memory_service::pb"
  - "CLI events use EventRole::User (user-originated) with ULID session IDs prefixed cli-"
  - "Timeline entity filter is client-side (daemon get_events lacks entity filter)"
  - "Summary navigates one level deep from root (browse_toc children) for matching time range"

patterns-established:
  - "Error pattern: match on connect/RPC result, print JsonEnvelope::error, exit(1)"
  - "Range parsing: Nd/Nw numeric suffix or named keywords (day/week/month/year)"

requirements-completed: [CLI-04, CLI-07, CLI-08, CLI-09, CLI-10]

duration: 7min
completed: 2026-03-22
---

# Phase 52 Plan 03: Write & Query Commands Summary

**Add, timeline, and summary CLI commands with gRPC ingest/query, time-range parsing, and TOC navigation**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-22T05:12:06Z
- **Completed:** 2026-03-22T05:19:33Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- memory add command ingests events via gRPC with kind-to-EventType mapping and ULID-generated IDs
- memory timeline queries events by time range with optional entity filtering
- memory summary navigates TOC hierarchy (root + one level) for compressed summaries
- All commands include tokens_estimated in meta envelope and exit non-zero on failure
- Full pr-precheck passes (fmt + clippy + test + doc)
- memory-daemon crate unchanged (CLI-08 verified)

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement memory add command** - `8b6b16d` (feat)
2. **Task 2: Implement timeline and summary commands** - `53bd74c` (feat)
3. **Task 3: Full build verification and cleanup** - `53c5aae` (chore)

## Files Created/Modified
- `crates/memory-cli/src/commands/add.rs` - Add command: build_event, kind_to_event_type, gRPC ingest
- `crates/memory-cli/src/commands/timeline.rs` - Timeline command: parse_range, map_proto_event, get_events RPC
- `crates/memory-cli/src/commands/summary.rs` - Summary command: parse_summary_range, node_overlaps, TOC navigation
- `crates/memory-cli/src/main.rs` - Removed dead_code annotation on output module
- `crates/memory-cli/src/client.rs` - Removed dead_code annotation on connect_client
- `crates/memory-cli/Cargo.toml` - Added memory-service dependency

## Decisions Made
- ProtoTocNode requires direct memory-service dependency (not re-exported from memory-client)
- CLI events use EventRole::User since they originate from user CLI input
- Entity filtering for timeline is done client-side since get_events RPC lacks entity parameter
- Summary browses one level deep from root nodes that overlap the requested time range

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added memory-service dependency for ProtoTocNode**
- **Found during:** Task 2
- **Issue:** ProtoTocNode not re-exported from memory-client, needed for summary command
- **Fix:** Added memory-service workspace dependency to memory-cli Cargo.toml
- **Files modified:** crates/memory-cli/Cargo.toml
- **Verification:** cargo build -p memory-cli passes
- **Committed in:** 53bd74c

**2. [Rule 1 - Bug] Removed stale dead_code annotations**
- **Found during:** Task 3
- **Issue:** #[allow(dead_code)] on output and client modules no longer needed
- **Fix:** Removed annotations since commands now use these modules
- **Files modified:** crates/memory-cli/src/main.rs, crates/memory-cli/src/client.rs
- **Verification:** cargo clippy passes without dead_code warnings
- **Committed in:** 53c5aae

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes necessary for correct compilation and clean clippy output. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 6 CLI commands implemented (search, context, add, timeline, summary, recall)
- Full pr-precheck passes
- Ready for integration testing and benchmark suite phases

---
*Phase: 52-simple-cli-api*
*Completed: 2026-03-22*
