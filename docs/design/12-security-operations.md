# Security & Operations Guide

This document provides comprehensive security and operational guidance for the agent-memory system. It covers the security model, data privacy, operational procedures, disaster recovery, performance tuning, monitoring, troubleshooting, and upgrade procedures.

---

## 1. Security Model

### 1.1 Local-Only Design

The agent-memory daemon is designed exclusively for local operation. It is **not intended for network exposure** and should never be deployed as a network-accessible service.

**Key Security Characteristics:**

| Aspect | Implementation |
|--------|----------------|
| Network Binding | Binds to `0.0.0.0:50051` by default (configurable) |
| Protocol | gRPC only (no HTTP/REST) |
| Authentication | None (trusted local user model) |
| Encryption | None (local loopback traffic) |
| Access Control | OS-level file permissions only |

> **WARNING**: Do not expose the gRPC port to external networks. The daemon has no authentication or encryption and is designed for single-user local access only.

**Recommended Network Configuration:**

```toml
# ~/.config/agent-memory/config.toml
# Bind only to localhost for additional safety
grpc_host = "127.0.0.1"
grpc_port = 50051
```

### 1.2 No Authentication (Trusted Local User)

The system operates under a trusted local user model:

- **Single User**: Designed for one user per installation
- **OS-Level Security**: Relies on operating system user permissions
- **No Token/Password**: No authentication tokens or passwords required
- **Process Isolation**: Each project can have its own RocksDB instance

**Why No Authentication:**

1. Local daemon accessed only by local processes (Claude Code hooks, CLI tools)
2. Authentication would add complexity without security benefit for local-only use
3. OS file permissions protect the database directory

### 1.3 Data Isolation Per Project

Each project maintains isolated storage through separate RocksDB instances:

```
~/.local/share/agent-memory/
  db/                      # Default project database
  projects/
    project-alpha/         # Project-specific database
    project-beta/          # Another project database
```

**Isolation Guarantees:**

- Separate column families per instance
- No cross-project data access
- Independent checkpoint and recovery state

**Multi-Agent Mode Configuration:**

```toml
# Separate stores (default) - maximum isolation
multi_agent_mode = "separate"

# Unified store with tags - shared database, tagged isolation
multi_agent_mode = "unified"
agent_id = "claude-code"
```

### 1.4 No Secrets in Storage

The agent-memory system is designed to store conversation history, not secrets.

> **CRITICAL**: Never store API keys, passwords, tokens, or other credentials in conversation events. The storage is not encrypted.

**What Gets Stored:**

- User prompts and messages
- Assistant responses
- Tool invocation results
- Session metadata (timestamps, event IDs)
- TOC summaries and keywords
- Grips (excerpts with event references)

**What Should NOT Be Stored:**

- API keys or tokens
- Passwords or credentials
- Private keys or certificates
- Personally identifiable information (PII) if avoidable
- Financial or health data

---

## 2. Data Privacy

### 2.1 What Data Is Stored

The system stores conversation data across six column families:

| Column Family | Contents | Purpose |
|---------------|----------|---------|
| `events` | Raw conversation events | User messages, assistant responses, tool results |
| `toc_nodes` | TOC hierarchy nodes | Year/Month/Week/Day/Segment summaries |
| `toc_latest` | Version pointers | Latest version of each TOC node |
| `grips` | Evidence excerpts | Links between summaries and source events |
| `outbox` | Async queue | Pending index updates |
| `checkpoints` | Recovery state | Background job progress |

**Event Data Structure:**

```
Event {
  event_id: ULID (timestamp + random)
  event_type: session_start | user_message | tool_result | assistant_stop | session_end
  timestamp: Unix milliseconds
  text: Message content
  role: user | assistant | system | tool
  agent_id: Source agent identifier
  session_id: Conversation session identifier
}
```

### 2.2 Where Data Is Stored

**Default Locations by Platform:**

