---
name: memory-status
description: Check health and status of agent-memory installation
parameters:
  - name: verbose
    description: Show detailed diagnostics
    required: false
    type: flag
  - name: json
    description: Output in JSON format
    required: false
    type: flag
skills:
  - memory-setup
---

# Memory Status

Check health and status of agent-memory installation and daemon with comprehensive diagnostics.

## Usage

```
/memory-status
/memory-status --verbose
/memory-status --json
```

## Flags

| Flag | Description |
|------|-------------|
| `--verbose` | Show detailed diagnostics including config, storage stats, logs |
| `--json` | Output in JSON format for programmatic use |

## Health Checks

The status command performs these health checks in sequence:

| Check | Description | Pass Condition |
|-------|-------------|----------------|
| Binary Installed | memory-daemon in PATH | `which memory-daemon` succeeds |
| Daemon Running | Process is active | `memory-daemon status` returns "running" |
| Port Listening | Server accepting connections | Port 50051 (or configured) in use by daemon |
| gRPC Connectivity | Can make RPC calls | `memory-daemon query root` succeeds |
| Database Accessible | Storage readable/writable | `memory-daemon admin stats` succeeds |
| Recent Events | Events being captured | Event count > 0 (warning if 0) |
| CCH Hook Configured | Hook handler in hooks.yaml | `grep memory-ingest hooks.yaml` finds match |

## Process

### Quick Status (default)

```bash
# Step 1: Check if daemon binary exists
DAEMON_PATH=$(which memory-daemon 2>/dev/null)
if [ -z "$DAEMON_PATH" ]; then
  echo "NOT_INSTALLED"
  exit 1
fi

# Step 2: Check daemon status
DAEMON_STATUS=$(memory-daemon status 2>/dev/null)
# Returns: running, stopped, or error

# Step 3: Check if accepting connections (only if running)
if [ "$DAEMON_STATUS" = "running" ]; then
  memory-daemon query --endpoint http://[::1]:50051 root 2>/dev/null
  if [ $? -eq 0 ]; then
    CONNECTIVITY="healthy"
  else
    CONNECTIVITY="unhealthy"
  fi
fi

# Step 4: Check CCH hook
HOOK_CONFIGURED="false"
if grep -q "memory-ingest" ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null; then
  HOOK_CONFIGURED="true"
fi

# Step 5: Get storage path from config
STORAGE_PATH=$(grep -A2 '\[storage\]' ~/.config/memory-daemon/config.toml 2>/dev/null | grep 'path' | cut -d'"' -f2)
STORAGE_PATH=${STORAGE_PATH:-~/.memory-store}
```

### Verbose Status (--verbose)

```bash
# Version information
echo "=== Versions ==="
memory-daemon --version 2>/dev/null || echo "memory-daemon: NOT_INSTALLED"
memory-ingest --version 2>/dev/null || echo "memory-ingest: NOT_INSTALLED"

# Daemon status details
echo "=== Daemon ==="
memory-daemon status 2>/dev/null

# PID file check
PID_FILE="$HOME/Library/Application Support/memory-daemon/daemon.pid"
[ -f "$PID_FILE" ] && echo "PID: $(cat "$PID_FILE")" || echo "PID: not found"

# Storage statistics
echo "=== Storage ==="
STORAGE_PATH=$(grep -A2 '\[storage\]' ~/.config/memory-daemon/config.toml 2>/dev/null | grep 'path' | cut -d'"' -f2)
STORAGE_PATH=${STORAGE_PATH:-~/.memory-store}
memory-daemon admin --db-path "$STORAGE_PATH" stats 2>/dev/null || echo "Storage: not accessible"

# Disk usage
du -sh "$STORAGE_PATH" 2>/dev/null || echo "Disk: unknown"

# Configuration dump
echo "=== Configuration ==="
cat ~/.config/memory-daemon/config.toml 2>/dev/null || echo "Config: not found"

# CCH hooks check
echo "=== CCH Integration ==="
cat ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null | grep -A5 memory-ingest || echo "CCH hook: not configured"

# Environment variables
echo "=== Environment ==="
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI_API_KEY: set" || echo "OPENAI_API_KEY: not set"
[ -n "$ANTHROPIC_API_KEY" ] && echo "ANTHROPIC_API_KEY: set" || echo "ANTHROPIC_API_KEY: not set"

# Recent logs (last 20 lines)
echo "=== Recent Logs ==="
LOG_FILE="$HOME/Library/Logs/memory-daemon/daemon.log"
[ ! -f "$LOG_FILE" ] && LOG_FILE="$HOME/.local/state/memory-daemon/daemon.log"
tail -20 "$LOG_FILE" 2>/dev/null || echo "Logs: not found"
```

