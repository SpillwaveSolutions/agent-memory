# Storage Architecture

**Document Version:** 1.0
**Last Updated:** 2026-01-31
**Status:** Authoritative Reference

## Overview

The agent-memory storage layer provides durable, append-only persistence for conversation events and their derived summaries. Built on RocksDB with a separate Tantivy index for full-text search, the architecture prioritizes:

1. **Append-only semantics** - Events are immutable once written
2. **Time-based ordering** - Efficient range queries by timestamp
3. **Transactional consistency** - Atomic batch writes with outbox pattern
4. **Crash recovery** - Checkpoints and idempotent operations
5. **Separation of concerns** - Column families isolate different data types

---

## 1. Storage Philosophy

### 1.1 Append-Only Design Rationale

The storage layer treats events as immutable facts. Once an event is written, it is never modified or deleted during normal operation.

**Benefits:**
- **Simplicity**: No complex update logic or conflict resolution
- **Auditability**: Complete history preserved for debugging and compliance
- **Performance**: Sequential writes optimize for SSDs and log-structured merge trees
- **Recovery**: No partial states to reconcile after crashes

**Trade-offs:**
- Storage growth requires periodic compaction
- Corrections require new events (not mutations)
- Deleted content remains in storage until compacted

### 1.2 Immutability Benefits

```
Event Timeline (immutable):
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ E-001  │───▶│ E-002  │───▶│ E-003  │───▶│ E-004  │
│ User   │    │ Agent  │    │ Tool   │    │ Agent  │
│ 10:00  │    │ 10:01  │    │ 10:02  │    │ 10:03  │
└────────┘    └────────┘    └────────┘    └────────┘
     │
     ▼
  Cannot modify E-001; only append new events
```

Immutability enables:
- **Lock-free reads**: No coordination needed between readers
- **Safe iteration**: Range scans see consistent snapshot
- **Simple replication**: Just ship new events
- **Predictable I/O**: Write-once pattern

### 1.3 Why RocksDB Was Chosen

| Requirement | RocksDB Capability |
|-------------|-------------------|
| Embedded database | Single-process, no network dependency |
| Column families | Logical separation without multiple databases |
| Universal compaction | Optimal for append-only workloads |
| Compression | Built-in Zstd support reduces disk usage |
| Durability | Write-ahead log guarantees persistence |
| Range scans | Efficient prefix iteration for time queries |
| Rust bindings | Mature `rust-rocksdb` crate |

**Alternatives Considered:**
- **SQLite**: Less efficient for time-series append patterns
- **sled**: Less mature, uncertain maintenance status
- **LMDB**: Single-writer limitation problematic for concurrent ingestion

---

## 2. RocksDB Column Families

Column families provide logical isolation within a single RocksDB instance. Each CF has its own LSM tree, memtable, and compaction settings.

### 2.1 Column Family Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         RocksDB Instance                                 │
│                   (~/.local/share/agent-memory/db/)                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   events    │  │  toc_nodes  │  │ toc_latest  │  │      grips      │ │
│  │             │  │             │  │             │  │                 │ │
│  │ Append-only │  │  Versioned  │  │   Pointer   │  │    Provenance   │ │
│  │   events    │  │    nodes    │  │    table    │  │     anchors     │ │
│  │             │  │             │  │             │  │                 │ │
│  │ Universal   │  │   Default   │  │   Default   │  │     Default     │ │
│  │ Compaction  │  │ Compaction  │  │ Compaction  │  │   Compaction    │ │
│  │ + Zstd      │  │             │  │             │  │                 │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────┘ │
│                                                                          │
│  ┌─────────────────────────────┐  ┌─────────────────────────────────┐   │
│  │          outbox             │  │         checkpoints             │   │
│  │                             │  │                                 │   │
│  │    Async work queue         │  │     Job recovery state          │   │
│  │                             │  │                                 │   │
│  │    FIFO Compaction          │  │     Default Compaction          │   │
│  └─────────────────────────────┘  └─────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Column Family Details

