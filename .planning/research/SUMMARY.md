# Project Research Summary

**Project:** Agent Memory — v2.6 Episodic Memory, Ranking Quality, Lifecycle & Observability
**Domain:** Rust-based cognitive memory architecture for AI agents (gRPC service, 14-crate workspace)
**Researched:** 2026-03-11
**Confidence:** HIGH

## Executive Summary

Agent Memory v2.6 is a mature milestone adding four orthogonal capabilities to a production-proven 14-crate Rust system: episodic memory (task outcome recording and retrieval), ranking quality (salience + usage-based decay composition), lifecycle automation (scheduled vector/BM25/episode pruning), and observability RPCs (admin metrics for dedup, ranking, episodes). The system already has 7 shipped milestones (v1.0–v2.5), 48,282 LOC, 122 plans, and a complete 6-layer retrieval stack (TOC, agentic search, BM25, vector, topic graph, ranking). The critical architectural insight is that v2.6 requires zero new external dependencies — every new feature plugs into existing patterns (RocksDB column families, Tokio scheduler jobs, Arc<Storage> handler injection, proto field extensions) rather than introducing structural changes.

The recommended approach is additive integration in four phases (39–42). Phase 39 lays the episodic storage foundation (CF_EPISODES column family + proto schema), Phase 40 implements the EpisodeHandler RPCs (StartEpisode, RecordAction, CompleteEpisode, GetSimilarEpisodes), Phase 41 wires the RankingPayloadBuilder (salience × usage decay × stale penalty = explainable final score + observability extensions), and Phase 42 registers lifecycle scheduler jobs (EpisodeRetentionJob, VectorPruneJob). The architecture is dependency-ordered: storage before handlers, handlers before ranking composition, ranking before lifecycle. The key feature dependency that must be respected is that hybrid search wiring (BM25 routing) should come before or alongside salience/usage ranking to ensure ranking signals have results to operate on.

The primary risks come from the existing dedup architecture (v2.5): the HNSW vector index does NOT contain raw event embeddings (only TOC summaries), so dedup and similarity comparisons must use the in-memory InFlightBuffer as the primary source rather than the index. Stale filtering must be bounded (max 30% score reduction) and must exempt structural memory kinds (Constraint, Definition, Procedure) to avoid burying critical historical context. Ranking signals must be composed with a defined formula before implementation to avoid score collapse — multiplicative stacking of salience + usage + stale + novelty penalties can crush all scores to near-zero, triggering false fallback-chain activations and dropping valid results below the min_confidence threshold.

## Key Findings

### Recommended Stack

The v2.6 stack requires no new external dependencies. All features are implemented via existing crates. See `.planning/research/STACK.md` for full details.

**Core technologies:**
- **RocksDB (0.22):** Episodic storage via new CF_EPISODES and CF_EPISODE_METRICS column families; append-only, crash-safe — already in production
- **Candle + all-MiniLM-L6-v2:** Episode embeddings for GetSimilarEpisodes; 384-dim, CPU-only, ~5ms per embedding — validated since v2.0
- **usearch HNSW (v2):** Vector similarity search for episode retrieval; O(log n) approximate nearest neighbor — in production since v2.2
- **Tantivy BM25 (0.25):** Hybrid search lexical tier; needs routing wiring to complete Layer 3/4 integration — implemented but not fully wired into routing handler
- **Tokio cron scheduler:** Background lifecycle jobs; framework exists since v1.0, needs EpisodeRetentionJob + VectorPruneJob registered
- **dashmap + Arc<RwLock>:** Usage stats tracking (access_count, last_accessed_ms) for ranking decay — already in CF_USAGE_COUNTERS
- **prost + tonic (0.13/0.12):** Proto schema extensions for Episode messages + 4 new RPCs; field numbers reserved above 200 — backward-compatible additions

