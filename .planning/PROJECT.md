# Agent Memory

## Current State

**Version:** v3.0 (In Progress)
**Status:** Building retrieval orchestration, CLI API, and benchmark suite

## Current Milestone: v3.0 Competitive Parity & Benchmarks

**Goal:** Close the three gaps that keep Agent-Memory from being the category leader: retrieval pipeline orchestration, a dead-simple CLI API, and a benchmark suite that produces a publishable LOCOMO score.

**Target features:**
- Retrieval Orchestrator crate (query expansion, RRF fusion, LLM reranking)
- Simple `memory` CLI binary (search, context, recall, add, timeline, summary)
- Benchmark suite with custom harness + LOCOMO adapter
- Positioning writeup (side quest, not a GSD phase)

**Previous version:** v2.7 (Shipped 2026-03-22) — Multi-runtime installer with 6 converters

**Spec reference:** `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md`
**Plan references:**
- `docs/superpowers/plans/2026-03-21-v3-phase-a-retrieval-orchestrator.md`
- `docs/superpowers/plans/2026-03-21-v3-phase-b-simple-cli-api.md`
- `docs/superpowers/plans/2026-03-21-v3-phase-c-benchmark-suite.md`

The system implements a complete 6-layer cognitive stack with control plane, multi-agent support, semantic dedup, retrieval quality filtering, multi-runtime installer, and comprehensive testing:
- Layer 0: Raw Events (RocksDB) — agent-tagged, dedup-aware (store-and-skip-outbox)
- Layer 1: TOC Hierarchy (time-based navigation) — contributing_agents tracking
- Layer 2: Agentic TOC Search (index-free, always works)
- Layer 3: Lexical Teleport (BM25/Tantivy)
- Layer 4: Semantic Teleport (Vector/HNSW) — also used for dedup similarity checks
- Layer 5: Conceptual Discovery (Topic Graph) — agent-filtered queries
- Layer 6: Ranking Policy (salience, usage, novelty, lifecycle) + StaleFilter (time-decay, supersession)
- Control: Retrieval Policy (intent routing, tier detection, fallbacks)
- Dedup: InFlightBuffer + HNSW composite gate, configurable threshold, fail-open
- Installer: memory-installer crate with RuntimeConverter trait, 6 converters, tool mapping tables
- Adapters: Claude Code, OpenCode, Gemini CLI, Copilot CLI, Codex CLI (via installer)
- Discovery: ListAgents, GetAgentActivity, agent-filtered topics
- Testing: 46 cargo E2E tests + 144 bats CLI tests across 5 CLIs
- CI/CD: Dedicated E2E job + CLI matrix report in GitHub Actions
- Setup: Quickstart, full guide, agent setup docs + 4 wizard-style setup skills
- Benchmarks: perf_bench harness with baseline metrics across all retrieval layers

~56,400 LOC Rust across 15 crates. memory-installer with 6 runtime converters. 46 E2E tests + 144 bats tests. Cross-CLI matrix report.

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

### Validated (v2.1 - Shipped 2026-02-10)

**Multi-Agent Ecosystem (v2.1)**
- [x] OpenCode plugin — 3 commands, 5 skills, navigator agent, event capture — v2.1
- [x] Gemini CLI adapter — hook handler, TOML commands, skills, install skill — v2.1
- [x] Copilot CLI adapter — hook handler, session synthesis, skills, navigator agent — v2.1
- [x] Agent-tagged events with Event.agent field and TocNode.contributing_agents — v2.1
- [x] Unified cross-agent queries (all agents by default, --agent filter) — v2.1
- [x] Agent discovery RPCs (ListAgents, GetAgentActivity) — v2.1
- [x] Agent-filtered topic queries (GetTopTopics with agent_filter) — v2.1
- [x] CLOD format spec and converter CLI (4 adapter generators) — v2.1
- [x] Cross-agent usage guide, adapter authoring guide, UPGRADING docs — v2.1

<details>
<summary>v2.0.0 Validated (Shipped 2026-02-07)</summary>

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

</details>

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

### Validated (v2.2 - Shipped 2026-02-11)

**Production Hardening (v2.2)**
- [x] All gRPC stub RPCs wired (GetRankingStatus, PruneVectorIndex, PruneBm25Index) — v2.2
- [x] ListAgents session_count fixed via event scanning — v2.2
- [x] Agent field on TeleportResult and VectorTeleportMatch — v2.2
- [x] 29 E2E tests across 7 files (pipeline, BM25, vector, topic, multi-agent, degradation, error paths) — v2.2
- [x] Dedicated E2E CI job in GitHub Actions with separate pass/fail reporting — v2.2
- [x] E2E tests run on every PR, required for merge via ci-success gate — v2.2

