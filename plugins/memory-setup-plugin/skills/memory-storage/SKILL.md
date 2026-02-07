---
name: memory-storage
description: |
  This skill should be used when the user asks to "configure storage",
  "set up retention policies", "configure GDPR mode", "tune memory performance",
  "change storage path", or "configure data cleanup". Provides interactive wizard
  for storage configuration with state detection.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Storage Skill

Configure storage paths, data retention policies, cleanup schedules, GDPR compliance mode, and performance tuning for agent-memory.

## When Not to Use

- Initial installation (use `/memory-setup` first)
- Querying past conversations (use memory-query plugin)
- Configuring LLM providers (use `/memory-llm`)
- Multi-agent configuration (use `/memory-agents`)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `/memory-storage` | Interactive storage wizard | `/memory-storage` |
| `/memory-storage --minimal` | Use defaults, minimal questions | `/memory-storage --minimal` |
| `/memory-storage --advanced` | Show all options including cron and performance | `/memory-storage --advanced` |
| `/memory-storage --fresh` | Re-configure all options from scratch | `/memory-storage --fresh` |

## Question Flow

```
State Detection
      |
      v
+------------------+
| Step 1: Storage  | <- Skip if path exists (unless --fresh)
| Path             |
+--------+---------+
         |
         v
+------------------+
| Step 2: Retention| <- Skip if policy configured
| Policy           |
+--------+---------+
         |
         v
+------------------+
| Step 3: Cleanup  | <- --advanced only
| Schedule         |
+--------+---------+
         |
         v
+------------------+
| Step 4: Archive  | <- --advanced only
| Strategy         |
+--------+---------+
         |
         v
+------------------+
| Step 5: GDPR     | <- Show if EU locale detected or --advanced
| Mode             |
+--------+---------+
         |
         v
+------------------+
| Step 6: Perf     | <- --advanced only
| Tuning           |
+--------+---------+
         |
         v
    Execution
```

## State Detection

Before beginning configuration, detect current system state to skip completed steps.

### Detection Commands

```bash
# Check if storage path is configured
grep -A5 '\[storage\]' ~/.config/memory-daemon/config.toml 2>/dev/null | grep path

# Check retention configuration
grep retention ~/.config/memory-daemon/config.toml 2>/dev/null

# Check current disk usage
du -sh ~/.memory-store 2>/dev/null && df -h ~/.memory-store 2>/dev/null | tail -1

# Check if archive exists
ls ~/.memory-archive 2>/dev/null

# Detect locale for GDPR
locale | grep -E "^LANG=.*_(AT|BE|BG|HR|CY|CZ|DK|EE|FI|FR|DE|GR|HU|IE|IT|LV|LT|LU|MT|NL|PL|PT|RO|SK|SI|ES|SE)" && echo "EU_LOCALE"
```

### State Summary Format

```
Current Storage State
---------------------
Storage Path:    ~/.memory-store (4.2 GB used, 120 GB available)
Retention:       Not configured
Cleanup:         Not configured
Archive:         Not configured
GDPR Mode:       Not configured
Performance:     Default settings

Recommended:     Configure retention policy
```

## Wizard Steps

### Step 1: Storage Path

**Skip if:** path configured in config.toml AND not `--fresh`

```
question: "Where should agent-memory store conversation data?"
header: "Storage"
options:
  - label: "~/.memory-store (Recommended)"
    description: "Standard user location, works on all platforms"
  - label: "~/.local/share/agent-memory/db"
    description: "XDG-compliant location for Linux"
  - label: "Custom path"
    description: "Specify a custom storage location"
multiSelect: false
```

**If Custom selected:**

```
question: "Enter the custom storage path:"
header: "Path"
type: text
validation: "Path must be writable with at least 100MB free space"
```

### Step 2: Retention Policy

**Skip if:** retention configured AND not `--fresh`

```
question: "How long should conversation data be retained?"
header: "Retention"
options:
  - label: "Forever (Recommended)"
    description: "Keep all data permanently for maximum historical context"
  - label: "90 days"
    description: "Quarter retention, good balance of history and storage"
  - label: "30 days"
    description: "One month retention, lower storage usage"
  - label: "7 days"
    description: "Short-term memory only, minimal storage"
multiSelect: false
```

