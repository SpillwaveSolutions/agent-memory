# Requirements: Agent Memory v2.6

**Defined:** 2026-03-10
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v2.6 Requirements

Requirements for Retrieval Quality, Lifecycle & Episodic Memory milestone. Each maps to roadmap phases.

### Hybrid Search

- [ ] **HYBRID-01**: BM25 wired into HybridSearchHandler (currently hardcoded `bm25_available() = false`)
- [ ] **HYBRID-02**: Hybrid search returns combined BM25 + vector results via RRF score fusion
- [ ] **HYBRID-03**: BM25 fallback enabled in retrieval routing when vector index unavailable
- [ ] **HYBRID-04**: E2E test verifies hybrid search returns results from both BM25 and vector layers

### Ranking

- [ ] **RANK-01**: Salience score calculated at write time on TOC nodes (length_density + kind_boost + pinned_boost)
- [ ] **RANK-02**: Salience score calculated at write time on Grips
- [ ] **RANK-03**: `is_pinned` field added to TocNode and Grip (default false)
- [ ] **RANK-04**: Usage tracking: `access_count` and `last_accessed` updated on retrieval hits
- [ ] **RANK-05**: Usage-based decay penalty applied in retrieval ranking (1.0 / (1.0 + 0.15 * access_count))
- [ ] **RANK-06**: Combined ranking formula: similarity * salience_factor * usage_penalty
- [ ] **RANK-07**: Ranking composites with existing StaleFilter (score floor at 50% to prevent collapse)
- [ ] **RANK-08**: Salience and usage_decay configurable via config.toml sections
- [ ] **RANK-09**: E2E test: pinned/high-salience items rank higher than low-salience items
- [ ] **RANK-10**: E2E test: frequently-accessed items score lower than fresh items (usage decay)

### Lifecycle

- [ ] **LIFE-01**: Vector pruning scheduler job calls existing `prune(age_days)` on configurable schedule
- [ ] **LIFE-02**: CLI command: `memory-daemon admin prune-vectors --age-days N`
- [ ] **LIFE-03**: Config: `[lifecycle.vector] segment_retention_days` controls pruning threshold
- [ ] **LIFE-04**: BM25 rebuild with level filter excludes fine-grain docs after rollup
- [ ] **LIFE-05**: CLI command: `memory-daemon admin rebuild-bm25 --min-level day`
- [ ] **LIFE-06**: Config: `[lifecycle.bm25] min_level_after_rollup` controls BM25 retention granularity
- [ ] **LIFE-07**: E2E test: old segments pruned from vector index after lifecycle job runs

### Observability

- [ ] **OBS-01**: `buffer_size` exposed in GetDedupStatus (currently hardcoded 0)
- [ ] **OBS-02**: `deduplicated` field added to IngestEventResponse (deferred proto change from v2.5)
- [ ] **OBS-03**: Dedup threshold hit rate and events_skipped rate exposed via admin RPC
- [ ] **OBS-04**: Ranking metrics (salience distribution, usage decay stats) queryable via admin RPC
- [ ] **OBS-05**: CLI: `memory-daemon status --verbose` shows dedup/ranking health summary

### Episodic Memory

- [ ] **EPIS-01**: Episode struct with episode_id, task, plan, actions, outcome_score, lessons_learned, failure_modes, embedding, created_at
- [ ] **EPIS-02**: Action struct with action_type, input, result, timestamp
- [ ] **EPIS-03**: CF_EPISODES column family in RocksDB for episode storage
- [ ] **EPIS-04**: StartEpisode gRPC RPC creates new episode and returns episode_id
- [ ] **EPIS-05**: RecordAction gRPC RPC appends action to in-progress episode
- [ ] **EPIS-06**: CompleteEpisode gRPC RPC finalizes episode with outcome_score, lessons, failure_modes
- [ ] **EPIS-07**: GetSimilarEpisodes gRPC RPC searches by vector similarity on episode embeddings
- [ ] **EPIS-08**: Value-based retention: episodes scored by distance from 0.65 optimal outcome
- [ ] **EPIS-09**: Retention threshold: episodes with value_score < 0.18 eligible for pruning
- [ ] **EPIS-10**: Configurable via `[episodic]` config section (enabled, value_threshold, max_episodes)
- [ ] **EPIS-11**: E2E test: create episode → complete → search by similarity returns match
- [ ] **EPIS-12**: E2E test: value-based retention correctly identifies low/high value episodes

