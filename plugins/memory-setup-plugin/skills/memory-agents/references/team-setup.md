# Team Setup

Configure agent-memory for team use with shared storage and collaborative memory.

## Overview

Team mode enables multiple users to share conversation memory, enabling:

- Cross-user knowledge discovery
- Collaborative context building
- Team-wide memory search
- Shared learning from conversations

## Use Cases

| Scenario | Configuration | Benefit |
|----------|---------------|---------|
| Development team | Read-only sharing | Learn from teammates' discoveries |
| Pair programming | Full sharing | Shared context between partners |
| Enterprise | Custom permissions | Fine-grained access control |
| Open source project | Read-only sharing | Community knowledge base |

## Permission Models

| Mode | Read Access | Write Access | Use Case |
|------|-------------|--------------|----------|
| Read-only | All team | Own agent only | Default team visibility |
| Full sharing | All team | All team | Collaborative work |
| Custom | Configurable | Configurable | Enterprise needs |

### Read-Only Sharing (Recommended)

Each team member can:
- See all team members' events
- Write only to their own agent ID
- Search across all team data
- Cannot modify others' events

```toml
[agents]
mode = "team"
agent_id = "alice-claude"

[team]
name = "engineering"
storage_path = "/shared/memory-store/"
shared = false  # read-only
```

### Full Sharing

All team members can:
- See all team members' events
- Write to shared store without agent ID tagging
- Search across all team data
- Events are attributed to writer

```toml
[agents]
mode = "team"
agent_id = "alice-claude"

[team]
name = "engineering"
storage_path = "/shared/memory-store/"
shared = true  # full sharing
```

### Custom Permissions

Fine-grained control over who can see/write what:

```toml
[agents]
mode = "team"
agent_id = "alice-claude"

[team]
name = "engineering"
storage_path = "/shared/memory-store/"
permissions = "custom"

[team.read_access]
agents = ["alice-claude", "bob-cursor", "charlie-copilot"]

[team.write_access]
agents = ["alice-claude", "bob-cursor"]
```

## Setup Steps

### 1. Choose Shared Storage

Select a location accessible to all team members:

| Storage Type | Path Example | Pros | Cons |
|--------------|--------------|------|------|
| NFS mount | `/nfs/team-memory/` | Simple | Network dependency |
| Cloud sync | `~/Dropbox/team-memory/` | Accessible anywhere | Sync conflicts |
| Local server | `ssh://server/memory/` | Controlled | Requires connectivity |

### 2. Configure Each Team Member

Each team member runs:

```bash
/memory-agents --team
```

And provides:
- Their unique agent ID (e.g., `alice-claude`)
- Team name (same for all members)
- Shared storage path (same for all members)

### 3. Verify Team Access

```bash
# List team members
memory-daemon admin list-agents

# Search team data
memory-daemon query "recent discussions"

# Check your agent ID
grep agent_id ~/.config/memory-daemon/config.toml
```

## Network Considerations

### NFS/Network Storage

For network-mounted storage:

```toml
[team]
storage_path = "/mnt/nfs/team-memory/"
lock_strategy = "flock"  # Use file locking
retry_on_lock = true     # Retry if locked
lock_timeout_secs = 30   # Timeout for locks
```

### Cloud Sync (Dropbox, OneDrive)

For cloud-synced storage:

```toml
[team]
storage_path = "~/Dropbox/team-memory/"
sync_safe = true         # Wait for sync before write
conflict_strategy = "timestamp"  # Use latest by timestamp
```

### Remote Server

For SSH-accessible storage:

```bash
# Mount remote storage locally
sshfs user@server:/memory /mnt/remote-memory

# Configure path
[team]
storage_path = "/mnt/remote-memory/"
```

## Security Considerations

### Access Control

1. Set appropriate file permissions on shared storage
2. Use team-specific storage paths
3. Consider encryption for sensitive data

### Agent ID Security

1. Use unique, identifiable agent IDs
2. Include username or employee ID
3. Don't share agent configurations

### Audit

Enable audit logging for compliance:

```toml
[team]
audit_log = "/var/log/memory-daemon/team-audit.log"
log_reads = true
log_writes = true
```

## Configuration Example

### Full Team Configuration

```toml
[agents]
mode = "team"
storage_strategy = "unified"
agent_id = "alice-claude"
query_scope = "all"

[team]
name = "engineering"
storage_path = "/shared/engineering-memory/"
shared = false
sync_safe = true

# Optional: audit logging
audit_log = "/var/log/memory-daemon/engineering-audit.log"
```

## Troubleshooting

### "Cannot access team storage"

```bash
# Check path exists and is writable
ls -la /shared/team-memory/
touch /shared/team-memory/.test && rm /shared/team-memory/.test
```

### "Agent ID conflict"

```bash
# List existing agents
memory-daemon admin list-agents

# Choose unique ID
/memory-agents --fresh
```

### "Slow queries"

```bash
# Check network storage performance
time ls /shared/team-memory/

# Consider local cache
[team]
local_cache = true
cache_path = "~/.memory-cache/"
```