### Step 3: Cleanup Schedule

**Skip if:** `--minimal` mode OR not `--advanced`

```
question: "When should automatic cleanup run?"
header: "Schedule"
options:
  - label: "Daily at 3 AM (Recommended)"
    description: "Runs during off-hours, catches expired data quickly"
  - label: "Weekly on Sunday"
    description: "Less frequent cleanup, lower system impact"
  - label: "Disabled"
    description: "Manual cleanup only with memory-daemon admin cleanup"
  - label: "Custom cron"
    description: "Specify a custom cron expression"
multiSelect: false
```

**If Custom cron selected:**

```
question: "Enter cron expression (e.g., '0 2 * * 0' for Sundays at 2 AM):"
header: "Cron"
type: text
validation: "Must be valid 5-field cron expression"
```

### Step 4: Archive Strategy

**Skip if:** `--minimal` mode OR not `--advanced`

```
question: "How should old data be archived before deletion?"
header: "Archive"
options:
  - label: "Compress to archive (Recommended)"
    description: "Saves space, data recoverable from ~/.memory-archive/"
  - label: "Export to JSON"
    description: "Human-readable backup before deletion"
  - label: "No archive"
    description: "Delete directly (irreversible)"
multiSelect: false
```

### Step 5: GDPR Mode

**Show if:** EU locale detected OR `--advanced` flag

```
question: "Enable GDPR-compliant deletion mode?"
header: "GDPR"
options:
  - label: "No (Recommended)"
    description: "Standard retention with tombstones for recovery"
  - label: "Yes"
    description: "Complete data removal, audit logging, export-before-delete"
multiSelect: false
```

### Step 6: Performance Tuning

**Skip if:** `--minimal` mode OR not `--advanced`

```
question: "Configure storage performance parameters?"
header: "Performance"
options:
  - label: "Balanced (Recommended)"
    description: "64MB write buffer, 4 background jobs - works for most users"
  - label: "Low memory"
    description: "16MB write buffer, 1 background job - for constrained systems"
  - label: "High performance"
    description: "128MB write buffer, 8 background jobs - for heavy usage"
  - label: "Custom"
    description: "Specify write_buffer_size_mb and max_background_jobs"
multiSelect: false
```

**If Custom selected:**

```
question: "Enter write buffer size in MB (16-256):"
header: "Buffer"
type: number
validation: "16 <= value <= 256"
```

```
question: "Enter max background jobs (1-16):"
header: "Jobs"
type: number
validation: "1 <= value <= 16"
```

## Config Generation

After wizard completion, generate or update config.toml:

```bash
# Create or update storage and retention sections
cat >> ~/.config/memory-daemon/config.toml << 'EOF'

[storage]
path = "~/.memory-store"
write_buffer_size_mb = 64
max_background_jobs = 4

[retention]
policy = "forever"
cleanup_schedule = "0 3 * * *"
archive_strategy = "compress"
archive_path = "~/.memory-archive"
gdpr_mode = false
EOF
```

### Config Value Mapping

| Wizard Choice | Config Value |
|---------------|--------------|
| Forever | `policy = "forever"` |
| 90 days | `policy = "days:90"` |
| 30 days | `policy = "days:30"` |
| 7 days | `policy = "days:7"` |
| Daily at 3 AM | `cleanup_schedule = "0 3 * * *"` |
| Weekly on Sunday | `cleanup_schedule = "0 3 * * 0"` |
| Disabled | `cleanup_schedule = ""` |
| Compress to archive | `archive_strategy = "compress"` |
| Export to JSON | `archive_strategy = "json"` |
| No archive | `archive_strategy = "none"` |
| Balanced | `write_buffer_size_mb = 64`, `max_background_jobs = 4` |
| Low memory | `write_buffer_size_mb = 16`, `max_background_jobs = 1` |
| High performance | `write_buffer_size_mb = 128`, `max_background_jobs = 8` |

