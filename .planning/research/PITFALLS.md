# Domain Pitfalls: Semantic Dedup & Retrieval Quality

**Domain:** Vector-based ingest-time deduplication and stale result filtering for append-only event store
**Researched:** 2026-03-05
**Overall Confidence:** HIGH (verified against codebase architecture, all-MiniLM-L6-v2 documentation, and vector search community patterns)

---

## Critical Pitfalls

Mistakes that cause data loss, silent retrieval degradation, or require architectural rework.

---

### Pitfall 1: Dedup Window Gap -- Recent Events Not Yet Indexed

**What goes wrong:** Ingest-time dedup checks the HNSW vector index for similar events, but the vector index is populated asynchronously via the outbox pipeline. Events ingested in rapid succession (e.g., during an active session) will NOT find each other as duplicates because the first event has not been indexed yet when the second arrives.

**Why it happens:** The current architecture is: `IngestEvent -> RocksDB + Outbox (synchronous) -> IndexingPipeline drains outbox -> HNSW index (asynchronous)`. The outbox is processed in batches (`batch_size: 100`) by the scheduler. Between the time an event is stored and the time its embedding lands in the HNSW index, there is a gap of seconds to minutes depending on scheduler interval. Dedup checks against the HNSW index during this gap find nothing.

**Codebase evidence:** `crates/memory-indexing/src/pipeline.rs` processes outbox entries in `process_batch()`, and `crates/memory-indexing/src/vector_updater.rs` only indexes TOC nodes and grips (not raw events). The existing `NoveltyChecker` in `crates/memory-service/src/novelty.rs` already has a `skipped_index_not_ready` metric for exactly this case -- it fails open and stores anyway.

**Consequences:** Burst duplicates (the most common kind -- repeated tool calls, re-sent messages within a session) are exactly the duplicates that NEVER get caught. The system catches only duplicates separated by enough time for the index to catch up. This is backwards: the most valuable dedup happens within-session, not across-sessions.

**Prevention:**
1. Maintain a short-lived in-memory "recent embeddings" buffer (last N embeddings, ring buffer) alongside the HNSW index. Check both during dedup.
2. Size the buffer to cover the maximum expected indexing lag (e.g., 500 embeddings covers ~5 minutes at 100 events/minute).
3. The buffer is volatile (lost on restart) but that is acceptable -- the HNSW index handles cross-restart dedup, the buffer handles within-session dedup.
4. Alternative: perform synchronous embedding + HNSW insertion on the ingest path for dedup purposes, while keeping the outbox for TOC/BM25 indexing. This is simpler but adds latency to the ingest path.

**Detection:** Metric: `rejected_duplicate` count is near zero during active sessions. Events with identical or near-identical text appear in close succession in the event store.

**Phase to address:** Dedup implementation phase. This is THE critical design decision -- the dedup architecture cannot be retrofitted easily.

**Severity:** CRITICAL

---

### Pitfall 2: Threshold Miscalibration for all-MiniLM-L6-v2

**What goes wrong:** The similarity threshold for "duplicate" is set too high (false negatives -- duplicates slip through) or too low (false positives -- unique events silently dropped). With all-MiniLM-L6-v2, the cosine similarity distribution is model-specific and non-intuitive.

**Why it happens:** all-MiniLM-L6-v2 cosine similarity scores are positively skewed in the range [0.07, 0.80] with a mean around 0.39 for unrelated content. Research shows that a threshold of 0.659 was used for literature deduplication. The current `NoveltyConfig` has a default threshold of 0.82, which was set for novelty filtering (a different use case than dedup). Cosine similarity with this model does NOT produce scores near 1.0 for paraphrased content -- semantically similar but differently worded text might score 0.65-0.80, while true duplicates (identical or near-identical text) score 0.85+.

**Codebase evidence:** `crates/memory-types/src/config.rs` shows `default_novelty_threshold()` returns 0.82. The HNSW index uses `MetricKind::Cos` (cosine distance) in `crates/memory-vector/src/hnsw.rs`, and the search method converts distance to similarity via `1.0 - dist`.

