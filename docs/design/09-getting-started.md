# Getting Started and Integration Guide

This guide walks you through setting up Agent Memory, integrating it with Claude Code, and using it to recall past conversations.

## What is Agent Memory?

### The Problem

AI coding agents like Claude Code have a context window - they can only remember the current conversation. When you ask "what did we discuss about authentication last week?", the agent has no way to answer without scanning through potentially thousands of past conversation logs.

**Current approaches are broken:**
- **Brute-force scanning** - Loading entire conversation histories consumes tokens and is slow
- **Vector search alone** - Returns fragments without context or verification
- **Manual search** - Users have to dig through logs themselves

### The Solution

Agent Memory provides **agentic search through progressive disclosure**. Instead of loading everything, the agent navigates a time-based hierarchy:

```
Year (2026: "focus on authentication")
  -> Month (January: "JWT implementation")
    -> Week (Week 3: "token refresh logic")
      -> Day (Thursday: "fixed expiration bug")
        -> Segment (actual conversation excerpt)
```

At each level, the agent reads a summary and decides whether to drill down. This mirrors how humans search email - you filter by date, scan subjects, then open the relevant thread.

### Key Benefits

| Benefit | Description |
|---------|-------------|
| **Low token overhead** | Navigate with ~500 tokens vs. 50,000+ for full scan |
| **Verifiable answers** | Every summary links to source conversations via "grips" |
| **Automatic capture** | Hooks record conversations passively - zero manual effort |
| **Fast queries** | RocksDB provides millisecond lookups |
| **Private and local** | All data stays on your machine |

### Use Cases

- "What authentication approach did we decide on last week?"
- "Show me our discussion about database migration"
- "What was the solution to that JWT bug?"
- "Summarize what we worked on yesterday"
- "Find the conversation where we discussed Rust error handling"

---

## Quick Start

Get up and running in 5 minutes.

### Prerequisites

- **Rust 1.82+** with Cargo
- **Claude Code** (or another compatible AI coding agent)
- **protoc** (Protocol Buffers compiler)

```bash
# Check Rust version
rustc --version  # Should be 1.82 or higher

# Check protoc
protoc --version
```

### Step 1: Build and Install

```bash
# Clone the repository
git clone https://github.com/SpillwaveSolutions/agent-memory.git
cd agent-memory

# Build release binaries
cargo build --release

# Install to local bin directory
mkdir -p ~/.local/bin
cp target/release/memory-daemon ~/.local/bin/
cp target/release/memory-ingest ~/.local/bin/

# Add to PATH (add to your shell profile for persistence)
export PATH="$HOME/.local/bin:$PATH"

# Verify installation
memory-daemon --version
```

### Step 2: Start the Daemon

```bash
# Start the memory daemon
memory-daemon start

# Verify it's running
memory-daemon status
```

You should see:
```
Memory daemon is running (PID: 12345)
```

### Step 3: Configure Claude Code Hooks

Copy the hooks configuration to capture conversations automatically:

```bash
# Create Claude Code hooks directory if it doesn't exist
mkdir -p ~/.claude

# Copy the example hooks configuration
cp examples/hooks.yaml ~/.claude/hooks.yaml
```

The hooks.yaml captures these events:
- `SessionStart` / `SessionEnd` - Conversation lifecycle
- `UserPromptSubmit` - Your messages to Claude
- `PostToolUse` - Tool executions (file edits, commands)
- `SubagentStart` / `SubagentStop` - Multi-agent workflows

### Step 4: First Ingestion Test

Test that events are being captured:

```bash
# Send a test event
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-123","message":"Hello world"}' | memory-ingest

# Expected output:
# {"continue":true}

# Check that the daemon received it
memory-daemon query --endpoint http://[::1]:50051 root
```

### Step 5: First Query

After some conversations have been captured:

```bash
# Get TOC root (year-level summaries)
memory-daemon query --endpoint http://[::1]:50051 root

# Navigate to a specific node
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"

# Browse children of a node
memory-daemon query --endpoint http://[::1]:50051 browse --parent-id "toc:month:2026-01" --limit 10
```

---

## Architecture Overview

Agent Memory uses a layered architecture optimized for agentic navigation.

### Components