| Column Family | Purpose | Key Pattern | Value Type | Compaction |
|--------------|---------|-------------|------------|------------|
| `events` | Raw conversation events | `evt:{ts:013}:{ulid}` | Event JSON | Universal + Zstd |
| `toc_nodes` | Versioned TOC hierarchy | `toc:{node_id}:v{ver:06}` | TocNode JSON | Default |
| `toc_latest` | Latest version pointers | `latest:{node_id}` | u32 (big-endian) | Default |
| `grips` | Provenance anchors | `{grip_id}` or `node:{id}:{grip}` | Grip JSON | Default |
| `outbox` | Async processing queue | `outbox:{seq:020}` | OutboxEntry JSON | FIFO |
| `checkpoints` | Job recovery state | `checkpoint:{job_name}` | Checkpoint bytes | Default |

### 2.3 Access Patterns by Column Family

**events CF:**
- Write: Sequential append during ingestion
- Read: Time-range scans for context retrieval
- Pattern: Write-heavy, range-read-heavy

**toc_nodes CF:**
- Write: Append new version on summary update
- Read: Point lookup by versioned key
- Pattern: Read-heavy after initial summarization

**toc_latest CF:**
- Write: Update pointer when new version written
- Read: Point lookup to find current version
- Pattern: Balanced read/write

**grips CF:**
- Write: Batch insert during summarization
- Read: Lookup by grip_id or scan by node
- Pattern: Read-heavy for provenance checks

**outbox CF:**
- Write: Append with every event ingestion
- Read: Sequential scan from head
- Delete: After successful processing
- Pattern: Queue (FIFO)

**checkpoints CF:**
- Write: Periodic job progress saves
- Read: On startup for recovery
- Pattern: Low volume, high durability

---

## 3. Key Design

The key format enables efficient time-based queries while maintaining uniqueness and ordering.

### 3.1 Time-Prefixed Key Architecture

```
Key Format: {prefix}:{timestamp_ms:013}:{ulid}

┌────────────────────────────────────────────────────────────────┐
│                        Event Key Example                        │
├────────────────────────────────────────────────────────────────┤
│   evt:1706540400000:01HPXYZ123456789ABCDEFGH                   │
│   │   │             │                                          │
│   │   │             └── ULID (26 chars, Crockford Base32)      │
│   │   │                 - Encodes timestamp + random            │
│   │   │                 - Globally unique                       │
│   │   │                 - Lexicographically sortable           │
│   │   │                                                        │
│   │   └── Timestamp (13 digits, zero-padded)                   │
│   │       - Milliseconds since Unix epoch                      │
│   │       - 1706540400000 = 2024-01-29T15:00:00Z               │
│   │       - Zero-padding ensures lexicographic = chronological │
│   │                                                            │
│   └── Prefix                                                   │
│       - "evt" for events                                       │
│       - Enables CF-wide prefix scans                           │
└────────────────────────────────────────────────────────────────┘
```

### 3.2 Why This Format Enables Efficient Queries

**Lexicographic ordering equals chronological ordering:**

```
Earlier event:  evt:0001706540400000:01HPXYZ...
Later event:    evt:0001706540401000:01HPXYZ...
                    ▲
                    │
        String comparison works for time ordering
```

**Range scans are prefix iterations:**

```rust
// Get all events between two timestamps
fn get_events_in_range(start_ms: i64, end_ms: i64) {
    let start_key = format!("evt:{:013}:", start_ms);
    let end_key = format!("evt:{:013}:", end_ms);

    // RocksDB iterates forward from start_key
    // Stop when key >= end_key
}
```

### 3.3 Key Format Reference

| Entity | Key Format | Example |
|--------|------------|---------|
| Event | `evt:{ts:013}:{ulid}` | `evt:1706540400000:01HPXYZ123456789ABCDEFGH` |
| TOC Node (versioned) | `toc:{node_id}:v{ver:06}` | `toc:day:2024-01-29:v000003` |
| TOC Latest | `latest:{node_id}` | `latest:toc:day:2024-01-29` |
| Grip | `{grip_id}` | `grip:1706540400000:test123` |
| Grip Index | `node:{node_id}:{grip_id}` | `node:toc:day:2024-01-29:grip:123` |
| Outbox | `outbox:{seq:020}` | `outbox:00000000000000000042` |
| Checkpoint | `checkpoint:{job_name}` | `checkpoint:segmenter` |

