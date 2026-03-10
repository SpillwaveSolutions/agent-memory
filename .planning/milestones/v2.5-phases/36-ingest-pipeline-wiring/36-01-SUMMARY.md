---
phase: 36-ingest-pipeline-wiring
plan: 01
one_liner: "Wire DedupGate into ingest path with store-and-skip-outbox for duplicates"
subsystem: ingest-pipeline
tags: [dedup, ingest, novelty, embeddings]
dependency_graph:
  requires: [35-01, 35-02]
  provides: [dedup-branching-in-ingest, put-event-only, candle-embedder-adapter]
  affects: [memory-service, memory-storage, memory-types]
tech_stack:
  added: []
  patterns: [store-and-skip-outbox, structural-bypass, post-store-buffer-push]
key_files:
  created: []
  modified:
    - crates/memory-storage/src/db.rs
    - crates/memory-types/src/event.rs
    - crates/memory-service/src/novelty.rs
    - crates/memory-service/src/ingest.rs
    - crates/memory-service/src/lib.rs
key_decisions:
  - "CandleEmbedderAdapter uses spawn_blocking for CPU-bound embed calls"
  - "DedupResult carries embedding alongside should_store for post-store buffer push"
  - "deduplicated field deferred to proto update in 36-02"
metrics:
  duration: 4min
  completed: 2026-03-06
---

# Phase 36 Plan 01: Ingest Pipeline Dedup Wiring Summary

Wire DedupGate into the MemoryServiceImpl ingest path with store-and-skip-outbox behavior for duplicate events, structural event bypass, CandleEmbedderAdapter, and NoveltyChecker injection.

## What Was Done

### Task 1: Foundation Types and Methods
- Added `Storage::put_event_only` -- stores events in RocksDB without outbox entries (DEDUP-03)
- Added `EventType::is_structural()` -- identifies SessionStart/End, SubagentStart/Stop (DEDUP-04)
- Added `CandleEmbedderAdapter` -- bridges `CandleEmbedder` to `EmbedderTrait` via `tokio::task::spawn_blocking`
- Added `DedupResult` struct with `should_store` and `embedding` fields
- Refactored `NoveltyChecker::should_store` to delegate to new `should_store_with_embedding`
- Modified `check_similarity` to return `(is_novel, Vec<f32>)` preserving embedding

### Task 2: Ingest Pipeline Integration
- Added `novelty_checker: Option<Arc<NoveltyChecker>>` field to `MemoryServiceImpl`
- Updated all 8 constructors with `novelty_checker: None` for backward compatibility
- Added `set_novelty_checker` method for post-construction injection
- Modified `ingest_event` with dedup branching:
  - Structural events bypass dedup entirely
  - Duplicate events stored via `put_event_only` (no outbox, no indexing)
  - Novel events stored via `put_event` (normal path with outbox)
  - Novel event embeddings pushed to InFlightBuffer after confirmed storage
- `deduplicated` field in response deferred until proto update (36-02)

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

- Workspace build: PASS
- Clippy (all 3 crates): PASS (zero warnings)
- Tests: 194 tests pass (89 memory-service, 35 memory-storage, 70 memory-types)
- All existing tests pass without modification

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 09d372b | put_event_only, is_structural, CandleEmbedderAdapter, DedupResult |
| 2 | 640daa8 | NoveltyChecker injection and ingest_event dedup branching |
