# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-08)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.1 Multi-Agent Ecosystem — OpenCode plugin, Gemini/Copilot adapters, cross-agent sharing

## Current Position

Milestone: v2.1 Multi-Agent Ecosystem
Phase: 22 — Copilot CLI Adapter — Complete
Plan: 3 of 3 complete
Status: Phase 22 complete (3/3 plans, 6 tasks, 17 files); Phase 23 ready
Last activity: 2026-02-10 — Phase 22 Plan 03 executed (install skill, README, .gitignore)

Progress v2.1: [████████████████░░░░] 83% (5/6 phases) — Phase 23 pending

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
| 21 | Gemini CLI Adapter | Complete (4/4 plans, incl. gap closure) |
| 22 | Copilot CLI Adapter | Complete (3/3 plans) |
| 23 | Cross-Agent Discovery + Documentation | Ready |

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
- Install skill uses jq recursive merge (*) for settings.json to preserve existing user configuration
- Install skill excludes itself from global deployment (no need to install the installer)
- README provides three installation paths: automated skill, manual global, manual per-project
- Settings.json precedence documented with 5-level hierarchy
- Runtime jq walk() capability test instead of version string parsing (more portable)
- del()-based fallback redaction covers top level + one level deep for jq < 1.6
- perl preferred for ANSI stripping (CSI+OSC+SS2/SS3); sed fallback for minimal systems

### Phase 22 Decisions

- Single script with event type as $1 argument (matching Gemini adapter pattern, less code duplication)
- Runtime jq walk() capability test (same approach as Gemini adapter, more portable than version parsing)
- Perl preferred for ANSI stripping with sed fallback (CSI+OSC+SS2/SS3 coverage)
- del()-based fallback redaction for jq < 1.6 (top level + one level deep)
- Session file cleanup only on user_exit or complete reasons (preserves resumed sessions)
- No stdout output from hook script (Copilot ignores stdout for most events)
- Skills use .github/skills/ path (Copilot canonical, not .claude/skills/)
- Navigator agent is a separate .agent.md file with infer:true (unlike Gemini embedded in skill)
- No TOML command files (Copilot uses skills, not TOML commands)
- Command-equivalent instructions embedded in memory-query skill for search/recent/context
- Agent uses tools: execute, read, search (Copilot CLI tool names)
- plugin.json uses minimal fields (name, version, description, author, repository)
- Install skill copies hook config directly (no settings.json merge -- Copilot uses standalone .github/hooks/*.json)
- Three installation paths: plugin install, install skill, manual per-project
- Install skill excludes itself from target project deployment
- README documents all Copilot-specific gaps: AssistantResponse, SubagentStart/Stop, Bug #991 per-prompt
- Adapter comparison table covers Copilot vs Gemini vs Claude Code across 11 dimensions

## Next Steps

1. Execute Phase 23 (Cross-Agent Discovery + Documentation)
2. Complete v2.1 milestone

## Phase 21 Summary

**Completed:** 2026-02-10

**Artifacts created:**
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` -- Shell hook handler (185 lines)
- `plugins/memory-gemini-adapter/.gemini/settings.json` -- Hook configuration template
- `plugins/memory-gemini-adapter/.gemini/commands/memory-search.toml` -- Search slash command
- `plugins/memory-gemini-adapter/.gemini/commands/memory-recent.toml` -- Recent slash command
- `plugins/memory-gemini-adapter/.gemini/commands/memory-context.toml` -- Context slash command
- `plugins/memory-gemini-adapter/.gemini/skills/memory-query/SKILL.md` -- Core query + Navigator (508 lines)
- `plugins/memory-gemini-adapter/.gemini/skills/retrieval-policy/SKILL.md` -- Tier detection
- `plugins/memory-gemini-adapter/.gemini/skills/topic-graph/SKILL.md` -- Topic exploration
- `plugins/memory-gemini-adapter/.gemini/skills/bm25-search/SKILL.md` -- Keyword search
- `plugins/memory-gemini-adapter/.gemini/skills/vector-search/SKILL.md` -- Semantic search
- `plugins/memory-gemini-adapter/.gemini/skills/memory-gemini-install/SKILL.md` -- Install skill (472 lines)
- `plugins/memory-gemini-adapter/README.md` -- Complete documentation (453 lines)
- `plugins/memory-gemini-adapter/.gitignore` -- OS/editor ignores

**Plans:** 4 plans (3 core + 1 gap closure), 8 tasks, 16 files
**Verification:** All must-haves passed across all 4 plans

**Gap closure (Plan 04):** Fixed 3 UAT findings -- jq 1.5 compat (del-based fallback), perl ANSI stripping (CSI+OSC), per-project path rewriting docs

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

## Phase 22 Summary

**Completed:** 2026-02-10

**Artifacts created:**
- `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` -- Shell hook handler (238 lines)
- `plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json` -- Hook configuration (45 lines)
- `plugins/memory-copilot-adapter/.github/skills/memory-query/SKILL.md` -- Core query + commands (474 lines)
- `plugins/memory-copilot-adapter/.github/skills/retrieval-policy/SKILL.md` -- Tier detection
- `plugins/memory-copilot-adapter/.github/skills/topic-graph/SKILL.md` -- Topic exploration
- `plugins/memory-copilot-adapter/.github/skills/bm25-search/SKILL.md` -- Keyword search
- `plugins/memory-copilot-adapter/.github/skills/vector-search/SKILL.md` -- Semantic search
- `plugins/memory-copilot-adapter/.github/agents/memory-navigator.agent.md` -- Navigator agent (249 lines)
- `plugins/memory-copilot-adapter/.github/skills/memory-copilot-install/SKILL.md` -- Install skill (414 lines)
- `plugins/memory-copilot-adapter/README.md` -- Complete documentation (448 lines)
- `plugins/memory-copilot-adapter/plugin.json` -- Plugin manifest
- `plugins/memory-copilot-adapter/.gitignore` -- OS/editor ignores

**Plans:** 3 plans, 6 tasks, 17 files
**Verification:** All must-haves passed across all 3 plans

---
*Updated: 2026-02-10 after Phase 22 Plan 03 execution (install skill, README, .gitignore)*
