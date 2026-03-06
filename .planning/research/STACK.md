# Technology Stack: v2.5 Semantic Dedup & Retrieval Quality

**Project:** Agent Memory v2.5
**Researched:** 2026-03-05
**Focus:** Ingest-time semantic dedup gate and stale result filtering

## Key Finding: No New Dependencies Required

The existing stack already provides everything needed for both features. This milestone is purely a **feature implementation** on top of existing infrastructure, not a stack expansion.

**Confidence:** HIGH -- based on direct codebase inspection of all relevant crates.

## Existing Stack (Relevant to v2.5)

### Already Present -- Use As-Is

| Technology | Version (Locked) | Crate | Role in v2.5 |
|------------|-----------------|-------|---------------|
| usearch | 2.23.0 | memory-vector | HNSW index for dedup similarity search at ingest |
| candle-core/nn/transformers | 0.8.4 | memory-embeddings | all-MiniLM-L6-v2 embedding generation for dedup |
| RocksDB | 0.22 | memory-storage | Dedup metadata storage, staleness markers |
| chrono | 0.4 | memory-types | Timestamp comparison for staleness decay |
| tokio | 1.43 | memory-service | Async timeout for dedup gate (fail-open) |
| serde/serde_json | 1.0 | memory-types | Config serialization for dedup/staleness settings |

### No Version Bumps Needed

All current versions support the required operations:
- **usearch 2.23.0**: `search()` returns distances, `add()` inserts vectors -- both needed for dedup gate. Already validated in `HnswIndex::search()` at `crates/memory-vector/src/hnsw.rs`.
- **candle 0.8.4**: `embed()` generates 384-dim vectors -- same embedder used for query-path vector teleport. Already wrapped in `CandleEmbedder` at `crates/memory-embeddings/`.
- **RocksDB 0.22**: Column families support metadata storage. `VectorMetadata` at `crates/memory-vector/src/metadata.rs` already maps vector IDs to doc IDs with timestamps (`VectorEntry.created_at`).

## Integration Points for v2.5

### Feature 1: Ingest-Time Semantic Dedup Gate

**What exists:** The `NoveltyChecker` at `crates/memory-service/src/novelty.rs` already implements the exact pattern needed -- a fail-open, opt-in, async vector similarity check at ingest time. It:
- Has `EmbedderTrait` and `VectorIndexTrait` abstractions
- Implements timeout with fail-open behavior
- Tracks metrics (skipped_disabled, skipped_no_embedder, skipped_no_index, skipped_index_not_ready, skipped_error, skipped_timeout, skipped_short_text, stored_novel, rejected_duplicate)
- Uses `NoveltyConfig` with threshold (default 0.82), timeout (50ms), min_text_length (50)
- Is disabled by default, requires explicit opt-in

**What needs to change:** The current `NoveltyChecker` uses its own `EmbedderTrait` and `VectorIndexTrait` that are **not wired to the actual usearch index**. The `check_similarity()` method delegates to abstract traits but the real `HnswIndex` and `CandleEmbedder` are not connected. The implementation needs:

1. **Wire `NoveltyChecker` to real `HnswIndex`** -- Implement `VectorIndexTrait` for `Arc<RwLock<HnswIndex>>` with `VectorMetadata` lookup to convert vector IDs back to doc IDs
2. **Wire `NoveltyChecker` to real `CandleEmbedder`** -- Implement `EmbedderTrait` for `Arc<CandleEmbedder>` (wrapping the sync `embed()` call in `tokio::task::spawn_blocking`)
3. **Integrate into ingest path** -- The `MemoryServiceImpl` at `crates/memory-service/src/ingest.rs` needs to call `NoveltyChecker::should_store()` before `storage.put_event()`
4. **Adjust threshold** -- Current default of 0.82 may need tuning; 0.92 is more appropriate for dedup (vs novelty detection which should be looser)

