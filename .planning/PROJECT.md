# Agent Memory

## Current State

**Version:** v2.0.0 (Shipped 2026-02-07)
**Status:** Full cognitive architecture complete

The system now implements a complete 6-layer cognitive stack with control plane:
- Layer 0: Raw Events (RocksDB)
- Layer 1: TOC Hierarchy (time-based navigation)
- Layer 2: Agentic TOC Search (index-free, always works)
- Layer 3: Lexical Teleport (BM25/Tantivy)
- Layer 4: Semantic Teleport (Vector/HNSW)
- Layer 5: Conceptual Discovery (Topic Graph)
- Layer 6: Ranking Policy (salience, usage, novelty, lifecycle)
- Control: Retrieval Policy (intent routing, tier detection, fallbacks)

## Current Milestone: v2.1 Multi-Agent Ecosystem

**Goal:** Extend Agent Memory to work across the AI agent ecosystem with full Claude parity.

**Target features:**
- OpenCode plugin (query commands, event capture, navigator agent)
- Gemini CLI hook adapter (event capture + plugin equivalent)
- GitHub Copilot CLI hook adapter (event capture + plugin equivalent)
- Cross-agent memory sharing (agent-tagged events, unified queries, filter-by-agent)

## What This Is

**Agent Memory is a cognitive architecture for agents** — not just a memory system.

A local, append-only conversational memory system for AI agents (Claude Code, OpenCode, Gemini CLI, GitHub Copilot CLI) that supports agentic search via a permanent hierarchical Table of Contents (TOC), grounded in time-based navigation. The TOC acts as a Progressive Disclosure Architecture: the agent always starts with summaries and navigates downward only when needed. Indexes (vector/BM25) are accelerators, not dependencies.

**See:** [Cognitive Architecture Manifesto](../docs/COGNITIVE_ARCHITECTURE.md) for the complete philosophy.

## Core Value

**An agent can answer "what were we talking about last week?" without scanning everything.**

Time-based TOC navigation beats brute-force search. If everything else fails, the TOC + time hierarchy must work.

### Progressive Disclosure Architecture (PDA)

The TOC implements **Progressive Disclosure Architecture** — the same pattern used in well-designed Agentic Skills. Just as a skill reveals complexity progressively, Agent Memory reveals conversation detail progressively:

| Agentic Skills | Agent Memory |
|----------------|--------------|
| Start simple, reveal options as needed | Start with summaries, reveal events as needed |
| Agent discovers capabilities through exploration | Agent discovers answers through navigation |
| Complexity hidden until required | Raw events hidden until required |

**The key insight: Agentic search beats brute-force scanning.**

Instead of loading thousands of events into context, an agent navigates:
1. **Year** → "2024: heavy focus on authentication" → drill down
2. **Week** → "Week 3: JWT implementation" → drill down
3. **Day** → "Thursday: token expiration debugging" → drill down
4. **Segment** → Summary bullets with grip links → expand grip
5. **Grip** → Raw event excerpt with full context → answer verified

This mirrors how humans search email: filter by date, scan subjects, open the relevant thread. The agent never reads everything — it uses summaries to navigate to exactly what it needs.

## Cognitive Architecture

Agent Memory implements a layered cognitive architecture:

| Layer | Capability | Purpose |
|-------|------------|---------|
| 0 | Raw Events | Immutable truth (RocksDB) |
| 1 | TOC Hierarchy | Time-based navigation |
| 2 | Agentic TOC Search | Index-free term matching (always works) |
| 3 | Lexical Teleport | BM25 keyword acceleration |
| 4 | Semantic Teleport | Vector embedding similarity |
| 5 | Conceptual Discovery | Topic graph enrichment |
| 6 | Ranking Policy | Salience, usage decay, novelty |
| Control | Retrieval Policy | Intent routing, tier detection, fallbacks |

**Key Principle:** Indexes are accelerators, not dependencies. If any index fails, the system degrades gracefully.

**Control Plane:** Skills are the executive function — they decide how to use capabilities, not the capabilities themselves.

**References:**
- [Cognitive Architecture Manifesto](../docs/COGNITIVE_ARCHITECTURE.md)
- [Agent Retrieval Policy PRD](../docs/prds/agent-retrieval-policy-prd.md)

## Requirements

### Validated (v2.0.0 - Shipped 2026-02-07)

**Cognitive Layers (v2.0)**
- [x] Background scheduler with Tokio cron, timezone handling, overlap policy — v2.0
- [x] Index-free agentic TOC search (Layer 2, always works) — v2.0
- [x] BM25 teleport via Tantivy (Layer 3) — v2.0
- [x] Vector teleport via usearch HNSW with local embeddings (Layer 4) — v2.0
- [x] Outbox-driven index ingestion (rebuildable) — v2.0
- [x] Topic graph memory with HDBSCAN clustering (Layer 5) — v2.0
- [x] Salience scoring at write time — v2.0
- [x] Usage tracking with cache-first reads — v2.0
- [x] Novelty filtering (opt-in) — v2.0
- [x] Index lifecycle automation — v2.0
- [x] Intent classification (Explore/Answer/Locate/TimeBoxed) — v2.0
- [x] Tier detection (5 capability tiers) — v2.0
- [x] Fallback chains with graceful degradation — v2.0
- [x] Explainability payload for skill contracts — v2.0

