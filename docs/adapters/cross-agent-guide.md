# Cross-Agent Usage Guide

## Overview

Agent Memory supports multiple AI coding agents simultaneously. Each agent captures conversation events independently, but all data flows into a shared memory store. This guide explains how to work with multiple agents, discover which agents contributed memories, and query across or within specific agents.

## Supported Agents

Agent Memory provides adapters for four AI coding agents:

| Feature | Claude Code | OpenCode | Gemini CLI | Copilot CLI |
|---------|-------------|----------|------------|-------------|
| **Event Capture** | hooks.yaml (CCH) | Plugin (TypeScript) | settings.json hooks | hooks.json |
| **Commands** | .md + YAML frontmatter | .md + $ARGUMENTS | .toml commands | Skills (embedded) |
| **Skills** | .claude/skills/ | .opencode/skill/ | .gemini/skills/ | .github/skills/ |
| **Agent Tag** | `claude` | `opencode` | `gemini` | `copilot` |
| **Session Source** | Hook event session_id | Hook event session_id | Temp file rotation | Temp file rotation |
| **Install Method** | Plugin marketplace | Plugin | Install skill / manual | Plugin / install skill |

All adapters share the same underlying memory daemon and storage. Events are tagged with the originating agent so you can query across all agents or filter to a specific one.

## Installation

Each adapter has its own installation process. See the adapter-specific README for detailed instructions:

| Agent | Setup Guide |
|-------|-------------|
| Claude Code | [Claude Code Setup](../../plugins/memory-query-plugin/README.md) |
| OpenCode | [OpenCode Plugin Setup](../../plugins/memory-opencode-plugin/README.md) |
| Gemini CLI | [Gemini Adapter Setup](../../plugins/memory-gemini-adapter/README.md) |
| Copilot CLI | [Copilot Adapter Setup](../../plugins/memory-copilot-adapter/README.md) |

### Prerequisites

All adapters require:

1. **memory-daemon** binary installed and running
2. **memory-ingest** binary installed (for hook-based capture)
3. The agent itself installed and configured

```bash
# Build and install binaries
cargo build --release -p memory-daemon -p memory-ingest
cp target/release/memory-daemon ~/.local/bin/
cp target/release/memory-ingest ~/.local/bin/

# Start the daemon
memory-daemon start
```

## Agent Discovery

### Listing Agents

See which agents have contributed memories:

```bash
$ memory-daemon agents list

Contributing Agents:
  AGENT            FIRST SEEN               LAST SEEN                NODES
  claude           2026-01-15 10:30 UTC     2026-02-10 14:22 UTC      847
  opencode         2026-02-01 09:00 UTC     2026-02-10 13:45 UTC      156
  gemini           2026-02-05 11:15 UTC     2026-02-09 16:30 UTC       42
  copilot          2026-02-08 08:00 UTC     2026-02-10 12:00 UTC       23
```

The `NODES` column shows the approximate number of TOC nodes each agent contributed to. This is an O(k) operation over TOC nodes, not a full event scan.

### Agent Activity

View agent activity over time:

```bash
# All agents, daily buckets (default)
$ memory-daemon agents activity

Agent Activity (day buckets):
  DATE           AGENT            EVENTS
  2026-02-08     claude               34
  2026-02-08     opencode             12
  2026-02-09     claude               28
  2026-02-09     gemini                8
  2026-02-10     claude               15
  2026-02-10     opencode              7
  2026-02-10     copilot               3
```

```bash
# Filter to a specific agent
$ memory-daemon agents activity --agent claude

Agent Activity (day buckets):
  DATE           AGENT            EVENTS
  2026-02-08     claude               34
  2026-02-09     claude               28
  2026-02-10     claude               15
```

```bash
# Specify time range and weekly buckets
$ memory-daemon agents activity --from 2026-02-01 --to 2026-02-10 --bucket week

Agent Activity (week buckets):
  DATE           AGENT            EVENTS
  2026-02-03     claude              142
  2026-02-03     opencode             56
  2026-02-03     gemini               21
  2026-02-10     claude               77
  2026-02-10     opencode             19
  2026-02-10     copilot               3
```

Time arguments accept both `YYYY-MM-DD` format and Unix epoch milliseconds.

### Topics by Agent

View top topics for a specific agent using the retrieval route with agent filter:

```bash
$ memory-daemon retrieval route "what topics were discussed" --agent opencode

Query Routing
----------------------------------------------------------------------
Query: "what topics were discussed"

Results (5 found):
----------------------------------------------------------------------
1. [Agentic] toc:week:2026-W06 (score: 0.8500)
   OpenCode plugin development and testing
   Type: toc
   Agent: opencode
```

## Cross-Agent Queries

### Default: All Agents

By default, all queries return results from all agents. There is no need to specify an agent filter to get comprehensive results:

```bash
# Returns results from claude, opencode, gemini, copilot
$ memory-daemon retrieval route "authentication implementation"
```

This is the recommended approach for most queries -- you get the broadest context across all your coding sessions regardless of which agent you used.

### Filtered Queries

Use the `--agent` flag to restrict results to a specific agent:

```bash
# Only results from Claude Code sessions
$ memory-daemon retrieval route "authentication" --agent claude

# Only results from OpenCode sessions
$ memory-daemon retrieval route "authentication" --agent opencode

# BM25 keyword search filtered to Gemini
$ memory-daemon teleport search "JWT tokens" --agent gemini

# Vector semantic search filtered to Copilot
$ memory-daemon teleport vector-search --query "error handling patterns" --agent copilot

# Hybrid search filtered to Claude
$ memory-daemon teleport hybrid-search --query "database migrations" --agent claude
```

### Retrieval with Agent Context

When results include agent information, the output shows which agent the memory came from:

```bash
$ memory-daemon retrieval route "what did we discuss about testing"

Results (3 found):
----------------------------------------------------------------------
1. [BM25] toc:day:2026-02-09:seg-3 (score: 0.9200)
   Discussed integration testing patterns for gRPC services
   Type: toc
   Agent: claude

2. [Vector] toc:day:2026-02-08:seg-1 (score: 0.8700)
   Set up test fixtures for OpenCode plugin
   Type: toc
   Agent: opencode

3. [Agentic] toc:day:2026-02-07:seg-2 (score: 0.7500)
   Reviewed unit test coverage for adapter hooks
   Type: toc
   Agent: gemini
```

The `Agent` field in each result tells you which coding agent was used during that conversation. This helps you understand the context of each memory.

## Common Workflows

### 1. "What was I discussing in OpenCode last week?"

Combine agent activity with filtered retrieval:

```bash
# Check what was happening last week
$ memory-daemon agents activity --agent opencode --from 2026-02-03 --to 2026-02-09

# Search within OpenCode sessions
$ memory-daemon retrieval route "what was I working on" --agent opencode
```

### 2. "Show me topics shared between Claude and Gemini"

Compare topics across agents:

```bash
# Get Claude topics
$ memory-daemon retrieval route "main topics" --agent claude

# Get Gemini topics
$ memory-daemon retrieval route "main topics" --agent gemini

# Search across both for a specific topic
$ memory-daemon retrieval route "authentication" --agent claude
$ memory-daemon retrieval route "authentication" --agent gemini
```

### 3. "Find all conversations about authentication"

Search across all agents without a filter:

```bash
# Broad search across all agents
$ memory-daemon retrieval route "authentication implementation"

# Keyword-specific search
$ memory-daemon teleport search "JWT OAuth token"

# Semantic search for conceptual matches
$ memory-daemon teleport vector-search --query "how did we handle user login"
```

### 4. "Which agent had the most activity today?"

Use agent listing and activity:

```bash
# Quick overview
$ memory-daemon agents list

# Today's activity breakdown
$ memory-daemon agents activity --from 2026-02-10
```

### 5. "Continue a conversation from a different agent"

When switching from one agent to another, search for the prior context:

```bash
# You were using Claude but now using OpenCode
# Find what you discussed in Claude
$ memory-daemon retrieval route "the feature I was building" --agent claude

# The results give you context to continue in OpenCode
```

## How Agent Tagging Works

Each adapter sets an `agent` field in the event payload during ingestion:

- **Claude Code**: The `memory-ingest` binary sets `agent: "claude"` based on the CCH hook environment
- **OpenCode**: The plugin explicitly sets `agent: "opencode"` in the event JSON
- **Gemini CLI**: The hook script sets `agent: "gemini"` in the JSON payload
- **Copilot CLI**: The hook script sets `agent: "copilot"` in the JSON payload

The agent tag is stored in the event metadata and propagated to TOC nodes via `TocNode.contributing_agents`. This enables efficient agent discovery without scanning all events.

## Data Model

All agents share the same underlying data model:

```
Event {
    event_id: ULID,
    session_id: String,
    timestamp: i64,
    event_type: String,
    role: String,
    text: String,
    agent: String,          // "claude", "opencode", "gemini", "copilot"
    metadata: HashMap,
}
```

Events are stored in a single RocksDB database. The TOC hierarchy (Year > Month > Week > Day > Segment) spans all agents. Each TOC node tracks which agents contributed to it via the `contributing_agents` field.

## Troubleshooting

### No agents appearing in `agents list`

1. Verify the daemon is running: `memory-daemon status`
2. Check that events have been ingested: `memory-daemon query root`
3. Verify the adapter's hook/plugin is configured correctly (see adapter README)
4. Test event capture manually: send a test event via the adapter's hook script

### Agent filter returning no results

1. Check the agent name is correct (lowercase): `claude`, `opencode`, `gemini`, `copilot`
2. Verify the agent has events: `memory-daemon agents list`
3. Try without the agent filter to see if results exist at all
4. Check the time range if using `--from`/`--to` flags

### Events missing agent tag

Events ingested before the agent tagging feature (v2.1) will not have an agent tag. These events appear in all queries (no agent filter) but are not associated with any specific agent.

To identify which events lack agent tags, browse the TOC and look for nodes where `contributing_agents` is empty.

### Inconsistent activity counts

Agent activity counts are derived from time-bounded event scans. The `agents list` command uses TOC node counts (approximate). For exact counts, use `agents activity` with a specific time range.
