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
    <true/>

    <key>StandardOutPath</key>
    <string>/Users/YOUR_USERNAME/Library/Logs/memory-daemon/stdout.log</string>

    <key>StandardErrorPath</key>
    <string>/Users/YOUR_USERNAME/Library/Logs/memory-daemon/stderr.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>OPENAI_API_KEY</key>
        <string>sk-your-key-here</string>
    </dict>
</dict>
</plist>
```

**Commands:**

```bash
# Load service
launchctl load ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist

# Unload service
launchctl unload ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist

# Check status
launchctl list | grep memory-daemon

# View logs
tail -f ~/Library/Logs/memory-daemon/stdout.log
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

Create `~/.config/systemd/user/memory-daemon.service`:

```ini
[Unit]
Description=Agent Memory Daemon
After=network.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/memory-daemon start --foreground
Restart=on-failure
RestartSec=5

Environment=OPENAI_API_KEY=sk-your-key-here
Environment=MEMORY_STORAGE_PATH=%h/.memory-store

[Install]
WantedBy=default.target
```

**Commands:**

```bash
# Reload systemd
systemctl --user daemon-reload

# Enable on startup
systemctl --user enable memory-daemon

# Start now
systemctl --user start memory-daemon

# Check status
systemctl --user status memory-daemon

# View logs
journalctl --user -u memory-daemon -f
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

Using PowerShell:

```powershell
# Create scheduled task
$action = New-ScheduledTaskAction -Execute "$env:USERPROFILE\.cargo\bin\memory-daemon.exe" -Argument "start --foreground"
$trigger = New-ScheduledTaskTrigger -AtLogon
$principal = New-ScheduledTaskPrincipal -UserId "$env:USERNAME" -LogonType Interactive
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries

Register-ScheduledTask -TaskName "MemoryDaemon" -Action $action -Trigger $trigger -Principal $principal -Settings $settings
```

**Management:**

```powershell
# Start task
Start-ScheduledTask -TaskName "MemoryDaemon"

# Stop task
Stop-ScheduledTask -TaskName "MemoryDaemon"

# Check status
Get-ScheduledTask -TaskName "MemoryDaemon" | Get-ScheduledTaskInfo

# Remove task
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
