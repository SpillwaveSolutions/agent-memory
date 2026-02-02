# Data Schema Reference

**Document Version:** 1.0
**Last Updated:** 2026-02-01
**Status:** Authoritative Reference

## Overview

This document provides a complete reference for the agent-memory data schema, explaining how events are stored, how segments are formed from events, and how the TOC hierarchy organizes conversation memory. All keys are temporal-based, enabling efficient time-range queries.

**Related Documents:**
- [Storage Architecture](07-storage-architecture.md) - RocksDB internals, compaction, operational guidance
- [Domain Model](03-domain-model.md) - Conceptual entity relationships

---

## 1. Event Schema

Events are the fundamental unit of storage — immutable records of conversation turns.

### 1.1 Event Data Structure

```rust
pub struct Event {
    /// Unique identifier (ULID string)
    pub event_id: String,

    /// Session this event belongs to
    pub session_id: String,

    /// Source timestamp (when event occurred, NOT ingestion time)
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: EventType,

    /// Role of the author
    pub role: EventRole,

    /// Event content/text
    pub text: String,

    /// Additional metadata (tool names, file paths, etc.)
    pub metadata: HashMap<String, String>,
}
```

### 1.2 Event Types

```rust
enum EventType {
    SessionStart,      // Session began
    UserMessage,       // User input
    AssistantMessage,  // Assistant response
    ToolResult,        // Tool call returned
    AssistantStop,     // Assistant finished
    SubagentStart,     // Subagent spawned
    SubagentStop,      // Subagent completed
    SessionEnd,        // Session ended
}
```

### 1.3 Event Roles

```rust
enum EventRole {
    User,       // Human input
    Assistant,  // AI response
    System,     // System message
    Tool,       // Tool invocation/result
}
```

### 1.4 Event Storage Key Format

```
Key:   evt:{timestamp_ms:013}:{ulid}
       ─────────────────────────────
       │    │                   │
       │    │                   └── 26-char ULID for uniqueness
       │    └── Milliseconds since epoch, zero-padded to 13 digits
       └── Prefix identifying event records

Examples:
  evt:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
  evt:1706540401500:01HN4QXMP7ABCDEFGHIJKLMNOP
```

**Why this format?**
- Zero-padded timestamp enables **lexicographic ordering** = time ordering
- RocksDB prefix iteration allows efficient **time-range scans**
- ULID ensures uniqueness even for events in the same millisecond

### 1.5 Event JSON Example

```json
{
  "event_id": "01HN4QXKN6YWXVKZ3JMHP4BCDE",
  "session_id": "session-xyz-123",
  "timestamp": 1706540400000,
  "event_type": "user_message",
  "role": "user",
  "text": "How do I implement JWT authentication?",
  "metadata": {
    "project": "/home/user/my-app"
  }
}
```

---

## 2. Segment Schema (Leaf Sections)

Segments are **groups of events** created by the segmentation engine. They become the leaf nodes of the TOC hierarchy.

### 2.1 Segment Data Structure

```rust
pub struct Segment {
    /// Unique segment identifier
    pub segment_id: String,

    /// Events from previous segment for context continuity
    pub overlap_events: Vec<Event>,

    /// Events in this segment (excluding overlap)
    pub events: Vec<Event>,

    /// Start time (first event, excluding overlap)
    pub start_time: DateTime<Utc>,

    /// End time (last event)
    pub end_time: DateTime<Utc>,

    /// Token count of events (excluding overlap)
    pub token_count: usize,
}
```

### 2.2 How Segments are Created