**Critical constraint:** All proto additions must use field numbers above 200 (reserved for Phase 23+ per PROJECT.md). The CF_EPISODES key format is `ep:{start_ts:013}:{ulid}` — lexicographic ordering enables time-range scans without secondary indexes. No SQL, no separate vector DB, no streaming RPCs, no LLM-based summarization.

### Expected Features

See `.planning/research/FEATURES.md` for full feature details with complexity analysis and implementation patterns.

**Must have (table stakes):**
- Hybrid Search (BM25 + Vector fusion via RRF) — lexical + semantic search is industry standard; currently hardcoded routing logic in hybrid handler
- Salience Scoring at Write Time — high-value events (Definitions, Constraints) must rank higher; proto fields exist, need population at ingest
- Usage-Based Decay in Ranking — access_count-weighted score adjustment; CF_USAGE_COUNTERS exists, needs threading into ranking pipeline
- Vector Index Pruning — prevents unbounded HNSW index growth; VectorIndexPipeline::prune() API exists, needs scheduler wiring
- BM25 Index Maintenance — prevents Tantivy segment bloat; Bm25LifecycleConfig exists, needs job wiring
- Admin Observability RPCs — GetDedupMetrics, GetRankingStatus extensions; operators need production visibility
- Episodic Memory Storage + RPCs — CF_EPISODES + StartEpisode/RecordAction/CompleteEpisode/GetSimilarEpisodes

**Should have (competitive differentiators):**
- Value-Based Episode Retention — percentile-based culling (delete value_score below p25, retain p50–p75 sweet spot)
- RankingPayload with explanation field — per-result explainability ("salience=0.8, usage=0.905, stale=0.0 → final=0.724")
- GetSimilarEpisodes with vector similarity — "we solved this before" retrieval pattern bridging episodic to semantic memory

**Defer (v2.7+):**
- Adaptive Lifecycle Policies — storage-pressure-based threshold adjustment (HIGH complexity, needs usage data to tune)
- Cross-Episode Learning Patterns — NLP/clustering on episode summaries (VERY HIGH complexity, requires separate NLP pipeline)
- Real-Time Outcome Feedback Loop — agent self-correction via reward signaling (out of scope for memory service)
- LLM-Based Episode Summarization — API dependency, hallucination risk, high latency (anti-pattern for local-first design)

### Architecture Approach

The v2.6 architecture is purely additive: four new components plug into the existing handler pattern (Arc<Storage> injection, checkpoint-based jobs, on-demand metrics computation). No architectural rewrite is required. The component dependency order (39 → 40 → 41 → 42) matches storage-before-handler, handler-before-ranking, ranking-before-lifecycle. All new storage uses RocksDB column families (CF_EPISODES, CF_EPISODE_METRICS) with the existing append-only immutability invariant. See `.planning/research/ARCHITECTURE.md` for full data flow diagrams and Rust struct definitions.

**Major components:**
1. **EpisodeHandler** (`crates/memory-service/src/episode.rs`) — 4 RPCs for episode lifecycle; uses Arc<Storage> + optional VectorTeleportHandler for similarity search; episodes are immutable after CompleteEpisode (enforces append-only invariant)
2. **RankingPayloadBuilder** (`crates/memory-service/src/ranking.rs`) — composes salience × usage_adjusted × (1 - stale_penalty) into final_score with human-readable explanation; extends TeleportResult proto field
3. **ObservabilityHandler extensions** — GetRankingStatus + GetDedupStatus + GetEpisodeMetrics; reads from primary CF data, no separate metrics store (single source of truth, no sync issues)
4. **EpisodeRetentionJob** (`crates/memory-scheduler/src/jobs/episode_retention.rs`) — daily 2am cron; deletes episodes where (age > 180d AND value_score < 0.3); checkpoint-based crash recovery
5. **VectorPruneJob** (`crates/memory-scheduler/src/jobs/vector_prune.rs`) — weekly Sunday 1am; copy-on-write HNSW rebuild in temp directory with atomic rename; zero query downtime during rebuild

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for full analysis with codebase evidence. All pitfalls are from v2.5's dedup/ranking architecture that v2.6 must build on top of correctly.

