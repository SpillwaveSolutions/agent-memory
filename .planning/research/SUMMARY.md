# Project Research Summary

**Project:** Agent Memory v2.5 — Semantic Dedup & Retrieval Quality
**Domain:** Ingest-time semantic deduplication and stale result filtering for append-only event store
**Researched:** 2026-03-05
**Confidence:** HIGH

## Executive Summary

Agent Memory v2.5 adds two capabilities: ingest-time semantic deduplication to prevent near-identical events from polluting the vector and BM25 indexes, and stale result filtering to downrank superseded content at query time. All four research streams confirm that no new Rust crate dependencies are required — usearch 2.23.0, Candle 0.8.4, RocksDB 0.22, and chrono 0.4 already provide everything needed. The codebase already contains a largely-complete `NoveltyChecker` in `memory-service/src/novelty.rs` that implements the correct fail-open, opt-in, metric-tracked pattern — the primary work is wiring it to real infrastructure and resolving four critical design decisions identified by the pitfalls researcher.

The hardest problem is not the feature implementation itself but the architectural constraints that must be resolved first. The PITFALLS researcher identified four critical issues that contradict the naive implementation: (1) the HNSW index contains TOC nodes and grips, NOT raw events, so comparing incoming events against it produces misleading similarity scores; (2) the async outbox pipeline creates a timing gap where burst duplicates (the most common kind) escape detection entirely; (3) ingest-time event dropping breaks the append-only invariant that TOC segmentation depends on; and (4) stale filtering stacks multiplicatively with existing ranking penalties, risking score collapse on high-salience historical content. Each of these requires a design decision before implementation begins.

The recommended approach addresses all four: use a two-tier dedup system (in-memory in-flight buffer of 384-dim embeddings as primary, HNSW as secondary for cross-session), store-and-skip-indexing instead of dropping events to preserve append-only semantics, set a conservative default threshold of 0.85 with dry-run mode for calibration, and exempt high-salience memory kinds (Constraint, Definition, Procedure) from stale filtering entirely. The architecture researcher and pitfalls researcher are in full agreement on this approach, and the STACK researcher confirms no new dependencies are needed to implement it.

## Key Findings

### Recommended Stack

No new dependencies. The entire milestone is implemented using existing crates — nothing in `Cargo.toml` changes. See `.planning/research/STACK.md` for full detail.

**Core technologies:**
- **usearch 2.23.0** (`memory-vector`): HNSW index for cross-session dedup similarity search — already has `search()` and `add()`, already instantiated in the service
- **Candle 0.8.4** (`memory-embeddings`): all-MiniLM-L6-v2 local embedding generation — already wrapped in `CandleEmbedder`, already generates 384-dim vectors; no external API dependency
- **RocksDB 0.22** (`memory-storage`): dedup metadata storage, staleness markers, existing `VectorEntry.created_at` timestamps cover all staleness needs
- **chrono 0.4** (`memory-types`): timestamp comparison for staleness decay — already used throughout
- **tokio 1.43** (`memory-service`): async timeout for dedup gate (fail-open on timeout) — already used in `NoveltyChecker`

**What NOT to add:** SimHash/MinHash crates, Bloom filter crates, external embedding services, separate time-series databases for staleness, ordered-float crate — all are overkill for the 384-dim cosine similarity + exponential decay approach.

### Expected Features

See `.planning/research/FEATURES.md` for full detail with dependency graph.

**Must have (table stakes):**
- **Ingest-time vector similarity gate** — core dedup; without it, repeated agent conversations fill indexes with near-identical content degrading retrieval quality. `NoveltyChecker` pattern exists, needs wiring.
- **Configurable similarity threshold** — different projects have different repetition patterns; `NoveltyConfig.threshold` already exists (default 0.82, should be raised to 0.85 for dedup).
- **Fail-open on dedup errors** — dedup must never block ingestion; already implemented in `NoveltyChecker::should_store()` with 6 skip paths.
- **Temporal decay in ranking** — old results about superseded topics must rank lower; `VectorEntry` already stores `timestamp_millis`.
- **Dedup metrics/observability** — operators need to know how many events were deduplicated to tune thresholds; `NoveltyMetrics` already tracks the right counters, needs gRPC exposure.
- **Minimum text length bypass** — short events (session_start, tool_result status lines) skip dedup entirely; `NoveltyConfig.min_text_length` already exists.

