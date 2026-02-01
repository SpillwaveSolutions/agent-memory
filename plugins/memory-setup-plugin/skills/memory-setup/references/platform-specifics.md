# Platform Specifics

Platform-specific installation, configuration, and operation details.

## macOS

### Paths

| Purpose | Path |
|---------|------|
| Binary | `~/.cargo/bin/memory-daemon` |
| Config | `~/.config/memory-daemon/config.toml` |
| Data | `~/.memory-store/` |
| Logs | `~/Library/Logs/memory-daemon/` |
| PID file | `~/Library/Application Support/memory-daemon/daemon.pid` |

### Install via Homebrew (Future)

```bash
# Not yet available, use cargo install
brew install spillwave/tap/memory-daemon
```

### launchd Service (Auto-start)

launchd is macOS's native service manager. It starts the daemon at login and restarts it if it crashes.

#### Setup Script (Automated)

The setup wizard generates the plist file automatically. Here's what it does:

```bash
#!/bin/bash
# Setup launchd auto-start for memory-daemon

# Get current user info
USERNAME=$(whoami)
HOME_DIR=$(eval echo "~$USERNAME")
PLIST_PATH="$HOME_DIR/Library/LaunchAgents/com.spillwave.memory-daemon.plist"

# Create log directory
mkdir -p "$HOME_DIR/Library/Logs/memory-daemon"

# Find memory-daemon binary
DAEMON_PATH=$(which memory-daemon 2>/dev/null || echo "$HOME_DIR/.cargo/bin/memory-daemon")

# Generate plist
cat > "$PLIST_PATH" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.spillwave.memory-daemon</string>

    <key>ProgramArguments</key>
    <array>
        <string>$DAEMON_PATH</string>
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
    <string>$HOME_DIR/Library/Logs/memory-daemon/stdout.log</string>

    <key>StandardErrorPath</key>
    <string>$HOME_DIR/Library/Logs/memory-daemon/stderr.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:$HOME_DIR/.cargo/bin</string>
    </dict>
</dict>
</plist>
EOF

# Load the service
launchctl load "$PLIST_PATH"

echo "Auto-start enabled. Daemon will start on login."
```

#### Manual Setup

Create `~/Library/LaunchAgents/com.spillwave.memory-daemon.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.spillwave.memory-daemon</string>

    <key>ProgramArguments</key>
    <array>
        <string>/Users/YOUR_USERNAME/.cargo/bin/memory-daemon</string>
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
    <string>/Users/YOUR_USERNAME/Library/Logs/memory-daemon/stdout.log</string>

    <key>StandardErrorPath</key>
    <string>/Users/YOUR_USERNAME/Library/Logs/memory-daemon/stderr.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>OPENAI_API_KEY</key>
        <string>sk-your-key-here</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/Users/YOUR_USERNAME/.cargo/bin</string>
    </dict>
</dict>
</plist>
```

**Important:** Replace `YOUR_USERNAME` with your actual username.

#### Management Commands

```bash
# Load service (enable auto-start)
launchctl load ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist

# Unload service (disable auto-start)
launchctl unload ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist

# Start service now (without waiting for login)
launchctl start com.spillwave.memory-daemon

# Stop service
launchctl stop com.spillwave.memory-daemon

# Check if service is running
launchctl list | grep memory-daemon

# View service info
launchctl print gui/$(id -u)/com.spillwave.memory-daemon

# View logs
tail -f ~/Library/Logs/memory-daemon/stdout.log
tail -f ~/Library/Logs/memory-daemon/stderr.log
```

#### Troubleshooting launchd

| Issue | Cause | Solution |
|-------|-------|----------|
| Service won't load | Invalid plist syntax | Run `plutil -lint ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist` |
| Starts then stops immediately | Missing binary or wrong path | Verify `which memory-daemon` matches plist path |
| No logs appearing | Log directory doesn't exist | `mkdir -p ~/Library/Logs/memory-daemon` |
| Environment variables not set | Not in plist | Add to EnvironmentVariables dict in plist |
| Restarts too fast | Crashes repeatedly | Check stderr.log, increase ThrottleInterval |

