# Adapter Authoring Guide

## Overview

This guide explains how to create a new agent-memory adapter for an AI coding agent. An adapter connects an agent's event system to the agent-memory daemon, enabling conversation capture, memory retrieval, and cross-agent discovery.

If you are looking to **use** an existing adapter, see the [Cross-Agent Usage Guide](cross-agent-guide.md) instead.

## Architecture

An adapter bridges two concerns:

1. **Event Capture**: Hook into the agent's lifecycle events and feed them to `memory-ingest`
2. **Skills/Commands**: Provide the agent with commands to query the memory system

```
┌────────────────────────────────────┐
│        AI Coding Agent             │
│  (Claude, OpenCode, Gemini, etc.)  │
└──────────┬─────────────┬───────────┘
           │             │
   Hook/Plugin     Skills/Commands
   Events              │
           │             │
           ▼             ▼
┌──────────────┐  ┌──────────────┐
│ memory-ingest│  │ memory-daemon│
│ (stdin JSON) │  │ (gRPC CLI)   │
└──────┬───────┘  └──────────────┘
       │
       ▼
┌──────────────────────┐
│   Memory Daemon      │
│   (gRPC Server)      │
│                      │
│   ┌──────────────┐   │
│   │   RocksDB    │   │
│   └──────────────┘   │
└──────────────────────┘
```

## The AgentAdapter Trait

The core interface for adapters is defined in `crates/memory-adapters/src/lib.rs`:

```rust
pub trait AgentAdapter {
    /// Canonical lowercase agent identifier (e.g., "claude", "opencode")
    fn agent_name(&self) -> &str;

    /// Detect if running in this agent's environment
    fn detect(&self) -> bool;

    /// Return adapter configuration (paths, settings)
    fn config(&self) -> AdapterConfig;
}
```

### `agent_name()`

Returns the canonical, lowercase identifier for this agent. This string is used as the `agent` field in events and for `--agent` filter matching.

**Convention**: Use a single lowercase word. Examples: `"claude"`, `"opencode"`, `"gemini"`, `"copilot"`.

### `detect()`

Returns `true` if the current environment is running inside this agent. Detection methods vary by agent:

- **Claude Code**: Check for `CLAUDE_CODE` environment variable or `.claude/` directory
- **OpenCode**: Check for `.opencode/` directory or `OPENCODE_HOME` variable
- **Gemini CLI**: Check for `.gemini/` directory or `GEMINI_API_KEY` variable
- **Copilot CLI**: Check for `.github/copilot/` directory

### `config()`

Returns an `AdapterConfig` with paths and settings:

```rust
pub struct AdapterConfig {
    /// Where hook scripts/configs live (e.g., ".claude/hooks.yaml")
    pub hooks_path: PathBuf,
    /// Where skills are installed (e.g., ".claude/skills/")
    pub skills_path: PathBuf,
    /// Where commands are installed (e.g., "commands/")
    pub commands_path: PathBuf,
    /// Additional adapter-specific settings
    pub settings: HashMap<String, String>,
}
```

## Event Capture

Event capture is the core function of an adapter. The agent's lifecycle events must be transformed into JSON and piped to the `memory-ingest` binary.

### Hook Script Pattern

Most agents support a hook or plugin system that fires on lifecycle events. The general pattern is:

```bash
#!/usr/bin/env bash
# Hook script for <agent-name> adapter

# 1. Read the event from stdin or arguments
EVENT_JSON=$(cat)

# 2. Extract relevant fields
EVENT_TYPE=$(echo "$EVENT_JSON" | jq -r '.event_type // .hook_event_name // "unknown"')
SESSION_ID=$(echo "$EVENT_JSON" | jq -r '.session_id // "unknown"')

# 3. Transform to memory-ingest format
MEMORY_EVENT=$(jq -n \
    --arg event_type "$EVENT_TYPE" \
    --arg session_id "$SESSION_ID" \
    --arg agent "<agent-name>" \
    --arg timestamp "$(date +%s000)" \
    '{
        hook_event_name: $event_type,
        session_id: $session_id,
        agent: $agent,
        timestamp: ($timestamp | tonumber)
    }')

# 4. Pipe to memory-ingest (backgrounded, fail-open)
echo "$MEMORY_EVENT" | memory-ingest &

# 5. Return success to the agent
echo '{"continue":true}'
```

### Event Types

Map your agent's lifecycle events to these standard memory event types:

| Memory Event Type | When to Fire | Required Fields |
|-------------------|-------------|-----------------|
| `SessionStart` | New conversation begins | session_id |
| `UserPromptSubmit` | User sends a message | session_id, message |
| `PostToolUse` | Tool execution completes | session_id, tool, result |
| `Stop` | Assistant finishes responding | session_id |
| `SubagentStart` | Sub-agent spawned | session_id, subagent_id |
| `SubagentStop` | Sub-agent completed | session_id, subagent_id |
| `SessionEnd` | Conversation ends | session_id |

