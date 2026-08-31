---
gsd_state_version: 1.0
milestone_name: Make It True
status: in_progress
stopped_at: null
last_updated: "2026-08-30T22:30:00.000Z"
last_activity: 2026-08-30 — Phase 56 merged (#34); Phase 57 Shop Window & Positioning in execution
progress:
  total_phases: 6
  completed_phases: 4
  total_plans: 14
  completed_plans: 11
  percent: 79
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-22)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v3.1 Phase 57 — Shop Window & Positioning (root README, LICENSE, positioning writeup, supported-surface tiering)

## Current Position

Phase: 57 of 58 (Shop Window & Positioning)
Plan: 01-03 implemented on `claude/phase-54-toolchain-drift-3k4fer`
Status: Phases 54, 54.5, 55, 56 merged; Phase 57 in review
Last activity: 2026-08-30 — #34 merged; Phase 57 README/LICENSE/positioning/scope-trim

Progress: [████████░░] 11/14 plans merged; Phase 57's 3 plans are implemented and in review
(Phase 58 is a side quest, not a GSD phase)

## Out-of-band Work

### Open PRs

| PR | What | Status |
|---|---|---|
| _(none open)_ | | |

### Recently Merged

| PR | What | Merged |
|---|---|---|
| #34 | Phase 56 Honest Benchmarks | 2026-08-30 |
| #35 | Phase 54.5 truth leaks + rustc 1.97 pin | 2026-08-30 |
| #33 | Phase 55 Performance Truth | 2026-08-30 |
| #32 | Phase 54 Integration Truth | 2026-08-30 |
| #31 | v3.1 Make It True design spec | 2026-08-30 |
| #30 | Phase 53 Benchmark Suite | 2026-08-30 |
| #25 | Phase 53.5: cross-project federated query | 2026-05-14 |
| #29 | Phase 52: Simple CLI API | 2026-05-14 |
| #28 | Phase 51: Retrieval Orchestrator | 2026-04-28 |

## Decisions

- v3.1 scope: Make It True — no new capabilities; close claim/reality gap (Phases 54-58)
- Phase 54.5: explainability reports what ran; shared HNSW handle; CI pins rust-toolchain.toml to 1.97
- Phase 55: split setup vs query in `perf_bench`; p90/p99 withheld below 10/30 samples
- Warm = one setup + N query samples; cold = new store per iteration
- Phase 56: substring metric is `context_hit_rate`; HOLD LOCOMO comparison marketing until `locomo_llm_judge` artifact exists
- Phase 57 tiering: Tier 1 = Claude Code + Codex CLI (PR gate); Tier 2 = Gemini + Copilot (weekly schedule)
- Phase 57: OpenCode removed rather than archived — a converter whose methods return empty is a false success, not a gap
- Phase 57: no comparative benchmark claim ships while the only committed results are mock-backend / mock-judge
