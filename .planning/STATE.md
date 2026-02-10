# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-08)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.1 Multi-Agent Ecosystem — OpenCode plugin, Gemini/Copilot adapters, cross-agent sharing

## Current Position

Milestone: v2.1 Multi-Agent Ecosystem
Phase: 21 — Gemini CLI Adapter — IN PROGRESS
Plan: 2 of 3 complete
Status: Plans 01-02 complete (hooks + commands/skills), Plan 03 remaining (install skill + README)
Last activity: 2026-02-10 — Phase 21 Plan 02 executed (2 tasks, TOML commands + skills with navigator)

Progress v2.1: [██████████░░░░░░░░░░] 50% (3/6 phases)

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

### Phase 20 Decisions

- Agent field uses serde(default) for backward-compatible JSON parsing
- HookEvent.with_agent() follows existing builder pattern from Phase 18
- RetrievalResult.agent reads from metadata HashMap (forward-compatible with index rebuilds)
- OpenCode plugin uses Bun $ shell API to pipe JSON to memory-ingest (no gRPC in TypeScript)
- Hardcoded agent:opencode in payload (plugin IS OpenCode, no detection needed)
- session.idle mapped to Stop hook event for R1.4.1/R1.4.2 (session end + checkpoint)
- Only RetrievalResult has agent field in proto; TeleportResult/VectorTeleportMatch lack it (future index metadata work)
- CLI agent display uses if-let conditional for backward compatibility

### Technical Debt (Accepted)

- 3 stub RPCs: GetRankingStatus, PruneVectorIndex, PruneBm25Index (admin features)
- Missing SUMMARY.md files for some phases

## v2.1 Phase Summary

| Phase | Name | Status |
|-------|------|--------|
| 18 | Agent Tagging Infrastructure | ✓ Complete |
| 19 | OpenCode Commands and Skills | Complete (5/5 plans) |
| 20 | OpenCode Event Capture + Unified Queries | Complete (3/3 plans) |
| 21 | Gemini CLI Adapter | In Progress (2/3 plans) |
| 22 | Copilot CLI Adapter | Ready |
| 23 | Cross-Agent Discovery + Documentation | Blocked by 21, 22 |

### Phase 21 Decisions

- Function wrapping with trap ERR EXIT for fail-open shell hooks (more robust than || true)
- $HOME env var in settings.json command path for global install (Gemini supports env var expansion)
- MEMORY_INGEST_DRY_RUN and MEMORY_INGEST_PATH env vars for testing and path override
- Redact sensitive keys from tool_input and JSON message fields only (not structural fields)
- Added .gemini/ override to .gitignore (global gitignore blocks .gemini/ directories)
- Navigator agent logic embedded in memory-query SKILL.md (Gemini has no separate agent definition format)
- Skills are separate copies from OpenCode plugin (not symlinks) for portability
- TOML commands are self-contained with full instructions (Gemini does not auto-load skills from commands)
- Parallel invocation instructions included in Navigator Mode for reduced query latency

## Next Steps

1. `/gsd:execute-phase 21` — Continue Gemini CLI adapter (Plan 03 remaining: install skill + README)
2. `/gsd:plan-phase 22` — Plan Copilot CLI adapter
3. `/gsd:plan-phase 23` — Plan Cross-Agent Discovery + Documentation (after 21 & 22)

## Phase 20 Summary

**Completed:** 2026-02-09

**Artifacts created/modified:**
- `crates/memory-ingest/src/main.rs` — CchEvent.agent field with serde(default)
- `crates/memory-client/src/hook_mapping.rs` — HookEvent.agent with with_agent() builder
- `crates/memory-service/src/retrieval.rs` — RetrievalResult.agent from metadata
- `crates/memory-daemon/src/commands.rs` — --agent filter wiring + agent display
- `plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` — Event capture plugin
- `plugins/memory-opencode-plugin/README.md` — Event capture documentation

**Tests:** 126 tests passing (13 memory-client + 14 memory-ingest + 64 memory-service + 35 memory-daemon)
**Verification:** 11/11 must-haves passed, 6/7 requirements satisfied (R4.2.3 deferred)

## Phase 19 Summary

**Completed:** 2026-02-09

**Artifacts created:**
- `plugins/memory-opencode-plugin/.opencode/command/memory-search.md` — Search command with $ARGUMENTS
- `plugins/memory-opencode-plugin/.opencode/command/memory-recent.md` — Recent command with $ARGUMENTS
- `plugins/memory-opencode-plugin/.opencode/command/memory-context.md` — Context command with $ARGUMENTS
- `plugins/memory-opencode-plugin/.opencode/skill/memory-query/SKILL.md` — Core query skill
- `plugins/memory-opencode-plugin/.opencode/skill/retrieval-policy/SKILL.md` — Tier detection skill
- `plugins/memory-opencode-plugin/.opencode/skill/topic-graph/SKILL.md` — Topic exploration skill
- `plugins/memory-opencode-plugin/.opencode/skill/bm25-search/SKILL.md` — Keyword search skill
- `plugins/memory-opencode-plugin/.opencode/skill/vector-search/SKILL.md` — Semantic search skill
- `plugins/memory-opencode-plugin/.opencode/agents/memory-navigator.md` — Navigator agent
- `plugins/memory-opencode-plugin/README.md` — Installation and usage docs

**Verification:** 20/20 must-haves passed, 16/16 requirements satisfied

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
*Updated: 2026-02-10 after Phase 21 Plan 02 execution (TOML commands + skills with navigator)*