**Stack impact:** Zero new crates. The `NoveltyChecker` pattern is already built; it just needs plumbing.

### Feature 2: Stale Result Filtering/Downranking

**What exists:** The ranking layer already has these components:
- **Salience scoring** (`crates/memory-types/src/salience.rs`): Write-time importance scoring with `SalienceScorer`, formula: `base(0.35) + length_density + kind_boost + pinned_boost`
- **Usage decay** (`crates/memory-types/src/usage.rs`): `usage_penalty()` function using `1 / (1 + decay_factor * access_count)`, `apply_usage_penalty()` multiplies score by penalty
- **VectorMetadata** (`crates/memory-vector/src/metadata.rs`): `VectorEntry.created_at` timestamp (ms since epoch) already stored for every indexed vector
- **Retrieval policy** (`crates/memory-retrieval/src/`): Intent classification, tier detection, execution orchestration with `StopConditions` including `min_confidence`

**What needs to be added (pure Rust, no new deps):**

1. **Staleness config** -- Add `StalenessConfig` to `crates/memory-types/src/config.rs` alongside `NoveltyConfig`:
   - `enabled: bool` (default: false, matching existing opt-in pattern)
   - `decay_half_life_days: f32` (default: 30.0) -- score halves every N days
   - `supersession_threshold: f32` (default: 0.90) -- similarity above which newer content supersedes older
   - `max_age_penalty: f32` (default: 0.1) -- floor for time decay (never fully zero out old results)

2. **Time-decay scoring** -- Add `staleness_penalty()` to `crates/memory-types/src/usage.rs` (adjacent to existing `usage_penalty()`):
   - Formula: `max(max_age_penalty, 0.5^(age_days / half_life_days))` -- exponential decay with floor
   - Applied as multiplicative factor on retrieval scores, same pattern as `apply_usage_penalty()`
   - Uses `chrono::Utc::now()` vs `VectorEntry.created_at` -- both already available

3. **Supersession detection** -- When multiple results are semantically similar (cosine > supersession_threshold), keep only the most recent:
   - Compare pairwise similarity of top-K results (embeddings available via `VectorMetadata` + `HnswIndex`)
   - For each cluster of similar results, retain the newest by `created_at`
   - This reuses `HnswIndex::search()` and `VectorMetadata::get()` -- no new dependencies

4. **Ranking integration** -- Apply staleness penalty in the retrieval/query layer at `crates/memory-service/src/teleport_service.rs` or `crates/memory-service/src/query.rs`

**Stack impact:** Zero new crates. All computation uses existing `chrono` timestamps and `usearch` similarity scores.

## Recommended Stack

### Core Framework (NO CHANGES)

| Technology | Version | Purpose | Why No Change |
|------------|---------|---------|---------------|
| usearch | 2.23.0 | HNSW vector index | Already supports search() for dedup gate |
| candle-* | 0.8.4 | Local embeddings | Already generates 384-dim vectors |
| RocksDB | 0.22 | Storage + metadata | Already stores timestamps for staleness |
| tokio | 1.43 | Async runtime | Already used for timeout in NoveltyChecker |
| chrono | 0.4 | Time calculations | Already used for timestamps throughout |

### Supporting Libraries (NO CHANGES)

| Library | Version | Purpose | Why No Change |
|---------|---------|---------|---------------|
| serde/serde_json | 1.0 | Config/metadata serialization | Already serializes NoveltyConfig, VectorEntry |
| tracing | 0.1 | Logging for dedup decisions | Already used in NoveltyChecker |
| thiserror | 2.0 | Error types for new error variants | Already used in all crates |
| async-trait | (existing) | Async trait bounds for EmbedderTrait/VectorIndexTrait | Already used in memory-service |

### What NOT to Add

