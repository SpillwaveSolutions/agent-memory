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