## Validation

Before applying configuration, validate:

```bash
# 1. Path exists or can be created
mkdir -p "$STORAGE_PATH" 2>/dev/null && echo "[check] Path writable" || echo "[x] Cannot create path"

# 2. Write permissions verified
touch "$STORAGE_PATH/.test" 2>/dev/null && rm "$STORAGE_PATH/.test" && echo "[check] Write permission OK" || echo "[x] No write permission"

# 3. Minimum 100MB free disk space
FREE_KB=$(df -k "$STORAGE_PATH" 2>/dev/null | tail -1 | awk '{print $4}')
[ "$FREE_KB" -gt 102400 ] && echo "[check] Disk space OK ($(($FREE_KB/1024))MB free)" || echo "[x] Less than 100MB free"

# 4. Cron expression valid (if custom)
# Use online validator or test with: echo "$CRON_EXPR" | grep -E '^[0-9*,/-]+ [0-9*,/-]+ [0-9*,/-]+ [0-9*,/-]+ [0-9*,/-]+$'

# 5. Archive path writable (if archiving enabled)
mkdir -p "$ARCHIVE_PATH" 2>/dev/null && echo "[check] Archive path OK" || echo "[x] Cannot create archive path"
```

## Output Formatting

### Success Display

```
==================================================
 Storage Configuration Complete!
==================================================

[check] Storage path configured: ~/.memory-store
[check] Retention policy set: Forever
[check] Cleanup schedule: Daily at 3 AM
[check] Archive strategy: Compress to ~/.memory-archive/
[check] GDPR mode: Disabled
[check] Performance: Balanced (64MB buffer, 4 jobs)

Configuration written to ~/.config/memory-daemon/config.toml

Next steps:
  * Restart daemon: memory-daemon restart
  * Configure LLM: /memory-llm
  * Multi-agent setup: /memory-agents
```

### Partial Success Display

```
==================================================
 Storage Configuration Partially Complete
==================================================

[check] Storage path configured: ~/.memory-store
[check] Retention policy set: 30 days
[!] Cleanup schedule: Skipped (use --advanced to configure)
[!] Archive strategy: Using default (compress)
[check] GDPR mode: Disabled

What's missing:
  * Advanced options not configured (cleanup schedule, archive strategy)

To configure advanced options:
  /memory-storage --advanced
```

### Error Display

```
[x] Storage Configuration Failed
---------------------------------

Error: Cannot write to storage path /custom/path

To fix:
  1. Check directory permissions: ls -la /custom/path
  2. Create directory if needed: sudo mkdir -p /custom/path
  3. Set ownership: sudo chown $USER /custom/path
  4. Re-run: /memory-storage --fresh

Need help? Run: /memory-status --verbose
```

## Mode Behaviors

### Default Mode (`/memory-storage`)

- Runs state detection
- Skips configured options
- Shows steps 1, 2, and 5 (if EU locale)
- Uses defaults for advanced options

### Minimal Mode (`/memory-storage --minimal`)

- Skips all advanced options (steps 3, 4, 6)
- Uses recommended defaults
- Fastest path to configuration

### Advanced Mode (`/memory-storage --advanced`)

- Shows ALL six steps
- Includes cleanup schedule, archive strategy, performance tuning
- Shows GDPR option regardless of locale

### Fresh Mode (`/memory-storage --fresh`)

- Ignores existing configuration
- Asks all questions from scratch
- Can combine with --advanced: `/memory-storage --fresh --advanced`

## Reference Files

For detailed information, see:

- [Retention Policies](references/retention-policies.md) - Policy options and storage impact
- [GDPR Compliance](references/gdpr-compliance.md) - GDPR mode details and trade-offs
- [Archive Strategies](references/archive-strategies.md) - Archive options and recovery

## Related Skills

After storage configuration, consider:

- `/memory-llm` - Configure LLM provider for summarization
- `/memory-agents` - Set up multi-agent configuration
- `/memory-setup` - Full installation wizard
- `/memory-status` - Check current system status
