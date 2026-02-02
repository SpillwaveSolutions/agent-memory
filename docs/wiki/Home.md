# Agent Memory Wiki

Welcome to the Agent Memory documentation wiki.

Agent Memory is a local, append-only conversational memory system for AI coding agents (Claude Code, OpenCode, Gemini CLI, GitHub Copilot CLI). It enables agents to answer questions like "what were we talking about last week?" without scanning through entire conversation histories.

## Core Value

**Agentic search through Progressive Disclosure Architecture (PDA)**. Instead of brute-force scanning thousands of events, agents navigate a time-based Table of Contents hierarchy, reading summaries at each level until they find what they need.

## Quick Navigation

### Getting Started

- [README](README) - Project overview and quick start guide
- [Usage Guide](Usage-Guide) - Starting the daemon, CLI commands, troubleshooting
- [Architecture](Architecture) - Component overview, crate structure, data flow

### API & Integration

- [API Reference](API-Reference) - gRPC RPCs, data types, examples
- [Integration Guide](Integration-Guide) - Client library, hook handlers, gRPC examples

### Project Planning (GSD)

- [Project Overview](Project-Overview) - Core value, requirements, constraints, key decisions
- [Roadmap](Roadmap) - 13-phase development plan with detailed success criteria
- [Current State](Current-State) - Active phase, velocity metrics, accumulated context
- [Requirements](Requirements) - v1/v2 requirements with traceability matrix
- [Milestones](Milestones) - Milestone history and completion records

### Phase Documentation

#### v1.0 Phases (Complete)

| Phase | Title | Description |
|-------|-------|-------------|
| [Phase 1](Phase-1-Foundation) | Foundation | Storage layer, domain types, gRPC scaffolding, daemon binary |
| [Phase 2](Phase-2-TOC-Building) | TOC Building | Segmentation, summarization, time hierarchy construction |
| [Phase 3](Phase-3-Grips-Provenance) | Grips & Provenance | Excerpt storage, summary-to-grip linking, expand capability |
| [Phase 5](Phase-5-Integration) | Integration | Hook handler connection, query CLI, admin commands |
| [Phase 6](Phase-6-End-to-End) | End-to-End Demo | Full workflow validation from ingestion to query |

#### v1.5 Phases (Complete)

| Phase | Title | Description |
|-------|-------|-------------|
| [Phase 7](Phase-7-CCH-Integration) | CCH Integration | Initial hook integration planning |
| [Phase 8](Phase-8-CCH-Hook-Integration) | CCH Hook Integration | Automatic event capture via CCH hooks |
| [Phase 9](Phase-9-Setup-Plugin) | Setup & Installer Plugin | Interactive setup wizard plugin with commands |
| [Phase 10](Phase-10-Background-Scheduler) | Background Scheduler | In-process Tokio cron scheduler for TOC rollups |

#### v2.0 Phases (Complete)

| Phase | Title | Description |
|-------|-------|-------------|
| [Phase 10.5](Phase-10.5-Agentic-TOC-Search) | Agentic TOC Search | Index-free term matching via SearchNode/SearchChildren |
| [Phase 11](Phase-11-BM25-Teleport) | BM25 Teleport (Tantivy) | Full-text search index for keyword-based teleportation |
| [Phase 12](Phase-12-Vector-Teleport) | Vector Teleport (HNSW) | Semantic similarity search via local HNSW |
| [Phase 13](Phase-13-Outbox-Index-Ingestion) | Outbox Index Ingestion | Event-driven index updates for rebuildable indexes |
| [Phase 14](Phase-14-Topic-Graph-Memory) | Topic Graph Memory | HDBSCAN clustering, LLM labeling, importance scoring |

#### v2.1 Phases (Planned)

| Phase | Title | Description |
|-------|-------|-------------|
| [Phase 15](Phase-15-Configuration-Wizard) | Configuration Wizard Skills | AskUserQuestion-based interactive config wizards |

## Architecture Overview

```
                        AI Agent (Claude Code, etc.)
                                    |
                                    | gRPC
                                    v
+---------------------------------------------------------------+
|                        Memory Daemon                           |
|  +-------------+  +-------------+  +-----------------------+   |
|  | Ingestion   |  |   Query     |  |   Scheduler           |   |
|  | Service     |  |   Service   |  |   (Background Jobs)   |   |
|  +-------------+  +-------------+  +-----------------------+   |
|                          |                                     |
|  +-----------------------------------------------------+       |
|  |              Search Layer                           |       |
|  |  +----------------+  +---------------------------+  |       |
|  |  | BM25 (Tantivy) |  | Vector HNSW (usearch)     |  |       |
|  +-----------------------------------------------------+       |
|  +-----------------------------------------------------+       |
|  |              Topic Graph Layer                      |       |
|  |  +----------------+  +---------------------------+  |       |
|  |  | HDBSCAN        |  | LLM Labels & Importance   |  |       |
|  +-----------------------------------------------------+       |
|  +-----------------------------------------------------+       |
|  |                 Storage Layer (RocksDB)             |       |
|  |  +--------+ +----------+ +-------+ +------------+   |       |
|  |  | Events | | TOC Nodes| | Grips | | Topics     |   |       |
|  +-----------------------------------------------------+       |
+---------------------------------------------------------------+
```

## Key Concepts

### Table of Contents (TOC)

Time-based hierarchy for navigating conversations:
- **Year** - Annual summary with major themes
- **Month** - Monthly summary with focus areas
- **Week** - Weekly summary with specific topics
- **Day** - Daily summary with conversation segments
- **Segment** - Individual conversation chunk with grips

### Grips (Provenance)

Anchors linking summary bullets to source evidence:
- Contains excerpt text from original conversation
- Links to start/end event IDs for verification
- Enables drill-down from summary to raw events

### Progressive Disclosure

Agents navigate the TOC hierarchy level by level:
1. Read summary at current level
2. Decide: drill down, move laterally, or expand grip
3. Only access raw events when verification needed

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Storage | RocksDB |
| API | gRPC (tonic) |
| Summarizer | Pluggable (API or local LLM) |
| BM25 Search | Tantivy (full-text indexing) |
| Vector Search | usearch HNSW + Candle (all-MiniLM-L6-v2) |
| Topic Clustering | HDBSCAN (density-based) |
| Scheduler | tokio-cron-scheduler |

## Quick Links

- [GitHub Repository](https://github.com/spillwave/agent-memory)
- [Proto Definitions](https://github.com/spillwave/agent-memory/blob/main/proto/memory.proto)
- [Crate Documentation](https://docs.rs/agent-memory)

---

*Last updated: 2026-02-02*
