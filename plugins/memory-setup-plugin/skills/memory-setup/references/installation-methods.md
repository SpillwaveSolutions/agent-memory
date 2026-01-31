# Installation Methods

Detailed installation instructions for agent-memory components.

## Prerequisites

### Rust Toolchain

```bash
# Check if Rust is installed
rustc --version

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Protobuf Compiler (optional, for building from source)

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# Fedora
sudo dnf install protobuf-compiler
```

## Method 1: Cargo Install (Recommended)

Install from crates.io (when published) or directly from GitHub:

```bash
# From GitHub (latest)
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon

# From GitHub (specific version)
cargo install --git https://github.com/SpillwaveSolutions/agent-memory --tag v1.0.0 memory-daemon

# Install with CCH ingest handler
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-ingest
```

**Verify installation:**

```bash
memory-daemon --version
memory-ingest --version  # If installed
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
