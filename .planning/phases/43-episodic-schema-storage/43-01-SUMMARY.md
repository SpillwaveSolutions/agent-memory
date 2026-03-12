---
phase: "43"
plan: "01"
subsystem: episodic-memory
tags: [episode, schema, storage, column-family, config]
dependency_graph:
  requires: []
  provides: [Episode, Action, ActionResult, EpisodeStatus, EpisodicConfig, CF_EPISODES, episode-storage-ops]
  affects: [memory-types, memory-storage]
tech_stack:
  added: []
  patterns: [serde-default-backward-compat, pub-crate-field-access, ulid-keyed-cf-iteration]
key_files:
  created:
    - crates/memory-types/src/episode.rs
    - crates/memory-storage/src/episodes.rs
  modified:
    - crates/memory-types/src/lib.rs
    - crates/memory-types/src/config.rs
    - crates/memory-storage/src/lib.rs
    - crates/memory-storage/src/column_families.rs
    - crates/memory-storage/src/db.rs
decisions:
  - "ActionResult uses tagged enum (status+detail) for JSON clarity"
  - "Storage.db made pub(crate) for cross-module CF access within memory-storage"
  - "Value scoring uses midpoint-distance formula: (1.0 - |outcome - midpoint|).max(0.0)"
  - "EpisodicConfig disabled by default (explicit opt-in like dedup)"
  - "list_episodes uses reverse ULID iteration for newest-first ordering"
metrics:
  duration: "8min"
  completed: "2026-03-11"
---

# Phase 43 Plan 01: Episode Schema, Storage, and Column Family Summary

Episode types, CF_EPISODES column family, CRUD storage ops, EpisodicConfig, and midpoint-distance value scoring for episodic memory foundation.

## What Was Built

### Episode Types (memory-types)
- `Episode` struct: episode_id, task, plan, actions, status, outcome/value scores, lessons, failure modes, embedding, timestamps, agent
- `Action` struct: action_type, input, result, timestamp
- `ActionResult` enum: Success(String), Failure(String), Pending (tagged JSON)
- `EpisodeStatus` enum: InProgress, Completed, Failed
- `Episode::calculate_value_score()` static method with midpoint-distance formula
- `Episode::complete()` and `Episode::fail()` convenience methods
- Full serde(default) on all optional fields for backward compatibility

### CF_EPISODES Column Family (memory-storage)
- New `CF_EPISODES` constant added to column_families.rs
- Registered in ALL_CF_NAMES array and build_cf_descriptors()
- Default RocksDB Options (no special compaction needed)

### Episode Storage Operations (memory-storage)
- `store_episode()` -- serialize to JSON, store in CF_EPISODES
- `get_episode()` -- lookup by episode_id
- `list_episodes(limit)` -- reverse ULID iteration for newest-first
- `update_episode()` -- overwrite by ID
- `delete_episode()` -- remove by ID
- Uses generic `put/get/delete` public API for store/get/delete
- Direct `db.iterator_cf` for reverse iteration (pub(crate) access)

### EpisodicConfig (memory-types)
- `enabled` (bool, default false) -- explicit opt-in
- `value_threshold` (f32, default 0.18) -- minimum value for retention
- `midpoint_target` (f32, default 0.65) -- sweet spot for learning value
- `max_episodes` (usize, default 1000) -- retention limit
- Wired into Settings with `[episodic]` TOML section
- Validation for all fields

## Test Results

- memory-types: 91 tests passing (85 existing + 6 new)
- memory-storage: 42 tests passing (35 existing + 7 new)
- New tests cover: serialization roundtrip, backward compat, CRUD operations, newest-first ordering, value scoring edge cases, config validation

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Made Storage.db field pub(crate)**
- **Found during:** Task 3
- **Issue:** episodes.rs needed direct RocksDB iterator access for reverse iteration, but Storage.db was private and inaccessible from sibling modules
- **Fix:** Changed `db: DB` to `pub(crate) db: DB` in Storage struct
- **Files modified:** crates/memory-storage/src/db.rs
- **Commit:** 71cbb83

**2. [Task consolidation] Tasks 1 and 5 merged**
- Value scoring function (Task 5) was implemented alongside Episode struct (Task 1) since `calculate_value_score` is a natural method on Episode. All required tests included.

## Commits

| Commit | Description |
|--------|-------------|
| 937a61d | feat(43-01): define Episode, Action, and ActionResult types |
| 0421c2e | feat(43-01): add CF_EPISODES column family for episodic memory |
| 71cbb83 | feat(43-01): add episode CRUD storage operations |
| bacb8a8 | feat(43-01): add EpisodicConfig with value scoring parameters |
| f7608d3 | chore(43-01): apply cargo fmt formatting fixes |
