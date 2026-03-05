# Topic Graph Command Reference

Complete CLI reference for topic graph exploration commands.

## topics status

Topic graph health and statistics.

```bash
memory-daemon topics status [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

### Output Fields

| Field | Description |
|-------|-------------|
| Enabled | Whether topic extraction is enabled |
| Healthy | Topic graph health status |
| Total Topics | All topics (active + dormant) |
| Active Topics | Topics with importance > 0.1 |
| Dormant Topics | Topics with importance < 0.1 |
| Last Extraction | Timestamp of last extraction job |
| Half-Life Days | Time decay half-life setting |

## topics top

List top topics by importance.

```bash
memory-daemon topics top [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--limit <N>` | 10 | Number of topics to return |
| `--include-dormant` | false | Include dormant topics |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

## topics query

Find topics matching a query.

```bash
memory-daemon topics query [OPTIONS] <QUERY>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<QUERY>` | Yes | Query text to match topics |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--limit <N>` | 10 | Number of topics to return |
| `--min-similarity <F>` | 0.5 | Minimum similarity score (0.0-1.0) |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

## topics related

Get related topics.

```bash
memory-daemon topics related [OPTIONS] --topic-id <TOPIC_ID>
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--topic-id <ID>` | required | Topic ID to find relations for |
| `--limit <N>` | 10 | Number of related topics |
| `--type <TYPE>` | all | Relation type: all, similar, parent, child |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

## topics nodes

Get TOC nodes associated with a topic.

```bash
memory-daemon topics nodes [OPTIONS] --topic-id <TOPIC_ID>
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--topic-id <ID>` | required | Topic ID |
| `--limit <N>` | 20 | Number of nodes to return |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |
