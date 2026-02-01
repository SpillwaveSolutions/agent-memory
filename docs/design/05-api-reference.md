# API Reference

This document provides comprehensive API documentation for the Agent Memory system, including gRPC service definitions, CLI commands, and client library usage.

## Table of Contents

1. [gRPC Service Overview](#grpc-service-overview)
2. [Ingestion API](#ingestion-api)
3. [Query APIs](#query-apis)
4. [Scheduler APIs](#scheduler-apis)
5. [CLI Reference](#cli-reference)
6. [Client Library](#client-library)

---

## gRPC Service Overview

### MemoryService Definition

The `MemoryService` is the primary gRPC interface for the Agent Memory system. It provides event ingestion, TOC navigation, event retrieval, and scheduler management.

**Proto file location**: `proto/memory.proto`

**Package**: `memory`

```protobuf
service MemoryService {
    // Ingestion
    rpc IngestEvent(IngestEventRequest) returns (IngestEventResponse);

    // TOC Navigation (QRY-01 through QRY-03)
    rpc GetTocRoot(GetTocRootRequest) returns (GetTocRootResponse);
    rpc GetNode(GetNodeRequest) returns (GetNodeResponse);
    rpc BrowseToc(BrowseTocRequest) returns (BrowseTocResponse);

    // Event Retrieval (QRY-04 through QRY-05)
    rpc GetEvents(GetEventsRequest) returns (GetEventsResponse);
    rpc ExpandGrip(ExpandGripRequest) returns (ExpandGripResponse);

    // Scheduler Management (SCHED-05)
    rpc GetSchedulerStatus(GetSchedulerStatusRequest) returns (GetSchedulerStatusResponse);
    rpc PauseJob(PauseJobRequest) returns (PauseJobResponse);
    rpc ResumeJob(ResumeJobRequest) returns (ResumeJobResponse);
}
```

### Connection Details

| Setting | Value |
|---------|-------|
| Default Endpoint | `http://[::1]:50051` |
| Protocol | gRPC (HTTP/2) |
| Port | 50051 (configurable) |
| TLS | Not required (local-only) |
| Authentication | None (designed for local use) |

**Connection Example**:

```bash
# Using grpcurl
grpcurl -plaintext localhost:50051 list

# Using grpcurl with IPv6
grpcurl -plaintext '[::1]:50051' list
```

### Service Discovery

The service supports gRPC reflection for runtime API discovery:

```bash
# List all services
grpcurl -plaintext localhost:50051 list

# Describe the MemoryService
grpcurl -plaintext localhost:50051 describe memory.MemoryService

# Describe a specific message type
grpcurl -plaintext localhost:50051 describe memory.Event
```

---

## Ingestion API

### IngestEvent

Ingests a conversation event into storage. This is the primary mechanism for capturing conversation data.

**Behavior**:
- Events are persisted to RocksDB with time-prefixed keys
- Idempotent: returns `created=false` if `event_id` already exists
- Atomically writes an outbox entry for async index updates

#### Request: IngestEventRequest

```protobuf
message IngestEventRequest {
    Event event = 1;  // Required: The event to ingest
}
```

#### Event Message

```protobuf
message Event {
    string event_id = 1;                    // Required: ULID string (26 chars)
    string session_id = 2;                  // Required: Session identifier
    int64 timestamp_ms = 3;                 // Required: Unix epoch milliseconds
    EventType event_type = 4;               // Required: Type of event
    EventRole role = 5;                     // Required: Author role
    string text = 6;                        // Event content/text
    map<string, string> metadata = 7;       // Optional: Additional metadata
}
```

**Field Specifications**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `event_id` | string | Yes | Client-generated ULID (26 characters). Must be unique. |
| `session_id` | string | Yes | Session identifier. Any non-empty string. |
| `timestamp_ms` | int64 | Yes | Event timestamp in milliseconds since Unix epoch. Used for ordering. |
| `event_type` | EventType | Yes | Type of conversation event. See enum values below. |
| `role` | EventRole | Yes | Role of the message author. See enum values below. |
| `text` | string | No | Event content. Can be empty for session markers. |
| `metadata` | map | No | Key-value pairs for additional context (tool names, file paths, etc.) |

#### EventType Enum

```protobuf
enum EventType {
    EVENT_TYPE_UNSPECIFIED = 0;      // Default/invalid
    EVENT_TYPE_SESSION_START = 1;    // Session begins
    EVENT_TYPE_USER_MESSAGE = 2;     // User prompt/input
    EVENT_TYPE_ASSISTANT_MESSAGE = 3; // AI response
    EVENT_TYPE_TOOL_RESULT = 4;      // Tool execution output
    EVENT_TYPE_ASSISTANT_STOP = 5;   // AI stops generating
    EVENT_TYPE_SUBAGENT_START = 6;   // Subagent spawned
    EVENT_TYPE_SUBAGENT_STOP = 7;    // Subagent completes
    EVENT_TYPE_SESSION_END = 8;      // Session ends
}
```

| Value | Integer | Description | Typical Role |
|-------|---------|-------------|--------------|
| `SESSION_START` | 1 | Conversation session begins | System |
| `USER_MESSAGE` | 2 | User prompt/input | User |
| `ASSISTANT_MESSAGE` | 3 | AI response | Assistant |
| `TOOL_RESULT` | 4 | Tool execution output | Tool |
| `ASSISTANT_STOP` | 5 | AI stops generating | Assistant |
| `SUBAGENT_START` | 6 | Subagent spawned | System |
| `SUBAGENT_STOP` | 7 | Subagent completes | System |
| `SESSION_END` | 8 | Session ends | System |

#### EventRole Enum

```protobuf
enum EventRole {
    EVENT_ROLE_UNSPECIFIED = 0;  // Default (maps to User)
    EVENT_ROLE_USER = 1;         // Human user
    EVENT_ROLE_ASSISTANT = 2;    // AI assistant
    EVENT_ROLE_SYSTEM = 3;       // System message
    EVENT_ROLE_TOOL = 4;         // Tool output
}
```

| Value | Integer | Description |
|-------|---------|-------------|
| `USER` | 1 | Human user input |
| `ASSISTANT` | 2 | AI assistant responses |
| `SYSTEM` | 3 | System-generated events |
| `TOOL` | 4 | Tool execution results |

#### Response: IngestEventResponse

```protobuf
message IngestEventResponse {
    string event_id = 1;  // The event_id that was stored
    bool created = 2;     // True if new, false if duplicate (idempotent)
}
```

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | string | The stored event ID (echoes input) |
| `created` | bool | `true` if event was newly created, `false` if already existed |

#### Error Codes

| gRPC Code | Condition | Description |
|-----------|-----------|-------------|
| `INVALID_ARGUMENT` | Missing event | The `event` field is null/missing |
| `INVALID_ARGUMENT` | Empty event_id | The `event_id` field is empty |
| `INVALID_ARGUMENT` | Empty session_id | The `session_id` field is empty |
| `INVALID_ARGUMENT` | Invalid timestamp | The `timestamp_ms` is out of valid range |
| `INTERNAL` | Storage failure | Failed to write to RocksDB |

#### Example: Ingest a User Message

```bash
grpcurl -plaintext -d '{
  "event": {
    "event_id": "01HXYZABC123DEF456GHI789",
    "session_id": "session-2026-01-30-001",
    "timestamp_ms": 1738281600000,
    "event_type": 2,
    "role": 1,
    "text": "What is Rust and why should I use it?"
  }
}' localhost:50051 memory.MemoryService/IngestEvent
```

**Response**:
```json
{
  "eventId": "01HXYZABC123DEF456GHI789",
  "created": true
}
```

#### Example: Ingest a Tool Result with Metadata

```bash
grpcurl -plaintext -d '{
  "event": {
    "event_id": "01HXYZDEF456GHI789JKL012",
    "session_id": "session-2026-01-30-001",
    "timestamp_ms": 1738281601000,
    "event_type": 4,
    "role": 4,
    "text": "File contents: fn main() { println!(\"Hello\"); }",
    "metadata": {
      "tool_name": "Read",
      "file_path": "/src/main.rs"
    }
  }
}' localhost:50051 memory.MemoryService/IngestEvent
```

#### Example: Idempotent Re-ingestion

```bash
# First ingestion - created=true
grpcurl -plaintext -d '{
  "event": {
    "event_id": "01HXYZABC123",
    "session_id": "session-1",
    "timestamp_ms": 1738281600000,
    "event_type": 2,
    "role": 1,
    "text": "Hello"
  }
}' localhost:50051 memory.MemoryService/IngestEvent
# Response: {"eventId": "01HXYZABC123", "created": true}

# Second ingestion with same event_id - created=false
grpcurl -plaintext -d '{
  "event": {
    "event_id": "01HXYZABC123",
    "session_id": "session-1",
    "timestamp_ms": 1738281600000,
    "event_type": 2,
    "role": 1,
    "text": "Hello"
  }
}' localhost:50051 memory.MemoryService/IngestEvent
# Response: {"eventId": "01HXYZABC123", "created": false}
```

---

## Query APIs

### GetTocRoot

Returns the root-level TOC nodes (year level). These represent the top of the hierarchical Table of Contents.

#### Request: GetTocRootRequest

```protobuf
message GetTocRootRequest {}
```

No parameters required.

#### Response: GetTocRootResponse

```protobuf
message GetTocRootResponse {
    repeated TocNode nodes = 1;  // Year-level nodes, sorted by time descending
}
```

| Field | Type | Description |
|-------|------|-------------|
| `nodes` | TocNode[] | Array of year-level nodes, most recent first |

#### TocNode Message

```protobuf
message TocNode {
    string node_id = 1;              // e.g., "toc:year:2026"
    TocLevel level = 2;              // Hierarchy level
    string title = 3;                // Display title, e.g., "2026"
    optional string summary = 4;     // Summary text (may be empty)
    repeated TocBullet bullets = 5;  // Summary bullet points
    repeated string keywords = 6;    // Searchable keywords
    repeated string child_node_ids = 7; // Child node IDs
    int64 start_time_ms = 8;         // Period start timestamp
    int64 end_time_ms = 9;           // Period end timestamp
    int32 version = 10;              // Version number
}
```

| Field | Type | Description |
|-------|------|-------------|
| `node_id` | string | Unique identifier. Format: `toc:{level}:{identifier}` |
| `level` | TocLevel | Hierarchy level (Year, Month, Week, Day, Segment) |
| `title` | string | Human-readable title for display |
| `summary` | string | Optional AI-generated summary of the period |
| `bullets` | TocBullet[] | Key points with provenance links |
| `keywords` | string[] | Searchable tags extracted from content |
| `child_node_ids` | string[] | IDs of child nodes in the hierarchy |
| `start_time_ms` | int64 | Start of the time period (Unix ms) |
| `end_time_ms` | int64 | End of the time period (Unix ms) |
| `version` | int32 | Monotonically increasing version number |

#### TocLevel Enum

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

| Value | Integer | Node ID Example | Title Example |
|-------|---------|-----------------|---------------|
| `YEAR` | 1 | `toc:year:2026` | "2026" |
| `MONTH` | 2 | `toc:month:2026-01` | "January 2026" |
| `WEEK` | 3 | `toc:week:2026-W05` | "Week 5, 2026" |
| `DAY` | 4 | `toc:day:2026-01-30` | "January 30, 2026" |
| `SEGMENT` | 5 | `toc:segment:01HXYZ` | "Rust discussion" |

#### TocBullet Message

```protobuf
message TocBullet {
    string text = 1;              // Bullet point text
    repeated string grip_ids = 2; // IDs of supporting grips
}
```

#### Example: Get Root Nodes

```bash
grpcurl -plaintext localhost:50051 memory.MemoryService/GetTocRoot
```

**Response**:
```json
{
  "nodes": [
    {
      "nodeId": "toc:year:2026",
      "level": 1,
      "title": "2026",
      "summary": "Discussions on Rust, Python, and system design",
      "bullets": [
        {
          "text": "Rust memory safety patterns",
          "gripIds": ["grip:01HXYZ123"]
        },
        {
          "text": "Python async programming",
          "gripIds": ["grip:01HXYZA456"]
        }
      ],
      "keywords": ["rust", "python", "async", "memory-safety"],
      "childNodeIds": ["toc:month:2026-01"],
      "startTimeMs": "1704067200000",
      "endTimeMs": "1735689599000",
      "version": 3
    },
    {
      "nodeId": "toc:year:2025",
      "level": 1,
      "title": "2025",
      "childNodeIds": ["toc:month:2025-12", "toc:month:2025-11"],
      "startTimeMs": "1672531200000",
      "endTimeMs": "1704067199000",
      "version": 1
    }
  ]
}
```

---

### GetNode

Retrieves a specific TOC node by its ID, including all summary information and child references.

#### Request: GetNodeRequest

```protobuf
message GetNodeRequest {
    string node_id = 1;  // Required: Node ID to retrieve
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `node_id` | string | Yes | The node ID (e.g., "toc:year:2026", "toc:month:2026-01") |

#### Response: GetNodeResponse

```protobuf
message GetNodeResponse {
    optional TocNode node = 1;  // The node, or null if not found
}
```

| Field | Type | Description |
|-------|------|-------------|
| `node` | TocNode | The requested node, or null/absent if not found |

#### Error Codes

| gRPC Code | Condition | Description |
|-----------|-----------|-------------|
| `INVALID_ARGUMENT` | Empty node_id | The `node_id` field is empty |
| `INTERNAL` | Storage failure | Failed to read from RocksDB |

#### Example: Get a Year Node

```bash
grpcurl -plaintext -d '{"node_id": "toc:year:2026"}' \
  localhost:50051 memory.MemoryService/GetNode
```

**Response**:
```json
{
  "node": {
    "nodeId": "toc:year:2026",
    "level": 1,
    "title": "2026",
    "summary": "System design and programming discussions",
    "childNodeIds": ["toc:month:2026-01"],
    "startTimeMs": "1704067200000",
    "endTimeMs": "1735689599000",
    "version": 2
  }
}
```

#### Example: Get a Segment Node

```bash
grpcurl -plaintext -d '{"node_id": "toc:segment:01HXYZABC123"}' \
  localhost:50051 memory.MemoryService/GetNode
```

#### Example: Node Not Found

```bash
grpcurl -plaintext -d '{"node_id": "toc:year:1999"}' \
  localhost:50051 memory.MemoryService/GetNode
```

**Response**:
```json
{}
```

---

### BrowseToc

Browses child nodes of a TOC node with pagination support. Use this to navigate down the hierarchy.

#### Request: BrowseTocRequest

```protobuf
message BrowseTocRequest {
    string parent_id = 1;                    // Required: Parent node ID
    int32 limit = 2;                         // Max results (default: 20)
    optional string continuation_token = 3; // Token for pagination
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `parent_id` | string | Yes | - | ID of the parent node |
| `limit` | int32 | No | 20 | Maximum children to return (1-100) |
| `continuation_token` | string | No | - | Token from previous response for next page |

#### Response: BrowseTocResponse

```protobuf
message BrowseTocResponse {
    repeated TocNode children = 1;           // Child nodes
    optional string continuation_token = 2;  // Token for next page
    bool has_more = 3;                       // True if more results available
}
```

| Field | Type | Description |
|-------|------|-------------|
| `children` | TocNode[] | Array of child nodes |
| `continuation_token` | string | Token to pass for the next page (null if no more) |
| `has_more` | bool | `true` if additional results exist beyond this page |

#### Pagination Pattern

The `continuation_token` is an opaque string representing the offset into the result set. Pass it unchanged to the next request.

```
Page 1: BrowseTocRequest { parent_id: "toc:year:2026", limit: 5 }
        -> Response has continuation_token: "5", has_more: true

Page 2: BrowseTocRequest { parent_id: "toc:year:2026", limit: 5, continuation_token: "5" }
        -> Response has continuation_token: "10", has_more: true

Page 3: BrowseTocRequest { parent_id: "toc:year:2026", limit: 5, continuation_token: "10" }
        -> Response has continuation_token: null, has_more: false
```

#### Error Codes

| gRPC Code | Condition | Description |
|-----------|-----------|-------------|
| `INVALID_ARGUMENT` | Empty parent_id | The `parent_id` field is empty |
| `INTERNAL` | Storage failure | Failed to read from RocksDB |

#### Example: Browse Months in a Year

```bash
grpcurl -plaintext -d '{
  "parent_id": "toc:year:2026",
  "limit": 3
}' localhost:50051 memory.MemoryService/BrowseToc
```

**Response**:
```json
{
  "children": [
    {
      "nodeId": "toc:month:2026-01",
      "level": 2,
      "title": "January 2026",
      "childNodeIds": ["toc:week:2026-W01", "toc:week:2026-W02"],
      "startTimeMs": "1704067200000",
      "endTimeMs": "1706745599000"
    },
    {
      "nodeId": "toc:month:2026-02",
      "level": 2,
      "title": "February 2026"
    }
  ],
  "continuationToken": "2",
  "hasMore": true
}
```

#### Example: Get Next Page

```bash
grpcurl -plaintext -d '{
  "parent_id": "toc:year:2026",
  "limit": 3,
  "continuation_token": "2"
}' localhost:50051 memory.MemoryService/BrowseToc
```

---

### GetEvents

Retrieves raw events within a time range. Useful for displaying conversation history or exporting data.

#### Request: GetEventsRequest

```protobuf
message GetEventsRequest {
    int64 from_timestamp_ms = 1;  // Start time (inclusive)
    int64 to_timestamp_ms = 2;    // End time (inclusive)
    int32 limit = 3;              // Max events (default: 50)
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `from_timestamp_ms` | int64 | Yes | - | Start of time range (inclusive), Unix ms |
| `to_timestamp_ms` | int64 | Yes | - | End of time range (inclusive), Unix ms |
| `limit` | int32 | No | 50 | Maximum events to return |

#### Response: GetEventsResponse

```protobuf
message GetEventsResponse {
    repeated Event events = 1;  // Events in range, sorted by timestamp
    bool has_more = 2;          // True if more events beyond limit
}
```

| Field | Type | Description |
|-------|------|-------------|
| `events` | Event[] | Events within the time range, ordered by timestamp |
| `has_more` | bool | `true` if more events exist beyond the limit |

#### Example: Get Events from Last Hour

```bash
# Calculate timestamps (example: last hour)
# from: now - 1 hour = 1738278000000
# to: now = 1738281600000

grpcurl -plaintext -d '{
  "from_timestamp_ms": 1738278000000,
  "to_timestamp_ms": 1738281600000,
  "limit": 100
}' localhost:50051 memory.MemoryService/GetEvents
```

**Response**:
```json
{
  "events": [
    {
      "eventId": "01HXYZABC123",
      "sessionId": "session-001",
      "timestampMs": "1738278100000",
      "eventType": 2,
      "role": 1,
      "text": "What is Rust?"
    },
    {
      "eventId": "01HXYZABC124",
      "sessionId": "session-001",
      "timestampMs": "1738278150000",
      "eventType": 3,
      "role": 2,
      "text": "Rust is a systems programming language..."
    }
  ],
  "hasMore": false
}
```

#### Example: Paginating Through Large Results

```bash
# First batch
grpcurl -plaintext -d '{
  "from_timestamp_ms": 1738000000000,
  "to_timestamp_ms": 1738281600000,
  "limit": 50
}' localhost:50051 memory.MemoryService/GetEvents
# If has_more=true, use the last event's timestamp as from_timestamp_ms for next batch
```

---

### ExpandGrip

Expands a grip (provenance anchor) to show the context events around the excerpt. Grips link summary bullets back to source conversations.

#### Request: ExpandGripRequest

```protobuf
message ExpandGripRequest {
    string grip_id = 1;              // Required: Grip ID to expand
    optional int32 events_before = 2; // Events before excerpt (default: 3)
    optional int32 events_after = 3;  // Events after excerpt (default: 3)
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `grip_id` | string | Yes | - | The grip ID to expand |
| `events_before` | int32 | No | 3 | Number of context events before the excerpt |
| `events_after` | int32 | No | 3 | Number of context events after the excerpt |

#### Grip Message

```protobuf
message Grip {
    string grip_id = 1;        // Unique identifier
    string excerpt = 2;        // Text excerpt from source
    string event_id_start = 3; // First event in excerpt
    string event_id_end = 4;   // Last event in excerpt
    int64 timestamp_ms = 5;    // Timestamp of first event
    string source = 6;         // Creator (e.g., "segment_summarizer")
}
```

| Field | Type | Description |
|-------|------|-------------|
| `grip_id` | string | Unique identifier for the grip |
| `excerpt` | string | Text excerpt linking to the summary |
| `event_id_start` | string | Event ID where excerpt begins |
| `event_id_end` | string | Event ID where excerpt ends |
| `timestamp_ms` | int64 | Timestamp of the first event |
| `source` | string | Component that created this grip |

#### Response: ExpandGripResponse

```protobuf
message ExpandGripResponse {
    optional Grip grip = 1;           // The grip (null if not found)
    repeated Event events_before = 2; // Context before excerpt
    repeated Event excerpt_events = 3; // Events in the excerpt
    repeated Event events_after = 4;  // Context after excerpt
}
```

| Field | Type | Description |
|-------|------|-------------|
| `grip` | Grip | The grip metadata, or null if not found |
| `events_before` | Event[] | Context events preceding the excerpt |
| `excerpt_events` | Event[] | The events that make up the excerpt |
| `events_after` | Event[] | Context events following the excerpt |

#### Error Codes

| gRPC Code | Condition | Description |
|-----------|-----------|-------------|
| `INVALID_ARGUMENT` | Empty grip_id | The `grip_id` field is empty |
| `INTERNAL` | Storage failure | Failed to read from RocksDB |

#### Example: Expand a Grip

```bash
grpcurl -plaintext -d '{
  "grip_id": "grip:01HXYZABC123",
  "events_before": 2,
  "events_after": 2
}' localhost:50051 memory.MemoryService/ExpandGrip
```

**Response**:
```json
{
  "grip": {
    "gripId": "grip:01HXYZABC123",
    "excerpt": "Rust's ownership system prevents data races...",
    "eventIdStart": "01HXYZEVENT001",
    "eventIdEnd": "01HXYZEVENT003",
    "timestampMs": "1738281600000",
    "source": "segment_summarizer"
  },
  "eventsBefore": [
    {
      "eventId": "01HXYZEVENT000",
      "sessionId": "session-001",
      "timestampMs": "1738281595000",
      "eventType": 2,
      "role": 1,
      "text": "Tell me about Rust safety"
    }
  ],
  "excerptEvents": [
    {
      "eventId": "01HXYZEVENT001",
      "sessionId": "session-001",
      "timestampMs": "1738281600000",
      "eventType": 3,
      "role": 2,
      "text": "Rust's ownership system prevents data races..."
    },
    {
      "eventId": "01HXYZEVENT002",
      "sessionId": "session-001",
      "timestampMs": "1738281605000",
      "eventType": 2,
      "role": 1,
      "text": "How does the borrow checker work?"
    },
    {
      "eventId": "01HXYZEVENT003",
      "sessionId": "session-001",
      "timestampMs": "1738281610000",
      "eventType": 3,
      "role": 2,
      "text": "The borrow checker enforces..."
    }
  ],
  "eventsAfter": [
    {
      "eventId": "01HXYZEVENT004",
      "sessionId": "session-001",
      "timestampMs": "1738281615000",
      "eventType": 2,
      "role": 1,
      "text": "What about lifetimes?"
    }
  ]
}
```

#### Example: Grip Not Found

```bash
grpcurl -plaintext -d '{"grip_id": "nonexistent"}' \
  localhost:50051 memory.MemoryService/ExpandGrip
```

**Response**:
```json
{
  "eventsBefore": [],
  "excerptEvents": [],
  "eventsAfter": []
}
```

---

## Scheduler APIs

The scheduler manages background jobs for TOC rollup, segment summarization, and outbox processing.

### GetSchedulerStatus

Returns the current scheduler state and status of all registered jobs.

#### Request: GetSchedulerStatusRequest

```protobuf
message GetSchedulerStatusRequest {}
```

No parameters required.

#### Response: GetSchedulerStatusResponse

```protobuf
message GetSchedulerStatusResponse {
    bool scheduler_running = 1;       // Whether scheduler loop is active
    repeated JobStatusProto jobs = 2; // Status of all registered jobs
}
```

| Field | Type | Description |
|-------|------|-------------|
| `scheduler_running` | bool | `true` if the scheduler tick loop is active |
| `jobs` | JobStatusProto[] | Array of job status objects |

#### JobStatusProto Message

```protobuf
message JobStatusProto {
    string job_name = 1;         // Job identifier
    string cron_expr = 2;        // Cron schedule expression
    int64 last_run_ms = 3;       // Last execution time (0 if never run)
    int64 last_duration_ms = 4;  // Last execution duration
    JobResultStatus last_result = 5; // Last execution result
    optional string last_error = 6;  // Error message if failed
    int64 next_run_ms = 7;       // Next scheduled run time
    uint64 run_count = 8;        // Total successful runs
    uint64 error_count = 9;      // Total failed runs
    bool is_running = 10;        // Currently executing
    bool is_paused = 11;         // Paused by user
}
```

| Field | Type | Description |
|-------|------|-------------|
| `job_name` | string | Unique job identifier (e.g., "hourly-rollup") |
| `cron_expr` | string | Cron expression (e.g., "0 0 * * * *" for hourly) |
| `last_run_ms` | int64 | Timestamp of last execution (0 if never run) |
| `last_duration_ms` | int64 | Duration of last execution in milliseconds |
| `last_result` | JobResultStatus | Result of last execution |
| `last_error` | string | Error message if last execution failed |
| `next_run_ms` | int64 | Timestamp of next scheduled run |
| `run_count` | uint64 | Cumulative successful executions |
| `error_count` | uint64 | Cumulative failed executions |
| `is_running` | bool | `true` if job is currently executing |
| `is_paused` | bool | `true` if job is paused |

#### JobResultStatus Enum

```protobuf
enum JobResultStatus {
    JOB_RESULT_STATUS_UNSPECIFIED = 0;
    JOB_RESULT_STATUS_SUCCESS = 1;
    JOB_RESULT_STATUS_FAILED = 2;
    JOB_RESULT_STATUS_SKIPPED = 3;
}
```

| Value | Integer | Description |
|-------|---------|-------------|
| `UNSPECIFIED` | 0 | Job has never run |
| `SUCCESS` | 1 | Last run completed successfully |
| `FAILED` | 2 | Last run failed with an error |
| `SKIPPED` | 3 | Last run was skipped (e.g., overlap policy) |

#### Example: Get Scheduler Status

```bash
grpcurl -plaintext localhost:50051 memory.MemoryService/GetSchedulerStatus
```

**Response**:
```json
{
  "schedulerRunning": true,
  "jobs": [
    {
      "jobName": "hourly-rollup",
      "cronExpr": "0 0 * * * *",
      "lastRunMs": "1738278000000",
      "lastDurationMs": "1250",
      "lastResult": 1,
      "nextRunMs": "1738281600000",
      "runCount": "24",
      "errorCount": "0",
      "isRunning": false,
      "isPaused": false
    },
    {
      "jobName": "segment-summarizer",
      "cronExpr": "0 */15 * * * *",
      "lastRunMs": "1738280700000",
      "lastDurationMs": "3500",
      "lastResult": 1,
      "nextRunMs": "1738281600000",
      "runCount": "96",
      "errorCount": "2",
      "isRunning": false,
      "isPaused": false
    },
    {
      "jobName": "outbox-processor",
      "cronExpr": "0 * * * * *",
      "lastRunMs": "1738281540000",
      "lastDurationMs": "150",
      "lastResult": 1,
      "nextRunMs": "1738281600000",
      "runCount": "1440",
      "errorCount": "0",
      "isRunning": true,
      "isPaused": false
    }
  ]
}
```

---

### PauseJob

Pauses a scheduled job. The job will skip execution when its scheduled time arrives.

#### Request: PauseJobRequest

```protobuf
message PauseJobRequest {
    string job_name = 1;  // Required: Job name to pause
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `job_name` | string | Yes | The name of the job to pause |

#### Response: PauseJobResponse

```protobuf
message PauseJobResponse {
    bool success = 1;         // True if job was paused
    optional string error = 2; // Error message if failed
}
```

| Field | Type | Description |
|-------|------|-------------|
| `success` | bool | `true` if the job was successfully paused |
| `error` | string | Error description if `success` is `false` |

#### Example: Pause a Job

```bash
grpcurl -plaintext -d '{"job_name": "hourly-rollup"}' \
  localhost:50051 memory.MemoryService/PauseJob
```

**Response (success)**:
```json
{
  "success": true
}
```

**Response (job not found)**:
```json
{
  "success": false,
  "error": "Job not found: hourly-rollup"
}
```

---

### ResumeJob

Resumes a paused job. The job will execute at its next scheduled time.

#### Request: ResumeJobRequest

```protobuf
message ResumeJobRequest {
    string job_name = 1;  // Required: Job name to resume
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `job_name` | string | Yes | The name of the job to resume |

#### Response: ResumeJobResponse

```protobuf
message ResumeJobResponse {
    bool success = 1;         // True if job was resumed
    optional string error = 2; // Error message if failed
}
```

| Field | Type | Description |
|-------|------|-------------|
| `success` | bool | `true` if the job was successfully resumed |
| `error` | string | Error description if `success` is `false` |

#### Example: Resume a Job

```bash
grpcurl -plaintext -d '{"job_name": "hourly-rollup"}' \
  localhost:50051 memory.MemoryService/ResumeJob
```

**Response**:
```json
{
  "success": true
}
```

---

## CLI Reference

The `memory-daemon` binary provides a command-line interface for managing the daemon and querying data.

### Global Options

```
memory-daemon [OPTIONS] <COMMAND>

Options:
  -c, --config <PATH>    Path to config file (default: ~/.config/agent-memory/config.toml)
  -l, --log-level <LEVEL>  Log level: trace, debug, info, warn, error
  -h, --help             Print help
  -V, --version          Print version
```

### Daemon Management

#### start

Start the memory daemon.

```bash
memory-daemon start [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `--foreground` | `-f` | Run in foreground (don't daemonize) |
| `--port <PORT>` | `-p` | Override gRPC port (default: 50051) |
| `--db-path <PATH>` | | Override database path |

**Examples**:

```bash
# Start as background daemon
memory-daemon start

# Start in foreground for debugging
memory-daemon start --foreground

# Start on custom port
memory-daemon start -p 9999

# Start with custom database location
memory-daemon start --db-path /var/lib/agent-memory/db
```

#### stop

Stop the running daemon.

```bash
memory-daemon stop
```

**Example**:

```bash
memory-daemon stop
# Output: Daemon stopped
```

#### status

Show daemon status.

```bash
memory-daemon status
```

**Example Output**:

```
Daemon Status: Running
  PID: 12345
  Port: 50051
  Uptime: 2h 15m 30s
  Events stored: 15,234
  Database size: 45.2 MB
```

### Query Subcommands

All query commands connect to the gRPC service. Use `-e` to specify a custom endpoint.

```bash
memory-daemon query [OPTIONS] <SUBCOMMAND>

Options:
  -e, --endpoint <URL>  gRPC endpoint (default: http://[::1]:50051)
```

#### query root

List root TOC nodes (year level).

```bash
memory-daemon query root
```

**Example Output**:

```
TOC Root Nodes:
  - toc:year:2026 "2026" (12 children)
  - toc:year:2025 "2025" (8 children)
```

#### query node

Get a specific TOC node.

```bash
memory-daemon query node <NODE_ID>
```

**Examples**:

```bash
memory-daemon query node toc:year:2026
memory-daemon query node toc:month:2026-01
memory-daemon query node toc:segment:01HXYZABC123
```

**Example Output**:

```
Node: toc:year:2026
  Title: 2026
  Level: Year
  Summary: Programming discussions covering Rust, Python, and system design
  Keywords: rust, python, async, memory-safety
  Children: 12
  Time Range: 2026-01-01 to 2026-12-31
  Version: 3
```

#### query browse

Browse children of a node with pagination.

```bash
memory-daemon query browse <PARENT_ID> [OPTIONS]
```

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--limit <N>` | `-l` | 20 | Maximum results per page |
| `--token <TOKEN>` | `-t` | - | Continuation token for next page |

**Examples**:

```bash
# Browse months in 2026
memory-daemon query browse toc:year:2026

# Limit to 5 results
memory-daemon query browse toc:year:2026 -l 5

# Get next page
memory-daemon query browse toc:year:2026 -l 5 -t "5"
```

**Example Output**:

```
Children of toc:year:2026:
  1. toc:month:2026-01 "January 2026"
  2. toc:month:2026-02 "February 2026"
  3. toc:month:2026-03 "March 2026"

Page 1 of 4 (has_more: true, next_token: "3")
```

#### query events

Get events in a time range.

```bash
memory-daemon query events --from <TIMESTAMP_MS> --to <TIMESTAMP_MS> [OPTIONS]
```

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--from <MS>` | | Required | Start timestamp (Unix milliseconds) |
| `--to <MS>` | | Required | End timestamp (Unix milliseconds) |
| `--limit <N>` | `-l` | 50 | Maximum events to return |

**Examples**:

```bash
# Get events from a specific hour
memory-daemon query events --from 1738278000000 --to 1738281600000

# Limit results
memory-daemon query events --from 1738278000000 --to 1738281600000 -l 10
```

**Example Output**:

```
Events (1738278000000 - 1738281600000):
  1. 01HXYZABC001 [USER] 2026-01-30 10:00:00
     "What is Rust?"
  2. 01HXYZABC002 [ASSISTANT] 2026-01-30 10:00:05
     "Rust is a systems programming language..."
  3. 01HXYZABC003 [USER] 2026-01-30 10:00:15
     "How does ownership work?"

Total: 3 events (has_more: false)
```

#### query expand

Expand a grip to show context.

```bash
memory-daemon query expand <GRIP_ID> [OPTIONS]
```

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--before <N>` | | 3 | Events before excerpt |
| `--after <N>` | | 3 | Events after excerpt |

**Examples**:

```bash
memory-daemon query expand grip:01HXYZABC123
memory-daemon query expand grip:01HXYZABC123 --before 5 --after 5
```

**Example Output**:

```
Grip: grip:01HXYZABC123
  Excerpt: "Rust's ownership system prevents data races..."
  Source: segment_summarizer
  Time: 2026-01-30 10:00:00

Context:
  --- BEFORE ---
  [USER] What about memory safety?

  --- EXCERPT ---
  [ASSISTANT] Rust's ownership system prevents data races...
  [USER] How does the borrow checker work?
  [ASSISTANT] The borrow checker enforces...

  --- AFTER ---
  [USER] What about lifetimes?
```

### Admin Subcommands

Administrative commands for database maintenance.

```bash
memory-daemon admin [OPTIONS] <SUBCOMMAND>

Options:
  --db-path <PATH>  Database path (default from config)
```

#### admin stats

Show database statistics.

```bash
memory-daemon admin stats
```

**Example Output**:

```
Database Statistics:
  Path: /home/user/.local/share/agent-memory/db
  Size: 45.2 MB

  Column Families:
    events: 15,234 entries, 38.1 MB
    toc: 156 entries, 2.4 MB
    grips: 892 entries, 4.1 MB
    outbox: 12 entries, 0.6 MB

  RocksDB Info:
    Compaction: 3 pending
    WAL: 2.1 MB
```

#### admin compact

Trigger RocksDB compaction.

```bash
memory-daemon admin compact [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--cf <NAME>` | Compact only specific column family |

**Examples**:

```bash
# Compact all column families
memory-daemon admin compact

# Compact only events
memory-daemon admin compact --cf events
```

#### admin rebuild-toc

Rebuild TOC from raw events.

```bash
memory-daemon admin rebuild-toc [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--from-date <YYYY-MM-DD>` | Start from this date |
| `--dry-run` | Show what would be done without changes |

**Examples**:

```bash
# Preview rebuild
memory-daemon admin rebuild-toc --dry-run

# Rebuild from January 2026
memory-daemon admin rebuild-toc --from-date 2026-01-01

# Full rebuild
memory-daemon admin rebuild-toc
```

### Scheduler Subcommands

Manage background jobs.

```bash
memory-daemon scheduler [OPTIONS] <SUBCOMMAND>

Options:
  -e, --endpoint <URL>  gRPC endpoint (default: http://[::1]:50051)
```

#### scheduler status

Show scheduler and job status.

```bash
memory-daemon scheduler status
```

**Example Output**:

```
Scheduler: Running

Jobs:
  hourly-rollup
    Schedule: 0 0 * * * * (hourly)
    Status: Active
    Last Run: 2026-01-30 10:00:00 (1.25s, SUCCESS)
    Next Run: 2026-01-30 11:00:00
    Stats: 24 runs, 0 errors

  segment-summarizer
    Schedule: 0 */15 * * * * (every 15 min)
    Status: Active
    Last Run: 2026-01-30 10:45:00 (3.5s, SUCCESS)
    Next Run: 2026-01-30 11:00:00
    Stats: 96 runs, 2 errors

  outbox-processor
    Schedule: 0 * * * * * (every minute)
    Status: RUNNING
    Last Run: 2026-01-30 10:59:00 (0.15s, SUCCESS)
    Next Run: 2026-01-30 11:00:00
    Stats: 1440 runs, 0 errors
```

#### scheduler pause

Pause a scheduled job.

```bash
memory-daemon scheduler pause <JOB_NAME>
```

**Example**:

```bash
memory-daemon scheduler pause hourly-rollup
# Output: Job 'hourly-rollup' paused
```

#### scheduler resume

Resume a paused job.

```bash
memory-daemon scheduler resume <JOB_NAME>
```

**Example**:

```bash
memory-daemon scheduler resume hourly-rollup
# Output: Job 'hourly-rollup' resumed
```

---

## Client Library

The `memory-client` crate provides a Rust client library for interacting with the memory daemon.

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
memory-client = { path = "../memory-client" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### MemoryClient

The primary client for connecting to the daemon.

#### Constants

```rust
pub const DEFAULT_ENDPOINT: &str = "http://[::1]:50051";
```

#### Connection

```rust
use memory_client::MemoryClient;

// Connect to default endpoint
let mut client = MemoryClient::connect_default().await?;

// Connect to custom endpoint
let mut client = MemoryClient::connect("http://localhost:9999").await?;
```

#### Ingesting Events

```rust
use memory_client::{MemoryClient, Event};
use memory_types::{EventType, EventRole};
use chrono::Utc;

let mut client = MemoryClient::connect_default().await?;

// Create an event
let event = Event::new(
    ulid::Ulid::new().to_string(),  // event_id
    "session-123".to_string(),       // session_id
    Utc::now(),                      // timestamp
    EventType::UserMessage,          // event_type
    EventRole::User,                 // role
    "What is Rust?".to_string(),     // text
);

// Ingest single event
let (event_id, created) = client.ingest(event).await?;
println!("Ingested: {} (new: {})", event_id, created);

// Ingest batch
let events = vec![event1, event2, event3];
let created_count = client.ingest_batch(events).await?;
println!("Created {} new events", created_count);
```

#### Querying TOC

```rust
use memory_client::MemoryClient;

let mut client = MemoryClient::connect_default().await?;

// Get root nodes
let year_nodes = client.get_toc_root().await?;
for node in year_nodes {
    println!("{}: {}", node.node_id, node.title);
}

// Get specific node
if let Some(node) = client.get_node("toc:year:2026").await? {
    println!("Found: {}", node.title);
}

// Browse children with pagination
let result = client.browse_toc("toc:year:2026", 10, None).await?;
for child in result.children {
    println!("  {}: {}", child.node_id, child.title);
}
if result.has_more {
    // Get next page
    let next = client.browse_toc("toc:year:2026", 10, result.continuation_token).await?;
}
```

#### Retrieving Events

```rust
use memory_client::MemoryClient;
use chrono::Utc;

let mut client = MemoryClient::connect_default().await?;

// Get events from last hour
let now = Utc::now().timestamp_millis();
let hour_ago = now - 3600000;

let result = client.get_events(hour_ago, now, 100).await?;
for event in result.events {
    println!("[{}] {}", event.role, event.text);
}
```

#### Expanding Grips

```rust
use memory_client::MemoryClient;

let mut client = MemoryClient::connect_default().await?;

let result = client.expand_grip("grip:01HXYZABC123", Some(3), Some(3)).await?;

if let Some(grip) = result.grip {
    println!("Excerpt: {}", grip.excerpt);

    println!("Before:");
    for event in result.events_before {
        println!("  {}", event.text);
    }

    println!("Excerpt Events:");
    for event in result.excerpt_events {
        println!("  {}", event.text);
    }

    println!("After:");
    for event in result.events_after {
        println!("  {}", event.text);
    }
}
```

### HookEvent Mapping

For integrating with code agent hooks, use the hook mapping utilities.

#### HookEventType

```rust
pub enum HookEventType {
    SessionStart,      // Maps to EventType::SessionStart
    UserPromptSubmit,  // Maps to EventType::UserMessage
    AssistantResponse, // Maps to EventType::AssistantMessage
    ToolUse,           // Maps to EventType::ToolResult
    ToolResult,        // Maps to EventType::ToolResult
    Stop,              // Maps to EventType::SessionEnd
    SubagentStart,     // Maps to EventType::SubagentStart
    SubagentStop,      // Maps to EventType::SubagentStop
}
```

#### HookEvent Builder

```rust
use memory_client::{HookEvent, HookEventType, map_hook_event};
use std::collections::HashMap;

// Simple hook event
let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Hello!");
let event = map_hook_event(hook);

// With timestamp
let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Hello!")
    .with_timestamp(Utc::now());

// With tool name
let hook = HookEvent::new("session-1", HookEventType::ToolResult, "File contents")
    .with_tool_name("Read");

// With metadata
let mut metadata = HashMap::new();
metadata.insert("file_path".to_string(), "/src/main.rs".to_string());
let hook = HookEvent::new("session-1", HookEventType::ToolResult, "File contents")
    .with_tool_name("Read")
    .with_metadata(metadata);

// Convert and ingest
let event = map_hook_event(hook);
client.ingest(event).await?;
```

### Result Types

#### BrowseTocResult

```rust
pub struct BrowseTocResult {
    pub children: Vec<TocNode>,
    pub continuation_token: Option<String>,
    pub has_more: bool,
}
```

#### GetEventsResult

```rust
pub struct GetEventsResult {
    pub events: Vec<Event>,
    pub has_more: bool,
}
```

#### ExpandGripResult

```rust
pub struct ExpandGripResult {
    pub grip: Option<Grip>,
    pub events_before: Vec<Event>,
    pub excerpt_events: Vec<Event>,
    pub events_after: Vec<Event>,
}
```

### Error Handling

```rust
use memory_client::{ClientError, MemoryClient};

match MemoryClient::connect("http://localhost:50051").await {
    Ok(client) => println!("Connected"),
    Err(ClientError::Connection(e)) => eprintln!("Connection failed: {}", e),
    Err(ClientError::Rpc(status)) => eprintln!("RPC error: {}", status),
    Err(ClientError::Serialization(msg)) => eprintln!("Serialization error: {}", msg),
    Err(ClientError::InvalidEndpoint(msg)) => eprintln!("Invalid endpoint: {}", msg),
}
```

#### ClientError Variants

```rust
pub enum ClientError {
    /// Failed to connect to the daemon
    Connection(tonic::transport::Error),

    /// RPC call failed
    Rpc(tonic::Status),

    /// Serialization/deserialization failed
    Serialization(String),

    /// Invalid endpoint URL
    InvalidEndpoint(String),
}
```

### Complete Example

```rust
use memory_client::{MemoryClient, HookEvent, HookEventType, map_hook_event};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to daemon
    let mut client = MemoryClient::connect_default().await?;

    // Ingest a session start
    let start = HookEvent::new("session-001", HookEventType::SessionStart, "")
        .with_timestamp(Utc::now());
    let (id, _) = client.ingest(map_hook_event(start)).await?;
    println!("Session started: {}", id);

    // Ingest user message
    let user_msg = HookEvent::new(
        "session-001",
        HookEventType::UserPromptSubmit,
        "What is Rust?"
    );
    client.ingest(map_hook_event(user_msg)).await?;

    // Ingest assistant response
    let assistant_msg = HookEvent::new(
        "session-001",
        HookEventType::AssistantResponse,
        "Rust is a systems programming language focused on safety..."
    );
    client.ingest(map_hook_event(assistant_msg)).await?;

    // Query the TOC
    let roots = client.get_toc_root().await?;
    println!("TOC has {} year nodes", roots.len());

    // Get recent events
    let now = Utc::now().timestamp_millis();
    let events = client.get_events(now - 60000, now, 10).await?;
    println!("Found {} recent events", events.events.len());

    Ok(())
}
```
