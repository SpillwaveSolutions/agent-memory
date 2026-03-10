---
phase: 38-e2e-validation
plan: 01
subsystem: testing
tags: [dedup, e2e, novelty-checker, mock-embedder, outbox, structural-bypass]

requires:
  - phase: 36-dedup-pipeline
    provides: "MemoryServiceImpl dedup gate with put_event_only for duplicates"
  - phase: 35-dedup-gate-foundation
    provides: "InFlightBuffer, DedupConfig, NoveltyChecker"
provides:
  - "E2E dedup tests proving TEST-01 (store-and-skip-outbox)"
  - "MockEmbedder and proto event helpers in TestHarness"
  - "SequentialEmbedder for multi-embedding test scenarios"
affects: [38-02, 38-03]

tech-stack:
  added: []
  patterns: ["MockEmbedder for deterministic dedup testing", "ULID-based event IDs for proto events", "SequentialEmbedder for orthogonal vector tests"]

key-files:
  created:
    - "crates/e2e-tests/tests/dedup_test.rs"
  modified:
    - "crates/e2e-tests/src/lib.rs"

key-decisions:
  - "Mock embedders (not real CandleEmbedder) for fast deterministic dedup tests"
  - "ULID event IDs required by storage layer key construction"
  - "SequentialEmbedder with VecDeque for multi-embedding scenarios"

patterns-established:
  - "MockEmbedder pattern: fixed embedding for deterministic cosine similarity"
  - "Proto event helpers: create_proto_event/create_proto_event_structural for service-level testing"
  - "SequentialEmbedder: pop-from-queue pattern for varying embeddings per call"

requirements-completed: [TEST-01]

duration: 3min
completed: 2026-03-10
---

# Phase 38 Plan 01: E2E Dedup Validation Summary

**E2E dedup tests with MockEmbedder proving store-and-skip-outbox, structural bypass, and response field correctness**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-10T03:15:32Z
- **Completed:** 2026-03-10T03:18:42Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- 4 E2E tests proving duplicate events stored in RocksDB but absent from outbox (TEST-01)
- Structural events (SessionStart) bypass dedup gate entirely (DEDUP-04)
- IngestEventResponse.deduplicated field verified for novel, duplicate, and structural events
- MockEmbedder and proto event helpers added to TestHarness for reuse

## Task Commits

Each task was committed atomically:

1. **Task 1: Add MockEmbedder and dedup helpers to TestHarness** - `31ce526` (feat)
2. **Task 2: Create dedup_test.rs with E2E dedup tests** - `7ac476f` (feat)

## Files Created/Modified
- `crates/e2e-tests/src/lib.rs` - Added MockEmbedder, uniform_normalized, create_proto_event, create_proto_event_structural
- `crates/e2e-tests/tests/dedup_test.rs` - 4 E2E dedup tests (396 lines) with SequentialEmbedder

## Decisions Made
- Used mock embedders instead of real CandleEmbedder for fast, deterministic tests
- Event IDs must be valid ULIDs (storage key construction requirement discovered during execution)
- SequentialEmbedder uses Mutex<VecDeque> to return different embeddings per call

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Event IDs must be valid ULIDs**
- **Found during:** Task 2 (dedup_test.rs creation)
- **Issue:** Plan used human-readable event IDs (e.g., "evt-dedup-001") but storage layer requires ULID-formatted event_ids for key construction
- **Fix:** Added `make_ulid()` helper using `ulid::Ulid::from_parts()` to generate valid ULID strings from timestamp + seed
- **Files modified:** crates/e2e-tests/tests/dedup_test.rs
- **Verification:** All 4 tests pass, clippy clean
- **Committed in:** 7ac476f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix for correct storage operation. No scope creep.

## Issues Encountered
None beyond the ULID requirement above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- MockEmbedder and proto event helpers ready for 38-02 (stale filter tests) and 38-03 (fail-open tests)
- TestHarness extended with dedup infrastructure for future test plans

## Self-Check: PASSED

- FOUND: crates/e2e-tests/src/lib.rs
- FOUND: crates/e2e-tests/tests/dedup_test.rs
- FOUND: commit 31ce526
- FOUND: commit 7ac476f

---
*Phase: 38-e2e-validation*
*Completed: 2026-03-10*
