---
phase: 16-memory-ranking-enhancements
plans: 5
subsystem: ranking-policy
tags: [salience, usage-tracking, novelty, lifecycle, vector-prune, bm25-prune]

# Dependency graph
requires:
  - phase: 14-topic-graph-memory
    provides: Time-decay pattern, embedding infrastructure
provides:
  - Salience scoring at write time (MemoryKind, SalienceScorer)
  - Usage tracking with cache-first reads (UsageTracker, CF_USAGE_COUNTERS)
  - Opt-in novelty filtering with fail-open behavior (NoveltyChecker)
  - Vector lifecycle automation per FR-08 (VectorLifecycleConfig, VectorPruneJob)
  - BM25 lifecycle automation per FR-09 (Bm25LifecycleConfig, Bm25PruneJob)
affects: [17-agent-retrieval-policy, memory-daemon]

# Tech tracking
tech-stack:
  added: [dashmap 6, lru 0.12, async-trait 0.1]
  patterns: [cache-first-reads, fail-open-behavior, opt-in-features]

key-files:
  created:
    - crates/memory-types/src/salience.rs
    - crates/memory-types/src/usage.rs
    - crates/memory-storage/src/usage.rs
    - crates/memory-service/src/novelty.rs
    - crates/memory-vector/src/lifecycle.rs
    - crates/memory-search/src/lifecycle.rs
    - crates/memory-scheduler/src/jobs/vector_prune.rs
    - crates/memory-scheduler/src/jobs/bm25_prune.rs
  modified:
    - crates/memory-types/src/lib.rs
    - crates/memory-types/src/toc.rs
    - crates/memory-types/src/grip.rs
    - crates/memory-types/src/config.rs
    - crates/memory-storage/src/column_families.rs
    - crates/memory-storage/src/lib.rs
    - crates/memory-service/src/lib.rs
    - crates/memory-vector/src/lib.rs
    - crates/memory-search/src/lib.rs
    - crates/memory-scheduler/src/lib.rs
    - crates/memory-scheduler/src/jobs/mod.rs
    - proto/memory.proto

key-decisions:
  - "Salience computed at write time (not read) to preserve append-only model"
  - "Usage counters stored in separate CF_USAGE_COUNTERS to not mutate TocNode/Grip"
  - "NoveltyChecker is DISABLED by default with fail-open behavior on any error"
  - "BM25 lifecycle DISABLED by default per PRD append-only philosophy"
  - "Vector lifecycle ENABLED by default with per-level retention (30d/365d/1825d)"
  - "Month/Year levels are PROTECTED and never pruned (stable anchors)"
  - "Backward compatible with v2.0.0 data via serde defaults"

patterns-established:
  - "Opt-in features: Use enabled: false as default, require explicit opt-in"
  - "Fail-open behavior: Store event on any check failure (timeout, error, missing deps)"
  - "Cache-first reads: Return cached data immediately, queue prefetch for misses"
  - "Protected levels: Month/Year never pruned, serve as stable anchors"
  - "Scheduler-triggered lifecycle: Jobs call admin RPC, don't own the pipeline"

# Metrics
duration: ~2 hours
completed: 2026-02-05
---

# Phase 16: Memory Ranking Enhancements Summary

**Salience scoring, usage tracking, novelty filtering, and index lifecycle automation**

## Overview

Phase 16 implements the Ranking Policy layer (Layer 6) of the cognitive architecture. It provides:

1. **Salience scoring** - Importance calculated at write time (MemoryKind classification, length density, pinned boost)
2. **Usage tracking** - Access pattern counters with cache-first reads and batched writes
3. **Novelty filtering** - Opt-in duplicate detection with fail-open behavior
4. **Vector lifecycle** - Automated pruning per FR-08 retention rules (30d/365d/1825d)
5. **BM25 lifecycle** - Optional pruning per FR-09 with post-prune optimization

## Files Created