| Platform | Data Directory |
|----------|---------------|
| macOS | `~/Library/Application Support/agent-memory/db/` |
| Linux | `~/.local/share/agent-memory/db/` |
| Windows | `%APPDATA%\agent-memory\db\` |

**Configuration Location:**

| Platform | Config Directory |
|----------|-----------------|
| macOS | `~/.config/agent-memory/config.toml` |
| Linux | `~/.config/agent-memory/config.toml` |
| Windows | `%APPDATA%\agent-memory\config.toml` |

**PID File Location:**

| Platform | PID File |
|----------|----------|
| macOS/Linux | `$XDG_RUNTIME_DIR/agent-memory/daemon.pid` or `~/.cache/agent-memory/daemon.pid` |
| Windows | `%TEMP%\agent-memory\daemon.pid` |

### 2.3 Data Retention (Append-Only)

The agent-memory system uses an **append-only** storage model:

- **No Deletions**: Events are never deleted once stored
- **No Mutations**: Events are immutable after creation
- **Versioned TOC**: TOC nodes append new versions, preserving history
- **Growth Management**: Use compaction to reclaim space from obsolete SST files

> **NOTE**: If you need to delete sensitive data, you must delete the entire RocksDB directory. There is no selective deletion API.

**Implications:**

1. Storage grows over time (plan for disk capacity)
2. Sensitive data persists indefinitely once stored
3. Compliance requirements may necessitate periodic database purges

### 2.4 No External Transmission

The daemon does not transmit data externally with one exception:

| Component | External Communication |
|-----------|----------------------|
| gRPC Service | Local only (loopback) |
| Storage | Local filesystem only |
| Scheduler | Local operations only |
| **Summarizer** | **API calls to LLM providers** |

> **WARNING**: The summarizer component can transmit conversation data to external LLM APIs (OpenAI, Anthropic) for summary generation. Review summarizer configuration carefully.

**Summarizer Privacy Configuration:**

```toml
[summarizer]
# Use local summarizer to prevent external transmission
provider = "local"

# Or use API summarizer (data sent to provider)
provider = "openai"
model = "gpt-4o-mini"
# API key loaded from environment, not config file
```

---

## 3. Operational Procedures

### 3.1 Starting/Stopping Daemon

**Starting the Daemon:**

```bash
# Start in foreground (recommended for debugging)
memory-daemon start --foreground

# Start with custom port
memory-daemon start --foreground --port 50052

# Start with custom database path
memory-daemon start --foreground --db-path /path/to/db

# Start with custom config file
memory-daemon --config /path/to/config.toml start --foreground

# Start with debug logging
memory-daemon --log-level debug start --foreground
```

**Stopping the Daemon:**

```bash
# Graceful shutdown via PID file
memory-daemon stop

# Force kill if stop fails (last resort)
kill -9 $(cat ~/.cache/agent-memory/daemon.pid)
```

**Checking Status:**

```bash
# Check if daemon is running
memory-daemon status

# Expected output when running:
# Memory daemon is running (PID 12345)
# PID file: /path/to/daemon.pid

# Expected output when not running:
# Memory daemon is NOT running (no PID file)
```

### 3.2 Health Monitoring

**gRPC Health Check:**

The daemon exposes a standard gRPC health check endpoint:

```bash
# Using grpcurl
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check

# Expected response:
# {"status": "SERVING"}
```

**Scheduler Status:**

```bash
# View all scheduled jobs
memory-daemon scheduler status

# Example output:
# Scheduler: RUNNING
#
# JOB                  STATUS       LAST RUN             NEXT RUN             RUNS       ERRORS
# toc_rollup_day       IDLE         2026-01-31 01:00     2026-02-01 01:00     45         0
# toc_rollup_week      IDLE         2026-01-26 02:00     2026-02-02 02:00     6          0
# toc_rollup_month     IDLE         2026-01-01 03:00     2026-02-01 03:00     1          0
# rocksdb_compaction   IDLE         2026-01-26 04:00     2026-02-02 04:00     4          0
```

**Database Statistics:**

```bash
# View storage statistics
memory-daemon admin stats

# Example output:
# Database Statistics
# ===================
# Path: /Users/alice/.local/share/agent-memory/db
#
# Events:             12543
# TOC Nodes:            234
# Grips:                892
# Outbox:                 0
#
# Disk Usage:        45.23 MB
```

### 3.3 Log Management

**Log Configuration:**

```toml
# ~/.config/agent-memory/config.toml
log_level = "info"  # trace, debug, info, warn, error
```

**Environment Variable Override:**

```bash
# Set log level via environment
MEMORY_LOG_LEVEL=debug memory-daemon start --foreground