```
+-------------------------------------------------------+
|                    AI Agent (Claude Code)             |
+-------------------------------------------------------+
                          |
                          | gRPC (port 50051)
                          v
+-------------------------------------------------------+
|                    Memory Daemon                      |
|  +-------------+  +-------------+  +---------------+  |
|  |  Ingestion  |  |    Query    |  |  TOC Builder  |  |
|  |   Service   |  |   Service   |  |  (Background) |  |
|  +-------------+  +-------------+  +---------------+  |
|                          |                            |
|  +---------------------------------------------------+|
|  |              Storage Layer (RocksDB)              ||
|  |  Events | TOC Nodes | Grips | Outbox | Checkpts  ||
|  +---------------------------------------------------+|
+-------------------------------------------------------+
                          ^
                          | JSON events
+-------------------------------------------------------+
|                    memory-ingest                      |
|         (CCH hook handler - reads from stdin)         |
+-------------------------------------------------------+
                          ^
                          | Hook events
+-------------------------------------------------------+
|               Claude Code / AI Agent                  |
+-------------------------------------------------------+
```

### Data Flow

1. **Ingestion**: Claude Code triggers hooks -> `memory-ingest` reads event -> sends to daemon via gRPC -> stored in RocksDB

2. **TOC Building**: Background job reads new events -> creates/updates time-based segments -> generates summaries with LLM -> stores TOC nodes and grips

3. **Query**: Agent requests TOC navigation -> daemon returns summaries and grips -> agent drills down to find answers

### Key Concepts

| Concept | Description |
|---------|-------------|
| **TOC (Table of Contents)** | Time-based hierarchy: Year -> Month -> Week -> Day -> Segment |
| **Node** | A TOC entry with title, summary bullets, keywords, and child links |
| **Grip** | Links a summary bullet to the source conversation events |
| **Event** | An immutable record of a conversation turn |
| **Segment** | A chunk of related events within a time window |

---

## Claude Code Integration

### How CCH Integration Works

The `memory-ingest` binary is a Claude Code Hooks (CCH) handler that:

1. Receives JSON events from Claude Code via stdin
2. Converts them to memory events
3. Sends them to the daemon via gRPC
4. Returns `{"continue":true}` to allow Claude to proceed

**Fail-open behavior**: If the daemon is down, `memory-ingest` still returns success. Conversations continue uninterrupted - you just won't capture those events.

### hooks.yaml Configuration

The example `hooks.yaml` (copy to `~/.claude/hooks.yaml`):

```yaml
version: "1.0"

settings:
  # Always allow Claude to continue, even if memory system fails
  fail_open: true
  # Timeout for hook script (seconds)
  script_timeout: 5

rules:
  # Capture conversation events to agent-memory
  - name: capture-to-memory
    description: Send conversation events to agent-memory daemon for TOC-based recall

    matchers:
      operations:
        # Session lifecycle
        - SessionStart
        - SessionEnd
        # User interactions
        - UserPromptSubmit
        # Tool activity
        - PostToolUse
        # Subagent spawning
        - SubagentStart
        - SubagentStop

    actions:
      run: "~/.local/bin/memory-ingest"
```

### Customizing Capture

To also capture assistant responses (more verbose but complete):

```yaml
rules:
  - name: capture-to-memory
    matchers:
      operations:
        - SessionStart
        - SessionEnd
        - UserPromptSubmit
        - AssistantResponse    # Add this for full responses
        - PostToolUse
        - SubagentStart
        - SubagentStop
    actions:
      run: "~/.local/bin/memory-ingest"
```

### Testing the Integration

```bash
# 1. Ensure daemon is running
memory-daemon status

# 2. Start a Claude Code session
# (have a conversation)

# 3. Check for captured events
memory-daemon query --endpoint http://[::1]:50051 root

# 4. If no events, check manually
echo '{"hook_event_name":"SessionStart","session_id":"test"}' | memory-ingest
```

---

## Using the Query Skill

When the memory-query plugin is installed, you can use these slash commands in Claude Code.

### /memory-search - Search by Topic

Search past conversations for a specific topic:

```
/memory-search authentication
/memory-search "JWT tokens" --period "last week"
/memory-search database --period january
```

**What it does:**
1. Checks daemon status
2. Gets TOC root to find available periods
3. Navigates to relevant time period
4. Searches node summaries for matching keywords
5. Presents results with grip IDs for drill-down

**Output format:**
```markdown
## Memory Search: authentication

### Week of January 20-26, 2026
**Summary:** JWT token implementation, OAuth2 provider integration

**Excerpts:**
- "Implemented JWT token refresh logic" `grip:1706540400000:01HN4QXKN6`
- "Fixed OAuth2 callback URL handling" `grip:1706540500000:01HN4QXYZ`

---
Expand any excerpt: /memory-context grip:ID
```

