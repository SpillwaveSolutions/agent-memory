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

---

## Method 2: Pre-built Binaries

Download pre-built binaries directly from GitHub releases. No Rust toolchain required.

### Release URL Pattern

Binaries are published at:

```
https://github.com/SpillwaveSolutions/agent-memory/releases/download/v{VERSION}/memory-{BINARY}-{OS}-{ARCH}.{EXT}
```

**Components:**

| Component | Values |
|-----------|--------|
| `{VERSION}` | `1.0.0`, `1.0.1`, `latest` |
| `{BINARY}` | `daemon`, `ingest` |
| `{OS}` | `darwin`, `linux`, `windows` |
| `{ARCH}` | `x86_64`, `arm64` |
| `{EXT}` | `.tar.gz` (macOS/Linux), `.zip` (Windows) |

### Platform Detection

**Detect your platform automatically:**

```bash
# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
  darwin) OS="darwin" ;;
  linux)  OS="linux" ;;
  mingw*|msys*|cygwin*) OS="windows" ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="arm64" ;;
esac

echo "Platform: ${OS}-${ARCH}"
```

### Platform-Specific Downloads

| Platform | Architecture | Filename | SHA256 |
|----------|--------------|----------|--------|
| macOS | arm64 (Apple Silicon) | `memory-daemon-darwin-arm64.tar.gz` | (in .sha256 file) |
| macOS | x86_64 (Intel) | `memory-daemon-darwin-x86_64.tar.gz` | (in .sha256 file) |
| Linux | x86_64 | `memory-daemon-linux-x86_64.tar.gz` | (in .sha256 file) |
| Linux | arm64 | `memory-daemon-linux-arm64.tar.gz` | (in .sha256 file) |
| Windows | x86_64 | `memory-daemon-windows-x86_64.zip` | (in .sha256 file) |

### Download Commands

**Automated download (latest release):**

```bash
# Set version (or "latest")
VERSION="latest"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
[[ "$ARCH" == "aarch64" ]] && ARCH="arm64"

# Get download URL for latest release
if [[ "$VERSION" == "latest" ]]; then
  RELEASE_URL=$(curl -sL https://api.github.com/repos/SpillwaveSolutions/agent-memory/releases/latest \
    | grep "browser_download_url.*memory-daemon-${OS}-${ARCH}" \
    | cut -d'"' -f4)
else
  RELEASE_URL="https://github.com/SpillwaveSolutions/agent-memory/releases/download/v${VERSION}/memory-daemon-${OS}-${ARCH}.tar.gz"
fi

echo "Downloading from: $RELEASE_URL"
```

**Download and extract:**

```bash
# Create temp directory
TMPDIR=$(mktemp -d)
cd "$TMPDIR"

# Download binary archive
curl -fsSL "$RELEASE_URL" -o memory-daemon.tar.gz

# Download checksum
curl -fsSL "${RELEASE_URL}.sha256" -o memory-daemon.tar.gz.sha256

# Verify checksum
echo "Verifying checksum..."
if command -v sha256sum &> /dev/null; then
  sha256sum -c memory-daemon.tar.gz.sha256
elif command -v shasum &> /dev/null; then
  shasum -a 256 -c memory-daemon.tar.gz.sha256
fi

# Extract
tar xzf memory-daemon.tar.gz

# List contents
ls -la
```

### Checksum Verification

**Always verify checksums to ensure binary integrity:**

```bash
# Each release includes .sha256 files
# memory-daemon-darwin-arm64.tar.gz.sha256 contains:
# abc123...  memory-daemon-darwin-arm64.tar.gz

# Verify on macOS
shasum -a 256 -c memory-daemon-*.sha256

# Verify on Linux
sha256sum -c memory-daemon-*.sha256

# Manual verification
EXPECTED=$(cat memory-daemon-*.sha256 | awk '{print $1}')
ACTUAL=$(shasum -a 256 memory-daemon-*.tar.gz | awk '{print $1}')
[[ "$EXPECTED" == "$ACTUAL" ]] && echo "Checksum OK" || echo "CHECKSUM MISMATCH!"
```

### Install Binary

**Install to user directory (no sudo required):**

```bash
# Create local bin directory
mkdir -p ~/.local/bin

# Move binaries
mv memory-daemon ~/.local/bin/
mv memory-ingest ~/.local/bin/

# Add to PATH (if not already)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify
which memory-daemon
memory-daemon --version
```

**Install to system directory (requires sudo):**

```bash
# Move to /usr/local/bin (common for manually installed binaries)
sudo mv memory-daemon /usr/local/bin/
sudo mv memory-ingest /usr/local/bin/

# Set permissions
sudo chmod 755 /usr/local/bin/memory-daemon
sudo chmod 755 /usr/local/bin/memory-ingest

# Verify
memory-daemon --version
```

### Windows Installation

```powershell
# Download using PowerShell
$version = "1.0.0"
$url = "https://github.com/SpillwaveSolutions/agent-memory/releases/download/v$version/memory-daemon-windows-x86_64.zip"
$outfile = "$env:TEMP\memory-daemon.zip"

Invoke-WebRequest -Uri $url -OutFile $outfile

# Extract
Expand-Archive -Path $outfile -DestinationPath "$env:TEMP\memory-daemon"

# Move to user directory
$bindir = "$env:LOCALAPPDATA\Programs\memory-daemon"
New-Item -ItemType Directory -Force -Path $bindir
Move-Item "$env:TEMP\memory-daemon\*" $bindir

# Add to PATH
$path = [Environment]::GetEnvironmentVariable("Path", "User")
if ($path -notlike "*$bindir*") {
  [Environment]::SetEnvironmentVariable("Path", "$path;$bindir", "User")
}

# Verify (new terminal required for PATH update)
memory-daemon.exe --version
```

### Troubleshooting Binary Download

| Issue | Cause | Solution |
|-------|-------|----------|
| 404 Not Found | Wrong version/platform | Check release page for available downloads |
| Checksum mismatch | Corrupted download | Re-download, check network |
| Permission denied | Not executable | Run `chmod +x memory-daemon` |
| "cannot execute binary" | Wrong architecture | Download correct arch (x86_64 vs arm64) |
| macOS "unidentified developer" | Gatekeeper | Run `xattr -d com.apple.quarantine memory-daemon` |

---

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
