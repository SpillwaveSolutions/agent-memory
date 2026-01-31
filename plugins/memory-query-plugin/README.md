# Memory Query Plugin

A Claude Code plugin for querying past conversations from the agent-memory system.

## Overview

This plugin enables Claude to recall and search through past conversation history using a time-based Table of Contents (TOC) navigation pattern. It provides both explicit slash commands and an autonomous agent for complex queries.

## Installation

```bash
# Clone to your Claude skills directory
cd ~/.claude/skills
git clone https://github.com/SpillwaveSolutions/memory-query-agentic-plugin.git
```

Or install from the agent-memory monorepo:
```bash
ln -s /path/to/agent-memory/skills/memory-query-plugin ~/.claude/skills/memory-query-plugin
```

## Prerequisites

The memory-daemon must be running:

```bash
memory-daemon start
memory-daemon status
```

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

The **memory-navigator** agent handles complex queries that require:

- Cross-period searches
- Contextual recall with synthesis
- Multi-step TOC navigation
- Vague temporal references

Triggered by patterns like:
- "What did we discuss about..."
- "Remember when we..."
- "Find our previous conversation about..."

## Architecture

```
memory-query-plugin/
├── .claude-plugin/
│   └── marketplace.json     # Plugin manifest
├── skills/
│   └── memory-query/        # Core skill
│       ├── SKILL.md
│       └── references/
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

## License

MIT
