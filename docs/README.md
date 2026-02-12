# Agent Memory

A local, append-only conversational memory system for AI coding agents.

## Overview

Agent Memory enables AI agents to answer questions like "what were we talking about last week?" without scanning through entire conversation histories. It provides:

- **TOC-based Navigation**: Time-hierarchical Table of Contents (Year → Month → Week → Day → Segment) for efficient drill-down
- **Grips for Provenance**: Excerpts linked to source events for verifiable citations
- **Append-only Storage**: Immutable event log with RocksDB for durability
- **Hook-based Ingestion**: Passive capture from Claude Code, OpenCode, Gemini CLI hooks
- **gRPC API**: High-performance interface for agent integration

## Setup Guides

- [Quickstart (macOS + Linux)](setup/quickstart.md)
- [Full Guide (macOS + Linux)](setup/full-guide.md)
- [Agent Setup (Claude Code, OpenCode, Gemini CLI, Copilot CLI)](setup/agent-setup.md)

## Core Value: Agentic Search Through Progressive Disclosure

The fundamental insight behind Agent Memory is that **agents should search memory the same way they search codebases** - through intelligent, hierarchical exploration rather than brute-force scanning.

This approach mirrors **Progressive Disclosure Architecture (PDA)**, the same pattern used in well-designed Agentic Skills. Just as a skill progressively reveals complexity only when needed, Agent Memory progressively reveals conversation detail only when relevant:

- **Agentic Skills**: Start with a simple interface, reveal advanced options as the agent needs them
- **Agent Memory**: Start with high-level summaries, reveal raw events as the agent drills down

The key principle: **Agentic search beats brute-force scanning**. Instead of loading thousands of conversation events into context, an agent navigates a structured hierarchy, reading summaries at each level until it finds the area of interest, then drilling down for details.

This is how humans naturally search through information - you don't read every email to find a conversation from last week; you filter by date, scan subjects, then open the relevant thread.

## Progressive Disclosure Architecture (PDA)

Agent Memory implements a 5-step progressive disclosure pattern for memory navigation:

### The Navigation Pattern

| Step | Level | What the Agent Sees | Decision |
|------|-------|---------------------|----------|
| 1 | **Year** | "2024: 847 conversations about auth, databases, Rust" | Too broad → drill down |
| 2 | **Month** | "January: 156 conversations, heavy focus on authentication" | Promising → drill down |
| 3 | **Week** | "Week 3: JWT implementation, OAuth2 integration" | This is it → drill down |
| 4 | **Day** | "Thursday: Debugged JWT token expiration issue" | Found it → drill down |
| 5 | **Segment/Grip** | Actual conversation excerpt with event links | Verify → expand if needed |

At each level, the agent reads a **summary** (title, bullets, keywords) and decides whether to:
- **Drill down**: This area looks relevant, explore children
- **Move laterally**: Check sibling nodes for better matches
- **Expand grip**: Found the answer, get the raw events for verification

### Human Analogy: Email Search

Think about how you find an important email from last month:

1. **You don't**: Read every email from the beginning of time
2. **You do**: Filter to "last month" (time-based narrowing)
3. **You do**: Scan subject lines for keywords (summary-based search)
4. **You do**: Open the thread that looks right (drill-down)
5. **You do**: Read the specific message (raw content access)

Agent Memory gives AI agents the same efficient search pattern, but structured for programmatic access via gRPC.

### Concrete Example: "What did we discuss about authentication last week?"

Here's how an agent navigates the TOC to answer this query:

```
Agent Query: "What did we discuss about authentication last week?"

Step 1: GetTocRoot()
─────────────────────
Returns: [2024, 2023]
Agent thinks: "User said 'last week', so 2024 is current. Drill down."

Step 2: GetNode("2024")
───────────────────────
Returns:
  title: "2024"
  bullets: ["Authentication system redesign", "Database migration", "Rust learning"]
  keywords: ["auth", "jwt", "postgres", "rust"]
  children: [Jan, Feb, Mar...]
Agent thinks: "Auth mentioned. Current month is January. Drill down to January."

Step 3: GetNode("2024-01")
──────────────────────────
Returns:
  title: "January 2024"
  bullets: ["JWT implementation completed", "OAuth2 provider integration", "Session management refactor"]
  keywords: ["jwt", "oauth", "session", "token"]
  children: [Week1, Week2, Week3, Week4]
Agent thinks: "JWT and OAuth - definitely auth related. 'Last week' = Week 3. Drill down."

Step 4: GetNode("2024-W03")
───────────────────────────
Returns:
  title: "Week of January 15-21"
  bullets:
    - "Implemented JWT token refresh logic" [grip: grip-001]
    - "Fixed OAuth2 callback URL handling" [grip: grip-002]
    - "Added session timeout configuration" [grip: grip-003]
  keywords: ["jwt", "refresh", "oauth", "callback", "session", "timeout"]
  children: [Mon, Tue, Wed, Thu, Fri]
Agent thinks: "Found it! These bullets answer the question. Let me verify with grips."

Step 5: ExpandGrip("grip-001")
──────────────────────────────
Returns:
  excerpt: "User asked how to handle JWT refresh tokens without logging out"
  events: [
    {role: "user", text: "How do we refresh JWT tokens without forcing logout?"},
    {role: "assistant", text: "We can implement a sliding window refresh..."},
    {role: "user", text: "What about the race condition when multiple tabs..."}
  ]
Agent thinks: "Verified. I can now answer with confidence and cite the source."

Final Answer:
"Last week we discussed JWT token refresh implementation. Specifically, you asked
about refreshing tokens without forcing logout, and we solved the multi-tab race
condition using a sliding window approach. [Source: conversation from January 18]"
```

### Why PDA is Primary (Vector Search is an Accelerator)

| Approach | Tokens Used | Accuracy | Verifiability |
|----------|-------------|----------|---------------|
| **Brute-force scan** | 50,000+ | Medium | High (has source) |
| **Vector similarity alone** | 2,000 | Medium | Low (no context) |
| **PDA navigation** | 500 | High | High (grips link to source) |
| **PDA + Vector teleport** | 300 | High | High (best of both) |

Vector search alone might return "JWT refresh logic" as a match, but without the surrounding context, the agent can't verify if it's the right conversation or understand the full discussion. PDA gives both the answer AND the provenance.

**In Phase 2**, we add vector and BM25 indexes as *teleport accelerators* - they help the agent jump directly to promising TOC nodes, but the agent still navigates the hierarchy to get context. This combines the speed of similarity search with the verifiability of structured navigation.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        AI Agent (Claude Code, etc.)             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ gRPC
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Memory Daemon                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  Ingestion  │  │    Query    │  │   TOC Builder           │  │
│  │  Service    │  │   Service   │  │   (Background)          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Storage Layer (RocksDB)                 │  │
│  │  ┌────────┐ ┌──────────┐ ┌───────┐ ┌────────┐ ┌─────────┐ │  │
│  │  │ Events │ │ TOC Nodes│ │ Grips │ │ Outbox │ │Checkpts │ │  │
│  │  └────────┘ └──────────┘ └───────┘ └────────┘ └─────────┘ │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Hooks
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Hook Handlers                              │
│  (code_agent_context_hooks - external repository)               │
└─────────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Table of Contents (TOC)

The TOC is a time-based hierarchy that summarizes conversations:

```
2024 (Year)
├── January (Month)
│   ├── Week 1 (Week)
│   │   ├── Monday (Day)
│   │   │   ├── Segment 1: "Discussed auth implementation"
│   │   │   └── Segment 2: "Debugged database connection"
│   │   └── Tuesday (Day)
│   │       └── ...
│   └── Week 2
│       └── ...
└── February
    └── ...
```

Each node contains:
- **Title**: Human-readable period name
- **Bullets**: Summary points with linked grips
- **Keywords**: For search/filtering
- **Children**: Links to child nodes for drill-down

### Grips (Provenance)

Grips anchor summary bullets to source evidence:

```
Grip {
    excerpt: "User asked about Rust memory safety",
    event_id_start: "evt:1706540400000:01HN4QXKN6...",
    event_id_end: "evt:1706540500000:01HN4QXYZ...",
    timestamp: 2024-01-29T10:00:00Z,
    source: "segment_summarizer"
}
```