### Validated (v2.3 - Shipped 2026-02-12)

**Install & Setup Experience (v2.3)**
- [x] Quickstart, full guide, and agent setup documentation — v2.3
- [x] Four setup skills (install/configure/verify/troubleshoot) with wizard-style flows — v2.3
- [x] Performance benchmark harness with ingest, TOC, BM25, vector, topic graph latency — v2.3
- [x] Baseline metrics for all tier/mode combinations with p50/p90/p99 percentiles — v2.3

### Validated (v2.4 - Shipped 2026-03-05)

**Headless CLI Testing (v2.4)**
- [x] Shell-based E2E harness using bats-core with isolated workspaces per test — v2.4
- [x] Claude Code CLI headless tests (30 tests: smoke, hooks, pipeline, negative) — v2.4
- [x] Gemini CLI headless tests (28 tests) — v2.4
- [x] OpenCode CLI headless tests (25 tests) — v2.4
- [x] Copilot CLI headless tests (30 tests) — v2.4
- [x] Codex CLI adapter (commands + skills only, no hooks) — v2.4
- [x] Codex CLI headless tests (26 tests, hook tests skipped with annotation) — v2.4
- [x] Cross-CLI matrix report aggregating JUnit XML from all 5 CLIs — v2.4
- [x] CI integration with artifact retention on failure — v2.4

### Validated (v2.5 - Shipped 2026-03-10)

**Semantic Dedup & Retrieval Quality (v2.5)**
- [x] InFlightBuffer within-session dedup (256-entry buffer, 0.85 threshold) — v2.5
- [x] HNSW cross-session dedup via CompositeVectorIndex — v2.5
- [x] Store-and-skip-outbox for duplicates (append-only preserved) — v2.5
- [x] Structural events bypass dedup entirely — v2.5
- [x] Configurable similarity threshold via config.toml — v2.5
- [x] Fail-open dedup gate (embedding/search failures pass through) — v2.5
- [x] Time-decay stale filtering with configurable 14-day half-life — v2.5
- [x] Supersession detection for semantically similar newer content — v2.5
- [x] High-salience kind exemption from time-decay — v2.5
- [x] Configurable staleness parameters via config.toml — v2.5
- [x] 10 E2E tests proving dedup, stale filtering, and fail-open — v2.5

### Validated (v2.6 - Shipped 2026-03-16)

**Cognitive Retrieval (v2.6)**
- [x] BM25 wired into hybrid search handler and retrieval routing — v2.6
- [x] Salience scoring at write time (TOC nodes, Grips) — v2.6
- [x] Usage-based decay in retrieval ranking (access_count tracking) — v2.6
- [x] Vector index pruning via scheduler job — v2.6
- [x] BM25 lifecycle policy with level-filtered rebuild — v2.6
- [x] Admin RPCs for dedup metrics (buffer_size, events skipped) — v2.6
- [x] Ranking metrics exposure (salience distribution, usage stats) — v2.6
- [x] `deduplicated` field in IngestEventResponse — v2.6
- [x] Episode schema and RocksDB storage (CF_EPISODES) — v2.6
- [x] gRPC RPCs (StartEpisode, RecordAction, CompleteEpisode, GetSimilarEpisodes) — v2.6
- [x] Value-based retention (outcome score sweet spot) — v2.6
- [x] Retrieval integration for similar episode search — v2.6

### Validated (v2.7 - Shipped 2026-03-22)

**Multi-Runtime Portability (v2.7)**
- [x] Canonical plugin source from both `memory-query-plugin/` and `memory-setup-plugin/` directories — v2.7
- [x] `memory-installer` crate with CLI, plugin parser, RuntimeConverter trait — v2.7
- [x] Centralized tool mapping tables (11 tools × 6 runtimes) — v2.7
- [x] Claude converter (pass-through with path rewriting) — v2.7
- [x] Gemini converter (TOML format, tool mapping, settings.json hook merge) — v2.7
- [x] Codex converter (commands→skills, AGENTS.md generation) — v2.7
- [x] Copilot converter (skill format, .agent.md, hook scripts) — v2.7
- [x] Generic skills converter (runtime-agnostic, user-specified directory) — v2.7
- [x] Hook conversion with per-runtime formats and fail-open scripts — v2.7
- [x] 7 E2E integration tests for full converter pipeline — v2.7
- [x] Old adapter directories archived with README stubs — v2.7

### Known Gaps (v2.7)

- OC-01–06: OpenCode converter is a stub (methods return empty). Deferred to v3.0.

### Deferred / Future