Not all agents fire all event types. Map what is available:

| Agent | SessionStart | UserPrompt | PostToolUse | Stop | SubagentStart/Stop |
|-------|:---:|:---:|:---:|:---:|:---:|
| Claude Code | Yes | Yes | Yes | Yes | Yes |
| OpenCode | Yes | Yes | Yes | Yes | No |
| Gemini CLI | Yes | Yes | Yes | Yes | No |
| Copilot CLI | Yes | Yes | Yes | Yes | No |

### Session ID

Session IDs are critical for grouping events into conversations.

**Agents that provide session IDs**: Claude Code and OpenCode include a `session_id` in their hook events. Use it directly.

**Agents that require synthesis**: Gemini CLI and Copilot CLI do not provide explicit session IDs. Synthesize one:

```bash
# Session file approach (used by Gemini and Copilot adapters)
SESSION_FILE="${TMPDIR:-/tmp}/memory-<agent>-session"

# Check if existing session is recent (< 30 min)
if [ -f "$SESSION_FILE" ]; then
    LAST_MOD=$(stat -f %m "$SESSION_FILE" 2>/dev/null || stat -c %Y "$SESSION_FILE" 2>/dev/null)
    NOW=$(date +%s)
    ELAPSED=$((NOW - LAST_MOD))
    if [ "$ELAPSED" -lt 1800 ]; then
        SESSION_ID=$(cat "$SESSION_FILE")
    fi
fi

# Generate new session if needed
if [ -z "$SESSION_ID" ]; then
    SESSION_ID="<agent>-$(date +%s)-$$"
    echo "$SESSION_ID" > "$SESSION_FILE"
fi
```

### Fail-Open Pattern

Adapters MUST never block the agent's UI. If the memory daemon is down or an error occurs, the adapter must still return success.

**Required fail-open behaviors:**

1. **Background processing**: Pipe to `memory-ingest` in the background (`&`)
2. **Trap errors**: Use `trap` to catch failures silently
3. **Timeout guard**: Kill long-running operations
4. **Exit 0 always**: Return success on ALL inputs, including malformed JSON

```bash
#!/usr/bin/env bash
set -uo pipefail

# Fail-open: wrap everything in a function with error trapping
main() {
    trap 'exit 0' ERR EXIT

    # Set a timeout (5 seconds max)
    TIMEOUT_PID=""
    ( sleep 5 && kill -9 $$ 2>/dev/null ) &
    TIMEOUT_PID=$!

    # ... event processing logic ...

    # Pipe to memory-ingest (backgrounded)
    echo "$PAYLOAD" | "${MEMORY_INGEST_PATH:-memory-ingest}" &

    # Clean up timeout guard
    kill "$TIMEOUT_PID" 2>/dev/null
}

main "$@"

# Always exit success
exit 0
```

### Redaction

Sensitive data must be stripped before ingestion. The following patterns should be redacted:

**Sensitive keys**: `api_key`, `token`, `secret`, `password`, `credential`, `authorization`

**Redaction implementation using jq:**

```bash
# Test if jq supports walk() (requires jq 1.6+)
if echo '{}' | jq 'walk(.)' >/dev/null 2>&1; then
    # Modern jq: recursive walk
    REDACTED=$(echo "$JSON" | jq '
        walk(if type == "object" then
            with_entries(
                if (.key | test("api_key|token|secret|password|credential|authorization"; "i"))
                then .value = "[REDACTED]"
                else .
                end
            )
        else . end)
    ')
else
    # Fallback for jq < 1.6: del-based redaction (top level + one level deep)
    REDACTED=$(echo "$JSON" | jq '
        (if .tool_input? then
            .tool_input |= (if type == "object" then
                with_entries(
                    if (.key | test("api_key|token|secret|password|credential|authorization"; "i"))
                    then .value = "[REDACTED]"
                    else .
                    end
                )
            else . end)
        else . end) |
        with_entries(
            if (.key | test("api_key|token|secret|password|credential|authorization"; "i"))
            then .value = "[REDACTED]"
            else .
            end
        )
    ')
fi
```

### ANSI Stripping

Agent output often contains ANSI escape sequences. Strip these before JSON parsing:

```bash
# Preferred: perl (handles CSI, OSC, SS2/SS3)
strip_ansi() {
    perl -pe '
        s/\e\[[0-9;]*[A-Za-z]//g;   # CSI sequences
        s/\e\][^\a]*\a//g;           # OSC sequences
        s/\e\][^\e]*\e\\//g;         # OSC with ST terminator
        s/\e[NO].//g;                # SS2/SS3 sequences
    '
}

# Fallback: sed (basic CSI only)
strip_ansi_sed() {
    sed 's/\x1b\[[0-9;]*[a-zA-Z]//g'
}
```

