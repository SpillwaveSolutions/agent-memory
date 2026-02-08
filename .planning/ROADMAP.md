# Roadmap: v2.1 Multi-Agent Ecosystem

**Goal:** Extend Agent Memory to work across the AI agent ecosystem with full Claude parity.

**Milestone start:** 2026-02-08
**Phases:** 18-23 (continuing from v2.0)

---

## Phase 18: Agent Tagging Infrastructure

**Goal:** Add agent identifier to events and build adapter SDK foundation.

**Scope:**
- Add `agent` field to Event proto and storage layer
- Create adapter trait defining common interface
- Implement event normalization for multi-agent ingest
- Add `--agent` filter to query commands
- Update TOC nodes to track contributing agents

**Requirements:** R4.1.1, R4.1.2, R4.1.3, R4.2.2, R5.2.1, R5.2.2, R5.2.3

**Files to modify:**
- `proto/memory.proto` — Event message, query filters
- `crates/memory-core/src/models/` — Event model
- `crates/memory-storage/src/` — Storage layer agent support
- `crates/memory-daemon/src/` — CLI filter support
- New: `crates/memory-adapters/` — Adapter SDK crate

**Definition of done:**
- [ ] Events can be ingested with agent identifier
- [ ] Queries filter by agent when `--agent` specified
- [ ] Default queries return all agents
- [ ] Adapter trait compiles and documents interface

---

## Phase 19: OpenCode Commands and Skills

**Goal:** Create OpenCode plugin with commands, skills, and agent definition.

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
- `plugins/memory-opencode-plugin/.opencode/agent/memory-navigator.md`

**Definition of done:**
- [ ] Commands work in OpenCode with `$ARGUMENTS` substitution
- [ ] Skills load with YAML frontmatter
- [ ] Agent activates on trigger patterns
- [ ] Plugin README documents installation

---

## Phase 20: OpenCode Event Capture + Unified Queries

**Goal:** Capture OpenCode sessions and enable cross-agent queries.

**Scope:**
- Implement session lifecycle hooks for OpenCode
- Automatic `agent:opencode` tagging on ingest
- Unified query results across agents
- Agent-aware result ranking

**Requirements:** R1.4.1-R1.4.4, R4.2.1, R4.2.3

**Files to modify/create:**
- `plugins/memory-opencode-plugin/.opencode/hooks/` — Lifecycle hooks
- `crates/memory-daemon/src/retrieval/` — Unified query support
- `crates/memory-daemon/src/ingest/` — Agent detection

**Definition of done:**
- [ ] OpenCode sessions auto-ingest with agent tag
- [ ] `memory-daemon query search` returns multi-agent results
- [ ] Results show source agent in output
- [ ] Ranking considers agent affinity (optional)

---

## Phase 21: Gemini CLI Adapter

**Goal:** Create Gemini CLI hook adapter with full Claude parity.

**Scope:**
- Research Gemini CLI hook format and lifecycle
- Create hook configuration for session capture
- Port commands to Gemini-compatible format
- Implement `agent:gemini` tagging

**Requirements:** R2.1.1-R2.1.5, R2.2.1-R2.2.4, R2.3.1-R2.3.3

**Files to create:**
- `plugins/memory-gemini-adapter/` — Adapter plugin
- `plugins/memory-gemini-adapter/hooks/` — Gemini hook configs
- `plugins/memory-gemini-adapter/scripts/` — CLI wrappers
- `docs/adapters/gemini-installation.md` — Setup guide

**Definition of done:**
- [ ] Gemini sessions captured with `agent:gemini` tag
- [ ] Commands work via Gemini interface
- [ ] Cross-agent queries include Gemini memories
- [ ] Installation guide tested on fresh system

---

## Phase 22: Copilot CLI Adapter

**Goal:** Create GitHub Copilot CLI hook adapter with full Claude parity.

**Scope:**
- Research Copilot CLI hook format and lifecycle
- Create hook configuration for session capture
- Port commands to Copilot-compatible format
- Implement `agent:copilot` tagging

**Requirements:** R3.1.1-R3.1.3, R3.2.1-R3.2.3, R3.3.1-R3.3.3

**Files to create:**
- `plugins/memory-copilot-adapter/` — Adapter plugin
- `plugins/memory-copilot-adapter/hooks/` — Copilot hook configs
- `plugins/memory-copilot-adapter/scripts/` — CLI wrappers
- `docs/adapters/copilot-installation.md` — Setup guide

**Definition of done:**
- [ ] Copilot sessions captured with `agent:copilot` tag
- [ ] Commands work via Copilot interface
- [ ] Cross-agent queries include Copilot memories
- [ ] Installation guide tested on fresh system

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

---

## Dependencies

```
Phase 18 (Infrastructure)
    │
    ├──► Phase 19 (OpenCode Commands)
    │        │
    │        └──► Phase 20 (OpenCode Capture + Unified)
    │
    ├──► Phase 21 (Gemini Adapter) ─┐
    │                                │
    └──► Phase 22 (Copilot Adapter) ─┼──► Phase 23 (Discovery + Docs)
                                     │
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
