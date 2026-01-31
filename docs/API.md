# API Reference

## gRPC Service: MemoryService

The MemoryService provides event ingestion and query operations for the agent memory system.

Proto file: `proto/memory.proto`

## RPCs

### IngestEvent

Ingest a conversation event into storage.

**Request:**
```protobuf
message IngestEventRequest {
    Event event = 1;
}

message Event {
    string event_id = 1;      // ULID, client-generated
    string session_id = 2;    // Session identifier
    int64 timestamp_ms = 3;   // Unix epoch milliseconds
    EventType event_type = 4; // Type of event
    EventRole role = 5;       // Author role
    string text = 6;          // Content
    map<string, string> metadata = 7;  // Optional metadata
}
```

**Response:**
```protobuf
message IngestEventResponse {
    string event_id = 1;  // The stored event ID
    bool created = 2;     // True if new, false if duplicate (idempotent)
}
```

**Errors:**
- `INVALID_ARGUMENT`: Missing or empty event_id or session_id
- `INVALID_ARGUMENT`: Invalid timestamp (out of valid range)
- `INTERNAL`: Storage write failure

**Example:**
```bash
grpcurl -plaintext -d '{
  "event": {
    "event_id": "01HXYZABC123",
    "session_id": "session-456",
    "timestamp_ms": 1706600000000,
    "event_type": 2,
    "role": 1,
    "text": "What is Rust?"
  }
}' localhost:50051 memory.MemoryService/IngestEvent
```

---

### GetTocRoot

Get root TOC nodes (year level).

**Request:**
```protobuf
message GetTocRootRequest {}
```

**Response:**
```protobuf
message GetTocRootResponse {
    repeated TocNode nodes = 1;  // Year nodes, sorted by time descending
}
```

**Example:**
```bash
grpcurl -plaintext localhost:50051 memory.MemoryService/GetTocRoot
```

---

### GetNode

Get a specific TOC node by ID.

**Request:**
```protobuf
message GetNodeRequest {
    string node_id = 1;  // e.g., "toc:year:2026", "toc:month:2026-01"
}
```

**Response:**
```protobuf
message GetNodeResponse {
    optional TocNode node = 1;  // Null if not found
}
```

**Example:**
```bash
grpcurl -plaintext -d '{"node_id": "toc:year:2026"}' \
  localhost:50051 memory.MemoryService/GetNode
```

---

### BrowseToc

Browse children of a TOC node with pagination.

**Request:**
```protobuf
message BrowseTocRequest {
    string parent_id = 1;                    // Parent node ID
    int32 limit = 2;                         // Max results per page
    optional string continuation_token = 3; // Token from previous response
}
```

**Response:**
```protobuf
message BrowseTocResponse {
    repeated TocNode children = 1;           // Child nodes
    optional string continuation_token = 2;  // Token for next page (null if no more)
    bool has_more = 3;                       // True if more results available
}
```

**Example:**
```bash
# First page
grpcurl -plaintext -d '{"parent_id": "toc:year:2026", "limit": 10}' \
  localhost:50051 memory.MemoryService/BrowseToc

# Next page (using token from previous response)
grpcurl -plaintext -d '{
  "parent_id": "toc:year:2026",
  "limit": 10,
  "continuation_token": "abc123..."
}' localhost:50051 memory.MemoryService/BrowseToc
```

---

### GetEvents

Get events in a time range.

**Request:**
```protobuf
message GetEventsRequest {
    int64 from_timestamp_ms = 1;  // Start time (inclusive)
    int64 to_timestamp_ms = 2;    // End time (inclusive)
    int32 limit = 3;              // Max events to return
}
```

**Response:**
```protobuf
message GetEventsResponse {
    repeated Event events = 1;  // Events in range, sorted by time
    bool has_more = 2;          // True if more events beyond limit
}
```

**Example:**
```bash
grpcurl -plaintext -d '{
  "from_timestamp_ms": 1706600000000,
  "to_timestamp_ms": 1706700000000,
  "limit": 100
}' localhost:50051 memory.MemoryService/GetEvents
```

---

### ExpandGrip

Expand a grip to show context events around the excerpt.

**Request:**
```protobuf
message ExpandGripRequest {
    string grip_id = 1;              // Grip ID to expand
    optional int32 events_before = 2; // Events before excerpt (default: 5)
    optional int32 events_after = 3;  // Events after excerpt (default: 5)
}
```

**Response:**
```protobuf
message ExpandGripResponse {
    optional Grip grip = 1;           // The grip (null if not found)
    repeated Event events_before = 2; // Events before excerpt
    repeated Event excerpt_events = 3; // Events containing the excerpt
    repeated Event events_after = 4;  // Events after excerpt
}
```

**Example:**
```bash
grpcurl -plaintext -d '{
  "grip_id": "grip:1706600000000:01HXYZ",
  "events_before": 3,
  "events_after": 3
}' localhost:50051 memory.MemoryService/ExpandGrip
```

---

## Data Types

### Event

A conversation event.

