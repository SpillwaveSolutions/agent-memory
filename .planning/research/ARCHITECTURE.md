# Architecture Patterns

**Domain:** Semantic deduplication and stale result filtering for Agent Memory v2.5
**Researched:** 2026-03-05
**Confidence:** HIGH (based on direct codebase analysis of all relevant source files)

## Current Architecture (Baseline)

### Write Path (Ingest)

```
Hook Handler
    |
    v
gRPC IngestEvent RPC (memory-service/ingest.rs)
    |
    +--> Validate event_id, session_id
    +--> Convert proto Event -> domain Event
    +--> Serialize event bytes
    +--> Create OutboxEntry::for_toc(event_id, timestamp_ms)
    +--> storage.put_event(event_id, event_bytes, outbox_bytes)  [ATOMIC]
    +--> Return IngestEventResponse { event_id, created }
```

**Key observation:** The ingest handler is synchronous relative to the caller. It writes the event and outbox entry atomically to RocksDB, then returns. There is NO dedup check in the current write path.

### Async Index Path (Outbox Consumer)

```
Scheduler (memory-scheduler) triggers indexing job periodically
    |
    v
IndexingPipeline.process_batch(batch_size)
    |
    +--> storage.get_outbox_entries(start_sequence, limit)
    +--> For each registered IndexUpdater:
    |       +--> Filter entries this updater hasn't seen (checkpoint tracking)
    |       +--> updater.index_document(entry)  -- BM25 or Vector
    |       +--> Track success/error/skip per entry
    +--> Commit all indexes
    +--> Update checkpoints
    +--> Save checkpoints to RocksDB
```

**Registered updaters:** BM25Updater (Tantivy), VectorIndexUpdater (usearch HNSW)

### Vector Indexing Details

```
VectorIndexUpdater.process_entry(outbox_entry)
    |
    +--> If action == IndexEvent:
    |       +--> find_grip_for_event(event_id)  [currently returns None - simplified]
    |       +--> If grip found: index_grip(grip)
    |               +--> Check metadata for existing doc_id (skip if exists)
    |               +--> embedder.embed(text)   [CandleEmbedder, all-MiniLM-L6-v2]
    |               +--> metadata.next_vector_id()
    |               +--> hnsw_index.add(vector_id, embedding)
    |               +--> metadata.put(VectorEntry)
    |
    +--> If action == UpdateToc:
    |       +--> Skip (vector updater only handles IndexEvent)
```

**Key observation:** The vector index is populated from TOC nodes and grips AFTER they are created by the segmenter/summarizer. The current `find_grip_for_event` is a simplified stub returning None. In practice, TOC nodes are indexed when the `index_node` method is called directly during rebuild operations.

### Read Path (Retrieval)

```
RouteQuery RPC
    |
    v
RetrievalHandler
    +--> Classify intent (Explore/Answer/Locate/TimeBoxed)
    +--> Detect capability tier (Full/Hybrid/Semantic/Keyword/Agentic)
    +--> Build FallbackChain for intent+tier
    +--> RetrievalExecutor.execute(query, chain, conditions, mode, tier)
         |
         +--> Sequential: Try layers in order, stop at sufficient results
         +--> Parallel: Execute beam_width layers concurrently, pick best
         +--> Hybrid: Parallel first, sequential fallback
         |
         +--> Each layer returns SearchResult { doc_id, score, text_preview, ... }
         +--> Dedup by doc_id (in merge_results)
         +--> Return ExecutionResult with explainability
```

### Ranking Composition (Layer 6)

Current ranking components applied at different stages:

| Component | Stage | Location | Formula |
|-----------|-------|----------|---------|
| Salience | Write-time | `SalienceScorer` in memory-types | `0.35 + length_density + kind_boost + pinned_boost` |
| Usage decay | Read-time | `usage_penalty()` in memory-types | `score * 1/(1 + decay_factor * access_count)` |
| Novelty | Ingest-time | `NoveltyChecker` in memory-service | Cosine similarity gate (opt-in, fail-open) |

