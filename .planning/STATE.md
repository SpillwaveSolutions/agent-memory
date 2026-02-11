# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-10)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.2 Production Hardening — Phase 26 in progress

## Current Position

Milestone: v2.2 Production Hardening
Phase: 26 of 27 (E2E Advanced Scenario Tests)
Plan: 2 of 3 in current phase (26-01 and 26-02 done)
Status: In Progress
Last activity: 2026-02-11 — Completed 26-01 Multi-Agent E2E Tests

Progress: [######----] 67% (Phase 26)

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)
- v2.1 Multi-Agent Ecosystem: Shipped 2026-02-10 (6 phases, 22 plans)

## Performance Metrics

**Velocity:**
- Total plans completed: 8 (v2.2)
- Average duration: 20min
- Total execution time: 157min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 24 | 3 | 81min | 27min |
| 25 | 3 | 29min | 10min |
| 26 | 2 | 47min | 24min |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v2.2: E2E tests use cargo test infrastructure (not separate framework)
- v2.2: Tech debt resolved before E2E tests (agent fields needed for assertions)
- 24-01: Use SalienceConfig/NoveltyConfig defaults as truth for GetRankingStatus
- 24-01: Bound session event scan to 365 days for performance
- 24-01: BM25 lifecycle reported as false (no persistent config storage)
- 24-02: First contributing_agents entry used as primary agent for BM25 index
- 24-02: serde(default) on VectorEntry.agent for backward-compatible deserialization
- 24-02: with_agent() builder on VectorEntry to avoid breaking existing callers
- 24-03: Vector prune removes metadata only; orphaned HNSW vectors harmless until rebuild-index
- 24-03: BM25 prune is report-only (TeleportSearcher is read-only; deletion requires SearchIndexer)
- 24-03: Level matching for vectors uses doc_id prefix pattern (:day:, :week:, :segment:)
- 25-01: tempfile/rand as regular deps in e2e-tests since lib.rs is shared test infrastructure
- 25-01: Direct RetrievalHandler testing via tonic::Request without gRPC server
- 25-01: MockSummarizer grip extraction may yield zero grips; tests handle gracefully
- 25-02: Ranking assertions use segment membership (node+grip IDs) not exact node_id, since grips may outrank parent node
- 25-03: OnceLock<Arc<CandleEmbedder>> shared across tests to prevent concurrent model loading race
- 25-03: Vector E2E tests use #[ignore] due to ~80MB model download; topic tests run without ignore
- 25-03: Topic tests use direct TopicStorage::save_topic instead of full HDBSCAN clustering
- 26-01: build_toc_with_agent wrapper sets contributing_agents after TocBuilder (TocBuilder does not propagate agent from events)
- 26-01: Discovery test uses recent-timestamp events for session counting within 365-day window
- 26-01: Filtered query test verifies BM25 agent attribution directly; agent_filter not yet implemented in route_query handler
- 26-02: All four degradation scenarios tested without #[ignore] since they require no external resources
- 26-02: Agentic layer returns empty results (expected behavior per TOC navigation TODO); tests verify no-panic, not result content

### Technical Debt (target of this milestone)

- ~~GetRankingStatus stub~~ (DONE - 24-01)
- ~~2 stub RPCs: PruneVectorIndex, PruneBm25Index~~ (DONE - 24-03)
- ~~session_count = 0 in ListAgents~~ (DONE - 24-01)
- ~~TeleportResult/VectorTeleportMatch lack agent field~~ (DONE - 24-02)
- No automated E2E tests in CI

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-02-11
Stopped at: Completed 26-01-PLAN.md — Multi-Agent E2E Tests
Resume file: None
