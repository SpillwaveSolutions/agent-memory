---
name: bm25-search
description: |
  BM25 keyword search for agent-memory. Use when asked to "find exact terms", "keyword search", "search for specific function names", "locate exact phrase", or when semantic search returns too many results. Provides fast BM25 full-text search via Tantivy index.
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

See [Command Reference](references/command-reference.md) for full CLI options.
