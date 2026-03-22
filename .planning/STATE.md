---
gsd_state_version: 1.0
milestone: v3.0
milestone_name: Competitive Parity & Benchmarks
status: defining_requirements
stopped_at: null
last_updated: "2026-03-22T05:00:00.000Z"
last_activity: 2026-03-22 — Milestone v3.0 started
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-22)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v3.0 Competitive Parity & Benchmarks — Defining requirements

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-03-22 — Milestone v3.0 started

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
- [Phase 47]: format!-based YAML emitter with quoting for special chars and block scalar for multiline
- [Phase 47]: Shared helpers in converters/helpers.rs reusable by all converters
- [Phase 48]: Codex commands become skills/{name}/SKILL.md with YAML frontmatter
- [Phase 48]: AGENTS.md generated with skills list, agent descriptions, and sandbox recommendations
- [Phase 48]: Tool deduplication applied after Codex mapping (Write and Edit both map to edit)
- [Phase 48]: Agents become skill directories with SKILL.md (Gemini has no separate agent format)
- [Phase 48]: Shell variable escaping: ${VAR} to $VAR for Gemini template compatibility
- [Phase 48]: settings.json uses _comment array and __managed_by marker for safe merge
- [Phase 49]: target_dir uses .github/ (not .github/copilot/) matching Copilot CLI discovery
- [Phase 49]: Hook script embedded via include_str! from canonical adapter; camelCase events with bash/timeoutSec/comment fields
- [Phase 49]: Skills converter uses canonical Claude tool names (no remapping) for runtime-agnostic skills
- [Phase 50]: Used CARGO_MANIFEST_DIR for reliable workspace root discovery in integration tests
- [Phase 50]: Preserved memory-capture.sh for include_str! compile dependency in CopilotConverter

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
- v2.7 Multi-Runtime Portability: Shipped 2026-03-22 (6 phases, 11 plans)

## Cumulative Stats

- ~56,400 LOC Rust across 15 crates
- memory-installer with 6 runtime converters
- 46+ E2E tests + 144 bats CLI tests across 5 CLIs
- 50 phases, 146 plans across 9 milestones

## Session Continuity

**Last Session:** 2026-03-22T02:46:19.509Z
**Stopped At:** Completed 50-02-PLAN.md
**Resume File:** None