### /memory-recent - Recent Summaries

Show recent conversation summaries:

```
/memory-recent
/memory-recent --days 3
/memory-recent --days 14 --limit 20
```

**What it does:**
1. Gets TOC root to find current year
2. Navigates to current period (month, week)
3. Collects day nodes within the specified range
4. Presents summaries with timestamps

**Output format:**
```markdown
## Recent Conversations (Last 7 Days)

### January 30, 2026
**Topics:** rust, grpc, rocksdb

**Segments:**
1. **10:00** - Memory daemon implementation
   - Implemented gRPC service `grip:...`
   - Added RocksDB storage layer `grip:...`

---
Total: 8 segments across 5 days
```

### /memory-context - Expand Excerpt

Get full conversation context around a specific excerpt:

```
/memory-context grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
/memory-context grip:... --before 5 --after 5
```

**What it does:**
1. Validates grip ID format
2. Expands the grip to retrieve surrounding events
3. Presents the conversation thread

**Output format:**
```markdown
## Conversation Context

**Grip:** `grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE`
**Timestamp:** January 29, 2026 10:00:00

### Before (3 events)
| Role | Message |
|------|---------|
| user | How do we handle authentication? |
| assistant | We have several options... |
| user | Let's go with JWT |

### Excerpt (Referenced)
> Implemented JWT token refresh logic

### After (3 events)
| Role | Message |
|------|---------|
| assistant | I've added the refresh endpoint... |
| user | What about the race condition? |
| assistant | Good catch, we need a mutex... |
```

---

## CLI Reference Quick Guide

### Essential Commands

| Command | Description |
|---------|-------------|
| `memory-daemon start` | Start the daemon |
| `memory-daemon stop` | Stop the daemon |
| `memory-daemon status` | Check if daemon is running |
| `memory-daemon query root` | Get year-level TOC nodes |
| `memory-daemon query node --node-id ID` | Get specific TOC node |
| `memory-daemon query browse --parent-id ID` | Browse children |
| `memory-daemon query expand --grip-id ID` | Expand a grip |
| `memory-daemon admin stats` | Show storage statistics |
| `memory-daemon admin compact` | Compact the database |

### Common Workflows

**Check what's in memory:**
```bash
memory-daemon query --endpoint http://[::1]:50051 root
```

**Navigate the TOC:**
```bash
# Start at year
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"

# Drill to month
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:month:2026-01"

# Browse weeks in that month
memory-daemon query --endpoint http://[::1]:50051 browse --parent-id "toc:month:2026-01" --limit 10
```

**Get raw events from a time range:**
```bash
# Last hour
NOW=$(date +%s)000
HOUR_AGO=$(( $(date +%s) - 3600 ))000
memory-daemon query --endpoint http://[::1]:50051 events --from $HOUR_AGO --to $NOW --limit 50
```

**Check storage health:**
```bash
memory-daemon admin --db-path ~/.memory-store stats
```

---

## Configuration

### Configuration Hierarchy

Settings are loaded with this precedence (highest to lowest):

1. **CLI flags** - `memory-daemon start --port 50052`
2. **Environment variables** - `MEMORY_PORT=50052`
3. **Config file** - `~/.config/agent-memory/config.toml`
4. **Built-in defaults**

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MEMORY_PORT` | gRPC server port | 50051 |
| `MEMORY_DB_PATH` | RocksDB directory | ~/.memory-store |
| `MEMORY_LOG_LEVEL` | Log level (trace, debug, info, warn, error) | info |
| `MEMORY_GRPC_HOST` | gRPC host binding | 0.0.0.0 |
| `MEMORY_ENDPOINT` | Endpoint for memory-ingest | http://[::1]:50051 |

### Config File

Create `~/.config/agent-memory/config.toml`:

```toml
# Storage location
db_path = "~/.memory-store"

# gRPC server settings
grpc_port = 50051
grpc_host = "0.0.0.0"

# Log level
log_level = "info"

# Multi-agent mode: "separate" (per-project) or "unified" (shared with tags)
multi_agent_mode = "separate"

