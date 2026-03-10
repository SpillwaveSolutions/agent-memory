---
phase: 36-ingest-pipeline-wiring
plan: 03
one_liner: "Cross-session HNSW dedup via CompositeVectorIndex wiring into NoveltyChecker (closes DEDUP-02)"
subsystem: ingest-pipeline
tags: [dedup, hnsw, cross-session, novelty, composite-index]
dependency_graph:
  requires:
    - phase: 36-01
      provides: "NoveltyChecker, CandleEmbedderAdapter, ingest dedup branching"
    - phase: 36-02
      provides: "Proto deduplicated field, daemon NoveltyChecker wiring"
    - phase: 35-02
      provides: "InFlightBuffer, InFlightBufferIndex, VectorIndexTrait"
  provides:
    - "HnswIndexAdapter implementing VectorIndexTrait for HnswIndex"
    - "CompositeVectorIndex merging InFlightBuffer + HNSW results"
    - "NoveltyChecker::with_composite_index constructor"
    - "Daemon startup wires composite index when HNSW available"
  affects: [memory-service, memory-daemon]
tech_stack:
  added: []
  patterns: [composite-adapter, fail-open-composite, graceful-degradation]
key_files:
  created: []
  modified:
    - crates/memory-service/src/novelty.rs
    - crates/memory-daemon/src/commands.rs
key_decisions:
  - "CompositeVectorIndex searches all backends and returns highest-scoring result"
  - "HnswIndexAdapter is_ready returns false when HNSW is empty (no false positives)"
  - "Daemon falls back to buffer-only when HNSW directory does not exist"
  - "vector directory is 'vector' (matching existing prune job convention)"
metrics:
  duration: 4min
  completed: 2026-03-06
---

# Phase 36 Plan 03: Cross-Session HNSW Dedup Wiring Summary

**Cross-session HNSW dedup via CompositeVectorIndex wiring into NoveltyChecker (closes DEDUP-02)**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-06T07:53:53Z
- **Completed:** 2026-03-06T07:57:52Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `HnswIndexAdapter` that wraps `Arc<RwLock<HnswIndex>>` and implements `VectorIndexTrait`, converting HNSW `SearchResult` scores to `(String, f32)` tuples
- Added `CompositeVectorIndex` that searches multiple `VectorIndexTrait` backends, merges results by score descending, and truncates to top_k
- Added `NoveltyChecker::with_composite_index` constructor that wires InFlightBuffer (fast, within-session) and HNSW (persistent, cross-session) into a single composite
- Modified daemon startup to open HNSW from `{db_path}/vector` directory and use composite index when available, with buffer-only fallback
- 9 new unit tests covering composite behavior, HNSW adapter readiness, and search results

## Task Commits

Each task was committed atomically:

1. **Task 1: HnswIndexAdapter, CompositeVectorIndex, with_composite_index** - `3d1fe93` (feat)
2. **Task 2: Wire composite index in daemon startup** - `eca63b2` (feat)

## Files Modified

- `crates/memory-service/src/novelty.rs` - Added HnswIndexAdapter, CompositeVectorIndex, with_composite_index constructor, 9 new tests
- `crates/memory-daemon/src/commands.rs` - Modified NoveltyChecker creation to use composite index when HNSW available

## Decisions Made

- CompositeVectorIndex searches all backends and returns highest-scoring result (not first-match)
- HnswIndexAdapter::is_ready returns false when HNSW is empty (prevents false positives on fresh index)
- Daemon falls back to buffer-only when vector directory does not exist (graceful degradation)
- Vector directory path is `{db_path}/vector`, matching the existing prune job convention

## Deviations from Plan

None -- plan executed exactly as written.

## Gap Closure

This plan closes **DEDUP-02** (cross-session HNSW dedup):

| Before | After |
|--------|-------|
| NoveltyChecker used InFlightBuffer only (256-entry ring buffer) | NoveltyChecker uses CompositeVectorIndex (InFlightBuffer + HNSW) |
| Duplicates from >256 events ago undetected | All history checked via persistent HNSW index |
| Duplicates from previous sessions undetected | Cross-session duplicates detected via HNSW |
| DEDUP-02 status: Pending | DEDUP-02 status: Satisfied |

## Verification

- `cargo test --workspace --all-features` -- all tests pass (98 memory-service, full workspace green)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- clean
- `cargo fmt --all -- --check` -- formatted
- `with_composite_index` exists in novelty.rs (line 330)
- `HnswIndexAdapter` implements VectorIndexTrait (line 210)
- Daemon startup uses `with_composite_index` when HNSW available (commands.rs line 430)

## Issues Encountered

None

## User Setup Required

None -- no external service configuration required.

---
*Phase: 36-ingest-pipeline-wiring*
*Plan: 03 (gap closure)*
*Completed: 2026-03-06*
