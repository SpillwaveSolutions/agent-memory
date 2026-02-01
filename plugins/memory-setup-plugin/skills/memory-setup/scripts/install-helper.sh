#!/usr/bin/env bash
#
# install-helper.sh - Installation helper functions for agent-memory
#
# This script provides helper functions for detecting platform, checking
# installation status, and managing the memory-daemon installation.
#
# Usage:
#   source install-helper.sh
#   detect_platform
#   check_binary_installed
#
# Functions are designed to be called by the setup wizard or manually.

set -euo pipefail

# =============================================================================
# Platform Detection
# =============================================================================

# Detect operating system
# Sets: OS (darwin, linux, windows)
detect_os() {
    local os
    os=$(uname -s | tr '[:upper:]' '[:lower:]')

    case "$os" in
        darwin)
            OS="darwin"
            ;;
        linux)
            OS="linux"
            ;;
        mingw*|msys*|cygwin*)
            OS="windows"
            ;;
        *)
            OS="unknown"
            ;;
    esac

    export OS
    echo "$OS"
}

# Detect CPU architecture
# Sets: ARCH (x86_64, arm64)
detect_arch() {
    local arch
    arch=$(uname -m)

    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="arm64"
            ;;
        *)
            ARCH="unknown"
            ;;
    esac

    export ARCH
    echo "$ARCH"
}

# Detect full platform
# Sets: OS, ARCH, PLATFORM
# Returns: platform string (e.g., "darwin-arm64")
detect_platform() {
    detect_os >/dev/null
    detect_arch >/dev/null
    PLATFORM="${OS}-${ARCH}"
    export PLATFORM
    echo "$PLATFORM"
}

# =============================================================================
# Installation Checks
# =============================================================================

# Check if memory-daemon binary is installed
# Returns: 0 if installed, 1 if not
# Output: path to binary if found
check_binary_installed() {
    local binary="${1:-memory-daemon}"
    local path

    # Check in PATH
    if path=$(command -v "$binary" 2>/dev/null); then
        echo "$path"
        return 0
    fi

    # Check common locations
    local common_paths=(
        "$HOME/.cargo/bin/$binary"
        "$HOME/.local/bin/$binary"
        "/usr/local/bin/$binary"
        "/usr/bin/$binary"
    )

    for p in "${common_paths[@]}"; do
        if [[ -x "$p" ]]; then
            echo "$p"
            return 0
        fi
    done

    return 1
}

# Get installed binary version
# Args: binary name (default: memory-daemon)
# Output: version string or "unknown"
get_binary_version() {
    local binary="${1:-memory-daemon}"
    local version

    if command -v "$binary" &>/dev/null; then
        version=$("$binary" --version 2>/dev/null | head -1 | awk '{print $2}')
        echo "${version:-unknown}"
    else
        echo "not installed"
    fi
}

# Check if Rust toolchain is installed
# Returns: 0 if installed, 1 if not
check_rust_installed() {
    if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
        return 0
    fi
    return 1
}

# Get Rust version
# Output: version string or "not installed"
get_rust_version() {
    if check_rust_installed; then
        rustc --version | awk '{print $2}'
    else
        echo "not installed"
    fi
}

# =============================================================================
# Daemon Status Checks
# =============================================================================

# Check if memory-daemon is running
# Returns: 0 if running, 1 if not
# Output: PID if running
check_daemon_running() {
    # Try memory-daemon status command first
    if command -v memory-daemon &>/dev/null; then
        if memory-daemon status &>/dev/null; then
            # Extract PID from status output
            local pid
            pid=$(memory-daemon status 2>/dev/null | grep -oE 'PID:?\s*([0-9]+)' | grep -oE '[0-9]+' || true)
            if [[ -n "$pid" ]]; then
                echo "$pid"
                return 0
            fi
        fi
    fi

    # Fallback: check for process
    local pids
    pids=$(pgrep -f "memory-daemon.*start" 2>/dev/null || true)
    if [[ -n "$pids" ]]; then
        echo "$pids" | head -1
        return 0
    fi

    return 1
}

