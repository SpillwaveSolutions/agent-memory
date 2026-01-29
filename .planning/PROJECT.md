# Agent Memory

## What This Is

A local, append-only conversational memory system for AI agents (Claude Code, OpenCode, Gemini CLI, GitHub Copilot CLI) that supports agentic search via a permanent hierarchical Table of Contents (TOC), grounded in time-based navigation. The TOC acts as a Progressive Disclosure Architecture: the agent always starts with summaries and navigates downward only when needed. Indexes (vector/BM25) are accelerators, not dependencies.

## Core Value

**An agent can answer "what were we talking about last week?" without scanning everything.**

Time-based TOC navigation beats brute-force search. If everything else fails, the TOC + time hierarchy must work.

## Requirements

### Validated

(None yet — ship to validate)

### Active

**Phase 0 — MVP (Minimum Valuable Memory)**
- [ ] Append-only event storage in RocksDB
- [ ] Time-based TOC hierarchy (Year → Month → Week → Day → Segment)
- [ ] Summaries at each TOC node (title, bullets, keywords)
- [ ] Agent can navigate TOC via gRPC to find answers
- [ ] Deterministic drill-down path to raw events
- [ ] Hook handler client (Rust, cross-platform) ingests events via gRPC
- [ ] End-to-end query: "what did we discuss yesterday?" returns summary-based answer

**Phase 1 — Quality & Trust**
- [ ] Grips (excerpt + event pointer) anchor TOC summaries
- [ ] Provenance: every bullet links to source evidence
- [ ] Better segmentation (token-aware, topic shift boundaries)

**Phase 2 — Teleport (Indexes as Accelerators)**
- [ ] BM25 teleport via Tantivy (embedded)
- [ ] Vector teleport via local HNSW
- [ ] Outbox-driven index ingestion (rebuildable)
- [ ] Teleports return TOC node IDs or grip pointers, never content

**Phase 3 — Resilience (Heavy Scan Fallback)**
- [ ] Parallel scan by time bucket (4 workers)
- [ ] Range-limited by TOC (month/week)
- [ ] Produces grips as outputs

**Phase 4 — Intelligence (Deferred)**
- [ ] Learned topic TOC nodes
- [ ] Summary refinement
- [ ] Confidence scoring

### Out of Scope

- Graph database — TOC is a tree stored as records, no graph DB needed
- Multi-tenant concerns — single agent, local deployment
- Deletes / mutable history — append-only truth
- "Search everything all the time" — agentic navigation, not brute-force
- Premature optimization — teleports come in Phase 2
- HTTP server — gRPC only
- MCP integration — hooks are passive listeners, no token overhead

## Context

**Ingestion via Hooks (Passive Capture)**

Conversations are captured via agent hooks (Claude Code, OpenCode, Gemini CLI, GitHub Copilot CLI). Hook handlers send events to the daemon via gRPC. This is zero-token-overhead passive listening.

Event types (1:1 from hooks):
| Hook Event | Memory Event |
|------------|--------------|
| SessionStart | session_start |
| UserPromptSubmit | user_message |
| PostToolUse | tool_result |
| Stop | assistant_stop |
| SubagentStart | subagent_start |
| SubagentStop | subagent_stop |
| SessionEnd | session_end |

**Query Path**

CLI client and agent skill query the daemon. Agent receives TOC navigation tools:
- `get_toc_root` — top-level time periods
- `get_node(node_id)` — drill into specific period
- `get_events(time_range)` — raw events (last resort)
- `expand_grip(grip_id)` — context around excerpt
- `teleport_query(query)` — Phase 2+ index jump

**Related Work**

`code_agent_context_hooks` repo contains working hook handlers for Claude Code. This memory system is the backend those hooks feed into.

## Constraints

- **Language**: Rust — single binary, fast scans, predictable memory
- **API**: gRPC only (tonic/prost) — no HTTP server
- **Storage**: RocksDB — embedded, fast range scans, column families
- **Deployment**: Standalone daemon, per-project stores
- **Platforms**: macOS, Linux, Windows (cross-compile)
- **Multi-agent**: Configurable — unified store (events tagged) or separate stores
- **Summarizer**: Pluggable trait — API (Claude/GPT) or local inference
- **Config**: Layered — defaults → config file (~/.config/agent-memory/) → env vars → CLI flags
- **Testing**: Unit + Integration + Property-based + IQ/OQ

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| TOC as primary navigation | Agentic search beats brute-force; indexes are disposable | — Pending |
| Append-only storage | Immutable truth, no deletion complexity | — Pending |
| Hooks for ingestion | Zero token overhead, works across agents | — Pending |
| Per-project stores first | Simpler mental model, namespace for unified later | — Pending |
| Time-only TOC for MVP | Topics deferred to Phase 4, time is sufficient for v1 | — Pending |
| gRPC only (no HTTP) | Clean contract, no framework churn | — Pending |
| Pluggable summarizer | Start with API, swap to local later | — Pending |

---
*Last updated: 2026-01-29 after initialization*
