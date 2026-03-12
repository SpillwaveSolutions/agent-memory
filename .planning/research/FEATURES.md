# Feature Landscape: v2.6 Episodic Memory, Ranking Quality, Lifecycle & Observability

**Domain:** Agent Memory System - Cognitive Architecture with Retrieval Quality & Experience Learning
**Researched:** 2026-03-11
**Scope:** Episodic memory, salience scoring, usage-based decay, lifecycle automation, observability RPCs, hybrid search integration

---

## Table Stakes

Features users expect given the existing 6-layer cognitive stack. Missing these = system feels incomplete or untrustworthy.

| Feature | Why Expected | Complexity | Category | Notes |
|---------|--------------|-----------|----------|-------|
| **Hybrid Search (BM25 + Vector)** | Lexical + semantic search is industry standard for RAG; existing BM25/vector layers must interoperate | Medium | Retrieval | Currently hardcoded routing logic; needed to complete Layer 3/4 wiring |
| **Salience Scoring at Write Time** | High-value/structural events (Definitions, Constraints) must rank higher; already in design (Layer 6) | Low | Ranking | Write-time scoring avoids expensive retrieval-time computation; enables kind-based exemptions |
| **Usage-Based Decay in Ranking** | Frequently accessed memories fade; rarely touched memories strengthen — mimics human forgetting (Ebbinghaus) | Medium | Ranking | Requires access_count tracking on reads; integrates with existing StaleFilter (14-day half-life) |
| **Vector Index Pruning** | Memory grows unbounded; stale/low-value vectors waste storage and retrieval speed | Low | Lifecycle | Part of background scheduler; removes old/low-salience vectors periodically |
| **BM25 Index Maintenance** | Lexical index needs periodic rebuild/compaction; low-entropy shards waste search time | Low | Lifecycle | Level-filtered rebuild (only rebuild bottom N levels of TOC tree) |
| **Admin Observability RPCs** | Operators need visibility into dedup/ranking health; required for production troubleshooting | Low | Observability | GetDedupMetrics, GetRankingStatus RPCs; expose buffer_size, events_skipped, salience distribution |
| **Episodic Memory Storage & Schema** | Record task outcomes, search similar past episodes — enables learning from experience | Medium | Episodic | CF_EPISODES column family; Episode proto with start_time, actions, outcome, value_score |

---

## Differentiators

Features that set the system apart from naive implementations. Not expected, but highly valued by power users.

| Feature | Value Proposition | Complexity | Category | Notes |
|---------|-------------------|-----------|----------|-------|
| **Value-Based Episode Retention** | Delete low-value episodes, retain "Goldilocks zone" (medium utility); learn from successful experiences without storage bloat | High | Episodic | Prevents pathological retention (too high = dedup everything; too low = no learning); requires outcome scoring percentile analysis |
| **Retrieval Integration for Similar Episodes** | When answering a query, optionally search past episodes (GetSimilarEpisodes); surface "we solved this before and it worked" | High | Episodic | Bridges episodic → semantic; depends on episode embedding + vector search; powerful for repeated task patterns |
| **Adaptive Lifecycle Policies** | Retention thresholds adjust based on storage pressure, salience distribution, usage patterns | High | Lifecycle | Not essential v2.6; deferred for v2.7 adaptive optimization phase |
| **Multi-Layer Decay Coordination** | Stale filter + usage decay + episode retention all tune together (no conflicting signals) | Medium | Ranking | Requires tuning framework; candidates: weighted sum, per-layer thresholds, Bayesian composition |
| **Observability Dashboard Integration** | Admin RPC metrics feed into operator dashboards (Prometheus, CloudWatch, DataDog) | Low | Observability | External tool integration only; requires stable RPC interface + consistent metric names |
| **Cross-Episode Learning Patterns** | Identify repeated task types, success/failure patterns across episodes | Very High | Episodic | Requires NLP/clustering on episode summaries; deferred for v2.7+ self-improvement |

