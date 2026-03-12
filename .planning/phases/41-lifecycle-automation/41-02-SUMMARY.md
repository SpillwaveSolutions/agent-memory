---
phase: "41"
plan: "02"
subsystem: lifecycle
tags: [bm25-rebuild, lifecycle, e2e-test, search-indexer]
dependency_graph:
  requires: [memory-search, memory-scheduler, plan-41-01]
  provides: [bm25-rebuild-filter, bm25-rebuild-job, lifecycle-e2e]
  affects: [search-indexer, scheduler-jobs]
tech_stack:
  added: []
  patterns: [rebuild-with-filter, scheduler-job-pattern]
key_files:
  created:
    - crates/e2e-tests/tests/lifecycle_test.rs
    - crates/memory-scheduler/src/jobs/bm25_rebuild.rs
  modified:
    - crates/memory-search/src/indexer.rs
    - crates/memory-scheduler/src/jobs/mod.rs
    - crates/memory-scheduler/src/lib.rs
decisions:
  - rebuild_with_filter uses age_days=0 prune to remove all docs at levels below threshold
  - Bm25RebuildJob follows same pattern as existing VectorPruneJob and Bm25PruneJob
metrics:
  duration: ~25min
  completed: "2026-03-11"
---

# Phase 41 Plan 02: BM25 Lifecycle Policy + E2E Test Summary

BM25 rebuild with level filtering, scheduler job, CLI command, and E2E lifecycle tests.

## One-liner

SearchIndexer::rebuild_with_filter() removes fine-grain segment/grip docs, with Bm25RebuildJob scheduling and 5 E2E tests proving lifecycle operations work.

## What Was Done

### Task 1: Add BM25 lifecycle config
- Covered in Plan 41-01 Task 2 (Bm25LifecycleSettings with min_level_after_rollup, rebuild_schedule, per-level retention)

### Task 2: Add BM25 rebuild with level filter
- Added `SearchIndexer::rebuild_with_filter(min_level)` method
- Uses level ordering [segment, grip, day, week, month, year]
- Prunes all docs at levels below min_level threshold using existing prune(0, level, false)
- Commits atomically after all level removals

### Task 3: Add BM25 rebuild scheduler job
- Created `Bm25RebuildJob` in `crates/memory-scheduler/src/jobs/bm25_rebuild.rs`
- Follows same pattern as VectorPruneJob: with_rebuild_fn callback, cancellation token, cron scheduling
- Default: disabled, "0 4 * * 0" (weekly Sunday 4 AM), min_level="day"
- 7 unit tests (disabled default, cancel, callback, error, config, name, debug)

### Task 4: Add CLI command for manual BM25 rebuild
- Added `admin rebuild-bm25 --min-level day --search-path PATH` subcommand
- Validates min_level against known levels
- Calls rebuild_with_filter and reports results

### Task 5: E2E lifecycle test
- 5 tests in `crates/e2e-tests/tests/lifecycle_test.rs`:
  1. `test_bm25_prune_removes_old_segments` - old segment pruned, day node preserved
  2. `test_bm25_rebuild_with_level_filter` - segment+grip removed, day+week preserved
  3. `test_lifecycle_config_defaults` - vector enabled, BM25 disabled, correct retention values
  4. `test_prune_preserves_recent_docs` - recent docs untouched by prune
  5. `test_rebuild_with_segment_level_keeps_all` - min_level=segment removes nothing

## Deviations from Plan

None - plan executed exactly as written.

## Decisions Made

1. **rebuild_with_filter uses prune(0, level)**: Setting age_days=0 effectively prunes ALL docs at that level regardless of age, which is the intended behavior for level-based filtering
2. **Single commit for all plans**: Both 41-01 and 41-02 were committed together since they share files and the CLI commands require both plans' work to compile

## Self-Check: PASSED
