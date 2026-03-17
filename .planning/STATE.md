---
gsd_state_version: 1.0
milestone: v2.7
milestone_name: Multi-Runtime Portability
status: planning
stopped_at: Completed 46-03-PLAN.md
last_updated: "2026-03-17T22:27:37.390Z"
last_activity: 2026-03-17 — Phase 46 installer crate foundation complete
progress:
  total_phases: 6
  completed_phases: 2
  total_plans: 4
  completed_plans: 4
  percent: 33
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-16)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.7 Multi-Runtime Portability — Phase 46 complete, Phase 47 next

## Current Position

Phase: 47 of 50 (Claude & OpenCode Converters)
Plan: Ready to plan
Status: Phase 46 complete, ready to plan Phase 47
Last activity: 2026-03-17 — Phase 46 installer crate foundation complete

Progress: [███░░░░░░░] 33% (2/6 phases)

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
- [Phase 46]: Used owned Strings in installer types (not borrowed) for simplicity with trait objects
- [Phase 46]: Used Box<dyn RuntimeConverter> trait objects for converter dispatch
- [Phase 46]: Used gray_matter generic parse::<Value> for direct serde_json::Value deserialization of frontmatter
- [Phase 46]: Used match expression for tool maps (compile-time exhaustive, zero overhead)
- [Phase 46]: Callers handle mcp__* prefix check before calling map_tool (keeps static return type)
- [Phase 46]: Write-interceptor pattern: all converters produce Vec<ConvertedFile>, single write_files() handles dry-run

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

**Last Session:** 2026-03-17T20:08:59Z
**Stopped At:** Completed 46-03-PLAN.md
**Resume File:** None
