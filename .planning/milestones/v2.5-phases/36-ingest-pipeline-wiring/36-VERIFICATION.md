---
phase: 36-ingest-pipeline-wiring
verified: 2026-03-06T08:20:00Z
status: passed
score: 4/4 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 3/4
  gaps_closed:
    - "Cross-session duplicates are detected by querying the HNSW vector index for events similar to the incoming event"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Ingest two semantically identical events in separate sessions with dedup enabled"
    expected: "The second event should be marked deduplicated=true in the IngestEventResponse"
    why_human: "Requires CandleEmbedder model files loaded, real embedding inference, and composite index similarity check to return a hit -- cannot verify from static grep"
  - test: "Ingest a session_start event with dedup enabled"
    expected: "deduplicated=false, event appears in BM25 and HNSW indexes (outbox entry written)"
    why_human: "Structural bypass is code-verified but runtime indexing path requires scheduler and background workers to consume the outbox"
---

# Phase 36: Ingest Pipeline Wiring Verification Report

**Phase Goal:** Duplicate events are stored but excluded from indexes, preserving the append-only invariant while keeping indexes clean
**Verified:** 2026-03-06T08:20:00Z
**Status:** passed
**Re-verification:** Yes -- after gap closure plan 36-03 (DEDUP-02 cross-session HNSW wiring)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Cross-session duplicates are detected by querying the HNSW vector index | VERIFIED | `HnswIndexAdapter` implements `VectorIndexTrait` (novelty.rs L198-235). `CompositeVectorIndex` searches InFlightBuffer + HNSW and returns highest-scoring result (L242-279). `NoveltyChecker::with_composite_index` wires both backends (L330-348). Daemon startup opens HNSW from `{db_path}/vector` and calls `with_composite_index` when available (commands.rs L409-442). Commits: `3d1fe93`, `eca63b2`. |
| 2 | Duplicate events are written to RocksDB but skip the outbox | VERIFIED | `put_event_only` in db.rs L131-156 writes only CF_EVENTS, no outbox entry. `ingest.rs` L379-395 calls `put_event_only` inside the `if deduplicated` branch (DEDUP-03 comment present). |
| 3 | Structural events bypass the dedup gate and are always indexed | VERIFIED | `EventType::is_structural()` in event.rs L63-71 matches all 4 types. `ingest.rs` L368-370 short-circuits to `(false, None)` for structural events before reaching the checker (DEDUP-04 comment present). |
| 4 | IngestEventResponse includes a `deduplicated` field | VERIFIED | `proto/memory.proto` L208: `bool deduplicated = 3`. `ingest.rs` L422: `deduplicated` set in response. All 98 memory-service tests pass. |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/memory-service/src/novelty.rs` | `HnswIndexAdapter` implementing `VectorIndexTrait` | VERIFIED | L198-235: wraps `Arc<RwLock<HnswIndex>>`, `is_ready()` returns false when empty, `search()` converts HNSW `SearchResult` to `(String, f32)` tuples with correct cosine polarity |
| `crates/memory-service/src/novelty.rs` | `CompositeVectorIndex` struct | VERIFIED | L242-279: searches all backends, skips not-ready, merges results, sorts by score descending, truncates to top_k; fail-open on individual backend errors |
| `crates/memory-service/src/novelty.rs` | `NoveltyChecker::with_composite_index` constructor | VERIFIED | L330-348: wires InFlightBufferIndex + HnswIndexAdapter into CompositeVectorIndex; stores buffer ref for `push_to_buffer` after novel events |
| `crates/memory-daemon/src/commands.rs` | NoveltyChecker wired to composite index at startup | VERIFIED | L409-442: opens HNSW from `{db_path}/vector`; uses `with_composite_index` when HNSW available; falls back to `with_in_flight_buffer` when not; logs `hnsw: true/false` indicator |
| `crates/memory-storage/src/db.rs` | `put_event_only` method | VERIFIED | L131-156: stores event in CF_EVENTS without outbox entry (no regression from initial verification) |
| `crates/memory-types/src/event.rs` | `EventType::is_structural` helper | VERIFIED | L63-71: `matches!()` for all 4 structural types (no regression) |
| `crates/memory-service/src/ingest.rs` | Dedup branching with novelty_checker field | VERIFIED | L368-423: structural bypass, checker invocation, `put_event_only` for duplicates, response field set (no regression) |
| `proto/memory.proto` | `deduplicated` field on `IngestEventResponse` | VERIFIED | L208: `bool deduplicated = 3` (no regression) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `commands.rs` | `HnswIndex` | `HnswIndexAdapter::new(Arc<RwLock<HnswIndex>>)` | WIRED | L409-416: opens HNSW, wraps in `Arc<RwLock<>>`, passes to `with_composite_index` |
| `novelty.rs` | `memory_vector::HnswIndex` | `VectorIndexTrait` impl delegates to `VectorIndex::search` | WIRED | L219-234: calls `index.search(&query, top_k)`, maps `SearchResult.score` |
| `commands.rs` | `CompositeVectorIndex` | `NoveltyChecker::with_composite_index` | WIRED | L430-435: explicit call with `buffer` and `hnsw_index` args |
| `ingest.rs` | `db.rs` | `put_event_only` call for deduplicated events | WIRED | L383: `self.storage.put_event_only(&event_id, &event_bytes)` (no regression) |
| `ingest.rs` | `novelty.rs` | `NoveltyChecker.should_store_with_embedding` call | WIRED | L372 (no regression) |
| `commands.rs` | `server.rs` | Passes NoveltyChecker to server function | WIRED | L469 (no regression) |
| `ingest.rs` | `proto/memory.proto` | `deduplicated` field in response | WIRED | L422 (no regression) |

### Requirements Coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| DEDUP-02: HNSW vector index for cross-session duplicates | SATISFIED | `HnswIndexAdapter` + `CompositeVectorIndex` wired in daemon startup. Commits `3d1fe93`, `eca63b2`. |
| DEDUP-03: Duplicate events stored in RocksDB but skip outbox/indexing | SATISFIED | `put_event_only` + dedup branching in `ingest_event` fully implements store-and-skip-outbox. |
| DEDUP-04: Structural events bypass dedup | SATISFIED | `is_structural()` check + short-circuit in `ingest_event` covers all 4 structural types. |

**Documentation gap (warning, not blocker):** REQUIREMENTS.md checkboxes for DEDUP-02, DEDUP-03, and DEDUP-04 still show `[ ]` (unchecked), and the traceability table still shows "Pending" for all three. The code satisfies these requirements. The document should be updated to `[x]` and "Done" respectively.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/memory-service/src/ingest.rs` | 1009 | `buffer_size: 0` in `GetDedupStatus` (hardcoded) | Warning | Operational visibility incomplete; carried from initial verification, not introduced by 36-03 |
| None | - | No new TODO/placeholder stubs introduced by 36-03 | - | Gap closure code is substantive and complete |