The **SegmentBuilder** detects boundaries based on two thresholds:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Segment Boundary Detection                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. TIME GAP BOUNDARY (default: 30 minutes)                      │
│     ┌────┐ ┌────┐ ┌────┐        ┌────┐ ┌────┐                   │
│     │ E1 │ │ E2 │ │ E3 │  ≥30m  │ E4 │ │ E5 │                   │
│     └────┘ └────┘ └────┘  gap   └────┘ └────┘                   │
│     ─────────────────────────┬─────────────────                 │
│          Segment 1           │      Segment 2                   │
│                                                                  │
│  2. TOKEN THRESHOLD BOUNDARY (default: 4K tokens)               │
│     ┌────┐ ┌────┐ ┌────────────────┐ ┌────┐                    │
│     │ E1 │ │ E2 │ │ E3 (long)      │ │ E4 │                    │
│     │100 │ │200 │ │ 3800 tokens    │ │150 │                    │
│     └────┘ └────┘ └────────────────┘ └────┘                    │
│     ─────────────────────────┬───────────────                   │
│     Seg 1 (300 tok)          │  Seg 2 (3950 tok)               │
│                                                                  │
│  3. OVERLAP for context continuity                              │
│     Events within last 5 min OR last 500 tokens are copied     │
│     to the next segment as overlap_events                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.3 Segmentation Configuration

```rust
SegmentationConfig {
    time_threshold_ms: 1_800_000,     // 30 minutes
    token_threshold: 4096,             // 4K tokens
    overlap_time_ms: 300_000,          // 5 minutes overlap
    overlap_tokens: 500,               // 500 tokens overlap max
    max_tool_result_chars: 2000,       // Truncate tool results
}
```

### 2.4 Segment ID Format

```
Format: seg:{ulid}

Example: seg:01HN5SEGMENT123ABCDEFGHIJ
```

---

## 3. TOC Node Schema (Hierarchy)

TOC nodes form a **time-based hierarchy** that summarizes conversation content.

### 3.1 TOC Node Data Structure

```rust
pub struct TocNode {
    /// Unique identifier (temporal key)
    pub node_id: String,

    /// Level in hierarchy
    pub level: TocLevel,

    /// Human-readable title
    pub title: String,

    /// Start of time period
    pub start_time: DateTime<Utc>,

    /// End of time period
    pub end_time: DateTime<Utc>,

    /// Summary bullet points (with provenance)
    pub bullets: Vec<TocBullet>,

    /// Keywords for search/filtering
    pub keywords: Vec<String>,

    /// Child node IDs (for drill-down)
    pub child_node_ids: Vec<String>,

    /// Version number (append-only versioning)
    pub version: u32,

    /// When this version was created
    pub created_at: DateTime<Utc>,
}

pub struct TocBullet {
    /// Bullet text
    pub text: String,
    /// Grip IDs for provenance
    pub grip_ids: Vec<String>,
}
```

### 3.2 TOC Hierarchy Levels

```rust
enum TocLevel {
    Year,     // Top level
    Month,    // Contains weeks
    Week,     // Contains days
    Day,      // Contains segments
    Segment,  // Leaf node (contains events)
}
```

### 3.3 TOC Node ID Formats (All Temporal)

| Level | Format | Example |
|-------|--------|---------|
| **Year** | `toc:year:{YYYY}` | `toc:year:2026` |
| **Month** | `toc:month:{YYYY}:{MM}` | `toc:month:2026:01` |
| **Week** | `toc:week:{YYYY}:W{WW}` | `toc:week:2026:W04` |
| **Day** | `toc:day:{YYYY-MM-DD}` | `toc:day:2026-01-30` |
| **Segment** | `toc:segment:{YYYY-MM-DD}:{ULID}` | `toc:segment:2026-01-30:01HN4QXKN6...` |

### 3.4 Parent-Child Relationships

```
toc:year:2026
    │
    ├── toc:month:2026:01 (January 2026)
    │       │
    │       ├── toc:week:2026:W04 (Week 4)
    │       │       │
    │       │       ├── toc:day:2026-01-28
    │       │       │       │
    │       │       │       ├── toc:segment:2026-01-28:01HN4ABC...
    │       │       │       └── toc:segment:2026-01-28:01HN4DEF...
    │       │       │
    │       │       ├── toc:day:2026-01-29
    │       │       └── toc:day:2026-01-30
    │       │
    │       └── toc:week:2026:W05
    │
    └── toc:month:2026:02 (February 2026)
```

