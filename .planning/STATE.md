---
gsd_state_version: 1.0
milestone_name: Make It True
status: shipping
stopped_at: null
last_updated: "2026-08-31T01:30:00.000Z"
last_activity: 2026-08-31 — Phase 57 merged (#36); v3.1 shipped; Phase 58 launch prep (version 3.1.0, CHANGELOG, launch drafts)
progress:
  total_phases: 6
  completed_phases: 5
  total_plans: 14
  completed_plans: 14
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-22)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v3.1 Phase 58 — Launch (side quest): version 3.1.0, CHANGELOG, release archive fixes, launch drafts. The tag and the public posts are maintainer actions.

## Current Position

Phase: 58 of 58 (Launch — side quest, not a GSD phase)
Status: all v3.1 GSD phases merged (54, 54.5, 55, 56, 57). Launch prep in review.
Last activity: 2026-08-31 — #36 merged; version bumped 2.7.0 → 3.1.0

Progress: [██████████] 14/14 plans merged. v3.1 GSD work complete.

Remaining launch steps are maintainer actions: tag `v3.1.0` (publishes public
binaries), set the repo description/topics/Discussions, record the demo, and
post the blog and launch threads.

## Out-of-band Work

### Open PRs

| PR | What | Status |
|---|---|---|
| _(Phase 58 launch prep)_ | version 3.1.0, CHANGELOG, release fix, launch drafts | Open |

### Recently Merged

| PR | What | Merged |
|---|---|---|
| #36 | Phase 57 Shop Window & Positioning | 2026-08-31 |
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
- Phase 58: version is 3.1.0 — it had been stuck at 2.7.0 through the whole v3.0 and v3.1 line. Tags and GitHub releases exist through v2.7.0 (2026-03-22); v3.0 and v3.1 were milestone names that never shipped a release. An earlier note here claimed the repo had no tags — that was a shallow-clone artifact, not the truth
- Phase 58 blocker: this session cannot cut the release. `git push origin v3.1.0` returns HTTP 403 through the agent git proxy, and the GitHub App cannot `workflow_dispatch` release.yml ("Resource not accessible by integration"). The tag must be pushed by a maintainer
- Phase 58: release archives are `agent-memory-<version>-<platform>` and carry all four binaries; the CLI the quickstart needs was previously not shipped
- Phase 58: `admin rebuild-bm25` is a prune, not a rebuild — relabelled rather than renamed, and there is no event backfill path
