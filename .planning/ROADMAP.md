# Roadmap: Agent Memory

## Milestones

- ✅ **v1.0 MVP** — Phases 1-9 (shipped 2026-01-30)
- ✅ **v2.0 Scheduler+Teleport** — Phases 10-17 (shipped 2026-02-07)
- ✅ **v2.1 Multi-Agent Ecosystem** — Phases 18-23 (shipped 2026-02-10)
- ✅ **v2.2 Production Hardening** — Phases 24-27 (shipped 2026-02-11)
- ✅ **v2.3 Install & Setup Experience** — Phases 28-29 (shipped 2026-02-12)
- ✅ **v2.4 Headless CLI Testing** — Phases 30-34 (shipped 2026-03-05)
- ✅ **v2.5 Semantic Dedup & Retrieval Quality** — Phases 35-38 (shipped 2026-03-10)
- **v2.6 Retrieval Quality, Lifecycle & Episodic Memory** — Phases 39-44 (in progress)

## Phases

<details>
<summary>v1.0 MVP (Phases 1-9) -- SHIPPED 2026-01-30</summary>

- [x] Phase 1: Foundation (5/5 plans) -- completed 2026-01-29
- [x] Phase 2: TOC Building (3/3 plans) -- completed 2026-01-29
- [x] Phase 3: Grips & Provenance (3/3 plans) -- completed 2026-01-29
- [x] Phase 5: Integration (3/3 plans) -- completed 2026-01-30
- [x] Phase 6: End-to-End (2/2 plans) -- completed 2026-01-30
- [x] Phase 7: CCH Integration (1/1 plan) -- completed 2026-01-30
- [x] Phase 8: CCH Hook Integration (1/1 plan) -- completed 2026-01-30
- [x] Phase 9: Setup Installer Plugin (4/4 plans) -- completed 2026-01-30

See: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>v2.0 Scheduler+Teleport (Phases 10-17) -- SHIPPED 2026-02-07</summary>

- [x] Phase 10: Background Scheduler (4/4 plans) -- completed 2026-02-01
- [x] Phase 10.5: Agentic TOC Search (3/3 plans) -- completed 2026-02-01
- [x] Phase 11: BM25 Teleport Tantivy (4/4 plans) -- completed 2026-02-03
- [x] Phase 12: Vector Teleport HNSW (5/5 plans) -- completed 2026-02-03
- [x] Phase 13: Outbox Index Ingestion (4/4 plans) -- completed 2026-02-03
- [x] Phase 14: Topic Graph Memory (6/6 plans) -- completed 2026-02-05
- [x] Phase 15: Configuration Wizard Skills (5/5 plans) -- completed 2026-02-05
- [x] Phase 16: Memory Ranking Enhancements (5/5 plans) -- completed 2026-02-06
- [x] Phase 17: Agent Retrieval Policy (6/6 plans) -- completed 2026-02-07

See: `.planning/milestones/v2.0-ROADMAP.md`

</details>

<details>
<summary>v2.1 Multi-Agent Ecosystem (Phases 18-23) -- SHIPPED 2026-02-10</summary>

- [x] Phase 18: Agent Tagging Infrastructure (4/4 plans) -- completed 2026-02-08
- [x] Phase 19: OpenCode Commands and Skills (5/5 plans) -- completed 2026-02-09
- [x] Phase 20: OpenCode Event Capture + Unified Queries (3/3 plans) -- completed 2026-02-09
- [x] Phase 21: Gemini CLI Adapter (4/4 plans) -- completed 2026-02-10
- [x] Phase 22: Copilot CLI Adapter (3/3 plans) -- completed 2026-02-10
- [x] Phase 23: Cross-Agent Discovery + Documentation (3/3 plans) -- completed 2026-02-10

See: `.planning/milestones/v2.1-ROADMAP.md`

</details>

<details>
<summary>v2.2 Production Hardening (Phases 24-27) -- SHIPPED 2026-02-11</summary>

- [x] Phase 24: Proto & Service Debt Cleanup (3/3 plans) -- completed 2026-02-11
- [x] Phase 25: E2E Core Pipeline Tests (3/3 plans) -- completed 2026-02-11
- [x] Phase 26: E2E Advanced Scenario Tests (3/3 plans) -- completed 2026-02-11
- [x] Phase 27: CI/CD E2E Integration (1/1 plan) -- completed 2026-02-11

See: `.planning/milestones/v2.2-ROADMAP.md`

</details>

<details>
<summary>v2.3 Install & Setup Experience (Phases 28-29) -- SHIPPED 2026-02-12</summary>

- [x] Phase 28: Install & Configuration Skills + User Guides (1/1 plan) -- completed 2026-02-12
- [x] Phase 29: Performance Benchmarks (1/1 plan) -- completed 2026-02-12

See: `.planning/milestones/v2.3-ROADMAP.md`

</details>