---

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Automatic Memory Forgetting Without User Choice** | Agent should never silently delete memories; violates append-only principle and causality debugging | Lifecycle jobs are delete-by-policy (configurable); admins set thresholds; users can override |
| **Real-Time Outcome Feedback Loop (Agent Self-Correcting)** | Too complex for v2.6; requires agent control flow that's outside memory's scope | Record episode outcomes (human validation); v2.7 can add reward signaling to retrieval policy |
| **Graph-Based Episode Dependencies** | Tempting but overengineered; TOC tree + timestamps sufficient for temporal navigation | Use TOC + episode timestamps; cross-reference via event_id links; avoid graph DB complexity |
| **LLM-Based Episode Summarization** | High latency, API dependency, hallucination risk; hard to troubleshoot | Use salience scores + existing grip-based summaries (already in TOC); optionally add human review |
| **Per-Agent Lifecycle Scoping** | Multi-agent mode can defer this; would require partition keys in every pruning job | Lifecycle policies are global; agents filter on retrieval (agent-filtered queries already work) |
| **Continuous Outcome Recording** | If users must label every action, adoption suffers | Make outcome recording opt-in; batch via CompleteEpisode RPC with single outcome score |
| **Real-Time Index Rebuilds** | Blocking user queries during index maintenance kills UX | Schedule pruning jobs during off-hours; implement dry-run reporting for production safety |

---

## Feature Dependencies

Dependency graph for implementation order.

```
Hybrid Search (BM25 Router)
  ↓ (requires Layer 3/4 operational, unblocks routing logic)
Salience Scoring at Write Time
  ↓ (requires write-time scoring populated in TOC/Grips)
Usage-Based Decay in Ranking
  ↓ (requires access_count tracking + ranking pipeline)
Admin Observability RPCs
  ├─ (exposes dedup + ranking metrics)
  ↓
Vector/BM25 Index Lifecycle Jobs
  ├─ (scheduler jobs, can run parallel with above)
  ↓
Episodic Memory Storage & RPCs
  ├─ (depends on Event storage, independent of indexes)
  ├─ (can start parallel with lifecycle work)
  ↓
Value-Based Episode Retention
  ├─ (depends on outcome scoring; runs after retention policy jobs)
  ↓
Similar Episode Retrieval (Optional)
  └─ (depends on CompositeVectorIndex; runs post-episodic-memory)
```

**Critical Path (must do in order):**
1. Hybrid Search wiring (unblocks ranking)
2. Salience + Usage Decay (ranking works end-to-end)
3. Admin RPCs (observability for production)
4. Episodic Memory storage (independent, parallel-safe)
5. Value-based retention (completion feature, can defer 1 sprint)

**Parallel-Safe Work:**
- Index lifecycle jobs (no dependency on episodic memory)
- Admin RPC metrics gathering (can stub metrics early, populate later)

---

## Implementation Patterns

### Hybrid Search (BM25 + Vector Fusion)

**What it does:** Route queries to both BM25 and vector indexes; combine rankings via Reciprocal Rank Fusion (RRF) or weighted average.

**How it works (industry standard):**
1. **Parallel execution:** Run BM25 query + Vector query concurrently
2. **Score normalization:** Bring both to [0, 1] scale (RRF or linear mapping)
3. **Fusion:** Combine via RRF (no tuning) or weighted blend (tunable weights)
4. **Routing heuristic:**
   - Keyword-heavy query (identifiers, class names) → weight BM25 higher (0.6 BM25, 0.4 Vector)
   - Semantic query ("find discussions about X") → weight Vector higher (0.4 BM25, 0.6 Vector)
   - Default → equal weights (0.5 BM25, 0.5 Vector)

**Integration with existing retrieval policy:**
- Already has intent classification (Explore/Answer/Locate/TimeBoxed)
- Layer 3/4 searches are independent; hybrid merges at ranking stage
- Retrieval policy's tier detection and fallback chains already in place

**Complexity:** MEDIUM — RRF is simple math; requires coordinating two async searches.

**Expected behavior (validation):**
- Keyword queries (e.g., "JWT token") retrieve via BM25 without latency spike
- Semantic queries (e.g., "how did we handle auth?") use vector similarity
- Graceful fallback: if BM25 fails, vector search results are returned (and vice versa)

---

### Salience Scoring at Write Time

**What it does:** Assign importance scores (0.0-1.0) at ingest time based on event kind.

**How it works:**
- Already in Layer 6 design; KIND classification determines salience
- High-salience kinds: `constraint`, `definition`, `procedure`, `tool_result_error` (0.9-1.0)
- Medium-salience: `user_message`, `assistant_stop` (0.5-0.7)
- Low-salience: `session_start`, `session_end` (0.1-0.3)

