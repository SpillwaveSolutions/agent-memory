# Phase 38: E2E Validation - Research

**Researched:** 2026-03-09
**Domain:** End-to-end integration testing for dedup gate, stale filtering, and fail-open behavior
**Confidence:** HIGH

## Summary

Phase 38 adds three E2E test files to the existing `e2e-tests` crate, validating that the dedup gate (Phases 35-36), stale filter (Phase 37), and fail-open behavior work correctly through the complete ingest-to-query pipeline. The codebase already has a well-established E2E testing pattern with `TestHarness`, direct handler invocation (not gRPC), and `pretty_assertions` -- these tests follow the same patterns.

The key integration points are already fully implemented: `MemoryServiceImpl.ingest_event()` wires the `NoveltyChecker` dedup gate with `put_event_only()` for duplicates (skipping outbox), `RetrievalHandler.route_query()` applies `StaleFilter` post-merge, and both systems are configurable via `DedupConfig` and `StalenessConfig`. The E2E tests need to exercise these at the handler level, proving the three TEST-* requirements.

**Primary recommendation:** Use mock embeddings (not real CandleEmbedder) for dedup tests to keep tests fast and deterministic. Test at handler level (MemoryServiceImpl for ingest, RetrievalHandler for query) matching the established E2E pattern.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Test data strategy: Synthetic timestamps spanning weeks (0, 14, 28, 42 days ago) for deterministic stale filtering tests
- Existing `create_test_events` base timestamps already use fixed values -- extend this pattern
- Stale filtering: assert relative ordering only (newer.score > older.score), not exact percentages -- resilient to formula tuning
- Dedup: assert storage presence + index absence (3 assertions matching TEST-01)
- Supersession: verify `superseded_by` metadata flag appears in E2E query results -- proves full pipeline marks supersession
- Kind exemption: one E2E test proving Constraint/Definition/Procedure memories are NOT penalized by time-decay
- Fail-open: verify both storage preservation AND no error (all events in RocksDB + ingest succeeds)
- Embedder disabled via NoveltyChecker with embedder=None (natural fail path when CandleEmbedder fails to load)
- StaleFilter fail-open also tested: route_query returns results when metadata has no timestamp_ms
- Storage verification: assert all events stored in RocksDB even when dedup gate is non-functional
- One file per TEST-* requirement: `dedup_test.rs` (TEST-01), `stale_filter_test.rs` (TEST-02), `fail_open_test.rs` (TEST-03)
- Extend `TestHarness` with dedup/staleness helpers (e.g., `with_dedup_enabled()`, `with_staleness_config()`, `create_events_with_timestamps()`)
- Descriptive test function names: `test_dedup_duplicate_stored_but_not_indexed` style

### Claude's Discretion
- Whether to use real CandleEmbedder or mock embeddings for dedup tests
- Whether to test at handler level (RetrievalHandler) or full gRPC service level
- Whether to include structural event bypass test (DEDUP-04 coverage)
- Exact dedup assertion approach (storage + index presence/absence vs outbox skip assertion)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| e2e-tests crate | workspace | Test harness + helpers | All E2E tests live here, 7 existing test files |
| memory-service | workspace | MemoryServiceImpl, NoveltyChecker, RetrievalHandler | Contains ingest pipeline + query pipeline |
| memory-retrieval | workspace | StaleFilter | Staleness scoring module |
| memory-types | workspace | DedupConfig, StalenessConfig, Event | Config + domain types |
| memory-storage | workspace | Storage (RocksDB) | put_event, put_event_only, get_stats |
| memory-search | workspace | SearchIndex, TeleportSearcher | BM25 indexing for query-side assertions |
| pretty_assertions | 1.x | Test assertions | Already in dev-dependencies |
| tokio | workspace | Async runtime | All tests use `#[tokio::test]` |
| tempfile | workspace | Temp directories | TestHarness pattern |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| memory-vector | workspace | HnswIndex, VectorMetadata | Only if testing vector index absence for dedup |
| async-trait | workspace | EmbedderTrait mocking | Mock embedder for dedup tests |

**Installation:** No new dependencies needed. All crates already in e2e-tests Cargo.toml.

## Architecture Patterns

### Recommended Project Structure
```
crates/e2e-tests/
  src/lib.rs           # TestHarness + helpers (extend with dedup/staleness)
  tests/
    dedup_test.rs       # TEST-01: dedup E2E
    stale_filter_test.rs  # TEST-02: staleness E2E
    fail_open_test.rs     # TEST-03: fail-open E2E
    [7 existing test files]
```

