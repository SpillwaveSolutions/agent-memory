---
name: memory-agents
description: |
  This skill should be used when the user asks to "configure multi-agent memory",
  "set up team memory", "configure agent isolation", "set agent ID",
  "share memory between agents", or "configure cross-agent queries".
  Provides interactive wizard for multi-agent configuration.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Agents Skill

Configure multi-agent memory settings including store isolation, agent tagging, cross-agent query permissions, and team settings.

## When Not to Use

- Initial installation (use `/memory-setup` first)
- Querying past conversations (use memory-query plugin)
- Storage and retention configuration (use `/memory-storage`)
- LLM provider configuration (use `/memory-llm`)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `/memory-agents` | Interactive multi-agent wizard | `/memory-agents` |
| `/memory-agents --single` | Configure for single user mode | `/memory-agents --single` |
| `/memory-agents --team` | Configure for team use | `/memory-agents --team` |
| `/memory-agents --advanced` | Show all organization options | `/memory-agents --advanced` |
| `/memory-agents --fresh` | Re-configure from scratch | `/memory-agents --fresh` |

## Question Flow

```
State Detection
      |
      v
+------------------+
| Step 1: Usage    | <- Always ask (core decision)
| Mode             |
+--------+---------+
         |
    +----+----+
    |         |
    v         v
Single    Multi/Team
    |         |
    |    +----+----+
    |    |         |
    |    v         v
    | +------------------+
    | | Step 2: Storage  | <- Only if multi-agent or team
    | | Strategy         |
    | +--------+---------+
    |          |
    +----+-----+
         |
         v
+------------------+
| Step 3: Agent    | <- Always ask
| Identifier       |
+--------+---------+
         |
    +----+----+
    |         |
 Unified    Separate
    |         |
    v         |
+------------------+
| Step 4: Query    | <- Only if unified store
| Scope            |
+--------+---------+
    |         |
    +----+----+
         |
         v (if --advanced && separate)
+------------------+
| Step 5: Storage  | <- --advanced mode only
| Organization     |
+--------+---------+
         |
         v (if team mode)
+------------------+
| Step 6: Team     | <- Only if team mode selected
| Settings         |
+--------+---------+
         |
         v
    Execution
```

## State Detection

Before beginning configuration, detect current system state.

### Detection Commands

```bash
# Current multi-agent config
grep -A5 '\[agents\]' ~/.config/memory-daemon/config.toml 2>/dev/null

# Current agent_id
grep 'agent_id' ~/.config/memory-daemon/config.toml 2>/dev/null

# Detect other agents in store
ls ~/.memory-store/agents/ 2>/dev/null

# Get hostname and username for identifier suggestions
hostname
whoami

# Check for team config
grep -A5 '\[team\]' ~/.config/memory-daemon/config.toml 2>/dev/null
```

### State Summary Format

```
Current Agent State
-------------------
Mode:            Single user
Storage:         Unified store
Agent ID:        claude-code
Query Scope:     Own events only
Team:            Not configured

Other agents detected: cursor-ai, vscode-copilot

Recommended:     No changes needed for single user
```

## Wizard Steps

### Step 1: Usage Mode

**Always ask (core decision)**

```
question: "How will agent-memory be used?"
header: "Mode"
options:
  - label: "Single user (Recommended)"
    description: "One person, one agent (Claude Code)"
  - label: "Single user, multiple agents"
    description: "One person using Claude Code, Cursor, etc."
  - label: "Team mode"
    description: "Multiple users sharing memory on a team"
multiSelect: false
```

### Step 2: Storage Strategy

**Skip if:** Step 1 selected "Single user"

```
question: "How should agent data be stored?"
header: "Storage"
options:
  - label: "Unified store with tags (Recommended)"
    description: "Single database, agents identified by tag, easy cross-query"
  - label: "Separate stores per agent"
    description: "Complete isolation, cannot query across agents"
multiSelect: false
```

### Step 3: Agent Identifier

**Always ask (important for tracking)**

```
question: "Choose your agent identifier (tags all events from this instance):"
header: "Agent ID"
options:
  - label: "claude-code (Recommended)"
    description: "Standard identifier for Claude Code"
  - label: "claude-code-{hostname}"
    description: "Unique per machine for multi-machine setups (e.g., claude-code-macbook)"
  - label: "{username}-claude"
    description: "User-specific for shared machines (e.g., alice-claude)"
  - label: "Custom"
    description: "Specify a custom identifier"
multiSelect: false
```

**If Custom selected:**

```
question: "Enter your custom agent identifier:"
header: "ID"
type: text
validation: "3-50 characters, alphanumeric with hyphens and underscores"
```

### Step 4: Cross-Agent Query Permissions

**Skip if:** Storage strategy is "separate"

```
question: "What data should queries return?"
header: "Query Scope"
options:
  - label: "Own events only (Recommended)"
    description: "Query only this agent's data"
  - label: "All agents"
    description: "Query all agents' data (read-only)"
  - label: "Specified agents"
    description: "Query specific agents' data"
multiSelect: false
```

**If Specified agents selected:**

```
question: "Enter comma-separated list of agent IDs to include:"
header: "Agents"
type: text
placeholder: "claude-code, cursor-ai, vscode"
```

### Step 5: Storage Organization

**Skip unless:** `--advanced` AND separate storage selected

```
question: "How should separate stores be organized?"
header: "Organization"
options:
  - label: "~/.memory-store/{agent_id}/ (Recommended)"
    description: "Agent-specific subdirectories under main storage"
  - label: "Custom paths"
    description: "Specify custom storage paths per agent"
multiSelect: false
```

