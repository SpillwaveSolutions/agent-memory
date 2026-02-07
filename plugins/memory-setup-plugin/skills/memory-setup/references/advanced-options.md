# Advanced Options

Configuration options available in `/memory-setup --advanced` mode that cover gap settings not in the basic wizard.

## Overview

These options are for advanced users who need fine-grained control over agent-memory behavior. Most users can skip these and use defaults.

## Server Options

### Request Timeout (`timeout_secs`)

Maximum time for gRPC requests before timeout.

```toml
[server]
host = "[::1]"
port = 50051
timeout_secs = 30  # Default: 30 seconds
```

| Value | Use Case |
|-------|----------|
| 10 | Fast networks, quick failure |
| 30 | Default, balanced |
| 60 | Slow networks, large queries |
| 120 | Very slow connections |

**When to change:**
- Increase if seeing timeout errors on large queries
- Decrease for faster failure detection on unreliable networks

## TOC Segmentation Options

Control how conversations are segmented for Table of Contents generation.

### Token Overlap (`overlap_tokens`)

Number of tokens from previous segment included for context continuity.

```toml
[toc]
segment_min_tokens = 500
segment_max_tokens = 4000
time_gap_minutes = 30
overlap_tokens = 500  # Default: 500
```

| Value | Effect | Use Case |
|-------|--------|----------|
| 0 | No overlap | Distinct topics |
| 250 | Minimal | Quick transitions |
| 500 | Default | Good context (default) |
| 1000 | High | Continuous discussions |

**When to change:**
- Increase if context seems disconnected between segments
- Decrease if segments are too repetitive

### Time Overlap (`overlap_minutes`)

Minutes from previous segment included for temporal continuity.

```toml
[toc]
overlap_minutes = 5  # Default: 5
```

| Value | Effect | Use Case |
|-------|--------|----------|
| 0 | No overlap | Clean time boundaries |
| 5 | Default | Brief overlap (default) |
| 15 | Extended | Long-running discussions |
| 30 | High | All-day sessions |

**When to change:**
- Increase for conversations that span long periods
- Decrease for rapid topic switching

## Logging Options

Control daemon logging behavior.

### Log Level (`level`)

Minimum severity level for log output.

```toml
[logging]
level = "info"  # Default: info
```

| Level | Shows | Use Case |
|-------|-------|----------|
| `trace` | Everything | Deep debugging |
| `debug` | Debug + above | Development |
| `info` | Info + above | Normal operation (default) |
| `warn` | Warnings + errors | Production |
| `error` | Errors only | Minimal logging |

**Example:**
```toml
[logging]
level = "debug"  # Troubleshooting
```

### Log Format (`format`)

Output format for log messages.

```toml
[logging]
format = "pretty"  # Default: pretty
```

| Format | Output | Use Case |
|--------|--------|----------|
| `pretty` | Human-readable with colors | Interactive use |
| `json` | JSON structured logs | Log aggregation |
| `compact` | Minimal single-line | High volume |

**Examples:**

Pretty format:
```
2024-01-15T10:30:00Z INFO memory_daemon::server Started on [::1]:50051
```

JSON format:
```json
{"timestamp":"2024-01-15T10:30:00Z","level":"INFO","target":"memory_daemon::server","message":"Started on [::1]:50051"}
```

Compact format:
```
I 10:30:00 Started on [::1]:50051
```

### Log File (`file`)

Path for log file output. Empty means stderr only.

```toml
[logging]
file = ""  # Default: empty (stderr only)
```

**Examples:**

```toml
[logging]
# Log to file
file = "~/.local/state/memory-daemon/daemon.log"

# Multiple outputs (file + stderr)
file = "~/Library/Logs/memory-daemon/daemon.log"
also_stderr = true
```

| Configuration | Effect |
|---------------|--------|
| `file = ""` | Logs to stderr only |
| `file = "/path/to/log"` | Logs to file only |
| `file = "/path"` + `also_stderr = true` | Both file and stderr |

## Configuration Example

Full advanced configuration:

```toml
[server]
host = "[::1]"
port = 50051
timeout_secs = 60  # Extended timeout

[toc]
segment_min_tokens = 500
segment_max_tokens = 4000
time_gap_minutes = 30
overlap_tokens = 750    # Increased overlap
overlap_minutes = 10    # Extended time overlap

[logging]
level = "debug"         # Verbose for troubleshooting
format = "json"         # Structured for log aggregation
file = "/var/log/memory-daemon/daemon.log"
also_stderr = true
```

## Accessing Advanced Options

### Via Wizard

```bash
/memory-setup --advanced
```

This shows additional questions for these options.

### Via Manual Edit

```bash
# Open config file
$EDITOR ~/.config/memory-daemon/config.toml

# Add or modify sections
```

### Via Environment Variables

```bash
export MEMORY_LOG_LEVEL=debug
export MEMORY_LOG_FORMAT=json
memory-daemon start
```

## Defaults Summary

| Option | Default | Section |
|--------|---------|---------|
| `timeout_secs` | 30 | `[server]` |
| `overlap_tokens` | 500 | `[toc]` |
| `overlap_minutes` | 5 | `[toc]` |
| `level` | "info" | `[logging]` |
| `format` | "pretty" | `[logging]` |
| `file` | "" | `[logging]` |

## Troubleshooting with Logging

### Enable Debug Logging

```bash
# Temporarily
MEMORY_LOG_LEVEL=debug memory-daemon start

# Permanently
[logging]
level = "debug"
```

### View Logs

```bash
# macOS
tail -f ~/Library/Logs/memory-daemon/daemon.log

# Linux
tail -f ~/.local/state/memory-daemon/daemon.log

# Or stderr
memory-daemon start 2>&1 | tee daemon.log
```

### Common Log Patterns

**Startup issues:**
```bash
grep -E "ERROR|WARN|failed" daemon.log
```

**Connection problems:**
```bash
grep -E "connect|timeout|refused" daemon.log
```

**Performance issues:**
```bash
grep -E "slow|latency|duration" daemon.log
```