**The danger zones:**
- Threshold 0.90+: Only catches verbatim duplicates. Misses paraphrased content. Safe but mostly useless.
- Threshold 0.80-0.90: Catches near-duplicates. Sweet spot for conversational memory. Some paraphrases caught.
- Threshold 0.70-0.80: Aggressive dedup. Will catch related-but-different content. Risk of dropping updates to earlier topics.
- Threshold below 0.70: DANGEROUS. Will merge unrelated content. High false positive rate with this model.

**Consequences:** False positives cause PERMANENT data loss in an append-only store. Unlike stale filtering (which only affects ranking), dedup at ingest means the event is never stored. There is no undo. A user saying "implement authentication" and later "update authentication" could be deduplicated if the threshold is too low.

**Prevention:**
1. Default threshold should be conservative: 0.85 for dedup (higher than the 0.82 novelty threshold). Dedup consequences are irreversible; novelty filtering just hides results.
2. Log every rejected event with its similarity score and the doc_id of the matched "duplicate." This is essential for debugging and threshold tuning.
3. Provide a `dedup_dry_run` mode that logs what WOULD be rejected without actually rejecting. Run this in production for a week before enabling dedup.
4. Make threshold configurable per event_type. `session_start` and `session_end` events are frequently identical and should have a LOWER threshold (easier to dedup). `user_message` and `assistant_stop` events should have a HIGHER threshold (more diverse content, higher cost of false positives).
5. Consider a compound check: cosine similarity above threshold AND text overlap (e.g., Jaccard on tokens) above a second threshold. This dramatically reduces false positives.

**Detection:** Metric: ratio of `rejected_duplicate` to `total_ingested`. If above 20%, something is wrong. Audit log of rejected events with similarity scores.

**Phase to address:** Dedup implementation phase. Threshold tuning should be a separate sub-task with its own test fixtures.

**Severity:** CRITICAL

---

### Pitfall 3: Dedup Breaks the Append-Only Invariant Semantics

**What goes wrong:** The system's fundamental design principle is "append-only truth" -- events are immutable and never deleted. Ingest-time dedup silently drops events BEFORE they are appended, which is philosophically different from "stored but marked as duplicate" or "stored but downranked." Downstream components (TOC builder, segment creator, grip extractor) that rely on seeing every event may produce incorrect summaries.

**Why it happens:** The current ingest path writes events unconditionally. TOC segmentation uses event count and time thresholds. Rollup jobs aggregate ALL events in a time window. If dedup drops 30% of events in a busy session, segments become larger (fewer events = fewer segment boundaries), TOC summaries become sparser, and the progressive disclosure hierarchy becomes less navigable.

**Codebase evidence:** `crates/memory-toc/src/segmenter.rs` creates segments based on event count and time gaps. Dropping events changes both metrics. The TOC builder in `crates/memory-toc/src/builder.rs` assumes it sees all events.

**Consequences:** TOC quality degrades silently. Segments cover longer time spans. Day-level summaries miss topics that were discussed in deduplicated events. The user asks "what did we discuss about authentication?" and the TOC does not mention it because the relevant events were deduped.

**Prevention:**
1. Store ALL events unconditionally. Dedup should add a `dedup_status` field to the event metadata, NOT prevent storage. The event is stored but the outbox entry is NOT created (so it does not get indexed). This preserves the append-only invariant while preventing index bloat.
2. Alternative (less preferred): Store a lightweight "dedup tombstone" that records the event_id, timestamp, and the matched_event_id without the full text. This preserves the event count for segmentation while saving storage.
3. If true drop-at-ingest is required: adjust segment thresholds to account for dedup. But this couples two independent systems and is fragile.
4. TOC rollup jobs should NOT be affected by dedup regardless of approach -- they operate on stored events and TOC nodes, not on the index.

**Detection:** Compare event counts per segment before and after enabling dedup. If segments grow significantly larger, the segmenter is seeing fewer events.

**Phase to address:** Dedup design phase. This is an architectural decision that must be made before implementation.

