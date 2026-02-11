---
phase: 24-proto-service-debt-cleanup
plan: 03
subsystem: api
tags: [grpc, lifecycle, pruning, vector, bm25, tantivy, hnsw]

# Dependency graph
requires:
  - phase: 24-01
    provides: "GetRankingStatus wired with lifecycle config defaults"
provides:
  - "Working PruneVectorIndex RPC with metadata cleanup and dry_run support"
  - "Working PruneBm25Index RPC with lifecycle analysis and report-only mode"
  - "count_docs_before_cutoff method on TeleportSearcher for BM25 lifecycle analysis"
  - "metadata() accessor on VectorTeleportHandler"
affects: [25-e2e-test-suite, phase-26-observability]

# Tech tracking
tech-stack:
  added: []
  patterns: ["report-only prune for read-only indexes", "metadata-first pruning with deferred HNSW compaction"]

key-files:
  created: []
  modified:
    - "crates/memory-service/src/ingest.rs"
    - "crates/memory-service/src/vector.rs"
    - "crates/memory-search/src/searcher.rs"

key-decisions:
  - "Vector prune removes metadata entries only; orphaned HNSW vectors harmless until rebuild-index compaction"
  - "BM25 prune is report-only since TeleportSearcher is read-only; actual deletion requires SearchIndexer writer"
  - "Level matching for vectors uses doc_id prefix pattern (e.g., :day:, :week:, :segment:)"

patterns-established:
  - "Prune RPCs follow report-then-compact pattern: prune reports eligible items, rebuild-index compacts"
  - "count_docs_before_cutoff scans all Tantivy segments for level/timestamp-based document analysis"

# Metrics
duration: 11min
completed: 2026-02-11
---

# Phase 24 Plan 03: Prune RPCs Summary

**PruneVectorIndex and PruneBm25Index RPCs wired with lifecycle-based metadata cleanup and document analysis, supporting dry_run mode and level filtering**

## Performance

- **Duration:** 11 min
- **Started:** 2026-02-11T02:37:30Z
- **Completed:** 2026-02-11T02:48:42Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- PruneVectorIndex removes vector metadata entries older than retention cutoff, making HNSW vectors orphaned until rebuild
- PruneBm25Index scans indexed documents by level and timestamp, reporting what is eligible for pruning
- Both RPCs support dry_run mode, level filtering, age_days_override, and protected level enforcement
- Both RPCs handle "service not configured" gracefully with informative messages

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire PruneVectorIndex RPC with real lifecycle pruning** - `314fc8c` (feat)
2. **Task 2: Wire PruneBm25Index RPC with real lifecycle pruning** - `0959067` (feat)

## Files Created/Modified
- `crates/memory-service/src/ingest.rs` - PruneVectorIndex and PruneBm25Index RPC implementations with lifecycle config, dry_run, level filter
- `crates/memory-service/src/vector.rs` - Added `metadata()` accessor to VectorTeleportHandler
- `crates/memory-search/src/searcher.rs` - Added `count_docs_before_cutoff` method for BM25 lifecycle analysis

## Decisions Made
- Vector prune removes metadata entries only; orphaned HNSW vectors are harmless (metadata lookup fails, so they are not returned in search results) and can be compacted by rebuild-index later
- BM25 prune is report-only since the service only has the TeleportSearcher (reader); actual deletion requires the SearchIndexer (writer), which the rebuild-toc-index command manages
- Level matching for vector entries uses doc_id prefix pattern matching (`:segment:`, `:day:`, `:week:`) and doc_type for grips

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added count_docs_before_cutoff to TeleportSearcher**
- **Found during:** Task 2 (BM25 prune implementation)
- **Issue:** TeleportSearcher had no method to iterate/scan all documents by level and timestamp for lifecycle analysis
- **Fix:** Added `count_docs_before_cutoff` method that iterates Tantivy segment readers, reads stored fields, and counts docs matching level/cutoff criteria
- **Files modified:** crates/memory-search/src/searcher.rs
- **Verification:** Clippy clean, all tests pass
- **Committed in:** 0959067 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary addition to enable BM25 lifecycle analysis from the service layer. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 3 plans in Phase 24 complete
- DEBT-02 (PruneVectorIndex) and DEBT-03 (PruneBm25Index) stubs resolved
- Ready for Phase 25 (E2E test suite) or Phase 26 (observability)

## Self-Check: PASSED

All files and commits verified:
- FOUND: crates/memory-service/src/ingest.rs
- FOUND: crates/memory-service/src/vector.rs
- FOUND: crates/memory-search/src/searcher.rs
- FOUND: .planning/phases/24-proto-service-debt-cleanup/24-03-SUMMARY.md
- FOUND: commit 314fc8c
- FOUND: commit 0959067

---
*Phase: 24-proto-service-debt-cleanup*
*Completed: 2026-02-11*
