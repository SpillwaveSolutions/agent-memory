# Agent Identifiers

Agent identifiers tag all events from a specific agent instance, enabling filtering, attribution, and multi-agent queries.

## What is an Agent Identifier?

An agent identifier (agent_id) is a string that:

- Tags all events ingested by this agent instance
- Enables filtering queries to specific agents
- Provides attribution in multi-agent setups
- Persists across sessions and restarts

## Identifier Patterns

| Pattern | Example | Use Case |
|---------|---------|----------|
| Simple | `claude-code` | Single user, single machine |
| Host-specific | `claude-code-macbook` | Multi-machine setups |
| User-specific | `alice-claude` | Shared machines |
| Project-specific | `project-x-claude` | Per-project isolation |
| Tool-specific | `cursor-ai` | Different AI tools |
| Combined | `alice-macbook-claude` | Complex environments |

## Identifier Requirements

### Format Rules

- **Length:** 3-50 characters
- **Characters:** Alphanumeric, hyphens (-), underscores (_)
- **Start:** Must start with a letter
- **Case:** Case-sensitive (use lowercase by convention)

### Valid Examples

```
claude-code
cursor-ai
alice-claude
claude-code-macbook-pro
dev_agent_01
```

### Invalid Examples

```
cc              # Too short (< 3 chars)
1-claude        # Cannot start with number
claude code     # No spaces allowed
claude.code     # No dots allowed
my@agent        # No special characters
```

## Choosing an Identifier

### Single User, Single Machine

Use the default:

```toml
agent_id = "claude-code"
```

### Single User, Multiple Machines

Include hostname:

```bash
# Automatic hostname detection
hostname  # Returns: macbook-pro

# Use in config
agent_id = "claude-code-macbook-pro"
```

Or use environment variable:

```bash
export MEMORY_AGENT_ID="claude-code-$(hostname)"
```

### Shared Machine

Include username:

```bash
# Get username
whoami  # Returns: alice

# Use in config
agent_id = "alice-claude"
```

### Multiple AI Tools

Use tool name:

```toml
# For Claude Code
agent_id = "claude-code"

# For Cursor
agent_id = "cursor-ai"

# For VS Code Copilot
agent_id = "vscode-copilot"
```

### Per-Project Isolation

Include project name:

```toml
# Project Alpha
agent_id = "alpha-claude"

# Project Beta
agent_id = "beta-claude"
```

## Environment Variable Override

Set agent ID via environment variable:

```bash
# In shell profile
export MEMORY_AGENT_ID="claude-code-$(hostname)"

# Or per-session
MEMORY_AGENT_ID="test-agent" memory-daemon start
```

Environment variable takes precedence over config file.

## Changing Identifiers

### New Identifier (Fresh Start)

```toml
# Simply change the agent_id
agent_id = "new-agent-id"
```

Previous events remain tagged with old ID. New events use new ID.

### Migrating Events

To re-tag existing events:

```bash
# Export with old ID
memory-daemon admin export --agent old-id --output backup.json

# Re-import with new ID
memory-daemon admin import --agent-id new-id backup.json

# Optional: delete old events
memory-daemon admin delete --agent old-id
```

## Querying by Agent

### Filter to Specific Agent

```bash
# Your agent only
memory-daemon query --agent claude-code "topic"

# Another agent
memory-daemon query --agent cursor-ai "topic"
```

### Cross-Agent Query

```bash
# All agents
memory-daemon query --agent all "topic"

# Multiple specific agents
memory-daemon query --agent "claude-code,cursor-ai" "topic"
```

### Query Scope Configuration

```toml
[agents]
agent_id = "claude-code"
query_scope = "own"           # Only this agent's data
# query_scope = "all"         # All agents' data
# query_scope = "claude-code,cursor-ai"  # Specific agents
```

## Identifier in Events

Events are stored with agent_id metadata:

```json
{
  "id": "evt_abc123",
  "agent_id": "claude-code",
  "timestamp": "2024-01-15T10:30:00Z",
  "session_id": "sess_xyz",
  "content": "User asked about database optimization...",
  "summary": "Discussion of PostgreSQL indexing strategies"
}
```

## Best Practices

### Naming Conventions

1. **Use lowercase:** `claude-code` not `Claude-Code`
2. **Be descriptive:** `alice-macbook-claude` not `a1`
3. **Be consistent:** Use same pattern across machines
4. **Document:** Keep record of agent IDs and their purposes

### Security

1. Don't include sensitive info in agent ID
2. Use random suffix for public environments
3. Consider agent ID as semi-public information

### Maintenance

1. Periodically review agent IDs in use
2. Clean up unused agents
3. Document agent ID assignments

## Configuration Examples

### Development Setup

```toml
[agents]
agent_id = "claude-code-dev"
query_scope = "own"
```

### Multi-Machine Setup

```toml
# Machine 1 (Laptop)
[agents]
agent_id = "claude-code-laptop"
query_scope = "all"

# Machine 2 (Desktop)
[agents]
agent_id = "claude-code-desktop"
query_scope = "all"
```

### Team Setup

```toml
# Alice
[agents]
agent_id = "alice-claude"
query_scope = "all"

# Bob
[agents]
agent_id = "bob-claude"
query_scope = "all"
```