When an agent reads a summary bullet, it can expand the grip to see the original conversation context.

### Events

Events are the immutable records of agent interactions:

```
Event {
    event_id: "01HN4QXKN6YWXVKZ3JMHP4BCDE",
    session_id: "session-123",
    timestamp: 2024-01-29T10:00:00Z,
    event_type: "user_message",
    role: "user",
    text: "How does Rust prevent memory leaks?",
    metadata: {"project": "agent-memory"}
}
```

## Quick Start

### Prerequisites

- Rust 1.82+
- `protoc` (Protocol Buffers compiler)

### Build

```bash
cargo build --release
```

### Start the Daemon

```bash
# Start with defaults (port 50051, db at ~/.memory-store)
./target/release/memory-daemon start

# Start with custom settings
./target/release/memory-daemon start --port 50052 --db-path /path/to/db
```

### Stop/Status

```bash
# Stop the daemon
./target/release/memory-daemon stop

# Check if running
./target/release/memory-daemon status
```

### Configuration

Settings can be provided via (highest priority first):
1. Command-line flags
2. Environment variables (MEMORY_* prefix)
3. Config file (~/.config/memory-daemon/config.toml)
4. Defaults

Environment variables:
- `MEMORY_PORT` - gRPC port (default: 50051)
- `MEMORY_DB_PATH` - RocksDB path (default: ~/.memory-store)
- `MEMORY_LOG_LEVEL` - Log verbosity (default: info)

### CLI Commands

```bash
# Query commands (connect to running daemon)
memory-daemon query --endpoint http://[::1]:50051 root
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"
memory-daemon query --endpoint http://[::1]:50051 browse --parent-id "toc:year:2026" --limit 10
memory-daemon query --endpoint http://[::1]:50051 events --from 1706745600000 --to 1706832000000 --limit 100
memory-daemon query --endpoint http://[::1]:50051 expand --grip-id "grip:123:abc" --before 3 --after 3

# Admin commands (direct storage access)
memory-daemon admin --db-path ~/.memory-store stats
memory-daemon admin --db-path ~/.memory-store compact
memory-daemon admin --db-path ~/.memory-store compact --cf events
```

### Run the Demo

```bash
./scripts/demo.sh
```

This script starts the daemon, ingests sample events, and demonstrates querying.

## Project Structure (Monorepo)

```
agent-memory/
├── crates/                   # Rust crates (server, client, shared)
│   ├── memory-daemon/        # Server binary
│   ├── memory-service/       # gRPC service implementation
│   ├── memory-client/        # Client library for hook handlers
│   ├── memory-storage/       # RocksDB storage layer
│   ├── memory-toc/           # TOC building logic
│   └── memory-types/         # Shared types (Event, TocNode, Grip)
├── plugins/                  # Claude Code marketplace plugins
│   └── memory-query-plugin/  # Memory query plugin
│       ├── .claude-plugin/   # Plugin manifest
│       ├── skills/           # Core skill
│       ├── commands/         # Slash commands (/memory-search, etc.)
│       └── agents/           # Autonomous agents
├── proto/
│   └── memory.proto          # gRPC service definitions
├── docs/
│   └── README.md             # This file
├── scripts/                  # Helper scripts
└── .planning/                # Development planning documents
```

### Crates Overview

| Crate | Type | Description |
|-------|------|-------------|
| memory-daemon | Binary | gRPC server with start/stop/status commands |
| memory-service | Library | gRPC service implementation |
| memory-client | Library | Client for hook handlers to ingest events |
| memory-storage | Library | RocksDB storage with column families |
| memory-toc | Library | TOC building, summarization, rollups |
| memory-types | Library | Shared domain types |

### Skills/Plugins

| Plugin | Description |
|--------|-------------|
| memory-query-plugin | Query past conversations with /memory-search, /memory-recent, /memory-context |

## Supported Agents

Agent Memory supports multiple AI coding agents simultaneously. Each adapter captures events and provides skills/commands for its respective agent:

