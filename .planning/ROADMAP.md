# Roadmap: v2.1 Multi-Agent Ecosystem

**Goal:** Extend Agent Memory to work across the AI agent ecosystem with full Claude parity.

**Milestone start:** 2026-02-08
**Phases:** 18-23 (continuing from v2.0)

---

## Phase 18: Agent Tagging Infrastructure

**Goal:** Add agent identifier to events and build adapter SDK foundation.

**Plans:** 4 plans in 3 waves

Plans:
- [x] 18-01-PLAN.md — Add agent field to Event proto and Rust types
- [x] 18-02-PLAN.md — Create memory-adapters crate with AgentAdapter trait
- [x] 18-03-PLAN.md — Add contributing_agents to TocNode, --agent CLI filter
- [x] 18-04-PLAN.md — Wire agent through ingest and query paths

**Scope:**
- Add `agent` field to Event proto and storage layer
- Create adapter trait defining common interface
- Implement event normalization for multi-agent ingest
- Add `--agent` filter to query commands
- Update TOC nodes to track contributing agents

**Requirements:** R4.1.1, R4.1.2, R4.1.3, R4.2.2, R5.2.1, R5.2.2, R5.2.3

**Files to modify:**
- `proto/memory.proto` — Event message, query filters
- `crates/memory-types/src/` — Event and TocNode models
- `crates/memory-daemon/src/` — CLI filter support
- `crates/memory-service/src/` — Ingest handler
- `crates/memory-retrieval/src/` — Query filtering types
- New: `crates/memory-adapters/` — Adapter SDK crate

**Definition of done:**
- [x] Events can be ingested with agent identifier
- [x] Queries filter by agent when `--agent` specified
- [x] Default queries return all agents
- [x] Adapter trait compiles and documents interface

---

## Phase 19: OpenCode Commands and Skills

**Goal:** Create OpenCode plugin with commands, skills, and agent definition.

**Plans:** 5 plans in 2 waves

Plans:
- [x] 19-01-PLAN.md — Port commands (memory-search, memory-recent, memory-context) with $ARGUMENTS
- [x] 19-02-PLAN.md — Port core skills (memory-query, retrieval-policy, topic-graph)
- [x] 19-03-PLAN.md — Port teleport skills (bm25-search, vector-search)
- [x] 19-04-PLAN.md — Create memory-navigator agent with OpenCode format
- [x] 19-05-PLAN.md — Create plugin README and documentation

**Scope:**
- Port `/memory-search`, `/memory-recent`, `/memory-context` to OpenCode format
- Port memory-query, retrieval-policy, topic-graph, bm25-search, vector-search skills
- Port memory-navigator agent with trigger patterns
- Create `.opencode/` plugin structure

**Requirements:** R1.1.1-R1.1.5, R1.2.1-R1.2.7, R1.3.1-R1.3.4

**Files to create:**
- `plugins/memory-opencode-plugin/.opencode/command/memory-search.md`
- `plugins/memory-opencode-plugin/.opencode/command/memory-recent.md`
- `plugins/memory-opencode-plugin/.opencode/command/memory-context.md`
- `plugins/memory-opencode-plugin/.opencode/skill/memory-query/SKILL.md`
- `plugins/memory-opencode-plugin/.opencode/skill/retrieval-policy/SKILL.md`
- `plugins/memory-opencode-plugin/.opencode/skill/topic-graph/SKILL.md`
- `plugins/memory-opencode-plugin/.opencode/skill/bm25-search/SKILL.md`
- `plugins/memory-opencode-plugin/.opencode/skill/vector-search/SKILL.md`
- `plugins/memory-opencode-plugin/.opencode/agents/memory-navigator.md`

**Definition of done:**
- [x] Commands work in OpenCode with `$ARGUMENTS` substitution
- [x] Skills load with YAML frontmatter
- [x] Agent activates on trigger patterns
- [x] Plugin README documents installation

**Completed:** 2026-02-09

---

## Phase 20: OpenCode Event Capture + Unified Queries

**Goal:** Capture OpenCode sessions and enable cross-agent queries.

**Plans:** 3 plans in 2 waves

Plans:
- [x] 20-01-PLAN.md — Wire agent field through ingest-to-retrieval pipeline
- [x] 20-02-PLAN.md — Create OpenCode TypeScript event capture plugin
- [x] 20-03-PLAN.md — Add agent display to CLI output and update plugin docs

**Scope:**
- Implement session lifecycle hooks for OpenCode
- Automatic `agent:opencode` tagging on ingest
- Unified query results across agents
- Agent-aware result ranking

**Requirements:** R1.4.1-R1.4.4, R4.2.1, R4.2.3

**Files to modify/create:**
- `crates/memory-ingest/src/main.rs` — Agent field on CchEvent
- `crates/memory-client/src/hook_mapping.rs` — Agent propagation in HookEvent
- `crates/memory-service/src/retrieval.rs` — Populate RetrievalResult.agent
- `crates/memory-daemon/src/commands.rs` — Agent display in CLI output
- `plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` — Event capture plugin
- `plugins/memory-opencode-plugin/README.md` — Event capture documentation

**Definition of done:**
- [x] OpenCode sessions auto-ingest with agent tag
- [x] `memory-daemon query search` returns multi-agent results
- [x] Results show source agent in output
- [x] Ranking considers agent affinity (optional — deferred per research)

**Completed:** 2026-02-09

---

## Phase 21: Gemini CLI Adapter

**Goal:** Create Gemini CLI hook adapter with full Claude parity.

