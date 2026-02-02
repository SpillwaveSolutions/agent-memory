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

## Phase 10.5: Agentic TOC Search

### SearchNode

Search a specific TOC node for term matches.

**Request:**
```protobuf
message SearchNodeRequest {
    string node_id = 1;                    // Node to search
    string query = 2;                      // Search terms
    repeated SearchField fields = 3;       // Fields to search
    float min_score = 4;                   // Minimum match score (0.0-1.0, default 0.3)
}
```

**Response:**
```protobuf
message SearchNodeResponse {
    repeated SearchMatch matches = 1;      // Matching content with scores
    int32 total_matches = 2;               // Total number of matches
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty node_id or query
- `NOT_FOUND`: Node does not exist
- `INVALID_ARGUMENT`: min_score outside 0.0-1.0 range

**Example:**
```bash
grpcurl -plaintext -d '{
  "node_id": "toc:day:2026-01-15",
  "query": "rust async",
  "fields": ["SEARCH_FIELD_TITLE", "SEARCH_FIELD_SUMMARY", "SEARCH_FIELD_KEYWORDS"],
  "min_score": 0.5
}' localhost:50051 memory.MemoryService/SearchNode
```

---

### SearchChildren

Search children of a TOC node recursively.

**Request:**
```protobuf
message SearchChildrenRequest {
    string parent_id = 1;                  // Parent node
    string query = 2;                      // Search terms
    int32 max_depth = 3;                   // Max levels to search (default 3)
    int32 limit = 4;                       // Max results (default 20)
    float min_score = 5;                   // Minimum match score (default 0.3)
}
```

**Response:**
```protobuf
message SearchChildrenResponse {
    repeated NodeMatch matches = 1;        // Matching nodes with scores
    int32 nodes_searched = 2;              // Number of nodes searched
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty parent_id or query
- `NOT_FOUND`: Parent node does not exist
- `INVALID_ARGUMENT`: max_depth or limit less than 1

**Example:**
```bash
grpcurl -plaintext -d '{
  "parent_id": "toc:month:2026-01",
  "query": "memory optimization",
  "max_depth": 2,
  "limit": 10,
  "min_score": 0.4
}' localhost:50051 memory.MemoryService/SearchChildren
```

---

## Phase 11: BM25 Teleport

### GetTeleportStatus

Check BM25 index availability and stats.

**Request:**
```protobuf
message GetTeleportStatusRequest {}
```

**Response:**
```protobuf
message GetTeleportStatusResponse {
    bool available = 1;                    // True if index is ready
    int64 document_count = 2;              // Number of indexed documents
    int64 size_bytes = 3;                  // Index size in bytes
    int64 last_commit = 4;                 // Last commit timestamp (epoch ms)
}
```

**Example:**
```bash
grpcurl -plaintext localhost:50051 memory.MemoryService/GetTeleportStatus
```

---

### TeleportSearch

BM25 keyword search across indexed documents.

**Request:**
```protobuf
message TeleportSearchRequest {
    string query = 1;                      // Search query
    int32 limit = 2;                       // Max results (default 20)
    repeated DocType doc_types = 3;        // Filter by doc type (default both)
}
```

**Response:**
```protobuf
message TeleportSearchResponse {
    repeated TeleportResult results = 1;   // Matches with BM25 scores
    int64 query_time_ms = 2;               // Query execution time
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty query
- `UNAVAILABLE`: BM25 index not available

**Example:**
```bash
grpcurl -plaintext -d '{
  "query": "tokio async runtime",
  "limit": 10,
  "doc_types": ["DOC_TYPE_TOC_NODE", "DOC_TYPE_GRIP"]
}' localhost:50051 memory.MemoryService/TeleportSearch
```

---

## Phase 12: Vector Teleport

### GetVectorIndexStatus

Check vector index availability.

**Request:**
```protobuf
message GetVectorIndexStatusRequest {}
```

**Response:**
```protobuf
message GetVectorIndexStatusResponse {
    bool available = 1;                    // True if index is ready
    int64 vector_count = 2;                // Number of indexed vectors
    int32 dimension = 3;                   // Vector dimension (e.g., 384, 768)
    int64 size_bytes = 4;                  // Index size in bytes
}
```

**Example:**
```bash
grpcurl -plaintext localhost:50051 memory.MemoryService/GetVectorIndexStatus
```

---

### VectorTeleport

Semantic similarity search using embeddings.

**Request:**
```protobuf
message VectorTeleportRequest {
    string query = 1;                      // Natural language query
    int32 limit = 2;                       // Max results (default 20)
}
```

**Response:**
```protobuf
message VectorTeleportResponse {
    repeated VectorResult results = 1;     // Matches with cosine similarity scores
    int64 query_time_ms = 2;               // Query execution time
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty query
- `UNAVAILABLE`: Vector index not available

**Example:**
```bash
grpcurl -plaintext -d '{
  "query": "how to handle errors in async Rust",
  "limit": 15
}' localhost:50051 memory.MemoryService/VectorTeleport
```

---

### HybridSearch

Combined BM25 + vector search using Reciprocal Rank Fusion (RRF k=60).

**Request:**
```protobuf
message HybridSearchRequest {
    string query = 1;                      // Search query
    int32 limit = 2;                       // Max results (default 20)
    float bm25_weight = 3;                 // BM25 contribution (default 0.5)
    float vector_weight = 4;               // Vector contribution (default 0.5)
}
```

**Response:**
```protobuf
message HybridSearchResponse {
    repeated HybridResult results = 1;     // Fused results with combined scores
    bool bm25_available = 2;               // True if BM25 was used
    bool vector_available = 3;             // True if vector was used
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty query
- `INVALID_ARGUMENT`: Weights must be between 0.0 and 1.0
- `UNAVAILABLE`: Neither index available

**Example:**
```bash
grpcurl -plaintext -d '{
  "query": "async error handling patterns",
  "limit": 20,
  "bm25_weight": 0.4,
  "vector_weight": 0.6
}' localhost:50051 memory.MemoryService/HybridSearch
```

---

## Phase 14: Topic Graph

### GetTopicGraphStatus

Check topic graph availability.

**Request:**
```protobuf
message GetTopicGraphStatusRequest {}
```

**Response:**
```protobuf
message GetTopicGraphStatusResponse {
    bool available = 1;                    // True if graph is ready
    int64 topic_count = 2;                 // Number of topics
    int64 relationship_count = 3;          // Number of relationships
    int64 last_update = 4;                 // Last update timestamp (epoch ms)
}
```

**Example:**
```bash
grpcurl -plaintext localhost:50051 memory.MemoryService/GetTopicGraphStatus
```

---

### GetTopicsByQuery

Find topics matching a query.

**Request:**
```protobuf
message GetTopicsByQueryRequest {
    string query = 1;                      // Search query
    int32 limit = 2;                       // Max results (default 10)
}
```

**Response:**
```protobuf
message GetTopicsByQueryResponse {
    repeated Topic topics = 1;             // Matching topics
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty query
- `UNAVAILABLE`: Topic graph not available

**Example:**
```bash
grpcurl -plaintext -d '{
  "query": "rust memory management",
  "limit": 5
}' localhost:50051 memory.MemoryService/GetTopicsByQuery
```

---

### GetRelatedTopics

Get topics related to a given topic.

**Request:**
```protobuf
message GetRelatedTopicsRequest {
    string topic_id = 1;                   // Topic ID to find relations for
    repeated RelationshipType relationship_types = 2;  // Filter by relationship type
}
```

**Response:**
```protobuf
message GetRelatedTopicsResponse {
    repeated RelatedTopic related = 1;     // Related topics with relationship info
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty topic_id
- `NOT_FOUND`: Topic does not exist
- `UNAVAILABLE`: Topic graph not available

**Example:**
```bash
grpcurl -plaintext -d '{
  "topic_id": "topic:rust-async",
  "relationship_types": ["RELATIONSHIP_TYPE_SIMILAR", "RELATIONSHIP_TYPE_CHILD"]
}' localhost:50051 memory.MemoryService/GetRelatedTopics
```

---

### GetTopTopics

Get top topics by importance score with time decay.

**Request:**
```protobuf
message GetTopTopicsRequest {
    int32 limit = 1;                       // Max results (default 10)
    int32 since_days = 2;                  // Look back window in days (default 30)
}
```

**Response:**
```protobuf
message GetTopTopicsResponse {
    repeated RankedTopic topics = 1;       // Topics ranked by importance
}
```

**Example:**
```bash
grpcurl -plaintext -d '{
  "limit": 10,
  "since_days": 7
}' localhost:50051 memory.MemoryService/GetTopTopics
```

---

## gRPC Service: SchedulerService

The SchedulerService provides management operations for background scheduler jobs.

Proto file: `proto/scheduler.proto`

### ListJobs

List all registered scheduler jobs.

**Request:**
```protobuf
message ListJobsRequest {}
```

**Response:**
```protobuf
message ListJobsResponse {
    repeated JobInfo jobs = 1;             // All registered jobs
}
```

**Example:**
```bash
grpcurl -plaintext localhost:50051 scheduler.SchedulerService/ListJobs
```

---

### GetJob

Get status of a specific job.

**Request:**
```protobuf
message GetJobRequest {
    string job_name = 1;                   // Name of the job
}
```

**Response:**
```protobuf
message GetJobResponse {
    JobInfo job = 1;                       // Job details including status
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty job_name
- `NOT_FOUND`: Job does not exist

**Example:**
```bash
grpcurl -plaintext -d '{
  "job_name": "segment_summarizer"
}' localhost:50051 scheduler.SchedulerService/GetJob
```

---

### PauseJob

Pause a scheduler job.

**Request:**
```protobuf
message PauseJobRequest {
    string job_name = 1;                   // Name of the job to pause
}
```

**Response:**
```protobuf
message PauseJobResponse {
    bool success = 1;                      // True if paused successfully
    string message = 2;                    // Status message
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty job_name
- `NOT_FOUND`: Job does not exist
- `FAILED_PRECONDITION`: Job already paused

**Example:**
```bash
grpcurl -plaintext -d '{
  "job_name": "segment_summarizer"
}' localhost:50051 scheduler.SchedulerService/PauseJob
```

---

### ResumeJob

Resume a paused scheduler job.

**Request:**
```protobuf
message ResumeJobRequest {
    string job_name = 1;                   // Name of the job to resume
}
```

**Response:**
```protobuf
message ResumeJobResponse {
    bool success = 1;                      // True if resumed successfully
    string message = 2;                    // Status message
}
```

**Errors:**
- `INVALID_ARGUMENT`: Empty job_name
- `NOT_FOUND`: Job does not exist
- `FAILED_PRECONDITION`: Job not paused

**Example:**
```bash
grpcurl -plaintext -d '{
  "job_name": "segment_summarizer"
}' localhost:50051 scheduler.SchedulerService/ResumeJob
```

---

## Data Types

### Search Types (Phase 10.5)

#### SearchField

```protobuf
enum SearchField {
    SEARCH_FIELD_UNSPECIFIED = 0;
    SEARCH_FIELD_TITLE = 1;
    SEARCH_FIELD_SUMMARY = 2;
    SEARCH_FIELD_BULLETS = 3;
    SEARCH_FIELD_KEYWORDS = 4;
}
```

| Value | Description |
|-------|-------------|
| `TITLE` | Search in node titles |
| `SUMMARY` | Search in summaries |
| `BULLETS` | Search in bullet text |
| `KEYWORDS` | Search in keywords |

#### SearchMatch

```protobuf
message SearchMatch {
    string field = 1;                      // Field where match was found
    string text = 2;                       // Matched text
    float score = 3;                       // Match score (0.0-1.0)
    repeated string highlights = 4;        // Highlighted snippets
}
```

| Field | Type | Description |
|-------|------|-------------|
| `field` | string | Field name (title, summary, etc.) |
| `text` | string | Full text of matching content |
| `score` | float | Match relevance score |
| `highlights` | string[] | Text snippets with match highlighting |

#### NodeMatch

```protobuf
message NodeMatch {
    string node_id = 1;                    // ID of matching node
    TocNode node = 2;                      // Full node data
    repeated SearchMatch matches = 3;      // Matches within this node
    float aggregate_score = 4;             // Combined score for all matches
}
```

| Field | Type | Description |
|-------|------|-------------|
| `node_id` | string | TOC node identifier |
| `node` | TocNode | Full node data |
| `matches` | SearchMatch[] | Individual field matches |
| `aggregate_score` | float | Combined relevance score |

### Teleport Types (Phase 11)

#### DocType

```protobuf
enum DocType {
    DOC_TYPE_UNSPECIFIED = 0;
    DOC_TYPE_TOC_NODE = 1;
    DOC_TYPE_GRIP = 2;
}
```

| Value | Description |
|-------|-------------|
| `TOC_NODE` | Table of Contents node |
| `GRIP` | Provenance anchor |

#### TeleportResult

```protobuf
message TeleportResult {
    string doc_id = 1;                     // Document ID
    DocType doc_type = 2;                  // Document type
    string text = 3;                       // Document text
    float bm25_score = 4;                  // BM25 relevance score
    repeated string highlights = 5;        // Highlighted snippets
}
```

| Field | Type | Description |
|-------|------|-------------|
| `doc_id` | string | Unique document identifier |
| `doc_type` | DocType | Type of document |
| `text` | string | Document content |
| `bm25_score` | float | BM25 relevance score |
| `highlights` | string[] | Highlighted match snippets |

### Vector Types (Phase 12)

#### VectorResult

```protobuf
message VectorResult {
    string doc_id = 1;                     // Document ID
    DocType doc_type = 2;                  // Document type
    string text = 3;                       // Document text
    float similarity = 4;                  // Cosine similarity score (0.0-1.0)
}
```

| Field | Type | Description |
|-------|------|-------------|
| `doc_id` | string | Unique document identifier |
| `doc_type` | DocType | Type of document |
| `text` | string | Document content |
| `similarity` | float | Cosine similarity to query |

#### HybridResult

```protobuf
message HybridResult {
    string doc_id = 1;                     // Document ID
    DocType doc_type = 2;                  // Document type
    string text = 3;                       // Document text
    float combined_score = 4;              // RRF combined score
    float bm25_score = 5;                  // BM25 component score
    float vector_score = 6;                // Vector component score
    int32 bm25_rank = 7;                   // Rank in BM25 results
    int32 vector_rank = 8;                 // Rank in vector results
}
```

| Field | Type | Description |
|-------|------|-------------|
| `doc_id` | string | Unique document identifier |
| `doc_type` | DocType | Type of document |
| `text` | string | Document content |
| `combined_score` | float | RRF fused score |
| `bm25_score` | float | BM25 score (if available) |
| `vector_score` | float | Similarity score (if available) |
| `bm25_rank` | int32 | Position in BM25 ranking |
| `vector_rank` | int32 | Position in vector ranking |

### Topic Graph Types (Phase 14)

#### Topic

```protobuf
message Topic {
    string topic_id = 1;                   // Unique topic identifier
    string name = 2;                       // Topic name
    string description = 3;                // Topic description
    float importance = 4;                  // Importance score (0.0-1.0)
    int64 first_seen = 5;                  // First occurrence (epoch ms)
    int64 last_seen = 6;                   // Last occurrence (epoch ms)
    int32 occurrence_count = 7;            // Number of occurrences
}
```

| Field | Type | Description |
|-------|------|-------------|
| `topic_id` | string | Unique identifier (e.g., "topic:rust-async") |
| `name` | string | Human-readable topic name |
| `description` | string | Brief topic description |
| `importance` | float | Computed importance score |
| `first_seen` | int64 | First occurrence timestamp |
| `last_seen` | int64 | Most recent occurrence |
| `occurrence_count` | int32 | Total occurrence count |

#### RelationshipType

```protobuf
enum RelationshipType {
    RELATIONSHIP_TYPE_UNSPECIFIED = 0;
    RELATIONSHIP_TYPE_SIMILAR = 1;
    RELATIONSHIP_TYPE_CHILD = 2;
    RELATIONSHIP_TYPE_PARENT = 3;
    RELATIONSHIP_TYPE_SEQUENTIAL = 4;
}
```

| Value | Description |
|-------|-------------|
| `SIMILAR` | Topics are semantically similar |
| `CHILD` | Topic is a subtopic |
| `PARENT` | Topic is a parent topic |
| `SEQUENTIAL` | Topics often appear in sequence |

#### RelatedTopic

```protobuf
message RelatedTopic {
    Topic topic = 1;                       // The related topic
    RelationshipType relationship = 2;     // Type of relationship
    float strength = 3;                    // Relationship strength (0.0-1.0)
}
```

| Field | Type | Description |
|-------|------|-------------|
| `topic` | Topic | The related topic |
| `relationship` | RelationshipType | How topics are related |
| `strength` | float | Relationship strength |

#### RankedTopic

```protobuf
message RankedTopic {
    Topic topic = 1;                       // The topic
    float rank_score = 2;                  // Time-decayed importance score
    int32 rank = 3;                        // Position in ranking
}
```

| Field | Type | Description |
|-------|------|-------------|
| `topic` | Topic | The topic |
| `rank_score` | float | Score with time decay applied |
| `rank` | int32 | Position in ranking (1-based) |

### Scheduler Types

#### JobStatus

```protobuf
enum JobStatus {
    JOB_STATUS_UNSPECIFIED = 0;
    JOB_STATUS_RUNNING = 1;
    JOB_STATUS_PAUSED = 2;
    JOB_STATUS_IDLE = 3;
    JOB_STATUS_ERROR = 4;
}
```

| Value | Description |
|-------|-------------|
| `RUNNING` | Job is currently executing |
| `PAUSED` | Job is paused |
| `IDLE` | Job is waiting for next scheduled run |
| `ERROR` | Job encountered an error |

#### JobInfo

```protobuf
message JobInfo {
    string name = 1;                       // Job name
    string description = 2;                // Job description
    JobStatus status = 3;                  // Current status
    string schedule = 4;                   // Cron schedule expression
    int64 last_run = 5;                    // Last run timestamp (epoch ms)
    int64 next_run = 6;                    // Next scheduled run (epoch ms)
    int32 run_count = 7;                   // Total successful runs
    int32 error_count = 8;                 // Total error runs
    optional string last_error = 9;        // Last error message (if any)
}
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique job identifier |
| `description` | string | Human-readable description |
| `status` | JobStatus | Current job status |
| `schedule` | string | Cron expression (e.g., "0 */5 * * * *") |
| `last_run` | int64 | Last execution timestamp |
| `next_run` | int64 | Next scheduled execution |
| `run_count` | int32 | Successful execution count |
| `error_count` | int32 | Failed execution count |
| `last_error` | string | Most recent error message |

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
