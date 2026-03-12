---
phase: "41"
plan: "01"
subsystem: lifecycle
tags: [vector-prune, lifecycle, config, cli, scheduler]
dependency_graph:
  requires: [memory-vector, memory-scheduler, memory-search]
  provides: [vector-lifecycle-config, prune-cli]
  affects: [daemon-startup, admin-commands]
tech_stack:
  added: []
  patterns: [lifecycle-config-settings, cli-admin-commands]
key_files:
  created:
    - crates/memory-scheduler/src/jobs/bm25_rebuild.rs
  modified:
    - crates/memory-types/src/config.rs
    - crates/memory-types/src/lib.rs
    - crates/memory-daemon/src/cli.rs
    - crates/memory-daemon/src/commands.rs
    - crates/memory-scheduler/src/jobs/mod.rs
    - crates/memory-scheduler/src/lib.rs
decisions:
  - Vector lifecycle enabled by default; BM25 disabled (opt-in per PRD)
  - LifecycleConfig added to Settings for config.toml integration
metrics:
  duration: ~25min
  completed: "2026-03-11"
---

# Phase 41 Plan 01: Vector Pruning Wiring + CLI Command Summary

Lifecycle config integration, vector prune CLI, and daemon startup wiring for automated index management.

## One-liner

LifecycleConfig with per-level retention settings, PruneVectors CLI command, and BM25 rebuild job wired into daemon startup.

## What Was Done

### Task 1: Wire VectorPruneJob into daemon startup
- VectorPruneJob was already registered in `register_prune_jobs()` - verified existing wiring works
- Added BM25RebuildJob registration alongside existing prune jobs in daemon startup

### Task 2: Add lifecycle config section
- Added `LifecycleConfig` struct with `VectorLifecycleSettings` and `Bm25LifecycleSettings`
- Vector: enabled=true, segment_retention=30d, grip=30d, day=365d, week=1825d, prune_schedule="0 3 * * *"
- BM25: enabled=false (opt-in), min_level_after_rollup="day", rebuild_schedule="0 4 * * 0"
- BM25 also includes per-level retention: segment=30d, grip=30d, day=180d, week=1825d
- Added lifecycle field to Settings with serde(default) for backward compatibility
- Re-exported new types from memory-types lib.rs

### Task 3: Add CLI command for manual pruning
- Added `admin prune-vectors --age-days N --vector-path PATH --dry-run` subcommand
- Loads embedder, opens HNSW index and metadata, prunes per-level
- Added `admin rebuild-bm25 --min-level day --search-path PATH` subcommand (bonus for plan 41-02)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing functionality] Added BM25 rebuild CLI in plan 41-01**
- **Found during:** Task 3
- **Issue:** Plan 41-02 Task 4 specifies rebuild-bm25 CLI, but it shares handle_admin function with prune-vectors
- **Fix:** Added both CLI commands together to avoid partial match arm compilation errors
- **Files modified:** crates/memory-daemon/src/cli.rs, crates/memory-daemon/src/commands.rs

## Decisions Made

1. **Vector lifecycle enabled by default**: Vector indexes grow unbounded without pruning, so it makes sense to enable by default
2. **BM25 lifecycle disabled by default**: Per PRD append-only philosophy, BM25 pruning is opt-in only
3. **Config in memory-types**: LifecycleConfig lives in memory-types/config.rs alongside other config structs, not in individual crates

## Self-Check: PASSED