#### Removing Auto-start

```bash
# Unload service
launchctl unload ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist

# Remove plist file
rm ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist
```

### Apple Silicon (M1/M2/M3)

No special configuration needed. The binary compiles natively for arm64.

```bash
# Verify architecture
file $(which memory-daemon)
# Should show: Mach-O 64-bit executable arm64
```

---

## Linux

### Paths

| Purpose | Path |
|---------|------|
| Binary | `~/.cargo/bin/memory-daemon` |
| Config | `~/.config/memory-daemon/config.toml` |
| Data | `~/.local/share/memory-daemon/` or `~/.memory-store/` |
| Logs | `~/.local/state/memory-daemon/` |
| PID file | `$XDG_RUNTIME_DIR/memory-daemon.pid` or `/tmp/memory-daemon.pid` |

### systemd Service (Auto-start)

systemd user services run without root privileges and start automatically at login.

#### Setup Script (Automated)

The setup wizard generates the service file automatically:

```bash
#!/bin/bash
# Setup systemd user service for memory-daemon

# Create systemd user directory
mkdir -p ~/.config/systemd/user

# Find memory-daemon binary
DAEMON_PATH=$(which memory-daemon 2>/dev/null || echo "$HOME/.cargo/bin/memory-daemon")

# Generate service file
cat > ~/.config/systemd/user/memory-daemon.service << EOF
[Unit]
Description=Agent Memory Daemon
Documentation=https://github.com/SpillwaveSolutions/agent-memory
After=network.target

[Service]
Type=simple
ExecStart=$DAEMON_PATH start --foreground
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=10

# Environment variables
Environment=MEMORY_STORAGE_PATH=%h/.memory-store
Environment=PATH=/usr/local/bin:/usr/bin:/bin:%h/.cargo/bin

# Resource limits (optional)
# MemoryMax=512M
# CPUQuota=50%

[Install]
WantedBy=default.target
EOF

# Reload systemd
systemctl --user daemon-reload

# Enable and start
systemctl --user enable memory-daemon
systemctl --user start memory-daemon

echo "Auto-start enabled. Daemon is running."
```

#### Manual Setup

Create `~/.config/systemd/user/memory-daemon.service`:

```ini
[Unit]
Description=Agent Memory Daemon
Documentation=https://github.com/SpillwaveSolutions/agent-memory
After=network.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/memory-daemon start --foreground
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=10

# Environment from file (recommended for API keys)
EnvironmentFile=-%h/.config/memory-daemon/env

# Or inline environment variables
Environment=MEMORY_STORAGE_PATH=%h/.memory-store
Environment=MEMORY_LOG_LEVEL=info

[Install]
WantedBy=default.target
```

**Environment file (optional):** Create `~/.config/memory-daemon/env`:

```bash
OPENAI_API_KEY=sk-your-key-here
MEMORY_STORAGE_PATH=/home/username/.memory-store
```

#### Management Commands

```bash
# Reload systemd configuration
systemctl --user daemon-reload

# Enable auto-start on login
systemctl --user enable memory-daemon

# Disable auto-start
systemctl --user disable memory-daemon

# Start service now
systemctl --user start memory-daemon

# Stop service
systemctl --user stop memory-daemon

# Restart service
systemctl --user restart memory-daemon

# Check status
systemctl --user status memory-daemon

# View logs
journalctl --user -u memory-daemon -f

# View last 100 log lines
journalctl --user -u memory-daemon -n 100

# View logs since boot
journalctl --user -u memory-daemon -b
```

#### Enable Linger (Run Without Login)

By default, user services stop when you log out. To keep the daemon running:

```bash
# Enable lingering (keeps user services running after logout)
sudo loginctl enable-linger $(whoami)

# Check linger status
loginctl show-user $(whoami) | grep Linger
```

#### Troubleshooting systemd