### Pattern 1: Handler-Level Testing (Recommended)
**What:** Test at MemoryServiceImpl and RetrievalHandler level, not through gRPC
**When to use:** All E2E tests in this project
**Why:** Matches established pattern (all 7 existing files do this). Faster, simpler, no gRPC server setup.

```rust
// Ingest-side: MemoryServiceImpl with novelty_checker
let harness = TestHarness::new();
let mut service = MemoryServiceImpl::new(harness.storage.clone());
let checker = Arc::new(NoveltyChecker::with_in_flight_buffer(
    Some(embedder),
    buffer.clone(),
    DedupConfig { enabled: true, ..Default::default() },
));
service.set_novelty_checker(checker);

// Query-side: RetrievalHandler with staleness_config
let handler = RetrievalHandler::with_services(
    harness.storage.clone(),
    Some(bm25_searcher),
    None, // no vector handler needed
    None,
    StalenessConfig { enabled: true, ..Default::default() },
);
```

### Pattern 2: Mock Embeddings for Dedup (Recommended)
**What:** Use mock embedders that return fixed vectors, not real CandleEmbedder
**When to use:** All dedup tests (TEST-01)
**Why:** Real embedder requires ~80MB model download, makes tests slow and non-deterministic. Mock pattern already exists in novelty.rs unit tests.

```rust
// Mock embedder from existing pattern in novelty.rs
struct MockEmbedder {
    embedding: Vec<f32>,
}

#[async_trait::async_trait]
impl EmbedderTrait for MockEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
        Ok(self.embedding.clone())
    }
}
```

Note: The `EmbedderTrait` and `VectorIndexTrait` traits are already public in `memory_service::novelty`. The mock types from unit tests need to be recreated in the E2E test file (or elevated to TestHarness helpers).

### Pattern 3: Synthetic Timestamp Events
**What:** Create events with configurable timestamps spanning days/weeks
**When to use:** Staleness tests (TEST-02)
**Why:** Fixed timestamps make decay formula results deterministic and predictable.

```rust
// New helper in TestHarness or lib.rs
pub fn create_events_with_timestamps(
    session_id: &str,
    texts_and_offsets: &[(&str, i64)], // (text, days_ago)
) -> Vec<Event> {
    let now_ms = 1_706_540_400_000i64; // fixed reference
    texts_and_offsets.iter().enumerate().map(|(i, (text, days_ago))| {
        let ts_ms = now_ms - (days_ago * 86_400_000);
        // ... create Event with this timestamp
    }).collect()
}
```

### Pattern 4: Dedup Assertion (Store + Skip Outbox)
**What:** Verify duplicated events exist in RocksDB but were never indexed
**When to use:** TEST-01 dedup tests
**How:** The ingest pipeline calls `put_event_only()` for duplicates (no outbox entry) vs `put_event()` for novel events (with outbox). Assert:
1. Event exists in storage (get_event returns Some)
2. Outbox does NOT contain the event (outbox entry absent)
3. BM25 index does NOT find the event's content

```rust
// After ingesting duplicate via MemoryServiceImpl:
// 1. Storage has the event
let stored = harness.storage.get_event(&event_id).unwrap();
assert!(stored.is_some(), "Duplicate should still be in RocksDB");

// 2. Outbox should NOT have an entry for the duplicate
let outbox = harness.storage.get_outbox_entry(&event_id).unwrap();
assert!(outbox.is_none(), "Duplicate should NOT have outbox entry");

// 3. BM25 should NOT find it (since it was never indexed)
// (Index the TOC segment from novel events only, then search)
```

### Anti-Patterns to Avoid
- **Testing with real CandleEmbedder in dedup tests:** Makes tests slow (~80MB download), flaky, and non-deterministic. Use mock embedders.
- **Asserting exact score values for staleness:** The formula may be tuned. Assert relative ordering (newer > older) not exact percentages.
- **Testing through gRPC server:** All existing E2E tests use direct handler calls. gRPC setup adds complexity without testing value.
- **Creating new test harness patterns:** Extend the existing TestHarness, do not create a parallel infrastructure.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dedup gate behavior | Custom ingest simulation | `MemoryServiceImpl.ingest_event()` with `set_novelty_checker()` | Real pipeline path including DEDUP-03/04 logic |
| Stale filtering | Manual score calculation | `RetrievalHandler.route_query()` with `StalenessConfig` | Real post-merge filter application |
| Mock embeddings | Complex vector generation | Simple `MockEmbedder` returning fixed vectors | Existing pattern from novelty.rs unit tests |
| Temp storage | Manual dir management | `TestHarness::new()` | Handles tempdir + Storage + index paths |
| Outbox assertions | Custom RocksDB reads | `storage.get_outbox_entry()` | Already exists in Storage API |

