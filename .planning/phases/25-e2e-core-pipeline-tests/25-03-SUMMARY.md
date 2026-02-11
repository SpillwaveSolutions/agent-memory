---
phase: 25-e2e-core-pipeline-tests
plan: 03
subsystem: testing
tags: [e2e, vector-search, semantic, topic-graph, hnsw, embeddings, candle]

# Dependency graph
requires:
  - phase: 25-01
    provides: "e2e-tests crate with TestHarness and helper functions"
  - phase: 24-02
    provides: "Agent attribution on VectorEntry and TeleportResult"
provides:
  - "Vector semantic search E2E test proving similarity-ordered results"
  - "Topic graph clustering E2E test proving importance-ordered retrieval"
  - "Agent attribution verification on vector and topic results"
affects: [e2e-tests]

# Tech tracking
tech-stack:
  added: []
  patterns: [OnceLock shared embedder for concurrent test safety, direct VectorTeleportHandler testing]

key-files:
  created:
    - crates/e2e-tests/tests/vector_search_test.rs
    - crates/e2e-tests/tests/topic_graph_test.rs
  modified: []

key-decisions:
  - "OnceLock<Arc<CandleEmbedder>> shared across tests to prevent concurrent model loading race condition"
  - "Vector tests marked #[ignore] due to ~80MB model download; topic tests run without ignore"
  - "Topic tests use direct TopicStorage::save_topic instead of full HDBSCAN clustering pipeline"

patterns-established:
  - "OnceLock pattern: shared expensive resources across tests in same binary"
  - "Three-group semantic test pattern: distinct topics verify search ranking"

# Metrics
duration: 12min
completed: 2026-02-11
---

# Phase 25 Plan 03: Vector Search and Topic Graph E2E Tests Summary

**Vector semantic search E2E with 3-group similarity ranking and topic graph E2E with importance-ordered retrieval via TopicGraphHandler**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-11T04:14:46Z
- **Completed:** 2026-02-11T04:27:29Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Vector semantic search test proves 3 distinct topic groups (Rust, cooking, ML) return closest match first with score ordering
- Topic graph test proves get_top_topics returns topics ordered by importance score with correct limiting
- Topic keyword search finds matching topics by label and keyword overlap
- Agent attribution verified on both vector results (opencode agent) and topic graph status
- OnceLock pattern prevents concurrent model loading race condition between parallel tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement vector semantic search E2E test (E2E-03)** - `839aebb` (feat)
2. **Task 2: Implement topic graph clustering E2E test (E2E-04)** - `443aff8` (feat)

## Files Created/Modified
- `crates/e2e-tests/tests/vector_search_test.rs` - Vector semantic search E2E: 3-group similarity test + agent attribution test
- `crates/e2e-tests/tests/topic_graph_test.rs` - Topic graph E2E: importance ordering, keyword search, status reporting

## Decisions Made
- Used `OnceLock<Arc<CandleEmbedder>>` to share the embedding model across tests -- concurrent `load_default()` calls caused a tokenizer parse error from reading partially-written model files
- Vector tests use `#[ignore]` attribute since they require ~80MB model download on first run (model cached after that); topic tests run without ignore since they need no external resources
- Topic tests create topics directly via `TopicStorage::save_topic()` rather than running the full HDBSCAN clustering pipeline, since clustering requires embeddings and is integration-test scope

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed concurrent model loading race condition**
- **Found during:** Task 1
- **Issue:** Both vector tests loading CandleEmbedder concurrently caused tokenizer parse error (EOF at line 1 column 0) from reading partially-downloaded model files
- **Fix:** Introduced `OnceLock<Arc<CandleEmbedder>>` static to share single embedder instance across all tests in the file
- **Files modified:** crates/e2e-tests/tests/vector_search_test.rs
- **Verification:** Both tests pass consistently when run together
- **Committed in:** 839aebb

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Fix was necessary for test reliability. No scope creep.

## Issues Encountered
- Vector search test takes ~200 seconds due to 15 sequential embedding operations (each ~13s) -- acceptable for local/ignored test
- Clippy initially flagged `vec![]` as `useless_vec` on string literal arrays -- changed to array syntax

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 5 E2E tests in phase 25 are complete (pipeline, BM25, vector, topic graph)
- Phase 25 is fully done; phase 26 can proceed
- Vector tests require `-- --ignored` flag to run (model download dependency)

## Self-Check: PASSED

All created files verified present. All commit hashes verified in git log.

---
*Phase: 25-e2e-core-pipeline-tests*
*Completed: 2026-02-11*
