# Phase 4 Research: Query Layer

## Overview

Phase 4 exposes navigation and retrieval RPCs so agents can traverse the TOC hierarchy and access raw events. This enables the core use case: agents answering questions like "what did we discuss last week?" by navigating summaries and drilling down to source evidence.

## Requirements Analysis

### QRY-01: GetTocRoot RPC
Returns top-level time period nodes (years).
- Entry point for TOC navigation
- Returns list of year-level nodes with summaries
- Ordered by time (most recent first for typical use)

### QRY-02: GetNode RPC
Returns a specific node with its children and summary.
- Fetch single node by ID
- Include child_node_ids for navigation
- Include full summary (title, bullets, keywords)
- Include grip_ids for provenance drilling

### QRY-03: BrowseToc RPC
Supports paginated navigation of large child lists.
- Useful for days/weeks with many segments
- Page size parameter
- Continuation token for cursor-based pagination
- Returns subset of children with navigation info

### QRY-04: GetEvents RPC
Retrieves raw events by time range.
- Direct event access without TOC traversal
- Start/end timestamp parameters
- Optional limit for result size
- Returns events ordered by timestamp

### QRY-05: ExpandGrip RPC
Retrieves context around a grip excerpt.
- Already implemented in memory-toc::expand
- Expose via gRPC wrapper
- Return events_before, excerpt_events, events_after

## Existing Components

### Storage Layer (memory-storage)
- `get_toc_node(id)` - fetch single node
- `get_child_nodes(parent_id)` - fetch children
- `get_events_in_range(start_ms, end_ms)` - time range query
- `get_grip(id)` - fetch grip for expansion

### TOC Library (memory-toc)
- `GripExpander::expand(grip_id)` - context retrieval
- `TocLevel` enum - Year, Month, Week, Day, Segment
- Node ID format: `toc:{level}:{time_identifier}`

### Service Layer (memory-service)
- `MemoryServiceImpl` with `Arc<Storage>`
- Proto module with FILE_DESCRIPTOR_SET
- Health and reflection services configured

## Design Decisions

### Proto Message Design
1. **TocNode message**: Include all summary fields for one-fetch access
2. **Pagination**: Use cursor-based (continuation_token) not offset-based
3. **GetTocRoot**: Return years, allow caller to specify date filter
4. **Event retrieval**: Support both range and limit

### Implementation Approach
1. Add new proto messages and RPCs to memory.proto
2. Regenerate Rust bindings via build.rs
3. Add query module to memory-service with handler implementations
4. Each RPC handler uses Storage and expand utilities
5. Add tests for each RPC

## Plan Structure

### Plan 04-01: TOC Navigation RPCs
- Update proto with TocNode message and navigation RPCs
- Implement GetTocRoot (year nodes)
- Implement GetNode (fetch by ID)
- Implement BrowseToc (paginated children)

### Plan 04-02: Event Retrieval RPCs
- Add GetEvents proto and implementation
- Add ExpandGrip proto and implementation (wrapper around memory-toc)
- Integration tests for full workflow

## Proto Extensions

```protobuf
// TOC node summary
message TocNode {
    string node_id = 1;
    string level = 2;  // Year, Month, Week, Day, Segment
    string title = 3;
    repeated TocBullet bullets = 4;
    repeated string keywords = 5;
    repeated string child_node_ids = 6;
    int64 start_time_ms = 7;
    int64 end_time_ms = 8;
}

message TocBullet {
    string text = 1;
    repeated string grip_ids = 2;
}

// Navigation RPCs
message GetTocRootRequest {
    optional int32 year = 1;  // Filter by year, or all if not set
}

message GetTocRootResponse {
    repeated TocNode nodes = 1;
}

message GetNodeRequest {
    string node_id = 1;
}

message GetNodeResponse {
    TocNode node = 1;
}

message BrowseTocRequest {
    string parent_node_id = 1;
    int32 page_size = 2;
    string continuation_token = 3;
}

message BrowseTocResponse {
    repeated TocNode children = 1;
    string next_continuation_token = 2;
    bool has_more = 3;
}

// Event retrieval RPCs
message GetEventsRequest {
    int64 start_time_ms = 1;
    int64 end_time_ms = 2;
    int32 limit = 3;
}

message GetEventsResponse {
    repeated Event events = 1;
}

message ExpandGripRequest {
    string grip_id = 1;
    int32 events_before = 2;
    int32 events_after = 3;
}

message ExpandGripResponse {
    Grip grip = 1;
    repeated Event events_before = 2;
    repeated Event excerpt_events = 3;
    repeated Event events_after = 4;
}

message Grip {
    string grip_id = 1;
    string excerpt = 2;
    string event_id_start = 3;
    string event_id_end = 4;
    int64 timestamp_ms = 5;
    string source = 6;
    optional string toc_node_id = 7;
}
```

## Test Strategy

1. **Unit tests**: Each RPC handler with mock data
2. **Integration tests**: Full flow from ingest to query
3. **Edge cases**: Empty results, invalid IDs, pagination boundaries
