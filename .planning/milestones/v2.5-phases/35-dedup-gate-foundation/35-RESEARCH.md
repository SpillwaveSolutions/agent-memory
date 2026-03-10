# Phase 35: DedupGate Foundation - Research

**Researched:** 2026-03-05
**Domain:** Semantic dedup gate with in-flight buffer, config, and fail-open behavior
**Confidence:** HIGH

## Summary

Phase 35 introduces an in-flight buffer data structure and enhances the existing NoveltyChecker to become a DedupGate that detects within-session semantic duplicates before events reach indexing. The existing codebase already has a `NoveltyChecker` in `memory-service/src/novelty.rs` with the correct fail-open architecture, `EmbedderTrait` and `VectorIndexTrait` abstractions, and `NoveltyConfig` in `memory-types/src/config.rs`. The key work is: (1) creating an `InFlightBuffer` data structure that holds up to 256 recent embeddings for fast similarity lookup, (2) renaming/evolving `NoveltyConfig` to `DedupConfig` with serde alias for backward compat, and (3) wiring the checker to use the in-flight buffer as its vector search target.

The existing `NoveltyChecker` searches the HNSW index (which contains TOC nodes, not raw events). For Phase 35, the dedup gate needs to search the `InFlightBuffer` (raw event embeddings from the current session). HNSW cross-session checking comes in Phase 36. The `InFlightBuffer` is a simple ring buffer of `(event_id, Vec<f32>)` pairs with brute-force cosine similarity search -- at 256 entries x 384 dimensions, this is ~400KB and sub-millisecond to scan.

**Primary recommendation:** Create `InFlightBuffer` as a standalone data structure in `memory-types` (no external dependencies), create `DedupConfig` replacing `NoveltyConfig` with serde alias, then enhance `NoveltyChecker` in `memory-service` to use `InFlightBuffer` as a `VectorIndexTrait` implementation with comprehensive unit tests.

## Standard Stack

### Core (Already in Workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| memory-types | workspace | DedupConfig, InFlightBuffer data structure | Shared types crate, no heavy deps |
| memory-service | workspace | Enhanced NoveltyChecker (becomes DedupGate logic) | Already has novelty.rs with correct architecture |
| memory-embeddings | workspace | CandleEmbedder (all-MiniLM-L6-v2, 384-dim) | Already validated in v2.0 |
| async-trait | workspace | Async trait bounds for EmbedderTrait/VectorIndexTrait | Already used in novelty.rs |
| tokio | workspace | Async runtime, timeout for fail-open | Already used in novelty.rs |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chrono | workspace | Timestamps for buffer entries | Already a workspace dep |
| tracing | workspace | Debug/warn logging for dedup decisions | Already used in novelty.rs |

### No New Dependencies Required

Phase 35 requires zero new crate dependencies. Everything needed is already in the workspace. The `InFlightBuffer` is a simple in-memory ring buffer using only `Vec<f32>` and standard library types.

## Architecture Patterns

### Where Things Live

```
crates/
  memory-types/src/
    config.rs          # DedupConfig (replaces NoveltyConfig) + serde alias
    dedup.rs           # NEW: InFlightBuffer data structure
    lib.rs             # Re-export DedupConfig, InFlightBuffer
  memory-service/src/
    novelty.rs         # Enhanced NoveltyChecker wired to InFlightBuffer
```

### Pattern 1: InFlightBuffer as Ring Buffer

**What:** Fixed-capacity circular buffer storing recent event embeddings for brute-force similarity search.
**When to use:** Within-session dedup where N is small (256) and brute-force is faster than index overhead.

```rust
// In memory-types/src/dedup.rs
pub struct InFlightBuffer {
    entries: Vec<BufferEntry>,
    capacity: usize,
    head: usize,  // next write position
    len: usize,   // current fill level
}

pub struct BufferEntry {
    pub event_id: String,
    pub embedding: Vec<f32>,
}

impl InFlightBuffer {
    pub fn new(capacity: usize) -> Self { ... }
    pub fn push(&mut self, event_id: String, embedding: Vec<f32>) { ... }
    pub fn search(&self, query: &[f32], threshold: f32) -> Option<(String, f32)> { ... }
    pub fn len(&self) -> usize { ... }
    pub fn clear(&mut self) { ... }
}
```

**Key design:** The `search` method does brute-force cosine similarity over all entries, returning the best match above threshold. At 256 x 384 floats, this is ~393KB and completes in microseconds. No index structure needed.

### Pattern 2: DedupConfig with Backward-Compatible Alias

**What:** Rename NoveltyConfig to DedupConfig, keep `[novelty]` as serde alias in config.toml.
**When to use:** Config evolution without breaking existing installations.

