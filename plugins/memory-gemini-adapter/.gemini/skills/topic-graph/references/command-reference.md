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

### Examples

```bash
# Top 10 active topics
memory-daemon topics top

# Top 20 including dormant
memory-daemon topics top --limit 20 --include-dormant

# JSON output
memory-daemon topics top --format json
```

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

### Examples

```bash
# Find topics about authentication
memory-daemon topics query "authentication"

# High confidence only
memory-daemon topics query "error handling" --min-similarity 0.8
```

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

### Examples

```bash
# All relationships
memory-daemon topics related --topic-id "topic:authentication"

# Only similar topics
memory-daemon topics related --topic-id "topic:jwt" --type similar

# Parent topics (broader concepts)
memory-daemon topics related --topic-id "topic:jwt" --type parent
```

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

### Examples

```bash
# Get TOC nodes for topic
memory-daemon topics nodes --topic-id "topic:authentication"
```

## topics dormant

List dormant topics.

```bash
memory-daemon topics dormant [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--limit <N>` | 20 | Number of topics |
| `--older-than-days <N>` | 0 | Filter by age |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

## admin extract-topics

Force topic extraction.

```bash
memory-daemon admin extract-topics [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--since <TIMESTAMP>` | last_checkpoint | Extract from timestamp |
| `--batch-size <N>` | config | Batch size for processing |

## admin prune-topics

Prune old dormant topics.

```bash
memory-daemon admin prune-topics [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--dry-run` | false | Show what would be pruned |
| `--older-than-days <N>` | config | Override age threshold |

## GetTopicGraphStatus RPC

gRPC status check for topic graph.

### Request

```protobuf
message GetTopicGraphStatusRequest {
  // No fields - returns full status
}
```

### Response

```protobuf
message TopicGraphStatus {
  bool enabled = 1;
  bool healthy = 2;
  uint32 topic_count = 3;
  uint32 active_count = 4;
  uint32 dormant_count = 5;
  int64 last_extraction = 6;
  float half_life_days = 7;
}
```

## GetTopicsByQuery RPC

gRPC topic query.

### Request

```protobuf
message GetTopicsByQueryRequest {
  string query = 1;
  uint32 limit = 2;
  float min_similarity = 3;
}
```

### Response

```protobuf
message GetTopicsByQueryResponse {
  repeated TopicMatch topics = 1;
}

message TopicMatch {
  string topic_id = 1;
  string label = 2;
  float similarity = 3;
  float importance = 4;
  uint32 mention_count = 5;
  int64 last_seen = 6;
  repeated string related_topic_ids = 7;
}
```

## GetRelatedTopics RPC

gRPC related topics query.

### Request

```protobuf
message GetRelatedTopicsRequest {
  string topic_id = 1;
  uint32 limit = 2;
  string relation_type = 3;  // "all", "similar", "parent", "child"
}
```

### Response

```protobuf
message GetRelatedTopicsResponse {
  repeated TopicRelation relations = 1;
}

message TopicRelation {
  string topic_id = 1;
  string label = 2;
  string relation_type = 3;
  float strength = 4;
}
```

## GetTocNodesForTopic RPC

gRPC TOC nodes for topic.

### Request

```protobuf
message GetTocNodesForTopicRequest {
  string topic_id = 1;
  uint32 limit = 2;
}
```

### Response

```protobuf
message GetTocNodesForTopicResponse {
  repeated TopicNodeRef nodes = 1;
}

message TopicNodeRef {
  string node_id = 1;
  string title = 2;
  int64 timestamp = 3;
  float relevance = 4;
}
```
