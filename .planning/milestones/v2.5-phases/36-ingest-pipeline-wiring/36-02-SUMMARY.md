---
phase: 36-ingest-pipeline-wiring
plan: 02
one_liner: "Proto deduplicated field, GetDedupStatus RPC, and daemon NoveltyChecker wiring with fail-open CandleEmbedder"
subsystem: ingest-pipeline
tags: [dedup, ingest, grpc, proto, novelty, embeddings, observability]
dependency_graph:
  requires:
    - phase: 36-01
      provides: "NoveltyChecker, CandleEmbedderAdapter, set_novelty_checker, DedupResult"
    - phase: 35-01
      provides: "DedupConfig, NoveltyConfig alias"
    - phase: 35-02
      provides: "InFlightBuffer, InFlightBufferIndex"
  provides:
    - "deduplicated field on IngestEventResponse (proto field 3)"
    - "GetDedupStatus RPC for dedup observability"
    - "Daemon startup wires NoveltyChecker with CandleEmbedder"
    - "Fail-open CandleEmbedder initialization"
  affects: [memory-service, memory-daemon, proto]
tech_stack:
  added: []
  patterns: [fail-open-initialization, grpc-observability-rpc]
key_files:
  created: []
  modified:
    - proto/memory.proto
    - crates/memory-service/src/ingest.rs
    - crates/memory-service/src/server.rs
    - crates/memory-daemon/src/commands.rs
key_decisions:
  - "events_skipped in GetDedupStatus = total_stored minus stored_novel (all fail-open cases)"
  - "buffer_size hardcoded to 0 in GetDedupStatus (buffer len exposure deferred)"
metrics:
  duration: 6min
  completed: 2026-03-06
---

# Phase 36 Plan 02: Proto Changes and Daemon Wiring Summary

**Proto deduplicated field, GetDedupStatus RPC, and daemon NoveltyChecker wiring with fail-open CandleEmbedder**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-06T06:43:58Z
- **Completed:** 2026-03-06T06:50:17Z
- **Tasks:** 2
- **Files modified:** 4 (+ 3 formatting-only)

## Accomplishments

- Added `deduplicated` bool field (number 3) to IngestEventResponse proto, completing DEDUP-03 observability
- Added GetDedupStatus RPC with request/response messages exposing config and metrics
- Wired NoveltyChecker creation into daemon startup with CandleEmbedder fail-open initialization
- Updated run_server_with_scheduler to inject NoveltyChecker into MemoryServiceImpl

## Task Commits

Each task was committed atomically:

1. **Task 1: Proto changes and GetDedupStatus RPC handler** - `b3ea54d` (feat)
2. **Task 2: Wire NoveltyChecker into daemon startup** - `ad78e73` (feat)
3. **Formatting fixes** - `c1bc1ff` (chore)

## Files Created/Modified

- `proto/memory.proto` - Added deduplicated field, GetDedupStatus RPC and messages
- `crates/memory-service/src/ingest.rs` - Added GetDedupStatus handler, wired deduplicated in response
- `crates/memory-service/src/server.rs` - Added novelty_checker param to run_server_with_scheduler
- `crates/memory-daemon/src/commands.rs` - Create NoveltyChecker from Settings.dedup at startup

## Decisions Made

- events_skipped in GetDedupStatus calculated as total_stored - stored_novel (captures all fail-open skip categories)
- buffer_size hardcoded to 0 in response (InFlightBuffer len() not yet exposed; deferred)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Applied cargo fmt formatting fixes**
- **Found during:** Post-Task 2 verification
- **Issue:** cargo fmt --check failed on files from both current and previous phases
- **Fix:** Ran cargo fmt --all to normalize formatting
- **Files modified:** 7 files (commands.rs, ingest.rs, lib.rs, novelty.rs, db.rs, config.rs, dedup.rs)
- **Verification:** cargo fmt --all -- --check passes
- **Committed in:** c1bc1ff

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Formatting-only fix required for CI compliance. No scope creep.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Full dedup pipeline is wired: proto, service, daemon startup
- Clients can observe `deduplicated` field in IngestEventResponse
- GetDedupStatus RPC provides operational visibility
- Ready for phase 37 (retrieval quality) or additional dedup enhancements

---
*Phase: 36-ingest-pipeline-wiring*
*Completed: 2026-03-06*
