---
phase: 26-e2e-advanced-scenario-tests
plan: 02
subsystem: testing
tags: [e2e, degradation, fallback, retrieval-policy, tier-detection, graceful-degradation]

# Dependency graph
requires:
  - phase: 17-agent-retrieval-policy
    provides: "RetrievalHandler, CombinedStatus::detect_tier, FallbackChain, capability tiers"
  - phase: 25-e2e-core-pipeline-tests
    provides: "TestHarness, e2e-tests crate infrastructure, BM25 indexing patterns"
provides:
  - "E2E-06 graceful degradation tests covering missing-index scenarios"
  - "Verified Agentic-only tier works without panic when all indexes missing"
  - "Verified Keyword tier works correctly when only BM25 present"
  - "Verified capability warnings contain useful context about missing indexes"
affects: [26-e2e-advanced-scenario-tests, CI-pipeline]

# Tech tracking
tech-stack:
  added: []
  patterns: ["RetrievalHandler::with_services(storage, None, None, None) for degraded testing"]

key-files:
  created:
    - crates/e2e-tests/tests/degradation_test.rs
  modified: []

key-decisions:
  - "All four degradation scenarios tested without #[ignore] since they require no external resources"
  - "Agentic layer returns empty results (expected behavior per TOC navigation TODO); tests verify no-panic, not result content"

patterns-established:
  - "Degradation testing pattern: create RetrievalHandler with selective None params to simulate missing indexes"

# Metrics
duration: 22min
completed: 2026-02-11
---

# Phase 26 Plan 02: Graceful Degradation E2E Tests Summary

**4 E2E tests verifying retrieval pipeline degrades gracefully when indexes are missing: Agentic-only worst case, BM25 missing, vector missing with BM25 fallback to Keyword tier, and warning message quality validation**

## Performance

- **Duration:** 22 min
- **Started:** 2026-02-11T06:40:41Z
- **Completed:** 2026-02-11T07:03:03Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- test_degradation_all_indexes_missing: Proves worst-case Agentic-only mode works without panic when no search indexes are configured
- test_degradation_no_bm25_index: Proves BM25 missing scenario detects Agentic tier and route_query succeeds
- test_degradation_bm25_present_vector_missing: Proves Keyword tier works correctly with BM25 returning real results when vector/topics are absent
- test_degradation_capabilities_warnings_contain_context: Proves warning messages specifically mention BM25, Vector, and Topic when they are missing

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement graceful degradation E2E tests (E2E-06)** - `0e2e78d` (feat)

## Files Created/Modified
- `crates/e2e-tests/tests/degradation_test.rs` - 4 E2E tests for graceful degradation scenarios covering all-missing, BM25-missing, vector-missing, and warning quality

## Decisions Made
- All tests run without #[ignore] since they only need storage and optional BM25 index (no model downloads)
- Agentic layer currently returns empty results (per TOC navigation TODO); tests assert no-panic and correct tier detection rather than result content
- Warning content validation uses case-insensitive join-and-check pattern for readability

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- C++ toolchain issue: `cargo clean -p librocksdb-sys` invalidated build cache; resolved by sourcing `env.sh` which sets CXXFLAGS for the SDK's C++ headers

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- E2E-06 degradation tests complete and passing
- Ready for Plan 03 (remaining advanced scenario tests)

## Self-Check: PASSED

- FOUND: crates/e2e-tests/tests/degradation_test.rs
- FOUND: commit 0e2e78d

---
*Phase: 26-e2e-advanced-scenario-tests*
*Completed: 2026-02-11*