**Severity:** CRITICAL

---

### Pitfall 4: Stale Filtering Hides Critical Historical Context

**What goes wrong:** Stale result filtering downranks or removes older results, but in conversational memory, old context is frequently the MOST important. A user asking "what was the authentication approach we decided on?" needs the original decision (old), not the latest passing mention (new).

**Why it happens:** Stale filtering assumes that newer = more relevant. This is true for news feeds but false for decision records, architectural choices, and procedural knowledge. The existing ranking policy already has a usage decay factor (`crates/memory-types/src/usage.rs`), salience scoring (`crates/memory-types/src/salience.rs`), and novelty filtering (`crates/memory-service/src/novelty.rs`). Adding ANOTHER time-based penalty stacks decay effects, potentially burying high-salience old content.

**Codebase evidence:** The ranking policy in Layer 6 already applies `usage decay` (recent usage boosts ranking). The `SalienceScorer` assigns higher scores to constraints, definitions, and procedures -- exactly the high-value historical content that stale filtering would bury. The retrieval executor in `crates/memory-retrieval/src/executor.rs` applies `min_confidence` thresholds -- stale-penalized results could fall below this threshold and be discarded entirely.

**Consequences:** The system forgets its most important memories. Decisions are re-debated because the original rationale is downranked below the confidence threshold. The progressive disclosure architecture (TOC navigation) becomes the only reliable way to find old content, defeating the purpose of semantic search.

**Prevention:**
1. Stale filtering should NEVER apply to content classified as `Constraint`, `Definition`, or `Procedure` by the `SalienceScorer`. These memory kinds are timeless by nature.
2. Implement "supersession" not "staleness." An event is stale only if a NEWER event on the SAME topic exists with higher relevance. This requires topic-aware filtering, not simple time decay.
3. Stale penalty should be a multiplicative factor (e.g., 0.95 per week) applied AFTER salience scoring, not a hard cutoff. High-salience old content should still rank above low-salience new content.
4. The existing `novelty` system already handles "don't show me what I just saw." Stale filtering should handle "among these results, prefer recent ones" -- these are different concerns and should not be conflated.
5. Provide a `include_stale: bool` flag on the query API so agents can opt out of stale filtering for specific queries (e.g., "what did we decide about X last month?").

**Detection:** User reports of "I know we discussed X but the system can't find it." High-salience events (score > 0.8) appearing deep in result lists or not appearing at all.

**Phase to address:** Stale filtering implementation phase. Must be designed together with the existing ranking policy, not as an independent layer.

**Severity:** CRITICAL

---

## Moderate Pitfalls

Mistakes that cause performance issues, flaky tests, or degraded-but-recoverable behavior.

---

### Pitfall 5: Embedding Latency on the Ingest Hot Path

**What goes wrong:** Adding dedup to the ingest path means generating an embedding for EVERY incoming event BEFORE storing it. The Candle-based all-MiniLM-L6-v2 embedder runs on CPU. On macOS Apple Silicon this is fast (~5ms), but on CI Linux runners or older hardware, embedding generation can take 20-50ms per event. During a burst ingest (session with 100+ events), this adds 2-5 seconds of synchronous latency.

**Why it happens:** The current architecture generates embeddings only in the async indexing pipeline (not on the ingest path). Moving embedding to the ingest path is a fundamental change in the latency profile. The existing `NoveltyChecker` has a `timeout_ms` of 50ms for exactly this reason -- it expects the embedding to be fast but sets a hard timeout.

**Codebase evidence:** `crates/memory-embeddings/src/candle.rs` runs inference synchronously. The `NoveltyChecker` in `crates/memory-service/src/novelty.rs` wraps embedding in `tokio::time::timeout(timeout_duration, ...)`.

**Consequences:** Hook handlers (which call IngestEvent via gRPC) start timing out. The fail-open design means events are stored without dedup check, defeating the purpose. Under load, the dedup check adds enough latency to trigger the 50ms timeout on every event.