| Issue | Cause | Solution |
|-------|-------|----------|
| Service won't start | Invalid service file | `systemctl --user status memory-daemon` for details |
| "Unit not found" | Typo in filename | Must be `.service` extension, check filename |
| Stops on logout | Linger not enabled | `sudo loginctl enable-linger $(whoami)` |
| Environment not set | EnvironmentFile missing | Use `Environment=` directly or create env file |
| Permission denied | Wrong permissions | `chmod 644 ~/.config/systemd/user/memory-daemon.service` |

#### Removing Auto-start

```bash
# Stop and disable service
systemctl --user stop memory-daemon
systemctl --user disable memory-daemon

# Remove service file
rm ~/.config/systemd/user/memory-daemon.service

# Reload systemd
systemctl --user daemon-reload
```

### Distribution-Specific Notes

#### Ubuntu/Debian

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev

# Install protobuf (for building from source)
sudo apt-get install -y protobuf-compiler
```

#### Fedora/RHEL

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo dnf install -y gcc openssl-devel

# Install protobuf (for building from source)
sudo dnf install -y protobuf-compiler
```

#### Arch Linux

```bash
# Install Rust
sudo pacman -S rustup
rustup default stable

# Install build dependencies
sudo pacman -S base-devel openssl

# Install protobuf (for building from source)
sudo pacman -S protobuf
```

---

## Windows

### Paths

| Purpose | Path |
|---------|------|
| Binary | `%USERPROFILE%\.cargo\bin\memory-daemon.exe` |
| Config | `%APPDATA%\memory-daemon\config.toml` |
| Data | `%LOCALAPPDATA%\memory-daemon\data\` |
| Logs | `%LOCALAPPDATA%\memory-daemon\logs\` |

### Install via winget (Future)

```powershell
# Not yet available, use cargo install
winget install SpillwaveSolutions.memory-daemon
```

### Install via Cargo

```powershell
# Install Rust first (download from https://rustup.rs)
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon
```

### Task Scheduler (Auto-start)

Windows Task Scheduler can start the daemon at login and restart it if it crashes.

#### Setup Script (Automated)

The setup wizard generates and registers the task automatically:

```powershell
# Setup Task Scheduler auto-start for memory-daemon

# Find memory-daemon binary
$DaemonPath = "$env:USERPROFILE\.cargo\bin\memory-daemon.exe"
if (-not (Test-Path $DaemonPath)) {
    $DaemonPath = (Get-Command memory-daemon -ErrorAction SilentlyContinue).Source
    if (-not $DaemonPath) {
        Write-Error "memory-daemon.exe not found"
        exit 1
    }
}

# Create log directory
$LogDir = "$env:LOCALAPPDATA\memory-daemon\logs"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# Define task action
$action = New-ScheduledTaskAction `
    -Execute $DaemonPath `
    -Argument "start --foreground" `
    -WorkingDirectory "$env:USERPROFILE"

# Trigger at logon
$trigger = New-ScheduledTaskTrigger -AtLogon -User $env:USERNAME

# Run as current user, visible window (optional: use -WindowStyle Hidden)
$principal = New-ScheduledTaskPrincipal `
    -UserId $env:USERNAME `
    -LogonType Interactive `
    -RunLevel Limited

# Task settings
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -RestartCount 3 `
    -ExecutionTimeLimit (New-TimeSpan -Days 365)

# Register task
Register-ScheduledTask `
    -TaskName "MemoryDaemon" `
    -Action $action `
    -Trigger $trigger `
    -Principal $principal `
    -Settings $settings `
    -Description "Agent Memory Daemon - gRPC service for conversation history" `
    -Force

# Start task immediately
Start-ScheduledTask -TaskName "MemoryDaemon"

Write-Host "Auto-start enabled. Daemon is running."
```

#### Manual Setup via PowerShell

```powershell
# Create scheduled task
$action = New-ScheduledTaskAction `
    -Execute "$env:USERPROFILE\.cargo\bin\memory-daemon.exe" `
    -Argument "start --foreground"

$trigger = New-ScheduledTaskTrigger -AtLogon

