# Roadmap: Agent Memory

## Overview

This roadmap delivers a **cognitive architecture for agents** — a local, append-only conversational memory system with TOC-based agentic navigation. The journey proceeds from foundational storage through TOC construction, grips for provenance, query capabilities, hook integration, and culminates in a layered cognitive stack. Each phase builds on the previous, delivering a coherent capability that can be verified independently.

### Cognitive Layer Stack

Phases are grouped by the cognitive layer they implement:

| Layer | Phases | Capability | Status |
|-------|--------|------------|--------|
| **Foundation** (0-1) | 1-6 | Events + TOC hierarchy | Complete |
| **Integration** | 7-10 | Plugins, hooks, scheduler | Complete |
| **Agentic Navigation** (2) | 10.5 | Index-free search (always works) | Complete |
| **Keyword Acceleration** (3) | 11 | BM25/Tantivy teleport | Complete |
| **Semantic Acceleration** (4) | 12 | Vector/HNSW teleport | Complete |
| **Index Lifecycle** | 13 | Outbox-driven index updates | Complete |
| **Conceptual Enrichment** (5) | 14 | Topic graph discovery | Complete |
| **Configuration UX** | 15 | Interactive wizard skills | Planned |

**See:** [Cognitive Architecture Manifesto](../docs/COGNITIVE_ARCHITECTURE.md)

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3, 4, 5, 6): Planned milestone work
- Decimal phases (e.g., 2.1): Urgent insertions if needed (marked with INSERTED)

- [x] **Phase 1: Foundation** - Storage layer, domain types, gRPC scaffolding, configuration, daemon binary
- [x] **Phase 2: TOC Building** - Segmentation, summarization, time hierarchy construction
- [x] **Phase 3: Grips & Provenance** - Excerpt storage, summary-to-grip linking, expand capability
- [x] **Phase 4: Query Layer** - Navigation RPCs for TOC traversal and event retrieval
- [x] **Phase 5: Integration** - Hook handler connection, query CLI, admin commands
- [x] **Phase 6: End-to-End Demo** - Full workflow validation from ingestion to query answer
- [x] **Phase 7: Agentic Memory Plugin** - Claude Code marketplace plugin with commands, agents, graded skill
- [x] **Phase 8: CCH Hook Integration** - Automatic event capture via CCH hooks
- [x] **Phase 9: Setup & Installer Plugin** - Interactive setup wizard plugin with commands and agents
- [x] **Phase 10: Background Scheduler** - In-process Tokio cron scheduler for TOC rollups and periodic jobs
- [x] **Phase 10.5: Agentic TOC Search** - Index-free search using TOC navigation with progressive disclosure (INSERTED)
- [x] **Phase 11: BM25 Teleport (Tantivy)** - Full-text search index for keyword-based teleportation to relevant TOC nodes
- [x] **Phase 12: Vector Teleport (HNSW)** - Semantic similarity search via local HNSW vector index
- [x] **Phase 13: Outbox Index Ingestion** - Event-driven index updates from outbox for rebuildable search indexes
- [x] **Phase 14: Topic Graph Memory** - Semantic topic extraction, time-decayed importance, topic relationships for conceptual discovery
- [ ] **Phase 15: Configuration Wizard Skills** - Interactive AskUserQuestion-based configuration wizards for storage, LLM, and multi-agent settings

## Phase Details

### Phase 1: Foundation
**Goal**: Establish the storage layer, domain types, gRPC service structure, and daemon binary so events can be ingested and persisted
**Depends on**: Nothing (first phase)
**Requirements**: STOR-01, STOR-02, STOR-03, STOR-04, STOR-05, STOR-06, ING-01, ING-02, ING-03, ING-04, ING-05, GRPC-01, GRPC-02, GRPC-03, GRPC-04, CFG-01, CFG-02, CFG-03, CLI-01
**Success Criteria** (what must be TRUE):
  1. Daemon starts and accepts gRPC connections on configured port
  2. Events can be ingested via IngestEvent RPC and persisted to RocksDB
  3. Events are stored with time-prefixed keys enabling range scans
  4. Configuration loads from file, env vars, and CLI flags in correct precedence
  5. Daemon binary supports start/stop/status commands
**Plans**: 5 plans in 4 waves

