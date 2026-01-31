# Installation Methods

Detailed installation instructions for agent-memory components.

## Prerequisites

### Required: Rust Toolchain

The Rust toolchain is required for cargo installation (Method 1).

**Check existing installation:**

```bash
rustc --version    # Should show 1.75.0 or later
cargo --version    # Should match rustc version
```

**Install Rust (if not installed):**

```bash
# Official installer (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts, then reload shell
source ~/.cargo/env

# Or restart your terminal
```

**Troubleshooting Rust installation:**

| Issue | Solution |
|-------|----------|
| `command not found: rustc` | Run `source ~/.cargo/env` or restart terminal |
| Version too old | Run `rustup update stable` |
| Permission denied | Don't use `sudo` with rustup; it installs to `~/.cargo` |
| SSL certificate errors | Update CA certificates: `apt-get install ca-certificates` (Linux) |

### Optional: Protobuf Compiler (for building from source only)

Only needed if building from source (Method 3). Pre-built binaries and cargo install do NOT require protobuf.

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# Fedora
sudo dnf install protobuf-compiler

# Arch Linux
sudo pacman -S protobuf
```

---

## Method 1: Cargo Install (Recommended)

Cargo install is the recommended method. It compiles binaries optimized for your system and places them in `~/.cargo/bin/`.

### Prerequisites

- Rust toolchain (see above)
- Internet connection
- C compiler (usually pre-installed on macOS/Linux)

### Installation Commands

**Install from GitHub (latest release):**

```bash
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-ingest
```

**Install specific version:**

```bash
# By tag (recommended for stability)
cargo install --git https://github.com/SpillwaveSolutions/agent-memory --tag v1.0.0 memory-daemon

# By branch
cargo install --git https://github.com/SpillwaveSolutions/agent-memory --branch main memory-daemon

# By commit
cargo install --git https://github.com/SpillwaveSolutions/agent-memory --rev abc1234 memory-daemon
```

**Install from crates.io (when published):**

```bash
cargo install memory-daemon
cargo install memory-ingest
```

### Verify Installation

```bash
# Check binaries exist and are in PATH
which memory-daemon
which memory-ingest

# Check versions
memory-daemon --version
memory-ingest --version

# Expected output:
# memory-daemon 1.0.0
# memory-ingest 1.0.0
```

### Troubleshooting Cargo Install

| Issue | Cause | Solution |
|-------|-------|----------|
| `error: linker 'cc' not found` | Missing C compiler | macOS: `xcode-select --install`; Linux: `apt install build-essential` |
| `error: failed to compile` | Missing dependencies | Install libssl-dev (Linux): `apt install pkg-config libssl-dev` |
| `error: binary already exists` | Previous installation | Add `--force` flag to overwrite |
| Very slow compile | Debug build | Normal for first install (~5-10 min) |
| Out of memory | Limited RAM | Add `--jobs 1` to reduce parallelism |

### Updating

```bash
# Update to latest
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon --force

# Note: --force is required to overwrite existing binary
```

## Method 2: Pre-built Binaries

Download pre-built binaries from GitHub releases:

```bash
# Get latest release URL
RELEASE_URL=$(curl -s https://api.github.com/repos/SpillwaveSolutions/agent-memory/releases/latest | grep browser_download_url | grep $(uname -s | tr '[:upper:]' '[:lower:]') | cut -d'"' -f4)

# Download and extract
curl -L "$RELEASE_URL" | tar xz

# Move to bin directory
sudo mv memory-daemon /usr/local/bin/
sudo mv memory-ingest /usr/local/bin/
```

### Platform-Specific Downloads

| Platform | Architecture | Filename |
|----------|--------------|----------|
| macOS | arm64 (Apple Silicon) | `memory-daemon-darwin-arm64.tar.gz` |
| macOS | x86_64 (Intel) | `memory-daemon-darwin-x86_64.tar.gz` |
| Linux | x86_64 | `memory-daemon-linux-x86_64.tar.gz` |
| Linux | arm64 | `memory-daemon-linux-arm64.tar.gz` |
| Windows | x86_64 | `memory-daemon-windows-x86_64.zip` |

## Method 3: Build from Source

Clone and build the full workspace:

```bash
# Clone repository
git clone https://github.com/SpillwaveSolutions/agent-memory.git
cd agent-memory

# Build all binaries
cargo build --release

# Binaries are in target/release/
ls target/release/memory-daemon
ls target/release/memory-ingest
```

**Install locally:**

```bash
cargo install --path crates/memory-daemon
cargo install --path crates/memory-ingest
```

## Post-Installation Setup

### 1. Create Configuration Directory

```bash
mkdir -p ~/.config/memory-daemon
```

### 2. Create Default Configuration

```bash
cat > ~/.config/memory-daemon/config.toml << 'EOF'
[storage]
path = "~/.memory-store"

[server]
host = "[::1]"
port = 50051

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
# api_key loaded from OPENAI_API_KEY env var

[toc]
segment_min_tokens = 500
segment_max_tokens = 4000
time_gap_minutes = 30
EOF
```

### 3. Create Data Directory

```bash
mkdir -p ~/.memory-store
```

### 4. Test the Installation

```bash
# Start the daemon
memory-daemon start

# Check status
memory-daemon status

# Should show: running on [::1]:50051
```

## CCH Integration (Optional)

If using Claude Code Hooks for automatic event capture:

### 1. Install CCH

```bash
# Follow CCH installation instructions
# https://github.com/SpillwaveSolutions/code_agent_context_hooks
```

### 2. Configure Hook

Add to `~/.claude/code_agent_context_hooks/hooks.yaml`:

```yaml
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
```

### 3. Verify Integration

```bash
# Send a test event
echo '{"type":"session_start","timestamp":"2026-01-31T12:00:00Z"}' | memory-ingest
```

## Updating

```bash
# Update via cargo
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon --force

# Restart daemon after update
memory-daemon stop
memory-daemon start
```

## Uninstalling

```bash
# Remove binaries
rm $(which memory-daemon)
rm $(which memory-ingest)

# Optional: Remove data (WARNING: destroys all conversation history)
rm -rf ~/.memory-store

# Optional: Remove configuration
rm -rf ~/.config/memory-daemon
```