- Cross-project unified memory
- Per-agent dedup scoping
- Consolidation hook (extract durable knowledge from events, needs NLP/LLM)
- True daemonization (double-fork on Unix)
- API-based summarizer wiring (OpenAI/Anthropic)

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
| Adapter-per-agent plugins | Each agent gets its own plugin dir with native format | ✓ Validated v2.1 |
| Fail-open shell hooks | trap ERR EXIT, background processes, exit 0 always | ✓ Validated v2.1 |
| Agent field via serde(default) | Backward-compatible JSON parsing for agent tags | ✓ Validated v2.1 |
| O(k) agent discovery | Aggregate from TocNode.contributing_agents, not O(n) events | ✓ Validated v2.1 |
| CLOD as internal format | TOML-based portable command definition, not external standard | ✓ Validated v2.1 |
| Skills portable across agents | Same SKILL.md works in Claude/OpenCode/Copilot | ✓ Validated v2.1 |
| E2E tests via cargo test | Standard test infra, no separate framework | ✓ Validated v2.2 |
| Direct handler testing | tonic::Request without gRPC server; faster, simpler | ✓ Validated v2.2 |
| Dedicated E2E CI job | Separate from unit tests; clear reporting per CI-03 | ✓ Validated v2.2 |
| BM25 prune report-only | TeleportSearcher is read-only; deletion needs SearchIndexer | — Design decision v2.2 |
| Wizard-style setup skills | Confirm before edits, verification-only commands | ✓ Validated v2.3 |
| perf_bench as binary | Standalone binary in e2e-tests crate; not unit tests | ✓ Validated v2.3 |
| Baseline JSON with thresholds | warning/severe thresholds per step for regression detection | ✓ Validated v2.3 |
| Shell-first E2E harness | Fits CLI testing model; bats-core 1.12 | ✓ Validated v2.4 |
| Real CLI processes | Spawn actual CLIs headless, not simulated behavior | ✓ Validated v2.4 |
| One phase per CLI | Each CLI gets own harness phase; Claude Code first builds framework | ✓ Validated v2.4 |
| Keep both test layers | Existing cargo E2E tests stay; CLI harness is separate layer | ✓ Validated v2.4 |
| Codex adapter (no hooks) | Codex lacks hook support; skip hook-dependent tests | ✓ Validated v2.4 |
| Direct CchEvent ingest for hookless CLIs | OpenCode/Codex use pre-translated events for pipeline tests | ✓ Validated v2.4 |
| Cross-CLI matrix report | Python3 xml.etree parses JUnit XML; worst-case merge for multi-OS | ✓ Validated v2.4 |
| Store-and-skip-outbox for dedup | Preserves append-only invariant; duplicates stored but not indexed | ✓ Validated v2.5 |
| InFlightBuffer as primary dedup source | HNSW contains TOC nodes not raw events; buffer catches within-session | ✓ Validated v2.5 |
| Default similarity threshold 0.85 | Conservative for all-MiniLM-L6-v2; configurable via config.toml | ✓ Validated v2.5 |
| Structural events bypass dedup | Session markers must always be indexed for TOC integrity | ✓ Validated v2.5 |
| Max stale penalty 30% | Bounded to prevent score collapse with existing ranking layers | ✓ Validated v2.5 |
| High-salience kind exemption | Constraints/Definitions/Procedures are timeless; no decay | ✓ Validated v2.5 |
| CompositeVectorIndex for cross-session dedup | Searches both HNSW and InFlightBuffer, returns highest score | ✓ Validated v2.5 |
| std::sync::RwLock for InFlightBuffer | Operations are sub-microsecond; tokio RwLock overhead unnecessary | ✓ Validated v2.5 |

| Canonical source: keep two plugin dirs | User decision; installer reads from both via discovery manifest | ✓ Validated v2.7 |
| RuntimeConverter trait with Box<dyn> dispatch | Extensible without enum changes; each runtime is independent impl | ✓ Validated v2.7 |
| format!-based YAML/TOML emitters | No serde_yaml dependency; full control over quoting and block scalars | ✓ Validated v2.7 |
| Match expressions for tool maps | Compile-time exhaustive, zero overhead vs HashMap | ✓ Validated v2.7 |
| Write-interceptor for dry-run | Single write_files() handles dry-run; converters produce data only | ✓ Validated v2.7 |
| Hooks generated per-converter | Each runtime's hook mechanism too different for canonical YAML format | ✓ Validated v2.7 |
| OpenCode converter as stub | Full impl deferred; OpenCode runtime format still evolving | — Deferred v2.7 |
| Archive adapters (not delete) | One release cycle before removal; README stubs redirect to installer | ✓ Validated v2.7 |

---
*Last updated: 2026-03-22 after v3.0 milestone start*
