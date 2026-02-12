# Roadmap: Agent Memory

## Milestones

- ✅ **v1.0 MVP** — Phases 1-9 (shipped 2026-01-30)
- ✅ **v2.0 Scheduler+Teleport** — Phases 10-17 (shipped 2026-02-07)
- ✅ **v2.1 Multi-Agent Ecosystem** — Phases 18-23 (shipped 2026-02-10)
- ✅ **v2.2 Production Hardening** — Phases 24-27 (shipped 2026-02-11)
- **v2.3 TBD** — Phase 28 (planning)

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

### v2.3 Install & Setup Experience (Planning)

**Milestone Goal:** Improve installation, configuration, and user-facing setup guidance with agent skills and step-by-step docs.

- [ ] **Phase 28: Install & Configuration Skills + User Guides** (0/1 plans) -- not started
- [ ] **Phase 29: Performance Benchmarks** (0/1 plans) -- not started

## Phase Details

### Phase 28: Install & Configuration Skills + User Guides
**Goal:** Deliver step-by-step install/config user guides and agent skills that help users set up and validate the system.
**Depends on:** 19-opencode-commands-and-skills, 21-gemini-cli-adapter, 22-copilot-cli-adapter
**Requirements:** SETUP-01
**Success Criteria** (what must be TRUE):
  1. TBD
**Plans:** 1 plan
Plans:
- [ ] 28-01-PLAN.md -- Quickstart + Full Guide + agent setup guide + install/config/verify/troubleshoot skills

### Phase 29: Performance Benchmarks
**Goal:** Establish baseline ingest throughput and query latency across core retrieval layers.
**Depends on:** 24-proto-service-debt-cleanup, 25-e2e-core-pipeline-tests, 26-e2e-advanced-scenario-tests
**Requirements:** PERF-01
**Success Criteria** (what must be TRUE):
  1. TBD
**Plans:** 1 plan
Plans:
- [ ] 29-01-PLAN.md -- Perf benchmark harness + baseline reporting

## Progress

| Phase | Milestone | Plans | Status | Completed |
|-------|-----------|-------|--------|-----------|
| 1-9 | v1.0 | 20/20 | Complete | 2026-01-30 |
| 10-17 | v2.0 | 42/42 | Complete | 2026-02-07 |
| 18-23 | v2.1 | 22/22 | Complete | 2026-02-10 |
| 24-27 | v2.2 | 10/10 | Complete | 2026-02-11 |
| 28. Install & Configuration Skills + User Guides | v2.3 | 0/1 | Planned | — |
| 29. Performance Benchmarks | v2.3 | 0/1 | Planned | — |

---

*Updated: 2026-02-11 after v2.3 milestone initialization*