**Should have (differentiators):**
- **Supersession detection** — mark older events semantically replaced by newer content on same topic (goes beyond time decay); high complexity, architecture researcher provides concrete design.
- **Per-event-type dedup policies** — `session_start`/`session_end` never deduped, `user_message`/`assistant_stop` deduped with higher threshold; low complexity, high value.
- **Staleness half-life configuration** — configurable `half_life_days` for exponential decay rather than fixed curve.
- **Dedup dry-run mode** — log what WOULD be rejected without dropping events; critical for threshold tuning before production enable.
- **Agent-scoped dedup** — dedup within single agent's history, not across agents; requires post-filtering HNSW results by agent metadata.

**Defer to v2.6+:**
- **Agent-scoped dedup**: requires post-filtering HNSW results by agent metadata since usearch has no native metadata filtering — feasible but adds complexity; defer until multi-agent dedup is a validated pain point.
- **Stale result exclusion window per intent**: temporal decay covers 80% of the use case; add hard cutoff by `QueryIntent` only if decay alone proves insufficient.

**Anti-features (explicitly excluded):**
- Mutable event deletion on dedup — violates append-only invariant; mark by not indexing, never by deleting.
- LLM-based dedup decisions — adds API latency, cost, external dependency; use local Candle embeddings.
- Exact-match dedup only — misses semantic near-duplicates; use vector similarity.
- Global re-ranking of all stored events — O(n) at query time; apply staleness to top-k only.
- Retroactive dedup of historical events — expensive, risky; new events only going forward.
- Cross-project dedup — violates per-project isolation model.

### Architecture Approach

The architecture is an enhancement of existing patterns, not a new system. The `NoveltyChecker` in `memory-service/src/novelty.rs` IS the dedup gate — it already implements fail-open, opt-in, metric-rich semantics. Two new components are added alongside it: an `InFlightBuffer` (in-memory ring buffer of recent embeddings) and a `StaleFilter` (post-retrieval ranking adjustment). All three components follow the same four architectural patterns: fail-open gate, opt-in with sensible defaults, metric-rich observability, and trait-based abstractions for testability. See `.planning/research/ARCHITECTURE.md` for complete component designs with Rust structs and proto definitions.

**Major components:**
1. **DedupGate (enhanced NoveltyChecker)** (`memory-service/src/novelty.rs`) — rejects semantically duplicate events at ingest; two-tier check: InFlightBuffer first (O(n) linear scan on bounded set), then HNSW index (O(log n) for cross-session); wraps both in the existing timeout/fail-open wrapper
2. **InFlightBuffer** (`memory-service`, internal to DedupGate) — `VecDeque<InFlightEntry>` with max_size (256) and max_age (5 min) eviction; stores raw event embeddings for the timing gap window; ~400KB memory footprint; volatile (lost on restart, acceptable by design)
3. **StaleFilter** (`memory-service/src/stale.rs` or integrated into `memory-retrieval`) — post-retrieval, pre-return; applies exponential time decay and pairwise supersession detection on top-k results only (never O(n)); exempts Constraint/Definition/Procedure memory kinds
4. **DedupConfig / StaleConfig** (`memory-types/src/config.rs`) — extends existing `NoveltyConfig`; `[novelty]` kept as deprecated alias for backward compatibility via `serde(alias)`
5. **DedupMetrics** (extended `NoveltyMetrics`) — adds buffer hit rate, HNSW fallback rate; exposed via new `GetDedupStatus` gRPC RPC

**Data flow changes:**

```
Write path (BEFORE): IngestEvent -> validate -> serialize -> storage.put_event -> return
Write path (AFTER):  IngestEvent -> validate -> serialize -> DedupGate.should_store()
                                                               -> embed (CandleEmbedder)
                                                               -> check InFlightBuffer (linear)
                                                               -> check HNSW (if not in buffer)
                                                               -> if novel: add to buffer, STORE
                                                               -> if dup: SKIP indexing only*
                                                             -> if STORE: storage.put_event -> return {created: true}
                                                             -> if SKIP: store event (append-only!), skip outbox* -> return {created: false, deduplicated: true}

Read path (BEFORE):  RouteQuery -> classify -> execute layers -> merge -> return
Read path (AFTER):   RouteQuery -> classify -> execute layers -> merge -> StaleFilter.apply() -> return
```

*See Pitfall 3: "store event, skip outbox" preserves the append-only invariant for TOC segmentation.