## Output Format

### Default Output

```markdown
## Memory Status

| Component | Status | Details |
|-----------|--------|---------|
| Daemon | Running | PID 12345 on [::1]:50051 |
| Storage | Healthy | ~/.memory-store (45 MB) |
| CCH Hooks | Configured | Global hook active |
| API Keys | Available | OpenAI key set |

**Overall Status:** Healthy

**Quick Stats:**
- Events: 1,234
- TOC Nodes: 56
- Grips: 789
```

### Verbose Output (--verbose)

```markdown
## Memory Status (Verbose)

### Components

| Component | Status | Version | Path |
|-----------|--------|---------|------|
| memory-daemon | Installed | 1.0.0 | ~/.cargo/bin/memory-daemon |
| memory-ingest | Installed | 1.0.0 | ~/.cargo/bin/memory-ingest |

### Daemon

| Property | Value |
|----------|-------|
| Status | Running |
| PID | 12345 |
| Endpoint | http://[::1]:50051 |
| Started | 2026-01-31T10:00:00Z |
| Uptime | 2h 34m |

### Storage

| Metric | Value |
|--------|-------|
| Path | ~/.memory-store |
| Events | 1,234 |
| TOC Nodes | 56 |
| Grips | 789 |
| Disk Usage | 45 MB |
| Last Write | 2026-01-31T12:30:00Z |

### Configuration

```toml
[storage]
path = "~/.memory-store"
write_buffer_size_mb = 64
max_background_jobs = 4

[server]
host = "[::1]"
port = 50051
timeout_secs = 30

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
max_tokens = 1024
temperature = 0.3

[toc]
segment_min_tokens = 500
segment_max_tokens = 4000
time_gap_minutes = 30

[logging]
level = "info"
format = "pretty"
```

### CCH Integration

```yaml
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
```

**Hook Status:** Active (global)

### Environment Variables

| Variable | Status |
|----------|--------|
| OPENAI_API_KEY | Set (sk-...XXXX) |
| ANTHROPIC_API_KEY | Not set |
| MEMORY_LOG_LEVEL | Not set (using config) |

### Recent Logs

```
2026-01-31T12:00:00Z INFO  memory_daemon > Starting server on [::1]:50051
2026-01-31T12:00:01Z INFO  memory_daemon > Server started successfully
2026-01-31T12:00:05Z INFO  memory_daemon::service > Ingested event: session_start
2026-01-31T12:00:10Z INFO  memory_daemon::service > Ingested event: user_message
2026-01-31T12:15:00Z INFO  memory_daemon::toc > Built TOC segment with 12 events
```
```

### JSON Output (--json)

