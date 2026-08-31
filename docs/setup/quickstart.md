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

One archive per platform contains all four binaries: `memory-daemon`,
`memory-ingest`, `memory` (the CLI), and `memory-installer`. Assets are named
`agent-memory-<version>-<platform>.tar.gz`, and the tarball unpacks into a
directory of that name.

```bash
mkdir -p ~/.local/bin
VERSION=3.1.0
case "$(uname -s)" in Darwin) OS=macos ;; Linux) OS=linux ;; esac
case "$(uname -m)" in x86_64|amd64) ARCH=x86_64 ;; arm64|aarch64) ARCH=aarch64 ;; esac
ASSET="agent-memory-${VERSION}-${OS}-${ARCH}"

curl -fL "https://github.com/SpillwaveSolutions/agent-memory/releases/download/v${VERSION}/${ASSET}.tar.gz" \
  | tar xz -C /tmp
install -m 0755 /tmp/${ASSET}/memory-daemon /tmp/${ASSET}/memory-ingest \
                /tmp/${ASSET}/memory /tmp/${ASSET}/memory-installer ~/.local/bin/
```

Verify the download against the release's `SHA256SUMS.txt` if you want to.

### 3) Add binaries to PATH

- [ ] Ensure `~/.local/bin` is on your PATH

```bash
export PATH="$HOME/.local/bin:$PATH"
```

> Verify now (optional):
> - `memory-daemon --version`
> - `memory-ingest --version`
> - `memory --version`

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