**Integration point:**
- TocNode and Grip protos already have `salience_score` field (v2.5+)
- Populate at ingest time via `SalienceScorer::score_event(kind)` (static lookup)
- Used in Layer 6 ranking as multiplicative factor

**Complexity:** LOW — scoring rules are static lookup table; no ML required.

**Expected behavior:**
- Constraints/definitions never decay (exempted from StaleFilter)
- Session markers have low salience (deprioritized in ranking)
- Ranking score = base_score × salience_factor × (1 - stale_penalty) × (1 - usage_decay)

---

### Usage-Based Decay in Ranking

**What it does:** Reduce ranking score for frequently-accessed items (inverse recency); strengthen rarely-touched items.

**How it works:**
- Track `access_count` per TOC node / Grip (incremented on read)
- At retrieval ranking time: apply decay factor = 1.0 / log(access_count + 1) or exp(-access_count / K)
- Decay is multiplicative: `final_score = base_score × salience_factor × (1 - decay_factor) × (1 - stale_penalty)`

**Rationale:** Mimics human memory — rehearsed facts fade from conscious retrieval; novel facts stay sharp (Ebbinghaus forgetting curve validated in cognitive psychology).

**Tuning considerations:**
- Decay floor: never drop score below 20% (prevent collapse)
- Decay half-life: decay factor = 0.5 at access_count = 100 (tunable via config)
- Exempt structural events: high-salience kinds don't decay (same as StaleFilter)

**Complexity:** MEDIUM — requires tracking + lookup at ranking time; no external service.

**Expected behavior:**
- Recent queries with low access_count rank higher (novel information)
- Popular results (high access_count) gradually fade unless repeatedly accessed
- Salience exemptions prevent "boring but important" facts from disappearing

---

### Index Lifecycle Automation via Scheduler

**Vector Index Pruning:**
- **When:** Weekly or when storage threshold exceeded
- **What:** Remove vectors for events marked `skip_vector` or older than 90 days + low-salience
- **How:** HNSW index is rebuildable from TOC tree; deletion is safe
- **Job:** `VectorPruneJob` in background scheduler (framework exists since v1.0)
- **Dry-run:** Log what WOULD be deleted; allow admin override

**BM25 Index Maintenance:**
- **When:** Weekly or when search latency exceeds SLA
- **What:** Rebuild BM25 index for bottom N levels of TOC (recent events prioritized)
- **How:** Tantivy segment merge + compaction; can be online (dual indexes)
- **Job:** `Bm25RebuildJob` with level filtering
- **Dry-run:** Report segment stats before rebuild

**Complexity:** LOW — scheduler framework exists; jobs are independent.

**Expected behavior:**
- Vector index size decreases over time (no unbounded growth)
- BM25 latency stays consistent (no slowdown from segment bloat)
- Operators can monitor pruning effectiveness via metrics RPCs

---

### Admin Observability RPCs

**What users need to see:**

| Metric | RPC Field | Why | Example Value |
|--------|-----------|-----|-------|
| **Dedup Buffer Size** | `infl_buffer_size` | Is dedup gate backed up? | 128 / 256 entries |
| **Events Deduplicated (Session)** | `events_skipped_session` | How many duplicates caught? | 47 events |
| **Events Deduplicated (Cross-Session)** | `events_skipped_cross_session` | Long-term dedup working? | 312 events |
| **Salience Distribution** | `salience_histogram[0.0-0.2]`, etc. | Is content balanced? | {0.0-0.2: 100, 0.2-0.4: 50, ...} |
| **Usage Decay Distribution** | `access_count_p50`, `p99` | Are hot/cold patterns healthy? | p50=3, p99=157 |
| **Vector Index Size** | `vector_index_entries` | Storage used by vectors? | 18,432 entries |
| **BM25 Index Size** | `bm25_index_bytes` | Storage used by BM25? | 2.4 MB |
| **Last Pruning Timestamp** | `last_vector_prune_time` | When did cleanup last run? | 2026-03-09T14:30:00Z |

**Exposed via:**
- `GetRankingStatus` RPC (already stubbed v2.2)
- `GetDedupMetrics` RPC (new in v2.6)
- Both return structured proto with histogram buckets

