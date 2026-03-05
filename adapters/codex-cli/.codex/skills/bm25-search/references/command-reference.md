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
