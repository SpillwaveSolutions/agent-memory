# Feature Landscape

**Domain:** Semantic deduplication and retrieval quality for agent conversation memory
**Researched:** 2026-03-05

## Table Stakes

Features users expect from a dedup/stale-filtering system. Missing = the feature feels incomplete or broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Ingest-time vector similarity gate | Core dedup mechanism. Without it, repeated agent conversations fill the index with near-identical content, degrading retrieval quality. | Medium | Existing `NoveltyChecker` in `memory-service/src/novelty.rs` already implements the pattern (embed -> search top-1 -> threshold check). Must be wired into the actual ingest pipeline rather than being a standalone checker. |
| Configurable similarity threshold | Different projects have different repetition patterns. A code-heavy project tolerates lower thresholds than a conversational one. | Low | `NoveltyConfig.threshold` already exists (default 0.82). Expose through config.toml. Threshold is domain-specific; 0.80-0.90 is the practical range per community evidence. |
| Fail-open on dedup errors | Dedup must never block ingestion. If embedder is down, index not ready, or timeout hit, store the event anyway. | Low | Already implemented in `NoveltyChecker::should_store()` with full fail-open semantics (6 skip paths). This is validated design. |
| Temporal decay in ranking | Old results about superseded topics must rank lower than recent ones. Without this, stale answers pollute retrieval. | Medium | `VectorEntry` already stores `timestamp_millis`. Layer 6 ranking has `salience` and `usage_penalty` but no time-decay factor yet. Add exponential decay based on document age. |
| Dedup metrics/observability | Operators need to know how many events were deduplicated vs stored, to tune thresholds. | Low | `NoveltyMetrics` already tracks `rejected_duplicate`, `stored_novel`, and 6 skip categories. Expose via gRPC `GetDedupStats` or similar. |
| Minimum text length bypass | Short events (session_start, tool_result status lines) should skip dedup entirely -- they are structurally important but semantically thin. | Low | `NoveltyConfig.min_text_length` already exists (default 50 chars). Already implemented. |

## Differentiators

Features that set the dedup system apart from naive implementations. Not expected, but add significant value.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Supersession detection (content-aware staleness) | Instead of just time-decay, detect when a newer event semantically supersedes an older one on the same topic. Mark the older result as superseded. Goes beyond dumb temporal decay. | High | Requires comparing new ingest against existing similar entries and marking old entries with a `superseded_by` reference. Could use the same vector search but with a "supersession window" (e.g., only check events from same agent/session). |
| Per-event-type dedup policies | Different event types warrant different dedup behavior: `user_message` should be aggressively deduped, `session_start`/`session_end` should never be deduped, `assistant_stop` may have a looser threshold. | Low | Add `event_type` to the dedup decision. Simple match on `EventType` enum to select threshold or skip. |
| Staleness half-life configuration | Configurable half-life for temporal decay (e.g., 7 days, 30 days) rather than a fixed decay curve. Projects with fast-moving topics want aggressive decay; archival projects want gentle decay. | Low | Single `half_life_days` config parameter. Decay formula: `score * exp(-ln(2) * age_days / half_life_days)`. |
| Agent-scoped dedup | Dedup within a single agent's history, not across all agents. Agent A saying "let's fix the bug" and Agent B saying the same thing are independent events worth keeping. | Medium | Already have `Event.agent` field. Scope the vector similarity search with an agent filter. Requires post-filtering HNSW results by agent metadata since usearch has no native metadata filtering. |
| Dedup dry-run mode | Allow operators to see what WOULD be deduped without actually dropping events. Useful for threshold tuning. | Low | Add `dry_run` flag to `NoveltyConfig`. Log rejections but store anyway. Return dedup decisions in metrics. |
| Stale result exclusion window | Hard cutoff: results older than N days are excluded from retrieval entirely (not just downranked). Configurable per intent type -- `TimeBoxed` queries might exclude results older than 7 days while `Explore` queries include everything. | Medium | Add `max_age_days` to retrieval config per `QueryIntent`. Filter at query time before ranking. |