**Complexity:** LOW — reading metrics from existing data structures; no computation.

**Expected behavior:**
- Metrics RPCs respond in <100ms (cached, no expensive scans)
- Salience histogram shows multimodal distribution (not flat)
- Usage decay p50 < p99 by 50x+ (confirming hot/cold pattern)

---

### Episodic Memory Storage & RPCs

**What it does:** Record sequences of actions + outcomes from tasks, enabling "we solved this before" retrieval.

**Proto Schema:**
```protobuf
message Episode {
  string episode_id = 1;           // UUID
  int64 start_time_us = 2;         // micros since epoch
  int64 end_time_us = 3;           // 0 if incomplete
  string task_description = 4;     // "debug JWT token leak"
  repeated EpisodeAction actions = 5;  // sequence of steps
  EpisodeOutcome outcome = 6;      // success/partial/failure + value_score
  float value_score = 7;           // 0.0-1.0, outcome importance
  repeated string tags = 8;        // ["auth", "jwt"] for retrieval filtering
  string contributing_agent = 9;   // agent_id, reuses existing field
}

message EpisodeAction {
  int64 timestamp_us = 1;
  string action_type = 2;          // "query_memory", "tool_call", "decision"
  string description = 3;
  map<string, string> metadata = 4;
}

message EpisodeOutcome {
  string status = 1;               // "success" | "partial" | "failure"
  float outcome_value = 2;         // 0.0-1.0, how well did we do?
  string summary = 3;              // "JWT token rotation fixed in 3 steps"
  int64 duration_ms = 4;           // total task duration
}
```

**Storage:** RocksDB column family `CF_EPISODES`; keyed by episode_id; queryable by start_time range.

**RPCs:**
```protobuf
service EpisodeService {
  rpc StartEpisode(StartEpisodeRequest) returns (StartEpisodeResponse);
  rpc RecordAction(RecordActionRequest) returns (RecordActionResponse);
  rpc CompleteEpisode(CompleteEpisodeRequest) returns (CompleteEpisodeResponse);
  rpc GetSimilarEpisodes(GetSimilarEpisodesRequest) returns (GetSimilarEpisodesResponse);
  rpc ListEpisodes(ListEpisodesRequest) returns (ListEpisodesResponse);
}
```

**Complexity:** MEDIUM — new storage layer; RPCs are straightforward; outcome_value is user-provided (not computed).

**Expected behavior:**
- StartEpisode returns unique episode_id
- RecordAction appends to episode's action sequence
- CompleteEpisode commits outcome (idempotent)
- GetSimilarEpisodes returns episodes with similar task_description + tags
- Episodes survive crash recovery (like TOC nodes)

---

### Value-Based Episode Retention

**What it does:** Auto-delete low-value episodes; keep high-value ones; sweet-spot detection prevents pathological retention.

**Problem:** If all episodes are retained, system degrades (storage + retrieval latency). If auto-delete is too aggressive, learning is lost.

**Solution (industry pattern):** Retention threshold based on outcome score distribution.

**Algorithm:**
1. **Analyze distribution:** Compute p25, p50, p75 of value_score across recent episodes
2. **Sweet spot:** Retain episodes in range [p50, p75] or [p50, 1.0] depending on storage pressure
3. **Culling policy:** Delete episodes with value_score < p25 OR older than 180 days
4. **Tuning lever:** Config parameter `retention_percentile` (default 50)

**Rationale:**
- p25 (low-value): routine tasks, minimal learning value → delete early
- p50-p75 (sweet spot): moderately complex, high learning value → retain long-term
- p75+ (high-value): critical issues, precedent-setting → never auto-delete

**Complexity:** HIGH — requires statistical analysis + configurable tuning; deferred to v2.6.2.

**Expected behavior:**
- Retention job runs weekly without blocking writes
- Episodes with value_score < p25 are removed
- Operators can view retention policy metrics (deletion count, space reclaimed)

---

## MVP Recommendation

**Phase 1 (Weeks 1-2): Hybrid Search Wiring**
- Unblock Layer 3/4 routing logic
- Enables salience + usage-based ranking to have effect
- Complexity: MED, high impact

**Phase 2 (Weeks 2-3): Salience Scoring at Write Time**
- Low complexity, enables kind-based exemptions in decay
- Integrates naturally with existing TOC/Grip protos
- Complexity: LOW