# Summarizer configuration
[summarizer]
provider = "openai"            # "openai", "anthropic", or "local"
model = "gpt-4o-mini"          # Model name
# api_key loaded from OPENAI_API_KEY or ANTHROPIC_API_KEY env var
# api_base_url = "https://custom-endpoint.com/v1"  # Optional custom endpoint
```

### Summarizer Setup

The summarizer generates TOC node summaries from raw events.

**OpenAI (recommended for cost/speed):**
```bash
export OPENAI_API_KEY="sk-..."
```

Config:
```toml
[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

**Anthropic:**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Config:
```toml
[summarizer]
provider = "anthropic"
model = "claude-3-haiku-20240307"
```

**Local (Ollama):**
```toml
[summarizer]
provider = "local"
model = "llama3.2"
api_base_url = "http://localhost:11434/v1"
```

---

## Troubleshooting

### Common Issues

#### "command not found: memory-daemon"

**Cause:** Binary not in PATH

**Fix:**
```bash
# Add to PATH
export PATH="$HOME/.local/bin:$PATH"

# Or use full path
~/.local/bin/memory-daemon status
```

#### "connection refused"

**Cause:** Daemon not running

**Fix:**
```bash
# Check status
memory-daemon status

# Start if needed
memory-daemon start

# If status shows running but connection fails, check port
lsof -i :50051
```

#### "address already in use"

**Cause:** Port 50051 is taken

**Fix:**
```bash
# Find what's using the port
lsof -i :50051

# Kill it or use different port
memory-daemon start --port 50052

# Update memory-ingest endpoint
export MEMORY_ENDPOINT="http://[::1]:50052"
```

#### "no events captured"

**Cause:** CCH hooks not configured or daemon not running during conversation

**Fix:**
```bash
# Check hooks file exists
cat ~/.claude/hooks.yaml

# Verify memory-ingest is findable
which memory-ingest

# Test manually
echo '{"hook_event_name":"SessionStart","session_id":"test"}' | memory-ingest

# Check daemon
memory-daemon status
```

#### "summarization not working"

**Cause:** Missing or invalid API key

**Fix:**
```bash
# Check if key is set
echo ${OPENAI_API_KEY:+set}
echo ${ANTHROPIC_API_KEY:+set}

# Set the key
export OPENAI_API_KEY="sk-..."

# Restart daemon to pick up new key
memory-daemon stop && memory-daemon start
```

### Health Checks

**Quick diagnostic:**
```bash
# All-in-one health check
echo "=== Status ===" && memory-daemon status && \
echo "=== Port ===" && lsof -i :50051 && \
echo "=== Query ===" && memory-daemon query --endpoint http://[::1]:50051 root && \
echo "=== Hooks ===" && grep memory-ingest ~/.claude/hooks.yaml 2>/dev/null || echo "No hook"
```

**Storage health:**
```bash
memory-daemon admin --db-path ~/.memory-store stats
```

### Log Locations

| Platform | Log Path |
|----------|----------|
| macOS | `~/Library/Logs/memory-daemon/daemon.log` |
| Linux | `~/.local/state/memory-daemon/daemon.log` |

**Enable debug logging:**
```bash
MEMORY_LOG_LEVEL=debug memory-daemon start
```

### Quick Fixes

**Restart daemon:**
```bash
memory-daemon stop && sleep 2 && memory-daemon start
```

**Clear stale PID:**
```bash
# macOS
rm -f ~/Library/Application\ Support/memory-daemon/daemon.pid

# Linux
rm -f ~/.local/state/memory-daemon/daemon.pid

memory-daemon start
```

**Run compaction (if slow or large):**
```bash
memory-daemon admin --db-path ~/.memory-store compact
```

**Fix permissions:**
```bash
chmod 700 ~/.memory-store ~/.config/agent-memory
```

---

## Next Steps

Now that Agent Memory is running:

1. **Have some conversations** - Start using Claude Code normally. Events will be captured automatically.

2. **Wait for TOC building** - The daemon builds TOC nodes in the background. After a few minutes, you should see summaries.

3. **Try a query** - Ask Claude "what did we discuss recently?" or use `/memory-search`.

4. **Install the plugin** - For the best experience, install the memory-query plugin from the Claude Code marketplace.

5. **Explore the CLI** - Use `memory-daemon query` commands to understand the data structure.

For more details, see:
- [API Reference](../API.md) - gRPC service definitions
- [Architecture](../ARCHITECTURE.md) - Deep dive into internals
- [Usage Guide](../USAGE.md) - Advanced CLI usage