<details>
<summary>v2.4 Headless CLI Testing (Phases 30-34) -- SHIPPED 2026-03-05</summary>

- [x] Phase 30: Claude Code CLI Harness (6/6 plans) -- completed 2026-02-25
- [x] Phase 31: Gemini CLI Tests (2/2 plans) -- completed 2026-02-25
- [x] Phase 32: OpenCode CLI Tests (2/2 plans) -- completed 2026-02-26
- [x] Phase 33: Copilot CLI Tests (2/2 plans) -- completed 2026-03-05
- [x] Phase 34: Codex CLI Adapter + Tests + Matrix Report (3/3 plans) -- completed 2026-03-05

See: `.planning/milestones/v2.4-ROADMAP.md`

</details>

<details>
<summary>v2.5 Semantic Dedup & Retrieval Quality (Phases 35-38) -- SHIPPED 2026-03-10</summary>

- [x] Phase 35: DedupGate Foundation (2/2 plans) -- completed 2026-03-05
- [x] Phase 36: Ingest Pipeline Wiring (3/3 plans) -- completed 2026-03-06
- [x] Phase 37: StaleFilter (3/3 plans) -- completed 2026-03-09
- [x] Phase 38: E2E Validation (3/3 plans) -- completed 2026-03-10

See: `.planning/milestones/v2.5-ROADMAP.md`

</details>

### v2.6 Retrieval Quality, Lifecycle & Episodic Memory (In Progress)

**Milestone Goal:** Complete hybrid search wiring, add ranking intelligence with salience and usage decay, automate index lifecycle, expose operational observability metrics, and enable episodic memory for learning from past task outcomes.

- [ ] **Phase 39: BM25 Hybrid Wiring** - Wire BM25 into hybrid search handler and retrieval routing
- [ ] **Phase 40: Salience Scoring + Usage Decay** - Ranking quality with write-time salience and retrieval-time usage decay
- [ ] **Phase 41: Lifecycle Automation** - Scheduled vector pruning and BM25 lifecycle policies
- [ ] **Phase 42: Observability RPCs** - Admin metrics for dedup, ranking, and operational health
- [ ] **Phase 43: Episodic Memory Schema & Storage** - Episode and Action data model with RocksDB column family
- [ ] **Phase 44: Episodic Memory gRPC & Retrieval** - Episode lifecycle RPCs, similarity search, and value-based retention

## Phase Details

### Phase 39: BM25 Hybrid Wiring
**Goal**: Users get combined lexical and semantic search results from a single query, with BM25 serving as fallback when vector index is unavailable
**Depends on**: v2.5 (shipped)
**Requirements**: HYBRID-01, HYBRID-02, HYBRID-03, HYBRID-04
**Success Criteria** (what must be TRUE):
  1. A teleport_query returns results that include both BM25 keyword matches and vector similarity matches, fused via RRF scoring
  2. When the vector index is unavailable, route_query falls back to BM25-only results instead of returning empty
  3. The hybrid search handler reports bm25_available() = true (no longer hardcoded false)
  4. An E2E test proves that a query matching content indexed by both BM25 and vector returns combined results from both layers
**Plans**: 2

Plans:
- [ ] 39-01: Wire BM25 into HybridSearchHandler and retrieval routing
- [ ] 39-02: E2E hybrid search test

### Phase 40: Salience Scoring + Usage Decay
**Goal**: Retrieval results are ranked by a composed formula that rewards high-salience content, penalizes overused results, and composes cleanly with existing stale filtering
**Depends on**: Phase 39
**Requirements**: RANK-01, RANK-02, RANK-03, RANK-04, RANK-05, RANK-06, RANK-07, RANK-08, RANK-09, RANK-10
**Success Criteria** (what must be TRUE):
  1. TOC nodes and Grips have salience scores calculated at write time based on length density, kind boost, and pinned boost
  2. Retrieval results for pinned or high-salience items consistently rank higher than low-salience items of similar similarity
  3. Frequently accessed results receive a usage decay penalty so that fresh results surface above stale, over-accessed ones
  4. The combined ranking formula (similarity x salience_factor x usage_penalty) composes with StaleFilter without collapsing scores below min_confidence threshold
  5. Salience weights and usage decay parameters are configurable via config.toml sections
**Plans**: 3

Plans:
- [ ] 40-01: Salience scoring at write time
- [ ] 40-02: Usage-based decay in retrieval ranking
- [ ] 40-03: Ranking E2E tests

### Phase 41: Lifecycle Automation
**Goal**: Index sizes are automatically managed through scheduled pruning jobs, preventing unbounded growth of vector and BM25 indexes
**Depends on**: Phase 40
**Requirements**: LIFE-01, LIFE-02, LIFE-03, LIFE-04, LIFE-05, LIFE-06, LIFE-07
**Success Criteria** (what must be TRUE):
  1. Old vector index segments are automatically pruned by the scheduler based on configurable segment_retention_days
  2. An admin CLI command allows manual vector pruning with --age-days parameter
  3. BM25 index can be rebuilt with a --min-level filter that excludes fine-grain segment docs after rollup
  4. An admin CLI command allows manual BM25 rebuild with level filtering
  5. An E2E test proves that old segments are removed from the vector index after a lifecycle job runs
