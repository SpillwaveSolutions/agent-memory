# Agent Memory Quickstart (macOS + Linux)

Get agent-memory installed quickly using either a source build or a prebuilt binary.

Use this quickstart for the shortest path to a working single-agent setup. For a
deeper walkthrough, see the Full Guide.

## Choose Your Install Path

- Source build (recommended if you already have Rust)
- Prebuilt binaries (fastest if you do not want to compile)

## Prerequisites

Required:

- macOS or Linux
- `protoc` (Protocol Buffers compiler)

Optional (only for source builds):

- Rust 1.82+ with Cargo

Optional (for summarization):

- OpenAI or Anthropic API key

## Checklist

### 1) Confirm prerequisites

- [ ] macOS or Linux confirmed
- [ ] `protoc` installed (`protoc --version`)
- [ ] Rust installed if using source build (`rustc --version`)

> Verify now (optional):
> - `protoc --version`
> - `rustc --version`

### 2) Install agent-memory

Pick one path:

#### Option A: Source build

- [ ] Clone the repo
- [ ] Build release binaries

```bash
git clone https://github.com/SpillwaveSolutions/agent-memory.git
cd agent-memory
cargo build --release
```

#### Option B: Prebuilt binaries

- [ ] Download the latest release for your platform
- [ ] Unpack to a local bin directory

```bash
mkdir -p ~/.local/bin
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
curl -L "https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-daemon-${PLATFORM}-${ARCH}.tar.gz" | tar xz -C ~/.local/bin
curl -L "https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-ingest-${PLATFORM}-${ARCH}.tar.gz" | tar xz -C ~/.local/bin
chmod +x ~/.local/bin/memory-daemon ~/.local/bin/memory-ingest
```

### 3) Add binaries to PATH

- [ ] Ensure `~/.local/bin` is on your PATH

```bash
export PATH="$HOME/.local/bin:$PATH"
```

> Verify now (optional):
> - `memory-daemon --version`
> - `memory-ingest --version`

### 4) Create a minimal config (single-agent defaults)

- [ ] Create `~/.config/agent-memory/config.toml`

```toml
# ~/.config/agent-memory/config.toml
db_path = "~/.local/share/agent-memory/db"
grpc_port = 50051
grpc_host = "0.0.0.0"
log_level = "info"

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

> Verify now (optional):
> - `cat ~/.config/agent-memory/config.toml`

### 5) Dry-run the config (optional, recommended)

- [ ] Validate configuration before starting the daemon

```bash
memory-daemon config check
```

### 6) Start the daemon

- [ ] Start the memory daemon

```bash
memory-daemon start
```

> Verify now (optional):
> - `memory-daemon status`
> - `memory-daemon query --endpoint http://[::1]:50051 root`

### 7) Configure agent hooks (separate guide)

- [ ] Follow the agent-specific setup guide for your tool

See: [Agent Setup](agent-setup.md)

## Troubleshooting (Quick Fixes)

- `command not found`: ensure `~/.local/bin` is in PATH
- `connection refused`: daemon not running; run `memory-daemon start`
- `summarization failing`: set `OPENAI_API_KEY` or `ANTHROPIC_API_KEY`

For deeper troubleshooting, see the Full Guide.
