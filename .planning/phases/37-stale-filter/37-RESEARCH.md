# Phase 37: StaleFilter - Research

**Researched:** 2026-03-06
**Domain:** Query-time result scoring adjustment (time-decay + supersession)
**Confidence:** HIGH

## Summary

Phase 37 adds a StaleFilter component that adjusts query result scores post-merge in the retrieval pipeline. The filter applies exponential time-decay relative to the newest result in the set (not query time), marks older semantically-similar results as superseded with an additional penalty, and exempts high-salience memory kinds (Constraint, Definition, Procedure, Preference) from both penalties.

The critical implementation challenge is that `SearchResult` currently carries NO timestamp and NO memory_kind information. The `SimpleLayerExecutor` in `memory-service/src/retrieval.rs` constructs `SearchResult` with `metadata: HashMap::new()` for all layers, discarding timestamps from `TeleportResult.timestamp_ms` (BM25) and `VectorEntry.created_at` (vector). Plan 37-01 must first ensure timestamps and memory_kind flow through `SearchResult.metadata`, then implement the scoring math. Supersession detection requires pairwise similarity between results, which means either re-embedding at query time (expensive) or carrying embedding vectors in the result set.

**Primary recommendation:** Enrich `SearchResult.metadata` with `timestamp_ms` and `memory_kind` in the `SimpleLayerExecutor` before building the StaleFilter. For supersession, use pairwise cosine similarity on embeddings retrieved from VectorMetadata by doc_id, keeping it lazy (only when StaleFilter is enabled).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Reference point: relative to the newest result in the result set (not query time). If all results are old, no penalty applies.
- Formula: `score_adj = score * (1.0 - max_penalty * (1 - e^(-age/half_life)))` where age = newest_timestamp - this_timestamp
- Asymptotic approach to max penalty (never reaches 30%, smoothly approaches it)
- At 1 half-life (~14 days): ~19% penalty. At 2 half-lives: ~26%. At 3 half-lives: ~28.6%.
- Default half-life: 14 days (configurable via config.toml)
- Max penalty: 30% score reduction (asymptotic bound, not hard floor)
- Superseded results get a fixed 15% additional penalty on top of time-decay
- No transitivity: each result marked superseded at most once, even if multiple newer results are similar
- Add `superseded_by: <doc_id>` to result's metadata HashMap for explainability (no proto change needed)
- Supersession similarity threshold: 0.80 (lower than dedup's 0.85)
- Exempt from BOTH time-decay AND supersession: Constraint, Definition, Procedure, Preference
- Only Observation gets full decay treatment
- Hardcoded enum match (not configurable)
- Follows DedupConfig pattern for config struct design (flat struct, serde defaults)
- Enabled by default (unlike dedup which is opt-in)
- Combined max theoretical penalty: ~45% (30% decay + 15% supersession)

### Claude's Discretion
- Whether to apply uniform decay across retrieval layers or reduce decay for semantic results
- Whether to add a per-query `skip_staleness` flag (likely defer -- no proto change this phase)
- How to handle high-salience Observations (threshold exemption vs full decay)
- Supersession detection method: pairwise cosine check vs topic-based grouping

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| memory-retrieval | workspace | StaleFilter struct lives here alongside existing executor/scoring | Co-located with SearchResult and ExecutionResult |
| memory-types | workspace | StalenessConfig struct, MemoryKind re-use | Follows DedupConfig pattern in config.rs |
| memory-service | workspace | Wire StaleFilter into RetrievalHandler, enrich metadata | Where SimpleLayerExecutor converts layer results |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chrono | existing dep | Timestamp math for age calculation | Convert ms-since-epoch to duration |
| serde | existing dep | Config deserialization with defaults | StalenessConfig serde attributes |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Pairwise cosine for supersession | Topic-based grouping | Pairwise is O(n^2) but n is small (10-20 results); topic grouping requires topic graph availability |
| Carrying embeddings in results | Re-embedding at query time | Carrying is cheaper; re-embedding adds latency |

**Installation:**
No new dependencies needed. All required crates are already in the workspace.

## Architecture Patterns

### Recommended Project Structure
```
crates/memory-retrieval/src/
├── executor.rs       # SearchResult (add timestamp_ms, memory_kind to metadata)
├── stale_filter.rs   # NEW: StaleFilter, StalenessResult, time-decay + supersession logic
└── lib.rs            # Re-export StaleFilter

crates/memory-types/src/
├── config.rs         # ADD: StalenessConfig (follows DedupConfig pattern)
└── salience.rs       # EXISTING: MemoryKind enum (reuse, no changes)

crates/memory-service/src/
└── retrieval.rs      # MODIFY: SimpleLayerExecutor metadata enrichment + StaleFilter wiring
```

### Pattern 1: Metadata Enrichment in SimpleLayerExecutor
**What:** Pass `timestamp_ms` and `memory_kind` through `SearchResult.metadata` HashMap
**When to use:** When converting `TeleportResult` or `VectorEntry` to `SearchResult`
**Example:**
```rust
// In SimpleLayerExecutor::execute for BM25 layer
CrateLayer::BM25 => {
    if let Some(searcher) = &self.bm25_searcher {
        let opts = memory_search::SearchOptions::new().with_limit(limit);
        let results = searcher.search(query, opts).map_err(|e| e.to_string())?;
        Ok(results
            .into_iter()
            .map(|r| {
                let mut metadata = HashMap::new();
                if let Some(ts) = r.timestamp_ms {
                    metadata.insert("timestamp_ms".to_string(), ts.to_string());
                }
                // memory_kind not available from BM25 results -- use "observation" default
                metadata.insert("memory_kind".to_string(), "observation".to_string());
                SearchResult {
                    doc_id: r.doc_id,
                    doc_type: format!("{:?}", r.doc_type).to_lowercase(),
                    score: r.score,
                    text_preview: r.keywords.unwrap_or_default(),
                    source_layer: CrateLayer::BM25,
                    metadata,
                }
            })
            .collect())
    } else {
        Err("BM25 not available".to_string())
    }
}
```

### Pattern 2: StaleFilter as Post-Merge Scoring Pass
**What:** A pure function that takes `Vec<SearchResult>` + `StalenessConfig` and returns scored results
**When to use:** Applied after `merge_results` or `select_best_results`, before results are returned
**Example:**
```rust
// StaleFilter is a pure scoring pass -- no side effects
pub struct StaleFilter {
    config: StalenessConfig,
}

impl StaleFilter {
    pub fn apply(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        if results.is_empty() || !self.config.enabled {
            return results;
        }

        let newest_ts = self.find_newest_timestamp(&results);
        let mut adjusted = self.apply_time_decay(results, newest_ts);
        self.apply_supersession(&mut adjusted);
        adjusted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        adjusted
    }
}
```

### Pattern 3: StalenessConfig Following DedupConfig Pattern
**What:** Flat config struct with serde defaults, `[staleness]` TOML section
**When to use:** Configuration loading
**Example:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessConfig {
    #[serde(default = "default_staleness_enabled")]
    pub enabled: bool,  // true by default (unlike DedupConfig)

    #[serde(default = "default_half_life_days")]
    pub half_life_days: f32,  // 14.0

    #[serde(default = "default_max_penalty")]
    pub max_penalty: f32,  // 0.30

    #[serde(default = "default_supersession_penalty")]
    pub supersession_penalty: f32,  // 0.15

    #[serde(default = "default_supersession_threshold")]
    pub supersession_threshold: f32,  // 0.80
}
```

### Anti-Patterns to Avoid
- **Mutating scores in place during iteration:** Use a two-pass approach (time-decay first, then supersession) to avoid order-dependent scoring artifacts
- **Using query time as reference:** The CONTEXT.md locks the reference point as the newest result timestamp. Using wall-clock time would penalize ALL results in slow-moving projects
- **Supersession transitivity chains:** Each result can only be superseded once. Do NOT chain A->B->C supersession
- **Score floor instead of asymptotic bound:** The formula is multiplicative with asymptotic approach to 30%. Do NOT clamp with `max(score * 0.70, ...)` -- use the exponential formula

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Timestamp extraction from results | Custom timestamp parsing per layer | `SearchResult.metadata["timestamp_ms"]` convention | Already stored in BM25 (TeleportResult) and vector (VectorEntry) layers |
| MemoryKind classification | Re-classifying text at query time | Store memory_kind during indexing OR default to Observation | Classification is write-time concern per Phase 16 design |
| Config loading | Custom TOML parsing | `config` crate + serde, same as DedupConfig | Existing layered config infrastructure handles this |
| Cosine similarity | Manual vector math | Use existing dot-product pattern from dedup | Vectors are pre-normalized by CandleEmbedder |

**Key insight:** The StaleFilter is a pure scoring transformation. It should not perform I/O, embed text, or access storage. All inputs come through `SearchResult.metadata` and the result's `score` field.

## Common Pitfalls

### Pitfall 1: Missing Timestamps in SearchResult
**What goes wrong:** StaleFilter receives results with no `timestamp_ms` in metadata, cannot compute decay
**Why it happens:** `SimpleLayerExecutor` currently creates empty `HashMap::new()` for all layers, discarding timestamps from `TeleportResult.timestamp_ms` and `VectorEntry.created_at`
**How to avoid:** Enrich metadata in SimpleLayerExecutor BEFORE wiring StaleFilter. Fail-open: results without timestamps get no penalty.
**Warning signs:** All results get score 1.0 adjustment (no decay applied)

### Pitfall 2: Supersession Without Embeddings
**What goes wrong:** Cannot compute semantic similarity between results without their embedding vectors
**Why it happens:** SearchResult does not carry embedding vectors -- they live in VectorMetadata (RocksDB) or HNSW index
**How to avoid:** For supersession, look up embeddings from VectorMetadata by doc_id. If embeddings unavailable (BM25-only results), skip supersession for those results. This is fail-open behavior.
**Warning signs:** Supersession never triggers for BM25-only queries

### Pitfall 3: Score Collapse with Compound Penalties
**What goes wrong:** Results with both time-decay and supersession get penalized below useful thresholds
**Why it happens:** Max combined penalty is ~45% (30% decay + 15% supersession), which can drop a 0.5 score to ~0.275
**How to avoid:** The 30% max_penalty is asymptotic (never actually reached), and 15% supersession is a flat multiplier. Combined: `score * (1 - 0.30 * decay_factor) * 0.85` at worst. Verify in tests that minimum practical scores remain above 0.2 for reasonable inputs.
**Warning signs:** Results disappearing from top-N when they should still be relevant

### Pitfall 4: MemoryKind Not Available from Search Layers
**What goes wrong:** StaleFilter cannot determine which results to exempt from decay because memory_kind is not indexed in BM25 or vector stores
**Why it happens:** MemoryKind is computed at write time by SalienceScorer but not stored in search indexes (BM25 schema has no memory_kind field; VectorEntry has no memory_kind field)
**How to avoid:** Two options: (1) Default all search results to `Observation` and accept that exemption only works when kind is explicitly available; (2) Store memory_kind in TOC node metadata and propagate through search. Option 1 is pragmatic for 37-01; Option 2 can be a future enhancement.
**Warning signs:** Constraint/Definition results getting time-decayed when they shouldn't

### Pitfall 5: Supersession O(n^2) with Large Result Sets
**What goes wrong:** Pairwise cosine comparison becomes expensive with many results
**Why it happens:** Comparing each result pair is O(n^2); with 100 results that's 4,950 comparisons
**How to avoid:** Result sets are typically capped at 10-20 by StopConditions.max_nodes. At n=20, only 190 comparisons. This is negligible. If limit ever increases, add an early-exit when n > 50 (skip supersession).
**Warning signs:** Route query latency spikes when returning many results

## Code Examples

### Time-Decay Formula Implementation
```rust
// Source: CONTEXT.md locked formula
// score_adj = score * (1.0 - max_penalty * (1.0 - e^(-age/half_life)))
fn apply_time_decay_factor(score: f32, age_days: f64, config: &StalenessConfig) -> f32 {
    let half_life = config.half_life_days as f64;
    let decay = 1.0 - (-age_days / half_life).exp();  // 0 at age=0, approaches 1 at infinity
    let penalty = config.max_penalty as f64 * decay;   // 0 at age=0, approaches max_penalty
    let factor = 1.0 - penalty;                         // 1.0 at age=0, approaches (1 - max_penalty)
    (score as f64 * factor) as f32
}

// At age = 0 days:    factor = 1.000 (no penalty)
// At age = 14 days:   factor = 0.811 (~19% penalty)
// At age = 28 days:   factor = 0.740 (~26% penalty)
// At age = 42 days:   factor = 0.714 (~28.6% penalty)
// At age = infinity:  factor = 0.700 (max 30% penalty, never reached)
```

### Kind Exemption Check
```rust
// Source: memory-types/src/salience.rs MemoryKind enum
use memory_types::salience::MemoryKind;

fn is_exempt_from_staleness(kind: &MemoryKind) -> bool {
    matches!(
        kind,
        MemoryKind::Constraint | MemoryKind::Definition | MemoryKind::Procedure | MemoryKind::Preference
    )
}
```

### Supersession Detection (Pairwise)
```rust
// Compare newer results against older ones
// Results should be sorted by timestamp descending (newest first)
fn detect_supersession(
    results: &mut [SearchResult],
    threshold: f32,
    embeddings: &HashMap<String, Vec<f32>>,  // doc_id -> embedding
) {
    let n = results.len();
    for i in 0..n {
        if results[i].metadata.contains_key("superseded_by") {
            continue;  // Already superseded
        }
        // Only Observations can be superseded
        let kind_i = results[i].metadata.get("memory_kind")
            .and_then(|k| parse_memory_kind(k))
            .unwrap_or(MemoryKind::Observation);
        if is_exempt_from_staleness(&kind_i) {
            continue;
        }

        let ts_i = parse_timestamp(&results[i]);

        for j in 0..n {
            if i == j { continue; }
            let ts_j = parse_timestamp(&results[j]);
            if ts_j <= ts_i { continue; }  // j must be newer than i

            // Check semantic similarity
            if let (Some(emb_i), Some(emb_j)) = (
                embeddings.get(&results[i].doc_id),
                embeddings.get(&results[j].doc_id),
            ) {
                let similarity = dot_product(emb_i, emb_j);  // Pre-normalized vectors
                if similarity >= threshold {
                    results[i].metadata.insert(
                        "superseded_by".to_string(),
                        results[j].doc_id.clone(),
                    );
                    results[i].score *= 1.0 - 0.15;  // 15% supersession penalty
                    break;  // No transitivity -- superseded at most once
                }
            }
        }
    }
}
```

### StalenessConfig in Settings
```rust
// In Settings struct (memory-types/src/config.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // ... existing fields ...

    /// Staleness filter configuration.
    #[serde(default)]
    pub staleness: StalenessConfig,
}