1. **HNSW index contains TOC summaries, NOT raw events** — Reusing the existing HNSW index for raw event dedup produces misleading similarity scores (~0.6–0.7). The InFlightBuffer (256-entry, RwLock, stores raw event embeddings) is the correct primary dedup source for within-session comparison. HNSW search is secondary for cross-session only.

2. **Threshold miscalibration for all-MiniLM-L6-v2** — Cosine similarity scores cluster [0.07, 0.80] for unrelated content with this model. Default dedup threshold must be 0.85+ (not the 0.82 novelty default). Below 0.70 causes dangerous false positives and PERMANENT data loss in the append-only store. Use dry-run mode for one week before enabling dedup in production.

3. **Ranking score collapse from multiplicative signal stacking** — Salience × usage × stale × novelty penalties compound destructively. Define composition formula before implementation. Stale penalty must be bounded at max 30% reduction. Exempt Constraint/Definition/Procedure memory kinds from all decay signals. The `min_confidence: 0.3` threshold in RetrievalExecutor will silently drop results pushed below it.

4. **Append-only invariant: store events, skip outbox (not drop events)** — Dedup must store all events but skip the outbox entry for duplicates. Dropping events before storage breaks TOC segmentation (segment boundaries use event counts) and breaks causality debugging. The store-and-skip-outbox pattern (implemented in v2.5) is the architectural precedent.

5. **HNSW write lock blocks dedup reads during index rebuild** — VectorIndexUpdater holds write lock for batch inserts; dedup reads queue behind it. Use try_read() with InFlightBuffer fallback. The VectorPruneJob copy-on-write approach (temp dir → atomic rename) eliminates contention during lifecycle sweeps.

## Implications for Roadmap

Based on combined research, the suggested phase structure for v2.6 maps to phases 39–42 as defined in ARCHITECTURE.md. The ordering respects storage-before-handler dependencies, puts observability before lifecycle (so jobs can report metrics), and treats episodic storage as the foundation all other features depend on.

### Phase 39: Episodic Memory Storage Foundation

**Rationale:** All other v2.6 phases depend on CF_EPISODES and the Episode proto schema. This is the lowest-risk phase — pure storage additions following established patterns (cf_descriptors, serde-serialized structs, ULID keys). No handler logic, no new RPCs yet. Building storage first allows thorough unit testing before handler complexity is introduced.

**Delivers:** CF_EPISODES column family, CF_EPISODE_METRICS column family, Episode/EpisodeAction/EpisodeOutcome proto messages, Episode Rust struct in memory-types, Storage::put_episode/get_episode/scan_episodes helpers, unit tests for CRUD operations.

**Addresses:** "Episodic Memory Storage & Schema" (table stakes), foundation for "Value-Based Episode Retention."

**Avoids:** Embedding episode storage logic in the handler layer before the storage layer is tested and stable.

**Research flag:** Standard patterns — RocksDB column family additions are well-documented in existing codebase. No additional research needed; use CF_TOPICS and CF_TOPIC_LINKS additions from v2.0 as templates.

---

### Phase 40: Episodic Memory Handler & RPCs

**Rationale:** After storage foundation is stable, the handler can be built following the Arc<Storage> injection pattern used by RetrievalHandler and AgentDiscoveryHandler. This phase completes the episodic memory user-facing API before ranking or lifecycle features touch it. Episode similarity search (GetSimilarEpisodes) uses the existing HNSW index — the same vector infrastructure, different granularity than dedup.

**Delivers:** EpisodeHandler struct (memory-service/src/episode.rs), StartEpisode/RecordAction/CompleteEpisode/GetSimilarEpisodes RPCs, handler wired into MemoryServiceImpl, optional embedding generation on CompleteEpisode for similarity indexing, E2E test: start → record → complete → retrieve similar.

