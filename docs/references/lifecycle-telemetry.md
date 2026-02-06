# Lifecycle Telemetry Reference

This document describes the telemetry and observability features for Phase 16 index lifecycle management.

## Overview

Phase 16 introduced lifecycle pruning for both vector (FR-08) and BM25 (FR-09) indexes. Telemetry metrics track:

- Whether lifecycle pruning is enabled
- Last prune operation timestamp
- Count of items pruned in last operation

## GetRankingStatus RPC

The primary RPC for querying lifecycle telemetry.

### Request

```protobuf
message GetRankingStatusRequest {}
```

### Response

```protobuf
message GetRankingStatusResponse {
  // Salience scoring
  bool salience_enabled = 1;

  // Usage decay
  bool usage_decay_enabled = 2;

  // Novelty checking status
  bool novelty_enabled = 3;
  int64 novelty_checked_total = 4;
  int64 novelty_rejected_total = 5;
  int64 novelty_skipped_total = 6;

  // Vector lifecycle status (FR-08)
  bool vector_lifecycle_enabled = 7;
  int64 vector_last_prune_timestamp = 8;
  uint32 vector_last_prune_count = 9;

  // BM25 lifecycle status (FR-09)
  bool bm25_lifecycle_enabled = 10;
  int64 bm25_last_prune_timestamp = 11;
  uint32 bm25_last_prune_count = 12;
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `salience_enabled` | bool | Whether memory salience scoring is active |
| `usage_decay_enabled` | bool | Whether usage-based decay is active |
| `novelty_enabled` | bool | Whether novelty checking blocks redundant content |
| `novelty_checked_total` | int64 | Total content items checked for novelty |
| `novelty_rejected_total` | int64 | Items rejected as redundant |
| `novelty_skipped_total` | int64 | Items skipped (e.g., disabled, errors) |
| `vector_lifecycle_enabled` | bool | Whether vector pruning is enabled (default: true) |
| `vector_last_prune_timestamp` | int64 | Unix timestamp (ms) of last vector prune |
| `vector_last_prune_count` | uint32 | Vectors pruned in last operation |
| `bm25_lifecycle_enabled` | bool | Whether BM25 pruning is enabled (default: false) |
| `bm25_last_prune_timestamp` | int64 | Unix timestamp (ms) of last BM25 prune |
| `bm25_last_prune_count` | uint32 | Documents pruned in last operation |

## CLI Commands

### Query Lifecycle Status

```bash
# Via scheduler status (shows job run counts)
memory-daemon scheduler status

# Example output:
# Scheduler: Running
# Jobs:
#   hourly-rollup:     next=2026-02-06T12:00:00Z, runs=48, errors=0
#   daily-compaction:  next=2026-02-07T03:00:00Z, runs=2, errors=0
#   vector_prune:      next=2026-02-07T03:00:00Z, runs=1, errors=0
#   bm25_prune:        next=2026-02-07T03:00:00Z, runs=0, errors=0 (paused)
```

### Vector Index Stats

```bash
memory-daemon teleport vector-stats

# Example output:
# Vector Index Status
# -------------------
# Status:            Available
# Vectors:           12,456
# Dimension:         384
# Last Indexed:      2026-02-06T10:30:00Z
# Index Path:        /home/user/.agent-memory/vector.idx
# Index Size:        45.2 MB
# Lifecycle Enabled: true
# Last Prune:        2026-02-06T03:00:00Z
# Last Prune Count:  234
```

### BM25 Index Stats

```bash
memory-daemon teleport stats

# Example output:
# BM25 Index Status
# -----------------
# Status:            Available
# Documents:         8,234
# Terms:             156,789
# Last Indexed:      2026-02-06T10:30:00Z
# Index Path:        /home/user/.agent-memory/search/
# Index Size:        23.1 MB
# Lifecycle Enabled: false
# Last Prune:        (never)
# Last Prune Count:  0
```

## Prune Operations

### Vector Prune (FR-08)

```bash
# Dry run to see what would be pruned
memory-daemon admin prune-vector --dry-run

