---
phase: 38-e2e-validation
plan: 03
subsystem: testing
tags: [e2e, fail-open, dedup, staleness, graceful-degradation]

requires:
  - phase: 35-dedup-gate-foundation
    provides: "NoveltyChecker with fail-open behavior"
  - phase: 37-stale-filter
    provides: "StaleFilter with fail-open for missing timestamps"
provides:
  - "E2E proof that dedup gate fails open when embedder is None or erroring"
  - "E2E proof that StaleFilter passes results through without timestamps"
  - "TEST-03 requirement satisfied"
affects: []

tech-stack:
  added: []
  patterns: ["Proto event construction with ULID event_ids for E2E tests"]

key-files:
  created:
    - crates/e2e-tests/tests/fail_open_test.rs
  modified: []

key-decisions:
  - "Used ULID-based event_ids for proto events (storage requires valid ULIDs)"

patterns-established:
  - "FailingEmbedder pattern: local struct implementing EmbedderTrait for error-path E2E tests"
  - "make_proto_event helper: factory for proto events with valid ULIDs in E2E tests"

requirements-completed: [TEST-03]

duration: 2min
completed: 2026-03-09
---

# Phase 38 Plan 03: Fail-Open E2E Tests Summary

**Three E2E tests proving dedup gate and StaleFilter fail-open behavior: embedder=None, embedder errors, and missing timestamp metadata all result in normal operation**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-09T20:35:18Z
- **Completed:** 2026-03-09T20:37:25Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Proved embedder=None path: all 5 events ingest normally with outbox entries
- Proved embedder error path: FailingEmbedder returns errors but 3 events still pass through
- Proved StaleFilter fail-open: route_query returns results even without timestamp_ms metadata
- TEST-03 requirement fully satisfied

## Task Commits

Each task was committed atomically:

1. **Task 1: Create fail_open_test.rs with dedup and staleness fail-open E2E tests** - `a807815` (feat)

## Files Created/Modified
- `crates/e2e-tests/tests/fail_open_test.rs` - Three E2E fail-open tests proving TEST-03

## Decisions Made
- Used ULID-based event_ids in proto events (storage validates event_id as ULID format)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed event_id format for proto events**
- **Found during:** Task 1
- **Issue:** Plan suggested string-based event_ids like "fail-open-evt-0" but storage requires valid ULID format
- **Fix:** Changed make_proto_event to generate proper ULIDs using ulid::Ulid::from_parts()
- **Files modified:** crates/e2e-tests/tests/fail_open_test.rs
- **Verification:** All 3 tests pass after fix
- **Committed in:** a807815

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor fix for event_id format. No scope creep.

## Issues Encountered
None beyond the event_id format fix documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All three 38-xx plans complete (pipeline, dedup, fail-open E2E tests)
- Phase 38 E2E validation milestone ready for closure

---
*Phase: 38-e2e-validation*
*Completed: 2026-03-09*