```json
{
  "overall_status": "healthy",
  "checks": {
    "binary_installed": true,
    "daemon_running": true,
    "port_listening": true,
    "grpc_connectivity": true,
    "database_accessible": true,
    "recent_events": true,
    "cch_hook_configured": true
  },
  "daemon": {
    "installed": true,
    "running": true,
    "version": "1.0.0",
    "pid": 12345,
    "endpoint": "http://[::1]:50051",
    "started_at": "2026-01-31T10:00:00Z",
    "uptime_seconds": 9240
  },
  "ingest": {
    "installed": true,
    "version": "1.0.0",
    "path": "~/.cargo/bin/memory-ingest"
  },
  "storage": {
    "path": "~/.memory-store",
    "accessible": true,
    "events": 1234,
    "toc_nodes": 56,
    "grips": 789,
    "disk_usage_bytes": 47185920,
    "last_write": "2026-01-31T12:30:00Z"
  },
  "cch": {
    "hooks_file_exists": true,
    "hook_configured": true,
    "hook_scope": "global"
  },
  "config": {
    "path": "~/.config/memory-daemon/config.toml",
    "exists": true,
    "storage_path": "~/.memory-store",
    "server_host": "[::1]",
    "server_port": 50051,
    "summarizer_provider": "openai",
    "summarizer_model": "gpt-4o-mini"
  },
  "environment": {
    "openai_api_key": true,
    "anthropic_api_key": false,
    "memory_log_level": null
  },
  "timestamp": "2026-01-31T12:34:56Z"
}
```

## Health Status Levels

| Status | Meaning | Condition |
|--------|---------|-----------|
| Healthy | All components working | All checks pass |
| Degraded | Partially functional | Daemon running but some issues (no events, no API key) |
| Unhealthy | Not operational | Daemon not running or not responding |
| Not Installed | System not set up | memory-daemon binary not found |

### Status Determination Logic

```
Binary installed?
├── NO → Status: NOT_INSTALLED
│
└── YES → Daemon running?
    ├── NO → Status: UNHEALTHY (can be started)
    │
    └── YES → gRPC connectivity OK?
        ├── NO → Status: UNHEALTHY (process hung)
        │
        └── YES → All optional checks pass?
            ├── NO → Status: DEGRADED (functional but issues)
            │   └── Issues: no events, no API key, CCH not configured
            │
            └── YES → Status: HEALTHY
```

## Troubleshooting Hints

Based on status, provide actionable hints:

| Issue | Detected By | Hint |
|-------|-------------|------|
| Daemon not running | status check | Run `memory-daemon start` |
| CCH not configured | hook check | Run `/memory-setup` to configure hooks |
| No events | event count = 0 | Check CCH hooks, verify memory-ingest in PATH |
| Connection refused | gRPC check fails | Daemon may have crashed, check logs |
| No API key | env check | Set `OPENAI_API_KEY` or `ANTHROPIC_API_KEY` |
| Port in use | port check | Another process on 50051, use different port |
| Storage not accessible | admin stats fails | Check permissions on data directory |
| Stale PID | process check | Delete PID file and restart daemon |

### Automatic Hint Generation

When status is not HEALTHY, include hints section:

```markdown
### Recommended Actions

Based on the checks above, consider:

1. **[Priority: High]** Start the daemon: `memory-daemon start`
2. **[Priority: Medium]** Configure CCH hook: Run `/memory-setup`
3. **[Priority: Low]** Set API key for summarization: `export OPENAI_API_KEY=...`
```

## Examples

### Quick check

```
/memory-status
```

Output:
```markdown
## Memory Status

| Component | Status | Details |
|-----------|--------|---------|
| Daemon | Running | PID 12345 |
| Storage | Healthy | 1,234 events |
| CCH Hooks | Configured | Global |

**Overall Status:** Healthy
```

### Full diagnostics

```
/memory-status --verbose
```

### For scripting/automation

```
/memory-status --json
```

### Quick health check in scripts

```bash
# Check if memory system is healthy
STATUS=$(memory-daemon status 2>/dev/null)
if [ "$STATUS" = "running" ]; then
  memory-daemon query root >/dev/null 2>&1 && echo "HEALTHY" || echo "DEGRADED"
else
  echo "UNHEALTHY"
fi
```

## Integration with Troubleshooter

When status shows issues, the agent may suggest:

```markdown
**Issues Detected**

Your memory system has some issues. Would you like me to:

1. **Auto-fix** - I'll attempt to fix safe issues automatically
2. **Diagnose** - I'll run deeper diagnostics
3. **Manual** - Show me the manual fix steps

Reply with your choice (1/2/3) or say "fix it" to auto-fix.
```

This triggers the `setup-troubleshooter` agent for autonomous diagnosis and repair.