| Agent | Adapter | Event Capture | Install |
|-------|---------|---------------|---------|
| Claude Code | Built-in (hooks.yaml) | CCH binary | [Setup Guide](../plugins/memory-query-plugin/README.md) |
| OpenCode | Plugin (TypeScript) | Plugin system | [Setup Guide](../plugins/memory-opencode-plugin/README.md) |
| Gemini CLI | Shell hooks | settings.json | [Setup Guide](../plugins/memory-gemini-adapter/README.md) |
| Copilot CLI | Shell hooks | hooks.json | [Setup Guide](../plugins/memory-copilot-adapter/README.md) |

All adapters share the same memory daemon and storage. Events are tagged by agent for cross-agent discovery and filtering.

## Cross-Agent Discovery

When using multiple agents, you can discover which agents contributed memories and query across or within specific agents:

```bash
# List all contributing agents
memory-daemon agents list

# View agent activity timeline
memory-daemon agents activity --agent claude

# View topics for a specific agent
memory-daemon retrieval route "what topics" --agent opencode

# Search across all agents (default)
memory-daemon retrieval route "authentication implementation"

# Search within a specific agent
memory-daemon teleport search "JWT tokens" --agent gemini
```

See the [Cross-Agent Usage Guide](adapters/cross-agent-guide.md) for detailed workflows and examples.

## Development Phases

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

### Phase 2: Teleport Indexes (Accelerators)

While TOC navigation is the primary search mechanism, Phase 2 adds **teleport indexes** as accelerators for direct jumps into the hierarchy:

- **BM25 Keyword Search** (Tantivy) - Full-text search over event content and TOC summaries. Query "JWT refresh" returns matching TOC node IDs and grip IDs, letting the agent teleport directly to relevant time periods.

- **Vector Similarity Search** (HNSW) - Semantic search using embeddings. Query "how did we handle token expiration" finds conceptually similar conversations even if the exact words weren't used.

**Key principle**: Teleports return *pointers* (node IDs, grip IDs), not content. The agent still navigates the TOC to get context and verify relevance. Indexes are disposable accelerators - if they fail or drift, TOC navigation still works.

```
┌─────────────────────────────────────────────────────────────┐
│                    Teleport Indexes                          │
│  ┌─────────────────┐        ┌─────────────────┐             │
│  │ BM25 (Tantivy)  │        │ Vector (HNSW)   │             │
│  │ Keyword search  │        │ Semantic search │             │
│  └────────┬────────┘        └────────┬────────┘             │
│           │                          │                       │
│           └──────────┬───────────────┘                       │
│                      ▼                                       │
│              Return node_ids / grip_ids                      │
│                      │                                       │
│                      ▼                                       │
│              Agent navigates TOC from entry point            │
└─────────────────────────────────────────────────────────────┘
```

### Phase 3: Graph Database (Under Discussion)

For v2, we're evaluating whether to add a **graph database layer** to capture relationships that don't fit the time hierarchy:

- **Entity relationships**: "Project X" mentioned across multiple conversations
- **Topic threads**: Authentication discussions spanning weeks
- **Cross-references**: "As we discussed on Tuesday" links

This would complement (not replace) the TOC. The graph would provide alternative navigation paths while TOC remains the primary structure. Technologies under consideration include embedded graph stores or extending RocksDB with graph-like indexes.

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

## Agent Discovery

List all agents that have contributed memories:

```bash
memory-daemon agents list
```

View agent activity timeline:

```bash
# Activity for all agents (last 30 days, daily buckets)
memory-daemon agents activity

# Activity for a specific agent
memory-daemon agents activity --agent claude

# Activity with time range
memory-daemon agents activity --agent opencode --from 2026-02-01 --to 2026-02-10

# Weekly buckets
memory-daemon agents activity --bucket week
```

Agent discovery uses the `ListAgents` and `GetAgentActivity` gRPC RPCs. Agent counts are derived from `TocNode.contributing_agents` (O(k) over TOC nodes) and time-bounded event scans with chrono bucketing.

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

## Key Design Decisions

1. **TOC as Primary Navigation**: Agentic search via hierarchical drill-down beats brute-force scanning
2. **Append-Only Storage**: Immutable truth, no deletion complexity
3. **gRPC Only**: No HTTP server overhead
4. **Per-Project Stores**: Simpler mental model, configurable for unified mode
5. **Hook-Based Ingestion**: Zero token overhead, passive capture

## Out of Scope (v1)

