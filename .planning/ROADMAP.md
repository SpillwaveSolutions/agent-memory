# Roadmap: Agent Memory

## Milestones

- ✅ **v1.0 MVP** — Phases 1-9 (shipped 2026-01-30)
- ✅ **v2.0 Scheduler+Teleport** — Phases 10-17 (shipped 2026-02-07)
- ✅ **v2.1 Multi-Agent Ecosystem** — Phases 18-23 (shipped 2026-02-10)
- **v2.2 Production Hardening** — Phases 24-27 (in progress)

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

### v2.2 Production Hardening (In Progress)

**Milestone Goal:** Make Agent Memory CI-verified and production-ready by closing all tech debt, adding E2E pipeline tests, and strengthening CI/CD.

- [ ] **Phase 24: Proto & Service Debt Cleanup** - Wire stub RPCs, fix session_count, add agent fields to teleport results
- [ ] **Phase 25: E2E Core Pipeline Tests** - Full pipeline, index teleport, topic, and grip provenance tests
- [ ] **Phase 26: E2E Advanced Scenario Tests** - Multi-agent, graceful degradation, and error path tests
- [ ] **Phase 27: CI/CD E2E Integration** - E2E tests running in GitHub Actions on every PR

## Phase Details

### Phase 24: Proto & Service Debt Cleanup
**Goal**: All gRPC RPCs are fully wired and return real data; teleport results include agent attribution
**Depends on**: Nothing (standalone tech debt work)
**Requirements**: DEBT-01, DEBT-02, DEBT-03, DEBT-04, DEBT-05, DEBT-06
**Success Criteria** (what must be TRUE):
  1. GetRankingStatus RPC returns the current ranking configuration (salience weights, decay settings) instead of an unimplemented error
  2. PruneVectorIndex and PruneBm25Index RPCs trigger actual index cleanup and return a status indicating what was pruned
  3. ListAgents RPC returns accurate session_count by scanning events, not just TOC nodes
  4. TeleportResult and VectorTeleportMatch proto messages include an agent field populated from event metadata
**Plans**: TBD

### Phase 25: E2E Core Pipeline Tests
**Goal**: The core ingest-to-query pipeline is verified end-to-end by automated tests covering every search layer
**Depends on**: Phase 24 (agent fields and wired RPCs needed for complete assertions)
**Requirements**: E2E-01, E2E-02, E2E-03, E2E-04, E2E-07
**Success Criteria** (what must be TRUE):
  1. A test ingests events, triggers TOC segment build with grips, and verifies route_query returns results with correct provenance
  2. A test ingests events, builds BM25 index, and verifies bm25_search returns matching events ranked by relevance
  3. A test ingests events, builds vector index, and verifies vector_search returns semantically similar events
  4. A test ingests events, runs topic clustering, and verifies get_top_topics returns relevant topics
  5. A test ingests events with grips, calls expand_grip, and verifies source events with surrounding context are returned
**Plans**: TBD

### Phase 26: E2E Advanced Scenario Tests
**Goal**: Edge cases and multi-agent scenarios are verified: cross-agent queries, fallback chains, and error handling all work correctly
**Depends on**: Phase 25 (builds on core test infrastructure and helpers)
**Requirements**: E2E-05, E2E-06, E2E-08
**Success Criteria** (what must be TRUE):
  1. A test ingests events from multiple agents, verifies cross-agent query returns all results, and filtered query returns only the specified agent's results
  2. A test queries with missing indexes and verifies the system degrades gracefully to TOC-based fallback, still returning useful results
  3. A test sends malformed events and invalid queries, verifying graceful error responses (no panics, useful error messages)
**Plans**: TBD

### Phase 27: CI/CD E2E Integration
**Goal**: E2E tests run automatically in GitHub Actions on every PR, with clear pass/fail reporting
**Depends on**: Phase 25, Phase 26 (E2E tests must exist before CI can run them)
**Requirements**: CI-01, CI-02, CI-03
**Success Criteria** (what must be TRUE):
  1. GitHub Actions CI pipeline includes an E2E test job that runs the full E2E suite
  2. The E2E job triggers on pull requests to main (not just pushes to main)
  3. CI output shows E2E test count and individual pass/fail status separately from unit/integration tests
**Plans**: TBD

## Progress

**Execution Order:** 24 -> 25 -> 26 -> 27

| Phase | Milestone | Plans | Status | Completed |
|-------|-----------|-------|--------|-----------|
| 1-9 | v1.0 | 20/20 | Complete | 2026-01-30 |
| 10-17 | v2.0 | 42/42 | Complete | 2026-02-07 |
| 18-23 | v2.1 | 22/22 | Complete | 2026-02-10 |
| 24. Proto & Service Debt Cleanup | v2.2 | 0/TBD | Not started | - |
| 25. E2E Core Pipeline Tests | v2.2 | 0/TBD | Not started | - |
| 26. E2E Advanced Scenario Tests | v2.2 | 0/TBD | Not started | - |
| 27. CI/CD E2E Integration | v2.2 | 0/TBD | Not started | - |

---

*Updated: 2026-02-10 after v2.2 roadmap creation*