## Future Requirements

Deferred to v2.7+. Tracked but not in current roadmap.

### Consolidation

- **CONS-01**: Extract durable knowledge (preferences, constraints, procedures) from recent events
- **CONS-02**: Daily consolidation scheduler job with NLP/LLM pattern extraction
- **CONS-03**: CF_CONSOLIDATED column family for extracted knowledge atoms

### Cross-Project

- **XPROJ-01**: Unified memory queries across multiple project stores
- **XPROJ-02**: Cross-project dedup for shared context

### Agent Scoping

- **SCOPE-01**: Per-agent dedup thresholds (only dedup within same agent's history)
- **SCOPE-02**: Agent-filtered lifecycle policies

### Operational

- **OPS-01**: True daemonization (double-fork on Unix)
- **OPS-02**: API-based summarizer wiring (OpenAI/Anthropic when key present)
- **OPS-03**: Config example file (config.toml.example) shipped with binary

## Out of Scope

| Feature | Reason |
|---------|--------|
| LLM-based episode summarization | Adds latency, hallucination risk, external dependency |
| Automatic memory forgetting/deletion | Violates append-only invariant |
| Real-time outcome feedback loops | Out of scope for v2.6; need agent framework integration |
| Graph-based episode dependencies | Overengineered for initial episode support |
| Per-agent lifecycle scoping | Defer to v2.7 when multi-agent dedup is validated |
| Continuous outcome recording | Adoption killer — complete episodes only |
| Real-time index rebuilds | UX killer — batch via scheduler only |
| Cross-project memory | Requires architectural rethink of per-project isolation |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| HYBRID-01 | Phase 39 | Pending |
| HYBRID-02 | Phase 39 | Pending |
| HYBRID-03 | Phase 39 | Pending |
| HYBRID-04 | Phase 39 | Pending |
| RANK-01 | Phase 40 | Pending |
| RANK-02 | Phase 40 | Pending |
| RANK-03 | Phase 40 | Pending |
| RANK-04 | Phase 40 | Pending |
| RANK-05 | Phase 40 | Pending |
| RANK-06 | Phase 40 | Pending |
| RANK-07 | Phase 40 | Pending |
| RANK-08 | Phase 40 | Pending |
| RANK-09 | Phase 40 | Pending |
| RANK-10 | Phase 40 | Pending |
| LIFE-01 | Phase 41 | Pending |
| LIFE-02 | Phase 41 | Pending |
| LIFE-03 | Phase 41 | Pending |
| LIFE-04 | Phase 41 | Pending |
| LIFE-05 | Phase 41 | Pending |
| LIFE-06 | Phase 41 | Pending |
| LIFE-07 | Phase 41 | Pending |
| OBS-01 | Phase 42 | Pending |
| OBS-02 | Phase 42 | Pending |
| OBS-03 | Phase 42 | Pending |
| OBS-04 | Phase 42 | Pending |
| OBS-05 | Phase 42 | Pending |
| EPIS-01 | Phase 43 | Pending |
| EPIS-02 | Phase 43 | Pending |
| EPIS-03 | Phase 43 | Pending |
| EPIS-04 | Phase 44 | Pending |
| EPIS-05 | Phase 44 | Pending |
| EPIS-06 | Phase 44 | Pending |
| EPIS-07 | Phase 44 | Pending |
| EPIS-08 | Phase 44 | Pending |
| EPIS-09 | Phase 44 | Pending |
| EPIS-10 | Phase 44 | Pending |
| EPIS-11 | Phase 44 | Pending |
| EPIS-12 | Phase 44 | Pending |

**Coverage:**
- v2.6 requirements: 38 total
- Mapped to phases: 38
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-10*
*Last updated: 2026-03-10 after initial definition*