# Check if a port is available
# Args: port number
# Returns: 0 if available, 1 if in use
check_port_available() {
    local port="${1:-50051}"

    # Try netstat/ss
    if command -v ss &>/dev/null; then
        if ss -tuln | grep -q ":${port} "; then
            return 1
        fi
    elif command -v netstat &>/dev/null; then
        if netstat -tuln 2>/dev/null | grep -q ":${port} "; then
            return 1
        fi
    elif command -v lsof &>/dev/null; then
        if lsof -i ":${port}" &>/dev/null; then
            return 1
        fi
    fi

    return 0
}

# Get what process is using a port
# Args: port number
# Output: process info or "unknown"
get_port_user() {
    local port="${1:-50051}"

    if command -v lsof &>/dev/null; then
        lsof -i ":${port}" 2>/dev/null | tail -1 || echo "unknown"
    elif command -v ss &>/dev/null; then
        ss -tlnp "sport = :${port}" 2>/dev/null | tail -1 || echo "unknown"
    else
        echo "unknown"
    fi
}

# =============================================================================
# Installation Functions
# =============================================================================

# Install memory-daemon via cargo
# Args: [version] - optional version tag
# Returns: 0 on success, 1 on failure
install_binary_cargo() {
    local version="${1:-}"

    if ! check_rust_installed; then
        echo "Error: Rust toolchain not installed" >&2
        echo "Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
        return 1
    fi

    local args=(
        "install"
        "--git" "https://github.com/SpillwaveSolutions/agent-memory"
    )

    if [[ -n "$version" ]]; then
        args+=("--tag" "v${version}")
    fi

    args+=("memory-daemon")

    echo "Installing memory-daemon via cargo..."
    cargo "${args[@]}"

    # Also install memory-ingest
    args=("install" "--git" "https://github.com/SpillwaveSolutions/agent-memory")
    if [[ -n "$version" ]]; then
        args+=("--tag" "v${version}")
    fi
    args+=("memory-ingest")

    echo "Installing memory-ingest via cargo..."
    cargo "${args[@]}"

    return 0
}

# Install memory-daemon via binary download
# Args: [version] - version tag (default: latest)
# Returns: 0 on success, 1 on failure
install_binary_download() {
    local version="${1:-latest}"

    detect_platform >/dev/null

    if [[ "$OS" == "unknown" || "$ARCH" == "unknown" ]]; then
        echo "Error: Unknown platform: $PLATFORM" >&2
        return 1
    fi

    local base_url="https://github.com/SpillwaveSolutions/agent-memory/releases"
    local download_url
    local ext="tar.gz"
    [[ "$OS" == "windows" ]] && ext="zip"

    if [[ "$version" == "latest" ]]; then
        # Get latest release URL
        download_url=$(curl -sL "${base_url}/latest" |
            grep -oE "href=\"[^\"]*memory-daemon-${OS}-${ARCH}\.${ext}\"" |
            head -1 | sed 's/href="//;s/"$//')

        if [[ -n "$download_url" ]]; then
            download_url="https://github.com${download_url}"
        fi
    else
        download_url="${base_url}/download/v${version}/memory-daemon-${OS}-${ARCH}.${ext}"
    fi

    if [[ -z "$download_url" ]]; then
        echo "Error: Could not determine download URL" >&2
        return 1
    fi

    echo "Downloading from: $download_url"

    # Create temp directory
    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf '$tmpdir'" EXIT

    # Download
    if ! curl -fsSL "$download_url" -o "$tmpdir/memory-daemon.$ext"; then
        echo "Error: Download failed" >&2
        return 1
    fi

    # Download checksum
    if curl -fsSL "${download_url}.sha256" -o "$tmpdir/memory-daemon.$ext.sha256" 2>/dev/null; then
        # Verify checksum
        echo "Verifying checksum..."
        (cd "$tmpdir" && sha256sum -c "memory-daemon.$ext.sha256" 2>/dev/null) ||
        (cd "$tmpdir" && shasum -a 256 -c "memory-daemon.$ext.sha256" 2>/dev/null) ||
        echo "Warning: Could not verify checksum"
    fi

    # Extract
    cd "$tmpdir"
    if [[ "$ext" == "tar.gz" ]]; then
        tar xzf "memory-daemon.$ext"
    else
        unzip -q "memory-daemon.$ext"
    fi

    # Install to ~/.local/bin
    local install_dir="$HOME/.local/bin"
    mkdir -p "$install_dir"

    mv memory-daemon "$install_dir/"
    chmod +x "$install_dir/memory-daemon"

    if [[ -f memory-ingest ]]; then
        mv memory-ingest "$install_dir/"
        chmod +x "$install_dir/memory-ingest"
    fi

    echo "Installed to: $install_dir"
    echo "Ensure $install_dir is in your PATH"

    return 0
}

