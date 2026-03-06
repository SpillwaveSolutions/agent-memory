# Roadmap: Agent Memory

## Milestones

- ✅ **v1.0 MVP** — Phases 1-9 (shipped 2026-01-30)
- ✅ **v2.0 Scheduler+Teleport** — Phases 10-17 (shipped 2026-02-07)
- ✅ **v2.1 Multi-Agent Ecosystem** — Phases 18-23 (shipped 2026-02-10)
- ✅ **v2.2 Production Hardening** — Phases 24-27 (shipped 2026-02-11)
- ✅ **v2.3 Install & Setup Experience** — Phases 28-29 (shipped 2026-02-12)
- ✅ **v2.4 Headless CLI Testing** — Phases 30-34 (shipped 2026-03-05)
- **v2.5 Semantic Dedup & Retrieval Quality** — Phases 35-38 (in progress)

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

### v2.5 Semantic Dedup & Retrieval Quality (In Progress)

**Milestone Goal:** Reduce retrieval noise by preventing duplicate events at ingest via vector similarity and filtering stale results at query time.

#### Phase 35: DedupGate Foundation
**Goal**: Agents receive clean, deduplicated indexes because the system detects semantic duplicates before they reach indexing
**Depends on**: Phase 34 (existing HNSW + CandleEmbedder infrastructure from v2.0)
**Requirements**: DEDUP-01, DEDUP-05, DEDUP-06
**Success Criteria** (what must be TRUE):
  1. Incoming events are embedded and checked against an in-flight buffer (256 entries) that catches within-session duplicates
  2. Similarity threshold is configurable via config.toml with a default of 0.85
  3. When the embedder or vector search fails, events pass through unchanged (fail-open)
  4. DedupGate unit tests pass with MockEmbedder and MockVectorIndex proving duplicate detection and fail-open behavior
**Plans**: 2 plans

Plans:
- [ ] 35-01: InFlightBuffer data structure + DedupConfig in memory-types
- [ ] 35-02: Enhanced NoveltyChecker wired to real CandleEmbedder and HnswIndex with fail-open + unit tests

#### Phase 36: Ingest Pipeline Wiring
**Goal**: Duplicate events are stored but excluded from indexes, preserving the append-only invariant while keeping indexes clean
**Depends on**: Phase 35
**Requirements**: DEDUP-02, DEDUP-03, DEDUP-04
**Success Criteria** (what must be TRUE):
  1. Cross-session duplicates are detected by querying the HNSW vector index for events similar to the incoming event
  2. Duplicate events are written to RocksDB (append-only preserved) but skip the outbox so they are never indexed into HNSW or BM25
  3. Structural events (session_start, session_end, subagent_start, subagent_stop) bypass the dedup gate entirely and are always indexed
  4. IngestEventResponse includes a `deduplicated` field indicating whether the event was marked as a duplicate
**Plans**: TBD

Plans:
- [ ] 36-01: DedupGate injected into MemoryServiceImpl ingest path with store-and-skip-outbox behavior
- [ ] 36-02: Per-event-type bypass for structural events + proto additions (deduplicated field, GetDedupStatus RPC)

#### Phase 37: StaleFilter
**Goal**: Agents get fresher, more relevant results because outdated and superseded content is downranked at query time
**Depends on**: Phase 34 (existing retrieval infrastructure; independent of Phases 35-36)
**Requirements**: RETRV-01, RETRV-02, RETRV-03, RETRV-04
**Success Criteria** (what must be TRUE):
  1. Query results are downranked via exponential time-decay relative to the newest result in the result set
  2. Older results that are semantically similar to newer results on the same topic are marked as superseded and further downranked
  3. High-salience events (Constraint, Definition, Procedure memory kinds) are exempt from time-decay penalties
  4. Time-decay half-life is configurable via config.toml with a default of 14 days
  5. Maximum stale penalty is bounded at 30% score reduction to prevent score collapse with existing ranking layers
**Plans**: TBD

Plans:
- [ ] 37-01: StaleFilter component with time-decay, supersession detection, kind exemptions, and StalenessConfig
- [ ] 37-02: Wire StaleFilter into retrieval executor post-merge, pre-return + integration tests

#### Phase 38: E2E Validation
**Goal**: End-to-end tests prove dedup and stale filtering work correctly through the complete pipeline
**Depends on**: Phases 35, 36, 37
**Requirements**: TEST-01, TEST-02, TEST-03
**Success Criteria** (what must be TRUE):
  1. E2E test ingests duplicate events and verifies they exist in RocksDB storage but are absent from BM25 and vector index results
  2. E2E test ingests events spanning multiple time periods and verifies that stale results rank lower than recent results for the same query
  3. E2E test disables the embedder and verifies events still ingest successfully (fail-open proven end-to-end)
**Plans**: TBD

Plans:
- [ ] 38-01: E2E dedup tests (duplicate rejection, storage preservation, structural bypass)
- [ ] 38-02: E2E stale filtering tests (time-decay downranking, supersession, kind exemptions)
- [ ] 38-03: E2E fail-open tests (embedder failure, timeout, HNSW lock contention)

## Progress

| Phase | Milestone | Plans | Status | Completed |
|-------|-----------|-------|--------|-----------|
| 1-9 | v1.0 | 20/20 | Complete | 2026-01-30 |
| 10-17 | v2.0 | 42/42 | Complete | 2026-02-07 |
| 18-23 | v2.1 | 22/22 | Complete | 2026-02-10 |
| 24-27 | v2.2 | 10/10 | Complete | 2026-02-11 |
| 28-29 | v2.3 | 2/2 | Complete | 2026-02-12 |
| 30-34 | v2.4 | 15/15 | Complete | 2026-03-05 |
| 35 | v2.5 | 0/2 | Not started | - |
| 36 | v2.5 | 0/2 | Not started | - |
| 37 | v2.5 | 0/2 | Not started | - |
| 38 | v2.5 | 0/3 | Not started | - |

---

*Updated: 2026-03-05 after v2.5 roadmap created*