### Critical Pitfalls

The PITFALLS researcher identified 4 critical, 4 high-severity, and 3 minor pitfalls. See `.planning/research/PITFALLS.md` for full analysis with codebase evidence and detection guidance.

**Top 5 by severity:**

1. **HNSW index contains TOC nodes/grips, NOT raw events (Pitfall 8)** — Reusing the existing HNSW index for dedup compares incoming events to summaries, producing misleading similarity scores (~0.6-0.7 instead of 0.85+). Comparing "implement JWT token validation" (event) vs "Day summary: authentication work" (TOC node) will NOT catch the duplicate. **Prevention:** The InFlightBuffer (which stores raw event embeddings by design) is the primary dedup source; the HNSW index is a secondary fallback for cross-session only. Do NOT attempt to reuse the TOC/grip index for dedup at raw event granularity.

2. **Timing gap: burst duplicates escape detection (Pitfall 1)** — The outbox pipeline is async; events ingested in rapid succession cannot see each other in the HNSW index. Within-session duplicates (the most common kind) are exactly what the current design misses. **Prevention:** InFlightBuffer catches these — it holds raw embeddings for the last N events with a TTL covering the maximum expected indexing lag. Size 256 entries x 5min TTL covers typical session bursts.

3. **Dedup drops break the append-only invariant (Pitfall 3)** — Dropping events at ingest changes event counts, breaking TOC segment boundaries, causing segments to cover longer time spans, and potentially omitting discussed topics from day summaries. **Prevention:** Store ALL events; for dedup duplicates, store the event to RocksDB but do NOT create an outbox entry (so it is never indexed into HNSW or BM25). Event count is preserved for segmentation; index quality is preserved by not indexing duplicates. This is a critical design decision that must be made before implementation.

4. **Stale filtering hides critical historical context (Pitfall 4)** — Conversational memory is not a news feed; old context is frequently the most important. An agent asking "what was the authentication approach we decided on?" needs the ORIGINAL decision (old, high-salience), not the latest passing mention (new, low-salience). Stale filtering stacked with existing salience + usage_decay can bury the right answer below the `min_confidence` threshold. **Prevention:** Exempt `Constraint`, `Definition`, and `Procedure` memory kinds from staleness penalties entirely; cap maximum stale penalty at 30% score reduction; apply stale filtering AFTER the fallback chain resolves (not within individual layer results).

5. **Threshold miscalibration for all-MiniLM-L6-v2 (Pitfall 2)** — The model's cosine similarity distribution is non-intuitive: unrelated content scores 0.20-0.40, near-duplicates 0.75-0.85, verbatim duplicates 0.85+. The existing `NoveltyConfig` default of 0.82 was set for novelty detection (a different use case); for dedup the consequences of false positives are IRREVERSIBLE (event never stored). **Prevention:** Default threshold 0.85 for dedup; mandatory dry-run mode for first week; per-event-type thresholds; compound check (cosine + Jaccard token overlap) to reduce false positives.

**Additional high-severity pitfalls:**
- **Embedding latency on hot path (Pitfall 5)**: Candle runs synchronously; on CI Linux or older hardware, embedding takes 20-50ms. Prevention: text hash pre-check for exact duplicates before computing embedding; embedding cache; increase timeout to 200ms; skip structural events.
- **HNSW RwLock contention (Pitfall 6)**: Indexing pipeline holds write lock while dedup reads; under load, dedup times out during indexing runs. Prevention: use `try_read()` with buffer fallback; never block ingest path on HNSW lock.
- **Stale filtering interacts poorly with ranking layers (Pitfall 7)**: Score collapse when stale penalty stacks with salience + usage_decay + novelty. Prevention: bounded penalty (max 30%), test against existing 29 E2E queries before ship.
- **Dedup + Novelty double-filtering**: Two similarity checks on ingest path with different thresholds create unpredictable interaction. Prevention: dedup REPLACES novelty filtering; unify into single `DedupConfig`; keep `[novelty]` as deprecated alias.

## Implications for Roadmap

Based on combined research, the implementation should follow a dependency-aware 4-phase structure. The dedup work (write path, higher risk) comes before stale filtering (read path, lower risk). Design decisions must precede implementation to avoid the critical pitfalls.

### Phase 1: DedupGate Foundation