# =============================================================================
# Auto-start Setup Functions
# =============================================================================

# Setup auto-start based on current platform
# Returns: 0 on success, 1 on failure
setup_autostart() {
    detect_platform >/dev/null

    case "$OS" in
        darwin)
            setup_autostart_macos
            ;;
        linux)
            setup_autostart_linux
            ;;
        windows)
            echo "For Windows, run the PowerShell script in platform-specifics.md"
            return 1
            ;;
        *)
            echo "Error: Unsupported platform for auto-start" >&2
            return 1
            ;;
    esac
}

# Setup launchd auto-start on macOS
setup_autostart_macos() {
    local plist_path="$HOME/Library/LaunchAgents/com.spillwave.memory-daemon.plist"
    local daemon_path

    daemon_path=$(check_binary_installed memory-daemon) || {
        echo "Error: memory-daemon not installed" >&2
        return 1
    }

    # Create log directory
    mkdir -p "$HOME/Library/Logs/memory-daemon"

    # Generate plist
    cat > "$plist_path" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.spillwave.memory-daemon</string>

    <key>ProgramArguments</key>
    <array>
        <string>${daemon_path}</string>
        <string>start</string>
        <string>--foreground</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>10</integer>

    <key>StandardOutPath</key>
    <string>${HOME}/Library/Logs/memory-daemon/stdout.log</string>

    <key>StandardErrorPath</key>
    <string>${HOME}/Library/Logs/memory-daemon/stderr.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:${HOME}/.cargo/bin:${HOME}/.local/bin</string>
    </dict>
</dict>
</plist>
EOF

    # Load service
    launchctl load "$plist_path"

    echo "Auto-start enabled: $plist_path"
    return 0
}

# Setup systemd auto-start on Linux
setup_autostart_linux() {
    local service_path="$HOME/.config/systemd/user/memory-daemon.service"
    local daemon_path

    daemon_path=$(check_binary_installed memory-daemon) || {
        echo "Error: memory-daemon not installed" >&2
        return 1
    }

    # Create systemd user directory
    mkdir -p "$HOME/.config/systemd/user"

    # Generate service file
    cat > "$service_path" << EOF
[Unit]
Description=Agent Memory Daemon
Documentation=https://github.com/SpillwaveSolutions/agent-memory
After=network.target

[Service]
Type=simple
ExecStart=${daemon_path} start --foreground
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=10

Environment=MEMORY_STORAGE_PATH=%h/.memory-store
Environment=PATH=/usr/local/bin:/usr/bin:/bin:%h/.cargo/bin:%h/.local/bin

[Install]
WantedBy=default.target
EOF

    # Reload and enable
    systemctl --user daemon-reload
    systemctl --user enable memory-daemon
    systemctl --user start memory-daemon

    echo "Auto-start enabled: $service_path"
    return 0
}

# Remove auto-start based on current platform
# Returns: 0 on success, 1 on failure
remove_autostart() {
    detect_platform >/dev/null

    case "$OS" in
        darwin)
            remove_autostart_macos
            ;;
        linux)
            remove_autostart_linux
            ;;
        windows)
            echo "For Windows, run: Unregister-ScheduledTask -TaskName 'MemoryDaemon' -Confirm:\$false"
            return 1
            ;;
        *)
            echo "Error: Unsupported platform" >&2
            return 1
            ;;
    esac
}

# Remove launchd auto-start on macOS
remove_autostart_macos() {
    local plist_path="$HOME/Library/LaunchAgents/com.spillwave.memory-daemon.plist"

    if [[ -f "$plist_path" ]]; then
        launchctl unload "$plist_path" 2>/dev/null || true
        rm "$plist_path"
        echo "Auto-start removed"
    else
        echo "Auto-start not configured"
    fi

    return 0
}