# Or use RUST_LOG for more control
RUST_LOG=memory_daemon=debug,memory_storage=info memory-daemon start --foreground
```

**Log Output:**

Logs are written to stderr by default. For production use, redirect to a file:

```bash
# Redirect logs to file
memory-daemon start --foreground 2>&1 | tee -a /var/log/agent-memory/daemon.log

# Or use a process manager (systemd, launchd)
```

**Log Rotation:**

When running with file logging, implement log rotation:

```bash
# Example logrotate configuration (/etc/logrotate.d/agent-memory)
/var/log/agent-memory/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
}
```

### 3.4 Backup Procedures

**Full Database Backup:**

```bash
# Stop daemon before backup (recommended)
memory-daemon stop

# Copy entire database directory
cp -r ~/.local/share/agent-memory/db ~/backups/agent-memory-$(date +%Y%m%d)/

# Restart daemon
memory-daemon start --foreground
```

**Live Backup (Hot Copy):**

RocksDB supports live backups, but the agent-memory daemon does not expose this API directly. For live backups:

```bash
# Create a checkpoint (copy-on-write snapshot)
# This requires using RocksDB tools directly

# 1. Install ldb tool from RocksDB
# 2. Create checkpoint:
ldb --db=~/.local/share/agent-memory/db checkpoint --checkpoint_dir=/backup/checkpoint-$(date +%Y%m%d)
```

**Backup Verification:**

```bash
# Verify backup integrity by opening with memory-daemon
memory-daemon admin stats --db-path /backup/checkpoint-20260131/

# Should show valid statistics without errors
```

> **WARNING**: Always test backup restoration in a separate location before relying on backups for disaster recovery.

---

## 4. Disaster Recovery

### 4.1 Checkpoint-Based Recovery

The scheduler uses checkpoints to enable crash recovery for background jobs:

**Checkpoint Storage:**

- Stored in the `checkpoints` column family
- One checkpoint per job (e.g., `toc_rollup_day`, `toc_rollup_week`)
- Updated after each successful job run

**Recovery Process:**

When the daemon restarts after a crash:

1. Scheduler reads checkpoints from storage
2. Each job resumes from its last checkpoint
3. Partially completed work is redone (idempotent operations)

**Manual Checkpoint Reset:**

If a job is stuck, you can force a fresh start:

```bash
# Currently requires direct RocksDB access
# Future: memory-daemon admin reset-checkpoint --job toc_rollup_day
```

### 4.2 TOC Rebuild from Events

If the TOC hierarchy becomes corrupted, rebuild it from raw events:

**Dry Run (Preview):**

```bash
# See what would be rebuilt
memory-daemon admin rebuild-toc --dry-run

# Output:
# DRY RUN - No changes will be made
# Found 12543 events to process
# Would process events from 1704067200000 to 1706745600000
# First event timestamp: 1704067200123
# Last event timestamp: 1706745599876
```

**Full Rebuild:**

```bash
# Rebuild from specific date
memory-daemon admin rebuild-toc --from-date 2026-01-01

# Rebuild all (from beginning)
memory-daemon admin rebuild-toc
```

> **NOTE**: Full TOC rebuild requires running summarization on all events. This may incur API costs if using an external summarizer and can take significant time for large datasets.

### 4.3 Index Rebuild Procedures

Indexes (BM25, vector) are designed to be rebuilt from the outbox:

**Current Status (v1.0):**

Indexes are not yet implemented. When available:

```bash
# Future commands:
# memory-daemon admin rebuild-index --type bm25
# memory-daemon admin rebuild-index --type vector
# memory-daemon admin rebuild-index --all
```

**Outbox-Driven Rebuild:**

Indexes subscribe to the outbox queue. To force a rebuild:

1. Clear the index
2. Replay all events through the outbox
3. Index processes each event

---

## 5. Performance Tuning

### 5.1 RocksDB Configuration

The daemon uses RocksDB with these default settings:

**Global Options:**

```rust
// Universal compaction for append-only workload
db_opts.set_compaction_style(DBCompactionStyle::Universal);

