# Data Flow and Sequence Diagrams

This document provides detailed sequence diagrams showing how data flows through the Agent Memory system. Each diagram includes step-by-step explanations, data payloads, and error handling strategies.

## Table of Contents

1. [Event Ingestion Flow](#1-event-ingestion-flow)
2. [TOC Building Flow](#2-toc-building-flow)
3. [TOC Rollup Flow](#3-toc-rollup-flow)
4. [Query Resolution Flow](#4-query-resolution-flow)
5. [Grip Expansion Flow](#5-grip-expansion-flow)

---

## 1. Event Ingestion Flow

The ingestion path captures conversation events from agent hooks and persists them for later retrieval and summarization. This is the entry point for all memory data.

### Why This Design?

The ingestion flow is designed around three key principles:
- **Fail-open**: Never block the agent if memory is down
- **Idempotent**: Duplicate ingestion requests are safe
- **Atomic**: Event and outbox entry are written together or not at all

### Sequence Diagram

```mermaid
sequenceDiagram
    participant CCH as Claude Code Hook
    participant INB as memory-ingest Binary
    participant CLI as MemoryClient
    participant SVC as MemoryService (gRPC)
    participant CVT as Type Converter
    participant DB as RocksDB Storage
    participant OBX as Outbox CF

    Note over CCH: Agent conversation event occurs

    CCH->>INB: 1. Invoke hook handler
    Note right of CCH: HookEvent {<br/>  session_id, event_type,<br/>  content, timestamp<br/>}

    INB->>CLI: 2. map_hook_event(hook)
    Note right of INB: Converts HookEventType<br/>to EventType

    CLI->>CLI: 3. Generate ULID event_id
    Note right of CLI: ULID contains timestamp<br/>for time-ordered keys

    CLI->>SVC: 4. IngestEvent(request)
    Note right of CLI: gRPC call with<br/>ProtoEvent payload

    SVC->>CVT: 5. Validate & convert proto
    Note right of SVC: Check event_id,<br/>session_id not empty

    CVT-->>SVC: 6. Return domain Event

    SVC->>DB: 7. Check idempotency
    Note right of SVC: get_cf(events, event_key)

    alt Event already exists
        DB-->>SVC: 8a. Return existing bytes
        SVC-->>CLI: Response: created=false
    else Event is new
        DB-->>SVC: 8b. Return None

        SVC->>SVC: 9. Create OutboxEntry
        Note right of SVC: OutboxEntry::for_toc(<br/>  event_id, timestamp_ms<br/>)

        SVC->>DB: 10. Atomic batch write
        Note right of SVC: WriteBatch {<br/>  events_cf: event_bytes,<br/>  outbox_cf: outbox_bytes<br/>}

        DB->>OBX: 11. Outbox entry persisted
        Note right of DB: Sequence number assigned

        DB-->>SVC: 12. Write success
        SVC-->>CLI: Response: created=true
    end

    CLI-->>INB: 13. Return result
    INB-->>CCH: 14. Exit (fail-open)
    Note right of CCH: Agent continues<br/>regardless of result
```

### Step-by-Step Explanation

| Step | Component | Action | Data Payload |
|------|-----------|--------|--------------|
| 1 | CCH Hook | Invokes memory-ingest binary | `HookEvent { session_id, event_type, content, timestamp? }` |
| 2 | memory-ingest | Maps hook event to memory event | Converts `HookEventType::UserPromptSubmit` to `EventType::UserMessage` |
| 3 | MemoryClient | Generates unique event ID | ULID string (e.g., `01HRMF...`) containing timestamp |
| 4 | MemoryClient | Sends gRPC request | `IngestEventRequest { event: ProtoEvent }` |
| 5-6 | MemoryService | Validates and converts | Checks required fields, converts proto to domain `Event` |
| 7-8 | Storage | Idempotency check | Returns existing data or None |
| 9 | MemoryService | Creates outbox entry | `OutboxEntry { event_id, timestamp_ms, entry_type: Toc }` |
| 10-11 | Storage | Atomic write | RocksDB `WriteBatch` ensures both succeed or both fail |
| 12-14 | Response path | Returns result | `IngestEventResponse { event_id, created: bool }` |

### Data Payloads at Key Points

**Hook Event (Input)**
```rust
HookEvent {
    session_id: "session-abc123",
    event_type: HookEventType::UserPromptSubmit,
    content: "How do I implement JWT authentication?",
    timestamp: Some(2026-01-31T10:15:30Z),
    tool_name: None,
    metadata: None,
}
```

**Proto Event (Over Wire)**
```protobuf
Event {
    event_id: "01HRMF8K2X3YNPQRSTUVWXYZ",
    session_id: "session-abc123",
    timestamp_ms: 1738318530000,
    event_type: EVENT_TYPE_USER_MESSAGE,
    role: EVENT_ROLE_USER,
    text: "How do I implement JWT authentication?",
    metadata: {},
}
```

**Storage Key**
```
events CF key: <timestamp_ms_be><ulid_random>
             = 0x00000194F2B1E3E0<random_bytes>
```

### Error Handling

| Error Type | Handling Strategy | Impact |
|------------|-------------------|--------|
| Connection failure | Client logs error, returns | Agent continues (fail-open) |
| Invalid event_id | Returns `InvalidArgument` status | Client should retry with valid ID |
| Invalid session_id | Returns `InvalidArgument` status | Client should retry with valid session |
| Storage write failure | Returns `Internal` status | Outbox not written, event not persisted |
| Serialization error | Returns `Internal` status | Logs error for debugging |

### Async vs Sync Operations

- **Synchronous**: The entire ingestion path is synchronous from the agent's perspective
- **Blocking wait**: gRPC call blocks until storage confirms write
- **Why sync?**: Ensures the hook can report success/failure before agent continues

---

## 2. TOC Building Flow

When segments are created from ingested events, the TOC builder creates hierarchical summary nodes. This is where raw events become navigable summaries.

### Why This Design?

The TOC building flow exists because:
- **Progressive Disclosure**: Agents start with summaries, not raw events
- **Time Navigation**: Every event has a place in the Year > Month > Week > Day > Segment hierarchy
- **Provenance**: Grips link summaries back to source events

### Sequence Diagram

```mermaid
sequenceDiagram
    participant EVT as Events
    participant SEG as SegmentBuilder
    participant BLD as TocBuilder
    participant SUM as Summarizer
    participant GRP as GripExtractor
    participant DB as RocksDB Storage

    Note over EVT: Events arrive from outbox processing

    EVT->>SEG: 1. Stream events in time order
    Note right of EVT: Events with timestamps

    loop For each event
        SEG->>SEG: 2. Count tokens (tiktoken)
        Note right of SEG: Truncate tool results<br/>to max_chars

        SEG->>SEG: 3. Check boundaries
        Note right of SEG: Time gap > 30 min?<br/>Tokens > 4K?

        alt Boundary detected
            SEG->>SEG: 4. Flush segment
            Note right of SEG: Create Segment with<br/>overlap buffer

            SEG-->>BLD: 5. Emit segment
        end
    end

    BLD->>SUM: 6. summarize_events(events)
    Note right of BLD: All events including<br/>overlap events

    SUM->>SUM: 7. Generate summary
    Note right of SUM: API call or local LLM

    SUM-->>BLD: 8. Return Summary
    Note right of SUM: {title, bullets, keywords}

    BLD->>BLD: 9. Create segment node
    Note right of BLD: TocNode at Segment level

    BLD->>GRP: 10. extract_grips(events, bullets)
    Note right of BLD: Find supporting evidence

    GRP->>GRP: 11. Match bullet terms to events
    Note right of GRP: Term frequency analysis

    GRP-->>BLD: 12. Return extracted grips
    Note right of GRP: Vec<ExtractedGrip>

    loop For each grip
        BLD->>DB: 13. put_grip(grip)
        Note right of BLD: Store in grips CF<br/>with node index
    end

    BLD->>DB: 14. put_toc_node(segment_node)
    Note right of BLD: Store in toc_nodes CF

    BLD->>BLD: 15. ensure_parents()
    Note right of BLD: Create Day, Week,<br/>Month, Year nodes

    loop For each parent level
        BLD->>DB: 16. get_toc_node(parent_id)

        alt Parent exists
            BLD->>DB: 17a. Update child list
        else Parent missing
            BLD->>BLD: 17b. Create parent node
            Note right of BLD: Placeholder summary<br/>"Pending rollup..."
            BLD->>DB: 18. put_toc_node(parent)
        end
    end
```

### Step-by-Step Explanation

| Step | Component | Action | Output |
|------|-----------|--------|--------|
| 1-3 | SegmentBuilder | Process events, count tokens | Token count per event |
| 4-5 | SegmentBuilder | Detect boundary, emit segment | `Segment { events, overlap_events, token_count }` |
| 6-8 | Summarizer | Generate summary from events | `Summary { title, bullets, keywords }` |
| 9 | TocBuilder | Create segment-level node | `TocNode` with level=Segment |
| 10-12 | GripExtractor | Extract evidence grips | Grips linking bullets to events |
| 13-14 | Storage | Persist grips and node | Written to grips_cf, toc_nodes_cf |
| 15-18 | TocBuilder | Ensure parent hierarchy | Creates missing Day/Week/Month/Year nodes |

### Segmentation Boundaries

The segment builder uses two boundary detection criteria:

```
Time Gap Boundary:
  event.timestamp - last_event.timestamp > 30 minutes

Token Threshold Boundary:
  current_segment_tokens + event_tokens > 4K tokens
```

### Summary Structure

```rust
Summary {
    title: "JWT Authentication Implementation Discussion",
    bullets: [
        "Discussed pros and cons of JWT vs session tokens",
        "Implemented access and refresh token flow",
        "Added token expiration handling",
    ],
    keywords: ["jwt", "authentication", "tokens", "security"],
}
```

### Parent Node Creation

When a segment node is created, the builder ensures all parent nodes exist:

```
Segment: toc:segment:2026-01-31:abc123
    -> Day: toc:day:2026-01-31
        -> Week: toc:week:2026-05  (week 5 of 2026)
            -> Month: toc:month:2026-01
                -> Year: toc:year:2026
```

Parent nodes start with placeholder summaries that get replaced during rollup.

---

## 3. TOC Rollup Flow

Rollup jobs aggregate child summaries into parent summaries, building the time hierarchy from bottom up. This is how higher-level nodes get meaningful content.

### Why This Design?

Rollup serves several purposes:
- **Aggregation**: Combine many segment summaries into daily/weekly/monthly overviews
- **Efficiency**: Pre-compute summaries so queries return instantly
- **Recovery**: Checkpoints enable crash-safe processing

### Sequence Diagram

```mermaid
sequenceDiagram
    participant SCH as Scheduler
    participant JOB as RollupJob
    participant DB as RocksDB Storage
    participant SUM as Summarizer
    participant CKP as Checkpoint Store

    Note over SCH: Cron trigger fires<br/>(e.g., 1 AM daily)

    SCH->>JOB: 1. Execute day rollup job
    Note right of SCH: With overlap guard

    JOB->>CKP: 2. Load checkpoint
    Note right of JOB: job_name: "rollup_day"

    CKP-->>JOB: 3. Return last_processed_time
    Note right of CKP: Or MIN_UTC if none

    JOB->>DB: 4. get_toc_nodes_by_level(Day)
    Note right of JOB: Filter: start_time > checkpoint<br/>end_time < cutoff

    DB-->>JOB: 5. Return day nodes

    loop For each day node
        JOB->>JOB: 6. Check min_age
        Note right of JOB: Skip if period<br/>not yet closed

        JOB->>DB: 7. get_child_nodes(day_id)
        Note right of JOB: Get segment nodes

        DB-->>JOB: 8. Return child segments

        alt No children
            JOB->>JOB: 9a. Skip node
        else Has children
            JOB->>JOB: 9b. Convert to summaries
            Note right of JOB: Extract title, bullets,<br/>keywords from each

            JOB->>SUM: 10. summarize_children(summaries)
            Note right of JOB: Aggregate child summaries

            SUM-->>JOB: 11. Return rollup summary
            Note right of SUM: Combined title,<br/>top bullets, keywords

            JOB->>JOB: 12. Update node
            Note right of JOB: Set title, bullets,<br/>keywords, child_ids

            JOB->>DB: 13. put_toc_node(updated)
            Note right of JOB: New version created

            JOB->>CKP: 14. Save checkpoint
            Note right of JOB: last_processed_time =<br/>node.end_time
        end
    end

    JOB-->>SCH: 15. Return processed count
    Note right of JOB: Log: "Day rollup complete:<br/>N nodes processed"
```

### Rollup Hierarchy

Rollups execute in order from bottom to top:

```mermaid
graph TD
    SEG[Segment Nodes] -->|Day Rollup| DAY[Day Nodes]
    DAY -->|Week Rollup| WEEK[Week Nodes]
    WEEK -->|Month Rollup| MONTH[Month Nodes]
    MONTH -->|Year Rollup| YEAR[Year Nodes]

    style SEG fill:#e1f5fe
    style DAY fill:#b3e5fc
    style WEEK fill:#81d4fa
    style MONTH fill:#4fc3f7
    style YEAR fill:#29b6f6
```

### Schedule Configuration

```rust
RollupJobConfig {
    day_cron: "0 0 1 * * *",     // 1 AM daily
    week_cron: "0 0 2 * * 0",    // 2 AM Sunday
    month_cron: "0 0 3 1 * *",   // 3 AM 1st of month
    timezone: "UTC",
    jitter_secs: 300,            // Up to 5 min random delay
}
```

### Minimum Age Requirements

Each rollup level has a minimum age to avoid rolling up incomplete periods:

| Level | Min Age | Rationale |
|-------|---------|-----------|
| Day | 1 hour | Wait for current hour's segments |
| Week | 24 hours | Wait for current day to complete |
| Month | 24 hours | Wait for current day to complete |
| Year | 7 days | Wait for current week to complete |

### Checkpoint Structure

```rust
RollupCheckpoint {
    job_name: "rollup_day",
    level: TocLevel::Day,
    last_processed_time: 2026-01-30T23:59:59Z,
    processed_count: 5,
    created_at: 2026-01-31T01:05:23Z,
}
```

### Error Handling and Recovery

| Failure Point | Recovery Strategy |
|---------------|-------------------|
| Crash during summarization | Restart from checkpoint, re-process node |
| API summarizer timeout | Job fails, retries on next schedule |
| Storage write failure | Checkpoint not saved, node re-processed next run |
| Overlap (job still running) | Skip via OverlapPolicy::Skip |

---

## 4. Query Resolution Flow

When an agent asks "what did we discuss last week?", the query resolution flow navigates the TOC hierarchy to find relevant content.

### Why This Design?

The query path implements Progressive Disclosure:
- **Start broad**: Begin at year or month level
- **Drill down**: Navigate to more specific time periods
- **Verify**: Use grips to confirm with source events

### Sequence Diagram

```mermaid
sequenceDiagram
    participant AGT as Agent (Claude Code)
    participant SKL as Memory Navigator Skill
    participant CLI as MemoryClient
    participant SVC as MemoryService
    participant DB as RocksDB Storage

    Note over AGT: User asks: "What did we<br/>discuss last week?"

    AGT->>SKL: 1. Invoke memory skill
    Note right of AGT: Query: "last week"

    SKL->>CLI: 2. get_toc_root()
    Note right of SKL: Start at year level

    CLI->>SVC: 3. GetTocRoot RPC
    SVC->>DB: 4. get_toc_nodes_by_level(Year)
    DB-->>SVC: 5. Return year nodes
    SVC-->>CLI: 6. GetTocRootResponse

    CLI-->>SKL: 7. Year nodes list
    Note right of CLI: [2026, 2025, ...]<br/>sorted desc

    SKL->>SKL: 8. Select current year
    Note right of SKL: Based on query context

    SKL->>CLI: 9. get_node("toc:year:2026")
    CLI->>SVC: 10. GetNode RPC
    SVC->>DB: 11. get_toc_node(node_id)
    DB-->>SVC: 12. Return node + children
    SVC-->>CLI: 13. GetNodeResponse

    CLI-->>SKL: 14. Year node with months
    Note right of CLI: child_node_ids: [<br/>  toc:month:2026-01<br/>]

    SKL->>CLI: 15. browse_toc(year_id, limit=12)
    Note right of SKL: Get month children

    CLI->>SVC: 16. BrowseToc RPC
    SVC->>DB: 17. get_child_nodes(parent_id)
    DB-->>SVC: 18. Return month nodes
    SVC-->>CLI: 19. BrowseTocResponse

    CLI-->>SKL: 20. Month nodes list

    SKL->>SKL: 21. Find "last week"
    Note right of SKL: Calculate week from<br/>current date

    SKL->>CLI: 22. get_node("toc:week:2026-05")
    Note right of SKL: Week 5 of 2026

    CLI->>SVC: 23. GetNode RPC
    SVC->>DB: 24. get_toc_node(node_id)
    DB-->>SVC: 25. Return week node
    SVC-->>CLI: 26. GetNodeResponse

    CLI-->>SKL: 27. Week summary
    Note right of CLI: title, bullets,<br/>keywords, grip_ids

    SKL-->>AGT: 28. Present week summary
    Note right of SKL: "Last week we discussed:<br/>- JWT implementation<br/>- Token refresh flow"

    opt Agent wants details
        AGT->>SKL: 29. "Tell me more about JWT"
        SKL->>CLI: 30. expand_grip(grip_id)
        Note right of SKL: Get source events
    end
```

### Navigation Pattern

The agent navigates down the hierarchy:

```
1. GetTocRoot() -> [Year nodes]
2. BrowseToc(year_id) -> [Month nodes]
3. GetNode(month_id) -> Month summary
4. BrowseToc(month_id) -> [Week nodes]
5. GetNode(week_id) -> Week summary + grip_ids
6. ExpandGrip(grip_id) -> Source events (if needed)
```

### Response Payloads

**Week Node Response**
```rust
TocNode {
    node_id: "toc:week:2026-05",
    level: Week,
    title: "Week 5, 2026",
    bullets: [
        TocBullet {
            text: "Implemented JWT authentication with access and refresh tokens",
            grip_ids: ["grip:1738318530000:jwt123"],
        },
        TocBullet {
            text: "Debugged token expiration race condition",
            grip_ids: ["grip:1738405200000:debug456"],
        },
    ],
    keywords: ["jwt", "authentication", "tokens", "debugging"],
    child_node_ids: [
        "toc:day:2026-01-27",
        "toc:day:2026-01-28",
        ...
    ],
    start_time_ms: 1737936000000,
    end_time_ms: 1738540799000,
}
```

### Pagination

For large result sets, BrowseToc supports pagination:

```
Request 1:
  BrowseToc(parent_id, limit=10, token=None)
  -> children[0..9], continuation_token="10"

Request 2:
  BrowseToc(parent_id, limit=10, token="10")
  -> children[10..19], continuation_token="20"

Request 3:
  BrowseToc(parent_id, limit=10, token="20")
  -> children[20..25], continuation_token=None, has_more=false
```

---

## 5. Grip Expansion Flow

Grips provide provenance - they link summary bullets back to source events. When an agent needs to verify or get context, it expands a grip.

### Why This Design?

Grips solve the "trust but verify" problem:
- **Summaries**: Give agents quick answers
- **Grips**: Let agents verify with source material
- **Context**: Surrounding events help understand the excerpt

### Sequence Diagram

```mermaid
sequenceDiagram
    participant AGT as Agent
    participant CLI as MemoryClient
    participant SVC as MemoryService
    participant DB as RocksDB Storage

    Note over AGT: Agent wants to verify<br/>a summary bullet

    AGT->>CLI: 1. expand_grip(grip_id)
    Note right of AGT: grip_id from bullet.grip_ids

    CLI->>SVC: 2. ExpandGrip RPC
    Note right of CLI: {grip_id,<br/>events_before: 3,<br/>events_after: 3}

    SVC->>DB: 3. get_grip(grip_id)

    alt Grip not found
        DB-->>SVC: 4a. Return None
        SVC-->>CLI: Response with empty fields
    else Grip exists
        DB-->>SVC: 4b. Return Grip
        Note right of DB: {excerpt,<br/>event_id_start,<br/>event_id_end,<br/>timestamp_ms}

        SVC->>SVC: 5. Calculate time window
        Note right of SVC: grip_time +/- 1 hour

        SVC->>DB: 6. get_events_in_range(start, end)

        DB-->>SVC: 7. Return events in range

        SVC->>SVC: 8. Partition events
        Note right of SVC: Before / Excerpt / After<br/>based on event_ids

        SVC->>SVC: 9. Apply limits
        Note right of SVC: Take last N before,<br/>first N after

        SVC-->>CLI: 10. ExpandGripResponse
    end

    CLI-->>AGT: 11. ExpandGripResult
    Note right of CLI: {grip, events_before,<br/>excerpt_events, events_after}
```

### Step-by-Step Explanation

| Step | Action | Details |
|------|--------|---------|
| 1-2 | Client requests expansion | Includes optional before/after counts |
| 3-4 | Fetch grip from storage | Returns excerpt, event IDs, timestamp |
| 5 | Calculate time window | Default: 1 hour before/after grip timestamp |
| 6-7 | Fetch events in window | Range scan on events column family |
| 8 | Partition events | Three groups based on event_id_start/end |
| 9 | Apply limits | Default: 3 events before, 3 after |
| 10-11 | Return structured result | All three event groups plus grip metadata |

### Grip Structure

```rust
Grip {
    grip_id: "grip:1738318530000:jwt123",
    excerpt: "We should use RS256 for signing JWTs because...",
    event_id_start: "01HRMF8K2X3YNPQ...",  // First event of excerpt
    event_id_end: "01HRMF8K9Z7ABCD...",    // Last event of excerpt
    timestamp: 2026-01-31T10:15:30Z,
    source: "toc:segment:2026-01-31:abc123",
    toc_node_id: Some("toc:segment:2026-01-31:abc123"),
}
```

### Expansion Result

```rust
ExpandGripResult {
    grip: Grip { ... },

    events_before: [
        Event { text: "How should we sign JWTs?", role: User },
        Event { text: "Let me research that...", role: Assistant },
        Event { text: "Read security docs", role: Tool },
    ],

    excerpt_events: [
        Event { text: "We should use RS256 for signing JWTs because it provides asymmetric key security...", role: Assistant },
    ],

    events_after: [
        Event { text: "That makes sense", role: User },
        Event { text: "Great, I'll implement that", role: Assistant },
        Event { text: "Write token.rs", role: Tool },
    ],
}
```

### Context Window Strategy

The expansion uses a time-based window with event count limits:

```
                Time Window (1 hour before/after)
    |--------------------|----------|---------------------|
    |                    |  EXCERPT |                     |
    |    events_before   |  events  |    events_after     |
    |     (limit: 3)     |          |     (limit: 3)      |
    |--------------------|----------|---------------------|
```

This ensures:
- Relevant context is always available
- Response size stays bounded
- Time-adjacent events provide conversational flow

### Error Handling

| Error | Response | Agent Action |
|-------|----------|--------------|
| Grip not found | Empty response | Try different grip_id |
| Event deserialization fails | Skip event, log warning | Partial context returned |
| Time window empty | Empty before/after lists | Excerpt events only |

---

## Summary

The Agent Memory system's data flows are designed around these principles:

1. **Ingestion**: Fast, fail-open, idempotent writes
2. **TOC Building**: Events become segments become summaries
3. **Rollup**: Bottom-up aggregation with crash recovery
4. **Query**: Top-down navigation through time hierarchy
5. **Grip Expansion**: Provenance for verification

Each flow handles errors gracefully, ensuring the agent can always continue working even if memory operations fail.
