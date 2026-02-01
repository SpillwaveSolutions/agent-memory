# Agent Memory - Design Documentation

> A local, append-only conversational memory system with TOC-based agentic navigation.

## The Vision

AI coding agents are powerful but forgetful. Every conversation starts fresh, losing valuable context from previous sessions. When you ask "What did we discuss about authentication last week?" the agent has no way to answer without you manually digging through conversation logs.

Agent Memory solves this by giving agents a structured, navigable memory. Instead of scanning thousands of raw conversation events, agents navigate a hierarchical Table of Contents (TOC) that mirrors how humans naturally search through information. Just as you filter email by date, scan subject lines, and open relevant threads, an agent can drill down through Year, Month, Week, Day, and Segment summaries to find exactly what it needs.

The result: **"What were we discussing last week?" answered in seconds, not minutes.** Every answer comes with provenance through Grips, linking summaries back to source events so agents can verify their responses. This is Progressive Disclosure Architecture applied to memory, revealing detail only when needed, keeping token usage minimal while maximizing retrieval accuracy.

## Document Index

### Core Architecture
| Document | Description |
|----------|-------------|
| [Architecture Overview](01-architecture-overview.md) | System context, containers, components, deployment |
| [Domain Model](03-domain-model.md) | Events, TOC nodes, grips, storage schema |
| [Storage Architecture](07-storage-architecture.md) | RocksDB, column families, key design |

### Behavior & Flow
| Document | Description |
|----------|-------------|
| [Data Flow Sequences](02-data-flow-sequences.md) | Ingestion, TOC building, query resolution |
| [State Machines](04-state-machines.md) | Daemon, scheduler, job lifecycles |
| [TOC Navigation Guide](06-toc-navigation-guide.md) | How agents search memory efficiently |

### APIs & Integration
| Document | Description |
|----------|-------------|
| [API Reference](05-api-reference.md) | gRPC APIs, CLI commands, client library |
| [Getting Started](09-getting-started.md) | Installation, configuration, first steps |

### Operations & Decisions
| Document | Description |
|----------|-------------|
| [Scheduler Design](08-scheduler-design.md) | Background jobs, cron, overlap policies |
| [Architecture Decisions](10-architecture-decisions.md) | ADRs with rationale |
| [Security & Operations](12-security-operations.md) | Security model, procedures, troubleshooting |

### Diagrams
| Document | Description |
|----------|-------------|
| [PlantUML Diagrams](11-plantuml-diagrams.md) | Deployment, component, sequence diagrams |

## Quick Links

- **Want to understand the architecture?** Start with [Architecture Overview](01-architecture-overview.md)
- **Want to integrate?** See [Getting Started](09-getting-started.md)
- **Want to query memory?** See [TOC Navigation Guide](06-toc-navigation-guide.md)
- **Want to understand why?** See [Architecture Decisions](10-architecture-decisions.md)
- **Looking for API details?** See [API Reference](05-api-reference.md)

## Key Concepts

| Term | Definition |
|------|------------|
| **TOC** | Table of Contents. A time-hierarchical index (Year, Month, Week, Day, Segment) that enables agentic navigation through conversation history. |
| **Grip** | Provenance anchor. Links a summary bullet to the source events that support it, enabling verification of any claim. |
| **Segment** | The leaf node of the TOC. A coherent conversation chunk created when time gaps (30 min) or token thresholds (4K) are reached. |
| **Rollup** | The process of aggregating child node summaries into parent nodes. Day nodes roll up segments, weeks roll up days, etc. |
| **Teleport** | (Phase 2) Index-based jump directly to relevant TOC nodes. BM25 keyword search or vector similarity returns node IDs, not content. |
| **Progressive Disclosure** | The navigation pattern where agents start with high-level summaries and drill down only when needed, minimizing token usage. |
| **Event** | An immutable record of an agent interaction (user message, assistant response, tool result, etc.). |
| **Column Family** | A RocksDB partition for different data types (events, toc_nodes, grips, etc.). |
| **Outbox** | A queue of pending events awaiting TOC processing. Ensures crash recovery and idempotent updates. |
| **Checkpoint** | A saved position in background job processing. Enables resume after crash without reprocessing. |

## System Overview