**Plans:** 4 plans in 2 waves

Plans:
- [x] 21-01-PLAN.md — Hook capture script and settings.json configuration
- [x] 21-02-PLAN.md — TOML commands and skills with embedded navigator
- [x] 21-03-PLAN.md — Install skill, README, and documentation
- [x] 21-04-PLAN.md — Gap closure: jq 1.5 compatibility, ANSI stripping, per-project paths

**Scope:**
- Create hook handler shell script for Gemini lifecycle event capture
- Create settings.json hook configuration for 6 event types
- Port commands to TOML format with {{args}} substitution
- Copy skills with navigator logic embedded in memory-query
- Create install skill for automated setup
- Implement `agent:gemini` tagging

**Requirements:** R2.1.1-R2.1.5, R2.2.1-R2.2.4, R2.3.1-R2.3.3

**Files to create:**
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` — Hook handler script
- `plugins/memory-gemini-adapter/.gemini/settings.json` — Hook configuration
- `plugins/memory-gemini-adapter/.gemini/commands/` — TOML command files
- `plugins/memory-gemini-adapter/.gemini/skills/` — Skill directories
- `plugins/memory-gemini-adapter/README.md` — Setup guide

**Definition of done:**
- [x] Gemini sessions captured with `agent:gemini` tag
- [x] Commands work via Gemini interface
- [x] Cross-agent queries include Gemini memories
- [x] Installation guide tested on fresh system

**Completed:** 2026-02-10

---

## Phase 22: Copilot CLI Adapter

**Goal:** Create GitHub Copilot CLI hook adapter with full Claude parity.

**Plans:** 3 plans in 2 waves

Plans:
- [x] 22-01-PLAN.md — Hook capture script with session ID synthesis and hook config
- [x] 22-02-PLAN.md — Skills, navigator agent, and plugin manifest
- [x] 22-03-PLAN.md — Install skill, README, and documentation

**Scope:**
- Create hook handler shell script with session ID synthesis (Copilot does not provide session_id)
- Create .github/hooks/memory-hooks.json configuration for 5 event types
- Create SKILL.md skills (Copilot uses skills, not TOML commands)
- Create .agent.md navigator agent (Copilot supports proper agent files)
- Create plugin.json manifest for /plugin install support
- Create install skill for automated per-project setup
- Implement `agent:copilot` tagging

**Requirements:** R3.1.1-R3.1.3, R3.2.1-R3.2.3, R3.3.1-R3.3.3

**Files to create:**
- `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` — Hook handler with session ID synthesis
- `plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json` — Hook configuration
- `plugins/memory-copilot-adapter/.github/skills/` — Skill directories (5 skills + install skill)
- `plugins/memory-copilot-adapter/.github/agents/memory-navigator.agent.md` — Navigator agent
- `plugins/memory-copilot-adapter/plugin.json` — Plugin manifest
- `plugins/memory-copilot-adapter/README.md` — Setup guide

**Definition of done:**
- [x] Copilot sessions captured with `agent:copilot` tag
- [x] Skills work via Copilot interface (auto-activated)
- [x] Navigator agent available via /agent or auto-inference
- [x] Cross-agent queries include Copilot memories
- [x] Installation guide covers plugin install + per-project + manual

**Completed:** 2026-02-10

---

## Phase 23: Cross-Agent Discovery + Documentation

**Goal:** Complete cross-agent features and comprehensive documentation.

**Scope:**
- List contributing agents command
- Agent activity timeline
- Cross-agent topic linking
- CLOD format support (optional)
- Adapter authoring guide
- Cross-agent usage documentation

**Requirements:** R4.3.1-R4.3.3, R5.1.1-R5.1.3, R5.3.1-R5.3.3

**Files to create/modify:**
- `crates/memory-daemon/src/cli/agents.rs` — Agent listing command
- `docs/adapters/cross-agent-guide.md` — Usage guide
- `docs/adapters/authoring-guide.md` — Adapter development guide
- `docs/adapters/clod-format.md` — CLOD specification (optional)

**Definition of done:**
- [ ] `memory-daemon agents list` shows all contributing agents
- [ ] Agent activity visible in query results
- [ ] Documentation covers all three adapters
- [ ] Plugin authoring guide enables community contributions

**Plans:** 3 plans in 2 waves

Plans:
- [ ] 23-01-PLAN.md — Agent insights RPC/CLI (list, activity)
- [ ] 23-02-PLAN.md — Agent-aware topics and CLI surfacing
- [ ] 23-03-PLAN.md — CLOD spec + converter CLI + cross-agent/authoring docs

---

## Dependencies

```
Phase 18 (Infrastructure)
    |
    |---> Phase 19 (OpenCode Commands)
    |        |
    |        \---> Phase 20 (OpenCode Capture + Unified)
    |
    |---> Phase 21 (Gemini Adapter) --\
    |                                  |
    \---> Phase 22 (Copilot Adapter) --+---> Phase 23 (Discovery + Docs)
                                       |
```

- Phase 19-22 can run in parallel after Phase 18
- Phase 23 depends on all adapters being functional

---

## Success Metrics

| Metric | Target |
|--------|--------|
| OpenCode command parity | 100% of Claude commands |
| Gemini adapter parity | 100% of Claude functionality |
| Copilot adapter parity | 100% of Claude functionality |
| Cross-agent query latency | <100ms additional overhead |
| Documentation completeness | Installation + usage for all adapters |

---

*Created: 2026-02-08*
*Milestone: v2.1 Multi-Agent Ecosystem*
