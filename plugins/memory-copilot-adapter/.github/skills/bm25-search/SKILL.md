---
name: bm25-search
description: |
  BM25 keyword search for agent-memory. Use when asked to "find exact terms", "keyword search", "search for specific function names", "locate exact phrase", or when semantic search returns too many results. Provides fast BM25 full-text search via Tantivy index.
license: MIT
metadata:
  version: 2.1.0
  author: SpillwaveSolutions
---

# BM25 Keyword Search Skill

Fast full-text keyword search using BM25 scoring in the agent-memory system.

## When to Use

| Use Case | Best Search Type |
|----------|------------------|
| Exact keyword match | BM25 (`teleport search`) |
| Function/variable names | BM25 (exact terms) |
| Error messages | BM25 (specific phrases) |
| Technical identifiers | BM25 (case-sensitive) |
| Conceptual similarity | Vector search instead |

## When Not to Use

- Conceptual/semantic queries (use vector search)
- Synonym-heavy queries (use hybrid search)
- Current session context (already in memory)
- Time-based navigation (use TOC directly)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `teleport search` | BM25 keyword search | `teleport search "ConnectionTimeout"` |
| `teleport stats` | BM25 index status | `teleport stats` |
| `teleport rebuild` | Rebuild index | `teleport rebuild --force` |

## Prerequisites

```bash
memory-daemon status  # Check daemon
memory-daemon start   # Start if needed
```

## Validation Checklist

Before presenting results:
- [ ] Daemon running: `memory-daemon status` returns "running"
- [ ] BM25 index available: `teleport stats` shows `Status: Available`
- [ ] Query returns results: Check for non-empty `matches` array
- [ ] Scores are reasonable: Higher BM25 = better keyword match

## BM25 Search

### Basic Usage

```bash
# Simple keyword search
memory-daemon teleport search "JWT token"

# Search with options
memory-daemon teleport search "authentication" \
  --top-k 10 \
  --target toc

# Phrase search (exact match)
memory-daemon teleport search "\"connection refused\""
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `query` | required | Search query (positional) |
| `--top-k` | 10 | Number of results to return |
| `--target` | all | Filter: all, toc, grip |
| `--addr` | http://[::1]:50051 | gRPC server address |

### Output Format

```
BM25 Search: "JWT token"
Top-K: 10, Target: all

Found 4 results:
----------------------------------------------------------------------
1. [toc_node] toc:segment:abc123 (score: 12.45)
   JWT token validation and refresh handling...
   Time: 2026-01-30 14:32

2. [grip] grip:1738252800000:01JKXYZ (score: 10.21)
   The JWT library handles token parsing...
   Time: 2026-01-28 09:15
```

## Index Statistics

```bash
memory-daemon teleport stats
```

Output:
```
BM25 Index Statistics
----------------------------------------
Status:        Available
Documents:     2847
Terms:         45,231
Last Indexed:  2026-01-30T15:42:31Z
Index Path:    ~/.local/share/agent-memory/tantivy
Index Size:    12.5 MB
```

## Index Lifecycle Configuration

BM25 index lifecycle is controlled by configuration (Phase 16):

```toml
[teleport.bm25.lifecycle]
enabled = false  # Opt-in (append-only by default)
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 180
week_retention_days = 1825
# month/year: never pruned (protected)

[teleport.bm25.maintenance]
prune_schedule = "0 3 * * *"  # Daily at 3 AM
optimize_after_prune = true
```

### Pruning Commands

```bash
# Check what would be pruned
memory-daemon admin prune-bm25 --dry-run

# Execute pruning per lifecycle config
memory-daemon admin prune-bm25

# Prune specific level
memory-daemon admin prune-bm25 --level segment --age-days 14
```

## Index Administration

### Rebuild Index

```bash
# Full rebuild from RocksDB
memory-daemon teleport rebuild --force

# Rebuild specific levels
memory-daemon teleport rebuild --min-level day
```

### Index Optimization

```bash
# Compact index segments
memory-daemon admin optimize-bm25
```

## Search Strategy

### Decision Flow

```
User Query
    |
    v
+-- Contains exact terms/function names? --> BM25 Search
|
+-- Contains quotes "exact phrase"? --> BM25 Search
|
+-- Error message or identifier? --> BM25 Search
|
+-- Conceptual/semantic query? --> Vector Search
|
+-- Mixed or unsure? --> Hybrid Search
```

### Query Syntax

| Pattern | Example | Matches |
|---------|---------|---------|
| Single term | `JWT` | All docs containing "JWT" |
| Multiple terms | `JWT token` | Docs with "JWT" AND "token" |
| Phrase | `"JWT token"` | Exact phrase "JWT token" |
| Prefix | `auth*` | Terms starting with "auth" |

## Error Handling

| Error | Resolution |
|-------|------------|
| Connection refused | `memory-daemon start` |
| BM25 index unavailable | `teleport rebuild` or wait for build |
| No results | Check spelling, try broader terms |
| Slow response | Rebuild index or check disk |

## Combining with TOC Navigation

After finding relevant documents via BM25 search:

```bash
# Get BM25 search results
memory-daemon teleport search "ConnectionTimeout"
# Returns: toc:segment:abc123

# Navigate to get full context
memory-daemon query node --node-id "toc:segment:abc123"

# Expand grip for details
memory-daemon query expand --grip-id "grip:..." --before 3 --after 3
```

## Advanced: Tier Detection

The BM25 index is part of the retrieval tier system (Phase 17):

| Tier | Available Layers | BM25 Role |
|------|-----------------|-----------|
| Tier 1 (Full) | Topics + Hybrid + Agentic | Part of hybrid |
| Tier 2 (Hybrid) | BM25 + Vector + Agentic | Part of hybrid |
| Tier 4 (Keyword) | BM25 + Agentic | Primary search |
| Tier 5 (Agentic) | Agentic only | Not available |

Check current tier:
```bash
memory-daemon retrieval status
```

See [Command Reference](references/command-reference.md) for full CLI options.
