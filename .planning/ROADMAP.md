# Roadmap: Agent Memory

## Overview

This roadmap delivers a local, append-only conversational memory system with TOC-based agentic navigation. The journey proceeds from foundational storage through TOC construction, grips for provenance, query capabilities, hook integration, and culminates in an end-to-end demonstration. Each phase builds on the previous, delivering a coherent capability that can be verified independently.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3, 4, 5, 6): Planned milestone work
- Decimal phases (e.g., 2.1): Urgent insertions if needed (marked with INSERTED)

- [ ] **Phase 1: Foundation** - Storage layer, domain types, gRPC scaffolding, configuration, daemon binary
- [ ] **Phase 2: TOC Building** - Segmentation, summarization, time hierarchy construction
- [ ] **Phase 3: Grips & Provenance** - Excerpt storage, summary-to-grip linking, expand capability
- [ ] **Phase 4: Query Layer** - Navigation RPCs for TOC traversal and event retrieval
- [ ] **Phase 5: Integration** - Hook handler connection, query CLI, admin commands
- [ ] **Phase 6: End-to-End Demo** - Full workflow validation from ingestion to query answer

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
- [ ] 01-00-PLAN.md — Workspace scaffolding (crate structure, proto placeholder, docs/README.md)
- [ ] 01-01-PLAN.md — Storage layer (RocksDB setup, column families, compaction, time-prefixed keys)
- [ ] 01-02-PLAN.md — Domain types (Event, TocNode, Grip, OutboxEntry, Settings configuration)
- [ ] 01-03-PLAN.md — gRPC service scaffolding (tonic setup, protos, IngestEvent RPC, health, reflection)
- [ ] 01-04-PLAN.md — CLI daemon binary (start/stop/status commands, config loading, graceful shutdown)

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
- [ ] 02-01: Segmentation engine (time/token boundaries, overlap)
- [ ] 02-02: Summarizer trait and implementation
- [ ] 02-03: TOC hierarchy builder (nodes, rollups, checkpointing)

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
- [ ] 03-01: Grip storage and data model
- [ ] 03-02: Summarizer grip extraction integration
- [ ] 03-03: Grip expansion (context retrieval)

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
- [ ] 04-01: TOC navigation RPCs (GetTocRoot, GetNode, BrowseToc)
- [ ] 04-02: Event retrieval RPCs (GetEvents, ExpandGrip)

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
- [ ] 05-01: Hook handler integration (IngestEvent client, event mapping)
- [ ] 05-02: Query CLI (manual navigation, testing)
- [ ] 05-03: Admin commands (rebuild-toc, compact, status)

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
- [ ] 06-01: Integration test harness and demo script
- [ ] 06-02: Documentation and usage examples

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/5 | Planning complete | - |
| 2. TOC Building | 0/3 | Not started | - |
| 3. Grips & Provenance | 0/3 | Not started | - |
| 4. Query Layer | 0/2 | Not started | - |
| 5. Integration | 0/3 | Not started | - |
| 6. End-to-End Demo | 0/2 | Not started | - |

---
*Roadmap created: 2026-01-29*
*Phase 1 planned: 2026-01-29*
*Total plans: 18 across 6 phases*
*Total v1 requirements: 42 (41 mapped to phases, 1 external)*