// Limit background jobs to prevent resource exhaustion
db_opts.set_max_background_jobs(4);
```

**Column Family Tuning:**

| Column Family | Compaction | Compression | Notes |
|---------------|------------|-------------|-------|
| events | Universal | Zstd | Append-only, space-efficient |
| toc_nodes | Level | None | Small, frequent reads |
| toc_latest | Level | None | Small lookup table |
| grips | Level | None | Medium size, indexed |
| outbox | FIFO | None | Queue, auto-expires |
| checkpoints | Level | None | Tiny, infrequent |

**Advanced Tuning:**

For high-volume deployments, consider these RocksDB options (requires code changes):

```rust
// Increase write buffer for high ingestion rates
cf_opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB

// More write buffers before flush
cf_opts.set_max_write_buffer_number(4);

// Increase block cache for read-heavy workloads
let mut block_opts = BlockBasedOptions::default();
block_opts.set_block_cache(&Cache::new_lru_cache(128 * 1024 * 1024)); // 128MB
cf_opts.set_block_based_table_factory(&block_opts);
```

### 5.2 Memory Budgets

**Current Defaults:**

| Component | Memory Usage |
|-----------|--------------|
| RocksDB Block Cache | System default (~8MB) |
| Write Buffers | ~64MB per CF |
| gRPC Buffers | ~4MB |
| Scheduler | Minimal (<1MB) |

**Monitoring Memory:**

```bash
# Check process memory
ps aux | grep memory-daemon

# Detailed memory map (macOS)
vmmap $(pgrep memory-daemon) | head -50

# Detailed memory map (Linux)
pmap $(pgrep memory-daemon)
```

**Reducing Memory Usage:**

For resource-constrained environments:

```toml
# Use smaller database path on ramdisk
db_path = "/tmp/agent-memory/db"
```

### 5.3 Compaction Scheduling

**Default Schedule:**

| Job | Schedule | Jitter |
|-----|----------|--------|
| Day Rollup | 1:00 AM daily | 0-5 min |
| Week Rollup | 2:00 AM Sunday | 0-5 min |
| Month Rollup | 3:00 AM 1st of month | 0-5 min |
| RocksDB Compaction | 4:00 AM Sunday | 0-10 min |

**Manual Compaction:**

```bash
# Compact all column families
memory-daemon admin compact

# Compact specific column family
memory-daemon admin compact --cf events
memory-daemon admin compact --cf toc_nodes
```

**Pause During High Activity:**

```bash
# Pause compaction during peak hours
memory-daemon scheduler pause rocksdb_compaction

# Resume later
memory-daemon scheduler resume rocksdb_compaction
```

---

## 6. Monitoring

### 6.1 Health Check Endpoints

**gRPC Health Service:**

```protobuf
// Standard gRPC health check protocol
service Health {
  rpc Check(HealthCheckRequest) returns (HealthCheckResponse);
  rpc Watch(HealthCheckRequest) returns (stream HealthCheckResponse);
}
```

**Check with grpcurl:**

```bash
# One-time check
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check

# Watch for changes
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Watch
```

**Check with grpc-health-probe:**

```bash
# Install
go install github.com/grpc-ecosystem/grpc-health-probe@latest

# Check
grpc-health-probe -addr=localhost:50051
```

### 6.2 Scheduler Status

**CLI Status:**

```bash
memory-daemon scheduler status
```

**gRPC Status (Programmatic):**

```bash
# Using grpcurl
grpcurl -plaintext localhost:50051 memory.MemoryService/GetSchedulerStatus
```

**Status Fields:**

| Field | Description |
|-------|-------------|
| scheduler_running | Overall scheduler state |
| job_name | Job identifier |
| is_paused | Whether job is paused |
| is_running | Whether job is currently executing |
| last_run_ms | Timestamp of last run |
| next_run_ms | Timestamp of next scheduled run |
| run_count | Total successful runs |
| error_count | Total failed runs |
| last_result | SUCCESS, FAILED, or SKIPPED |
| last_error | Error message if last run failed |

### 6.3 Storage Statistics

**CLI Statistics:**

```bash
memory-daemon admin stats
```

**Key Metrics:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| event_count | Total events stored | Growth rate monitoring |
| toc_node_count | TOC hierarchy size | Should grow with events |
| grip_count | Evidence links | Proportional to TOC |
| outbox_count | Pending queue items | Should stay near 0 |
| disk_usage_bytes | Total storage size | Capacity planning |

---

## 7. Troubleshooting Guide

### 7.1 Common Failure Modes

**Daemon Won't Start:**

| Symptom | Cause | Solution |
|---------|-------|----------|
| "Address already in use" | Port 50051 in use | Stop other process or use `--port` |
| "Failed to open storage" | Corrupted database | Restore from backup |
| "Permission denied" | File permission issue | Check db_path ownership |
| Stale PID file | Previous crash | Remove PID file manually |

```bash
# Check if port is in use
lsof -i :50051

