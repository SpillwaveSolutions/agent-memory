# Retrieval Policy Command Reference

Complete CLI reference for retrieval policy commands.

## retrieval status

Check retrieval tier and layer availability.

```bash
memory-daemon retrieval status [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

### Output Fields

| Field | Description |
|-------|-------------|
| Current Tier | Tier number and name (1-5) |
| Available Layers | Healthy layers with stats |
| Unavailable Layers | Disabled or unhealthy layers |
| Layer Details | Health status, document counts |

### Examples

```bash
# Check tier status
memory-daemon retrieval status

# JSON output
memory-daemon retrieval status --format json
```

## retrieval classify

Classify query intent for optimal routing.

```bash
memory-daemon retrieval classify [OPTIONS] <QUERY>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<QUERY>` | Yes | Query text to classify |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

### Output Fields

| Field | Description |
|-------|-------------|
| Intent | Explore, Answer, Locate, or Time-boxed |
| Confidence | Classification confidence (0.0-1.0) |
| Time Constraint | Extracted time filter (if any) |
| Keywords | Extracted query keywords |
| Suggested Mode | Recommended execution mode |

### Examples

```bash
# Classify query intent
memory-daemon retrieval classify "What JWT issues did we have?"

# With time reference
memory-daemon retrieval classify "debugging session last Tuesday"
```

## retrieval route

Route query through optimal layers with full execution.

```bash
memory-daemon retrieval route [OPTIONS] <QUERY>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<QUERY>` | Yes | Query to route and execute |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--top-k <N>` | 10 | Number of results to return |
| `--max-depth <N>` | 3 | Maximum drill-down levels |
| `--max-nodes <N>` | 50 | Maximum nodes to visit |
| `--timeout <MS>` | 5000 | Query timeout in milliseconds |
| `--mode <MODE>` | auto | Execution mode: auto, sequential, parallel, hybrid |
| `--explain` | false | Include full explainability payload |
| `--addr <ADDR>` | http://[::1]:50051 | gRPC server address |
| `--format <FMT>` | text | Output: text, json |

### Examples

```bash
# Route with auto mode
memory-daemon retrieval route "authentication errors"

# Force parallel execution
memory-daemon retrieval route "explore recent topics" --mode parallel

# With explainability
memory-daemon retrieval route "JWT validation" --explain

# Time-constrained
memory-daemon retrieval route "debugging last week" --max-nodes 30
```

## GetRetrievalCapabilities RPC

gRPC capability check.

### Request

```protobuf
message GetRetrievalCapabilitiesRequest {
  // No fields - returns full status
}
```

### Response

```protobuf
message RetrievalCapabilities {
  uint32 current_tier = 1;
  string tier_name = 2;
  repeated LayerStatus layers = 3;
}

message LayerStatus {
  string layer = 1;  // "topics", "hybrid", "vector", "bm25", "agentic"
  bool healthy = 2;
  bool enabled = 3;
  string reason = 4;  // Why unavailable
  uint64 doc_count = 5;
}
```

## ClassifyQueryIntent RPC

gRPC intent classification.

### Request

```protobuf
message ClassifyQueryIntentRequest {
  string query = 1;
}
```

### Response

```protobuf
message QueryIntentClassification {
  string intent = 1;  // "Explore", "Answer", "Locate", "TimeBoxed"
  float confidence = 2;
  optional string time_constraint = 3;
  repeated string keywords = 4;
  string suggested_mode = 5;
}
```

## RouteQuery RPC

gRPC query routing with execution.

### Request

```protobuf
message RouteQueryRequest {
  string query = 1;
  uint32 top_k = 2;
  uint32 max_depth = 3;
  uint32 max_nodes = 4;
  uint32 timeout_ms = 5;
  string execution_mode = 6;  // "auto", "sequential", "parallel", "hybrid"
  bool include_explanation = 7;
}
```

### Response

```protobuf
message RouteQueryResponse {
  repeated MemoryMatch matches = 1;
  ExplainabilityPayload explanation = 2;
}

message MemoryMatch {
  string doc_id = 1;
  string doc_type = 2;  // "toc_node", "grip"
  float score = 3;
  string excerpt = 4;
  int64 timestamp = 5;
  string source_layer = 6;  // Which layer found this
}

message ExplainabilityPayload {
  uint32 tier_used = 1;
  string tier_name = 2;
  string intent = 3;
  string method = 4;
  repeated string layers_tried = 5;
  repeated string layers_succeeded = 6;
  repeated string fallbacks_used = 7;
  optional string time_constraint = 8;
  string stop_reason = 9;
  map<string, uint32> results_per_layer = 10;
  uint32 execution_time_ms = 11;
  float confidence = 12;
}
```
