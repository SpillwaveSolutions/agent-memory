# Configuration Options

Complete reference for agent-memory configuration.

## Configuration File Location

| Platform | Path |
|----------|------|
| macOS/Linux | `~/.config/memory-daemon/config.toml` |
| Windows | `%APPDATA%\memory-daemon\config.toml` |

## Full Configuration Reference

```toml
# ~/.config/memory-daemon/config.toml

#
# Storage Configuration
#
[storage]
# Path to RocksDB data directory
# Supports ~ for home directory expansion
path = "~/.memory-store"

# Maximum size of write buffer (MB)
# Higher values = more memory, better write performance
write_buffer_size_mb = 64

# Number of background compaction threads
max_background_jobs = 4


#
# Server Configuration
#
[server]
# gRPC server bind address
# Use [::1] for IPv6 localhost, 127.0.0.1 for IPv4
host = "[::1]"

# gRPC server port
port = 50051

# Request timeout (seconds)
timeout_secs = 30


#
# Summarizer Configuration
#
[summarizer]
# LLM provider: "openai" or "anthropic"
provider = "openai"

# Model to use for summarization
# OpenAI: gpt-4o-mini, gpt-4o, gpt-4-turbo
# Anthropic: claude-3-haiku-20240307, claude-3-sonnet-20240229
model = "gpt-4o-mini"

# API key (prefer environment variable)
# api_key = "sk-..."

# API endpoint (optional, for proxies or custom endpoints)
# api_endpoint = "https://api.openai.com/v1"

# Maximum tokens per summarization request
max_tokens = 1024

# Temperature for summarization (0.0 - 1.0)
# Lower = more deterministic, higher = more creative
temperature = 0.3


#
# TOC (Table of Contents) Configuration
#
[toc]
# Minimum tokens before creating a segment boundary
segment_min_tokens = 500

# Maximum tokens per segment
segment_max_tokens = 4000

# Time gap (minutes) that triggers a segment boundary
time_gap_minutes = 30

# Overlap tokens for context continuity
overlap_tokens = 500

# Overlap time (minutes) for context continuity
overlap_minutes = 5


#
# Rollup Configuration
#
[rollup]
# Minimum age (hours) before rolling up segments
# Prevents rolling up active/recent segments
min_age_hours = 24

# Schedule for automatic rollups (cron format)
# Default: daily at 3 AM
# schedule = "0 3 * * *"


#
# Logging Configuration
#
[logging]
# Log level: error, warn, info, debug, trace
level = "info"

# Log format: "json" or "pretty"
format = "pretty"

# Log file path (optional, logs to stderr if not set)
# file = "~/.local/state/memory-daemon/memory-daemon.log"
```

## Environment Variables

Environment variables override config file values:

| Variable | Description | Example |
|----------|-------------|---------|
| `MEMORY_STORAGE_PATH` | Data directory path | `~/.memory-store` |
| `MEMORY_SERVER_HOST` | Server bind address | `[::1]` |
| `MEMORY_SERVER_PORT` | Server port | `50051` |
| `OPENAI_API_KEY` | OpenAI API key | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API key | `sk-ant-...` |
| `MEMORY_LOG_LEVEL` | Logging level | `debug` |

**Example usage:**

```bash
MEMORY_SERVER_PORT=50052 memory-daemon start
```

## CCH Hooks Configuration

If using Claude Code Hooks, configure in `~/.claude/code_agent_context_hooks/hooks.yaml`:

```yaml
# Hooks configuration for agent-memory integration
version: "1"

hooks:
  # Capture all events
  - event: all
    handler:
      type: pipe
      command: memory-ingest
      # Optional: specify endpoint if not default
      # args: ["--endpoint", "http://[::1]:50052"]

  # Alternative: capture specific events only
  # - event: session_start
  #   handler:
  #     type: pipe
  #     command: memory-ingest
  #
  # - event: user_prompt_submit
  #   handler:
  #     type: pipe
  #     command: memory-ingest
```

## Configuration Precedence

1. Environment variables (highest)
2. Config file values
3. Built-in defaults (lowest)

## Common Configuration Scenarios

### Minimal Setup (Development)

```toml
[storage]
path = "~/.memory-store"

[server]
host = "[::1]"
port = 50051
```

### Production Setup

```toml
[storage]
path = "/var/lib/memory-daemon/data"
write_buffer_size_mb = 128
max_background_jobs = 8

[server]
host = "0.0.0.0"
port = 50051
timeout_secs = 60

[summarizer]
provider = "openai"
model = "gpt-4o"
temperature = 0.2

[toc]
segment_min_tokens = 1000
segment_max_tokens = 8000

[logging]
level = "info"
format = "json"
file = "/var/log/memory-daemon/daemon.log"
```

### Low-Memory Setup

```toml
[storage]
path = "~/.memory-store"
write_buffer_size_mb = 16
max_background_jobs = 1

[summarizer]
model = "gpt-4o-mini"
max_tokens = 512

[toc]
segment_max_tokens = 2000
```

### Custom API Endpoint (Proxy)

```toml
[summarizer]
provider = "openai"
api_endpoint = "https://my-proxy.example.com/v1"
model = "gpt-4o-mini"
```

## Validating Configuration

```bash
# Check config file syntax
memory-daemon config validate

# Show effective configuration (with env overrides)
memory-daemon config show
```