```rust
// In memory-types/src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_dedup_threshold")]
    pub threshold: f32,  // default 0.85 (changed from 0.82)

    #[serde(default = "default_dedup_timeout")]
    pub timeout_ms: u64,

    #[serde(default = "default_min_text_length")]
    pub min_text_length: usize,

    #[serde(default = "default_buffer_capacity")]
    pub buffer_capacity: usize,  // NEW: default 256
}

// Keep backward compat
pub type NoveltyConfig = DedupConfig;
```

### Pattern 3: VectorIndexTrait Adapter for InFlightBuffer

**What:** Wrap InFlightBuffer as a VectorIndexTrait impl so NoveltyChecker can use it without code changes.
**When to use:** When adapting a simple data structure to an existing trait interface.

```rust
// In memory-service/src/novelty.rs
pub struct InFlightBufferIndex {
    buffer: Arc<RwLock<InFlightBuffer>>,
}

#[async_trait]
impl VectorIndexTrait for InFlightBufferIndex {
    fn is_ready(&self) -> bool { true }  // always ready
    async fn search(&self, embedding: &[f32], top_k: usize) -> Result<Vec<(String, f32)>, String> {
        let buf = self.buffer.read().unwrap();
        // Return best match from brute-force scan
        ...
    }
}
```

### Pattern 4: Fail-Open at Every Level

**What:** The existing NoveltyChecker already implements fail-open correctly. The pattern must be preserved when adding InFlightBuffer.
**When to use:** Always -- this is a hard requirement (DEDUP-06).

The existing gates in `novelty.rs` (lines 143-238) already handle:
- Feature disabled -> pass through
- Short text -> pass through
- No embedder -> pass through
- No index -> pass through
- Index not ready -> pass through
- Timeout -> pass through
- Any error -> pass through

