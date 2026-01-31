# Usage Guide

## Starting the Daemon

### Basic Start

```bash
memory-daemon start
```

The daemon starts on port 50051 with database at `~/.memory-store`.

### With Options

```bash
memory-daemon start \
  --port 50052 \
  --db-path /var/lib/memory-store \
  --config /etc/memory-daemon/config.toml
```

### Check Status

```bash
memory-daemon status
```

Output:
```
Memory daemon is running (PID: 12345)
```

### Stop Daemon

```bash
memory-daemon stop
```

## Running as a Service

### systemd (Linux)

Create `/etc/systemd/system/memory-daemon.service`:

```ini
[Unit]
Description=Agent Memory Daemon
After=network.target

[Service]
Type=simple
User=memory
ExecStart=/usr/local/bin/memory-daemon start
ExecStop=/usr/local/bin/memory-daemon stop
Restart=on-failure
RestartSec=5

Environment=MEMORY_PORT=50051
Environment=MEMORY_DB_PATH=/var/lib/memory-store
Environment=MEMORY_LOG_LEVEL=info

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable memory-daemon
sudo systemctl start memory-daemon
sudo systemctl status memory-daemon
```

### launchd (macOS)

Create `~/Library/LaunchAgents/com.spillwave.memory-daemon.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.spillwave.memory-daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/memory-daemon</string>
        <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/memory-daemon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/memory-daemon.stderr.log</string>
</dict>
</plist>
```

Load and start:
```bash
launchctl load ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist
launchctl start com.spillwave.memory-daemon
```

## Query Commands

All query commands connect to a running daemon.

### Get TOC Root

Returns year-level TOC nodes:

```bash
memory-daemon query --endpoint http://[::1]:50051 root
```

Output:
```
Year nodes:
  toc:year:2026 - 2026 (156 events)
  toc:year:2025 - 2025 (1,234 events)
```

### Get Specific Node

```bash
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"
```

Output:
```
Node: toc:year:2026
  Title: 2026
  Level: Year
  Summary: Conversations about Rust development, agent memory system
  Keywords: rust, memory, grpc, rocksdb
  Children: 1 (toc:month:2026-01)
```

### Browse Children with Pagination

```bash
memory-daemon query --endpoint http://[::1]:50051 browse \
  --parent-id "toc:year:2026" \
  --limit 10 \
  --token <continuation_token>
```

### Get Events in Time Range

Timestamps are milliseconds since Unix epoch:

```bash
# Get events from last hour
NOW=$(date +%s)000
HOUR_AGO=$(( $(date +%s) - 3600 ))000

memory-daemon query --endpoint http://[::1]:50051 events \
  --from $HOUR_AGO \
  --to $NOW \
  --limit 100
```

Output:
```
Events (5 of 5):
  [2026-01-30 10:00:00] USER: What is Rust?
  [2026-01-30 10:00:05] ASSISTANT: Rust is a systems programming language...
  [2026-01-30 10:00:10] USER: How does it handle memory?
  ...
```

### Expand Grip Context

```bash
memory-daemon query --endpoint http://[::1]:50051 expand \
  --grip-id "grip:1706600000000:01HXYZ123" \
  --before 3 \
  --after 3
```

Output:
```
Grip: grip:1706600000000:01HXYZ123
  Excerpt: "User asked about authentication"

Events Before:
  [10:00:00] USER: Let's discuss security
  [10:00:05] ASSISTANT: What aspect?
  [10:00:10] USER: Authentication methods

Excerpt Events:
  [10:00:15] USER: How do we authenticate users?
  [10:00:20] ASSISTANT: We can use JWT tokens...

Events After:
  [10:00:25] USER: What about refresh tokens?
  [10:00:30] ASSISTANT: Refresh tokens allow...
  [10:00:35] USER: Perfect, let's implement that
```

## Admin Commands

Admin commands access storage directly (no daemon required).

### Storage Statistics

```bash
memory-daemon admin --db-path ~/.memory-store stats
```

Output:
```
Storage Statistics:
  Events:      1,234
  TOC Nodes:   56
  Grips:       789
  Outbox:      0
  Disk Usage:  12.5 MB
```

### Compact Database

Full compaction:
```bash
memory-daemon admin --db-path ~/.memory-store compact
```

Specific column family:
```bash
memory-daemon admin --db-path ~/.memory-store compact --cf events
```

Available column families:
- `events` - Raw conversation events
- `toc_nodes` - TOC node versions
- `toc_latest` - Latest version pointers
- `grips` - Excerpt provenance
- `outbox` - Pending TOC updates
- `checkpoints` - Job recovery state

### Rebuild TOC

```bash
memory-daemon admin --db-path ~/.memory-store rebuild-toc \
  --from-date 2026-01-01 \
  --dry-run
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MEMORY_PORT` | gRPC server port | 50051 |
| `MEMORY_DB_PATH` | RocksDB directory | ~/.memory-store |
| `MEMORY_LOG_LEVEL` | Logging level | info |
| `MEMORY_CONFIG` | Config file path | ~/.config/memory-daemon/config.toml |

## Log Levels

Set via `MEMORY_LOG_LEVEL` or `--log-level`:

| Level | Description |
|-------|-------------|
| `error` | Only errors |
| `warn` | Errors and warnings |
| `info` | Normal operation (default) |
| `debug` | Detailed operation |
| `trace` | Very verbose |

Example:
```bash
MEMORY_LOG_LEVEL=debug memory-daemon start
```

## Troubleshooting

### Daemon Won't Start

1. Check if already running:
   ```bash
   memory-daemon status
   ```

2. Check port availability:
   ```bash
   lsof -i :50051
   ```

3. Check logs:
   ```bash
   MEMORY_LOG_LEVEL=debug memory-daemon start 2>&1 | head -50
   ```

### Connection Refused

1. Verify daemon is running:
   ```bash
   memory-daemon status
   ```

2. Check endpoint format (include `http://`):
   ```bash
   # Correct
   memory-daemon query --endpoint http://[::1]:50051 root

   # Wrong
   memory-daemon query --endpoint [::1]:50051 root
   ```

### Database Locked

1. Stop any running daemon:
   ```bash
   memory-daemon stop
   ```

2. Check for stale PID files:
   ```bash
   rm -f ~/.local/state/memory-daemon/memory-daemon.pid
   ```

### Out of Disk Space

1. Check current usage:
   ```bash
   memory-daemon admin --db-path ~/.memory-store stats
   ```

2. Run compaction:
   ```bash
   memory-daemon admin --db-path ~/.memory-store compact
   ```