**Plans**: 2

Plans:
- [ ] 41-01: Vector pruning wiring + CLI command
- [ ] 41-02: BM25 lifecycle policy + E2E test

### Phase 42: Observability RPCs
**Goal**: Operators can inspect dedup, ranking, and system health metrics through admin RPCs and CLI, enabling production monitoring and debugging
**Depends on**: Phase 40
**Requirements**: OBS-01, OBS-02, OBS-03, OBS-04, OBS-05
**Success Criteria** (what must be TRUE):
  1. GetDedupStatus returns the actual InFlightBuffer size and dedup hit rate (no longer hardcoded 0)
  2. IngestEventResponse includes a deduplicated boolean field indicating whether the event was a duplicate
  3. Ranking metrics (salience distribution, usage decay stats) are queryable via admin RPC
  4. `memory-daemon status --verbose` prints a human-readable summary of dedup and ranking health
**Plans**: 2

Plans:
- [ ] 42-01: Dedup observability — buffer size + deduplicated field
- [ ] 42-02: Ranking metrics + verbose status CLI

### Phase 43: Episodic Memory Schema & Storage
**Goal**: The system has a persistent, queryable storage layer for task episodes with structured actions and outcomes
**Depends on**: v2.5 (shipped) — independent of Phases 39-42
**Requirements**: EPIS-01, EPIS-02, EPIS-03
**Success Criteria** (what must be TRUE):
  1. Episode struct exists with episode_id, task, plan, actions, outcome_score, lessons_learned, failure_modes, embedding, and created_at fields
  2. Action struct exists with action_type, input, result, and timestamp fields
  3. CF_EPISODES column family is registered in RocksDB and episodes can be stored and retrieved by ID
**Plans**: 1

Plans:
- [ ] 43-01: Episode schema, storage, and column family

### Phase 44: Episodic Memory gRPC & Retrieval
**Goal**: Agents can record task outcomes as episodes, search for similar past episodes by vector similarity, and the system retains episodes based on their learning value
**Depends on**: Phase 43
**Requirements**: EPIS-04, EPIS-05, EPIS-06, EPIS-07, EPIS-08, EPIS-09, EPIS-10, EPIS-11, EPIS-12
**Success Criteria** (what must be TRUE):
  1. An agent can start an episode, record actions during execution, and complete it with an outcome score and lessons learned
  2. GetSimilarEpisodes returns past episodes ranked by vector similarity to a query embedding, enabling "we solved this before" retrieval
  3. Value-based retention scores episodes by distance from the 0.65 optimal outcome, and episodes below the retention threshold are eligible for pruning
  4. Episodic memory is configurable via [episodic] config section (enabled flag, value_threshold, max_episodes)
  5. E2E tests prove the full episode lifecycle (create, record, complete, search) and value-based retention scoring
**Plans**: 3

Plans:
- [ ] 44-01: Episode gRPC proto definitions and handler
- [ ] 44-02: Similar episode search and value-based retention
- [ ] 44-03: Episodic memory E2E tests

## Progress

**Execution Order:**
Phases execute in numeric order: 39 → 40 → 41 → 42 → 43 → 44
Note: Phases 43-44 (Episodic Memory) are independent of 39-42 and could be parallelized.

| Phase | Milestone | Plans | Status | Completed |
|-------|-----------|-------|--------|-----------|
| 1-9 | v1.0 | 20/20 | Complete | 2026-01-30 |
| 10-17 | v2.0 | 42/42 | Complete | 2026-02-07 |
| 18-23 | v2.1 | 22/22 | Complete | 2026-02-10 |
| 24-27 | v2.2 | 10/10 | Complete | 2026-02-11 |
| 28-29 | v2.3 | 2/2 | Complete | 2026-02-12 |
| 30-34 | v2.4 | 15/15 | Complete | 2026-03-05 |
| 35-38 | v2.5 | 11/11 | Complete | 2026-03-10 |
| 39. BM25 Hybrid Wiring | v2.6 | 0/2 | Planned | - |
| 40. Salience + Usage Decay | v2.6 | 0/3 | Planned | - |
| 41. Lifecycle Automation | v2.6 | 0/2 | Planned | - |
| 42. Observability RPCs | v2.6 | 0/2 | Planned | - |
| 43. Episodic Schema & Storage | v2.6 | 0/1 | Planned | - |
| 44. Episodic gRPC & Retrieval | v2.6 | 0/3 | Planned | - |

---

*Updated: 2026-03-11 after v2.6 roadmap created*