**Addresses:** "Episodic Memory Storage & RPCs" (table stakes), "Retrieval Integration for Similar Episodes" (differentiator).

**Avoids:** HNSW lock contention during GetSimilarEpisodes — use try_read() pattern; never block on write lock. Episode records are immutable after CompleteEpisode — enforce via early return Err(EpisodeAlreadyCompleted) in RecordAction.

**Research flag:** Standard patterns — handler injection + ULID key + vector search are established in v2.5. No additional research needed.

---

### Phase 41: Ranking Payload & Observability

**Rationale:** Ranking quality improvements (salience + usage decay composition) are the highest-value retrieval changes in v2.6. They depend on v2.5's SalienceScorer and CF_USAGE_COUNTERS already being in place, and on Phase 39's Episode storage for GetEpisodeMetrics. This phase also extends admin observability RPCs to expose the metrics needed for lifecycle monitoring in Phase 42. Hybrid search BM25 routing wiring must be confirmed or completed here — FEATURES.md identifies it as the critical path prerequisite.

**Delivers:** RankingPayloadBuilder (memory-service/src/ranking.rs), composed final_score = salience × usage_adjusted × (1 - stale_penalty), explanation field in TeleportResult, GetRankingStatus extension (usage_tracked_count, memory_kind_distribution), GetDedupStatus extension (buffer_memory_bytes, dedup_rate_24h_percent), GetEpisodeMetrics RPC (new), unit tests for ranking formula, E2E test for RouteQuery explainability.

**Addresses:** "Salience Scoring at Write Time" (table stakes), "Usage-Based Decay in Ranking" (table stakes), "Admin Observability RPCs" (table stakes), "Multi-Layer Decay Coordination" (differentiator), "Hybrid Search" wiring (table stakes — confirm or complete).

**Avoids:** Score collapse from unbounded stale penalty — cap at max 30% reduction; exempt Constraint/Definition/Procedure from all decay; define formula as named constants before threading through callers. Apply stale filtering AFTER the fallback chain resolves, not within individual layer results.

**Research flag:** Needs attention before planning. The exact composition formula weights (salience=0.5, usage=0.3, stale=0.2) are initial guesses from STACK.md config — validate against E2E test queries before shipping. Also inspect `crates/memory-service/src/hybrid.rs` to confirm actual state of BM25 routing wiring.

---

### Phase 42: Lifecycle Automation Jobs

**Rationale:** Lifecycle jobs are last because they depend on Phase 39 (episode storage to scan), Phase 41 (observability to report job metrics), and the v2.5 scheduler framework. VectorPruneJob uses copy-on-write (temp dir + atomic rename) to avoid query downtime. BM25 pruning is explicitly deferred — it requires SearchIndexer write access that needs a separate design pass (noted as "Phase 42b" in ARCHITECTURE.md).

**Delivers:** EpisodeRetentionJob (daily 2am, deletes episodes where age > 180d AND value_score < 0.3), VectorPruneJob (weekly Sunday 1am, copy-on-write HNSW rebuild), checkpoint-based crash recovery for both jobs, cron registration in memory-daemon/src/main.rs, integration test for checkpoint recovery, E2E test for vector index shrinkage after prune.

**Addresses:** "Vector Index Pruning" (table stakes), "BM25 Index Maintenance" (table stakes, partial — full wiring deferred), "Value-Based Episode Retention" (differentiator, threshold-based initial implementation using value_score < 0.3 hardcoded rather than percentile analysis).

**Avoids:** Episode retention job deleting wrong records — conservative defaults (max_age=180d, threshold=0.3), dry-run mode, checkpoint recovery so aborted sweeps resume correctly. Vector prune locking out queries — copy-on-write pattern (temp directory → atomic rename) with RwLock on index directory pointer.

**Research flag:** The copy-on-write HNSW prune is the most novel engineering in v2.6. Validate that usearch supports the atomic directory rename pattern under concurrent reads. If HNSW metadata file format (embedding_id → timestamp mappings) is unclear from source, request a `/gsd:research-phase` before implementation.