Plans:
- [x] 01-00-PLAN.md — Workspace scaffolding (crate structure, proto placeholder, docs/README.md)
- [x] 01-01-PLAN.md — Storage layer (RocksDB setup, column families, compaction, time-prefixed keys)
- [x] 01-02-PLAN.md — Domain types (Event, TocNode, Grip, OutboxEntry, Settings configuration)
- [x] 01-03-PLAN.md — gRPC service scaffolding (tonic setup, protos, IngestEvent RPC, health, reflection)
- [x] 01-04-PLAN.md — CLI daemon binary (start/stop/status commands, config loading, graceful shutdown)

### Phase 2: TOC Building
**Goal**: Construct the time-based Table of Contents hierarchy with summaries at each level
**Depends on**: Phase 1
**Requirements**: TOC-01, TOC-02, TOC-03, TOC-04, TOC-05, TOC-06, SUMM-01, SUMM-02, SUMM-04
**Success Criteria** (what must be TRUE):
  1. Events are automatically segmented based on time/token thresholds
  2. Segments have overlap for context continuity
  3. TOC nodes exist at all time levels (Year, Month, Week, Day, Segment)
  4. Each TOC node contains title, bullets, keywords, and child references
  5. Day/Week/Month rollup jobs produce summaries from children with checkpoint recovery
**Plans**: TBD

Plans:
- [x] 02-01: Segmentation engine (time/token boundaries, overlap)
- [x] 02-02: Summarizer trait and implementation
- [x] 02-03: TOC hierarchy builder (nodes, rollups, checkpointing)

### Phase 3: Grips & Provenance
**Goal**: Anchor TOC summaries to source evidence through grips (excerpt + event pointers)
**Depends on**: Phase 2
**Requirements**: GRIP-01, GRIP-02, GRIP-03, GRIP-04, SUMM-03
**Success Criteria** (what must be TRUE):
  1. Grips are created during summarization with excerpt and event references
  2. TOC node bullets link to supporting grip IDs
  3. Grips are stored in dedicated column family
  4. Given a grip ID, the system returns context events around the excerpt
**Plans**: TBD

Plans:
- [x] 03-01: Grip storage and data model
- [x] 03-02: Summarizer grip extraction integration
- [x] 03-03: Grip expansion (context retrieval)

### Phase 4: Query Layer
**Goal**: Expose navigation RPCs so agents can traverse the TOC and retrieve events
**Depends on**: Phase 3
**Requirements**: QRY-01, QRY-02, QRY-03, QRY-04, QRY-05
**Success Criteria** (what must be TRUE):
  1. GetTocRoot returns top-level time period nodes
  2. GetNode returns a specific node with its children and summary
  3. BrowseToc supports paginated navigation of large child lists
  4. GetEvents retrieves raw events for a specified time range
  5. ExpandGrip retrieves context around a grip excerpt
**Plans**: TBD

Plans:
- [x] 04-01: TOC navigation RPCs (GetTocRoot, GetNode, BrowseToc)
- [x] 04-02: Event retrieval RPCs (GetEvents, ExpandGrip)

### Phase 5: Integration
**Goal**: Connect hook handlers for event ingestion and provide CLI tools for querying and administration
**Depends on**: Phase 4
**Requirements**: HOOK-02, HOOK-03, CLI-02, CLI-03
**Success Criteria** (what must be TRUE):
  1. Hook handlers can call IngestEvent RPC to send conversation events
  2. Event types from hooks map correctly to memory events
  3. Query CLI allows manual TOC navigation for testing
  4. Admin commands can rebuild TOC, trigger compaction, and show status
**Plans**: TBD

Plans:
- [x] 05-01: Hook handler integration (IngestEvent client, event mapping)
- [x] 05-02: Query CLI (manual navigation, testing)
- [x] 05-03: Admin commands (rebuild-toc, compact, status)

### Phase 6: End-to-End Demo
**Goal**: Validate the complete workflow from conversation capture through query resolution
**Depends on**: Phase 5
**Requirements**: (validation phase - no new requirements, validates all prior)
**Success Criteria** (what must be TRUE):
  1. Hook captures a conversation, events flow to daemon, TOC builds automatically
  2. Agent can navigate TOC via gRPC to find relevant time periods
  3. Query "what did we discuss yesterday?" returns summary-based answer
  4. Agent can drill down from summary to grips to raw events for verification
  5. System recovers gracefully from daemon restart (crash recovery)
**Plans**: TBD

Plans:
- [x] 06-01: Integration test harness and demo script
- [x] 06-02: Documentation and usage examples

