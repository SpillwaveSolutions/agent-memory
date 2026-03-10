# Phase 38: E2E Validation - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

End-to-end tests proving dedup gate (Phases 35-36) and stale filtering (Phase 37) work correctly through the complete ingest-to-query pipeline. Three test requirements: TEST-01 (dedup), TEST-02 (staleness), TEST-03 (fail-open).

</domain>

<decisions>
## Implementation Decisions

### Test data strategy
- Synthetic timestamps spanning weeks (0, 14, 28, 42 days ago) for deterministic stale filtering tests
- Existing `create_test_events` base timestamps already use fixed values — extend this pattern

### Assertion depth
- Stale filtering: assert relative ordering only (newer.score > older.score), not exact percentages — resilient to formula tuning
- Dedup: assert storage presence + index absence (3 assertions matching TEST-01)
- Supersession: verify `superseded_by` metadata flag appears in E2E query results — proves full pipeline marks supersession
- Kind exemption: one E2E test proving Constraint/Definition/Procedure memories are NOT penalized by time-decay
- Fail-open: verify both storage preservation AND no error (all events in RocksDB + ingest succeeds)

### Fail-open scenarios
- Embedder disabled via NoveltyChecker with embedder=None (natural fail path when CandleEmbedder fails to load)
- StaleFilter fail-open also tested: route_query returns results when metadata has no timestamp_ms
- Storage verification: assert all events stored in RocksDB even when dedup gate is non-functional

### Test organization
- One file per TEST-* requirement: `dedup_test.rs` (TEST-01), `stale_filter_test.rs` (TEST-02), `fail_open_test.rs` (TEST-03)
- Extend `TestHarness` with dedup/staleness helpers (e.g., `with_dedup_enabled()`, `with_staleness_config()`, `create_events_with_timestamps()`)
- Descriptive test function names: `test_dedup_duplicate_stored_but_not_indexed` style

### Claude's Discretion
- Whether to use real CandleEmbedder or mock embeddings for dedup tests
- Whether to test at handler level (RetrievalHandler) or full gRPC service level
- Whether to include structural event bypass test (DEDUP-04 coverage)
- Exact dedup assertion approach (storage + index presence/absence vs outbox skip assertion)

</decisions>

<specifics>
## Specific Ideas

- Stale filtering tests should create events at known half-life intervals (0, 14, 28, 42 days) to test at the exact curve points documented in CONTEXT.md Phase 37
- Dedup tests should prove the "store-and-skip-outbox" pattern: event exists in RocksDB but was never indexed
- Supersession E2E test: ingest two near-identical events with different timestamps, query, verify older has superseded_by in metadata

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TestHarness` in `e2e-tests/src/lib.rs`: temp storage, BM25/vector index paths — extend with dedup/staleness methods
- `create_test_events()`: creates sequential events with configurable text and fixed base timestamps
- `ingest_events()`: serializes and stores events with outbox entries
- `build_toc_segment()`: triggers MockSummarizer + grip extraction for full pipeline

### Established Patterns
- One test file per concern: `pipeline_test.rs`, `degradation_test.rs`, `error_path_test.rs`, `vector_search_test.rs`
- Tests use `#[tokio::test]` with `pretty_assertions::assert_eq`
- TestHarness creates temp dirs, opens Storage, sets up index paths
- Tests call handlers directly (RetrievalHandler, TeleportSearcher) — not through gRPC

### Integration Points
- `NoveltyChecker` in `memory-service/src/novelty.rs`: check method for dedup gate
- `RetrievalHandler.route_query()` in `memory-service/src/retrieval.rs`: where StaleFilter is applied post-merge
- `StaleFilter` in `memory-retrieval/src/stale_filter.rs`: apply_with_supersession for query-time scoring
- `DedupConfig` / `StalenessConfig` in `memory-types/src/config.rs`: test-specific configs

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 38-e2e-validation*
*Context gathered: 2026-03-09*
