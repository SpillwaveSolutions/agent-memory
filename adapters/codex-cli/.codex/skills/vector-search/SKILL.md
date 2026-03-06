---
name: vector-search
description: |
  Semantic vector search for agent-memory. Use when asked to "find similar discussions", "semantic search", "find related topics", "what's conceptually related to X", or when keyword search returns poor results. Provides vector similarity search and hybrid BM25+vector fusion.
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

## Error Handling

| Error | Resolution |
|-------|------------|
| Connection refused | `memory-daemon start` |
| Vector index unavailable | Wait for index build or check disk space |
| No results | Lower `--min-score`, try hybrid mode, broaden query |
| Slow response | Reduce `--top-k`, check index size |

See [Command Reference](references/command-reference.md) for full CLI options.