$principal = New-ScheduledTaskPrincipal `
    -UserId "$env:USERNAME" `
    -LogonType Interactive

$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable

Register-ScheduledTask `
    -TaskName "MemoryDaemon" `
    -Action $action `
    -Trigger $trigger `
    -Principal $principal `
    -Settings $settings
```

#### Manual Setup via Task Scheduler GUI

1. Open Task Scheduler (taskschd.msc)
2. Click "Create Task" (not "Create Basic Task")
3. **General tab:**
   - Name: MemoryDaemon
   - Description: Agent Memory Daemon
   - "Run only when user is logged on"
4. **Triggers tab:**
   - New > At log on > Specific user: Your username
5. **Actions tab:**
   - New > Start a program
   - Program: `C:\Users\YOUR_USERNAME\.cargo\bin\memory-daemon.exe`
   - Arguments: `start --foreground`
6. **Conditions tab:**
   - Uncheck "Start only if on AC power"
7. **Settings tab:**
   - Check "Allow task to be run on demand"
   - Check "If the task fails, restart every: 1 minute"
   - Set "Attempt to restart up to: 3 times"

#### Management Commands

```powershell
# Start task immediately
Start-ScheduledTask -TaskName "MemoryDaemon"

# Stop task
Stop-ScheduledTask -TaskName "MemoryDaemon"

# Check if task is running
Get-ScheduledTask -TaskName "MemoryDaemon" | Select-Object State

# Get detailed status
Get-ScheduledTask -TaskName "MemoryDaemon" | Get-ScheduledTaskInfo

# List all runs (last run result)
Get-ScheduledTask -TaskName "MemoryDaemon" | Get-ScheduledTaskInfo |
    Select-Object LastRunTime, LastTaskResult, NextRunTime

# View task XML (full configuration)
Export-ScheduledTask -TaskName "MemoryDaemon"
```

#### Troubleshooting Task Scheduler

| Issue | Cause | Solution |
|-------|-------|----------|
| Task won't start | Wrong path | Verify `Test-Path "$env:USERPROFILE\.cargo\bin\memory-daemon.exe"` |
| "Access denied" | Insufficient permissions | Re-create with current user as principal |
| Starts then stops | Crash on startup | Check logs in `%LOCALAPPDATA%\memory-daemon\logs` |
| Task runs but daemon not listening | Wrong arguments | Ensure `--foreground` flag is present |
| LastTaskResult = 1 | Process exited with error | Check daemon logs for startup errors |

**Task result codes:**

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Process exited with error |
| 267009 | Task currently running |
| 267014 | Task terminated by user |

#### Removing Auto-start

```powershell
# Stop and remove scheduled task
Stop-ScheduledTask -TaskName "MemoryDaemon" -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName "MemoryDaemon" -Confirm:$false
```

### Windows Subsystem for Linux (WSL)

For WSL users, you can run memory-daemon inside WSL:

```bash
# Inside WSL
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon

# Start daemon
memory-daemon start

# Access from Windows via localhost
# The daemon listens on [::1]:50051 which is accessible from Windows
```

### Firewall Configuration

If accessing from other machines:

```powershell
# Allow inbound connections on port 50051
New-NetFirewallRule -DisplayName "Memory Daemon" -Direction Inbound -Protocol TCP -LocalPort 50051 -Action Allow
```

---

## Docker (Cross-Platform)

For a containerized deployment:

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
RUN git clone https://github.com/SpillwaveSolutions/agent-memory.git .
RUN cargo build --release -p memory-daemon

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/memory-daemon /usr/local/bin/

EXPOSE 50051
VOLUME /data
VOLUME /config

ENV MEMORY_STORAGE_PATH=/data
ENV MEMORY_CONFIG_PATH=/config/config.toml

CMD ["memory-daemon", "start", "--foreground"]
```

**Run:**

```bash
docker build -t memory-daemon .
docker run -d \
  -p 50051:50051 \
  -v memory-data:/data \
  -v $(pwd)/config:/config \
  -e OPENAI_API_KEY=$OPENAI_API_KEY \
  memory-daemon
```