**Prevention:**
1. Use an embedding cache keyed by a hash of the input text. Many events have identical or near-identical text (session_start, session_end).
2. Pre-compute a text hash and check for exact duplicates BEFORE computing the embedding. Exact text match is O(1) and catches the most common case.
3. Increase the dedup timeout to 200ms (the embedding itself takes ~5-50ms; the HNSW search takes ~1ms). The 50ms default for novelty was set conservatively.
4. Consider batching: accumulate events for 100ms, then embed all at once. Candle supports batch inference which is faster per-event than sequential.
5. Skip dedup for event types that are never duplicated: `session_start`, `session_end`.

**Detection:** Metric: `skipped_timeout` count increasing. Ingest latency p99 increasing after dedup is enabled. Compare ingest throughput before and after.

**Phase to address:** Dedup implementation phase, performance tuning sub-task.

**Severity:** HIGH

---

### Pitfall 6: HNSW Search During Concurrent Write

**What goes wrong:** The dedup check reads from the HNSW index while the indexing pipeline writes to it. The `HnswIndex` is behind an `Arc<RwLock<HnswIndex>>`, so concurrent reads and writes are serialized. Under load, dedup searches block on indexing writes and vice versa.

**Why it happens:** The current design uses a single HNSW index instance shared between the `VectorIndexUpdater` (writer) and the search/teleport layer (reader). Adding dedup makes the ingest path a reader too. With `RwLock`, any write blocks ALL reads. If the indexing pipeline holds the write lock for a batch of 100 inserts, all dedup checks queue behind it.

**Codebase evidence:** `crates/memory-indexing/src/vector_updater.rs` takes `self.index.write()` for each vector insertion. `crates/memory-vector/src/hnsw.rs` uses `RwLock<Index>` internally.

**Consequences:** Dedup latency spikes to hundreds of milliseconds during index rebuilds. The fail-open timeout fires, and events are stored without dedup. During scheduled indexing jobs, dedup effectively does not work.

**Prevention:**
1. Use the in-memory "recent embeddings" buffer (from Pitfall 1) as the PRIMARY dedup source. It does not require the HNSW lock. Fall back to HNSW search only for cross-session dedup.
2. If HNSW search is needed for dedup: use `try_read()` with a fallback to the buffer. Never block the ingest path on the HNSW lock.
3. Consider a separate read-only HNSW snapshot for dedup queries (double the memory but zero contention). Refresh the snapshot periodically (every 60 seconds).
4. The usearch library supports concurrent reads natively -- the `RwLock` is defensive Rust wrapping. Investigate if `Arc<Index>` with `#[allow(clippy::readonly_write_lock)]` (already used in the codebase) can enable lock-free reads.

**Detection:** Dedup p99 latency > 50ms correlated with indexing pipeline activity. Monitor lock contention metrics.

**Phase to address:** Dedup implementation phase. Lock strategy should be decided during design.

**Severity:** HIGH

---

### Pitfall 7: Stale Filtering Interacts Poorly with Existing Ranking Layers

**What goes wrong:** The existing ranking stack has 3 layers: salience (write-time), usage decay (read-time), and novelty (ingest-time). Adding stale filtering creates a 4th ranking signal. The interaction between these signals is multiplicative: an event with moderate salience, low usage, some novelty penalty, AND a stale penalty can have its effective score crushed to near-zero even if it is the BEST answer to the query.

**Why it happens:** Each ranking signal was designed independently. Salience was designed at Phase 16. Usage decay at Phase 16. Novelty at Phase 16. Each assumes it is the primary discriminator. When stacked, they create a "score collapse" where most results have similar (low) scores, making ranking non-discriminative.

**Codebase evidence:** The `RetrievalExecutor` in `crates/memory-retrieval/src/executor.rs` uses `min_confidence: 0.3` as a hard cutoff. If stale filtering pushes a result from 0.4 to 0.25, it is silently dropped even though it was the best match.

**Consequences:** All retrieval modes return fewer results. The fallback chain fires more often (degrading to Agentic TOC search). Query quality appears to decrease after enabling stale filtering, even for queries where staleness is irrelevant.