## Anti-Features

Features to explicitly NOT build. These seem tempting but create more problems than they solve.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Mutable event deletion on dedup | Tempting to delete duplicate events from RocksDB. Violates the append-only invariant that is foundational to the architecture. Deleted events break grip references, TOC nodes, and crash recovery checkpoints. | Mark duplicates silently by not storing them at ingest time. Already-stored events stay forever. |
| Cross-project dedup | Comparing events across different project stores adds massive complexity and violates the per-project isolation model. | Keep dedup scoped to a single project store. Cross-project memory is explicitly deferred/out-of-scope. |
| LLM-based dedup decisions | Using an LLM to decide if two events are duplicates (like Mem0 does) adds API latency, cost, and a hard dependency on external services. Agent Memory uses local embeddings precisely to avoid API dependencies. | Use local vector similarity (all-MiniLM-L6-v2 via Candle, already in-process). The 50ms timeout is achievable with local embeddings but not with API calls. |
| Exact-match dedup only | Hashing-based exact dedup catches identical text but misses semantic near-duplicates ("let's fix the auth bug" vs "we need to address the authentication issue"). | Semantic similarity via embeddings catches both exact and near-duplicate content. Hash-based dedup is a subset of vector similarity at threshold=1.0. |
| Global re-ranking of all stored events | Re-ranking everything at query time based on staleness is O(n) and defeats the purpose of indexed search. | Apply staleness filtering/decay AFTER index search returns top-k candidates. Post-retrieval filtering keeps cost at O(k). |
| Retroactive dedup of existing events | Scanning all historical events to find and mark duplicates is expensive and risks flagging legitimate repeated discussions. | Apply dedup only to new events going forward. Historical data stays as-is. |

## Feature Dependencies

```
NoveltyChecker wired to ingest pipeline
    -> Configurable threshold (already exists in NoveltyConfig)
    -> Per-event-type policies (extends NoveltyChecker)
    -> Agent-scoped dedup (extends vector search with agent filter)
    -> Dedup dry-run mode (extends NoveltyChecker)
    -> Dedup metrics exposed via gRPC (extends existing NoveltyMetrics)

Temporal decay in ranking
    -> Staleness half-life config (extends ranking config)
    -> Stale result exclusion window (extends retrieval executor)
    -> Supersession detection (extends ingest + retrieval)

Vector similarity search at ingest (already exists: HnswIndex.search)
    -> NoveltyChecker integration (already partially built)
    -> Agent-scoped search filtering (needs metadata filter)
```

## MVP Recommendation

Prioritize:

1. **Wire NoveltyChecker into actual ingest pipeline** -- The checker exists but is not connected to the real ingest path. This is the single highest-value change: it immediately reduces noise in the vector/BM25 indexes.

2. **Temporal decay factor in Layer 6 ranking** -- Add time-based decay alongside existing salience and usage_penalty scores. Formula: `decay = exp(-ln(2) * age_days / half_life_days)`, default half-life 14 days. Apply as a multiplier on retrieval scores post-search.

3. **Per-event-type dedup bypass** -- Skip dedup for structural events (session_start, session_end, subagent_start, subagent_stop). Only dedup content-bearing events (user_message, assistant_stop, tool_result).

4. **Expose dedup metrics via gRPC** -- Wire existing `NoveltyMetrics` into a status RPC so operators can monitor dedup effectiveness and tune thresholds.

5. **E2E tests proving dedup works** -- Ingest duplicate events, verify only one is stored. Query with temporal decay, verify recent results rank higher.

Defer:
- **Supersession detection**: High complexity, requires topic-matching infrastructure beyond simple vector similarity. Research deeper in a future phase.
- **Agent-scoped dedup**: Requires post-filtering HNSW results by agent metadata since usearch has no native metadata filtering. Feasible but adds complexity. Defer until multi-agent dedup is a validated pain point.
- **Stale result exclusion window per intent**: Nice to have but temporal decay covers 80% of the use case. Add later if decay alone is insufficient.

