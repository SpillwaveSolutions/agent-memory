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

Check health and status of agent-memory installation and daemon.

## Usage

```
/memory-status
/memory-status --verbose
/memory-status --json
```

## Flags

| Flag | Description |
|------|-------------|
| `--verbose` | Show detailed diagnostics including config, storage stats |
| `--json` | Output in JSON format for programmatic use |

## Process

### Quick Status (default)

```bash
# Check if daemon binary exists
which memory-daemon && echo "INSTALLED" || echo "NOT_INSTALLED"

# Check daemon status
memory-daemon status

# Check if accepting connections
memory-daemon query --endpoint http://[::1]:50051 root 2>/dev/null && echo "HEALTHY" || echo "UNHEALTHY"
```

### Verbose Status (--verbose)

```bash
# Version information
memory-daemon --version
memory-ingest --version 2>/dev/null || echo "NOT_INSTALLED"

# Daemon status
memory-daemon status

# Storage statistics
memory-daemon admin --db-path ~/.memory-store stats

# Configuration
cat ~/.config/memory-daemon/config.toml

# CCH hooks check
cat ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null | grep -A2 memory-ingest

# Recent logs (last 10 lines)
tail -10 ~/Library/Logs/memory-daemon/daemon.log 2>/dev/null || \
tail -10 ~/.local/state/memory-daemon/daemon.log 2>/dev/null
```

## Output Format

### Default Output

```markdown
## Memory Status

| Component | Status |
|-----------|--------|
| Daemon | Running |
| Storage | Healthy |
| CCH Hooks | Configured |

**Endpoint:** `http://[::1]:50051`
**Data Path:** `~/.memory-store`
```

### Verbose Output

```markdown
## Memory Status (Verbose)

### Components

| Component | Status | Version |
|-----------|--------|---------|
| memory-daemon | Running | 1.0.0 |
| memory-ingest | Installed | 1.0.0 |

### Daemon

| Property | Value |
|----------|-------|
| Status | Running |
| PID | 12345 |
| Endpoint | http://[::1]:50051 |
| Uptime | 2h 34m |

### Storage

| Metric | Value |
|--------|-------|
| Events | 1,234 |
| TOC Nodes | 56 |
| Grips | 789 |
| Disk Usage | 45 MB |

### Configuration

```toml
[storage]
path = "~/.memory-store"

[server]
host = "[::1]"
port = 50051

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

### CCH Integration

```yaml
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
```

### Recent Logs

```
2026-01-31T12:00:00Z INFO  memory_daemon > Starting server on [::1]:50051
2026-01-31T12:00:01Z INFO  memory_daemon > Server started successfully
```
```

### JSON Output (--json)

```json
{
  "status": "healthy",
  "daemon": {
    "installed": true,
    "running": true,
    "version": "1.0.0",
    "pid": 12345,
    "endpoint": "http://[::1]:50051"
  },
  "storage": {
    "path": "~/.memory-store",
    "events": 1234,
    "toc_nodes": 56,
    "grips": 789,
    "disk_usage_bytes": 47185920
  },
  "cch": {
    "installed": true,
    "hook_configured": true
  },
  "config": {
    "summarizer_provider": "openai",
    "summarizer_model": "gpt-4o-mini"
  }
}
```

## Health Indicators

| Status | Meaning |
|--------|---------|
| Healthy | All components working |
| Degraded | Daemon running but some issues |
| Unhealthy | Daemon not running or not responding |
| Not Installed | memory-daemon binary not found |

## Troubleshooting Hints

Based on status, provide actionable hints:

| Issue | Hint |
|-------|------|
| Daemon not running | Run `memory-daemon start` |
| CCH not configured | Run `/memory-setup` to configure |
| No events | Check CCH hooks, events may not be ingested |
| Connection refused | Daemon may have crashed, check logs |

## Examples

**Quick check:**
```
/memory-status
```

**Full diagnostics:**
```
/memory-status --verbose
```

**For scripting:**
```
/memory-status --json
```