**Rationale:** Pure data structures and enhanced checker can be fully unit-tested before touching the ingest path. The InFlightBuffer and enhanced NoveltyChecker are the riskiest new code (they define correctness); isolate them for thorough testing.

**Delivers:** InFlightBuffer data structure; enhanced NoveltyChecker wired to real `CandleEmbedder` and `HnswIndex`; DedupConfig in memory-types; unit tests with MockEmbedder + MockVectorIndex.

**Addresses (from FEATURES.md):** Ingest-time vector similarity gate (table stakes), fail-open behavior (table stakes), configurable threshold (table stakes), minimum text length bypass (table stakes).

**Avoids (from PITFALLS.md):** Timing gap (Pitfall 1) via InFlightBuffer; TOC/grip index reuse (Pitfall 8) by using buffer as primary source; threshold miscalibration (Pitfall 2) by implementing dry-run mode.

**Needs research:** Threshold calibration for all-MiniLM-L6-v2 — need calibration test fixture with known similarity pairs covering identical, near-duplicate, related, and unrelated text pairs.

### Phase 2: Wire DedupGate into Ingest Path

**Rationale:** Depends on Phase 1 being solid. Changes the write path (higher risk than read path). Proto changes and integration tests required. Fail-open design ensures backward compatibility on any failure.

**Delivers:** DedupGate injected into `MemoryServiceImpl`; store-event-skip-outbox behavior for duplicates (preserving append-only invariant); proto additions (`IngestEventResponse.deduplicated`, `GetDedupStatus` RPC, field numbers 201+); integration tests proving dedup catches burst duplicates.

**Addresses (from FEATURES.md):** Dedup metrics/observability via gRPC (table stakes), per-event-type dedup bypass (differentiator), dedup dry-run mode (differentiator).

**Avoids (from PITFALLS.md):** Append-only invariant break (Pitfall 3) via store-event-skip-outbox design; HNSW RwLock contention (Pitfall 6) via try_read() + buffer fallback; embedding latency (Pitfall 5) via text hash pre-check and skip for structural events; dedup+novelty double-filtering via unified DedupConfig replacing NoveltyConfig.

**Standard patterns:** Wiring pattern is straightforward given Phase 1 foundation; unlikely to need deeper research.

### Phase 3: StaleFilter

**Rationale:** Read-path only — no data mutation concerns. Can be built/tested in parallel with Phase 2 if resources allow. Depends on having retrieval infrastructure in place (which predates v2.5).

**Delivers:** `StaleFilter` component in memory-service or memory-retrieval; `StalenessConfig` in memory-types (alongside `NoveltyConfig`); exponential time-decay factor applied post-retrieval on top-k results; pairwise supersession detection (O(k^2) bounded, k<=20); Constraint/Definition/Procedure kind exemptions; bounded penalty (max 30% reduction).

**Addresses (from FEATURES.md):** Temporal decay in ranking (table stakes), staleness half-life configuration (differentiator), stale result exclusion window (differentiator, partial).

**Avoids (from PITFALLS.md):** Historical context buried (Pitfall 4) via kind exemptions and bounded penalty; ranking score collapse (Pitfall 7) via bounded penalty and post-fallback-chain application; O(n^2) comparison (Architecture anti-pattern) by bounding to top-k.

**May need research:** Interaction between stale filtering and existing min_confidence threshold — run against existing 29 E2E queries to verify no regressions before finalizing score formula.

### Phase 4: E2E Validation and Observability

**Rationale:** Validates both features working end-to-end through the real pipeline. CLI bats tests provide regression coverage. Standard E2E patterns.

**Delivers:** E2E tests for duplicate event rejection, near-duplicate rejection, stale result downranking, fail-open on embedder failure, fail-open on timeout; CLI bats tests for dedup behavior; `GetDedupStatus` and `SetDedupThreshold` gRPC admin RPCs for runtime tuning.

**Addresses (from FEATURES.md):** E2E proof that dedup works (table stakes), dedup metrics exposed via gRPC (table stakes).

**Avoids (from PITFALLS.md):** Test fixture calibration problem (Pitfall 11) by building calibration test suite with pre-computed similarity pairs as ground truth; no runtime tuning gap (Pitfall 10) via admin RPCs; model version drift detection via model metadata in dedup index header.

**Standard patterns:** E2E test patterns well-established in this codebase (29 existing tests as reference); unlikely to need deeper research.

### Phase Ordering Rationale