# Remove stale PID file
rm ~/.cache/agent-memory/daemon.pid

# Check file permissions
ls -la ~/.local/share/agent-memory/db/
```

**Connection Refused:**

```bash
# Verify daemon is running
memory-daemon status

# Check if listening on correct port
netstat -an | grep 50051  # Linux
lsof -i :50051            # macOS

# Test gRPC connectivity
grpcurl -plaintext localhost:50051 list
```

**High Memory Usage:**

```bash
# Check RocksDB memory
memory-daemon admin stats

# If outbox is large, indexes may be backed up
# Compact to reduce memory
memory-daemon admin compact
```

**Slow Queries:**

```bash
# Check database size
memory-daemon admin stats

# If disk usage is high, run compaction
memory-daemon admin compact

# Check for pending outbox items
# High outbox count indicates indexing backlog
```

### 7.2 Diagnostic Commands

**Full System Check:**

```bash
#!/bin/bash
# Comprehensive health check script

echo "=== Daemon Status ==="
memory-daemon status

echo -e "\n=== gRPC Health ==="
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check 2>/dev/null || echo "gRPC unavailable"

echo -e "\n=== Scheduler Status ==="
memory-daemon scheduler status 2>/dev/null || echo "Scheduler unavailable"

echo -e "\n=== Database Stats ==="
memory-daemon admin stats 2>/dev/null || echo "Database unavailable"

echo -e "\n=== Disk Space ==="
df -h ~/.local/share/agent-memory/

echo -e "\n=== Process Info ==="
ps aux | grep memory-daemon | grep -v grep
```

**Debug Logging:**

```bash
# Start with trace logging for maximum detail
RUST_LOG=trace memory-daemon start --foreground 2>&1 | tee debug.log

# Search for errors
grep -i error debug.log
grep -i warn debug.log
```

### 7.3 Recovery Procedures

**Corrupt Database Recovery:**

```bash
# 1. Stop daemon
memory-daemon stop

# 2. Backup corrupt database (for analysis)
mv ~/.local/share/agent-memory/db ~/.local/share/agent-memory/db.corrupt

# 3. Restore from backup
cp -r ~/backups/agent-memory-20260130/ ~/.local/share/agent-memory/db

# 4. Start daemon
memory-daemon start --foreground

# 5. Verify
memory-daemon admin stats
```

**Stuck Job Recovery:**

```bash
# 1. Pause the stuck job
memory-daemon scheduler pause toc_rollup_day

# 2. Check job status
memory-daemon scheduler status

# 3. Restart daemon to reset job state
memory-daemon stop
memory-daemon start --foreground

# 4. Resume job
memory-daemon scheduler resume toc_rollup_day
```

**PID File Issues:**

```bash
# If daemon shows "running" but isn't responding

# 1. Check actual process
ps aux | grep memory-daemon

# 2. If no process, remove stale PID
rm ~/.cache/agent-memory/daemon.pid

# 3. If process exists but hung, kill it
kill -9 $(cat ~/.cache/agent-memory/daemon.pid)
rm ~/.cache/agent-memory/daemon.pid

# 4. Start fresh
memory-daemon start --foreground
```

---

## 8. Upgrade Procedures

### 8.1 Binary Updates

**Standard Upgrade:**

```bash
# 1. Stop daemon
memory-daemon stop

# 2. Backup database (recommended)
cp -r ~/.local/share/agent-memory/db ~/backups/agent-memory-pre-upgrade/