---

### Phase Ordering Rationale

- **Storage first (39):** Every other phase reads or writes CF_EPISODES. Storage changes are also the hardest to retrofit safely; establishing the schema early prevents cascading changes later.
- **Handler second (40):** EpisodeHandler provides the write path. Once it exists, Phase 41's GetEpisodeMetrics RPC has real data to aggregate.
- **Ranking third (41):** RankingPayloadBuilder is the highest-value retrieval change and has no lifecycle dependency. It also exposes the observability RPCs needed for lifecycle job reporting.
- **Lifecycle last (42):** Jobs are background processes that can be added after all core functionality is tested. They depend on Phase 39 storage + Phase 41 metrics infrastructure.
- **Hybrid search wiring:** FEATURES.md identifies this as the critical path prerequisite (unblocks routing logic so salience + usage decay have effect on real results). Treat this as a pre-Phase-39 patch or include at the start of Phase 41.

### Research Flags

**Needs deeper research during planning:**
- **Phase 41 (Ranking formula weights):** The salience_weight/usage_weight/stale_weight config values are initial guesses. Validate against real query sets before shipping. Run existing 39 E2E tests with ranking_payload enabled to verify no regressions.
- **Phase 41 (Hybrid BM25 routing):** Inspect `crates/memory-service/src/hybrid.rs` before writing the phase plan — FEATURES.md reports "hardcoded routing logic" but exact state is unconfirmed.
- **Phase 42 (VectorPruneJob copy-on-write):** usearch HNSW atomic directory rename behavior under concurrent reads is the key risk. Verify RwLock release timing and directory pointer swap semantics from `crates/memory-vector/src/hnsw.rs`.

**Standard patterns (skip research-phase):**
- **Phase 39 (Episodic storage):** RocksDB column family additions follow existing CF pattern exactly. Refer to CF_TOPICS and CF_TOPIC_LINKS additions in v2.0 as the template.
- **Phase 40 (EpisodeHandler):** Arc<Storage> handler injection is well-established; RetrievalHandler and AgentDiscoveryHandler are direct templates.
- **Phase 42 (EpisodeRetentionJob):** Checkpoint-based scheduler jobs follow the existing outbox_processor and rollup job patterns exactly.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | No new dependencies; all technologies verified against workspace Cargo.toml on 2026-03-11; zero uncertainty about what to use |
| Features | HIGH | Feature list derived from direct codebase analysis (existing proto stubs, half-implemented handlers) + 20+ industry sources on hybrid search, episodic memory, lifecycle patterns |
| Architecture | HIGH | Direct codebase analysis — existing handler patterns, column family descriptors, scheduler registration, and proto field numbers all confirmed; build order respects dependency graph |
| Pitfalls | HIGH | Pitfalls derived from codebase evidence (specific file paths, line numbers, metrics confirmed) + vector search community patterns; HNSW contention, threshold calibration, and score collapse are all verifiable |

**Overall confidence:** HIGH

The main uncertainty is not technical but operational: ranking formula weights (0.5/0.3/0.2) are initial guesses that require tuning against real query distributions once implemented. The copy-on-write HNSW prune is the most architecturally novel component and deserves a targeted investigation before Phase 42 planning.

### Gaps to Address

- **Hybrid search routing code:** STACK.md notes BM25 routing is "not fully wired into routing" and FEATURES.md confirms "hardcoded routing logic." Inspect `crates/memory-service/src/hybrid.rs` before writing the Phase 41 plan to understand exact wiring needed.
- **CF_USAGE_COUNTERS schema:** UsageStats struct needs `last_accessed_ms` field added (not just access_count). Verify current schema in `crates/memory-storage/src/usage.rs` before Phase 41 — existing data may need migration handling.
- **VectorPruneJob metadata format:** The HNSW index metadata file format (embedding_id → timestamp mappings) needs to be confirmed from the usearch crate API. ARCHITECTURE.md assumes a metadata file exists; verify this assumption in `crates/memory-vector/src/hnsw.rs`.
- **BM25 lifecycle wiring:** STACK.md explicitly defers BM25 prune to "Phase 42b" because "SearchIndexer write access" needs its own design. Plan as a stretch goal or explicit follow-on outside the v2.6 scope.
- **Value-based episode retention algorithm:** FEATURES.md rates this HIGH complexity and recommends deferring to v2.6.2. Phase 42 should implement a simple threshold (value_score < 0.3) rather than the full percentile-distribution algorithm.