- DedupGate foundation before wiring because the InFlightBuffer and trait adapters can be fully unit-tested in isolation — the highest-risk new code gets the most testing time before it touches the live ingest path.
- Ingest wiring before StaleFilter because write-path changes have higher risk than read-path changes; shipping dedup first also generates real dedup metrics to validate the approach.
- StaleFilter can proceed in parallel with Phase 2 if needed since they are independent subsystems (write path vs read path).
- E2E last because it validates both features working through the complete pipeline.
- Design decisions (append-only invariant, HNSW granularity, threshold defaults) must be recorded as architectural decisions before Phase 1 implementation begins — these cannot be retrofitted.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1 (DedupGate Foundation):** Threshold calibration for all-MiniLM-L6-v2 requires a calibration test that embeds known text pairs and records similarity distributions. Do not rely on intuition about similarity scores with this model.
- **Phase 3 (StaleFilter):** Score composition formula needs validation against existing 29 E2E tests. Run with stale filtering enabled and verify result count and top-score distributions show no regression before finalizing penalty bounds.

Phases with standard patterns (skip research-phase):
- **Phase 2 (Wire DedupGate into Ingest):** Straightforward wiring given Phase 1 foundation; proto extension pattern well-established (field numbers 201+).
- **Phase 4 (E2E Validation):** Standard bats + Rust E2E patterns; 29 existing tests provide strong reference.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Direct codebase inspection confirmed all existing crates sufficient; no new deps. Locked versions (usearch 2.23.0, candle 0.8.4, rocksdb 0.22) verified in Cargo.lock. |
| Features | HIGH | NoveltyChecker precedent validates the dedup pattern; stale filtering is standard ranking math. External sources (Mem0, temporal RAG research) provide corroboration. |
| Architecture | HIGH | In-flight buffer + HNSW dual-check is proven in vector DB literature. All 4 critical pitfalls have concrete prevention strategies based on direct code analysis. |
| Pitfalls | HIGH | All pitfalls verified with specific file paths and line references in the codebase. Model threshold distributions backed by published research on all-MiniLM-L6-v2. |

**Overall confidence:** HIGH

### Gaps to Address

These are unresolved questions that must be decided as architectural decisions at the start of Phase 1:

- **Threshold calibration**: Exact threshold values for all-MiniLM-L6-v2 dedup need a calibration test with known text pairs. Current recommendation (0.85 default) is conservative but not empirically validated against the specific event corpus. Build calibration fixture in Phase 1 before setting production defaults.

- **Append-only design decision**: "Store event, skip outbox" (PITFALLS recommendation) vs "drop at ingest" (STACK recommendation) need explicit resolution. The pitfalls researcher's analysis of TOC segmentation impact makes "store-and-skip-outbox" the recommended choice, but this is an architectural decision that affects Phase 1 design. Must be recorded in PROJECT.md before implementation.

- **HNSW lock contention strategy**: `try_read()` with buffer fallback vs periodic read-only HNSW snapshot. The in-flight buffer (Pitfall 6 prevention) is the primary defense, but the strategy for when try_read() fails needs explicit specification.

- **Score composition formula for stale filtering**: The exact weighting of `vector_similarity * salience_weight * recency_factor * usage_boost` needs to be defined before Phase 3 to avoid score collapse. The PITFALLS researcher recommends bounded penalty (max 30%), the ARCHITECTURE researcher suggests `superseded_penalty = 0.3` for explicitly superseded results. These must be reconciled with the existing min_confidence threshold of 0.3 in `RetrievalExecutor`.

- **Config backward compatibility**: `[novelty]` section in existing config.toml files must continue working. Use `serde(alias = "novelty")` on `DedupConfig`. Deprecation warning on startup when alias is used. This is a minor detail but must not be forgotten.

- **Per-event-type dedup exemptions**: session_start, session_end, subagent_start, subagent_stop should bypass dedup entirely (structural events). user_message and assistant_stop should be deduped with conservative threshold. tool_result is ambiguous — may need a moderate threshold since repeated tool calls ARE legitimate duplicates.

## Sources

