# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-08)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.1 Multi-Agent Ecosystem — OpenCode plugin, Gemini/Copilot adapters, cross-agent sharing

## Current Position

Milestone: v2.1 Multi-Agent Ecosystem
Phase: 18 — Agent Tagging Infrastructure — COMPLETE
Plan: All 4 plans executed
Status: Phase 18 complete, ready for Phase 19-22 (parallel)
Last activity: 2026-02-08 — Phase 18 executed (4 plans, 3 waves)

Progress v2.1: [███░░░░░░░░░░░░░░░░░] 17% (1/6 phases)

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)

## Accumulated Context

### Key Decisions (from v2.0)

Full decision log in PROJECT.md Key Decisions table.

- Indexes are accelerators, not dependencies (graceful degradation)
- Skills are the control plane (executive function)
- Local embeddings via Candle (no API dependency)
- Tier detection enables fallback chains
- Index lifecycle automation via scheduler

### v2.1 Context

**Research findings (2026-02-08):**

1. **Claude Code Plugin Format:**
   - `.claude-plugin/marketplace.json` — Plugin manifest
   - `commands/*.md` — YAML frontmatter with name, description, parameters, skills
   - `skills/{name}/SKILL.md` — Skill with YAML frontmatter + references/
   - `agents/*.md` — Agent with triggers and skill dependencies

2. **OpenCode Plugin Format:**
   - `.opencode/command/*.md` — Commands with `$ARGUMENTS` substitution
   - `.opencode/skill/{name}/SKILL.md` — Same skill format as Claude
   - `.opencode/agent/*.md` — Agent definitions (not hooks)
   - Skills are portable: same SKILL.md works in both

3. **Hook System Comparison:**
   - Claude Code: `.claude/hooks.yaml` via CCH binary
   - OpenCode: Uses plugins (commands/skills), not hooks for behavior
   - Gemini/Copilot: Hook systems similar to Claude (TBD research)

4. **Cross-Agent Strategy:**
   - Add `agent` field to Event proto
   - Auto-detect agent on ingest
   - Default queries return all agents
   - `--agent <name>` filter for single-agent queries

### Technical Debt (Accepted)

- 3 stub RPCs: GetRankingStatus, PruneVectorIndex, PruneBm25Index (admin features)
- Missing SUMMARY.md files for some phases

## v2.1 Phase Summary

| Phase | Name | Status |
|-------|------|--------|
| 18 | Agent Tagging Infrastructure | ✓ Complete |
| 19 | OpenCode Commands and Skills | Ready |
| 20 | OpenCode Event Capture + Unified Queries | Blocked by 19 |
| 21 | Gemini CLI Adapter | Ready |
| 22 | Copilot CLI Adapter | Ready |
| 23 | Cross-Agent Discovery + Documentation | Blocked by 21, 22 |

## Next Steps

1. `/gsd:plan-phase 19` — Plan OpenCode commands and skills
2. `/gsd:plan-phase 21` — Plan Gemini CLI adapter (can run parallel with 19)
3. `/gsd:plan-phase 22` — Plan Copilot CLI adapter (can run parallel with 19)

## Phase 18 Summary

**Completed:** 2026-02-08

**Artifacts created:**
- `proto/memory.proto` — Event.agent field, query request agent_filter fields
- `crates/memory-types/src/event.rs` — Event.agent with serde(default)
- `crates/memory-types/src/toc.rs` — TocNode.contributing_agents
- `crates/memory-adapters/` — New crate with AgentAdapter trait, AdapterConfig, AdapterError
- `crates/memory-daemon/src/cli.rs` — --agent filter on teleport and retrieval commands
- `crates/memory-retrieval/src/types.rs` — StopConditions.agent_filter
- `crates/memory-service/src/ingest.rs` — Agent extraction from proto Event

**Tests:** 61 memory-types + 19 memory-adapters + 53 memory-retrieval = 133 tests passing

---
*Updated: 2026-02-08 after Phase 18 execution*