**Phase 3 (Weeks 3-4): Usage-Based Decay in Retrieval Ranking**
- Multiplicative with StaleFilter; tunable floor
- Requires access_count tracking (add to TocNode/Grip)
- Complexity: MED

**Phase 4 (Weeks 4-5): Admin Observability RPCs**
- Expose metrics for production troubleshooting
- Low complexity, high operational value
- Complexity: LOW

**Phase 5 (Weeks 5-6): Vector Index Pruning + BM25 Lifecycle**
- Scheduler jobs; independent implementation
- Prevent unbounded index growth
- Complexity: LOW

**Phase 6 (Weeks 7-8, if time allows): Episodic Memory Storage & RPCs**
- Independent of ranking; can be built in parallel
- Complexity: MED, moderate impact

**Defer (v2.6.1 or v2.7):**
- **Value-Based Episode Retention** (v2.6.2) — Requires outcome scoring model; HIGH complexity
- **Similar Episode Retrieval** (v2.7) — Nice-to-have; HIGH complexity
- **Adaptive Lifecycle Policies** (v2.7) — Not essential; HIGH complexity

---

## Success Criteria

**v2.6 Feature Completeness:**
- [ ] Hybrid search queries route correctly (E2E test hitting both BM25 + Vector)
- [ ] Salience scores populated at write time (inspect TOC nodes/grips in RocksDB)
- [ ] Usage decay reduces scores predictably (access_count increments, ranking penalizes correctly)
- [ ] Admin metrics RPCs return non-zero values (GetRankingStatus, GetDedupMetrics)
- [ ] Index pruning jobs complete without errors (scheduler logs show cleanup)
- [ ] Episodic memory RPCs accept/return well-formed protos (round-trip test)
- [ ] 10+ E2E tests cover new features (hybrid routing, decay behavior, lifecycle jobs, observability)

**Regression Prevention:**
- [ ] All v2.5 tests still pass (dedup, stale filter, multi-agent)
- [ ] No new performance regressions (latency within 5% of v2.5 baseline)
- [ ] Graceful degradation holds (hybrid search falls back if BM25 fails, etc.)

---

## Integration with Existing Architecture

**Layers Affected:**

| Layer | Change | Impact |
|-------|--------|--------|
| Layer 0 (Events) | Add access_count tracking to event retrieval path | Minimal — new field, write-only during reads |
| Layer 1 (TOC) | Add salience_score, access_count to TocNode | Minimal — already has versioning for append-safe updates |
| Layer 2 (TOC Search) | None | None |
| Layer 3 (BM25) | Wire into hybrid routing; add pruning job | Medium — coordination with Layer 4 ranking |
| Layer 4 (Vector) | Wire into hybrid routing; add pruning job | Medium — coordination with Layer 3 ranking |
| Layer 5 (Topic Graph) | None | None |
| Layer 6 (Ranking) | Add salience factor, usage decay factor | Medium — multiplicative composition of factors |
| Control (Retrieval Policy) | Wire hybrid search router; tune fallback chains | Medium — new routing decision point |
| Scheduler | Add VectorPruneJob, Bm25RebuildJob | Low — framework already exists |
| Storage (RocksDB) | Add CF_EPISODES column family | Low — isolated new column family |

**No breaking changes** to existing gRPC contracts; new RPCs/fields added via proto `oneof` or new message types.

---

## Risk Mitigation

| Risk | Likelihood | Mitigation |
|------|------------|-----------|
| **Hybrid search combines incompatible scores** | MED | Normalize both indexes to [0, 1] before fusion; test with known-good queries |
| **Usage decay creates retrieval bias** | MED | Log all decay factors in traces; audit queries with low access_count but high relevance |
| **Index pruning deletes needed content** | LOW | Dry-run mode with reporting; never auto-delete structural events; admin confirmation |
| **Episode value_score inflation** | MED | Cap at 1.0; require outcome_value validation in RPC; monitor distribution metrics |
| **Episodic memory storage bloat** | MED | Implement retention policy early; set aggressive TTL during v2.6 pilot |
| **Observability metrics cause latency** | LOW | Metrics are computed on-demand or cached; profile before/after RPC calls |

---

## Sources