### 3.5 Title Generation

| Level | Title Format | Example |
|-------|--------------|---------|
| Year | `{YYYY}` | "2026" |
| Month | `{Month} {YYYY}` | "January 2026" |
| Week | `Week {WW} of {YYYY}` | "Week 4 of 2026" |
| Day | `{Weekday}, {Month} {DD}, {YYYY}` | "Thursday, January 30, 2026" |
| Segment | `{Month} {DD}, {YYYY} at {HH:MM}` | "January 30, 2026 at 14:30" |

---

## 4. Grip Schema (Provenance)

Grips link TOC bullet points back to source events for provenance verification.

### 4.1 Grip Data Structure

```rust
pub struct Grip {
    /// Unique identifier
    pub grip_id: String,

    /// The excerpt text anchored to events
    pub excerpt: String,

    /// First event supporting this excerpt
    pub event_id_start: String,

    /// Last event supporting this excerpt
    pub event_id_end: String,

    /// Timestamp of the excerpt
    pub timestamp: DateTime<Utc>,

    /// Source context (which summarizer produced this)
    pub source: String,

    /// Optional: TOC node ID that uses this grip
    pub toc_node_id: Option<String>,
}
```

### 4.2 Grip Key Format

```
Key:   grip:{timestamp_ms:013}:{ulid}

Example:
  grip:1706540400000:01HN4GRIP123ABCDEFGHIJK
```

### 4.3 Provenance Chain

```
TOC Bullet                          Grip                           Events
┌────────────────────┐    ┌─────────────────────────┐    ┌─────────────────┐
│ "Discussed JWT     │───▶│ grip_id: grip:123...    │───▶│ evt:1706540400… │
│  authentication"   │    │ excerpt: "User asked    │    │ evt:1706540401… │
│                    │    │  about JWT auth..."     │    │ evt:1706540402… │
│ grip_ids:          │    │ event_id_start: evt:... │    └─────────────────┘
│   ["grip:123..."]  │    │ event_id_end: evt:...   │
└────────────────────┘    └─────────────────────────┘
```

---

## 5. Column Families Summary

| Column Family | Key Format | Description | Compaction |
|---------------|------------|-------------|------------|
| `CF_EVENTS` | `evt:{ts:013}:{ulid}` | Raw conversation events | Universal + Zstd |
| `CF_TOC_NODES` | `toc:{level}:{time_id}[:v{n}]` | TOC hierarchy nodes (versioned) | Default |
| `CF_TOC_LATEST` | `toc:{level}:{time_id}` | Pointer to latest node version | Default |
| `CF_GRIPS` | `grip:{ts:013}:{ulid}` | Provenance anchors | Default |
| `CF_OUTBOX` | `outbox:{seq:020}` | Async index update queue | FIFO |
| `CF_CHECKPOINTS` | `checkpoint:{job_name}` | Crash recovery state | Default |

---

## 6. Data Flow: Events to TOC