// In config.toml
// [staleness]
// enabled = true
// half_life_days = 14.0
// max_penalty = 0.30
// supersession_penalty = 0.15
// supersession_threshold = 0.80
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No query-time scoring adjustment | StaleFilter with time-decay | Phase 37 (new) | Results freshness-weighted |
| Results returned as-is from layers | Post-merge scoring pass | Phase 37 (new) | Score semantics change |
| Empty SearchResult.metadata | Enriched with timestamp_ms, memory_kind | Phase 37 prerequisite | Enables all query-time adjustments |

**Current state of SearchResult.metadata:**
- Currently EMPTY (`HashMap::new()`) for all layer results in SimpleLayerExecutor
- BM25 `TeleportResult` HAS `timestamp_ms: Option<i64>` and `agent: Option<String>` -- being discarded
- Vector `VectorEntry` HAS `created_at: i64` and `agent: Option<String>` -- being discarded
- No memory_kind in any search index schema

## Open Questions

1. **How to get memory_kind into search results?**
   - What we know: MemoryKind is classified at write time by SalienceScorer. It's stored in... nowhere searchable. TOC nodes don't carry memory_kind. Neither BM25 schema nor VectorEntry has a memory_kind field.
   - What's unclear: Whether to index memory_kind in BM25/vector, or infer it, or default to Observation.
   - Recommendation: Default all results to `Observation` for Phase 37. This means exempt kinds (Constraint, etc.) will still get decayed. Accept this limitation. A future phase could add memory_kind to the indexing pipeline. This is the pragmatic choice because (a) it avoids schema changes, (b) most results are observations anyway, and (c) it still delivers the core value of time-decay.

