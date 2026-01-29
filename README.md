# Agent Memory

A local, append-only conversational memory system for AI agents with TOC-based agentic navigation.

## Overview

Agent Memory provides persistent conversational memory for AI coding assistants like Claude Code, OpenCode, Gemini CLI, and GitHub Copilot CLI. Instead of brute-force search over all conversations, it uses a hierarchical Table of Contents (TOC) organized by time, enabling agents to efficiently answer questions like "what were we talking about last week?" without scanning everything.

## Core Value

**An agent can answer "what were we talking about?" without scanning everything.**

Time-based TOC navigation beats brute-force search. The TOC acts as a Progressive Disclosure Architecture (PDA)—similar to how PDA works in Agentic Skills—where agents start with high-level summaries and use agentic search to find areas of interest, then drill down only when needed. This enables efficient navigation: an agent can quickly locate "conversations from last Tuesday about the database migration" without loading the entire history, drilling from Year → Month → Week → Day → Segment until it finds the relevant context.

## Key Features

- **Append-only event storage** - Immutable conversation history in RocksDB
- **Time-based TOC hierarchy** - Year → Month → Week → Day → Segment navigation
- **Summaries at every level** - Title, bullets, and keywords for quick orientation
- **Grips for provenance** - Every summary bullet links to source evidence
- **Zero-token ingestion** - Passive capture via agent hooks (no conversation overhead)
- **gRPC API** - Clean contract for agent integration
- **Local-first** - Per-project stores, no cloud dependency

## Progressive Disclosure Architecture (PDA)

The TOC hierarchy implements Progressive Disclosure Architecture, the same pattern used in Agentic Skills. Rather than exposing all conversation data at once, the system presents information in layers:

1. **Start broad** - Agent receives top-level time periods (years/months)
2. **Scan summaries** - Each node contains title, bullets, and keywords
3. **Identify relevance** - Agent uses agentic search to find areas of interest
4. **Drill down** - Navigate deeper only into promising branches
5. **Access details** - Retrieve raw events or grip excerpts when needed

This approach mirrors how humans navigate large information spaces: you don't read every email to find one from last week—you scan by date, then by subject, then read the specific message.

**Example navigation:**
```
"What did we discuss about authentication last week?"

Year 2026
  └── Month January
        └── Week 4 (Jan 20-26)
              ├── Day Monday: "API refactoring, test fixes"
              ├── Day Tuesday: "Authentication system design" ← relevant!
              │     └── Segment 2: "JWT vs session tokens discussion"
              │           └── Grip: "decided on JWT with refresh tokens"
              └── Day Wednesday: "Database migration planning"
```

The agent navigates to Week 4, scans day summaries, identifies Tuesday as relevant, drills into the authentication segment, and retrieves the specific grip with the decision. No brute-force search required.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      AI Agents                              │
│  (Claude Code, OpenCode, Gemini CLI, GitHub Copilot CLI)    │
└────────────────────────┬────────────────────────────────────┘
                         │ Hooks (passive capture)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Hook Handlers                              │
│            (event mapping, gRPC client)                      │
└────────────────────────┬────────────────────────────────────┘
                         │ IngestEvent RPC
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Memory Daemon                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Ingestion│  │   TOC    │  │ Summarizer│  │  Query   │    │
│  │  Layer   │  │ Builder  │  │  (pluggable)│  │  Layer   │    │
│  └────┬─────┘  └────┬─────┘  └─────┬─────┘  └────┬─────┘    │
│       └─────────────┴──────────────┴─────────────┘          │
│                         │                                    │
│                    ┌────▼────┐                               │
│                    │ RocksDB │                               │
│                    │ (events,│                               │
│                    │ toc,    │                               │
│                    │ grips)  │                               │
│                    └─────────┘                               │
└─────────────────────────────────────────────────────────────┘
```

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust | Single binary, fast scans, predictable memory |
| Storage | RocksDB | Embedded, fast range scans, column families |
| API | gRPC (tonic) | Clean contract, efficient serialization |
| Summarizer | Pluggable | API (Claude/GPT) or local inference |

## Query Tools

Agents interact with memory through these gRPC operations:

| Operation | Description |
|-----------|-------------|
| `get_toc_root` | Top-level time periods |
| `get_node(node_id)` | Drill into specific period |
| `get_events(time_range)` | Raw events (last resort) |
| `expand_grip(grip_id)` | Context around excerpt |
| `teleport_query(query)` | Index-based jump (v2) |

## Event Types

Events are captured via agent hooks with zero token overhead:

| Hook Event | Memory Event |
|------------|--------------|
| SessionStart | session_start |
| UserPromptSubmit | user_message |
| PostToolUse | tool_result |
| Stop | assistant_stop |
| SubagentStart | subagent_start |
| SubagentStop | subagent_stop |
| SessionEnd | session_end |

## Project Status

**Status: Early Development**

The project is currently in the planning phase. See `.planning/ROADMAP.md` for the full development roadmap.

### Roadmap

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Foundation - Storage, types, gRPC, config | Not started |
| 2 | TOC Building - Segmentation, summarization | Not started |
| 3 | Grips & Provenance - Excerpt storage, linking | Not started |
| 4 | Query Layer - Navigation RPCs | Not started |
| 5 | Integration - Hook handlers, CLI tools | Not started |
| 6 | End-to-End Demo - Full workflow validation | Not started |

## Installation

*Coming soon* - Installation instructions will be added once the initial implementation is complete.

## Usage

*Coming soon* - Usage examples will be added as the project progresses.

## Configuration

Configuration follows a layered precedence:

1. Built-in defaults
2. Config file (`~/.config/agent-memory/config.toml`)
3. Environment variables
4. CLI flags

## Development

### Prerequisites

- Rust (latest stable)
- protobuf compiler

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

## Related Projects

- **code_agent_context_hooks** - Hook handlers for Claude Code that feed events into this memory system

## Out of Scope

The following are explicitly excluded from v1:

- Graph database (TOC is a tree, not a graph)
- Multi-tenant support (single agent, local deployment)
- Delete/update events (append-only truth)
- HTTP API (gRPC only)
- MCP integration (hooks are passive, no token overhead)

## License

*TBD*

---

*For detailed planning documents, see the `.planning/` directory.*