Apply ANSI stripping to any field that may contain terminal output (e.g., `message`, `result`, `output`).

## Skills

Skills provide agents with instructions for using the memory system.

### Skill Format

Skills use a Markdown file with YAML frontmatter:

```markdown
---
name: memory-query
description: Query past conversations using agent-memory
---

# Memory Query

Instructions for querying the memory system...

## Commands

### Search
Run: `memory-daemon retrieval route "<query>"`

### Recent
Run: `memory-daemon query root` and navigate to recent nodes
```

### Skill Portability

The SKILL.md format is portable across Claude Code, OpenCode, and Copilot (same file format). Gemini embeds skill content differently (within TOML commands).

| Agent | Skill Location | Format |
|-------|---------------|--------|
| Claude Code | `.claude/skills/<name>/SKILL.md` | Markdown + YAML frontmatter |
| OpenCode | `.opencode/skill/<name>/SKILL.md` | Markdown + YAML frontmatter |
| Copilot CLI | `.github/skills/<name>/SKILL.md` | Markdown + YAML frontmatter |
| Gemini CLI | `.gemini/skills/<name>/SKILL.md` | Markdown + YAML frontmatter |

### Required Skills

Every adapter should include these core skills:

| Skill | Purpose |
|-------|---------|
| `memory-query` | Core retrieval with tier-aware routing |
| `retrieval-policy` | Tier detection and fallback chains |
| `topic-graph` | Topic exploration and relationship browsing |
| `bm25-search` | Keyword search via BM25 teleport |
| `vector-search` | Semantic search via vector embeddings |

These skills teach the agent how to navigate the memory hierarchy, use the retrieval router, and access different search tiers.

### Optional Skills

| Skill | Purpose |
|-------|---------|
| `memory-<agent>-install` | Automated installation skill |

The install skill automates adapter setup for a new project. It copies hooks, skills, and commands to the correct locations.

## Commands

Commands provide slash-command interfaces for agents.

### Command Format Differences

Each agent has its own command format:

| Agent | Format | Location | Substitution |
|-------|--------|----------|--------------|
| Claude Code | Markdown + YAML frontmatter | `commands/*.md` | Parameter names in YAML |
| OpenCode | Markdown + YAML frontmatter | `.opencode/command/*.md` | `$ARGUMENTS` |
| Gemini CLI | TOML with `[prompt]` | `.gemini/commands/*.toml` | `{{args}}` |
| Copilot CLI | Skills (embedded) | `.github/skills/*.md` | Parameters in body |

### Standard Commands

Every adapter should provide these commands:

| Command | Description |
|---------|-------------|
| `memory-search` | Search past conversations |
| `memory-recent` | Show recent activity |
| `memory-context` | Expand a specific memory for full context |

### CLOD Conversion

Instead of maintaining command definitions separately for each adapter, use the CLOD (Cross-Language Operation Definition) format to define commands once and generate all adapter variants:

```bash
# Write a CLOD definition
cat > memory-search.toml << 'EOF'
[command]
name = "memory-search"
description = "Search past conversations"
# ... parameters, process, output ...
EOF

# Generate all adapter command files
memory-daemon clod convert --input memory-search.toml --target all --out ./adapters
```

See the [CLOD Format Specification](clod-format.md) for the full format reference.

## Agent Tagging

Every event must include an `agent` field identifying the source agent.

### Setting the Agent Tag

In the hook script or plugin, set the agent field in the JSON payload:

```bash
# Shell hook (Gemini, Copilot)
PAYLOAD=$(echo "$EVENT" | jq --arg agent "<agent-name>" '. + {agent: $agent}')

# Or construct the payload with agent included
PAYLOAD=$(jq -n \
    --arg agent "<agent-name>" \
    --arg session_id "$SESSION_ID" \
    '{hook_event_name: "UserPromptSubmit", session_id: $session_id, agent: $agent}')
```

```typescript
// TypeScript plugin (OpenCode)
const payload = {
    hook_event_name: event.type,
    session_id: event.sessionId,
    agent: "opencode",
};
```

### Agent Name Convention

- Use a single lowercase word: `"claude"`, `"opencode"`, `"gemini"`, `"copilot"`
- Do not include version numbers or platform suffixes
- The name must be consistent across all events from this adapter
- The name is used for `--agent` filter matching in CLI commands

### How Tags Are Used

Agent tags enable:

1. **Per-agent filtering**: `memory-daemon retrieval route "query" --agent claude`
2. **Agent discovery**: `memory-daemon agents list` aggregates from `TocNode.contributing_agents`
3. **Activity tracking**: `memory-daemon agents activity --agent opencode`
4. **Cross-agent comparison**: Search the same topic across different agents