### Existing Novelty Checker (Important Precedent)

The system ALREADY has a `NoveltyChecker` in `memory-service/src/novelty.rs` that:
- Is **disabled by default** (opt-in via `NoveltyConfig.enabled`)
- Uses **fail-open** semantics (any failure -> store the event)
- Follows a **gate pattern**: check before store, but never block
- Has configurable **threshold** (default 0.82), **timeout** (default 50ms), **min_text_length** (default 50)
- Tracks detailed **metrics** (skipped_disabled, skipped_no_embedder, skipped_timeout, stored_novel, rejected_duplicate)
- Uses `EmbedderTrait` and `VectorIndexTrait` abstractions for testability

**This is the foundation for dedup.** The NoveltyChecker IS a semantic dedup gate. The question is: does it need enhancement, or is the timing gap the only real issue?

## Recommended Architecture for v2.5

### Design Principle: Dedup IS Enhanced Novelty

The existing `NoveltyChecker` already implements the core dedup pattern. Rather than building a parallel system, enhance it:

1. **Evolve** the NoveltyChecker to handle the timing gap (core architectural challenge)
2. **Add stale filtering** as a new read-time ranking component
3. **Keep the same fail-open, opt-in, metric-rich patterns**

### Component 1: DedupGate (Enhanced NoveltyChecker)

**Location:** `memory-service/src/novelty.rs` (extend existing)

**The Timing Problem:**
The vector index is built asynchronously from the outbox. When event N arrives, events N-1, N-2, etc. may not yet be in the HNSW index. Two near-simultaneous duplicate events will BOTH pass the dedup check because neither sees the other in the index.

**Solution: Two-tier dedup with in-flight buffer**

```
IngestEvent RPC
    |
    v
DedupGate (enhanced NoveltyChecker)
    |
    +--> GATE 1: Config enabled? (fail-open if disabled)
    +--> GATE 2: Text long enough? (skip short events)
    +--> GATE 3: Embedder available? (fail-open if not)
    |
    +--> Generate embedding for incoming event
    |
    +--> CHECK A: In-flight buffer (recent embeddings not yet indexed)
    |       +--> Linear scan of buffer (bounded size, e.g., 256 entries)
    |       +--> Cosine similarity against each buffered embedding
    |       +--> If max_similarity > threshold -> REJECT as duplicate
    |
    +--> CHECK B: HNSW index (historical indexed content)
    |       +--> hnsw_index.search(embedding, k=1)
    |       +--> If top_score > threshold -> REJECT as duplicate
    |
    +--> If novel:
    |       +--> Add embedding to in-flight buffer (with TTL/max-size eviction)
    |       +--> Return STORE
    |
    +--> If duplicate:
            +--> Increment rejected_duplicate metric
            +--> Return SKIP (event NOT stored)
```

**In-flight buffer design:**

```rust
struct InFlightBuffer {
    entries: VecDeque<InFlightEntry>,
    max_size: usize,     // Default: 256
    max_age: Duration,   // Default: 5 minutes
}

struct InFlightEntry {
    event_id: String,
    embedding: Vec<f32>,
    timestamp: Instant,
    session_id: String,
}
```

**Why this works:**
- The buffer catches duplicates that arrive faster than the indexing pipeline
- Buffer is small (256 entries x 384 dims x 4 bytes = ~400KB) -- trivial memory
- Linear scan of 256 vectors is <1ms -- well within the 50ms timeout
- Buffer entries auto-evict when old enough that they should be in the index
- Buffer is session-scoped (optional): only check within same session for tighter dedup

**Why NOT a separate index:**
- A second HNSW index adds complexity (two indexes to maintain/rebuild)
- The in-flight window is short (seconds to minutes), linear scan is fast enough
- Buffer entries naturally age out as the indexing pipeline catches up

### Component 2: StaleFilter (New Read-Time Ranking Component)

**Location:** New file `memory-service/src/stale.rs` or integrated into retrieval pipeline