### Phase 7: Agentic Memory Plugin
**Goal**: Provide a Claude Code marketplace plugin for querying past conversations with commands and autonomous agents
**Depends on**: Phase 6
**Requirements**: SKILL-01, SKILL-02, SKILL-03, PLUGIN-01, PLUGIN-02
**Success Criteria** (what must be TRUE):
  1. Plugin provides `/memory-search`, `/memory-recent`, `/memory-context` slash commands
  2. Autonomous agent handles complex multi-step memory queries
  3. Skill follows PDA (Progressive Disclosure Architecture) with layered references
  4. Skill passes quality grading (99/100, Grade A)
  5. Plugin uses marketplace.json manifest format
  6. Skill handles daemon connection failures gracefully via validation checklist
**Plans**: 1 plan complete

Plans:
- [x] 07-01: Agentic memory query plugin (marketplace.json, 3 commands, 1 agent, graded skill)

**Implemented Architecture:**
```
                                    Agent Memory
                                    +-----------------+
                                    |  memory-daemon  |
                                    |  (gRPC :50051)  |
                                    +-----------------+
                                             ^
                                             | CLI query
                                    +--------+--------+
                                    | memory-query    |
                                    | plugin          |
                                    | +-------------+ |
                                    | | 3 commands  | |
                                    | | 1 agent     | |
                                    | | SKILL.md    | |
                                    | +-------------+ |
                                    +-----------------+
```

**Plugin Components:**
| Component | File | Purpose |
|-----------|------|---------|
| Skill | skills/memory-query/SKILL.md | Core capability (99/100 grade) |
| Command | commands/memory-search.md | `/memory-search <topic>` |
| Command | commands/memory-recent.md | `/memory-recent [--days N]` |
| Command | commands/memory-context.md | `/memory-context <grip>` |
| Agent | agents/memory-navigator.md | Complex multi-step queries |

### Phase 8: CCH Hook Integration
**Goal**: Integrate agent-memory with code_agent_context_hooks (CCH) for automatic event capture
**Depends on**: Phase 7
**Requirements**: CCH-01, CCH-02, CCH-03
**Success Criteria** (what must be TRUE):
  1. CCH hooks.yaml can be configured to capture conversation events
  2. Hook handler maps CCH events to memory events
  3. Hook handler uses memory-client library to communicate with memory-daemon
  4. Events are automatically ingested without manual intervention
**Plans**: 1 plan complete

Plans:
- [x] 08-01: CCH hook handler (memory-ingest binary, event mapping, hooks.yaml configuration)

**CCH Event Mapping (Future):**
| CCH Event | Memory Event Type | Notes |
|-----------|------------------|-------|
| session-start | session_start | Captures session_id, project context |
| user-prompt | user_message | User's prompt text |
| post-tool-use | tool_result | Tool name, result summary |
| session-end | session_end | Session duration, token count |
| pre-compact | (no mapping) | Could trigger TOC rebuild |

### Phase 9: Setup & Installer Plugin
**Goal**: Provide an interactive setup wizard plugin that guides users through installing, configuring, and managing agent-memory
**Depends on**: Phase 8
**Requirements**: SETUP-01, SETUP-02, SETUP-03, SETUP-04, SETUP-05
**Success Criteria** (what must be TRUE):
  1. Plugin provides `/memory-setup` command that launches interactive wizard
  2. Wizard asks questions about: installation method, hook configuration, daemon settings, summarizer choice
  3. Plugin can install binaries (memory-daemon, memory-ingest) to user's system
  4. Plugin generates hooks.yaml configuration based on user answers
  5. Plugin provides `/memory-status` command to check installation health
  6. Plugin provides `/memory-config` command to modify settings after initial setup
  7. Autonomous agent handles complex setup troubleshooting
  8. Skill follows PDA with layered references for advanced configuration
**Plans**: 4 plans complete

Plans:
- [x] 09-01: Setup plugin structure (marketplace.json, skill, commands, agent)
- [x] 09-02: Interactive wizard flow (questions, configuration generation)
- [x] 09-03: Installation automation (binary installation, path setup)
- [x] 09-04: Health check and troubleshooting (status, diagnostics, fixes)

