---
phase: 37-stale-filter
plan: 02
subsystem: retrieval
tags: [staleness, supersession, cosine-similarity, embedding-retrieval]
dependency_graph:
  requires: [StaleFilter, StalenessConfig, metadata-enrichment]
  provides: [supersession-detection, embedding-lookup, stale-filter-wiring]
  affects: [memory-retrieval, memory-service, memory-vector]
tech_stack:
  added: []
  patterns: [pairwise-cosine-similarity, fail-open-supersession, dot-product-as-cosine]
key_files:
  created: []
  modified:
    - crates/memory-vector/src/hnsw.rs
    - crates/memory-service/src/vector.rs
    - crates/memory-retrieval/src/stale_filter.rs
    - crates/memory-service/src/retrieval.rs
    - crates/memory-service/src/ingest.rs
    - crates/e2e-tests/tests/error_path_test.rs
    - crates/e2e-tests/tests/degradation_test.rs
    - crates/e2e-tests/tests/pipeline_test.rs
    - crates/e2e-tests/tests/multi_agent_test.rs
    - crates/e2e-tests/src/bin/perf_bench.rs
decisions:
  - Dot product used as cosine similarity (vectors pre-normalized by CandleEmbedder)
  - Supersession iterates newest-first, breaks on first match (no transitivity)
  - StalenessConfig propagated via with_services parameter (not global state)
metrics:
  duration: 8min
  completed: "2026-03-07T03:55:00Z"
---

# Phase 37 Plan 02: StaleFilter Wiring + Supersession Detection Summary

Pairwise cosine supersession detection via HNSW embedding retrieval, wired into RetrievalHandler.route_query post-merge with fail-open semantics.

## What Was Built

### HnswIndex.get_vector (memory-vector)
- New `get_vector(id: u64) -> Result<Option<Vec<f32>>>` method on HnswIndex
- Uses usearch `Index::get()` to retrieve stored embeddings by internal vector ID
- Returns None for missing IDs (contains check before retrieval)
- Unit test verifies round-trip: add vector, retrieve, values match

### VectorTeleportHandler.get_embeddings_for_doc_ids (memory-service)
- New `get_embeddings_for_doc_ids(&self, doc_ids: &[String]) -> HashMap<String, Vec<f32>>` method
- Looks up VectorMetadata by doc_id, then retrieves embedding from HnswIndex
- Missing entries silently skipped (fail-open)
- Single read lock acquisition for batch efficiency

### StaleFilter.apply_with_supersession (memory-retrieval)
- New `apply_with_supersession(results, embeddings)` method accepting optional embedding map
- `apply()` now delegates to `apply_with_supersession(results, None)` for backward compatibility
- Supersession algorithm: sort by timestamp descending, for each older non-exempt result check pairwise cosine similarity against newer results
- When similarity >= 0.80 threshold: insert `superseded_by` metadata key, apply 15% score penalty
- No transitivity: each result superseded at most once (break on first match)
- Exempt kinds (Constraint, Definition, Procedure, Preference) skip supersession check
- `dot_product()` helper computes cosine similarity for pre-normalized vectors

### RetrievalHandler.route_query Wiring (memory-service)
- Added `staleness_config: StalenessConfig` field to RetrievalHandler struct
- `with_services()` constructor accepts StalenessConfig parameter
- `new()` uses StalenessConfig::default()
- In route_query: StaleFilter applied post-merge, pre-proto-conversion
- When staleness enabled: looks up embeddings via VectorTeleportHandler for supersession
- When vector handler unavailable: embeddings are None, only time-decay applies (fail-open)
- All 6 call sites in ingest.rs + 7 call sites in e2e tests updated

## Test Coverage

- 1 new HnswIndex test: get_vector round-trip
- 7 new StaleFilter supersession tests: marks_older_similar, no_transitivity, exempt_kinds_skipped, without_embeddings, combined_penalty, metadata_explainability, dissimilar_not_superseded
- All 18 StaleFilter tests pass (11 existing + 7 new)
- Full workspace test suite passes (all crates)
- Clippy clean, no warnings

## Deviations from Plan

None - plan executed exactly as written.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | d27ec92 | Embedding retrieval + supersession detection in StaleFilter |
| 2 | f24dcc1 | Wire StaleFilter into RetrievalHandler.route_query |

## Self-Check: PASSED
