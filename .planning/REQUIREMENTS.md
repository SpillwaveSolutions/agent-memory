# Requirements: Agent Memory

**Defined:** 2026-01-29
**Core Value:** Agent can answer "what were we talking about?" without scanning everything

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Storage

- [ ] **STOR-01**: Append-only event storage with time-prefixed keys (`evt:{ts}:{ulid}`)
- [ ] **STOR-02**: RocksDB column family isolation (events, toc_nodes, toc_latest, grips, outbox, checkpoints)
- [ ] **STOR-03**: Checkpoint-based crash recovery for background jobs
- [ ] **STOR-04**: Per-project RocksDB instances (one store per project directory)
- [ ] **STOR-05**: FIFO or Universal compaction for append-only workload
- [ ] **STOR-06**: Configurable multi-agent mode (unified store with tags OR separate stores)

### TOC Hierarchy

- [ ] **TOC-01**: Full time hierarchy (Year -> Month -> Week -> Day -> Segment)
- [ ] **TOC-02**: TOC nodes store title, bullets, keywords, child_node_ids
- [ ] **TOC-03**: Segment creation on time threshold (30 min) or token threshold (4K)
- [ ] **TOC-04**: Segment overlap for context continuity (5 min or 500 tokens)
- [ ] **TOC-05**: Day/Week/Month rollup jobs with checkpointing
- [ ] **TOC-06**: Versioned TOC nodes (append new version, don't mutate)

### Grips (Provenance)

- [ ] **GRIP-01**: Grip struct with excerpt, event_id_start, event_id_end, timestamp, source
- [ ] **GRIP-02**: TOC node bullets link to supporting grips
- [ ] **GRIP-03**: Grips stored in dedicated column family
- [ ] **GRIP-04**: ExpandGrip returns context events around excerpt

### Summarization

- [ ] **SUMM-01**: Pluggable Summarizer trait (async, supports API and local LLM)
- [ ] **SUMM-02**: Summarizer generates title, bullets, keywords from events
- [ ] **SUMM-03**: Summarizer extracts grips as evidence for bullets
- [ ] **SUMM-04**: Rollup summarizer aggregates child node summaries

### Ingestion

- [ ] **ING-01**: gRPC IngestEvent RPC accepts Event message
- [ ] **ING-02**: Event includes session_id, timestamp, role, text, metadata
- [ ] **ING-03**: Idempotent writes using event_id as key
- [ ] **ING-04**: Source timestamp used for ordering (not ingestion time)
- [ ] **ING-05**: Outbox entry written atomically with event (for future index updates)

### Query

- [ ] **QRY-01**: GetTocRoot RPC returns top-level time nodes
- [ ] **QRY-02**: GetNode RPC returns node with children and summary
- [ ] **QRY-03**: BrowseToc RPC supports pagination of children
- [ ] **QRY-04**: GetEvents RPC retrieves raw events by time range
- [ ] **QRY-05**: ExpandGrip RPC retrieves context around grip excerpt

### gRPC Service

- [ ] **GRPC-01**: Memory daemon exposes gRPC service (tonic)
- [ ] **GRPC-02**: Proto definitions for Event, TocNode, Grip, all RPCs
- [ ] **GRPC-03**: Health check endpoint (tonic-health)
- [ ] **GRPC-04**: Reflection endpoint for debugging (tonic-reflection)

### Hook Handler Integration

- [ ] **HOOK-01**: code_agent_context_hooks repo provides hook handlers
- [ ] **HOOK-02**: Hook handlers call daemon's IngestEvent RPC
- [ ] **HOOK-03**: Event types map 1:1 from hook events (SessionStart, UserPromptSubmit, PostToolUse, Stop, etc.)

### Configuration

- [ ] **CFG-01**: Layered config: defaults -> config file -> env vars -> CLI flags
- [ ] **CFG-02**: Config includes: db_path, grpc_port, summarizer settings
- [ ] **CFG-03**: Config file location: ~/.config/agent-memory/config.toml

### CLI

- [ ] **CLI-01**: Memory daemon binary with start/stop/status commands
- [ ] **CLI-02**: Query CLI for manual TOC navigation and testing
- [ ] **CLI-03**: Admin commands: rebuild-toc, compact, status

## v2 Requirements

Phase 7 (CCH Integration) and future enhancements.

### CCH Integration (Phase 7)

- [ ] **CCH-01**: Memory-ingest binary that CCH can invoke via `run` action
- [ ] **CCH-02**: Event mapping from CCH events to memory events (session-start → session_start, user-prompt → user_message, post-tool-use → tool_result, session-end → session_end)
- [ ] **CCH-03**: hooks.yaml template for agent-memory integration

### Agentic Memory Query Skill (Phase 7)

- [ ] **SKILL-01**: Claude Code skill with commands: /memory-search, /memory-recent, /memory-context
- [ ] **SKILL-02**: Skill uses memory-client library to communicate with daemon
- [ ] **SKILL-03**: Skill navigates TOC, expands grips, formats results for agent context

### Teleport Indexes

- **TELE-01**: BM25 teleport index via Tantivy (embedded)
- **TELE-02**: Vector teleport index via HNSW (embedded)
- **TELE-03**: Outbox relay consumes outbox entries, updates indexes
- **TELE-04**: TeleportQuery RPC searches BM25 and/or vector, returns node_ids/grip_ids
- **TELE-05**: Index rebuild command from outbox or TOC
- **TELE-06**: IndexStatus RPC reports index health
- **TELE-07**: GetTeleportStatus RPC returns config & health for agent skill discovery (teleport is optional)

### Heavy Scan Fallback

- **SCAN-01**: Parallel scan by time bucket (4 workers)
- **SCAN-02**: Range-limited by TOC bounds (month/week)
- **SCAN-03**: Produces grips as outputs

### Memory Ranking Enhancements (Phase 16)

- **RANK-01**: Salience scoring at write time for TOC nodes and Grips
- **RANK-02**: Usage tracking in separate CF (CF_USAGE_COUNTERS)
- **RANK-03**: Cache-first usage reads with LRU cache
- **RANK-04**: Novelty filtering (opt-in, disabled by default)
- **RANK-05**: Vector lifecycle automation per FR-08 retention rules
- **RANK-06**: BM25 lifecycle automation per FR-09 (disabled by default)
- **RANK-07**: Feature flags for all ranking enhancements
- **RANK-08**: Backward compatibility with v2.0.0 data
- **RANK-09**: Staged rollout support with master switch
- **RANK-10**: Config validation on startup

### Agent Retrieval Policy (Phase 17)

- **RETR-01**: Combined status check for all layer availability
- **RETR-02**: Tier detection algorithm (maps availability to tiers 1-5)
- **RETR-03**: Capability advertisement to skills
- **RETR-04**: Query intent classification (Explore/Answer/Locate/Time-boxed)
- **RETR-05**: Intent-aware routing to appropriate layers
- **RETR-06**: Time constraint extraction from queries
- **RETR-07**: Configuration-aware search (respects enabled layers)
- **RETR-08**: Graceful degradation on layer failure
- **RETR-09**: Partial result return on timeout
- **RETR-10**: Stop condition enforcement (max_depth, max_nodes, timeout)
- **RETR-11**: Timeout handling per intent type
- **RETR-12**: Scanning trigger conditions
- **RETR-13**: Tier/method reporting in results
- **RETR-14**: Fallback explanation in results
- **RETR-15**: Execution mode selection (Sequential/Parallel/Hybrid)
- **RETR-16**: Bounded fan-out for parallel execution
- **RETR-17**: Early stopping on sufficient results
- **RETR-18**: Rank merge across multiple layers
- **RETR-19**: Explainable arbitration for skill contracts

### Additional Hooks

- **HOOK-04**: OpenCode hook adapter
- **HOOK-05**: Gemini CLI hook adapter
- **HOOK-06**: GitHub Copilot CLI hook adapter

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Graph database | TOC is a tree stored as records; graph adds unnecessary complexity |
| Multi-tenant | Single agent, local deployment; no tenant isolation needed |
| Delete/update events | Append-only truth; corrections are new events |
| Vector search as primary | Time-based TOC navigation is primary; vector is accelerator |
| HTTP API | gRPC only; no REST/HTTP layer |
| MCP integration | Hooks are passive listeners; MCP consumes tokens |
| Automatic summarization on read | Summaries built during TOC construction, not query time |
| Real-time sync | Eventual consistency is sufficient |
| Cross-project memory | Per-project stores; sharing is explicit and deferred |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| STOR-01 | Phase 1 | ✅ Complete |
| STOR-02 | Phase 1 | ✅ Complete |
| STOR-03 | Phase 1 | ✅ Complete |
| STOR-04 | Phase 1 | ✅ Complete |
| STOR-05 | Phase 1 | ✅ Complete |
| STOR-06 | Phase 1 | ✅ Complete |
| ING-01 | Phase 1 | ✅ Complete |
| ING-02 | Phase 1 | ✅ Complete |
| ING-03 | Phase 1 | ✅ Complete |
| ING-04 | Phase 1 | ✅ Complete |
| ING-05 | Phase 1 | ✅ Complete |
| GRPC-01 | Phase 1 | ✅ Complete |
| GRPC-02 | Phase 1 | ✅ Complete |
| GRPC-03 | Phase 1 | ✅ Complete |
| GRPC-04 | Phase 1 | ✅ Complete |
| CFG-01 | Phase 1 | ✅ Complete |
| CFG-02 | Phase 1 | ✅ Complete |
| CFG-03 | Phase 1 | ✅ Complete |
| CLI-01 | Phase 1 | ✅ Complete |
| TOC-01 | Phase 2 | ✅ Complete |
| TOC-02 | Phase 2 | ✅ Complete |
| TOC-03 | Phase 2 | ✅ Complete |
| TOC-04 | Phase 2 | ✅ Complete |
| TOC-05 | Phase 2 | ✅ Complete |
| TOC-06 | Phase 2 | ✅ Complete |
| SUMM-01 | Phase 2 | ✅ Complete |
| SUMM-02 | Phase 2 | ✅ Complete |
| SUMM-04 | Phase 2 | ✅ Complete |
| GRIP-01 | Phase 3 | ✅ Complete |
| GRIP-02 | Phase 3 | ✅ Complete |
| GRIP-03 | Phase 3 | ✅ Complete |
| GRIP-04 | Phase 3 | ✅ Complete |
| SUMM-03 | Phase 3 | ✅ Complete |
| QRY-01 | Phase 4 | ✅ Complete |
| QRY-02 | Phase 4 | ✅ Complete |
| QRY-03 | Phase 4 | ✅ Complete |
| QRY-04 | Phase 4 | ✅ Complete |
| QRY-05 | Phase 4 | ✅ Complete |
| HOOK-01 | External | ✅ Complete |
| HOOK-02 | Phase 5 | ✅ Complete |
| HOOK-03 | Phase 5 | ✅ Complete |
| CLI-02 | Phase 5 | ✅ Complete |
| CLI-03 | Phase 5 | ✅ Complete |
| CCH-01 | Phase 7 | Pending |
| CCH-02 | Phase 7 | Pending |
| CCH-03 | Phase 7 | Pending |
| SKILL-01 | Phase 7 | Pending |
| SKILL-02 | Phase 7 | Pending |
| SKILL-03 | Phase 7 | Pending |

| TELE-01 | Phase 11 | ✅ Complete |
| TELE-04 | Phase 11 | ✅ Complete |
| TELE-05 | Phase 11 | ✅ Complete |
| TELE-06 | Phase 11 | ✅ Complete |
| TELE-07 | Phase 11 | ✅ Complete |
| TELE-02 | Phase 12 | ✅ Complete |
| TELE-03 | Phase 13 | ✅ Complete |
| RANK-01 | Phase 16 | ✅ Complete |
| RANK-02 | Phase 16 | ✅ Complete |
| RANK-03 | Phase 16 | ✅ Complete |
| RANK-04 | Phase 16 | ✅ Complete |
| RANK-05 | Phase 16 | ✅ Complete |
| RANK-06 | Phase 16 | ✅ Complete |
| RANK-07 | Phase 16 | ✅ Complete |
| RANK-08 | Phase 16 | ✅ Complete |
| RANK-09 | Phase 16 | ✅ Complete |
| RANK-10 | Phase 16 | ✅ Complete |
| RETR-01 | Phase 17 | ✅ Complete |
| RETR-02 | Phase 17 | ✅ Complete |
| RETR-03 | Phase 17 | ✅ Complete |
| RETR-04 | Phase 17 | ✅ Complete |
| RETR-05 | Phase 17 | ✅ Complete |
| RETR-06 | Phase 17 | ✅ Complete |
| RETR-07 | Phase 17 | ✅ Complete |
| RETR-08 | Phase 17 | ✅ Complete |
| RETR-09 | Phase 17 | ✅ Complete |
| RETR-10 | Phase 17 | ✅ Complete |
| RETR-11 | Phase 17 | ✅ Complete |
| RETR-12 | Phase 17 | ✅ Complete |
| RETR-13 | Phase 17 | ✅ Complete |
| RETR-14 | Phase 17 | ✅ Complete |
| RETR-15 | Phase 17 | ✅ Complete |
| RETR-16 | Phase 17 | ✅ Complete |
| RETR-17 | Phase 17 | ✅ Complete |
| RETR-18 | Phase 17 | ✅ Complete |
| RETR-19 | Phase 17 | ✅ Complete |

**Coverage:**
- v1 requirements: 42 total (all complete)
- v2 requirements: 6 new (Phase 7)
- Teleport requirements: 7 (Phases 11-13) - all complete
- Ranking requirements: 10 (Phase 16) - all complete
- Retrieval requirements: 19 (Phase 17) - all complete
- External (HOOK-01): 1 (complete)
- Total: 84 (77 complete, 7 pending Phase 7/CCH)

---
*Requirements defined: 2026-01-29*
*v1 milestone completed: 2026-01-30*
*Phase 7 requirements added: 2026-01-30*
*Phase 11-13 teleport requirements traced: 2026-02-01*
*Phase 16 (RANK-*) and Phase 17 (RETR-*) requirements added: 2026-02-05*
*Phase 11-17 requirements completed: 2026-02-05*