## Existing Infrastructure to Leverage

| Component | Location | What It Provides | What's Missing |
|-----------|----------|-----------------|----------------|
| `NoveltyChecker` | `memory-service/src/novelty.rs` | Full fail-open dedup logic with embed -> search -> threshold | Not wired into actual ingest pipeline |
| `NoveltyConfig` | `memory-types/src/config.rs` | `enabled`, `threshold` (0.82), `timeout_ms` (50), `min_text_length` (50) | No per-event-type policies |
| `NoveltyMetrics` | `memory-service/src/novelty.rs` | Atomic counters for all dedup outcomes | Not exposed via gRPC |
| `VectorEntry.timestamp_millis` | `memory-vector/src/index.rs` | Timestamp on every indexed document | Not used in ranking |
| `SalienceScorer` | `memory-types/src/salience.rs` | Write-time salience calculation | No temporal component |
| `usage_penalty()` | `memory-types/src/usage.rs` | Access-count based decay formula | No time-based decay |
| `HnswIndex` | `memory-vector/src/hnsw.rs` | Cosine similarity search via usearch | No metadata filtering for agent-scoped search |
| `IndexingPipeline` | `memory-indexing/src/pipeline.rs` | Outbox-driven batch indexing | Dedup check not part of pipeline |
| `VectorIndexUpdater` | `memory-indexing/src/vector_updater.rs` | Embeds and indexes TOC nodes and grips | Already skips duplicates by doc_id (exact match only) |

## Sources

- [Mem0: Building Production-Ready AI Agents with Scalable Long-Term Memory](https://arxiv.org/abs/2504.19413) -- Mem0 uses LLM-based memory extraction and dedup; we deliberately avoid this for latency reasons (MEDIUM confidence)
- [Temporal RAG: Why RAG Always Gets 'When' Questions Wrong](https://blog.sotaaz.com/post/temporal-rag-en) -- Temporal awareness critical for retrieval freshness (MEDIUM confidence)
- [Data Freshness Rot as the Silent Failure Mode in Production RAG Systems](https://glenrhodes.com/data-freshness-rot-as-the-silent-failure-mode-in-production-rag-systems-and-treating-document-shelf-life-as-a-first-class-reliability-concern-2/) -- Treats document shelf life as first-class concern (MEDIUM confidence)
- [Solving Freshness in RAG: A Simple Recency Prior](https://arxiv.org/html/2509.19376) -- Recency prior fused with semantic similarity for temporal ranking (MEDIUM confidence)
- [OpenAI Community: Rule of Thumb Cosine Similarity Thresholds](https://community.openai.com/t/rule-of-thumb-cosine-similarity-thresholds/693670) -- No universal threshold; 0.79-0.85 common for near-duplicate detection (MEDIUM confidence)
- [Data Deduplication at Trillion Scale](https://zilliz.com/blog/data-deduplication-at-trillion-scale-solve-the-biggest-bottleneck-of-llm-training) -- MinHash LSH at 0.8 threshold for near-duplicate detection at scale (MEDIUM confidence)
- [Enhancing RAG: A Study of Best Practices](https://arxiv.org/abs/2501.07391) -- RAG best practices including dedup in context assembly (HIGH confidence)
- [The Knowledge Decay Problem](https://ragaboutit.com/the-knowledge-decay-problem-how-to-build-rag-systems-that-stay-fresh-at-scale/) -- Staleness monitoring as ongoing operational concern (MEDIUM confidence)
- Existing codebase: `NoveltyChecker`, `NoveltyConfig`, `NoveltyMetrics`, `SalienceScorer`, `usage_penalty()`, `VectorEntry`, `HnswIndex` (HIGH confidence -- direct code inspection)