### 3.4 ULID Embedded Timestamp

The ULID format encodes timestamp in its first 48 bits:

```
ULID: 01HPXYZ123456789ABCDEFGH
      └──────┴──────────────┘
      Time    Randomness
      (48b)   (80b)

// Extract timestamp from ULID
let ulid: Ulid = event_id.parse()?;
let timestamp_ms = ulid.timestamp_ms() as i64;
```

This allows reconstruction of the full key from just the event_id.

---

## 4. Event Storage

### 4.1 Event Serialization

Events are serialized as JSON for storage:

```rust
// From crates/memory-types/src/event.rs
#[derive(Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,           // ULID
    pub session_id: String,         // Session identifier
    pub timestamp: DateTime<Utc>,   // Source timestamp
    pub event_type: EventType,      // UserMessage, AssistantMessage, etc.
    pub role: EventRole,            // User, Assistant, System, Tool
    pub text: String,               // Content
    pub metadata: HashMap<String, String>,  // Optional key-value pairs
}
```

**Serialized example:**
```json
{
  "event_id": "01HPXYZ123456789ABCDEFGH",
  "session_id": "session-abc123",
  "timestamp": 1706540400000,
  "event_type": "user_message",
  "role": "user",
  "text": "How do I configure RocksDB?",
  "metadata": {}
}
```

### 4.2 Time-Range Iteration

```
Query: Get events from 10:00 to 10:30 on 2024-01-29

Start timestamp: 1706526000000 (10:00:00 UTC)
End timestamp:   1706527800000 (10:30:00 UTC)

RocksDB Iteration:
┌────────────────────────────────────────────────────────────┐
│                        events CF                            │
├────────────────────────────────────────────────────────────┤
│ evt:1706525900000:...  │ 09:58:20  │ ← Before range, skip   │
│ evt:1706525999000:...  │ 09:59:59  │ ← Before range, skip   │
│─────────────────────────────────────────────────────────────│
│ evt:1706526000000:...  │ 10:00:00  │ ← Start iteration here │
│ evt:1706526030000:...  │ 10:00:30  │ ← Include              │
│ evt:1706526120000:...  │ 10:02:00  │ ← Include              │
│ evt:1706527700000:...  │ 10:28:20  │ ← Include              │
│─────────────────────────────────────────────────────────────│
│ evt:1706527800000:...  │ 10:30:00  │ ← Stop here (exclusive)│
│ evt:1706527900000:...  │ 10:31:40  │ ← After range, ignore  │
└────────────────────────────────────────────────────────────┘
```

### 4.3 Atomic Batch Writes

Every event write includes an outbox entry for transactional consistency:

```rust
// From crates/memory-storage/src/db.rs
pub fn put_event(
    &self,
    event_id: &str,
    event_bytes: &[u8],
    outbox_bytes: &[u8],
) -> Result<(EventKey, bool), StorageError> {
    // Check for idempotent write
    if self.db.get_cf(&events_cf, event_key.to_bytes())?.is_some() {
        return Ok((event_key, false));  // Already exists
    }

    // Atomic write: event + outbox entry
    let mut batch = WriteBatch::default();
    batch.put_cf(&events_cf, event_key.to_bytes(), event_bytes);
    batch.put_cf(&outbox_cf, outbox_key.to_bytes(), outbox_bytes);

    self.db.write(batch)?;  // Atomic commit
    Ok((event_key, true))
}
```

**Atomicity guarantees:**
- Both event and outbox entry written together
- If crash before write completes, neither is visible
- WAL ensures durability after write returns

---

## 5. TOC Storage

### 5.1 Node Versioning (Append New Versions)

TOC nodes are versioned to preserve history and enable safe updates:

```
TOC Node Versioning Flow:

Initial creation:
┌─────────────────────────────────────────────────────────┐
│ toc_latest                │ toc_nodes                   │
├───────────────────────────┼─────────────────────────────┤
│ latest:toc:day:2024-01-29 │ toc:day:2024-01-29:v000001 │
│         = 1               │         = { v1 content }   │
└───────────────────────────┴─────────────────────────────┘

After summary update:
┌─────────────────────────────────────────────────────────┐
│ toc_latest                │ toc_nodes                   │
├───────────────────────────┼─────────────────────────────┤
│ latest:toc:day:2024-01-29 │ toc:day:2024-01-29:v000001 │
│         = 2               │         = { v1 content }   │
│         ▲                 │ toc:day:2024-01-29:v000002 │
│         │ updated         │         = { v2 content }   │
└─────────│─────────────────┴─────────────────────────────┘
           │
    Latest pointer updated atomically with new version
```

### 5.2 Latest Pointer Pattern

The two-column-family pattern separates versioned data from version pointers:

```rust
// Write new version atomically with pointer update
pub fn put_toc_node(&self, node: &TocNode) -> Result<(), StorageError> {
    // Get current version
    let latest_key = format!("latest:{}", node.node_id);
    let current_version = self.get_latest_version(&latest_key)?;
    let new_version = current_version + 1;

    let versioned_key = format!("toc:{}:v{:06}", node.node_id, new_version);

    // Atomic write: new version + updated pointer
    let mut batch = WriteBatch::default();
    batch.put_cf(&nodes_cf, versioned_key, &node_bytes);
    batch.put_cf(&latest_cf, &latest_key, &new_version.to_be_bytes());

    self.db.write(batch)?;
}
```

**Benefits:**
- Version history preserved for debugging
- Atomic pointer update prevents inconsistent reads
- Easy rollback by updating pointer

### 5.3 Parent-Child Indexing

TOC nodes store child references for hierarchy traversal:

```rust
pub struct TocNode {
    pub node_id: String,
    pub level: TocLevel,           // Year, Month, Week, Day, Segment
    pub child_node_ids: Vec<String>,  // References to children
    // ...
}
```

**Hierarchy navigation:**

```
Year Node (toc:year:2024)
    └── child_node_ids: ["toc:month:2024-01", "toc:month:2024-02", ...]
        │
        ▼
Month Node (toc:month:2024-01)
    └── child_node_ids: ["toc:week:2024-w01", "toc:week:2024-w02", ...]
        │
        ▼
Week Node (toc:week:2024-w04)
    └── child_node_ids: ["toc:day:2024-01-22", "toc:day:2024-01-23", ...]
        │
        ▼
Day Node (toc:day:2024-01-29)
    └── child_node_ids: ["toc:segment:2024-01-29:abc", ...]
        │
        ▼
Segment Node (toc:segment:2024-01-29:abc123)
    └── child_node_ids: []  // Leaf node
```

---

## 6. Grip Storage

### 6.1 Grip ID Format

Grips anchor summaries to source events for provenance:

```
Grip ID Format: grip:{timestamp_ms:013}:{random_suffix}

Example: grip:1706540400000:test123

┌──────────────────────────────────────────────────────────────┐
│                          Grip                                 │
├──────────────────────────────────────────────────────────────┤
│ grip_id: "grip:1706540400000:test123"                        │
│ excerpt: "User asked about JWT authentication"                │
│ event_id_start: "01HPXYZ..."  ← First event in range         │
│ event_id_end: "01HPXYZ..."    ← Last event in range          │
│ timestamp: 2024-01-29T15:00:00Z                              │
│ source: "segment_summarizer"                                  │
│ toc_node_id: Some("toc:day:2024-01-29")                      │
└──────────────────────────────────────────────────────────────┘
```

### 6.2 Node-to-Grip Index

When grips are linked to TOC nodes, an index entry enables efficient lookup:

```
grips CF Data Layout:

Primary grip entry (by grip_id):
┌───────────────────────────────────┬───────────────────────┐
│ Key                               │ Value                 │
├───────────────────────────────────┼───────────────────────┤
│ grip:1706540400000:test123        │ { full grip JSON }    │
└───────────────────────────────────┴───────────────────────┘

Index entry (by node):
┌───────────────────────────────────┬───────────────────────┐
│ Key                               │ Value                 │
├───────────────────────────────────┼───────────────────────┤
│ node:toc:day:2024-01-29:grip:123  │ (empty)               │
└───────────────────────────────────┴───────────────────────┘
```