| Temptation | Why Avoid |
|------------|-----------|
| SimHash / MinHash crate | Overkill -- cosine similarity via usearch is sufficient for 384-dim vectors. SimHash trades accuracy for speed but HNSW is already O(log n). |
| Bloom filter crate | Adds complexity without benefit -- HNSW search is already O(log n) and provides similarity scores, not just membership |
| Separate dedup index | Unnecessary -- reuse existing HNSW index; dedup is just search-before-insert on the same index |
| External embedding service | Already have local Candle; adding API dependency violates zero-API-dependency design principle |
| Time-series DB for staleness | RocksDB already stores timestamps; exponential decay is a pure math function, not a query |
| Approximate dedup (LSH) | usearch cosine similarity is accurate enough for 384-dim; LSH adds false negatives which means lost dedup |
| ordered-float crate | Unnecessary for score comparison; f32 comparisons with `partial_cmp` are fine for ranking |
| New column family for dedup state | The existing `VectorMetadata` already stores everything needed (vector_id, doc_id, created_at, text_preview) |

## Architecture of Changes (Stack Perspective)

```
Ingest Path (BEFORE v2.5):
  gRPC IngestEvent -> NoveltyChecker (UNWIRED) -> Store in RocksDB -> Outbox -> Background indexing

Ingest Path (AFTER v2.5):
  gRPC IngestEvent -> NoveltyChecker (WIRED) -> Store in RocksDB -> Outbox -> Background indexing
                      |
                      +-> Embed text (CandleEmbedder -- already instantiated in service)
                      +-> Search HNSW (usearch -- already instantiated in service)
                      +-> If similarity > threshold: reject (fail-open on any error)

Query Path (BEFORE v2.5):
  Search -> Rank by relevance + salience + usage_decay

Query Path (AFTER v2.5):
  Search -> Rank by relevance + salience + usage_decay + [STALENESS] -> [SUPERSESSION] -> Return
                                                          |                |
                                                          |                +-> Pairwise cosine on top-K
                                                          |                +-> Keep newest per cluster
                                                          +-> Apply time-decay penalty (chrono math)
```

## Crate Dependency Changes

### memory-service (changes needed)
- **Already depends on:** memory-embeddings, memory-vector, memory-types, memory-storage, memory-search, memory-scheduler, tokio, async-trait
- **Needs:** Wire `NoveltyChecker` to real `HnswIndex` and `CandleEmbedder` implementations. Add supersession filter as post-processing step in teleport/hybrid results.
- **No new Cargo.toml entries.**

### memory-types (changes needed)
- **Already depends on:** serde, chrono
- **Needs:** Add `StalenessConfig` struct (same file as `NoveltyConfig`). Add `staleness_penalty()` and `apply_staleness_penalty()` functions (same file as `usage_penalty()`).
- **No new Cargo.toml entries.**

### memory-retrieval (may need changes)
- **Already depends on:** memory-types, chrono, async-trait
- **Needs:** If staleness filtering is done at the retrieval policy layer (vs service layer), add staleness config to execution context. `StopConditions` may need a `staleness_enabled` field.
- **No new Cargo.toml entries.**

### memory-vector (no changes)
- Already has: `HnswIndex` with `search()`, `VectorMetadata` with `VectorEntry.created_at`
- No modifications needed -- the vector layer is a read target for dedup, not modified.

### memory-indexing (no changes)
- Already has: `VectorIndexUpdater` that adds to HNSW index via outbox pipeline
- The dedup gate runs BEFORE event storage (and therefore before indexing), so no changes here.

### memory-embeddings (no changes)
- Already has: `CandleEmbedder` with `embed()` method, `EmbeddingModel` trait
- The dedup gate wraps this in `EmbedderTrait` adapter at the service layer.

## Configuration Design