### Step 6: Team Settings

**Skip unless:** Step 1 selected "Team mode"

```
question: "Configure team sharing settings?"
header: "Team"
options:
  - label: "Read-only sharing (Recommended)"
    description: "See team events, write to own store only"
  - label: "Full sharing"
    description: "All team members read/write to shared store"
  - label: "Custom permissions"
    description: "Configure per-agent permissions"
multiSelect: false
```

**Additional team questions:**

```
question: "Enter team name:"
header: "Name"
type: text
default: "default"
```

```
question: "Enter shared storage path:"
header: "Path"
type: text
default: "~/.memory-store/team/"
```

## Config Generation

After wizard completion, generate or update config.toml:

```bash
# Create or update agents section
cat >> ~/.config/memory-daemon/config.toml << 'EOF'

[agents]
mode = "single"
storage_strategy = "unified"
agent_id = "claude-code"
query_scope = "own"

[team]
name = "default"
storage_path = "~/.memory-store/team/"
shared = false
EOF
```

### Config Value Mapping

| Wizard Choice | Config Values |
|---------------|---------------|
| Single user | `mode = "single"` |
| Single user, multiple agents | `mode = "multi"` |
| Team mode | `mode = "team"` |
| Unified store with tags | `storage_strategy = "unified"` |
| Separate stores per agent | `storage_strategy = "separate"` |
| Own events only | `query_scope = "own"` |
| All agents | `query_scope = "all"` |
| Specified agents | `query_scope = "agent1,agent2"` |
| Read-only sharing | `[team] shared = false` |
| Full sharing | `[team] shared = true` |

## Validation

Before applying configuration, validate:

```bash
# 1. Agent ID format valid
echo "$AGENT_ID" | grep -E '^[a-zA-Z][a-zA-Z0-9_-]{2,49}$' && echo "[check] Agent ID format OK" || echo "[x] Invalid agent ID format"

# 2. Agent ID unique in unified store (if applicable)
if [ "$STORAGE_STRATEGY" = "unified" ]; then
  memory-daemon admin list-agents 2>/dev/null | grep -q "^$AGENT_ID$" && echo "[!] Agent ID already exists" || echo "[check] Agent ID unique"
fi

# 3. Storage path writable
mkdir -p "$STORAGE_PATH" 2>/dev/null && echo "[check] Storage path writable" || echo "[x] Cannot create storage path"

# 4. Team path accessible (if shared)
if [ "$MODE" = "team" ]; then
  touch "$TEAM_PATH/.test" 2>/dev/null && rm "$TEAM_PATH/.test" && echo "[check] Team path writable" || echo "[x] Team path not writable"
fi
```

## Output Formatting

### Success Display

```
==================================================
 Agent Configuration Complete!
==================================================

[check] Mode: Single user
[check] Storage: Unified store
[check] Agent ID: claude-code
[check] Query Scope: Own events only

Configuration written to ~/.config/memory-daemon/config.toml

Next steps:
  * Restart daemon: memory-daemon restart
  * Configure storage: /memory-storage
  * Configure LLM: /memory-llm
```

### Multi-Agent Success Display

```
==================================================
 Multi-Agent Configuration Complete!
==================================================

[check] Mode: Single user, multiple agents
[check] Storage: Unified store with tags
[check] Agent ID: claude-code-macbook
[check] Query Scope: All agents (read-only)

Other agents in this store:
  - cursor-ai (last active: 2 hours ago)
  - vscode-copilot (last active: 1 day ago)

Configuration written to ~/.config/memory-daemon/config.toml

Tip: Use 'memory-daemon query --agent cursor-ai <topic>' to search other agents' data.
```

### Team Success Display

```
==================================================
 Team Configuration Complete!
==================================================

[check] Mode: Team
[check] Storage: Unified store with tags
[check] Agent ID: alice-claude
[check] Team: engineering (read-only sharing)
[check] Shared Path: /shared/memory-store/

Team members:
  - alice-claude (you)
  - bob-cursor
  - charlie-copilot

Configuration written to ~/.config/memory-daemon/config.toml

Tip: All team events are visible. Your events are tagged as 'alice-claude'.
```

### Error Display

```
[x] Agent Configuration Failed
-------------------------------

Error: Agent ID 'claude-code' already exists in this store

To fix:
  1. Choose a unique identifier (e.g., 'claude-code-macbook')
  2. Or use separate storage strategy

Re-run: /memory-agents --fresh
```

## Mode Behaviors

### Default Mode (`/memory-agents`)

- Runs state detection
- Shows all applicable steps based on selections
- Skips configured options unless changes needed

### Single Mode (`/memory-agents --single`)

- Shortcut for single user configuration
- Sets defaults without asking
- Fastest path to configuration

### Team Mode (`/memory-agents --team`)

- Enables team-specific questions
- Configures shared storage
- Sets up team permissions

### Advanced Mode (`/memory-agents --advanced`)

- Shows storage organization options
- Enables custom path configuration
- Shows all available options

### Fresh Mode (`/memory-agents --fresh`)

- Ignores existing configuration
- Asks all questions from scratch
- Useful for reconfiguration

## Reference Files

For detailed information, see:

- [Storage Strategies](references/storage-strategies.md) - Unified vs separate storage
- [Team Setup](references/team-setup.md) - Team mode configuration
- [Agent Identifiers](references/agent-identifiers.md) - Identifier patterns and rules

## Related Skills

After agent configuration, consider:

- `/memory-storage` - Configure storage and retention
- `/memory-llm` - Configure LLM provider
- `/memory-setup` - Full installation wizard
- `/memory-status` - Check current system status