# 3. Install new binary
cargo install memory-daemon --force
# or replace binary from release package

# 4. Verify version
memory-daemon --version

# 5. Start daemon
memory-daemon start --foreground

# 6. Verify health
memory-daemon admin stats
memory-daemon scheduler status
```

**Zero-Downtime Upgrade (Future):**

Currently not supported. The daemon must be stopped for upgrades.

### 8.2 Data Migration (If Needed)

**Schema Changes:**

Agent-memory uses RocksDB with forward-compatible serialization:

- Prost/protobuf for events and RPC messages
- JSON for TOC nodes and grips (serde)

Most schema changes are backward compatible. When migration is required:

```bash
# Future migration command
# memory-daemon admin migrate --from v1.0 --to v2.0

# Current approach: rebuild TOC after major version changes
memory-daemon admin rebuild-toc
```

**Database Format Changes:**

If RocksDB format changes:

```bash
# 1. Export data (future feature)
# memory-daemon admin export --output events.json

# 2. Create fresh database
rm -rf ~/.local/share/agent-memory/db

# 3. Start with new version
memory-daemon start --foreground

# 4. Import data (future feature)
# memory-daemon admin import --input events.json
```

### 8.3 Rollback Procedures

**Quick Rollback:**

```bash
# 1. Stop new version
memory-daemon stop

# 2. Restore old binary
# (from backup or package manager)

# 3. Restore database if needed
rm -rf ~/.local/share/agent-memory/db
cp -r ~/backups/agent-memory-pre-upgrade/ ~/.local/share/agent-memory/db

# 4. Start old version
memory-daemon start --foreground
```

**Version Compatibility Matrix:**

| From Version | To Version | Migration Required | Backward Compatible |
|--------------|------------|-------------------|---------------------|
| v1.0.x | v1.0.y | No | Yes |
| v1.0.x | v1.1.x | TBD | TBD |
| v1.x | v2.x | Likely | TBD |

---

## Appendix A: Configuration Reference

**Complete Config File:**

```toml
# ~/.config/agent-memory/config.toml

# Storage
db_path = "~/.local/share/agent-memory/db"

# gRPC Server
grpc_host = "127.0.0.1"  # Use "0.0.0.0" for all interfaces (not recommended)
grpc_port = 50051

# Multi-agent mode
multi_agent_mode = "separate"  # or "unified"
agent_id = "claude-code"       # Required for unified mode

# Logging
log_level = "info"  # trace, debug, info, warn, error

# Summarizer
[summarizer]
provider = "openai"       # or "anthropic", "local"
model = "gpt-4o-mini"     # Model name
# api_key loaded from OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable
```

**Environment Variables:**

| Variable | Description |
|----------|-------------|
| MEMORY_DB_PATH | Override database path |
| MEMORY_GRPC_PORT | Override gRPC port |
| MEMORY_GRPC_HOST | Override gRPC host |
| MEMORY_LOG_LEVEL | Override log level |
| MEMORY_SUMMARIZER_PROVIDER | Override summarizer provider |
| MEMORY_SUMMARIZER_MODEL | Override summarizer model |
| OPENAI_API_KEY | OpenAI API key for summarizer |
| ANTHROPIC_API_KEY | Anthropic API key for summarizer |

---

## Appendix B: Command Reference

**Daemon Commands:**

```bash
memory-daemon start [--foreground] [--port PORT] [--db-path PATH]
memory-daemon stop
memory-daemon status
```

**Query Commands:**

```bash
memory-daemon query root
memory-daemon query node NODE_ID
memory-daemon query browse PARENT_ID [--limit N] [--token TOKEN]
memory-daemon query events --from MS --to MS [--limit N]
memory-daemon query expand GRIP_ID [--before N] [--after N]
```

**Admin Commands:**

```bash
memory-daemon admin stats [--db-path PATH]
memory-daemon admin compact [--cf CF_NAME]
memory-daemon admin rebuild-toc [--from-date YYYY-MM-DD] [--dry-run]
```

**Scheduler Commands:**

```bash
memory-daemon scheduler status
memory-daemon scheduler pause JOB_NAME
memory-daemon scheduler resume JOB_NAME
```

---

*Last updated: 2026-01-31*