**What is "stale"?** A result is stale when newer content semantically supersedes it. For example:
- "We decided to use PostgreSQL" superseded by "We switched to RocksDB"
- "JWT tokens expire in 1 hour" superseded by "JWT tokens now expire in 24 hours"

**Approach: Timestamp-based decay with semantic overlap detection**

```
RetrievalExecutor returns raw results
    |
    v
StaleFilter (post-retrieval, pre-return)
    |
    +--> For each result pair (i, j) where i.timestamp < j.timestamp:
    |       +--> If cosine_similarity(i.embedding, j.embedding) > overlap_threshold:
    |               +--> Mark i as "superseded by j"
    |               +--> Apply staleness penalty to i.score
    |
    +--> Apply time-based decay:
    |       +--> age_days = (now - result.timestamp).days()
    |       +--> decay = 1.0 / (1.0 + staleness_decay_factor * age_days)
    |       +--> result.score *= decay
    |
    +--> Re-sort results by adjusted score
    +--> Return filtered results
```

**Integration with existing ranking:**

```
Final score = base_score
            * salience_factor       (write-time, from SalienceScorer)
            * usage_penalty         (read-time, from usage tracking)
            * staleness_factor      (read-time, NEW)
```

**Where staleness_factor:**
```
staleness_factor = time_decay * supersession_penalty

time_decay = 1.0 / (1.0 + staleness_decay * age_days)
supersession_penalty = if superseded { 0.3 } else { 1.0 }
```

**Configuration:**

```rust
pub struct StaleConfig {
    /// Whether stale filtering is enabled (default: true for v2.5)
    pub enabled: bool,
    /// Cosine similarity threshold for considering two results as covering same topic
    /// Range: 0.0-1.0, higher = stricter (default: 0.85)
    pub overlap_threshold: f32,
    /// Decay factor for time-based staleness (default: 0.01)
    /// Higher = more aggressive time penalty
    pub decay_factor: f32,
    /// Score multiplier when result is superseded (default: 0.3)
    pub superseded_penalty: f32,
    /// Minimum age in days before time decay kicks in (default: 7)
    pub grace_period_days: u32,
}
```

### Component Boundaries

| Component | Responsibility | Communicates With | Crate |
|-----------|---------------|-------------------|-------|
| DedupGate (enhanced NoveltyChecker) | Reject semantically duplicate events at ingest | Embedder, HNSW index, InFlightBuffer | memory-service |
| InFlightBuffer | Track recent un-indexed embeddings for dedup gap | DedupGate only (internal) | memory-service |
| StaleFilter | Downrank superseded/old results at query time | RetrievalExecutor, Embedder | memory-service or memory-retrieval |
| DedupConfig | Configuration for dedup gate | Settings, NoveltyConfig (extend) | memory-types |
| StaleConfig | Configuration for staleness filtering | Settings | memory-types |
| DedupMetrics | Extended novelty metrics with buffer stats | DedupGate | memory-service |

### Data Flow Changes

**Write path change (before/after):**

```
BEFORE:
  IngestEvent -> validate -> serialize -> storage.put_event (atomic) -> return

AFTER:
  IngestEvent -> validate -> serialize
      -> DedupGate.should_store(event)
          -> embed(event.text)
          -> check InFlightBuffer (linear scan)
          -> check HNSW index (if not caught by buffer)
          -> if novel: add to buffer, return STORE
          -> if duplicate: return SKIP
      -> if STORE: storage.put_event (atomic) -> return {created: true}
      -> if SKIP: return {created: false, deduplicated: true}  [new response field]
```

**Read path change (before/after):**

```
BEFORE:
  RouteQuery -> classify -> tier detect -> execute layers -> merge -> return

AFTER:
  RouteQuery -> classify -> tier detect -> execute layers -> merge
      -> StaleFilter.apply(results, stale_config)
          -> pairwise overlap check (optional, O(n^2) but n is small ~10-20)
          -> time decay
          -> re-sort
      -> return
```

### Proto Changes Required