## Config Precedence

Agent Memory follows a 5-level configuration hierarchy (highest priority first):

| Level | Source | Example |
|-------|--------|---------|
| 1 | CLI flags | `--port 50052` |
| 2 | Environment variables | `MEMORY_PORT=50052` |
| 3 | Project config file | `.agent-memory/config.toml` in project root |
| 4 | User/global config | `~/.config/agent-memory/config.toml` |
| 5 | Built-in defaults | Port 50051, DB at `~/.memory-store` |

### Adapter-Specific Config

Adapters may have their own configuration files:

| Agent | Config Location | Format |
|-------|----------------|--------|
| Claude Code | `~/.claude/hooks.yaml` | YAML |
| OpenCode | `.opencode/` directory | Plugin system |
| Gemini CLI | `.gemini/settings.json` | JSON |
| Copilot CLI | `.github/hooks/memory-hooks.json` | JSON |

These adapter configs control which events are captured and how they are processed. They are separate from the memory daemon config.

### Config Override for Testing

During adapter development, use environment variables to override defaults:

```bash
# Use a test database
export MEMORY_DB_PATH=/tmp/test-memory-db

# Use dry-run mode (no daemon required)
export MEMORY_INGEST_DRY_RUN=1

# Override memory-ingest binary path
export MEMORY_INGEST_PATH=./target/debug/memory-ingest
```

## Testing Your Adapter

### Dry-Run Mode

Test event capture without a running daemon:

```bash
# Enable dry-run mode
export MEMORY_INGEST_DRY_RUN=1

# Send a test event
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-1","agent":"myagent","message":"hello"}' | memory-ingest

# Output: dry-run event logged (no daemon connection)
```

### Integration Testing

Test with a running daemon:

```bash
# 1. Start daemon with test database
memory-daemon start --db-path /tmp/test-db

# 2. Send test events through your adapter's hook script
echo '{"hook_event_name":"SessionStart","session_id":"test-1","agent":"myagent"}' | ./your-hook-script.sh
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-1","agent":"myagent","message":"test message"}' | ./your-hook-script.sh

# 3. Verify events were captured
memory-daemon agents list
# Should show "myagent" in the list

# 4. Verify event content
memory-daemon query root
# Navigate to recent nodes to see test events

# 5. Clean up
memory-daemon stop
rm -rf /tmp/test-db
```

### Verification Checklist

Before publishing your adapter, verify:

- [ ] Events are captured for all supported lifecycle events
- [ ] Session IDs are consistent within a conversation
- [ ] Agent tag is set correctly (lowercase, consistent)
- [ ] Fail-open: adapter returns success even when daemon is down
- [ ] Redaction: sensitive keys are stripped from event payloads
- [ ] ANSI stripping: terminal escape sequences are removed
- [ ] Skills work: agent can execute memory queries
- [ ] Commands work: slash commands produce correct results
- [ ] `memory-daemon agents list` shows your agent after event capture

## Publishing

### Directory Structure

Follow the established adapter directory structure:

```
plugins/memory-<agent>-adapter/
  .<agent>/                    # Agent-specific config directory
    hooks/                     # Hook scripts
    skills/                    # Skill definitions
    commands/                  # Command definitions (if applicable)
    settings.json              # Hook configuration (if applicable)
  README.md                    # Installation and usage documentation
  .gitignore                   # OS/editor ignores
  plugin.json                  # Plugin manifest (if applicable)
```

### README Requirements

Your adapter README should include:

1. **Overview**: What the adapter does
2. **Installation**: Three paths (automated, global, per-project)
3. **Event mapping**: Which agent events map to which memory events
4. **Supported features**: What works and what does not
5. **Troubleshooting**: Common issues and solutions
6. **Comparison table**: How this adapter compares to others (optional)

### Contributing

To contribute your adapter to the agent-memory repository:

1. Create a branch: `feature/memory-<agent>-adapter`
2. Follow the directory structure above
3. Include comprehensive tests
4. Ensure all CI checks pass (`cargo fmt`, `cargo clippy`, `cargo test`)
5. Submit a pull request with the adapter comparison table updated

## Reference Implementations

Study these existing adapters for patterns and best practices:

| Adapter | Strengths | Location |
|---------|-----------|----------|
| Claude Code | Simplest hook integration | `plugins/memory-query-plugin/` |
| OpenCode | TypeScript plugin example | `plugins/memory-opencode-plugin/` |
| Gemini CLI | Shell hook with settings.json | `plugins/memory-gemini-adapter/` |
| Copilot CLI | Hook + skill hybrid approach | `plugins/memory-copilot-adapter/` |

Each adapter README documents the specific patterns used and why.
