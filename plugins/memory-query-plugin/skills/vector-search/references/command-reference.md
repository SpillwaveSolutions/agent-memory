# Vector Search Command Reference

Complete CLI reference for vector search commands.

## teleport vector-search

Semantic similarity search using vector embeddings.

### Synopsis

```bash
memory-daemon teleport vector-search [OPTIONS] --query <QUERY>
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--query` | `-q` | required | Query text to embed and search |
| `--top-k` | | 10 | Maximum number of results to return |
| `--min-score` | | 0.0 | Minimum similarity score threshold (0.0-1.0) |
| `--target` | | all | Filter by document type: all, toc, grip |
| `--addr` | | http://[::1]:50051 | gRPC server address |

### Examples

```bash
# Basic semantic search
memory-daemon teleport vector-search -q "authentication patterns"

# With minimum score threshold
memory-daemon teleport vector-search -q "debugging" --min-score 0.6

# Search only TOC nodes
memory-daemon teleport vector-search -q "testing strategies" --target toc

# Search only grips (excerpts)
memory-daemon teleport vector-search -q "error messages" --target grip

# Limit results
memory-daemon teleport vector-search -q "best practices" --top-k 5

# Custom endpoint
memory-daemon teleport vector-search -q "query" --addr http://localhost:9999
```

### Output Fields

| Field | Description |
|-------|-------------|
| doc_type | Type of document: toc_node or grip |
| doc_id | Document identifier |
| score | Similarity score (0.0-1.0, higher is better) |
| text_preview | Truncated preview of matched content |
| timestamp | Document creation time |

---

## teleport hybrid-search

Combined BM25 keyword + vector semantic search with RRF fusion.

### Synopsis

```bash
memory-daemon teleport hybrid-search [OPTIONS] --query <QUERY>
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--query` | `-q` | required | Search query |
| `--top-k` | | 10 | Maximum number of results |
| `--mode` | | hybrid | Search mode: hybrid, vector-only, bm25-only |
| `--bm25-weight` | | 0.5 | Weight for BM25 in fusion (0.0-1.0) |
| `--vector-weight` | | 0.5 | Weight for vector in fusion (0.0-1.0) |
| `--target` | | all | Filter by document type: all, toc, grip |
| `--addr` | | http://[::1]:50051 | gRPC server address |

### Search Modes

| Mode | Description |
|------|-------------|
| `hybrid` | Combines BM25 and vector with RRF fusion |
| `vector-only` | Uses only vector similarity (ignores BM25 index) |
| `bm25-only` | Uses only BM25 keyword matching (ignores vector index) |

### Examples

```bash
# Default hybrid search
memory-daemon teleport hybrid-search -q "JWT authentication"

# Vector-only mode
memory-daemon teleport hybrid-search -q "similar concepts" --mode vector-only

# BM25-only mode for exact keywords
memory-daemon teleport hybrid-search -q "ConnectionError" --mode bm25-only

# Favor semantic matching
memory-daemon teleport hybrid-search -q "related topics" \
  --bm25-weight 0.3 \
  --vector-weight 0.7

# Favor keyword matching
memory-daemon teleport hybrid-search -q "function_name" \
  --bm25-weight 0.8 \
  --vector-weight 0.2

# Filter to grip documents only
memory-daemon teleport hybrid-search -q "debugging" --target grip
```

### Output Fields

| Field | Description |
|-------|-------------|
| mode_used | Actual mode used (may differ if index unavailable) |
| bm25_available | Whether BM25 index was available |
| vector_available | Whether vector index was available |
| matches | List of ranked results |

---

## teleport vector-stats

Display vector index statistics.

### Synopsis

```bash
memory-daemon teleport vector-stats [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr` | http://[::1]:50051 | gRPC server address |

### Examples

```bash
# Show vector index stats
memory-daemon teleport vector-stats

# Custom endpoint
memory-daemon teleport vector-stats --addr http://localhost:9999
```

### Output Fields

| Field | Description |
|-------|-------------|
| Status | Whether index is available for searches |
| Vectors | Number of vectors in the index |
| Dimension | Embedding dimension (e.g., 384 for MiniLM) |
| Last Indexed | Timestamp of last index update |
| Index Path | File path to index on disk |
| Index Size | Size of index file |
| Lifecycle Enabled | Whether vector lifecycle pruning is enabled |
| Last Prune | Timestamp of last prune operation |
| Last Prune Count | Vectors pruned in last operation |

---

## teleport stats

Display BM25 index statistics (for comparison).

### Synopsis

```bash
memory-daemon teleport stats [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr` | http://[::1]:50051 | gRPC server address |

---

## teleport search

BM25 keyword search (non-vector).

### Synopsis

```bash
memory-daemon teleport search [OPTIONS] <QUERY>
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `<QUERY>` | | required | Search keywords |
| `--doc-type` | `-t` | all | Filter: all, toc, grip |
| `--limit` | `-n` | 10 | Maximum results |
| `--addr` | | http://[::1]:50051 | gRPC server address |

### Examples

```bash
# Basic BM25 search
memory-daemon teleport search "authentication"

# Filter to TOC nodes
memory-daemon teleport search "JWT" -t toc

# Limit results
memory-daemon teleport search "debugging" -n 5
```

---

## Comparison: When to Use Each

| Scenario | Recommended Command |
|----------|---------------------|
| Exact function/variable name | `teleport search` (BM25) |
| Conceptual query | `teleport vector-search` |
| General purpose | `teleport hybrid-search` |
| Error messages | `teleport search` or `hybrid --bm25-weight 0.8` |
| Finding similar topics | `teleport vector-search` |
| Technical documentation | `teleport hybrid-search` |

---

## Lifecycle Telemetry

Vector lifecycle metrics are available via the `GetRankingStatus` RPC.

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

### Vector Lifecycle Configuration

Default retention periods (per PRD FR-08):

| Level | Retention | Notes |
|-------|-----------|-------|
| Segment | 30 days | High churn, rolled up quickly |
| Grip | 30 days | Same as segment |
| Day | 365 days | Mid-term recall |
| Week | 5 years | Long-term recall |
| Month | Never | Protected (stable anchor) |
| Year | Never | Protected (stable anchor) |

**Note:** Vector lifecycle pruning is ENABLED by default, unlike BM25.

### admin prune-vector

Prune old vectors from the HNSW index.

```bash
memory-daemon admin prune-vector [OPTIONS]
```

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--dry-run` | false | Show what would be pruned |
| `--level <LEVEL>` | all | Prune specific level only |
| `--age-days <N>` | config | Override retention days |

#### Examples

```bash
# Dry run - see what would be pruned
memory-daemon admin prune-vector --dry-run

# Prune per configuration
memory-daemon admin prune-vector

# Prune segments older than 14 days
memory-daemon admin prune-vector --level segment --age-days 14
```
