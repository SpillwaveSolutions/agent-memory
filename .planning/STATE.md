---
gsd_state_version: 1.0
milestone: v3.0
milestone_name: Competitive Parity & Benchmarks
status: in_progress
stopped_at: null
last_updated: "2026-05-12T00:00:00.000Z"
last_activity: 2026-05-12 — Phase 52 (Simple CLI API) rebased onto main and opening PR
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

Phase: 52 of 53 (Simple CLI API) — opening PR
Plan: 3 of 3 complete (52-01 scaffold, 52-02 read-path, 52-03 write/query commands)
Status: Phase 51 + 51.5 + 52 done; Phase 53 (Benchmark Suite) next
Last activity: 2026-05-12 — Rebased gsd/phase-52-simple-cli-api onto post-Phase-51 main; opening PR

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

- ~58,000 LOC Rust across 16 crates (memory-orchestrator from Phase 51, memory-cli from Phase 52)
- 52 phases (Phase 1-52), 153 plans across 9 milestones (counting Phase 51's 3 plans + 51.5 + 52's 3 plans)
- 46+ E2E tests + 144 bats CLI tests + orchestrator unit tests + memory-cli unit tests

## Session Continuity

**Last Session:** 2026-05-12
**Stopped At:** Phase 52 rebased onto main; opening PR
**Resume File:** None