### Phase 10: Background Scheduler
**Goal**: Provide in-process Tokio-based cron scheduler for periodic background jobs (TOC rollups, compaction, index maintenance)
**Depends on**: Phase 9
**Requirements**: SCHED-01, SCHED-02, SCHED-03, SCHED-04, SCHED-05
**Success Criteria** (what must be TRUE):
  1. Cron expressions parsed and scheduled via tokio-cron-scheduler
  2. Timezone-aware scheduling with DST handling (chrono-tz)
  3. Overlap policy configurable: skip or concurrent (queue deferred - adds complexity)
  4. Jitter support to spread load across instances
  5. Graceful shutdown stops scheduling, finishes current job, or cancels safely
  6. TOC rollup jobs (day/week/month) run on schedule
  7. Job status observable via CLI/gRPC (last run, next run, success/failure)
**Plans**: 4 plans in 3 waves

Plans:
- [x] 10-01-PLAN.md — Scheduler infrastructure (memory-scheduler crate, tokio-cron-scheduler, timezone handling)
- [x] 10-02-PLAN.md — Job registry and lifecycle (JobRegistry, overlap policy, jitter utilities)
- [x] 10-03-PLAN.md — TOC rollup jobs (wire existing rollups to scheduler, daemon integration)
- [x] 10-04-PLAN.md — Job observability (GetSchedulerStatus RPC, CLI scheduler commands)

### Phase 10.5: Agentic TOC Search (INSERTED)
**Goal**: Add foundational agentic search using TOC navigation with simple term matching - works without any index dependencies
**Depends on**: Phase 10
**Requirements**: SEARCH-01, SEARCH-02, SEARCH-03, SEARCH-04, SEARCH-05
**Success Criteria** (what must be TRUE):
  1. SearchNode RPC searches within a single node's fields (title, summary, bullets, keywords)
  2. SearchChildren RPC searches across all children of a parent node at a specified level
  3. Simple term-overlap scoring without external dependencies (no Tantivy, no HNSW)
  4. Agent can navigate TOC using search to find relevant content
  5. Search results include grip IDs for provenance verification
  6. Explainable navigation paths show why each level was chosen
  7. CLI search command available for testing
**Plans**: 3 plans in 3 waves

Plans:
- [x] 10.5-01-PLAN.md — Core search logic (search_node function, term overlap scoring, unit tests)
- [x] 10.5-02-PLAN.md — gRPC integration (SearchNode/SearchChildren RPCs, integration tests)
- [x] 10.5-03-PLAN.md — CLI and agent (search command, navigator agent updates, documentation)

**Documentation:**
- Technical Plan: docs/plans/phase-10.5-agentic-toc-search.md
- PRD: docs/prds/agentic-toc-search-prd.md

### Phase 11: BM25 Teleport (Tantivy)
**Goal**: Enable fast keyword-based search that "teleports" agents directly to relevant TOC nodes or grips without traversing the hierarchy
**Depends on**: Phase 10
**Requirements**: TELE-01, TELE-04, TELE-05, TELE-06, TELE-07
**Success Criteria** (what must be TRUE):
  1. Tantivy embedded index stores searchable text from TOC summaries and grip excerpts
  2. BM25 search returns ranked TOC node IDs or grip pointers
  3. Search results include relevance scores for agent decision-making
  4. Index is incrementally updated as new TOC nodes are created
  5. CLI provides `teleport search <query>` command for testing
**Plans**: 4 plans in 3 waves

**Documentation:**
- PRD: docs/prds/bm25-teleport-prd.md
- Research: .planning/phases/11-bm25-teleport-tantivy/11-RESEARCH.md

Plans:
- [x] 11-01-PLAN.md — Tantivy integration (memory-search crate, schema, index setup)
- [x] 11-02-PLAN.md — Indexing pipeline (TOC node and grip text extraction, document mapping)
- [x] 11-03-PLAN.md — Search API (gRPC TeleportSearch RPC, BM25 scoring)
- [x] 11-04-PLAN.md — CLI and testing (teleport command, background commit job)

### Phase 12: Vector Teleport (HNSW)
**Goal**: Enable semantic similarity search for conceptually related content even when keywords don't match
**Depends on**: Phase 11
**Requirements**: TELE-02, TELE-04 (vector support), TELE-05, TELE-06, FR-09 (Outbox indexing), FR-10 (Checkpoint recovery)
**Success Criteria** (what must be TRUE):
  1. Local HNSW index stores embeddings for TOC summaries and grips
  2. Embedding generation uses local model (no API dependency)
  3. Vector search returns semantically similar TOC nodes or grips
  4. Hybrid search combines BM25 and vector scores
  5. Index rebuild is fast (<1 minute for 10k nodes)
  6. Outbox-driven indexing automatically indexes new TOC nodes and grips
  7. Checkpoint-based recovery ensures crash safety for indexing
