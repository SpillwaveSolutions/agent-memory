---
gsd_state_version: 1.0
milestone_name: Competitive Parity & Benchmarks
status: in_progress
stopped_at: null
last_updated: "2026-05-14T00:00:00.000Z"
last_activity: 2026-05-14 — Phase 52 merged via PR #29; Phase 53.5 (cross-project) re-rebased for merge
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 7
  completed_plans: 7
  percent: 75
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-22)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v3.0 Phase 52 — Simple CLI API (PR review)

## Current Position

Phase: 53.5 of 53 (cross-project federation, out-of-band) — landing via PR #25
Plan: 1 of 1 complete (53.5-01 cross-project federated query)
Status: Phase 51 + 51.5 + 52 merged; Phase 53.5 (cross-project) merging next; Phase 53 (Benchmark Suite) still pending
Last activity: 2026-05-14 — Phase 52 merged via PR #29; PR #25 re-rebased onto post-Phase-52 main

Progress: [████████░░] 75% (3 of 4 phases)

## Out-of-band Work

### Open PRs

| PR | Branch | Status | Reviewed | Notes |
|---|---|---|---|---|
| #25 | `feature/v3.0-cross-project-memory` | Open, CI green | Not yet | Recorded as Phase 53.5 (decimal-phase pattern, mirrors 51.5); rebased onto main 2026-05-08 |
| #27 | merged 2026-04-28 as `3a73582` | Merged | — | Recorded as Phase 51.5; supersedes closed PR #26 |
| #28 | merged 2026-04-28 as `85f3303` | Merged | — | Phase 51 Retrieval Orchestrator |

### Local-only Branches (still stacked, pending PRs)

- `gsd/phase-{53..58}` — 6-phase stack of GSD work covering remaining v3.0 (Phase 53 Benchmark Suite), v3.1 (Phases 54-56), and v3.2 (Phases 57-58). Each branch backed up to origin 2026-05-12 (no PRs). Pending strategic decision: per-milestone PRs vs. per-phase. **Note:** the planning files on these branches describe v3.0/v3.1 as "shipped" — that reflects local execution intent, not origin/main reality.

## Performance Metrics

**Velocity:**
- Total plans completed: 146 (across 9 milestones)
- Average duration: ~15 min
- Total execution time: ~36 hours

**Milestone History:**
See .planning/MILESTONES.md

## Decisions

- v3.0 scope: Retrieval orchestrator, simple CLI API, benchmark suite (3 phases) + Phase 51.5 (out-of-band summarizer wiring)
- Orchestrator wraps existing RetrievalExecutor (no changes to memory-retrieval crate)
- CLI uses new `memory` binary (memory-daemon and hook handlers unchanged)
- LOCOMO dataset never committed (gitignored)
- Existing implementation plans in docs/superpowers/plans/ will be converted to GSD plans
- Phase 51.5 inserted as a decimal phase (out-of-band insertion pattern from `/gsd:insert-phase`) since the summarizer wiring shipped before Phase 51 itself
- [Phase 51]: RerankMode defaults to Heuristic (no LLM cost by default)
- [Phase 51]: RankedResult uses f64 for fusion precision, SearchResult uses f32
- [Phase 51]: RRF deduplicates by doc_id, keeping first-seen SearchResult
- [Phase 51]: HeuristicReranker trims to top 10 (MAX_RESULTS const)
- [Phase 51]: Token estimation: chars * 0.75 + 50 overhead
- [Phase 51]: MemoryOrchestrator accepts Box<dyn Reranker> via with_reranker() for test injection
- [Phase 52]: All CLI commands route through gRPC (no direct RocksDB access) — daemon stays single source of truth
- [Phase 52]: JsonEnvelope output pattern: ok/error/context_ok constructors, TTY detection via IsTerminal
- [Phase 52]: New `memory-cli` crate (binary name: `memory`) added to workspace — separate from `memory-daemon`
- Phase 53.5 inserted as a decimal phase (out-of-band, mirrors 51.5) for cross-project federation work originally authored against the Phase 51 slot — preserves v3.0 Competitive Parity scope (Phases 51-53)
- [Phase 53.5]: TOC-based primary fallback in `federated_query` when BM25/vector indexes aren't built — ensures cross-project mode always works
- [Phase 53.5]: Project attribution stored in `metadata["project"]` — same convention as `metadata["agent"]` from v2.1
- [Phase 53.5]: `federated_query` is a pure function — matches existing `enrich_with_salience` pattern
- [Phase 53.5]: `open_read_only` uses `DB::open_cf_for_read_only` from rocksdb 0.22 with `create_if_missing(false)`

## Blockers

- None

## Accumulated Context

- Spec reference: docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md
- Phase A plan: docs/superpowers/plans/2026-03-21-v3-phase-a-retrieval-orchestrator.md
- Phase B plan: docs/superpowers/plans/2026-03-21-v3-phase-b-simple-cli-api.md
- Phase C plan: docs/superpowers/plans/2026-03-21-v3-phase-c-benchmark-suite.md

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)
- v2.1 Multi-Agent Ecosystem: Shipped 2026-02-10 (6 phases, 22 plans)
- v2.2 Production Hardening: Shipped 2026-02-11 (4 phases, 10 plans)
- v2.3 Install & Setup Experience: Shipped 2026-02-12 (2 phases, 2 plans)
- v2.4 Headless CLI Testing: Shipped 2026-03-05 (5 phases, 15 plans)
- v2.5 Semantic Dedup & Retrieval Quality: Shipped 2026-03-10 (4 phases, 11 plans)
- v2.6 Cognitive Retrieval: Shipped 2026-03-16 (6 phases, 13 plans)
- v2.7 Multi-Runtime Portability: Shipped 2026-03-22 (6 phases, 11 plans)

## Cumulative Stats

- ~58,400 LOC Rust across 16 crates (memory-orchestrator from Phase 51, memory-cli from Phase 52) + federated module in memory-service (Phase 53.5)
- 52 phases (Phase 1-52 + 53.5), 154 plans across 9 milestones
- 50+ E2E tests + 144 bats CLI tests + orchestrator + memory-cli + 9 federated unit tests + 4 cross-project e2e tests

## Session Continuity

**Last Session:** 2026-05-12
**Stopped At:** Phase 52 rebased onto main; opening PR
**Resume File:** None