2. **Where to get embeddings for supersession detection?**
   - What we know: VectorMetadata stores doc_id -> vector_id mappings. The HNSW index stores actual vectors. `VectorTeleportHandler` has access to both.
   - What's unclear: Whether to pass embeddings through SearchResult or look them up lazily in StaleFilter.
   - Recommendation: Have the StaleFilter accept an optional `embeddings: HashMap<String, Vec<f32>>` parameter. The wiring code in RetrievalHandler looks up embeddings for the result set's doc_ids before calling StaleFilter. If vector layer is unavailable, skip supersession (fail-open). This keeps StaleFilter pure and testable.

3. **Uniform decay vs layer-specific decay (Claude's discretion)?**
   - What we know: Semantic results (vector) already have similarity-based scoring. BM25 results have TF-IDF scoring. Time-decay applies uniformly.
   - Recommendation: Apply uniform decay. The time-decay formula adjusts the absolute score, not the relative ranking. Since scores across layers are already heterogeneous, adding layer-specific decay coefficients would over-complicate without clear benefit. Keep it simple.

4. **High-salience Observations (Claude's discretion)?**
   - What we know: Salience is computed at write time (0.0-1.2 range). High-salience observations could be exempted.
   - Recommendation: Apply full decay to all Observations regardless of salience. Reason: salience is about importance, staleness is about freshness. An important but outdated observation should still be downranked. If needed, a future "salience boost" could counterbalance staleness.

## Sources

### Primary (HIGH confidence)
- `crates/memory-retrieval/src/executor.rs` - SearchResult struct, ExecutionResult, merge_results pattern
- `crates/memory-types/src/config.rs` - DedupConfig pattern for StalenessConfig
- `crates/memory-types/src/salience.rs` - MemoryKind enum variants and exemption logic
- `crates/memory-topics/src/importance.rs` - Existing time-decay formula reference (half-life pattern)
- `crates/memory-service/src/retrieval.rs` - SimpleLayerExecutor, RetrievalHandler wiring point
- `crates/memory-search/src/searcher.rs` - TeleportResult with timestamp_ms field
- `crates/memory-vector/src/metadata.rs` - VectorEntry with created_at field
- `.planning/phases/37-stale-filter/37-CONTEXT.md` - Locked decisions and formula

### Secondary (MEDIUM confidence)
- `crates/memory-search/src/schema.rs` - BM25 schema fields (timestamp_ms stored as string)
- `crates/memory-search/src/document.rs` - How timestamps get indexed in BM25

### Tertiary (LOW confidence)
- None -- all findings based on codebase inspection

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All code paths inspected, crate structure understood
- Architecture: HIGH - Integration point clearly identified (SimpleLayerExecutor + RetrievalHandler)
- Pitfalls: HIGH - Metadata gap (empty HashMap) verified by code inspection
- Time-decay formula: HIGH - Locked in CONTEXT.md with exact math

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain, no external dependencies)