```toml
# In ~/.config/agent-memory/config.toml

# Existing config -- already implemented, just needs wiring
[novelty]
enabled = false              # Opt-in dedup gate (existing field)
threshold = 0.92             # Bump from 0.82 for stricter dedup
timeout_ms = 100             # Bump from 50ms to allow embedding + search
min_text_length = 50         # Existing field, keep as-is

# New config section
[staleness]
enabled = false              # Opt-in, matching novelty pattern
decay_half_life_days = 30.0  # Score halves every 30 days
supersession_threshold = 0.90 # Cosine sim for "this supersedes that"
max_age_penalty = 0.1        # Floor -- never fully zero out old results
```

**Design decision:** Keep `NoveltyConfig` name and semantics -- the "novelty check" IS the "dedup gate." The name `novelty` accurately describes checking whether incoming content is novel relative to existing content. Adding a separate `DedupConfig` would duplicate the same structure.

**Threshold tuning note:** The default threshold should be raised from 0.82 to 0.92 because:
- 0.82 is appropriate for "is this content novel enough to be interesting?" (novelty detection)
- 0.92 is appropriate for "is this content essentially the same thing?" (dedup)
- The difference matters: at 0.82, paraphrased content gets rejected; at 0.92, only near-identical content does

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Dedup mechanism | Reuse NoveltyChecker + real HNSW | Separate dedup index (hash-based) | NoveltyChecker already implements the pattern; hash-based loses semantic similarity |
| Dedup mechanism | Reuse NoveltyChecker + real HNSW | Content hash (SHA-256) | Catches only exact duplicates; misses semantic duplicates like paraphrases |
| Staleness scoring | Exponential time decay | Linear decay | Exponential is standard for memory/forgetting curves; old results should not linearly vanish |
| Supersession | Pairwise cosine of top-K | Track explicit supersession links in storage | Explicit links require schema changes, complex bookkeeping, and backfill; pairwise cosine is stateless |
| Config pattern | Opt-in with fail-open | Always-on | Matches existing novelty/usage patterns; lets users enable when ready |
| Threshold default | 0.92 for dedup | 0.82 (existing) | 0.82 is too aggressive for dedup; rejects legitimately different content |

## Installation

```bash
# No new dependencies -- just build
cargo build --workspace

# No Cargo.toml changes needed
# All features implemented using existing crates
```

## Proto Changes

The gRPC proto at `proto/memory.proto` may need minor additions:
- `IngestEventResponse` could include a `deduplicated: bool` field indicating the event was rejected
- `GetRankingStatusResponse` could include staleness config status
- Field numbers >200 are reserved for new additions (per project convention)

No new RPCs needed. Dedup is transparent to callers (event just silently not stored). Staleness is transparent to callers (results just ranked differently).

## Sources

- Direct codebase inspection: `crates/memory-service/src/novelty.rs` -- NoveltyChecker with EmbedderTrait, VectorIndexTrait, fail-open, metrics, disabled-by-default
- Direct codebase inspection: `crates/memory-vector/src/hnsw.rs` -- HnswIndex wrapping usearch with cosine similarity, search() returns 1.0-distance
- Direct codebase inspection: `crates/memory-vector/src/metadata.rs` -- VectorEntry with created_at timestamp, VectorMetadata RocksDB store
- Direct codebase inspection: `crates/memory-types/src/config.rs` -- NoveltyConfig with threshold 0.82, timeout 50ms, disabled by default
- Direct codebase inspection: `crates/memory-types/src/usage.rs` -- usage_penalty() and apply_usage_penalty() patterns
- Direct codebase inspection: `crates/memory-types/src/salience.rs` -- SalienceScorer write-time scoring
- Direct codebase inspection: `crates/memory-indexing/src/vector_updater.rs` -- VectorIndexUpdater pipeline
- Direct codebase inspection: `crates/memory-service/src/ingest.rs` -- IngestEvent RPC handler
- Direct codebase inspection: `crates/memory-retrieval/src/types.rs` -- StopConditions, CapabilityTier, QueryIntent
- Cargo.lock: usearch 2.23.0, candle-core 0.8.4, tantivy 0.25.0, rocksdb 0.22
