---
phase: 25-e2e-core-pipeline-tests
plan: 02
subsystem: testing
tags: [e2e, bm25, teleport, relevance-ranking, doc-type-filter, agent-attribution]

# Dependency graph
requires:
  - phase: 25-e2e-core-pipeline-tests
    plan: 01
    provides: "e2e-tests crate with shared TestHarness and helper functions"
  - phase: 24-proto-service-debt
    provides: "Agent attribution in TocNode.contributing_agents and BM25 index"
provides:
  - "BM25 teleport E2E test with relevance ranking verification"
  - "Doc type filtering E2E test (TocNode vs Grip isolation)"
  - "Agent attribution E2E test (contributing_agents through BM25)"
affects: [25-03, e2e-tests]

# Tech tracking
tech-stack:
  added: []
  patterns: [segment-membership doc_id tracking for mixed node+grip ranking assertions]

key-files:
  created:
    - crates/e2e-tests/tests/bm25_teleport_test.rs
  modified: []

key-decisions:
  - "Ranking assertions check segment membership (node or grip) rather than exact node_id, since grips may outrank their parent node"

patterns-established:
  - "Multi-segment BM25 test pattern: create N topic segments, index all nodes+grips, verify per-topic queries rank correct segment first"
  - "Track per-segment doc_id sets (node + grip IDs) for ranking assertions in mixed-type search results"

# Metrics
duration: 3min
completed: 2026-02-11
---

# Phase 25 Plan 02: BM25 Teleport E2E Tests Summary

**BM25 search E2E tests verifying relevance ranking across 3 topic segments, doc type filtering, and agent attribution propagation through Tantivy index**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-11T04:15:05Z
- **Completed:** 2026-02-11T04:17:57Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- test_bm25_ingest_index_search_ranked: proves 3 distinct topic segments are ranked correctly by BM25 relevance (Rust query returns Rust segment first, Python query returns Python segment first, gibberish returns 0 results)
- test_bm25_search_filters_by_doc_type: proves DocType::TocNode and DocType::Grip filters isolate correct document types in search results
- test_bm25_search_with_agent_attribution: proves contributing_agents propagates through BM25 indexing -- agent-attributed nodes return Some("claude"), non-attributed nodes return None

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement BM25 teleport E2E test with relevance ranking (E2E-02)** - `6b3d58d` (feat)

## Files Created/Modified
- `crates/e2e-tests/tests/bm25_teleport_test.rs` - Three BM25 E2E tests covering relevance ranking, doc type filtering, and agent attribution

## Decisions Made
- Ranking assertions check segment membership (node_id OR grip_id from that segment) rather than exact node_id. Grips contain the raw excerpt text which may score higher than the TocNode's combined title+bullets for specific queries. A grip from the correct segment ranking first still proves the pipeline works correctly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed ranking assertion to use segment membership instead of exact node_id**
- **Found during:** Task 1
- **Issue:** Plan specified checking results[0].doc_id == node_id, but grips from the same segment may rank higher than the parent TocNode for specific keyword queries
- **Fix:** Track per-segment doc_id sets (node + all grip IDs) and assert top result is in the correct segment set
- **Files modified:** crates/e2e-tests/tests/bm25_teleport_test.rs
- **Verification:** All 3 tests pass
- **Committed in:** 6b3d58d

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Assertion fix necessary for test correctness with BM25's actual ranking behavior. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All BM25 search E2E tests passing, ready for plan 25-03 (vector search E2E)
- TestHarness and helper functions proven across both pipeline and BM25 tests

## Self-Check: PASSED

All created files verified present. Commit hash 6b3d58d verified in git log.

---
*Phase: 25-e2e-core-pipeline-tests*
*Completed: 2026-02-11*
