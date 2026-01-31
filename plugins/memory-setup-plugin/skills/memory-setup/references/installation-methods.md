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

Complete uninstallation guide. Follow steps in order.

### Step 1: Stop the Daemon

**Ensure the daemon is stopped before removing files:**

```bash
# Stop via CLI
memory-daemon stop

# Verify stopped
memory-daemon status
# Should show: not running

# Force kill if needed
pkill -f "memory-daemon.*start" || true
```

### Step 2: Remove Auto-Start

**macOS (launchd):**

```bash
# Unload and remove plist
launchctl unload ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist 2>/dev/null
rm -f ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist

# Verify
launchctl list | grep memory-daemon  # Should return nothing
```

**Linux (systemd):**

```bash
# Stop and disable service
systemctl --user stop memory-daemon
systemctl --user disable memory-daemon

# Remove service file
rm -f ~/.config/systemd/user/memory-daemon.service
systemctl --user daemon-reload

# Verify
systemctl --user status memory-daemon  # Should show "could not be found"
```

**Windows (Task Scheduler):**

```powershell
# Stop and remove scheduled task
Stop-ScheduledTask -TaskName "MemoryDaemon" -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName "MemoryDaemon" -Confirm:$false

# Verify
Get-ScheduledTask -TaskName "MemoryDaemon"  # Should error
```

### Step 3: Remove Binaries

**Find and remove installed binaries:**

```bash
# Find binary locations
which memory-daemon
which memory-ingest

# Remove from found locations
rm -f $(which memory-daemon 2>/dev/null)
rm -f $(which memory-ingest 2>/dev/null)

# Also check common locations
rm -f ~/.cargo/bin/memory-daemon
rm -f ~/.cargo/bin/memory-ingest
rm -f ~/.local/bin/memory-daemon
rm -f ~/.local/bin/memory-ingest
rm -f /usr/local/bin/memory-daemon
rm -f /usr/local/bin/memory-ingest

# Verify
which memory-daemon  # Should return nothing
```

**Windows:**

```powershell
# Remove binaries
Remove-Item "$env:USERPROFILE\.cargo\bin\memory-daemon.exe" -ErrorAction SilentlyContinue
Remove-Item "$env:USERPROFILE\.cargo\bin\memory-ingest.exe" -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\Programs\memory-daemon\*" -Recurse -ErrorAction SilentlyContinue
```

### Step 4: Remove Configuration (Optional)

**Only remove if you don't plan to reinstall:**

```bash
# Remove config directory
rm -rf ~/.config/memory-daemon

# Remove CCH hooks
rm -f ~/.claude/code_agent_context_hooks/hooks.yaml

# Remove project-level hooks (in each project)
rm -f .claude/hooks.yaml
```

**Windows:**

```powershell
Remove-Item "$env:APPDATA\memory-daemon" -Recurse -ErrorAction SilentlyContinue
```

### Step 5: Remove Data (Optional - DESTRUCTIVE)

**WARNING: This permanently destroys all conversation history!**

```bash
# List what will be deleted
ls -la ~/.memory-store

# Confirm you want to delete
echo "This will permanently delete all conversation history"
read -p "Type 'DELETE' to confirm: " confirm
if [ "$confirm" = "DELETE" ]; then
    rm -rf ~/.memory-store
    echo "Data removed"
fi
```

**Windows:**

```powershell
# Show what will be deleted
Get-ChildItem "$env:LOCALAPPDATA\memory-daemon\data" -Recurse

# Remove (BE CAREFUL!)
$confirm = Read-Host "Type 'DELETE' to permanently remove all data"
if ($confirm -eq "DELETE") {
    Remove-Item "$env:LOCALAPPDATA\memory-daemon" -Recurse -Force
    Write-Host "Data removed"
}
```

### Step 6: Clean Up Logs (Optional)

```bash
# macOS
rm -rf ~/Library/Logs/memory-daemon

# Linux
rm -rf ~/.local/state/memory-daemon
```

**Windows:**

```powershell
Remove-Item "$env:LOCALAPPDATA\memory-daemon\logs" -Recurse -ErrorAction SilentlyContinue
```

### Complete Uninstall Script

**For convenience, use the install-helper script:**

```bash
# Source the helper
source /path/to/install-helper.sh

# Full uninstall (keeps data and config)
uninstall

# Full uninstall including config
uninstall --remove-config

# Full uninstall including data (DESTRUCTIVE)
uninstall --remove-data --remove-config
```

### Verification Checklist

After uninstalling, verify these all return errors or "not found":

```bash
# Binary check
which memory-daemon
which memory-ingest
memory-daemon --version

# Process check
pgrep -f memory-daemon

# Auto-start check (macOS)
launchctl list | grep memory

# Auto-start check (Linux)
systemctl --user list-units | grep memory
```

### Reinstalling After Uninstall

If you kept your data (`~/.memory-store`), reinstalling will reconnect to existing conversation history:

```bash
# Reinstall binary
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon --force

# Recreate config
mkdir -p ~/.config/memory-daemon
memory-daemon config init  # Or run setup wizard

# Start daemon
memory-daemon start

# Verify data recovered
memory-daemon query root  # Should show existing TOC
```
