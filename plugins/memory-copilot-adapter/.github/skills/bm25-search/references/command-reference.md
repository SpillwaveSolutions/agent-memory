# BM25 Search Command Reference

Complete CLI reference for BM25 keyword search commands.

## teleport search

Full-text BM25 keyword search.

```bash
memory-daemon teleport search [OPTIONS] <QUERY>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<QUERY>` | Yes | Search query (supports phrases in quotes) |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--top-k <N>` | 10 | Number of results to return |
| `--target <TYPE>` | all | Filter: all, toc, grip |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

### Examples

```bash
# Basic search
memory-daemon teleport search "authentication"

# Phrase search
memory-daemon teleport search "\"exact phrase match\""

# Top 5 TOC nodes only
memory-daemon teleport search "JWT" --top-k 5 --target toc

# JSON output
memory-daemon teleport search "error handling" --format json
```

## teleport stats

BM25 index statistics.

```bash
memory-daemon teleport stats [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

### Output Fields

| Field | Description |
|-------|-------------|
| Status | Available, Rebuilding, Unavailable |
| Documents | Total indexed documents |
| Terms | Unique terms in index |
| Last Indexed | Timestamp of last update |
| Index Path | Filesystem location |
| Index Size | Size on disk |
| Lifecycle Enabled | Whether BM25 lifecycle pruning is enabled |
| Last Prune | Timestamp of last prune operation |
| Last Prune Count | Documents pruned in last operation |

## teleport rebuild

Rebuild BM25 index from storage.

```bash
memory-daemon teleport rebuild [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--force` | false | Skip confirmation prompt |
| `--min-level <LEVEL>` | segment | Minimum TOC level: segment, day, week, month |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |

### Examples

```bash
# Full rebuild with confirmation
memory-daemon teleport rebuild

# Force rebuild without prompt
memory-daemon teleport rebuild --force

# Only index day level and above
memory-daemon teleport rebuild --min-level day
```

## admin prune-bm25

Prune old documents from BM25 index.

```bash
memory-daemon admin prune-bm25 [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--dry-run` | false | Show what would be pruned |
| `--level <LEVEL>` | all | Prune specific level only |
| `--age-days <N>` | config | Override retention days |

### Examples

```bash
# Dry run - see what would be pruned
memory-daemon admin prune-bm25 --dry-run

# Prune per configuration
memory-daemon admin prune-bm25

# Prune segments older than 14 days
memory-daemon admin prune-bm25 --level segment --age-days 14
```

## admin optimize-bm25

Optimize BM25 index segments.

```bash
memory-daemon admin optimize-bm25 [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |

## GetTeleportStatus RPC

gRPC status check for BM25 index.

### Request

```protobuf
message GetTeleportStatusRequest {
  // No fields - returns full status
}
```

### Response

```protobuf
message TeleportStatus {
  bool bm25_enabled = 1;
  bool bm25_healthy = 2;
  uint64 bm25_doc_count = 3;
  int64 bm25_last_indexed = 4;
  string bm25_index_path = 5;
  uint64 bm25_index_size_bytes = 6;
  // Lifecycle metrics (Phase 16)
  int64 bm25_last_prune_timestamp = 60;
  uint32 bm25_last_prune_segments = 61;
  uint32 bm25_last_prune_days = 62;
}
```

## TeleportSearch RPC

gRPC BM25 search.

### Request

```protobuf
message TeleportSearchRequest {
  string query = 1;
  uint32 top_k = 2;
  string target = 3;  // "all", "toc", "grip"
}
```

### Response

```protobuf
message TeleportSearchResponse {
  repeated TeleportMatch matches = 1;
}

message TeleportMatch {
  string doc_id = 1;
  string doc_type = 2;
  float score = 3;
  string excerpt = 4;
  int64 timestamp = 5;
}
```

## Lifecycle Telemetry

BM25 lifecycle metrics are available via the `GetRankingStatus` RPC.

### GetRankingStatus RPC

Returns lifecycle and ranking status for all indexes.

```protobuf
message GetRankingStatusRequest {}

message GetRankingStatusResponse {
  // Salience and usage decay
  bool salience_enabled = 1;
  bool usage_decay_enabled = 2;

  // Novelty checking
  bool novelty_enabled = 3;
  int64 novelty_checked_total = 4;
  int64 novelty_rejected_total = 5;
  int64 novelty_skipped_total = 6;

  // Vector lifecycle (FR-08)
  bool vector_lifecycle_enabled = 7;
  int64 vector_last_prune_timestamp = 8;
  uint32 vector_last_prune_count = 9;

  // BM25 lifecycle (FR-09)
  bool bm25_lifecycle_enabled = 10;
  int64 bm25_last_prune_timestamp = 11;
  uint32 bm25_last_prune_count = 12;
}
```

### BM25 Lifecycle Configuration

Default retention periods (per PRD FR-09):

| Level | Retention | Notes |
|-------|-----------|-------|
| Segment | 30 days | High churn, rolled up quickly |
| Grip | 30 days | Same as segment |
| Day | 180 days | Mid-term recall while rollups mature |
| Week | 5 years | Long-term recall |
| Month | Never | Protected (stable anchor) |
| Year | Never | Protected (stable anchor) |

**Note:** BM25 lifecycle pruning is DISABLED by default per PRD "append-only, no eviction" philosophy. Must be explicitly enabled in configuration.