**Prevention:**
1. Define a clear score composition formula BEFORE implementation. Example: `final_score = vector_similarity * salience_weight * recency_factor * usage_boost`. Each factor should have a defined range and purpose.
2. Stale penalty should be BOUNDED: never reduce a score by more than 30%. A stale penalty of `max(0.7, 1.0 - age_weeks * 0.02)` caps the downrank.
3. Test with the existing E2E test queries. Run the existing 29 E2E tests with stale filtering enabled and verify no regressions.
4. Add a `ranking_explanation` field to `SearchResult` that shows each factor's contribution. This already exists conceptually in the `ExecutionResult.explanation` field but needs per-result detail.

**Detection:** A/B metrics: compare result counts and top-score distributions with and without stale filtering. Alert if average result count drops > 20%.

**Phase to address:** Stale filtering implementation phase. Must be tested against the existing ranking stack before ship.

**Severity:** HIGH

---

### Pitfall 8: Dedup of TOC Nodes vs. Raw Events Confusion

**What goes wrong:** The vector index currently contains TOC nodes and grips, NOT raw events. The dedup check at ingest needs to compare against raw event embeddings, but the index does not contain them. Comparing an incoming event against TOC node embeddings will produce misleading similarity scores because TOC nodes are summaries, not individual events.

**Why it happens:** The `VectorIndexUpdater` in `crates/memory-indexing/src/vector_updater.rs` indexes TOC nodes (via `index_toc_node`) and grips (via `index_grip`). Raw events are NOT indexed -- the `process_entry` method for `OutboxAction::IndexEvent` tries to find a grip for the event and indexes that, or skips if no grip exists. This means the HNSW index is NOT a dedup-compatible data source for raw event comparison.

**Codebase evidence:** `VectorIndexUpdater::process_entry()` line 183-199 shows that `IndexEvent` actions look for grips, not raw event text. `find_grip_for_event()` currently returns `None` always (line 206: "Simplified lookup - return None for now").

**Consequences:** Dedup against the existing HNSW index would compare "implement JWT token validation" (incoming event) against "Day summary: authentication work, JWT implementation" (TOC node). The similarity would be moderate (~0.6-0.7) but not high enough to trigger dedup. The system fails to detect duplicates even when they exist.

**Prevention:**
1. Create a SEPARATE dedup index (or a separate metadata namespace within the existing HNSW) that indexes raw event text. This index exists solely for dedup purposes.
2. Alternatively, use the in-memory buffer approach (Pitfall 1) which operates on raw event embeddings by design.
3. Do NOT try to reuse the existing TOC/grip vector index for dedup. The granularity mismatch makes it unreliable.
4. If creating a separate dedup index: it can be smaller (lower capacity, lower ef_construction) since it only needs to answer "is there something very similar?" not "what are the top-k results?"

**Detection:** Dedup tests that ingest two identical events and assert the second is rejected. If both are stored, the dedup index is not seeing raw events.

**Phase to address:** Dedup design phase. This is an early architectural decision.

**Severity:** HIGH

---

## Minor Pitfalls

Mistakes that cause inconvenience but are fixable without rework.

---

### Pitfall 9: Embedding Dimension Mismatch After Model Change

**What goes wrong:** If the embedding model is ever swapped (e.g., from all-MiniLM-L6-v2 at 384-dim to a larger model), the dedup index becomes incompatible. The HNSW index cannot mix dimensions.

**Prevention:**
1. Store the model name and dimension in the dedup index metadata.
2. On startup, verify the configured model matches the index metadata. If mismatched, rebuild.
3. This is already partially handled -- `HnswConfig` has `dimension: 384` as default and `VectorIndexUpdater` checks `embedding.dimension() != self.config.dimension`.

**Phase to address:** Dedup implementation phase. Add model metadata to the dedup index header.

**Severity:** LOW

---

### Pitfall 10: Dedup Config Not Exposed via gRPC Admin API

