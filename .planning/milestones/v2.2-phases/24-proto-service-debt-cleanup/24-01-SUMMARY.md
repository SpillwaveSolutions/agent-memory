---
phase: 24-proto-service-debt-cleanup
plan: 01
subsystem: api
tags: [grpc, salience, novelty, agent-discovery, session-count]

# Dependency graph
requires:
  - phase: 23-agent-discovery
    provides: ListAgents RPC with TOC-based aggregation
  - phase: 16-salience-scoring
    provides: SalienceConfig and NoveltyConfig types
provides:
  - GetRankingStatus RPC returning real config data (salience, novelty, decay, lifecycle)
  - ListAgents RPC returning accurate session_count from event scanning
affects: [24-proto-service-debt-cleanup, 25-e2e-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Event scanning bounded to 365 days for session counting"
    - "Config defaults as source of truth for status RPCs"

key-files:
  created: []
  modified:
    - crates/memory-service/src/ingest.rs
    - crates/memory-service/src/agents.rs
    - crates/memory-service/src/vector.rs
    - crates/memory-service/src/hybrid.rs
    - crates/memory-service/src/teleport_service.rs

key-decisions:
  - "Use SalienceConfig::default() and NoveltyConfig::default() as truth for GetRankingStatus"
  - "Bound session event scan to 365 days for performance"
  - "BM25 lifecycle reported as false (no config storage)"

patterns-established:
  - "Status RPCs read from config defaults when no persistent state exists"
  - "O(n) event scans bounded by time window for safety"

# Metrics
duration: 23min
completed: 2026-02-11
---

# Phase 24 Plan 01: Wire RPC Stubs Summary

**GetRankingStatus returns real salience/novelty/decay config; ListAgents returns session_count from event scanning**

## Performance

- **Duration:** 23 min
- **Started:** 2026-02-11T01:47:48Z
- **Completed:** 2026-02-11T02:10:51Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- GetRankingStatus RPC now returns actual configuration data: salience_enabled=true, usage_decay_enabled=true, novelty_enabled=false from SalienceConfig/NoveltyConfig defaults
- ListAgents RPC computes accurate session_count by scanning events for distinct session_ids per agent (bounded to last 365 days)
- All existing tests continue to pass; 2 new tests added

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire GetRankingStatus to return real config data** - `fbbca17` (feat)
2. **Task 2: Fix ListAgents session_count via event scanning** - `fe62f5c` (feat)

## Files Created/Modified
- `crates/memory-service/src/ingest.rs` - Replaced stub GetRankingStatus with real config values; added test
- `crates/memory-service/src/agents.rs` - Added count_sessions_per_agent() for session counting; updated and added tests
- `crates/memory-service/src/vector.rs` - Added agent field to VectorMatch construction (blocking fix)
- `crates/memory-service/src/hybrid.rs` - Added agent field to RrfEntry and VectorMatch (blocking fix)
- `crates/memory-service/src/teleport_service.rs` - Added agent field to TeleportSearchResult (blocking fix)

## Decisions Made
- Used SalienceConfig::default() and NoveltyConfig::default() as truth source for GetRankingStatus (no persistent config store yet)
- Set usage_decay_enabled=true always (per Phase 16 design)
- vector_lifecycle_enabled reflects whether vector_service is Some (runtime check)
- bm25_lifecycle_enabled hardcoded false (no way to know config without storing it)
- Bounded session event scan to 365 days to keep O(n) manageable

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed missing agent field in VectorMatch, TeleportSearchResult, and RrfEntry**
- **Found during:** Task 1 (compilation failed)
- **Issue:** Uncommitted proto changes on branch added optional agent field to VectorMatch and TeleportSearchResult messages, but service code was not updated to include the new field
- **Fix:** Added agent field to VectorMatch construction in vector.rs, hybrid.rs; added to RrfEntry struct and From impl in hybrid.rs; added to TeleportSearchResult in teleport_service.rs; added to test in hybrid.rs
- **Files modified:** crates/memory-service/src/vector.rs, crates/memory-service/src/hybrid.rs, crates/memory-service/src/teleport_service.rs
- **Verification:** cargo check -p memory-service and cargo clippy -p memory-service pass
- **Committed in:** fbbca17 (part of Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Fix was necessary to compile the crate. The proto changes were from a prior uncommitted plan step on the branch. No scope creep.

## Issues Encountered
- C++ standard library headers not found during RocksDB compilation on macOS. Resolved by setting CPATH to include SDK headers: `CPATH="$(xcrun --show-sdk-path)/usr/include/c++/v1:$(xcrun --show-sdk-path)/usr/include"`. This is a local toolchain issue, not a code problem.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- GetRankingStatus and ListAgents are now wired to real data
- PruneVectorIndex and PruneBm25Index RPCs remain stubs (addressed in Plan 24-02 or 24-03)
- Agent field in TeleportSearchResult/VectorMatch is wired but returns None/None until search indexers populate it

## Self-Check: PASSED

- All 5 modified files exist on disk
- Commit fbbca17 (Task 1) verified in git log
- Commit fe62f5c (Task 2) verified in git log
- SUMMARY.md exists at expected path

---
*Phase: 24-proto-service-debt-cleanup*
*Completed: 2026-02-11*