```protobuf
message Event {
    string event_id = 1;
    string session_id = 2;
    int64 timestamp_ms = 3;
    EventType event_type = 4;
    EventRole role = 5;
    string text = 6;
    map<string, string> metadata = 7;
}
```

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | string | ULID identifier (client-generated, 26 chars) |
| `session_id` | string | Session identifier (any non-empty string) |
| `timestamp_ms` | int64 | Unix epoch milliseconds |
| `event_type` | EventType | Type of event (see enum below) |
| `role` | EventRole | Author role (see enum below) |
| `text` | string | Event content |
| `metadata` | map | Optional key-value pairs |

### EventType

```protobuf
enum EventType {
    EVENT_TYPE_UNSPECIFIED = 0;
    EVENT_TYPE_SESSION_START = 1;
    EVENT_TYPE_USER_MESSAGE = 2;
    EVENT_TYPE_ASSISTANT_MESSAGE = 3;
    EVENT_TYPE_TOOL_RESULT = 4;
    EVENT_TYPE_ASSISTANT_STOP = 5;
    EVENT_TYPE_SUBAGENT_START = 6;
    EVENT_TYPE_SUBAGENT_STOP = 7;
    EVENT_TYPE_SESSION_END = 8;
}
```

| Value | Description |
|-------|-------------|
| `SESSION_START` | Conversation session begins |
| `USER_MESSAGE` | User prompt/input |
| `ASSISTANT_MESSAGE` | AI response |
| `TOOL_RESULT` | Tool execution output |
| `ASSISTANT_STOP` | AI stops generating |
| `SUBAGENT_START` | Subagent spawned |
| `SUBAGENT_STOP` | Subagent completes |
| `SESSION_END` | Session ends |

### EventRole

```protobuf
enum EventRole {
    EVENT_ROLE_UNSPECIFIED = 0;
    EVENT_ROLE_USER = 1;
    EVENT_ROLE_ASSISTANT = 2;
    EVENT_ROLE_SYSTEM = 3;
    EVENT_ROLE_TOOL = 4;
}
```

| Value | Description |
|-------|-------------|
| `USER` | Human user |
| `ASSISTANT` | AI assistant |
| `SYSTEM` | System message |
| `TOOL` | Tool output |

### TocNode

A node in the Table of Contents hierarchy.

```protobuf
message TocNode {
    string node_id = 1;
    TocLevel level = 2;
    string title = 3;
    optional string summary = 4;
    repeated TocBullet bullets = 5;
    repeated string keywords = 6;
    repeated string child_node_ids = 7;
    int64 start_time_ms = 8;
    int64 end_time_ms = 9;
    int32 version = 10;
}
```

| Field | Type | Description |
|-------|------|-------------|
| `node_id` | string | Unique identifier (e.g., "toc:year:2026") |
| `level` | TocLevel | Hierarchy level |
| `title` | string | Human-readable title |
| `summary` | string | Optional summary text |
| `bullets` | TocBullet[] | Summary bullet points |
| `keywords` | string[] | Searchable keywords |
| `child_node_ids` | string[] | Child node IDs |
| `start_time_ms` | int64 | Period start timestamp |
| `end_time_ms` | int64 | Period end timestamp |
| `version` | int32 | Version number (increases on update) |

### TocLevel

```protobuf
enum TocLevel {
    TOC_LEVEL_UNSPECIFIED = 0;
    TOC_LEVEL_YEAR = 1;
    TOC_LEVEL_MONTH = 2;
    TOC_LEVEL_WEEK = 3;
    TOC_LEVEL_DAY = 4;
    TOC_LEVEL_SEGMENT = 5;
}
```

### TocBullet

A bullet point in a TOC node.

```protobuf
message TocBullet {
    string text = 1;
    repeated string grip_ids = 2;
}
```

| Field | Type | Description |
|-------|------|-------------|
| `text` | string | Bullet text |
| `grip_ids` | string[] | IDs of grips supporting this bullet |

### Grip

Provenance anchor linking summaries to source events.

```protobuf
message Grip {
    string grip_id = 1;
    string excerpt = 2;
    string event_id_start = 3;
    string event_id_end = 4;
    int64 timestamp_ms = 5;
    string source = 6;
}
```

| Field | Type | Description |
|-------|------|-------------|
| `grip_id` | string | Unique identifier |
| `excerpt` | string | Text excerpt from source |
| `event_id_start` | string | First event in excerpt |
| `event_id_end` | string | Last event in excerpt |
| `timestamp_ms` | int64 | Timestamp of first event |
| `source` | string | Creator (e.g., "segment_summarizer") |

---

## Health Check

The service exposes a standard gRPC health check endpoint.

```bash
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

Response:
```json
{
  "status": "SERVING"
}
```

---

## Reflection

The service supports gRPC reflection for API discovery.

```bash
# List services
grpcurl -plaintext localhost:50051 list

# Describe service
grpcurl -plaintext localhost:50051 describe memory.MemoryService

# Describe message type
grpcurl -plaintext localhost:50051 describe memory.Event
```