```protobuf
message IngestEventResponse {
    string event_id = 1;
    bool created = 2;
    bool deduplicated = 201;        // NEW: true if rejected as duplicate
    float similarity_score = 202;   // NEW: highest similarity score found
}

message DedupConfig {
    bool enabled = 1;
    float threshold = 2;
    uint64 timeout_ms = 3;
    uint32 min_text_length = 4;
    uint32 buffer_size = 5;         // In-flight buffer max entries
    uint64 buffer_ttl_secs = 6;     // In-flight buffer entry TTL
}

message StaleConfig {
    bool enabled = 1;
    float overlap_threshold = 2;
    float decay_factor = 3;
    float superseded_penalty = 4;
    uint32 grace_period_days = 5;
}

// New RPC for dedup status
message GetDedupStatusRequest {}
message GetDedupStatusResponse {
    bool enabled = 1;
    float threshold = 2;
    uint64 total_checked = 3;
    uint64 total_rejected = 4;
    uint64 buffer_size = 5;
    uint64 buffer_capacity = 6;
}
```

**Proto field numbers:** Use 201+ range (reserved for Phase 23+ per project convention).

## Patterns to Follow

### Pattern 1: Fail-Open Gate (from existing NoveltyChecker)

**What:** Any check that could prevent event storage MUST fail open.
**When:** Always, for any ingest-time gate.
**Why:** The system's core invariant is that hooks never block the agent. If the dedup check fails (embedder down, timeout, index corrupt), the event MUST be stored anyway.

```rust
pub async fn should_store(&self, event: &Event) -> DedupDecision {
    if !self.config.enabled {
        return DedupDecision::Store(DedupReason::Disabled);
    }
    // ... checks ...
    match timeout(duration, self.check_dedup(event)).await {
        Ok(Ok(decision)) => decision,
        Ok(Err(_)) => DedupDecision::Store(DedupReason::Error),    // fail-open
        Err(_) => DedupDecision::Store(DedupReason::Timeout),      // fail-open
    }
}
```

### Pattern 2: Opt-In with Sensible Defaults (from NoveltyConfig)

**What:** New features disabled by default, enabled via config.
**When:** Any feature that changes existing behavior.
**Why:** Backward compatibility. Existing users should see no change until they opt in.

```toml
# config.toml
[dedup]
enabled = true
threshold = 0.85
buffer_size = 256

[stale]
enabled = true
decay_factor = 0.01
```

### Pattern 3: Metric-Rich Observability (from NoveltyMetrics)

**What:** Every code path through the gate tracks a metric.
**When:** Any decision point in dedup or stale filtering.
**Why:** Debugging and tuning. Users need to know WHY events were rejected or WHY results were downranked.

### Pattern 4: Trait-Based Abstractions for Testing (from EmbedderTrait/VectorIndexTrait)

**What:** Core dedup logic depends on traits, not concrete types.
**When:** Any component that interacts with embedder or vector index.
**Why:** MockEmbedder and MockVectorIndex enable fast, deterministic unit tests.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Separate Dedup Index

**What:** Building a second HNSW index specifically for dedup checking.
**Why bad:** Double the maintenance, double the rebuild logic, double the disk usage. The in-flight buffer + existing HNSW covers the same ground with far less complexity.
**Instead:** In-flight buffer (256 entries, linear scan) + existing HNSW index.

### Anti-Pattern 2: Blocking Dedup Check

**What:** Making the IngestEvent RPC wait for dedup check with no timeout.
**Why bad:** Violates fail-open principle. If embedder is slow, all ingestion stalls.
**Instead:** Timeout (50ms default), fail-open on timeout.

### Anti-Pattern 3: Mutating Events for Staleness

**What:** Adding a `stale` flag to stored events or TOC nodes.
**Why bad:** Violates append-only model. Staleness is a read-time property that depends on what other content exists.
**Instead:** Compute staleness at query time from timestamps and similarity.

### Anti-Pattern 4: O(n^2) Pairwise Comparison on Large Result Sets

**What:** Running pairwise overlap detection on hundreds of results.
**Why bad:** 100 results = 4,950 comparisons, each requiring an embedding lookup.
**Instead:** Only apply pairwise overlap to the top-k results (10-20 max). Results beyond top-k are already low-ranked.