**What goes wrong:** Operators cannot tune dedup threshold or check dedup metrics without restarting the daemon. The existing gRPC API has `GetRankingStatus` but no dedup-specific admin RPCs.

**Prevention:**
1. Add `GetDedupStatus` RPC that returns: enabled, threshold, recent_embeddings_buffer_size, rejected_count, total_checked.
2. Add `SetDedupThreshold` RPC for runtime tuning (write to config, no restart needed).
3. Expose dedup metrics in the existing status/health endpoint.

**Phase to address:** Admin API phase after dedup is working.

**Severity:** LOW

---

### Pitfall 11: Test Fixtures for Dedup Are Hard to Get Right

**What goes wrong:** E2E tests for dedup need to ingest events that are "similar enough" to trigger dedup but "different enough" to test edge cases. Hand-crafting these is error-prone because the similarity score depends on the embedding model's learned representation, not on human intuition.

**Prevention:**
1. Create a calibration test that embeds a known set of text pairs and records their similarity scores. This becomes the ground truth for threshold selection.
2. Include these pairs in the test fixtures:
   - Identical text: score ~1.0 (should always dedup)
   - Same text with typo: score ~0.95 (should dedup)
   - Same topic, different phrasing: score ~0.75-0.85 (threshold-dependent)
   - Related topics: score ~0.55-0.70 (should NOT dedup)
   - Unrelated: score ~0.20-0.40 (should NOT dedup)
3. Run the calibration test as part of CI. If the model or tokenizer changes, the calibration catches it.

**Phase to address:** Dedup testing phase.

**Severity:** LOW

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Dedup Design | Index gap for recent events (Pitfall 1) | In-memory ring buffer for recent embeddings |
| Dedup Design | Raw events not in HNSW index (Pitfall 8) | Separate dedup index or buffer, not reuse TOC index |
| Dedup Design | Append-only invariant broken (Pitfall 3) | Store events, skip outbox entry; don't drop events |
| Dedup Implementation | Threshold miscalibration (Pitfall 2) | Default 0.85, dry-run mode, per-event-type thresholds |
| Dedup Implementation | Embedding latency on hot path (Pitfall 5) | Text hash pre-check, embedding cache, skip structural events |
| Dedup Implementation | HNSW lock contention (Pitfall 6) | try_read() with buffer fallback, never block ingest |
| Stale Filtering Design | Historical context buried (Pitfall 4) | Exempt Constraint/Definition/Procedure kinds from staleness |
| Stale Filtering Implementation | Ranking score collapse (Pitfall 7) | Bounded penalty (max 30% reduction), test against existing E2E |
| Testing | Fixture calibration (Pitfall 11) | Pre-computed similarity pairs as ground truth |
| Admin/Observability | No runtime tuning (Pitfall 10) | GetDedupStatus + SetDedupThreshold RPCs |

---

## Integration Pitfalls (Adding to Existing System)

These pitfalls are specific to adding dedup and stale filtering on top of the existing Agent Memory v2.4 architecture.

### Dedup + Novelty Double-Filtering

**What goes wrong:** The existing `NoveltyChecker` already does similarity-based filtering at ingest time. Adding dedup creates TWO similarity checks on the ingest path with potentially different thresholds. Events may pass one filter but fail the other, or the two filters may interact unpredictably.

**Prevention:**
- Dedup REPLACES novelty filtering. They solve the same problem. Remove `NoveltyChecker` or refactor it into the dedup system.
- If both are kept: dedup should run FIRST (it is cheaper with the buffer), and novelty should only run if dedup says "not a duplicate."
- Unify the config: one `DedupConfig` with `threshold`, `timeout_ms`, `min_text_length`, replacing `NoveltyConfig`.

### Dedup + TOC Segmentation Interaction

**What goes wrong:** Segment boundaries depend on event count and time gaps. If dedup drops events, segments become larger and less navigable. The existing TOC quality that users rely on degrades.

**Prevention:**
- Use the "store event, skip indexing" approach (Pitfall 3) so event counts are preserved for segmentation.
- If dropping events: adjust segment thresholds proportionally. But this is fragile and not recommended.

