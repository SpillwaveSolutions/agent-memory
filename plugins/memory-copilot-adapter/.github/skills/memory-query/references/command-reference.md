# Memory Query Command Reference

Detailed reference for all memory-daemon query commands.

## Connection

All query commands require connection to a running memory-daemon:

```bash
# Default endpoint
--endpoint http://[::1]:50051

# Custom endpoint
--endpoint http://localhost:50052
```

## Query Commands

### root

Get the TOC root nodes (top-level time periods).

```bash
memory-daemon query --endpoint http://[::1]:50051 root
```

**Output:** List of year nodes with summary information.

### node

Get a specific TOC node by ID.

```bash
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"
```

**Parameters:**
- `--node-id` (required): The node identifier

**Node ID Formats:**
| Level | Format | Example |
|-------|--------|---------|
| Year | `toc:year:YYYY` | `toc:year:2026` |
| Month | `toc:month:YYYY-MM` | `toc:month:2026-01` |
| Week | `toc:week:YYYY-Www` | `toc:week:2026-W04` |
| Day | `toc:day:YYYY-MM-DD` | `toc:day:2026-01-30` |
| Segment | `toc:segment:YYYY-MM-DDTHH:MM:SS` | `toc:segment:2026-01-30T14:30:00` |

**Output:** Node with title, bullets, keywords, and children list.

### browse

Browse children of a TOC node with pagination.

```bash
memory-daemon query --endpoint http://[::1]:50051 browse \
  --parent-id "toc:month:2026-01" \
  --limit 10
```

**Parameters:**
- `--parent-id` (required): Parent node ID to browse
- `--limit` (optional): Maximum results (default: 50)
- `--continuation-token` (optional): Token for next page

**Output:** Paginated list of child nodes.

### events

Retrieve raw events by time range.

```bash
memory-daemon query --endpoint http://[::1]:50051 events \
  --from 1706745600000 \
  --to 1706832000000 \
  --limit 100
```

**Parameters:**
- `--from` (required): Start timestamp in milliseconds
- `--to` (required): End timestamp in milliseconds
- `--limit` (optional): Maximum events (default: 100)

**Output:** Raw event records with full text and metadata.

### expand

Expand a grip to retrieve context around an excerpt.

```bash
memory-daemon query --endpoint http://[::1]:50051 expand \
  --grip-id "grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE" \
  --before 3 \
  --after 3
```

**Parameters:**
- `--grip-id` (required): The grip identifier
- `--before` (optional): Events before excerpt (default: 2)
- `--after` (optional): Events after excerpt (default: 2)

**Grip ID Format:** `grip:{timestamp_ms}:{ulid}`
- timestamp_ms: 13-digit millisecond timestamp
- ulid: 26-character ULID

**Output:** Context structure with:
- `before`: Events preceding the excerpt
- `excerpt`: The referenced conversation segment
- `after`: Events following the excerpt

## Search Commands

### search

Search TOC nodes for matching content.

**Usage:**
```bash
memory-daemon query search --query <QUERY> [OPTIONS]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--query`, `-q` | Search terms (required) | - |
| `--node` | Search within specific node | - |
| `--parent` | Search children of parent | - |
| `--fields` | Fields to search (comma-separated) | all |
| `--limit` | Maximum results | 10 |

**Fields:**
- `title` - Node title
- `summary` - Derived from bullets
- `bullets` - Individual bullet points (includes grip IDs)
- `keywords` - Extracted keywords

**Examples:**
```bash
# Search at root level
memory-daemon query search --query "authentication debugging"

# Search within month
memory-daemon query search --node "toc:month:2026-01" --query "JWT"

# Search week's children (days)
memory-daemon query search --parent "toc:week:2026-W04" --query "token refresh"

# Search only in bullets and keywords
memory-daemon query search --query "OAuth" --fields "bullets,keywords" --limit 20
```

**Output:**
```
Search Results for children of toc:week:2026-W04
Query: "token refresh"
Found: 2 nodes

Node: toc:day:2026-01-30 (score=0.85)
  Title: Thursday, January 30
  Matches:
    - [bullets] Fixed JWT token refresh rotation
    - [keywords] authentication
```

## Retrieval Commands

### retrieval status

Check available retrieval tier and layers.

```bash
memory-daemon retrieval status
```

### retrieval classify

Classify a query's intent for optimal routing.

```bash
memory-daemon retrieval classify "What JWT issues did we have?"
```

### retrieval route

Route a query through optimal layers with automatic execution.

```bash
memory-daemon retrieval route "authentication errors" --top-k 10 --explain
```

## Event Types

| Type | Description |
|------|-------------|
| `session_start` | Session began |
| `session_end` | Session ended |
| `user_message` | User prompt/message |
| `assistant_message` | Assistant response |
| `tool_result` | Tool execution result |
| `subagent_start` | Subagent spawned |
| `subagent_stop` | Subagent completed |

## Admin Commands

For administrative operations (requires direct storage access):

```bash
# Storage statistics
memory-daemon admin --db-path ~/.memory-store stats

# Compact storage
memory-daemon admin --db-path ~/.memory-store compact

# Compact specific column family
memory-daemon admin --db-path ~/.memory-store compact --cf events
```

## Troubleshooting

### Connection Issues

```bash
# Check daemon status
memory-daemon status

# Start daemon if not running
memory-daemon start

# Check port availability
lsof -i :50051
```

### No Results

1. Verify TOC has been built (requires events to be ingested)
2. Check time range parameters
3. Navigate TOC hierarchy to confirm data exists

### Performance

- Use `--limit` to control result size
- Navigate TOC hierarchy rather than scanning all events
- Use grips for targeted context retrieval