### Primary (HIGH confidence — direct codebase inspection)
- `crates/memory-service/src/novelty.rs` — existing `NoveltyChecker` with `EmbedderTrait`, `VectorIndexTrait`, fail-open, metrics (6 skip categories), `NoveltyConfig` integration
- `crates/memory-service/src/ingest.rs` — `MemoryServiceImpl`, `IngestEvent` handler, `storage.put_event()` atomic write
- `crates/memory-indexing/src/pipeline.rs` — `IndexingPipeline`, `process_batch()`, outbox checkpoint tracking
- `crates/memory-indexing/src/vector_updater.rs` — `VectorIndexUpdater`, `find_grip_for_event()` returns None (critical: raw events NOT indexed), `index_toc_node()`, `index_grip()`
- `crates/memory-vector/src/hnsw.rs` — `HnswIndex`, `Arc<RwLock<Index>>`, `MetricKind::Cos`, `search()` returns 1.0-distance
- `crates/memory-vector/src/metadata.rs` — `VectorEntry.created_at` (ms since epoch), `VectorMetadata` RocksDB store
- `crates/memory-types/src/config.rs` — `NoveltyConfig` (threshold 0.82, timeout 50ms, disabled by default)
- `crates/memory-types/src/usage.rs` — `usage_penalty()`, `apply_usage_penalty()` (pattern for staleness functions)
- `crates/memory-types/src/salience.rs` — `SalienceScorer`, `MemoryKind` enum (Constraint, Definition, Procedure)
- `crates/memory-retrieval/src/executor.rs` — `RetrievalExecutor`, `min_confidence: 0.3`, fallback chain execution
- `crates/memory-retrieval/src/types.rs` — `QueryIntent`, `CapabilityTier`, `StopConditions`
- `Cargo.lock` — usearch 2.23.0, candle-core 0.8.4, tantivy 0.25.0, rocksdb 0.22 (versions locked)
- `.planning/PROJECT.md` — architectural decisions, requirements, constraints

### Secondary (MEDIUM confidence — published research and community)
- [Mem0: Building Production-Ready AI Agents](https://arxiv.org/abs/2504.19413) — LLM-based memory extraction and dedup (we deliberately avoid for latency reasons)
- [Temporal RAG: Why RAG Gets 'When' Questions Wrong](https://blog.sotaaz.com/post/temporal-rag-en) — temporal awareness critical for retrieval freshness
- [AI-Driven Semantic Similarity Pipeline (2025)](https://arxiv.org/html/2509.15292v1) — threshold calibration at 0.659 for literature dedup; score distribution [0.07, 0.80] for all-MiniLM-L6-v2
- [Solving Freshness in RAG: A Simple Recency Prior](https://arxiv.org/html/2509.19376) — recency prior fused with semantic similarity for temporal ranking
- [OpenAI Community: Cosine Similarity Thresholds](https://community.openai.com/t/rule-of-thumb-cosine-similarity-thresholds/693670) — no universal threshold; 0.79-0.85 common for near-duplicate detection
- [Data Deduplication at Trillion Scale](https://zilliz.com/blog/data-deduplication-at-trillion-scale-solve-the-biggest-bottleneck-of-llm-training) — MinHash LSH at 0.8 threshold for near-duplicate detection
- [Enhancing RAG: Best Practices](https://arxiv.org/abs/2501.07391) — dedup in context assembly best practices
- [Data Freshness Rot in Production RAG](https://glenrhodes.com/data-freshness-rot-as-the-silent-failure-mode-in-production-rag-systems-and-treating-document-shelf-life-as-a-first-class-reliability-concern-2/) — document shelf life as first-class reliability concern
- [OpenSearch Vector Dedup RFC](https://github.com/opensearch-project/k-NN/issues/2795) — 22% indexing speedup, 66% size reduction from dedup
- [Event Sourcing Projection Deduplication](https://domaincentric.net/blog/event-sourcing-projection-patterns-deduplication-strategies) — at-least-once delivery and idempotency patterns
- [8 Common Mistakes in Vector Search](https://kx.com/blog/8-common-mistakes-in-vector-search/) — normalization and default threshold pitfalls

### Tertiary (LOW confidence — needs validation)
- [all-MiniLM-L6-v2 Similarity Discussion](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/discussions/16) — community discussion of similarity thresholds; needs calibration test to validate against actual event corpus
- [pgvector HNSW Dedup Issue](https://github.com/pgvector/pgvector/issues/760) — HNSW index not used with combined dedup+distance ordering; usearch behavior may differ

---
*Research completed: 2026-03-05*
*Synthesized by: gsd-synthesizer from STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
*Ready for roadmap: yes*