### Unit Test Coverage (New Tests from 36-03)

9 new tests added to `novelty.rs` (confirmed in test module starting at L849):

| Test | Verifies |
|------|----------|
| `test_composite_returns_highest_scoring_result` | CompositeVectorIndex returns best score from two backends |
| `test_composite_returns_all_results_when_top_k_large` | CompositeVectorIndex merges and sorts all results |
| `test_composite_gracefully_handles_one_failing_backend` | Fail-open: error from one backend does not block the other |
| `test_composite_is_ready_when_any_index_ready` | is_ready true when at least one backend ready |
| `test_composite_not_ready_when_no_index_ready` | is_ready false when all backends not ready |
| `test_composite_skips_not_ready_indexes` | Not-ready backends are skipped during search |
| `test_hnsw_adapter_not_ready_when_empty` | HnswIndexAdapter.is_ready false on empty index |
| `test_hnsw_adapter_ready_when_has_vectors` | HnswIndexAdapter.is_ready true after add() |
| `test_hnsw_adapter_search_returns_results` | HnswIndexAdapter.search returns (id, score) tuples with correct polarity |

Total memory-service tests: 98 passed, 0 failed.

### Human Verification Required

#### 1. End-to-end cross-session dedup detection

**Test:** Start daemon, ingest event A in session-1, then ingest event A (identical text) in session-2 with dedup enabled in config
**Expected:** Second ingest returns `deduplicated=true`; event appears in RocksDB but not in HNSW or BM25 indexes
**Why human:** Requires CandleEmbedder model files loaded, real embedding inference, composite index returning a hit above threshold -- cannot verify from static analysis

#### 2. Structural event bypass at runtime

**Test:** Ingest a `session_start` event with dedup enabled
**Expected:** `deduplicated=false`, event appears in BM25 and HNSW indexes (outbox entry written and processed)
**Why human:** Structural bypass is code-verified but runtime indexing path requires scheduler and background workers to consume the outbox

### Gap Closure Summary

The single gap from the initial verification (DEDUP-02: cross-session HNSW dedup not wired) is closed.

**Before 36-03:** `NoveltyChecker` used `InFlightBuffer` (256-entry in-memory ring buffer) as the sole similarity backend. Duplicates from more than 256 events ago or from previous process sessions were not detected.

**After 36-03:**
- `HnswIndexAdapter` wraps `Arc<RwLock<HnswIndex>>` and implements `VectorIndexTrait`, delegating search to the persistent HNSW index.
- `CompositeVectorIndex` searches both `InFlightBufferIndex` (fast, within-session) and `HnswIndexAdapter` (persistent, cross-session), returning the highest-scoring match from either.
- `NoveltyChecker::with_composite_index` constructor wires both backends with the InFlightBuffer still used for `push_to_buffer` after novel events.
- Daemon startup (`commands.rs` L409-442) opens the HNSW index from `{db_path}/vector` when the directory exists. When available, it calls `with_composite_index`. When not available, it falls back to `with_in_flight_buffer` (graceful degradation). Both modes log the active configuration.

**Regression check:** DEDUP-03 (`put_event_only` branching), DEDUP-04 (structural bypass), and Truth 4 (response `deduplicated` field) all verified unchanged. No regressions introduced by 36-03.

---

_Verified: 2026-03-06T08:20:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification: Yes — after gap closure plan 36-03_