### Anti-Pattern 5: Dedup on Raw Events Instead of Content

**What:** Checking dedup at the raw event level (every user_message, tool_result, etc.).
**Why bad:** Many events are legitimately similar (e.g., "yes", "okay", session_start). Dedup should focus on substantive content.
**Instead:** Only dedup events with `min_text_length >= 50` (already in NoveltyConfig). Consider only user_message and assistant_message types.

## Scalability Considerations

| Concern | At 100 events/day | At 1K events/day | At 10K events/day |
|---------|-------------------|-------------------|-------------------|
| InFlightBuffer size | 256 entries plenty | 256 entries fine (5min TTL) | May need 512-1024 entries |
| Dedup latency | <5ms | <10ms (buffer scan) | <20ms (larger buffer) |
| HNSW search for dedup | <5ms | <10ms | <15ms (larger index) |
| Stale pairwise check | Negligible (10 results) | Negligible | Negligible (still 10-20 results) |
| Buffer memory | ~400KB | ~400KB | ~1.6MB at 1024 entries |

## Build Order (Dependency-Aware)

```
Phase 1: DedupGate foundation
    +--> DedupConfig in memory-types (extends NoveltyConfig)
    +--> InFlightBuffer in memory-service (pure data structure, no deps)
    +--> Enhanced NoveltyChecker with buffer integration
    +--> Unit tests with MockEmbedder + MockVectorIndex

Phase 2: Wire DedupGate into IngestEvent
    +--> Inject DedupGate into MemoryServiceImpl
    +--> Add dedup check before storage.put_event
    +--> Proto changes (IngestEventResponse.deduplicated)
    +--> Integration tests

Phase 3: StaleFilter
    +--> StaleConfig in memory-types
    +--> StaleFilter implementation
    +--> Integration with RetrievalExecutor (post-processing step)
    +--> Unit tests

Phase 4: E2E validation
    +--> E2E test: duplicate events rejected
    +--> E2E test: near-duplicate events rejected
    +--> E2E test: stale results downranked
    +--> E2E test: fail-open on embedder failure
    +--> CLI bats tests for dedup behavior
```

**Rationale for this order:**
1. DedupGate first because StaleFilter can be built independently, but DedupGate changes the write path (higher risk, needs more testing)
2. InFlightBuffer before wiring because it can be tested in isolation as a pure data structure
3. StaleFilter after DedupGate because it is read-path only (lower risk, no data mutation)
4. E2E last because it needs both features working end-to-end

## Sources

- Direct codebase analysis of:
  - `crates/memory-service/src/novelty.rs` -- existing NoveltyChecker pattern (fail-open, opt-in, metrics)
  - `crates/memory-service/src/ingest.rs` -- IngestEvent handler (MemoryServiceImpl, event storage)
  - `crates/memory-indexing/src/pipeline.rs` -- IndexingPipeline (outbox processing, checkpoint tracking)
  - `crates/memory-indexing/src/vector_updater.rs` -- VectorIndexUpdater (HNSW + Candle integration)
  - `crates/memory-vector/src/hnsw.rs` -- HnswIndex (usearch wrapper, cosine similarity)
  - `crates/memory-vector/src/index.rs` -- VectorIndex trait (search, add, remove interface)
  - `crates/memory-retrieval/src/executor.rs` -- RetrievalExecutor (fallback chains, merge, scoring)
  - `crates/memory-retrieval/src/types.rs` -- QueryIntent, CapabilityTier, StopConditions, ExecutionMode
  - `crates/memory-types/src/salience.rs` -- SalienceScorer (write-time importance scoring)
  - `crates/memory-types/src/usage.rs` -- UsageStats, usage_penalty (read-time decay)
  - `crates/memory-types/src/config.rs` -- NoveltyConfig, Settings (layered config)
  - `crates/memory-types/src/outbox.rs` -- OutboxEntry, OutboxAction (async index pipeline)
  - `.planning/PROJECT.md` -- requirements, architectural decisions, constraints
