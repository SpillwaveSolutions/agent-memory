# Vector Search Command Reference

Complete CLI reference for vector search commands.

## teleport vector-search

Semantic similarity search using vector embeddings.

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

# Limit results
memory-daemon teleport vector-search -q "best practices" --top-k 5
```

## teleport hybrid-search

Combined BM25 keyword + vector semantic search with RRF fusion.

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
```

## teleport vector-stats

Display vector index statistics.

```bash
memory-daemon teleport vector-stats [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr` | http://[::1]:50051 | gRPC server address |

### Output Fields

| Field | Description |
|-------|-------------|
| Status | Whether index is available for searches |
| Vectors | Number of vectors in the index |
| Dimension | Embedding dimension (e.g., 384 for MiniLM) |
| Last Indexed | Timestamp of last index update |
| Index Path | File path to index on disk |
| Index Size | Size of index file |