```
┌─────────────────────────────────────────────────────────────────┐
│                         DATA FLOW                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. INGESTION                                                   │
│     ┌────────────┐     IngestEvent RPC      ┌──────────────┐   │
│     │ CCH Hooks  │ ──────────────────────► │ CF_EVENTS    │   │
│     │ (capture)  │                          │ + CF_OUTBOX  │   │
│     └────────────┘                          └──────┬───────┘   │
│                                                    │            │
│  2. SEGMENTATION                                   ▼            │
│     ┌──────────────────────────────────────────────────────┐   │
│     │ SegmentBuilder reads events, detects boundaries:      │   │
│     │   - 30 min time gap → new segment                     │   │
│     │   - 4K tokens → new segment                           │   │
│     │   - 5 min / 500 token overlap for continuity          │   │
│     └──────────────────────────┬───────────────────────────┘   │
│                                │                                │
│  3. SUMMARIZATION              ▼                                │
│     ┌──────────────────────────────────────────────────────┐   │
│     │ Summarizer (LLM) for each segment:                    │   │
│     │   - Generates title, bullets, keywords                │   │
│     │   - Extracts Grips (excerpt + event pointers)         │   │
│     │   - Creates TocNode at segment level                  │   │
│     └──────────────────────────┬───────────────────────────┘   │
│                                │                                │
│  4. ROLLUP                     ▼                                │
│     ┌──────────────────────────────────────────────────────┐   │
│     │ Scheduled jobs aggregate children into parents:       │   │
│     │   Segments → Day → Week → Month → Year               │   │
│     │   Each level summarizes its children                  │   │
│     └──────────────────────────┬───────────────────────────┘   │
│                                │                                │
│  5. QUERY                      ▼                                │
│     ┌──────────────────────────────────────────────────────┐   │
│     │ Agent navigates TOC hierarchy:                        │   │
│     │   GetTocRoot → Year nodes                             │   │
│     │   BrowseToc → Drill down to month/week/day/segment    │   │
│     │   ExpandGrip → Get source events for provenance       │   │
│     └──────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Complete Record Chain Example

This example shows how a single user message flows through the entire system:

### Step 1: Event Ingested

```
User conversation at 2026-01-30 14:30:00 UTC

Key: evt:1706621400000:01HN5ABC123DEFGHIJKLMNOP
Value: {
  "event_id": "01HN5ABC123DEFGHIJKLMNOP",
  "session_id": "session-xyz",
  "timestamp": 1706621400000,
  "event_type": "user_message",
  "role": "user",
  "text": "How do I implement JWT authentication?"
}
```

### Step 2: Segment Created

After time/token threshold is reached:

```
Segment ID: seg:01HN5SEGMENT123ABC
Contains: [event_01, event_02, event_03, ...]
Overlap: [prev_event_05, prev_event_06]  # context from previous
Start: 2026-01-30T14:30:00Z
End: 2026-01-30T15:15:00Z
Tokens: 2847
```

### Step 3: TOC Node Created (Segment Level)

```
Key: toc:segment:2026-01-30:01HN5SEGMENT123ABC
Value: {
  "node_id": "toc:segment:2026-01-30:01HN5SEGMENT123ABC",
  "level": "segment",
  "title": "January 30, 2026 at 14:30",
  "start_time": 1706621400000,
  "end_time": 1706624100000,
  "bullets": [
    {
      "text": "Discussed JWT authentication implementation",
      "grip_ids": ["grip:1706621400000:01HN5GRIP456DEF"]
    },
    {
      "text": "Reviewed token expiration strategies",
      "grip_ids": ["grip:1706622000000:01HN5GRIP789GHI"]
    }
  ],
  "keywords": ["JWT", "authentication", "tokens", "security"],
  "child_node_ids": [],
  "version": 1
}
```

### Step 4: Grip Created (Provenance)

```
Key: grip:1706621400000:01HN5GRIP456DEF
Value: {
  "grip_id": "01HN5GRIP456DEF",
  "excerpt": "User asked: How do I implement JWT authentication?",
  "event_id_start": "01HN5ABC123DEFGHIJKLMNOP",
  "event_id_end": "01HN5ABC456GHIJKLMNOPQRS",
  "timestamp": 1706621400000,
  "source": "segment_summarizer",
  "toc_node_id": "toc:segment:2026-01-30:01HN5SEGMENT123ABC"
}
```

### Step 5: TOC Node Created (Day Level) After Rollup

```
Key: toc:day:2026-01-30
Value: {
  "node_id": "toc:day:2026-01-30",
  "level": "day",
  "title": "Thursday, January 30, 2026",
  "start_time": 1706572800000,
  "end_time": 1706659199999,
  "bullets": [
    {
      "text": "JWT authentication implementation discussion",
      "grip_ids": ["grip:1706621400000:01HN5GRIP456DEF"]
    },
    {
      "text": "Database migration planning",
      "grip_ids": ["grip:1706635800000:01HN5GRIPXYZABC"]
    }
  ],
  "keywords": ["JWT", "authentication", "database", "migration"],
  "child_node_ids": [
    "toc:segment:2026-01-30:01HN5SEGMENT123ABC",
    "toc:segment:2026-01-30:01HN5SEGMENT789DEF"
  ],
  "version": 1
}
```

### Step 6: Higher Levels Roll Up Similarly

```
Week:  toc:week:2026:W05
Month: toc:month:2026:01
Year:  toc:year:2026
```

---

## 8. Query Examples

### 8.1 Get Events in Time Range

```rust
// Query: Events from 14:00 to 15:00 on 2026-01-30
let start_key = "evt:1706619600000:";  // 14:00
let end_key = "evt:1706623200000:";    // 15:00

