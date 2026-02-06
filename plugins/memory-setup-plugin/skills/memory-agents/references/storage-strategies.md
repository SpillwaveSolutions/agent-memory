# Storage Strategies

Multi-agent storage strategies control how agent-memory isolates or shares data between different AI agents.

## Overview

When multiple AI agents (Claude Code, Cursor, VS Code Copilot, etc.) use agent-memory, you can choose how their data is organized.

## Strategy Comparison

| Strategy | Isolation | Cross-Query | Complexity | Disk Usage | Use Case |
|----------|-----------|-------------|------------|------------|----------|
| Unified with tags | Logical | Yes (configurable) | Simple | Lower | Most multi-agent setups |
| Separate stores | Physical | No | Higher | Higher | Strict isolation needed |

## Unified Store with Tags (Recommended)

All agents share a single database. Events are tagged with their source agent ID.

### How It Works

```
~/.memory-store/
├── db/                  # Single database
│   ├── events/          # All agents' events
│   └── indices/         # Shared indices
└── metadata.json        # Store metadata
```

Each event includes:
```json
{
  "id": "evt_123",
  "agent_id": "claude-code",
  "timestamp": "2024-01-15T10:30:00Z",
  "content": "..."
}
```

### Query Filtering

```bash
# Query only your agent's data
memory-daemon query --agent claude-code "topic"

# Query all agents' data
memory-daemon query --agent all "topic"

# Query specific agents
memory-daemon query --agent "claude-code,cursor-ai" "topic"
```

### Pros

- Single backup location
- Easy cross-agent search
- Lower disk usage (shared indices)
- Simple configuration
- Unified search results

### Cons

- All data in one database
- Requires query discipline for isolation
- Shared performance impact

### Configuration

```toml
[agents]
mode = "multi"
storage_strategy = "unified"
agent_id = "claude-code"
query_scope = "own"  # or "all" or "claude-code,cursor-ai"
```

## Separate Stores

Each agent has its own independent database. Complete physical isolation.

### How It Works

```
~/.memory-store/
├── claude-code/         # Claude Code database
│   ├── db/
│   └── metadata.json
├── cursor-ai/           # Cursor database
│   ├── db/
│   └── metadata.json
└── vscode-copilot/      # VS Code Copilot database
    ├── db/
    └── metadata.json
```

### Pros

- Maximum isolation
- Independent backups
- Per-agent storage limits
- No cross-agent data leaks
- Independent performance

### Cons

- No cross-agent queries
- Higher disk usage (separate indices)
- More complex configuration
- Multiple databases to manage

### Configuration

```toml
[agents]
mode = "multi"
storage_strategy = "separate"
agent_id = "claude-code"
storage_path = "~/.memory-store/claude-code/"
```

## Decision Tree

```
Do you need to search across agents?
├── YES
│   └── Use Unified Store
│       └── Want isolation by default?
│           ├── YES → query_scope = "own"
│           └── NO  → query_scope = "all"
│
└── NO
    └── Is privacy critical between agents?
        ├── YES → Use Separate Stores
        └── NO  → Use Unified Store (simpler)
```

## Migration

### Unified to Separate

```bash
# Export each agent's data
for agent in $(memory-daemon admin list-agents); do
  memory-daemon admin export --agent "$agent" --output "$agent.json"
done

# Create separate stores
for agent in $(memory-daemon admin list-agents); do
  mkdir -p ~/.memory-store/$agent
  memory-daemon admin import --db-path ~/.memory-store/$agent "$agent.json"
done
```

### Separate to Unified

```bash
# Create unified store
mkdir -p ~/.memory-store/unified

# Import each agent's data with tags
for agent_dir in ~/.memory-store/*/; do
  agent=$(basename "$agent_dir")
  memory-daemon admin import \
    --db-path ~/.memory-store/unified \
    --agent-id "$agent" \
    "$agent_dir/export.json"
done
```

## Best Practices

### Unified Store

1. Always set appropriate `query_scope`
2. Use consistent agent ID naming
3. Regular backups of single store
4. Monitor total storage usage

### Separate Stores

1. Use automation for backups
2. Consider disk space per agent
3. Document which agent uses which path
4. Set up monitoring per store

## Configuration Examples

### Single User, Multiple Agents (Unified)

```toml
[agents]
mode = "multi"
storage_strategy = "unified"
agent_id = "claude-code"
query_scope = "all"
```

### Enterprise Isolation (Separate)

```toml
[agents]
mode = "multi"
storage_strategy = "separate"
agent_id = "secure-agent"
storage_path = "/secure/memory-store/secure-agent/"
```

### Team Mode (Unified with Sharing)

```toml
[agents]
mode = "team"
storage_strategy = "unified"
agent_id = "alice-claude"
query_scope = "all"

[team]
name = "engineering"
storage_path = "/shared/team-memory/"
shared = true
```