**Plans**: 5 plans in 4 waves

**Documentation:**
- PRD: docs/prds/hierarchical-vector-indexing-prd.md
- Technical Plan: docs/plans/phase-12-vector-teleport.md
- Research: .planning/phases/12-vector-teleport-hnsw/12-RESEARCH.md

Plans:
- [x] 12-01-PLAN.md — Embedding infrastructure (memory-embeddings crate, Candle model, caching)
- [x] 12-02-PLAN.md — Vector index (memory-vector crate, usearch HNSW, metadata storage)
- [x] 12-02b-PLAN.md — Vector indexing pipeline (outbox consumer, checkpoint recovery, admin commands)
- [x] 12-03-PLAN.md — gRPC integration (VectorTeleport, HybridSearch, GetVectorIndexStatus RPCs)
- [x] 12-04-PLAN.md — CLI and documentation (teleport commands, user guide)

### Phase 13: Outbox Index Ingestion
**Goal**: Drive index updates from the existing outbox pattern for rebuildable, crash-safe search indexes
**Depends on**: Phase 12
**Requirements**: TELE-03
**Success Criteria** (what must be TRUE):
  1. Outbox entries trigger index updates for new TOC nodes and grips
  2. Index consumer tracks checkpoint for crash recovery
  3. Full index rebuild from storage is supported via admin command
  4. Index state is independent of primary storage (can be deleted and rebuilt)
  5. Indexing is async and doesn't block event ingestion
**Plans**: 4 plans in 3 waves

**Documentation:**
- Research: .planning/phases/13-outbox-index-ingestion/13-RESEARCH.md

Plans:
- [x] 13-01-PLAN.md — Outbox consumer infrastructure (memory-indexing crate, outbox reading, checkpoint tracking)
- [x] 13-02-PLAN.md — Incremental index updates (IndexingPipeline, dispatch logic, mock tests)
- [x] 13-03-PLAN.md — Full rebuild command (admin rebuild-indexes, dry-run support)
- [x] 13-04-PLAN.md — Scheduler integration (background job, GetIndexingStatus RPC)

### Phase 14: Topic Graph Memory
**Goal**: Enable conceptual discovery through semantic topics extracted from TOC summaries with time-decayed importance scoring
**Depends on**: Phase 12 (uses embedding infrastructure)
**Requirements**: TOPIC-01 through TOPIC-08
**Success Criteria** (what must be TRUE):
  1. Topics extracted from TOC summaries via embedding clustering with LLM labeling
  2. Topics stored in CF_TOPICS column family with importance scores
  3. Time-decayed importance scoring surfaces recent/frequent topics
  4. Topic relationships (similar topics, parent/child hierarchy) discoverable
  5. Topic navigation RPCs enable agents to explore conceptual connections
  6. Topic lifecycle management (pruning dormant topics, resurrection on reactivation)
  7. Fully optional via configuration (topics.enabled = false disables all processing)
  8. GetTopicGraphStatus RPC enables feature discovery
**Plans**: 6 plans in 6 waves

**Documentation:**
- PRD: docs/prds/topic-graph-memory-prd.md
- Technical Plan: docs/plans/topic-graph-memory.md

Plans:
- [x] 14-01-PLAN.md — Topic extraction (memory-topics crate, CF_TOPICS, HDBSCAN clustering, cosine similarity)
- [x] 14-02-PLAN.md — Topic labeling (LLM integration with keyword fallback, stopword filtering)
- [x] 14-03-PLAN.md — Importance scoring (exponential time decay with configurable half-life)
- [x] 14-04-PLAN.md — Topic relationships (similarity detection, parent/child hierarchy, cycle prevention)
- [x] 14-05-PLAN.md — Navigation RPCs (5 gRPC endpoints: status, query, nodes, top, related)
- [x] 14-06-PLAN.md — Lifecycle management (pruning, resurrection, scheduler jobs, CLI commands)

