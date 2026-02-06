# Retention Policies

Data retention policies control how long agent-memory stores conversation data before cleanup.

## Overview

Retention policies balance storage costs against historical context availability. Choose based on your usage patterns and compliance requirements.

## Policy Options

| Policy | Config Value | Storage Impact | Use Case |
|--------|--------------|----------------|----------|
| Forever | `forever` | Grows unbounded | Maximum historical context, research, long-term memory |
| 90 Days | `days:90` | ~3 months data | Balance of history and storage, typical professional use |
| 30 Days | `days:30` | ~1 month data | Lower storage needs, recent context sufficient |
| 7 Days | `days:7` | ~1 week data | Short-term memory only, constrained storage |

## Cleanup Schedule

Cleanup runs according to a cron schedule. Data older than the retention period is removed.

| Schedule | Cron Expression | Description |
|----------|-----------------|-------------|
| Daily at 3 AM | `0 3 * * *` | Recommended - runs during off-hours |
| Weekly on Sunday | `0 3 * * 0` | Lower system impact, less frequent |
| Daily at midnight | `0 0 * * *` | Alternative off-hours schedule |
| Every 6 hours | `0 */6 * * *` | Aggressive cleanup for constrained storage |
| Disabled | (empty) | Manual cleanup only |

### Cron Format

```
┌───────────── minute (0-59)
│ ┌───────────── hour (0-23)
│ │ ┌───────────── day of month (1-31)
│ │ │ ┌───────────── month (1-12)
│ │ │ │ ┌───────────── day of week (0-6, 0=Sunday)
│ │ │ │ │
* * * * *
```

**Examples:**
- `0 3 * * *` - Daily at 3:00 AM
- `0 3 * * 0` - Every Sunday at 3:00 AM
- `30 2 1 * *` - First day of each month at 2:30 AM
- `0 */4 * * *` - Every 4 hours

## Data Lifecycle

```
Event Ingested
      |
      v
+------------------+
| Active Storage   | <- Immediately queryable
| ~/.memory-store  |
+--------+---------+
         |
         | (retention period elapsed)
         v
+------------------+
| Archive Decision | <- Based on archive_strategy
+--------+---------+
    |         |
    |         v
    |   +------------------+
    |   | Archive Storage  | <- If compress or json
    |   | ~/.memory-archive|
    |   +------------------+
    v
+------------------+
| Deleted          | <- Removed from active storage
+------------------+
```

## Storage Estimation

Estimate storage requirements based on usage patterns:

| Usage Level | Events/Day | Average Event Size | Daily Growth | Monthly Growth |
|-------------|------------|-------------------|--------------|----------------|
| Light | 100 | ~10KB | ~1MB | ~30MB |
| Medium | 500 | ~10KB | ~5MB | ~150MB |
| Heavy | 2,000 | ~10KB | ~20MB | ~600MB |
| Team | 10,000 | ~10KB | ~100MB | ~3GB |

### Formula

```
Storage = events_per_day * avg_event_size_kb * retention_days / 1024 MB

Example (Medium usage, 90-day retention):
Storage = 500 * 10 * 90 / 1024 = 439 MB
```

## Configuration Example

```toml
[retention]
# Keep data for 90 days
policy = "days:90"

# Run cleanup daily at 3 AM
cleanup_schedule = "0 3 * * *"

# Compress old data before deletion
archive_strategy = "compress"
archive_path = "~/.memory-archive"
```

## Policy Recommendations

| Scenario | Recommended Policy | Reason |
|----------|-------------------|--------|
| Personal development | Forever | Valuable long-term context |
| Professional use | 90 days | Balance of history and storage |
| Limited storage (< 10GB free) | 30 days | Prevent storage exhaustion |
| Compliance requirements | Based on policy | Match organizational requirements |
| Shared/team machine | 30 days | Fair resource usage |
| Privacy-focused | 7 days | Minimize data retention |

## Manual Cleanup

For manual cleanup outside the schedule:

```bash
# Run cleanup now
memory-daemon admin cleanup

# Cleanup with specific retention
memory-daemon admin cleanup --older-than 30d

# Dry run (show what would be deleted)
memory-daemon admin cleanup --dry-run
```

## Monitoring

Check retention status:

```bash
# View oldest and newest events
memory-daemon admin stats

# Check storage usage
du -sh ~/.memory-store

# View archive size
du -sh ~/.memory-archive
```
