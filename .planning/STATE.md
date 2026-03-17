---
gsd_state_version: 1.0
milestone: v2.7
milestone_name: Multi-Runtime Portability
status: planning
stopped_at: Completed 45-01-PLAN.md
last_updated: "2026-03-17T18:56:51.667Z"
last_activity: 2026-03-16 — v2.7 roadmap created
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 1
  completed_plans: 1
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-16)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.7 Multi-Runtime Portability — Phase 45 complete, Phase 46 next

## Current Position

Phase: 46 of 50 (Installer Parser)
Plan: Ready to plan
Status: Phase 45 complete, ready to plan Phase 46
Last activity: 2026-03-17 — Phase 45 canonical source consolidation complete

Progress: [██░░░░░░░░] 17% (1/6 phases)

## Decisions

- Installer written in Rust (new workspace crate `memory-installer`)
- Canonical source format is Claude plugin format (YAML frontmatter Markdown)
- Keep both plugin directories (memory-query-plugin + memory-setup-plugin) as canonical source
- Converter trait pattern — one impl per runtime (6 converters)
- Tool name mapping tables centralized in `tool_maps.rs` (11 tools x 6 runtimes)
- Runtime-neutral storage at `~/.config/agent-memory/`
- Old manual adapters archived (not deleted) and replaced by installer output
- `gray_matter` 0.3.2 for frontmatter parsing (serde_yaml deprecated)
- `walkdir` 2.5 for directory traversal
- Managed-section markers for safe merge/upgrade/uninstall of shared config files
- `--dry-run` implemented as write-interceptor on output stage (not per-converter)
- [Phase 45]: Keep two plugin directories (no merge) per user decision; CANON-02 hooks deferred to Phase 49

## Blockers

- None

## Accumulated Context

- Phases 47 and 48 are independent after Phase 46 (parallelizable)
- Phase 49 requires both 47 and 48 (SkillsConverter extracts patterns from all converters)
- Pitfall: Gemini settings.json and OpenCode opencode.json must be merged not overwritten
- Pitfall: Unmapped tools must warn not silently drop
- Pitfall: Hook event name divergence (PascalCase vs camelCase) causes silent capture failure
- Pitfall: Archive adapters, do not delete (retire in v2.8 after one release cycle)
- OpenCode hook API shape needs verification before Phase 49
- Windows hook script strategy (WSL vs .bat/.ps1) must be decided before Phase 49

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

## Cumulative Stats

- ~50,000+ LOC Rust across 14 crates
- 5 adapter plugins (Claude Code, OpenCode, Gemini CLI, Copilot CLI, Codex CLI)
- 45+ E2E tests + 144 bats CLI tests across 5 CLIs
- 44 phases, 135 plans across 8 milestones

## Session Continuity

**Last Session:** 2026-03-17T18:56:51.665Z
**Stopped At:** Completed 45-01-PLAN.md
**Resume File:** None