### Phase 15: Configuration Wizard Skills
**Goal**: Create interactive AskUserQuestion-based configuration wizard skills for advanced storage, LLM, and multi-agent configuration
**Depends on**: Phase 9 (Setup & Installer Plugin)
**Requirements**: CONFIG-01, CONFIG-02, CONFIG-03, CONFIG-04
**Success Criteria** (what must be TRUE):
  1. `/memory-storage` skill provides interactive wizard for storage paths, retention, cleanup, GDPR mode
  2. `/memory-llm` skill provides interactive wizard for LLM provider, model discovery, cost estimation, API testing
  3. `/memory-agents` skill provides interactive wizard for multi-agent mode, agent tagging, cross-agent queries
  4. All 29 config options are addressable through wizard skills (coverage verified)
  5. State detection skips already-configured options
  6. Skills follow existing memory-setup patterns (--minimal, --advanced, --fresh flags)
**Plans**: 5 plans in 3 waves

**Documentation:**
- Technical Plan: docs/plans/configuration-wizard-skills-plan.md

Plans:
- [ ] 15-01-PLAN.md — memory-storage skill (storage paths, retention, cleanup, GDPR, performance tuning)
- [ ] 15-02-PLAN.md — memory-llm skill (provider, model discovery, API testing, cost estimation, budget)
- [ ] 15-03-PLAN.md — memory-agents skill (multi-agent mode, agent ID, query scope, team settings)
- [ ] 15-04-PLAN.md — Reference documentation (retention-policies.md, provider-comparison.md, storage-strategies.md)
- [ ] 15-05-PLAN.md — Plugin integration and memory-setup updates (marketplace.json, gap resolution)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9 -> 10 -> 10.5 -> 11 -> 12 -> 13 -> 14 -> 15

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 5/5 | Complete | 2026-01-30 |
| 2. TOC Building | 3/3 | Complete | 2026-01-30 |
| 3. Grips & Provenance | 3/3 | Complete | 2026-01-30 |
| 4. Query Layer | 2/2 | Complete | 2026-01-30 |
| 5. Integration | 3/3 | Complete | 2026-01-30 |
| 6. End-to-End Demo | 2/2 | Complete | 2026-01-30 |
| 7. Agentic Memory Plugin | 1/1 | Complete | 2026-01-30 |
| 8. CCH Hook Integration | 1/1 | Complete | 2026-01-30 |
| 9. Setup & Installer Plugin | 4/4 | Complete | 2026-01-31 |
| 10. Background Scheduler | 4/4 | Complete | 2026-01-31 |
| 10.5. Agentic TOC Search | 3/3 | Complete | 2026-02-02 |
| 11. BM25 Teleport (Tantivy) | 4/4 | Complete | 2026-02-02 |
| 12. Vector Teleport (HNSW) | 5/5 | Complete | 2026-02-02 |
| 13. Outbox Index Ingestion | 4/4 | Complete | 2026-02-02 |
| 14. Topic Graph Memory | 6/6 | Complete | 2026-02-02 |
| 15. Configuration Wizard Skills | 0/5 | Planned | - |

---
*Roadmap created: 2026-01-29*
*v1 Milestone completed: 2026-01-30*
*Phase 7 completed: 2026-01-30 (Agentic Memory Plugin)*
*Phase 8 completed: 2026-01-30 (CCH Hook Integration)*
*Phase 9 completed: 2026-01-31 (Setup & Installer Plugin)*
*v2.0 phases added: 2026-01-31 (Phase 10 Scheduler + Phases 11-13 Teleport)*
*Phase 10 plans created: 2026-01-31*
*Phase 11 plans created: 2026-01-31*
*Phase 10.5 added: 2026-02-01 (Agentic TOC Search - inserted phase)*
*Phase 14 added: 2026-02-01 (Topic Graph Memory - conceptual enrichment layer)*
*Total plans: 48 across 15 phases (22 v1.0 + 26 v2.0)*
*Phase 12 plans created: 2026-02-01 (5 plans including outbox indexing pipeline)*
*Phase 15 added: 2026-02-01 (Configuration Wizard Skills - AskUserQuestion-based config wizards)*
*Total plans: 53 across 16 phases (22 v1.0 + 31 v2.0)*
*Phase 10.5 completed: 2026-02-02 (Agentic TOC Search - 3 plans)*
*Phase 11 completed: 2026-02-02 (BM25 Teleport - 4 plans)*
*Phase 12 completed: 2026-02-02 (Vector Teleport - 5 plans)*
*Phase 13 completed: 2026-02-02 (Outbox Index Ingestion - 4 plans)*
*Phase 14 completed: 2026-02-02 (Topic Graph Memory - 6 plans)*
