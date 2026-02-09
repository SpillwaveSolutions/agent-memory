---
name: vector-search
description: |
  Semantic vector search for agent-memory. Use when asked to "find similar discussions", "semantic search", "find related topics", "what's conceptually related to X", or when keyword search returns poor results. Provides vector similarity search and hybrid BM25+vector fusion.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Vector Search Skill

Semantic similarity search using vector embeddings in the agent-memory system.

## When to Use

| Use Case | Best Search Type |
|----------|------------------|
| Exact keyword match | BM25 (`teleport search`) |
| Conceptual similarity | Vector (`teleport vector-search`) |
| Best of both worlds | Hybrid (`teleport hybrid-search`) |
| Typos/synonyms | Vector or Hybrid |
| Technical terms | BM25 or Hybrid |

## When Not to Use

- Current session context (already in memory)
- Time-based queries (use TOC navigation instead)
- Counting or aggregation (not supported)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `teleport vector-search` | Semantic search | `teleport vector-search -q "authentication patterns"` |
| `teleport hybrid-search` | BM25 + Vector | `teleport hybrid-search -q "JWT token handling"` |
| `teleport vector-stats` | Index status | `teleport vector-stats` |

## Prerequisites

```bash
memory-daemon status  # Check daemon
memory-daemon start   # Start if needed
```

## Validation Checklist

Before presenting results:
- [ ] Daemon running: `memory-daemon status` returns "running"
- [ ] Vector index available: `teleport vector-stats` shows `Status: Available`
- [ ] Query returns results: Check for non-empty `matches` array
- [ ] Scores are reasonable: 0.7+ is strong match, 0.5-0.7 moderate

## Vector Search

### Basic Usage

```bash
# Simple semantic search
memory-daemon teleport vector-search -q "authentication patterns"

# With filtering
memory-daemon teleport vector-search -q "debugging strategies" \
  --top-k 5 \
  --min-score 0.6 \
  --target toc
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `-q, --query` | required | Query text to embed and search |
| `--top-k` | 10 | Number of results to return |
| `--min-score` | 0.0 | Minimum similarity (0.0-1.0) |
| `--target` | all | Filter: all, toc, grip |
| `--addr` | http://[::1]:50051 | gRPC server address |

### Output Format

```
Vector Search: "authentication patterns"
Top-K: 10, Min Score: 0.00, Target: all

Found 3 results:
----------------------------------------------------------------------
1. [toc_node] toc:segment:abc123 (score: 0.8542)
   Implemented JWT authentication with refresh token rotation...
   Time: 2026-01-30 14:32

2. [grip] grip:1738252800000:01JKXYZ (score: 0.7891)
   The OAuth2 flow handles authentication through the identity...
   Time: 2026-01-28 09:15
```

## Hybrid Search

Combines BM25 keyword matching with vector semantic similarity using Reciprocal Rank Fusion (RRF).

### Basic Usage

```bash
# Default hybrid mode (50/50 weights)
memory-daemon teleport hybrid-search -q "JWT authentication"

# Favor vector semantics
memory-daemon teleport hybrid-search -q "similar topics" \
  --bm25-weight 0.3 \
  --vector-weight 0.7

# Favor keyword matching
memory-daemon teleport hybrid-search -q "exact_function_name" \
  --bm25-weight 0.8 \
  --vector-weight 0.2
```

### Search Modes

| Mode | Description | Use When |
|------|-------------|----------|
| `hybrid` | RRF fusion of both | Default, general purpose |
| `vector-only` | Only vector similarity | Conceptual queries, synonyms |
| `bm25-only` | Only keyword matching | Exact terms, debugging |

```bash
# Force vector-only mode
memory-daemon teleport hybrid-search -q "similar concepts" --mode vector-only

# Force BM25-only mode
memory-daemon teleport hybrid-search -q "exact_function" --mode bm25-only
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `-q, --query` | required | Search query |
| `--top-k` | 10 | Number of results |
| `--mode` | hybrid | hybrid, vector-only, bm25-only |
| `--bm25-weight` | 0.5 | BM25 weight in fusion |
| `--vector-weight` | 0.5 | Vector weight in fusion |
| `--target` | all | Filter: all, toc, grip |
| `--addr` | http://[::1]:50051 | gRPC server address |

### Output Format

```
Hybrid Search: "JWT authentication"
Mode: hybrid, BM25 Weight: 0.50, Vector Weight: 0.50

Mode used: hybrid (BM25: yes, Vector: yes)

Found 5 results:
----------------------------------------------------------------------
1. [toc_node] toc:segment:abc123 (score: 0.9234)
   JWT token validation and refresh handling...
   Time: 2026-01-30 14:32
```

## Index Statistics

```bash
memory-daemon teleport vector-stats
```

Output:
```
Vector Index Statistics
----------------------------------------
Status:        Available
Vectors:       1523
Dimension:     384
Last Indexed:  2026-01-30T15:42:31Z
Index Path:    ~/.local/share/agent-memory/vector.idx
Index Size:    2.34 MB
```

## Search Strategy

### Decision Flow

```
User Query
    |
    v
+-- Contains exact terms/function names? --> BM25 Search
|
+-- Conceptual/semantic query? --> Vector Search
|
+-- Mixed or unsure? --> Hybrid Search (default)
```

### Recommended Workflows

**Finding related discussions:**
```bash
# Start with hybrid for broad coverage
memory-daemon teleport hybrid-search -q "error handling patterns"

# If too noisy, increase min-score or switch to vector
memory-daemon teleport vector-search -q "error handling patterns" --min-score 0.7
```

**Debugging with exact terms:**
```bash
# Use BM25 for exact matches
memory-daemon teleport search "ConnectionTimeout"

# Or hybrid with BM25 bias
memory-daemon teleport hybrid-search -q "ConnectionTimeout" --bm25-weight 0.8
```

**Exploring concepts:**
```bash
# Pure semantic search for conceptual exploration
memory-daemon teleport vector-search -q "best practices for testing"
```

## Error Handling

| Error | Resolution |
|-------|------------|
| Connection refused | `memory-daemon start` |
| Vector index unavailable | Wait for index build or check disk space |
| No results | Lower `--min-score`, try hybrid mode, broaden query |
| Slow response | Reduce `--top-k`, check index size |

## Advanced

### Tuning Weights

The hybrid search uses Reciprocal Rank Fusion (RRF):
- Higher BM25 weight: Better for exact keyword matches
- Higher vector weight: Better for semantic similarity
- Equal weights (0.5/0.5): Balanced for general queries

### Combining with TOC Navigation

After finding relevant documents via vector search:

```bash
# Get vector search results
memory-daemon teleport vector-search -q "authentication"
# Returns: toc:segment:abc123

# Navigate to get full context
memory-daemon query node --node-id "toc:segment:abc123"

# Expand grip for details
memory-daemon query expand --grip-id "grip:..." --before 3 --after 3
```

See [Command Reference](references/command-reference.md) for full CLI options.