## Sources

### Primary (HIGH confidence — codebase analysis)
- `crates/memory-types/src/` — SalienceScorer, UsageStats, UsageConfig, DedupConfig, StalenessConfig (confirmed 2026-03-11)
- `crates/memory-storage/src/` — dashmap 6.0, lru 0.12, RocksDB 0.22, CF definitions
- `crates/memory-search/src/lifecycle.rs` — Bm25LifecycleConfig, retention_map
- `crates/memory-scheduler/` — Tokio cron job framework, OverlapPolicy, JitterConfig
- `crates/memory-vector/src/hnsw.rs` — HNSW index wrapper, RwLock, cosine distance
- `crates/memory-service/src/novelty.rs` — NoveltyChecker fail-open design, timeout handling
- `crates/memory-indexing/src/vector_updater.rs` — Confirmed: indexes TOC nodes/grips, NOT raw events
- `proto/memory.proto` — Field numbers, existing message types, reserved ranges
- `.planning/PROJECT.md` — v2.6 requirements, architectural decisions
- `docs/plans/memory-ranking-enhancements-rfc.md` — Episodic memory Tier 2 spec

### Secondary (HIGH confidence — industry sources)
- [all-MiniLM-L6-v2 Model Card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) — Threshold calibration (0.659 for literature dedup, 0.85+ for conversational dedup)
- [Elastic: A Comprehensive Hybrid Search Guide](https://www.elastic.co/what-is/hybrid-search) — RRF fusion (k=60 constant), parallel BM25+vector execution
- [Google Vertex AI: About Hybrid Search](https://docs.cloud.google.com/vertex-ai/docs/vector-search/about-hybrid-search) — Score normalization patterns
- [Memory Patterns for AI Agents](https://dev.to/gantz/memory-patterns-for-ai-agents-short-term-long-term-and-episodic-5ff1) — Episodic memory design for agentic systems
- [Designing Memory Architectures for Production-Grade GenAI Systems](https://medium.com/@avijitswain11/designing-memory-architectures-for-production-grade-genai-systems-2c20f71f9a45) — Cognitive architecture layers
- [AI-Driven Semantic Similarity Pipeline (2025)](https://arxiv.org/html/2509.15292v1) — Threshold calibration, score distribution [0.07, 0.80] for all-MiniLM-L6-v2
- [8 Common Mistakes in Vector Search](https://kx.com/blog/8-common-mistakes-in-vector-search/) — Threshold defaults, normalization pitfalls
- [OpenSearch Vector Dedup RFC](https://github.com/opensearch-project/k-NN/issues/2795) — 22% indexing speedup, 66% size reduction from dedup

### Tertiary (MEDIUM confidence — community patterns)
- [Event Sourcing Projection Deduplication](https://domaincentric.net/blog/event-sourcing-projection-patterns-deduplication-strategies) — Store-and-skip-outbox pattern validation
- [Redis: Full-text search for RAG apps: BM25 and hybrid search](https://redis.io/blog/full-text-search-for-rag-the-precision-layer/) — Hybrid search production patterns
- [What is agent observability?](https://www.braintrust.dev/articles/agent-observability-tracing-tool-calls-memory) — Admin metrics for agentic systems

---
*Research completed: 2026-03-11*
*Synthesized by: gsd-synthesizer from STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
*Ready for roadmap: yes*
