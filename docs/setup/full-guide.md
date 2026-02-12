# Agent Memory Full Guide (macOS + Linux)

This guide walks you through a complete single-agent installation of agent-memory.
It covers two installation paths (source build or prebuilt binaries), a full sample
configuration, an optional dry-run config check, and recommended verification steps.

If you only need the essentials, see the Quickstart.

## Scope

- Platforms: macOS and Linux only
- Mode: single-agent setup (multi-agent configuration is out of scope here)
- Agent hooks: handled in a separate agent-specific guide

## Prerequisites

Required:

- macOS or Linux
- `protoc` (Protocol Buffers compiler)

Optional (only for source builds):

- Rust 1.82+ with Cargo

Optional (for summarization):

- OpenAI or Anthropic API key

> Verify now (optional):
> - `protoc --version`
> - `rustc --version`

## Step 1: Choose an install path

You have two supported options:

1. Build from source (recommended if you already have Rust)
2. Use prebuilt binaries (fastest path)

### Option A: Build from source

```bash
git clone https://github.com/SpillwaveSolutions/agent-memory.git
cd agent-memory
cargo build --release
```

Copy binaries to a local bin directory:

```bash
mkdir -p ~/.local/bin
cp target/release/memory-daemon ~/.local/bin/
cp target/release/memory-ingest ~/.local/bin/
```

### Option B: Prebuilt binaries

```bash
mkdir -p ~/.local/bin
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
curl -L "https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-daemon-${PLATFORM}-${ARCH}.tar.gz" | tar xz -C ~/.local/bin
curl -L "https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-ingest-${PLATFORM}-${ARCH}.tar.gz" | tar xz -C ~/.local/bin
chmod +x ~/.local/bin/memory-daemon ~/.local/bin/memory-ingest
```

## Step 2: Ensure binaries are on PATH

```bash
export PATH="$HOME/.local/bin:$PATH"
```

> Verify now (optional):
> - `memory-daemon --version`
> - `memory-ingest --version`

## Step 3: Create a single-agent config

Create the config directory and set defaults inline.

```bash
mkdir -p ~/.config/agent-memory
```

### Default values (single-agent mode)

- `db_path`: `~/.local/share/agent-memory/db`
- `grpc_port`: `50051`
- `grpc_host`: `0.0.0.0`
- `log_level`: `info`
- `summarizer.provider`: `openai`
- `summarizer.model`: `gpt-4o-mini`

### Full sample config

```toml
# ~/.config/agent-memory/config.toml

# Core settings
db_path = "~/.local/share/agent-memory/db"
grpc_port = 50051
grpc_host = "0.0.0.0"
log_level = "info"

# Single-agent mode is the default. Multi-agent options are not covered here.
multi_agent_mode = "separate"

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

If you prefer Anthropic, set:

```toml
[summarizer]
provider = "anthropic"
model = "claude-3-5-haiku-latest"
```

> Verify now (optional):
> - `cat ~/.config/agent-memory/config.toml`

## Step 4: Dry-run / config check (recommended)

Before starting the daemon, run a config check to validate the file.

```bash
memory-daemon config check
```

If the check fails, fix the reported issues before continuing.

## Step 5: Start the daemon

```bash
memory-daemon start
```

> Verify now (optional):
> - `memory-daemon status`
> - `memory-daemon query --endpoint http://[::1]:50051 root`

## Step 6: Connect your agent (separate guide)

Agent integration is intentionally separate so the core install flow stays clean.
Pick the guide that matches your tool:

See: [Agent Setup](agent-setup.md)

## Advanced options (minimal)

If you need advanced configuration (custom paths, ports, or tuning), see:

- [Configuration Reference](../references/configuration-reference.md)

Keep advanced changes minimal unless you have a specific reason to deviate from
the defaults.

## Troubleshooting

Common issues and fixes:

### "command not found: memory-daemon"

Ensure the install directory is on PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### "connection refused"

The daemon is not running or the port is wrong:

```bash
memory-daemon status
```

If the port is busy, choose a new one in `config.toml` and restart the daemon.

### "summarization not working"

Set an API key for the provider you chose:

```bash
export OPENAI_API_KEY="sk-..."
# or
export ANTHROPIC_API_KEY="sk-ant-..."
```

### "no events captured"

Agent hooks are not configured yet. Follow the agent setup guide and verify the
hooks file points to `memory-ingest`.

## Next steps

- Keep the daemon running while you work
- Use `/memory-status` to check health
- Explore `/memory-search` and `/memory-recent` once events are captured