### Stale Filtering + Existing Fallback Chain

**What goes wrong:** The `RetrievalExecutor` uses a fallback chain that tries layers sequentially until results meet `min_confidence`. Stale filtering reduces scores, which triggers more fallbacks. The system degrades to Agentic TOC search more often, which is slower and less precise.

**Prevention:**
- Apply stale filtering AFTER the fallback chain resolves, not within individual layer results. This preserves the fallback logic's confidence thresholds.
- Alternatively: raise `min_confidence` check to account for the expected stale penalty range.

### Backward Compatibility of Config

**What goes wrong:** Existing config files have `[novelty]` section. Adding `[dedup]` is fine, but removing `[novelty]` breaks existing deployments.

**Prevention:**
- Keep `[novelty]` as a deprecated alias for `[dedup]` using `serde(alias = "novelty")`.
- Log a deprecation warning on startup if `[novelty]` is used.

---

## Sources

### Model-Specific Threshold Research
- [all-MiniLM-L6-v2 Model Card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) - Model capabilities, training data, cosine similarity behavior
- [AI-Driven Semantic Similarity Pipeline (2025)](https://arxiv.org/html/2509.15292v1) - Threshold calibration at 0.659 for literature dedup, score distribution [0.07, 0.80]
- [all-MiniLM-L6-v2 Similarity Discussion](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/discussions/16) - Community discussion of similarity thresholds

### Vector Dedup Architecture
- [OpenSearch Vector Dedup RFC](https://github.com/opensearch-project/k-NN/issues/2795) - 22% indexing speedup, 66% size reduction from dedup
- [pgvector HNSW Dedup Issue](https://github.com/pgvector/pgvector/issues/760) - HNSW index not used when combining dedup with distance ordering
- [Qdrant Vector Search in Production](https://qdrant.tech/articles/vector-search-production/) - Production patterns for vector dedup

### Stale Filtering and Retrieval Quality
- [8 Common Mistakes in Vector Search](https://kx.com/blog/8-common-mistakes-in-vector-search/) - Ignoring normalization, relying on defaults
- [23 RAG Pitfalls](https://www.nb-data.com/p/23-rag-pitfalls-and-how-to-fix-them) - Metadata and recency signal pitfalls
- [Azure Databricks Vector Search Quality Guide](https://learn.microsoft.com/en-us/azure/databricks/vector-search/vector-search-retrieval-quality) - Retrieval quality best practices
- [Vespa: Vector Search Reaching Its Limit](https://blog.vespa.ai/vector-search-is-reaching-its-limit/) - Beyond pure vector similarity

### Event Sourcing Dedup Patterns
- [Event Sourcing Projection Deduplication](https://domaincentric.net/blog/event-sourcing-projection-patterns-deduplication-strategies) - At-least-once delivery and idempotency
- [Idempotent Command Handling](https://event-driven.io/en/idempotent_command_handling/) - Race conditions in event stores
- [Event Deduplication in Batch and Stream Processing](https://www.upsolver.com/blog/how-to-deduplicate-events-in-batch-and-stream-processing-using-primary-keys) - Primary key dedup patterns

### Codebase References
- `crates/memory-service/src/novelty.rs` - Existing novelty checker with fail-open design, timeout handling, metrics
- `crates/memory-indexing/src/pipeline.rs` - Outbox-driven async indexing pipeline with checkpoint recovery
- `crates/memory-indexing/src/vector_updater.rs` - Vector index updater (indexes TOC nodes and grips, NOT raw events)
- `crates/memory-vector/src/hnsw.rs` - HNSW index wrapper with RwLock, cosine distance, usearch backend
- `crates/memory-types/src/config.rs` - NoveltyConfig with threshold 0.82, timeout 50ms
- `crates/memory-types/src/salience.rs` - Salience scoring with MemoryKind classification
- `crates/memory-retrieval/src/executor.rs` - Fallback chain execution with min_confidence threshold
- `crates/memory-types/src/outbox.rs` - Outbox entry types (IndexEvent, UpdateToc)