**Lookup by node (prefix scan):**

```rust
pub fn get_grips_for_node(&self, node_id: &str) -> Result<Vec<Grip>, StorageError> {
    let prefix = format!("node:{}:", node_id);

    // Iterate index entries
    for (key, _) in self.prefix_scan(&grips_cf, &prefix) {
        let grip_id = extract_grip_id_from_key(&key);
        let grip = self.get_grip(&grip_id)?;
        grips.push(grip);
    }
}
```

### 6.3 Efficient Provenance Lookup

```
Provenance Query Flow:

1. TOC bullet references grip_ids
   TocBullet { text: "Discussed authentication", grip_ids: ["grip:123"] }

2. Lookup grip by ID
   get_grip("grip:1706540400000:test123") → Grip

3. Grip contains event range
   event_id_start → event_id_end

4. Retrieve source events
   get_events_in_range(start_ms, end_ms) → Vec<Event>

Result: Original conversation that supports the summary
```

---

## 7. Outbox Pattern

### 7.1 Transactional Consistency

The outbox pattern ensures index updates are never lost:

```
Event Ingestion with Outbox:

┌─────────────────────────────────────────────────────────────────┐
│                     Atomic Write Batch                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Event written to events CF                                   │
│     ┌────────────────────────────────────────────────────────┐  │
│     │ evt:1706540400000:01HPXYZ... → { event JSON }         │  │
│     └────────────────────────────────────────────────────────┘  │
│                                                                  │
│  2. Outbox entry written to outbox CF                            │
│     ┌────────────────────────────────────────────────────────┐  │
│     │ outbox:00000000000000000042 → { event_id, action }    │  │
│     └────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Both writes committed atomically via WriteBatch                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              Background Worker (async, separate thread)
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Worker reads outbox entry                                    │
│  4. Processes action (update TOC, index for search)             │
│  5. Deletes outbox entry after successful processing            │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 At-Least-Once Delivery

The outbox guarantees at-least-once delivery semantics:

```
Failure Scenarios and Recovery:

Scenario A: Crash before processing
┌────────────┐     ┌────────────┐
│ Write      │────▶│   CRASH    │
│ Event +    │     │            │
│ Outbox     │     └────────────┘
└────────────┘            │
                          ▼
                  On Restart:
                  ┌────────────────┐
                  │ Outbox entry   │
                  │ still present  │───▶ Reprocessed
                  └────────────────┘

Scenario B: Crash during processing
┌────────────┐     ┌────────────┐     ┌────────────┐
│ Read       │────▶│ Process    │────▶│   CRASH    │
│ Outbox     │     │ (partial)  │     │            │
└────────────┘     └────────────┘     └────────────┘
                                             │
                                             ▼
                                     On Restart:
                                     ┌────────────────┐
                                     │ Outbox entry   │
                                     │ not deleted    │───▶ Reprocessed
                                     └────────────────┘

Key: Outbox entry only deleted AFTER successful processing
     Processing must be idempotent
```

### 7.3 Checkpoint Recovery

```rust
// Checkpoint structure
pub struct Checkpoint {
    job_name: String,
    last_processed_sequence: u64,
    last_processed_timestamp: i64,
    saved_at: DateTime<Utc>,
}
```

**Recovery flow:**

```
Daemon Startup:

1. Open RocksDB
   └── Recovers WAL automatically

2. Load checkpoint for each job
   ┌─────────────────────────────────────┐
   │ get_checkpoint("segmenter")         │
   │ → last_processed_sequence: 12345    │
   └─────────────────────────────────────┘

3. Scan outbox from checkpoint
   ┌─────────────────────────────────────┐
   │ Iterate outbox where seq > 12345    │
   │ Process each entry                  │
   │ Update checkpoint periodically      │
   └─────────────────────────────────────┘

4. Resume normal processing
```

---

## 8. Tantivy Index

The Tantivy full-text search index operates separately from RocksDB, providing BM25-scored keyword search.

### 8.1 Separate MmapDirectory

```
Storage Layout:

~/.local/share/agent-memory/
├── db/                          ← RocksDB instance
│   ├── CURRENT
│   ├── MANIFEST-*
│   ├── OPTIONS-*
│   ├── *.sst                    ← SST files (per CF)
│   └── ...
│
└── bm25-index/                  ← Tantivy index (separate)
    ├── meta.json                ← Index metadata
    ├── .tantivy-meta.lock       ← Writer lock
    └── *.managed.idx            ← Segment files
```

**Why separate directories:**
- Independent lifecycle management
- Can rebuild search index without touching RocksDB
- Different compaction/merge strategies
- Tantivy uses memory-mapped files (MmapDirectory)

### 8.2 Schema Design

```rust
// From .planning/phases/11-bm25-teleport-tantivy/11-RESEARCH.md
pub fn build_teleport_schema() -> Schema {
    let mut builder = Schema::builder();

    // Document type: "toc_node" or "grip"
    builder.add_text_field("doc_type", STRING | STORED);

    // Primary key: node_id or grip_id
    builder.add_text_field("doc_id", STRING | STORED);

    // TOC level for filtering
    builder.add_text_field("level", STRING);

    // Searchable text content
    builder.add_text_field("text", TEXT);

    // Keywords for search
    builder.add_text_field("keywords", TEXT | STORED);

    // Timestamp for recency
    builder.add_text_field("timestamp_ms", STRING | STORED);

    builder.build()
}
```

### 8.3 Relationship to RocksDB

```
Data Flow: RocksDB ──▶ Tantivy

1. Event ingested to RocksDB
   ┌────────────────────────────────────────────┐
   │ events CF: full event JSON                 │
   │ outbox CF: IndexEvent action               │
   └────────────────────────────────────────────┘
                      │
                      ▼
2. Background worker processes outbox
   ┌────────────────────────────────────────────┐
   │ Read event from RocksDB                    │
   │ Extract searchable fields                  │
   └────────────────────────────────────────────┘
                      │
                      ▼
3. Index document in Tantivy
   ┌────────────────────────────────────────────┐
   │ tantivy::IndexWriter::add_document()       │
   │ Periodic commit (every minute)             │
   └────────────────────────────────────────────┘
                      │
                      ▼
4. Search queries use Tantivy
   ┌────────────────────────────────────────────┐
   │ Query: "authentication JWT"                │
   │ Result: [doc_id: "toc:day:2024-01-29"]    │
   └────────────────────────────────────────────┘
                      │
                      ▼
5. Fetch full content from RocksDB
   ┌────────────────────────────────────────────┐
   │ get_toc_node("toc:day:2024-01-29")        │
   │ Return complete TocNode                    │
   └────────────────────────────────────────────┘
```

**Key principle:** Tantivy stores only IDs and searchable text. Full content fetched from RocksDB.

---

## 9. Compaction Strategy

### 9.1 Scheduled Compaction

RocksDB compaction runs automatically, but can be triggered manually:

```rust
// From crates/memory-storage/src/db.rs
pub fn compact(&self) -> Result<(), StorageError> {
    // Full database compaction
    self.db.compact_range::<&[u8], &[u8]>(None, None);

    // Per-CF compaction
    for cf_name in ALL_CF_NAMES {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            self.db.compact_range_cf::<&[u8], &[u8]>(&cf, None, None);
        }
    }
    Ok(())
}
```

**When to trigger manual compaction:**
- After bulk imports
- Before backup/snapshot
- When disk space is constrained
- Scheduled maintenance window

### 9.2 Level-Based vs. Universal Compaction

```
Universal Compaction (events CF):

Good for append-only workloads:
┌────────────────────────────────────────────────────────────┐
│ L0: [SST-new] [SST-new] [SST-new]                         │
│                    │                                       │
│                    ▼ Size ratio trigger                   │
│ L1: [  Merged SST files  ]                                │
│                    │                                       │
│                    ▼ Size ratio trigger                   │
│ L2: [     Larger merged SST      ]                        │
└────────────────────────────────────────────────────────────┘

Benefits:
- Lower write amplification
- Better for time-series data
- Simpler merge policy

Default Compaction (other CFs):