<details>
<summary>v1.0.0 Validated (Shipped 2026-01-30)</summary>

**Storage & Foundation**
- [x] Append-only event storage in RocksDB with time-prefixed keys
- [x] 6 column families: events, toc_nodes, toc_latest, grips, outbox, checkpoints
- [x] Checkpoint-based crash recovery for background jobs
- [x] Per-project RocksDB instances
- [x] Configurable multi-agent mode (unified store with tags OR separate stores)

**TOC Hierarchy**
- [x] Time-based TOC hierarchy (Year → Month → Week → Day → Segment)
- [x] TOC nodes store title, bullets, keywords, child_node_ids
- [x] Segment creation on time threshold (30 min) or token threshold (4K)
- [x] Segment overlap for context continuity (5 min or 500 tokens)
- [x] Day/Week/Month rollup jobs with checkpointing
- [x] Versioned TOC nodes (append new version, don't mutate)

**Grips (Provenance)**
- [x] Grip struct with excerpt, event_id_start, event_id_end, timestamp, source
- [x] TOC node bullets link to supporting grips
- [x] Grips stored in dedicated column family
- [x] ExpandGrip returns context events around excerpt

**Summarization**
- [x] Pluggable Summarizer trait (async, supports API and local LLM)
- [x] Summarizer generates title, bullets, keywords from events
- [x] Summarizer extracts grips as evidence for bullets
- [x] Rollup summarizer aggregates child node summaries

**gRPC Service & Query**
- [x] gRPC IngestEvent RPC accepts Event message
- [x] GetTocRoot, GetNode, BrowseToc RPCs for TOC navigation
- [x] GetEvents, ExpandGrip RPCs for event retrieval
- [x] Health check and reflection endpoints

**Integration**
- [x] Hook handlers call daemon's IngestEvent RPC
- [x] CCH integration via memory-ingest binary (fail-open)
- [x] Claude Code plugin with 3 commands and memory-navigator agent
- [x] Query CLI for manual TOC navigation
- [x] Admin CLI for rebuild-toc, compact, status

</details>

### Active (v2.1 Multi-Agent Ecosystem)

**OpenCode Plugin**
- [ ] Query commands (/memory-search, /memory-recent, /memory-context)
- [ ] Event capture (conversation events to daemon)
- [ ] Navigator agent for complex queries

**Gemini CLI Adapter**
- [ ] Hook handler for Gemini CLI events
- [ ] Event capture to daemon
- [ ] Plugin/skill equivalent for queries

**GitHub Copilot CLI Adapter**
- [ ] Hook handler for Copilot CLI events
- [ ] Event capture to daemon
- [ ] Plugin/skill equivalent for queries

**Cross-Agent Memory Sharing**
- [ ] Agent-tagged events (source agent in metadata)
- [ ] Unified query (see all agents by default)
- [ ] Filter-by-agent option for scoped queries

**Deferred (v2.2+)**
- Automated E2E tests in CI
- Performance benchmarks

### Out of Scope

- Graph database — TOC is a tree stored as records, no graph DB needed
- Multi-tenant concerns — single agent, local deployment
- Deletes / mutable history — append-only truth
- "Search everything all the time" — agentic navigation, not brute-force
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
- `teleport_query(query)` — BM25/vector index jump
- `classify_intent(query)` — intent classification
- `route_query(query)` — full retrieval with fallbacks

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
| TOC as primary navigation | Agentic search beats brute-force; indexes are disposable | ✓ Validated v1.0, v2.0 |
| Append-only storage | Immutable truth, no deletion complexity | ✓ Validated v1.0 |
| Hooks for ingestion | Zero token overhead, works across agents | ✓ Validated v1.0 |
| Per-project stores first | Simpler mental model, namespace for unified later | ✓ Validated v1.0 |
| gRPC only (no HTTP) | Clean contract, no framework churn | ✓ Validated v1.0, v2.0 |
| Pluggable summarizer | Start with API, swap to local later | ✓ Validated v1.0 |
| Fail-open CCH integration | Never block Claude if memory is down | ✓ Validated v1.0 |
| Indexes as accelerators | BM25/Vector are optional; TOC always works | ✓ Validated v2.0 |
| Local embeddings | all-MiniLM-L6-v2 via Candle; no API dependency | ✓ Validated v2.0 |
| Graceful degradation | Tier detection enables fallback chains | ✓ Validated v2.0 |
| Skills as control plane | Skills decide how to use layers; layers are passive | ✓ Validated v2.0 |

---
*Last updated: 2026-02-08 after v2.1 milestone initialization*