- [Designing Memory Architectures for Production-Grade GenAI Systems | Avijit Swain | March 2026](https://medium.com/@avijitswain11/designing-memory-architectures-for-production-grade-genai-systems-2c20f71f9a45)
- [Memory Patterns for AI Agents: Short-term, Long-term, and Episodic | DEV Community](https://dev.to/gantz/memory-patterns-for-ai-agents-short-term-long-term-and-episodic-5ff1)
- [From Storage to Experience: A Survey on the Evolution of LLM Agent Memory Mechanisms | Preprints.org](https://www.preprints.org/manuscript/202601.0618)
- [Implementing Cognitive Memory for Autonomous Robots: Hebbian Learning, Decay, and Consolidation in Production | Varun Sharma | Medium](https://medium.com/@29.varun/implementing-cognitive-memory-for-autonomous-robots-hebbian-learning-decay-and-consolidation-in-faea53b3973a)
- [A Comprehensive Hybrid Search Guide | Elastic](https://www.elastic.co/what-is/hybrid-search)
- [About hybrid search | Vertex AI | Google Cloud Documentation](https://docs.cloud.google.com/vertex-ai/docs/vector-search/about-hybrid-search)
- [Full-text search for RAG apps: BM25 & hybrid search | Redis](https://redis.io/blog/full-text-search-for-rag-the-precision-layer/)
- [7 Hybrid Search Recipes: BM25 + Vectors Without Lag | Hash Block | Medium](https://medium.com/@connect.hashblock/7-hybrid-search-recipes-bm25-vectors-without-lag-467189542bf0)
- [Hybrid Search: Combining BM25 and Semantic Search for Better Results with Langchain | Akash A Desai | Medium](https://medium.com/etoai/hybrid-search-combining-bm25-and-semantic-search-for-better-results-with-lan-1358038fe7e6)
- [Hybrid Search RAG in the Real World: Graphs, BM25, and the End of Black-Box Retrieval | NetApp Community](https://community.netapp.com/t5/Tech-ONTAP-Blogs/Hybrid-RAG-in-the-Real-World-Graphs-BM25-and-the-End-of-Black-Box-Retrieval/ba-p/464834)
- [Index lifecycle management (ILM) in Elasticsearch | Elastic Docs](https://www.elastic.co/docs/manage-data/lifecycle/index-lifecycle-management)
- [What is agent observability? Tracing tool calls, memory, and multi-step reasoning | Braintrust](https://www.braintrust.dev/articles/agent-observability-tracing-tool-calls-memory)
- [Observability for AI Workloads: A New Paradigm for a New Era | Dotan Horovits | Medium | January 2026](https://horovits.medium.com/observability-for-ai-workloads-a-new-paradigm-for-a-new-era-b8972ba1b6ba)
- [AI Agent Memory Security Requires More Observability | Valdez Ladd | Medium | December 2025](https://medium.com/@oracle_43885/ai-agent-memory-security-requires-more-observability-b12053e39ff0)
- [Building Self-Improving AI Agents: Techniques in Reinforcement Learning and Continual Learning | Technology.org | March 2026](https://www.technology.org/2026/03/02/self-improving-ai-agents-reinforcement-continual-learning/)
- [Process vs. Outcome Reward: Which is Better for Agentic RAG Reinforcement Learning | OpenReview](https://openreview.net/forum?id=h3LlJ6Bh4S)
- [Experiential Reinforcement Learning | Microsoft Research](https://www.microsoft.com/en-us/research/articles/experiential-reinforcement-learning/)
- [A Survey on the Memory Mechanism of Large Language Model-based Agents | ACM Transactions on Information Systems](https://dl.acm.org/doi/10.1145/3748302)
- [Evaluating Memory in LLM Agents via Incremental Multi-Turn Interactions | ICLR 2026 | GitHub](https://github.com/HUST-AI-HYZ/MemoryAgentBench)
- [Cache Replacement Policies Explained for System Performance | Aerospike](https://aerospike.com/blog/cache-replacement-policies/)
- [How to Configure LRU and LFU Eviction in Redis | OneUptime | January 2026](https://oneuptime.com/blog/post/2026-01-25-redis-lru-lfu-eviction/view)

---

**Last Updated:** 2026-03-11
**For Milestone:** v2.6 Retrieval Quality, Lifecycle & Episodic Memory
