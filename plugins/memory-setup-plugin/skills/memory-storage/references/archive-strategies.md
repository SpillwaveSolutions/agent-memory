# Archive Strategies

Archive strategies control what happens to data when it exceeds the retention period.

## Overview

Before deleting old data, you can archive it for potential recovery or historical analysis.

## Strategy Options

| Strategy | Config Value | Description | Recovery |
|----------|--------------|-------------|----------|
| Compress | `compress` | Gzip to ~/.memory-archive/ | Yes, from archive |
| JSON Export | `json` | Human-readable backup | Yes, manual import |
| No Archive | `none` | Delete directly | No recovery |

## Compress Strategy (Recommended)

Archives data as compressed gzip files, balancing storage savings with recoverability.

### How It Works

1. Data exceeding retention period is identified
2. Data is exported to a temporary file
3. File is gzipped and moved to archive directory
4. Original data is deleted from active storage

### Archive Format

```
~/.memory-archive/
├── 2024-01-15_events.json.gz
├── 2024-01-16_events.json.gz
└── 2024-01-17_events.json.gz
```

### Configuration

```toml
[retention]
archive_strategy = "compress"
archive_path = "~/.memory-archive"
```

### Recovery

```bash
# List archived files
ls -la ~/.memory-archive/

# Decompress a specific archive
gunzip -c ~/.memory-archive/2024-01-15_events.json.gz > recovered.json

# Import recovered data
memory-daemon admin import recovered.json
```

## JSON Export Strategy

Exports data as human-readable JSON files before deletion.

### How It Works

1. Data exceeding retention period is identified
2. Data is exported as formatted JSON
3. File is saved to archive directory
4. Original data is deleted from active storage

### Archive Format

```
~/.memory-archive/
├── 2024-01-15_events.json
├── 2024-01-16_events.json
└── 2024-01-17_events.json
```

### Configuration

```toml
[retention]
archive_strategy = "json"
archive_path = "~/.memory-archive"
```

### Benefits

- Human-readable for manual inspection
- Easy to process with standard tools (jq, grep)
- No decompression needed

### Drawbacks

- Uses more disk space than compressed
- Not suitable for large volumes

## No Archive Strategy

Deletes data directly without backup. **This is irreversible.**

### When to Use

- Storage is severely constrained
- Data has no long-term value
- Privacy requirements prohibit retention
- GDPR mode with strict data minimization

### Configuration

```toml
[retention]
archive_strategy = "none"
```

### Warning

```
[!] Data deleted with archive_strategy = "none" cannot be recovered.
    Ensure this aligns with your data retention requirements.
```

## Disk Space Considerations

| Strategy | Space Overhead | Active Storage Impact |
|----------|---------------|----------------------|
| Compress | ~20-30% of original | None after archival |
| JSON | ~100% of original | None after archival |
| None | 0% | None |

### Archive Size Estimation

```
Archive size (compress) = daily_data_size * 0.25 * retention_days
Archive size (json) = daily_data_size * retention_days

Example (5MB/day, 90 days retention):
  Compress: 5MB * 0.25 * 90 = 112.5 MB archive
  JSON: 5MB * 90 = 450 MB archive
```

## Archive Maintenance

### Cleanup Old Archives

Archives can grow indefinitely. Consider periodic cleanup:

```bash
# Remove archives older than 1 year
find ~/.memory-archive -name "*.json*" -mtime +365 -delete

# Check archive size
du -sh ~/.memory-archive
```

### Archive Rotation

Add to crontab for automatic archive cleanup:

```bash
# Weekly cleanup of archives older than 1 year
0 4 * * 0 find ~/.memory-archive -mtime +365 -delete
```

## Configuration Examples

### Conservative (Maximum Recovery)

```toml
[retention]
policy = "days:90"
archive_strategy = "json"
archive_path = "~/.memory-archive"
```

### Balanced (Recommended)

```toml
[retention]
policy = "days:90"
archive_strategy = "compress"
archive_path = "~/.memory-archive"
```

### Minimal Storage

```toml
[retention]
policy = "days:30"
archive_strategy = "none"
```

### GDPR with Export

```toml
[retention]
policy = "days:90"
archive_strategy = "json"
archive_path = "~/.memory-archive"
gdpr_mode = true
gdpr_export_before_delete = true
```
