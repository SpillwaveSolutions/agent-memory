---
phase: 35-dedup-gate-foundation
plan: 02
subsystem: dedup
tags: [novelty-checker, in-flight-buffer, vector-index, fail-open, dedup-gate]

requires:
  - phase: 35-dedup-gate-foundation
    provides: InFlightBuffer ring buffer and DedupConfig
  - phase: 16-memory-ranking-enhancements
    provides: NoveltyChecker service with VectorIndexTrait
provides:
  - InFlightBufferIndex adapter implementing VectorIndexTrait
  - NoveltyChecker.with_in_flight_buffer constructor for pipeline integration
  - NoveltyChecker.push_to_buffer method for post-store embedding injection
  - 7 new unit tests proving dedup detection and fail-open behavior
affects: [36-dedup-pipeline-integration, memory-service-ingest]

tech-stack:
  added: []
  patterns: [adapter-pattern-for-trait-impl, arc-rwlock-shared-buffer, mock-trait-testing]

key-files:
  created: []
  modified:
    - crates/memory-service/src/novelty.rs

key-decisions:
  - "InFlightBufferIndex uses threshold 0.0 in find_similar to return any match; caller does threshold comparison"
  - "push_to_buffer is explicit (not auto-push in should_store) to avoid pushing embeddings for events that fail to store"
  - "std::sync::RwLock used (not tokio) since InFlightBuffer operations are fast in-memory"

patterns-established:
  - "Adapter pattern: wrapping domain types to implement service traits (InFlightBufferIndex -> VectorIndexTrait)"
  - "Explicit push-after-store: dedup buffer only updated after confirmed storage"

duration: 3min
completed: 2026-03-06
---

# Phase 35 Plan 02: InFlightBuffer Integration into NoveltyChecker Summary

**InFlightBufferIndex adapter implementing VectorIndexTrait with push-after-novel buffer injection and 7 mock-based tests proving duplicate detection and fail-open behavior**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-06T03:09:58Z
- **Completed:** 2026-03-06T03:13:06Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- InFlightBufferIndex adapter wraps Arc<RwLock<InFlightBuffer>> to implement VectorIndexTrait
- NoveltyChecker gains with_in_flight_buffer constructor and push_to_buffer method for Phase 36 pipeline integration
- 7 new unit tests cover duplicate detection, novel pass-through, fail-open on error/no-index/not-ready, push-then-dedup, and empty-buffer scenarios
- All 11 novelty tests pass (4 existing + 7 new), full workspace clippy clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Create InFlightBufferIndex adapter and enhance NoveltyChecker** - `8bb39b7` (feat)
2. **Task 2: Add comprehensive unit tests for dedup detection and fail-open** - `1738ffa` (test)

## Files Created/Modified
- `crates/memory-service/src/novelty.rs` - InFlightBufferIndex adapter, enhanced NoveltyChecker with buffer field, push_to_buffer, with_in_flight_buffer constructor, 7 new tests with MockEmbedder/FailingEmbedder/MockVectorIndex

## Decisions Made
- InFlightBufferIndex delegates to find_similar with threshold 0.0, letting NoveltyChecker's check_similarity handle threshold comparison
- push_to_buffer is an explicit call (not auto-push in should_store) so the ingest pipeline controls when embeddings enter the buffer
- Used std::sync::RwLock (not tokio::sync::RwLock) since buffer operations are sub-microsecond in-memory

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- NoveltyChecker is fully wired to InFlightBuffer via InFlightBufferIndex adapter
- Phase 36 can inject NoveltyChecker::with_in_flight_buffer into the ingest pipeline
- push_to_buffer API is ready for the pipeline to call after confirmed event storage
- All 11 unit tests validate the dedup foundation is correct

---
*Phase: 35-dedup-gate-foundation*
*Completed: 2026-03-06*