Leveled for point lookups:
┌────────────────────────────────────────────────────────────┐
│ L0: Unsorted SST files                                     │
│ L1: Sorted, non-overlapping                                │
│ L2: 10x size of L1, sorted                                 │
│ L3: 10x size of L2, sorted                                 │
└────────────────────────────────────────────────────────────┘

Benefits:
- Better read amplification
- Predictable point lookup latency
- Good for key-value patterns
```

### 9.3 Performance Tuning

**Configuration in db.rs:**

```rust
pub fn open(path: &Path) -> Result<Self, StorageError> {
    let mut db_opts = Options::default();

    // Universal compaction for append-only
    db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);

    // Background job parallelism
    db_opts.set_max_background_jobs(4);

    // ...
}
```

**Column family tuning:**

```rust
// events CF: Zstd compression for space efficiency
fn events_options() -> Options {
    let mut opts = Options::default();
    opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
    opts
}

// outbox CF: FIFO for queue behavior
fn outbox_options() -> Options {
    let mut opts = Options::default();
    opts.set_compaction_style(rocksdb::DBCompactionStyle::Fifo);
    opts.set_fifo_compaction_options(&rocksdb::FifoCompactOptions::default());
    opts
}
```

**Operational recommendations:**

| Setting | Recommended Value | Rationale |
|---------|------------------|-----------|
| `max_background_jobs` | 4 | Balance between throughput and CPU |
| Compression | Zstd for events | 3-5x compression, good speed |
| WAL size | Default (64MB) | Sufficient for batch writes |
| Block cache | 128MB | Cache hot blocks in memory |
| Write buffer | 64MB | Buffer before flush |

---

## 10. Operational Guidance

### 10.1 Storage Statistics

```rust
// Get database statistics
pub fn get_stats(&self) -> Result<StorageStats, StorageError> {
    StorageStats {
        event_count: count_cf_entries(&events_cf),
        toc_node_count: count_cf_entries(&toc_nodes_cf),
        grip_count: count_cf_entries(&grips_cf),
        outbox_count: count_cf_entries(&outbox_cf),
        disk_usage_bytes: get_disk_usage(),
    }
}
```

### 10.2 Monitoring Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `outbox_count` | Pending work items | > 1000 (processing lag) |
| `disk_usage_bytes` | Total storage size | > 80% of disk |
| `event_count` | Total events stored | Informational |
| `compaction_pending_bytes` | Bytes waiting for compaction | > 1GB |

### 10.3 Backup and Recovery

```bash
# Option 1: RocksDB checkpoint (online, consistent)
# Creates a hard-link based snapshot
rocksdb_checkpoint /source/db /backup/db

# Option 2: File copy (offline)
systemctl stop memory-daemon
cp -r ~/.local/share/agent-memory/db /backup/
systemctl start memory-daemon

# Tantivy index can be rebuilt from RocksDB
# Use rebuild command after restoring RocksDB
memory-admin rebuild-index
```

### 10.4 Disaster Recovery

```
Recovery Procedure:

1. Stop daemon
   systemctl stop memory-daemon

2. Restore RocksDB from backup
   rm -rf ~/.local/share/agent-memory/db
   cp -r /backup/db ~/.local/share/agent-memory/db

3. Remove Tantivy index (will be rebuilt)
   rm -rf ~/.local/share/agent-memory/bm25-index

4. Start daemon
   systemctl start memory-daemon

5. Trigger index rebuild
   memory-admin rebuild-index

6. Verify
   memory-admin stats
```

---

## Summary

The agent-memory storage architecture combines RocksDB for durable persistence with Tantivy for full-text search:

- **RocksDB** provides append-only event storage with column family isolation
- **Time-prefixed keys** enable efficient range scans for context retrieval
- **Outbox pattern** ensures reliable async processing without message loss
- **TOC versioning** preserves history while enabling safe updates
- **Grips** anchor summaries to source events for provenance
- **Tantivy** delivers BM25-scored keyword search independently of primary storage
- **Checkpoints** enable crash recovery with at-least-once semantics

This design prioritizes durability, queryability, and operational simplicity while maintaining the flexibility to evolve the search layer independently.