## Common Pitfalls

### Pitfall 1: Outbox Entry Verification
**What goes wrong:** Testing dedup by looking at BM25 results without verifying outbox skip
**Why it happens:** The outbox is the real mechanism -- events skip indexing because they skip the outbox
**How to avoid:** Assert outbox absence directly via `storage.get_outbox_entry()`, not just BM25 absence
**Warning signs:** Test passes when dedup is broken because events were never indexed anyway

### Pitfall 2: Timestamp Reference Point for Staleness
**What goes wrong:** Using wall-clock time instead of newest-result-relative timestamps
**Why it happens:** Forgetting that StaleFilter uses the newest timestamp in the result set as reference
**How to avoid:** Create events with fixed timestamps and include a "newest" event at a known offset. The reference is internal to the result set.
**Warning signs:** Tests fail intermittently based on when they run

### Pitfall 3: InFlightBuffer State Across Tests
**What goes wrong:** Buffer retains state from previous test's events
**Why it happens:** Shared static state or buffer not reset
**How to avoid:** Each test creates its own fresh `InFlightBuffer` via `TestHarness` helpers
**Warning signs:** Tests pass individually but fail when run together

### Pitfall 4: BM25 Index Population for Query-Side Tests
**What goes wrong:** StaleFilter tests return empty results because BM25 has no data
**Why it happens:** StaleFilter operates on `route_query` results, which require indexed data
**How to avoid:** For TEST-02, the pipeline must be: create events -> ingest -> build TOC -> index into BM25 -> then query. Follow the pattern from `pipeline_test.rs`.
**Warning signs:** `route_query` returns has_results=false

### Pitfall 5: Metadata Required for Staleness
**What goes wrong:** StaleFilter has no effect because `timestamp_ms` is missing from metadata
**Why it happens:** `build_metadata()` in retrieval.rs populates `timestamp_ms` from BM25 search results, which come from `TeleportSearcher`. The searcher gets `timestamp_ms` from indexed TocNode data.
**How to avoid:** When indexing TOC nodes for staleness tests, ensure the TocNode has timestamps that propagate through to search result metadata. Alternatively, test at the `StaleFilter.apply()` level directly with hand-crafted `SearchResult` metadata.
**Warning signs:** All results have identical scores after filtering

### Pitfall 6: Structural Event Bypass Must Use MemoryServiceImpl
**What goes wrong:** Testing structural bypass at wrong level misses the DEDUP-04 check
**Why it happens:** The structural bypass check (`event.event_type.is_structural()`) happens in `MemoryServiceImpl.ingest_event()`, not in `NoveltyChecker`
**How to avoid:** Test DEDUP-04 through `MemoryServiceImpl.ingest_event()` with a structural event type (SessionStart)
**Warning signs:** Structural events are checked by dedup (they should bypass entirely)

## Code Examples

### Example 1: Dedup Test Setup (TEST-01)
```rust
use std::sync::{Arc, RwLock};
use memory_service::{MemoryServiceImpl, NoveltyChecker};
use memory_service::novelty::EmbedderTrait;
use memory_types::config::DedupConfig;
use memory_types::dedup::InFlightBuffer;

struct MockEmbedder {
    embedding: Vec<f32>,
}

#[async_trait::async_trait]
impl EmbedderTrait for MockEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
        Ok(self.embedding.clone())
    }
}

fn uniform_normalized(dim: usize) -> Vec<f32> {
    let val = 1.0 / (dim as f32).sqrt();
    vec![val; dim]
}

#[tokio::test]
async fn test_dedup_duplicate_stored_but_not_indexed() {
    let harness = TestHarness::new();
    let dim = 384;
    let embedding = uniform_normalized(dim);

    let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
    let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
        embedding: embedding.clone(),
    });
    let checker = Arc::new(NoveltyChecker::with_in_flight_buffer(
        Some(embedder),
        buffer.clone(),
        DedupConfig {
            enabled: true,
            threshold: 0.85,
            min_text_length: 10,
            ..Default::default()
        },
    ));

    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(checker.clone());

    // Ingest first event (novel -- buffer empty)
    // Ingest second event (duplicate -- same embedding)
    // Assert: both in RocksDB, only first has outbox entry
}
```