# Prune per configuration
memory-daemon admin prune-vector

# Prune specific level
memory-daemon admin prune-vector --level segment --age-days 14
```

#### Response Fields

```protobuf
message PruneVectorIndexResponse {
  bool success = 1;
  uint32 segments_pruned = 2;
  uint32 grips_pruned = 3;
  uint32 days_pruned = 4;
  uint32 weeks_pruned = 5;
  string message = 6;
}
```

### BM25 Prune (FR-09)

```bash
# Dry run to see what would be pruned
memory-daemon admin prune-bm25 --dry-run

# Prune per configuration
memory-daemon admin prune-bm25

# Prune specific level with custom retention
memory-daemon admin prune-bm25 --level segment --age-days 7
```

#### Response Fields

```protobuf
message PruneBm25IndexResponse {
  bool success = 1;
  uint32 segments_pruned = 2;
  uint32 grips_pruned = 3;
  uint32 days_pruned = 4;
  uint32 weeks_pruned = 5;
  bool optimized = 6;
  string message = 7;
}
```

## Retention Configuration

### Vector Lifecycle (FR-08)

| Level | Default Retention | Configurable |
|-------|-------------------|--------------|
| Segment | 30 days | Yes |
| Grip | 30 days | Yes |
| Day | 365 days | Yes |
| Week | 1825 days (5 years) | Yes |
| Month | Never | No (protected) |
| Year | Never | No (protected) |

**Default:** ENABLED

### BM25 Lifecycle (FR-09)

| Level | Default Retention | Configurable |
|-------|-------------------|--------------|
| Segment | 30 days | Yes |
| Grip | 30 days | Yes |
| Day | 180 days | Yes |
| Week | 1825 days (5 years) | Yes |
| Month | Never | No (protected) |
| Year | Never | No (protected) |

**Default:** DISABLED (per PRD "append-only, no eviction" philosophy)

## Configuration

### Vector Lifecycle Config

```toml
[vector.lifecycle]
enabled = true
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 365
week_retention_days = 1825
```

### BM25 Lifecycle Config

```toml
[bm25.lifecycle]
enabled = false  # Must be explicitly enabled
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 180
week_retention_days = 1825

[bm25.maintenance]
prune_schedule = "0 3 * * *"  # Daily at 3 AM
optimize_after_prune = true
```

## Scheduler Jobs

### vector_prune

- **Schedule:** Daily at 3:00 AM (configurable)
- **Behavior:** Prunes vectors older than retention policy per level
- **Metrics:** Updates `vector_last_prune_timestamp` and `vector_last_prune_count`

### bm25_prune

- **Schedule:** Daily at 3:00 AM (configurable)
- **Behavior:** Prunes BM25 documents older than retention policy per level
- **Note:** DISABLED by default; must be explicitly enabled
- **Metrics:** Updates `bm25_last_prune_timestamp` and `bm25_last_prune_count`

## Monitoring Best Practices

1. **Check scheduler status regularly:**
   ```bash
   memory-daemon scheduler status
   ```

2. **Monitor prune job errors:**
   - Look for `error_count > 0` in scheduler status
   - Check logs for prune job failures

3. **Track index growth vs. prune rate:**
   - Compare index sizes over time
   - Ensure prune rate keeps pace with ingestion

4. **Alert on stale prune timestamps:**
   - If `last_prune_timestamp` is older than expected schedule
   - May indicate job failures or configuration issues

## Implementation Status

| Feature | Proto Defined | RPC Implemented | CLI Implemented |
|---------|---------------|-----------------|-----------------|
| GetRankingStatus | Yes | Pending | Pending |
| PruneVectorIndex | Yes | Pending | Pending |
| PruneBm25Index | Yes | Pending | Pending |
| vector_prune job | N/A | Yes (placeholder) | N/A |
| bm25_prune job | N/A | Yes (placeholder) | N/A |

**Note:** The scheduler jobs exist and log their intent, but the actual RPC calls to prune indexes are pending full implementation.