The following are excluded from v1:

- ~~Graph database~~ → Under discussion for v2 (see Phase 3 above)
- Multi-tenant support (single agent, local deployment)
- Delete/update events (append-only truth)
- HTTP API (gRPC only)
- MCP integration (hooks are passive, no token overhead)

## CCH Integration

Agent Memory integrates with code_agent_context_hooks (CCH) to automatically capture conversation events from Claude Code and other AI coding agents.

### Quick Setup

```bash
# 1. Build the hook handler
cargo build --release -p memory-ingest

# 2. Install to local bin
mkdir -p ~/.local/bin
cp target/release/memory-ingest ~/.local/bin/

# 3. Start the memory daemon
./target/release/memory-daemon start

# 4. Copy the hooks configuration
cp examples/hooks.yaml ~/.claude/hooks.yaml
```

### How It Works

The `memory-ingest` binary is a lightweight CCH hook handler that:

1. **Reads** CCH JSON events from stdin
2. **Maps** them to memory events using `memory-client`
3. **Sends** them to the daemon via gRPC
4. **Returns** `{"continue":true}` to stdout

The binary is designed to be fast (<100ms) and fail-open - if the daemon is down, it still returns success to avoid blocking Claude.

### Event Capture

CCH sends these events to agent-memory:

| CCH Event | Memory Event | Description |
|-----------|--------------|-------------|
| SessionStart | session_start | New conversation started |
| UserPromptSubmit | user_message | User submitted a prompt |
| PostToolUse | tool_result | Tool execution completed |
| SessionEnd/Stop | session_end | Conversation ended |
| SubagentStart | subagent_start | Subagent spawned |
| SubagentStop | subagent_stop | Subagent completed |

### Configuration

The example `hooks.yaml` includes:

```yaml
rules:
  - name: capture-to-memory
    matchers:
      operations:
        - SessionStart
        - UserPromptSubmit
        - PostToolUse
        - SessionEnd
        - SubagentStart
        - SubagentStop
    actions:
      run: "~/.local/bin/memory-ingest"
```

### Testing the Integration

```bash
# Test with a sample event (daemon not required for this test)
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-123","message":"Hello world"}' | ./target/release/memory-ingest
# Expected output: {"continue":true}

# Test with daemon running
./target/release/memory-daemon start
echo '{"hook_event_name":"SessionStart","session_id":"test-123"}' | ./target/release/memory-ingest

# Verify events were captured
./target/release/memory-daemon query events --from 0 --to $(date +%s)000 --limit 10
```

### Troubleshooting

**Events not being captured:**
1. Verify daemon is running: `memory-daemon status`
2. Check binary is installed: `which memory-ingest`
3. Test manually with echo command above

**Daemon connection errors:**
- The binary fails open - events are lost but Claude continues
- Check daemon port: default is 50051
- Set custom endpoint: `export MEMORY_ENDPOINT="http://localhost:50052"`

**Hook not triggering:**
- Verify hooks.yaml is in correct location (~/.claude/hooks.yaml for Claude Code)
- Check hooks.yaml syntax with a YAML validator
- Ensure CCH is properly installed and configured

## Documentation

| Document | Description |
|----------|-------------|
| [Cross-Agent Usage Guide](adapters/cross-agent-guide.md) | Using agent-memory with multiple AI agents |
| [Adapter Authoring Guide](adapters/authoring-guide.md) | Building a new adapter for an AI agent |
| [CLOD Format Specification](adapters/clod-format.md) | Universal command definition format |
| [Configuration Reference](references/configuration-reference.md) | Complete configuration options with defaults |
| [Lifecycle Telemetry](references/lifecycle-telemetry.md) | Metrics and monitoring for index lifecycle |
| [UPGRADING](UPGRADING.md) | Version upgrade instructions and migration notes |
| [API Reference](API.md) | gRPC service documentation |
| [Architecture](ARCHITECTURE.md) | Component structure |
| [Design Docs](design/README.md) | Detailed architecture and design documentation |

## Related Projects

- **code_agent_context_hooks** - Hook handlers for Claude Code that feed events into this memory system

## License

MIT

## Contributing

See `.planning/PROJECT.md` for detailed project context and roadmap.
