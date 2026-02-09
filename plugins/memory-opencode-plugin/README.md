# Memory Query Plugin for OpenCode

A plugin for [OpenCode](https://opencode.ai/) that provides intelligent memory retrieval with tier-aware routing, intent classification, and automatic fallback chains.

**Version:** 2.0.0

## Overview

This plugin enables OpenCode to recall and search through past conversation history using a layered cognitive architecture. It automatically detects available search capabilities (Topics, Hybrid, Semantic, Keyword, Agentic) and routes queries through optimal layers with intelligent fallbacks.

## Prerequisites

- **memory-daemon** installed and running ([agent-memory](https://github.com/SpillwaveSolutions/agent-memory))
- **OpenCode** installed ([opencode.ai](https://opencode.ai/))

Verify the daemon is running:

```bash
memory-daemon status
memory-daemon start   # Start if not running
```

## Installation

### Global Installation

Copy the plugin files to your OpenCode global configuration directory:

```bash
cp -r plugins/memory-opencode-plugin/.opencode/* ~/.config/opencode/
```

This makes the commands, skills, and agent available in all projects.

### Per-Project Installation

Symlink or copy the `.opencode` directory into your project root:

```bash
# Option 1: Symlink (recommended for development)
ln -s /path/to/agent-memory/plugins/memory-opencode-plugin/.opencode .opencode

# Option 2: Copy
cp -r /path/to/agent-memory/plugins/memory-opencode-plugin/.opencode .opencode
```

Per-project installation makes the plugin available only within that project.

## Commands

| Command | Description |
|---------|-------------|
| `/memory-search <topic>` | Search conversations by topic or keyword |
| `/memory-recent` | Show recent conversation summaries |
| `/memory-context <grip-id>` | Expand a specific memory excerpt |

### /memory-search

Search past conversations by topic or keyword.

```
/memory-search <topic> [--period <value>]
```

**Examples:**

```
/memory-search authentication
/memory-search "JWT tokens" --period "last week"
/memory-search "database migration" --period january
```

**Arguments:**
- `<topic>` -- Topic or keyword to search (required)
- `--period <value>` -- Time period filter (optional)

### /memory-recent

Display recent conversation summaries.

```
/memory-recent [--days N] [--limit N]
```

**Examples:**

```
/memory-recent
/memory-recent --days 3
/memory-recent --days 14 --limit 20
```

**Arguments:**
- `--days <N>` -- Number of days to look back (default: 7)
- `--limit <N>` -- Maximum segments to show (default: 10)

### /memory-context

Expand a grip ID to see full conversation context around an excerpt.

```
/memory-context <grip-id> [--before N] [--after N]
```

**Examples:**

```
/memory-context grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
/memory-context grip:1706540400000:01HN4QXKN6 --before 10 --after 10
```

**Arguments:**
- `<grip-id>` -- Grip ID to expand (required, format: `grip:{timestamp}:{ulid}`)
- `--before <N>` -- Events to include before excerpt (default: 3)
- `--after <N>` -- Events to include after excerpt (default: 3)

## Agent

The **memory-navigator** agent handles complex queries with full tier awareness and intelligent routing.

### Invocation

Use `@memory-navigator` followed by your query:

```
@memory-navigator What topics have we discussed recently?
@memory-navigator What approaches have we tried for caching?
@memory-navigator Find the exact error message from JWT validation
@memory-navigator What happened in yesterday's debugging session?
```

### When to Use

Use `@memory-navigator` when your query benefits from intelligent routing:

- **Explore intent** -- "What topics have we discussed recently?"
- **Answer intent** -- "What approaches have we tried for caching?"
- **Locate intent** -- "Find the exact error message from JWT validation"
- **Time-boxed intent** -- "What happened in yesterday's debugging session?"

The agent automatically classifies your query intent, selects the optimal retrieval tier, and falls back through layers as needed. Every response includes explainability metadata showing the method used.

## Skills

| Skill | Purpose | When Used |
|-------|---------|-----------|
| `memory-query` | Core query capability with tier awareness | All memory retrieval operations |
| `retrieval-policy` | Tier detection, intent classification, fallbacks | Query routing and capability detection |
| `topic-graph` | Topic exploration and discovery | Tier 1 (Full) -- when topic index is available |
| `bm25-search` | Keyword search via BM25 index | Tier 1-4 -- when BM25 index is available |
| `vector-search` | Semantic similarity search | Tier 1-3 -- when vector index is available |

## Retrieval Tiers

The plugin automatically detects available search capabilities and routes queries through the optimal tier. Higher tiers provide more search layers; lower tiers gracefully degrade.

| Tier | Name | Capabilities | Best For |
|------|------|--------------|----------|
| 1 | Full | Topics + Hybrid + Agentic | Semantic exploration, topic discovery |
| 2 | Hybrid | BM25 + Vector + Agentic | Balanced keyword + semantic search |
| 3 | Semantic | Vector + Agentic | Conceptual similarity queries |
| 4 | Keyword | BM25 + Agentic | Exact term matching |
| 5 | Agentic | TOC navigation only | Always works (no indices required) |

Check your current tier:

```bash
memory-daemon retrieval status
```

Tier 5 (Agentic) is always available and requires no indices. As you build BM25 and vector indices, the system automatically upgrades to higher tiers with more powerful search capabilities.

## Architecture

```
plugins/memory-opencode-plugin/
├── .opencode/
│   ├── command/                    # Slash commands
│   │   ├── memory-search.md
│   │   ├── memory-recent.md
│   │   └── memory-context.md
│   └── skill/                      # Skills (folder per skill)
│       ├── memory-query/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── retrieval-policy/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── topic-graph/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── bm25-search/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       └── vector-search/
│           ├── SKILL.md
│           └── references/
│               └── command-reference.md
├── README.md
└── .gitignore
```

## Event Capture

The plugin includes an automatic event capture system that records your OpenCode sessions into agent-memory. This enables cross-agent memory sharing -- conversations from OpenCode become searchable alongside Claude Code sessions.

### How It Works

The event capture plugin (`.opencode/plugin/memory-capture.ts`) hooks into OpenCode lifecycle events:

| Event | Hook | What's Captured |
|-------|------|----------------|
| Session start | `session.created` | New session with project directory |
| Session end | `session.idle` | Session checkpoint/completion |
| User messages | `message.updated` | User prompts |
| Assistant responses | `message.updated` | AI responses |
| Tool executions | `tool.execute.after` | Tool name and arguments |

All events are automatically tagged with `agent:opencode` and include the project directory for context.

### Prerequisites

- `memory-ingest` binary in PATH (installed with agent-memory)
- `memory-daemon` running (events are silently dropped if daemon is unavailable)

### Behavior

- **Fail-open**: Event capture never blocks OpenCode. If `memory-ingest` is not available or the daemon is down, events are silently dropped.
- **Automatic**: No configuration needed. The plugin activates when OpenCode loads the plugin directory.
- **Cross-agent queries**: Once events are captured, use `memory-daemon retrieval route "query"` to search across both Claude Code and OpenCode sessions. Use `--agent opencode` to filter to OpenCode-only results.

### Configuration

| Environment Variable | Default | Purpose |
|---------------------|---------|---------|
| `MEMORY_INGEST_PATH` | `memory-ingest` | Override path to memory-ingest binary |

### Verifying Capture

After an OpenCode session, verify events were captured:

```bash
# Search for recent events
memory-daemon query events --from $(date -v-1H +%s000) --to $(date +%s000) --limit 5

# Search with agent filter
memory-daemon retrieval route "your query" --agent opencode
```

## Troubleshooting

### Daemon not running

**Symptom:** "Connection refused" errors from commands.

**Solution:**

```bash
memory-daemon start
memory-daemon status   # Verify it shows "running"
```

### No results found

**Symptom:** Commands return empty results.

**Possible causes:**
- No conversation data has been ingested yet
- Search terms do not match any stored content
- Time period filter is too narrow

**Solution:**
- Verify data exists: `memory-daemon query root` should show year nodes
- Broaden your search terms or remove the `--period` filter
- Try `/memory-recent` to see what data is available

### Connection refused

**Symptom:** Commands fail with connection errors.

**Solution:**

```bash
# Check if daemon is listening
memory-daemon status

# Start with explicit endpoint
memory-daemon start --endpoint http://[::1]:50051

# Verify connectivity
memory-daemon query --endpoint http://[::1]:50051 root
```

### Skills not loading

**Symptom:** Commands or agent not available in OpenCode.

**Possible causes:**
- Plugin not installed in a recognized path
- Skill directory name does not match skill name in SKILL.md

**Solution:**
- Verify installation path: `ls ~/.config/opencode/skill/` or `ls .opencode/skill/`
- Ensure directory names are lowercase with hyphens only

## Related

- [agent-memory](https://github.com/SpillwaveSolutions/agent-memory) -- The memory daemon and storage system
- [memory-query-plugin](../memory-query-plugin/) -- Claude Code version of this plugin
- [code_agent_context_hooks](https://github.com/SpillwaveSolutions/code_agent_context_hooks) -- Hook integration for automatic event capture

## Version History

- **v2.0.0**: Tier-aware routing, intent classification, fallback chains, OpenCode native format
- **v1.0.0**: Basic TOC navigation and search (Claude Code only)

## License

MIT
