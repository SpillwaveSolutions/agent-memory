# Memory Query Plugin

A Claude Code plugin for intelligent memory retrieval with tier-aware routing, intent classification, and automatic fallback chains.

## Overview

This plugin enables Claude to recall and search through past conversation history using a layered cognitive architecture. It automatically detects available search capabilities (Topics, Hybrid, Semantic, Keyword, Agentic) and routes queries through optimal layers with intelligent fallbacks.

## Features

- **Tier-Aware Routing**: Automatically detects available search layers (Tier 1-5)
- **Intent Classification**: Routes Explore/Answer/Locate/Time-boxed queries optimally
- **Fallback Chains**: Gracefully degrades when layers are unavailable
- **Explainability**: Every query result includes tier used, layers tried, and fallbacks
- **Salience Ranking**: Results ranked by importance, recency, and relevance

## Installation

```bash
# Clone to your Claude skills directory
cd ~/.claude/skills
git clone https://github.com/SpillwaveSolutions/memory-query-agentic-plugin.git
```

Or install from the agent-memory monorepo:
```bash
ln -s /path/to/agent-memory/plugins/memory-query-plugin ~/.claude/skills/memory-query-plugin
```

## Prerequisites

The memory-daemon must be running:

```bash
memory-daemon start
memory-daemon status

# Check retrieval tier
memory-daemon retrieval status
```

## Capability Tiers

| Tier | Name | Layers | Best For |
|------|------|--------|----------|
| 1 | Full | Topics + Hybrid + Agentic | Semantic exploration |
| 2 | Hybrid | BM25 + Vector + Agentic | Balanced search |
| 3 | Semantic | Vector + Agentic | Conceptual queries |
| 4 | Keyword | BM25 + Agentic | Exact term matching |
| 5 | Agentic | TOC only | Always works |

## Commands

| Command | Description |
|---------|-------------|
| `/memory-search <topic>` | Search conversations by topic or keyword |
| `/memory-recent` | Show recent conversation summaries |
| `/memory-context <grip-id>` | Expand a specific memory excerpt |

### Examples

```
/memory-search authentication
/memory-search "JWT tokens" --period "last week"
/memory-recent --days 7
/memory-context grip:1706540400000:01HN4QXKN6
```

## Agent

The **memory-navigator** agent handles complex queries with full tier awareness:

- **Explore intent**: "What topics have we discussed recently?"
- **Answer intent**: "What approaches have we tried for caching?"
- **Locate intent**: "Find the exact error message from JWT validation"
- **Time-boxed intent**: "What happened in yesterday's debugging session?"

Includes explainability in every response:
```
Method: Hybrid tier (BM25 + Vector reranking)
Layers: bm25 (5 results), vector (3 results)
Fallbacks: 0
Confidence: 0.87
```

## Skills

| Skill | Purpose |
|-------|---------|
| `memory-query` | Core query capability with tier awareness |
| `retrieval-policy` | Tier detection, intent classification, fallbacks |
| `topic-graph` | Topic exploration (Tier 1) |
| `bm25-search` | Keyword search (Tier 2-4) |
| `vector-search` | Semantic search (Tier 2-3) |

## Architecture

```
memory-query-plugin/
├── .claude-plugin/
│   └── marketplace.json     # Plugin manifest (v2.0.0)
├── skills/
│   ├── memory-query/        # Core query skill
│   ├── retrieval-policy/    # Tier detection & routing
│   ├── topic-graph/         # Topic exploration
│   ├── bm25-search/         # Keyword search
│   └── vector-search/       # Semantic search
├── commands/                # Slash commands
│   ├── memory-search.md
│   ├── memory-recent.md
│   └── memory-context.md
├── agents/                  # Autonomous agents
│   └── memory-navigator.md
└── README.md
```

## Related

- [agent-memory](https://github.com/SpillwaveSolutions/agent-memory) - The memory daemon and storage system
- [code_agent_context_hooks](https://github.com/SpillwaveSolutions/code_agent_context_hooks) - Hook integration for automatic event capture

## Version History

- **v2.0.0**: Tier-aware routing, intent classification, fallback chains (Phase 16-17)
- **v1.0.0**: Basic TOC navigation and search

## License

MIT