```
                            AI Agent (Claude Code, etc.)
                                        |
                                        | gRPC
                                        v
+-----------------------------------------------------------------------+
|                           Memory Daemon                                |
|  +---------------+  +---------------+  +---------------------------+  |
|  |   Ingestion   |  |    Query      |  |   TOC Builder             |  |
|  |   Service     |  |   Service     |  |   (Background)            |  |
|  +---------------+  +---------------+  +---------------------------+  |
|                              |                                         |
|  +-----------------------------------------------------------------+  |
|  |                    Storage Layer (RocksDB)                       |  |
|  |  +--------+ +----------+ +-------+ +--------+ +-----------+     |  |
|  |  | Events | | TOC Nodes| | Grips | | Outbox | | Checkpts  |     |  |
|  |  +--------+ +----------+ +-------+ +--------+ +-----------+     |  |
|  +-----------------------------------------------------------------+  |
+-----------------------------------------------------------------------+
                                        |
                                        | Hooks
                                        v
+-----------------------------------------------------------------------+
|                         Hook Handlers                                  |
|           (code_agent_context_hooks - external repository)            |
+-----------------------------------------------------------------------+
```

## Navigation Pattern

The TOC enables efficient agentic search through a 5-step progressive disclosure pattern:

| Step | Level | What the Agent Sees | Decision |
|------|-------|---------------------|----------|
| 1 | **Year** | "2024: 847 conversations about auth, databases, Rust" | Too broad, drill down |
| 2 | **Month** | "January: 156 conversations, heavy focus on authentication" | Promising, drill down |
| 3 | **Week** | "Week 3: JWT implementation, OAuth2 integration" | This is it, drill down |
| 4 | **Day** | "Thursday: Debugged JWT token expiration issue" | Found it, drill down |
| 5 | **Segment/Grip** | Actual conversation excerpt with event links | Verify, expand if needed |

At each level, the agent reads a summary and decides whether to drill down, move laterally, or expand a grip for verification.

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust | Single binary, fast scans, predictable memory |
| Storage | RocksDB | Embedded, fast range scans, column families |
| API | gRPC (tonic) | Clean contract, efficient serialization |
| Summarizer | Pluggable | API (Claude/GPT) or local inference |

## Development Status

| Phase | Description | Status |
|-------|-------------|--------|
| 1. Foundation | Storage, types, gRPC scaffolding, daemon | Complete |
| 2. TOC Building | Segmentation, summarization, hierarchy | Complete |
| 3. Grips & Provenance | Excerpt storage, linking, expansion | Complete |
| 4. Query Layer | Navigation RPCs, event retrieval | Complete |
| 5. Integration | Hook handlers, CLI, admin commands | Complete |
| 6. End-to-End Demo | Full workflow validation | Complete |
| 7. Agentic Plugin | Claude Code plugin with commands, agents | Complete |
| 8. CCH Integration | Automatic event capture via hooks | Complete |
| 9. Setup/Installer Plugin | User-friendly installation | Complete |
| 10. Background Scheduler | Cron-based rollup jobs | Complete |
| 11. BM25 Teleport | Tantivy keyword search | In Progress |

## Contributing

### Getting Started

1. Clone the repository
2. Install Rust 1.82+ and `protoc`
3. Run `cargo build --release`
4. Start the daemon: `./target/release/memory-daemon start`
5. Run the demo: `./scripts/demo.sh`

### Code Quality Standards

- **Tests**: Comprehensive unit and integration tests required
- **Type Safety**: Full Rust type system with strict clippy lints
- **Documentation**: Rustdoc comments for all public APIs
- **Formatting**: `cargo fmt` before committing

### Architecture Principles

1. **TOC is primary**: Indexes are accelerators, not dependencies
2. **Append-only**: Never mutate historical data
3. **Fail-open**: Never block the agent if memory is unavailable
4. **Grips for provenance**: Every summary must link to source events
5. **Progressive disclosure**: Start with summaries, reveal detail on demand

### Related Documentation

- [Main README](../README.md) - Project overview and quick start
- [API Reference](../API.md) - gRPC service documentation
- [Architecture](../ARCHITECTURE.md) - Component structure
- [Integration Guide](../INTEGRATION.md) - Client library usage
- [Usage Guide](../USAGE.md) - CLI commands and operations

---

*This documentation covers Agent Memory v1.0.0 and ongoing Phase 2 development.*