# Remove systemd auto-start on Linux
remove_autostart_linux() {
    local service_path="$HOME/.config/systemd/user/memory-daemon.service"

    if [[ -f "$service_path" ]]; then
        systemctl --user stop memory-daemon 2>/dev/null || true
        systemctl --user disable memory-daemon 2>/dev/null || true
        rm "$service_path"
        systemctl --user daemon-reload
        echo "Auto-start removed"
    else
        echo "Auto-start not configured"
    fi

    return 0
}

# =============================================================================
# Configuration Generation
# =============================================================================

# Generate default config.toml
# Args: [storage_path] [port] [provider] [model]
generate_config() {
    local storage_path="${1:-~/.memory-store}"
    local port="${2:-50051}"
    local provider="${3:-openai}"
    local model="${4:-gpt-4o-mini}"
    local config_dir="$HOME/.config/memory-daemon"
    local config_path="$config_dir/config.toml"

    mkdir -p "$config_dir"
    chmod 700 "$config_dir"

    cat > "$config_path" << EOF
# Agent Memory Configuration
# Generated by install-helper.sh

[storage]
path = "$storage_path"

[server]
host = "[::1]"
port = $port

[summarizer]
provider = "$provider"
model = "$model"
# API key loaded from environment variable

[toc]
segment_min_tokens = 500
segment_max_tokens = 4000
time_gap_minutes = 30

[logging]
level = "info"
format = "pretty"
EOF

    chmod 600 "$config_path"
    echo "Config written to: $config_path"
}

# Generate global hooks.yaml
generate_hooks_global() {
    local hooks_dir="$HOME/.claude/code_agent_context_hooks"
    local hooks_path="$hooks_dir/hooks.yaml"

    mkdir -p "$hooks_dir"

    cat > "$hooks_path" << 'EOF'
# Claude Code Hooks Configuration
# Generated by install-helper.sh

version: "1"

hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
      fail_open: true
EOF

    chmod 644 "$hooks_path"
    echo "Hooks written to: $hooks_path"
}

# =============================================================================
# Uninstall Functions
# =============================================================================

# Full uninstall
# Args: [--remove-data] [--remove-config]
uninstall() {
    local remove_data=false
    local remove_config=false

    for arg in "$@"; do
        case "$arg" in
            --remove-data) remove_data=true ;;
            --remove-config) remove_config=true ;;
        esac
    done

    echo "Stopping daemon..."
    if check_daemon_running >/dev/null; then
        memory-daemon stop 2>/dev/null || true
    fi

    echo "Removing auto-start..."
    remove_autostart 2>/dev/null || true

    echo "Removing binaries..."
    local binary_path
    if binary_path=$(check_binary_installed memory-daemon 2>/dev/null); then
        rm -f "$binary_path"
        echo "Removed: $binary_path"
    fi
    if binary_path=$(check_binary_installed memory-ingest 2>/dev/null); then
        rm -f "$binary_path"
        echo "Removed: $binary_path"
    fi

    if [[ "$remove_config" == true ]]; then
        echo "Removing configuration..."
        rm -rf "$HOME/.config/memory-daemon"
        rm -f "$HOME/.claude/code_agent_context_hooks/hooks.yaml"
    fi

    if [[ "$remove_data" == true ]]; then
        echo "Removing data (WARNING: destroys conversation history)..."
        rm -rf "$HOME/.memory-store"
    fi

    echo "Uninstall complete"
}

# =============================================================================
# Main (for direct execution)
# =============================================================================

# If script is executed directly, run the specified function
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <function> [args...]"
        echo ""
        echo "Functions:"
        echo "  detect_platform         - Detect OS and architecture"
        echo "  check_binary_installed  - Check if memory-daemon is installed"
        echo "  check_daemon_running    - Check if daemon is running"
        echo "  check_port_available    - Check if port is available"
        echo "  install_binary_cargo    - Install via cargo"
        echo "  install_binary_download - Install via binary download"
        echo "  setup_autostart         - Setup auto-start"
        echo "  remove_autostart        - Remove auto-start"
        echo "  generate_config         - Generate config.toml"
        echo "  generate_hooks_global   - Generate global hooks.yaml"
        echo "  uninstall               - Full uninstall"
        exit 0
    fi

    func="$1"
    shift
    "$func" "$@"
fi