// RocksDB prefix iteration
for (key, value) in db.range(start_key..end_key) {
    let event = Event::from_bytes(&value)?;
    // Process event
}
```

### 8.2 Navigate TOC Hierarchy

```rust
// Start at year level
let years = get_toc_root()?;  // Returns year nodes

// Drill down to January
let jan = get_node("toc:month:2026:01")?;

// Get weeks in January
let weeks = browse_toc("toc:month:2026:01", limit=10)?;

// Drill to specific day
let day = get_node("toc:day:2026-01-30")?;

// Get segments for that day
let segments = browse_toc("toc:day:2026-01-30", limit=20)?;
```

### 8.3 Verify Provenance via Grip

```rust
// From a TOC bullet, get the grip
let grip_id = bullet.grip_ids[0];
let grip = get_grip(&grip_id)?;

// Expand to see source events
let context = expand_grip(&grip_id, events_before=3, events_after=3)?;
// Returns: original events that support the bullet claim
```

---

## 9. Key Design Principles

### 9.1 Why Time-Prefixed Keys?

1. **Lexicographic = Chronological**: Zero-padded timestamps sort correctly as strings
2. **Efficient Range Scans**: RocksDB prefix iteration for time queries
3. **Natural Partitioning**: Data naturally partitions by time for compaction

### 9.2 Why Append-Only?

1. **Simplicity**: No update conflicts or merge logic
2. **Auditability**: Complete history preserved
3. **Recovery**: No partial states after crashes
4. **Performance**: Sequential writes optimize for SSDs

### 9.3 Why ULID over UUID?

1. **Timestamp Embedded**: Can reconstruct key from just the ULID
2. **Lexicographic Sorting**: ULIDs sort chronologically
3. **Uniqueness**: 80 bits of randomness prevents collisions

### 9.4 Why Versioned TOC Nodes?

1. **History Preservation**: See how summaries evolved
2. **Safe Updates**: Atomic pointer swap prevents inconsistent reads
3. **Rollback Capability**: Can revert to previous version

---

## 10. Storage Estimates

| Entity | Avg Size | Count (1 year, active user) | Total |
|--------|----------|----------------------------|-------|
| Events | 500 bytes | 100,000 | 50 MB |
| TOC Nodes | 2 KB | 5,000 | 10 MB |
| Grips | 300 bytes | 20,000 | 6 MB |
| **Total** | | | **~66 MB/year** |

With Zstd compression on events: **~25-30 MB/year**

---

## Summary

The agent-memory data schema is built around:

- **Events**: Immutable conversation records with time-prefixed keys (`evt:{ts}:{ulid}`)
- **Segments**: Groups of events formed by time gaps (30m) or token limits (4K)
- **TOC Nodes**: Hierarchical summaries (Year → Month → Week → Day → Segment)
- **Grips**: Provenance anchors linking bullets to source events

All keys are temporal, enabling efficient time-based queries. The append-only design ensures durability and simplicity, while the TOC hierarchy enables progressive disclosure navigation.