This pattern MUST be preserved. The InFlightBuffer adds no new failure modes (it's in-memory, always available).

### Anti-Patterns to Avoid

- **Separate HNSW for dedup:** Don't create a separate vector index for dedup. The InFlightBuffer handles within-session; the existing HNSW handles cross-session (Phase 36).
- **Locking the buffer during embed:** Don't hold a lock on InFlightBuffer while calling the embedder. Embed first, then lock-and-search, then lock-and-push.
- **Mutable NoveltyChecker:** The current NoveltyChecker takes `&self` not `&mut self`. The InFlightBuffer must be behind `Arc<RwLock<>>` or `Arc<Mutex<>>` to maintain this.
- **Breaking the append-only invariant:** Phase 35 only does detection. It does NOT change storage behavior. Events always get stored. Phase 36 handles the store-and-skip-outbox logic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cosine similarity | Custom SIMD impl | Simple dot product on normalized vectors | Vectors are pre-normalized by Embedding::new(); 256x384 is trivially fast |
| Config file parsing | Manual TOML parser | `config` crate (already in workspace) | Layered config already works |
| Async trait bounds | Manual vtable | `async-trait` crate (already in workspace) | Already used in novelty.rs |
| Ring buffer | External crate | Simple Vec + head/len tracking | 256 entries is trivial; no need for a crate |

**Key insight:** The InFlightBuffer is intentionally simple. At 256 entries, brute-force beats any index structure. The complexity is in the fail-open wiring, not the data structure.

## Common Pitfalls

### Pitfall 1: Threshold Confusion (Distance vs Similarity)
**What goes wrong:** usearch HNSW returns cosine distance (0=identical, 2=opposite), but the threshold is expressed as cosine similarity (1=identical, 0=orthogonal). The existing `HnswIndex.search()` already converts: `1.0 - dist`. But `InFlightBuffer.search()` must also return similarity, not distance.
**Why it happens:** Different vector libraries use different conventions.
**How to avoid:** InFlightBuffer computes dot product on normalized vectors (= cosine similarity directly). Ensure the comparison is `score >= threshold` for duplicate detection (matching the existing `score <= threshold` for novelty which uses inverted logic: `threshold` means "above this = duplicate").
**Warning signs:** Tests pass with threshold 0.85 but real duplicates aren't caught (or vice versa).

### Pitfall 2: NoveltyChecker Score Polarity
**What goes wrong:** The existing NoveltyChecker in `check_similarity()` line 256 does `Ok(*score <= self.config.threshold)` -- meaning "score <= threshold means NOVEL (not duplicate)". This is inverted from intuition. A HIGH score means MORE similar, and a score ABOVE threshold means DUPLICATE (returns false = don't store).
**Why it happens:** The method returns `is_novel` (true = store, false = reject). So `score <= threshold` = novel, `score > threshold` = duplicate.
**How to avoid:** Keep this exact polarity. The InFlightBuffer's VectorIndexTrait adapter must return scores in the same range (0-1 similarity, higher = more similar).
**Warning signs:** All events pass through or all get rejected.

### Pitfall 3: Settings Not Including NoveltyConfig
**What goes wrong:** Currently `NoveltyConfig` exists in `memory-types/src/config.rs` but `Settings` struct does NOT include it. The config is not loaded from config.toml.
**Why it happens:** Phase 16 created the config struct but never wired it into Settings loading.
**How to avoid:** Add `dedup` field (with `serde(alias = "novelty")`) to `Settings` struct so it gets loaded from `[dedup]` or `[novelty]` section in config.toml.
**Warning signs:** Changing config.toml has no effect on dedup behavior.

### Pitfall 4: Phase 35 Scope Creep into Phase 36
**What goes wrong:** Trying to wire the DedupGate into the actual ingest pipeline in Phase 35.
**Why it happens:** Natural desire to see it working end-to-end.
**How to avoid:** Phase 35 delivers: InFlightBuffer, DedupConfig, enhanced NoveltyChecker with unit tests using mocks. Phase 36 wires it into MemoryServiceImpl. Keep them separate.
**Warning signs:** Touching `ingest.rs` or `MemoryServiceImpl` in Phase 35.

### Pitfall 5: Embedding Dimension Mismatch
**What goes wrong:** InFlightBuffer stores raw `Vec<f32>` without validating dimension matches the embedder (384 for all-MiniLM-L6-v2).
**Why it happens:** The buffer is generic but the embedder is model-specific.
**How to avoid:** Validate dimension on `push()` or at construction time via a `dimension` field on InFlightBuffer.
**Warning signs:** Garbage similarity scores from mismatched vector lengths.

## Code Examples

### InFlightBuffer Core Implementation

```rust
// memory-types/src/dedup.rs

/// Entry in the in-flight dedup buffer.
#[derive(Debug, Clone)]
pub struct BufferEntry {
    pub event_id: String,
    pub embedding: Vec<f32>,
}

/// Fixed-capacity ring buffer for within-session dedup.
///
/// Stores recent event embeddings and performs brute-force
/// cosine similarity search. At 256 entries x 384 dimensions,
/// this is ~393KB and sub-millisecond to scan.
#[derive(Debug)]
pub struct InFlightBuffer {
    entries: Vec<Option<BufferEntry>>,
    capacity: usize,
    dimension: usize,
    head: usize,
    count: usize,
}

impl InFlightBuffer {
    pub fn new(capacity: usize, dimension: usize) -> Self {
        Self {
            entries: (0..capacity).map(|_| None).collect(),
            capacity,
            dimension,
            head: 0,
            count: 0,
        }
    }

    /// Add an embedding to the buffer, overwriting oldest if full.
    pub fn push(&mut self, event_id: String, embedding: Vec<f32>) {
        debug_assert_eq!(embedding.len(), self.dimension);
        self.entries[self.head] = Some(BufferEntry { event_id, embedding });
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Find the most similar entry above threshold.
    /// Returns (event_id, similarity_score) if found.
    pub fn find_similar(&self, query: &[f32], threshold: f32) -> Option<(String, f32)> {
        let mut best: Option<(String, f32)> = None;
        for entry in self.entries.iter().flatten() {
            let sim = cosine_similarity(query, &entry.embedding);
            if sim >= threshold {
                if best.as_ref().map_or(true, |(_, s)| sim > *s) {
                    best = Some((entry.event_id.clone(), sim));
                }
            }
        }
        best
    }

    pub fn len(&self) -> usize { self.count }
    pub fn is_empty(&self) -> bool { self.count == 0 }
    pub fn capacity(&self) -> usize { self.capacity }
}

/// Cosine similarity between two normalized vectors (= dot product).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
```

### DedupConfig Evolution

```rust
// memory-types/src/config.rs

/// Configuration for semantic dedup gate.
///
/// Replaces NoveltyConfig. The [novelty] TOML section is kept as
/// serde alias for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupConfig {
    #[serde(default)]
    pub enabled: bool,

    /// Similarity threshold -- events above this are considered duplicates.
    /// Range: 0.0-1.0. Default: 0.85 (conservative for all-MiniLM-L6-v2).
    #[serde(default = "default_dedup_threshold")]
    pub threshold: f32,

    /// Maximum time for dedup check (ms). Fail-open on timeout.
    #[serde(default = "default_dedup_timeout")]
    pub timeout_ms: u64,

    /// Minimum text length to check.
    #[serde(default = "default_min_text_length")]
    pub min_text_length: usize,

    /// In-flight buffer capacity (number of recent embeddings to keep).
    #[serde(default = "default_buffer_capacity")]
    pub buffer_capacity: usize,
}

fn default_dedup_threshold() -> f32 { 0.85 }
fn default_dedup_timeout() -> u64 { 50 }
fn default_buffer_capacity() -> usize { 256 }

/// Backward compatibility alias.
pub type NoveltyConfig = DedupConfig;
```

### Settings Integration

```rust
// In Settings struct (memory-types/src/config.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // ... existing fields ...

    /// Dedup gate configuration.
    /// Supports both [dedup] and [novelty] TOML sections.
    #[serde(default, alias = "novelty")]
    pub dedup: DedupConfig,
}
```

### Mock-Based Unit Test Pattern

```rust
// memory-service/src/novelty.rs tests

struct MockEmbedder {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbedderTrait for MockEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
        Ok(self.embedding.clone())
    }
}

struct MockVectorIndex {
    results: Vec<(String, f32)>,
    ready: bool,
}

#[async_trait]
impl VectorIndexTrait for MockVectorIndex {
    fn is_ready(&self) -> bool { self.ready }
    async fn search(&self, _embedding: &[f32], _top_k: usize) -> Result<Vec<(String, f32)>, String> {
        Ok(self.results.clone())
    }
}

#[tokio::test]
async fn test_duplicate_detected_above_threshold() {
    let config = DedupConfig { enabled: true, threshold: 0.85, ..Default::default() };
    let embedder = Arc::new(MockEmbedder { embedding: vec![1.0; 384] });
    let index = Arc::new(MockVectorIndex {
        results: vec![("existing-event".to_string(), 0.92)],  // above 0.85
        ready: true,
    });
    let checker = NoveltyChecker::new(Some(embedder), Some(index), config);
    let event = test_event("This text is a near-duplicate of something in the buffer");
    assert!(!checker.should_store(&event).await);  // rejected as duplicate
}

#[tokio::test]
async fn test_fail_open_on_embedder_error() {
    // Embedder that always fails
    struct FailingEmbedder;
    #[async_trait]
    impl EmbedderTrait for FailingEmbedder {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
            Err("model load failed".to_string())
        }
    }
    let config = DedupConfig { enabled: true, ..Default::default() };
    let checker = NoveltyChecker::new(
        Some(Arc::new(FailingEmbedder)),
        Some(Arc::new(MockVectorIndex { results: vec![], ready: true })),
        config,
    );
    assert!(checker.should_store(&test_event("test")).await);  // fail-open: stored
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| NoveltyConfig (disabled, threshold 0.82) | DedupConfig (disabled, threshold 0.85) | Phase 35 | Higher threshold = more conservative dedup |
| NoveltyChecker searches HNSW only | NoveltyChecker searches InFlightBuffer first | Phase 35 | Catches within-session dupes without HNSW |
| No in-memory buffer | InFlightBuffer (256 entries, ring) | Phase 35 | Fast within-session dedup |
| NoveltyConfig not in Settings | DedupConfig in Settings with alias | Phase 35 | Config actually loadable from config.toml |

## Open Questions

1. **Should InFlightBuffer be per-session or global?**
   - What we know: The requirement says "within-session duplicates" and buffer is 256 entries.
   - What's unclear: Whether the buffer should reset on session_start or persist across sessions.
   - Recommendation: Global buffer (shared across sessions). The ring buffer naturally ages out old entries. Per-session would require session tracking complexity and miss cross-session duplicates within the buffer window. Phase 36 adds HNSW for full cross-session.

2. **Should DedupConfig.enabled default change from false to true?**
   - What we know: NoveltyConfig defaults to disabled. The roadmap implies dedup should be active.
   - What's unclear: Whether enabling by default would surprise existing users upgrading.
   - Recommendation: Keep `enabled: false` as default for Phase 35. Phase 36 (ingest wiring) can flip the default when the full pipeline is ready and tested.

## Sources

### Primary (HIGH confidence)
- `crates/memory-service/src/novelty.rs` -- existing NoveltyChecker with full fail-open architecture
- `crates/memory-types/src/config.rs` -- existing NoveltyConfig struct, Settings struct (missing dedup field)
- `crates/memory-embeddings/src/candle.rs` -- CandleEmbedder, 384-dim, all-MiniLM-L6-v2
- `crates/memory-embeddings/src/model.rs` -- Embedding type with cosine_similarity, normalization
- `crates/memory-vector/src/hnsw.rs` -- HnswIndex using usearch, cosine metric
- `crates/memory-vector/src/index.rs` -- VectorIndex trait (for reference, separate from VectorIndexTrait in novelty.rs)
- `.planning/REQUIREMENTS.md` -- DEDUP-01, DEDUP-05, DEDUP-06
- `.planning/ROADMAP.md` -- Phase 35 scope, plans 35-01 and 35-02

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` -- Prior decisions (threshold 0.85, InFlightBuffer, structural bypass, serde alias)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all dependencies already in workspace, no new crates needed
- Architecture: HIGH -- existing NoveltyChecker provides the exact pattern to follow; InFlightBuffer is trivially simple
- Pitfalls: HIGH -- score polarity issue is verified from source code (line 256 of novelty.rs); Settings gap verified from config.rs

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (stable -- all code is local, no external API changes)
