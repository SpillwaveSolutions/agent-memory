---
phase: 37-stale-filter
plan: 01
subsystem: retrieval
tags: [staleness, time-decay, scoring, query-time]
dependency_graph:
  requires: []
  provides: [StaleFilter, StalenessConfig, metadata-enrichment]
  affects: [memory-retrieval, memory-types, memory-service]
tech_stack:
  added: []
  patterns: [exponential-decay, kind-exemption, fail-open]
key_files:
  created:
    - crates/memory-retrieval/src/stale_filter.rs
  modified:
    - crates/memory-types/src/config.rs
    - crates/memory-types/src/lib.rs
    - crates/memory-retrieval/src/lib.rs
    - crates/memory-service/src/retrieval.rs
decisions:
  - All Observations get uniform decay regardless of salience score
  - memory_kind defaults to "observation" for all layers (BM25, Vector, Topics)
  - Topics layer has no timestamp (StaleFilter skips gracefully)
metrics:
  duration: 5min
  completed: "2026-03-07T03:44:00Z"
---

# Phase 37 Plan 01: StaleFilter + Metadata Enrichment Summary

Exponential time-decay filter with kind exemptions and configurable half-life, plus SearchResult metadata pipeline for timestamp_ms and memory_kind.

## What Was Built

### StalenessConfig (memory-types)
- New config struct with serde defaults: enabled=true, half_life_days=14.0, max_penalty=0.30, supersession_penalty=0.15, supersession_threshold=0.80
- Validation method checking all field ranges
- Added to Settings struct with `#[serde(default)]`
- Re-exported from crate root

### StaleFilter (memory-retrieval)
- `apply()` method: finds newest timestamp, applies time-decay, re-sorts by adjusted score
- Exponential decay formula: `score * (1.0 - max_penalty * (1.0 - exp(-age_days / half_life)))`
- Kind exemptions for Constraint, Definition, Procedure, Preference (case-insensitive)
- Fail-open: results without timestamps pass through unchanged
- TODO comment for supersession detection (Plan 37-02)

### Metadata Enrichment (memory-service)
- `build_metadata()` helper function for consistent metadata population
- BM25/Hybrid layers: propagate timestamp_ms (from TeleportResult) and agent
- Vector layer: propagate timestamp_ms (from VectorSearchResult) and agent
- Topics layer: memory_kind only (no timestamps available)
- All layers default memory_kind to "observation"

## Test Coverage

- 11 StaleFilter unit tests: empty/disabled passthrough, no-timestamp no-penalty, same-timestamp no-penalty, decay formula verification at 14/28/42 days, kind exemptions, observation decay, result reordering, mixed kinds, max penalty bound, case-insensitive exemption
- 6 StalenessConfig tests: defaults, validation pass/fail, serialization, Settings integration
- All 98 existing memory-service tests continue to pass

## Deviations from Plan

None - plan executed exactly as written.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 9905684 | StalenessConfig + StaleFilter with time-decay and kind exemptions |
| 2 | 50debdf | Enrich SimpleLayerExecutor metadata with timestamp_ms and memory_kind |