### Core Types (memory-types)
- `crates/memory-types/src/salience.rs` - MemoryKind enum, SalienceScorer, SalienceConfig
- `crates/memory-types/src/usage.rs` - UsageStats, UsageConfig, usage_penalty function

### Storage Layer (memory-storage)
- `crates/memory-storage/src/usage.rs` - UsageTracker with LRU cache and batched writes

### Service Layer (memory-service)
- `crates/memory-service/src/novelty.rs` - NoveltyChecker with fail-open behavior

### Index Lifecycle (memory-vector, memory-search)
- `crates/memory-vector/src/lifecycle.rs` - VectorLifecycleConfig, PruneStats
- `crates/memory-search/src/lifecycle.rs` - Bm25LifecycleConfig, Bm25PruneStats

### Scheduler Jobs (memory-scheduler)
- `crates/memory-scheduler/src/jobs/vector_prune.rs` - VectorPruneJob
- `crates/memory-scheduler/src/jobs/bm25_prune.rs` - Bm25PruneJob

## Key Design Decisions

### 1. Salience at Write Time
Salience is computed ONCE when TocNode/Grip is created, not on read. This preserves the append-only model and avoids expensive recomputation.

Formula: `salience = 0.35 + length_density + kind_boost + pinned_boost`

### 2. Separate Usage Storage
Usage counters are stored in CF_USAGE_COUNTERS column family, separate from TocNode/Grip. This maintains immutability of the core records.

### 3. Fail-Open Novelty
NoveltyChecker is DISABLED by default. When enabled, any failure (timeout, error, missing embedder/index) results in storing the event. Never blocks ingestion.

### 4. Protected Levels
Month and Year vectors/documents are NEVER pruned. They serve as stable anchors for historical recall.

### 5. Backward Compatibility
All new fields use serde defaults:
- `salience_score: f32` defaults to 0.5
- `memory_kind: MemoryKind` defaults to Observation
- `is_pinned: bool` defaults to false

v2.0.0 data deserializes correctly without migration.

## Proto Additions

```protobuf
// MemoryKind enum for salience classification
enum MemoryKind {
    MEMORY_KIND_OBSERVATION = 1;
    MEMORY_KIND_PREFERENCE = 2;
    MEMORY_KIND_PROCEDURE = 3;
    MEMORY_KIND_CONSTRAINT = 4;
    MEMORY_KIND_DEFINITION = 5;
}

// Lifecycle RPCs
rpc PruneVectorIndex(PruneVectorIndexRequest) returns (PruneVectorIndexResponse);
rpc PruneBm25Index(PruneBm25IndexRequest) returns (PruneBm25IndexResponse);
rpc GetRankingStatus(GetRankingStatusRequest) returns (GetRankingStatusResponse);
```

## Configuration Defaults

| Feature | Default | Notes |
|---------|---------|-------|
| Salience scoring | Enabled | Computed at write time |
| Usage decay | Disabled | OFF until validated |
| Novelty check | Disabled | Explicit opt-in required |
| Vector lifecycle | Enabled | 30d segment, 365d day, 1825d week |
| BM25 lifecycle | Disabled | Per PRD append-only philosophy |

## Retention Rules

| Level | Vector (FR-08) | BM25 (FR-09) |
|-------|----------------|--------------|
| Segment | 30 days | 30 days |
| Grip | 30 days | 30 days |
| Day | 365 days | 180 days |
| Week | 1825 days (5yr) | 1825 days (5yr) |
| Month | NEVER | NEVER |
| Year | NEVER | NEVER |

## Test Coverage

- memory-types: 56 tests passing (salience, usage, config)
- memory-search: 11 lifecycle tests passing
- memory-service: 4 novelty tests passing

## Next Phase Readiness

Phase 16 provides the ranking signals that Phase 17 (Agent Retrieval Policy) will use for:
- Intent routing based on salience scores
- Tier detection using availability status
- Fallback chains with ranking-aware ordering

---
*Phase: 16-memory-ranking-enhancements*
*Completed: 2026-02-05*