### Example 2: Staleness Test Setup (TEST-02)
```rust
use memory_service::RetrievalHandler;
use memory_types::config::StalenessConfig;

#[tokio::test]
async fn test_stale_results_downranked_relative_to_newer() {
    let harness = TestHarness::new();

    // Create events at 0, 14, 28, 42 days ago
    // Ingest, build TOC, index into BM25
    // ...

    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher),
        None,
        None,
        StalenessConfig {
            enabled: true,
            half_life_days: 14.0,
            max_penalty: 0.30,
            ..Default::default()
        },
    );

    let response = handler.route_query(Request::new(RouteQueryRequest {
        query: "matching query terms".to_string(),
        // ...
    })).await.unwrap();

    let results = response.into_inner().results;
    // Assert: newer results score >= older results score
}
```

### Example 3: Fail-Open Test Setup (TEST-03)
```rust
#[tokio::test]
async fn test_fail_open_embedder_disabled_events_still_stored() {
    let harness = TestHarness::new();

    // NoveltyChecker with embedder=None
    let checker = Arc::new(NoveltyChecker::new(
        None, // No embedder -- simulates load failure
        None,
        DedupConfig { enabled: true, ..Default::default() },
    ));

    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(checker);

    // Ingest events via service.ingest_event()
    // Assert: all events stored in RocksDB (storage.get_stats().event_count)
    // Assert: all events have outbox entries (fail-open means normal path)
    // Assert: ingest_event returns Ok (no error)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| NoveltyConfig | DedupConfig (NoveltyConfig is type alias) | Phase 35 | Use DedupConfig in new code |
| put_event (always with outbox) | put_event_only (no outbox for dedup) | Phase 36 | Dedup skips indexing via outbox skip |
| No staleness filtering | StaleFilter post-merge in route_query | Phase 37 | Query results now time-decayed |

## Open Questions

1. **Outbox entry verification API**
   - What we know: `put_event_only` skips outbox, `put_event` creates outbox entry
   - What's unclear: Whether Storage has a direct `get_outbox_entry(event_id)` API or if we need to scan
   - Recommendation: Check Storage API; if no direct getter, verify via outbox count or BM25 absence

2. **BM25 metadata timestamp propagation for staleness tests**
   - What we know: StaleFilter reads `timestamp_ms` from SearchResult metadata. BM25 TeleportResult has `timestamp_ms: Option<i64>`.
   - What's unclear: Whether MockSummarizer-generated TocNodes carry timestamps that propagate through BM25 indexing to search results
   - Recommendation: If metadata is missing, test StaleFilter directly with hand-crafted SearchResults (already proven in unit tests) and wrap with a thin E2E layer through route_query

## Sources

### Primary (HIGH confidence)
- `crates/e2e-tests/src/lib.rs` -- TestHarness, create_test_events, ingest_events helpers
- `crates/e2e-tests/tests/pipeline_test.rs` -- Full pipeline pattern (ingest -> TOC -> BM25 -> route_query)
- `crates/memory-service/src/ingest.rs` -- MemoryServiceImpl with dedup gate (lines 401-445)
- `crates/memory-service/src/novelty.rs` -- NoveltyChecker, EmbedderTrait, MockEmbedder pattern
- `crates/memory-retrieval/src/stale_filter.rs` -- StaleFilter with time-decay + supersession
- `crates/memory-service/src/retrieval.rs` -- RetrievalHandler.route_query() with staleness post-merge
- `crates/memory-types/src/config.rs` -- DedupConfig and StalenessConfig structs

### Secondary (MEDIUM confidence)
- `crates/e2e-tests/tests/degradation_test.rs` -- Degradation patterns relevant to fail-open testing
- `crates/e2e-tests/tests/error_path_test.rs` -- Error handling patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in workspace, no new deps needed
- Architecture: HIGH -- extending well-established TestHarness + handler-level testing pattern
- Pitfalls: HIGH -- identified from direct code inspection of integration points
- Code examples: HIGH -- derived from existing test patterns and production code

**Research date:** 2026-03-09
**Valid until:** Indefinite (testing internal APIs, not external dependencies)
