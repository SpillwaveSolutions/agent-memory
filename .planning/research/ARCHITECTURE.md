# Architecture Patterns

**Domain:** Hierarchical Conversational Memory System with Time-Based Navigation
**Researched:** 2026-01-29

## Executive Summary

This document describes the architecture for a Rust-based conversational memory system with hierarchical time-based navigation. The system uses a Table of Contents (TOC) tree as its primary navigation mechanism, with append-only event storage in RocksDB. Teleport indexes (BM25/vector) serve as optional accelerators, not dependencies.

The architecture draws from several established patterns:
- **H-MEM (Hierarchical Memory)** patterns for multi-layer memory organization
- **TimescaleDB continuous aggregates** for hierarchical rollup strategies
- **Transactional outbox pattern** for reliable index updates
- **RocksDB column families** for workload isolation

**Confidence:** HIGH (patterns well-established, user design decisions clear)

---

## Recommended Architecture

```
                                    +-----------------+
                                    |   Agent/CLI     |
                                    |   (Query)       |
                                    +--------+--------+
                                             |
                                             | gRPC
                                             v
+---------------+    gRPC      +---------------------------+
| Hook Handler  |------------->|      Memory Daemon        |
| (Ingestion)   |              |                           |
+---------------+              |  +---------------------+  |
                               |  |    Service Layer    |  |
                               |  | (tonic gRPC server) |  |
                               |  +----------+----------+  |
                               |             |              |
                               |  +----------v----------+  |
                               |  |   Domain Layer      |  |
                               |  | (TOC, Events, Grips)|  |
                               |  +----------+----------+  |
                               |             |              |
                               |  +----------v----------+  |
                               |  |   Storage Layer     |  |
                               |  |   (RocksDB)         |  |
                               +--+---------+-----------+--+
                                            |
                          +-----------------+-----------------+
                          |                 |                 |
                    +-----v-----+     +-----v-----+     +-----v-----+
                    |  Outbox   |     | Tantivy   |     |   HNSW    |
                    |  Relay    |     |  (BM25)   |     |  (Vector) |
                    +-----------+     +-----------+     +-----------+
```

### Component Boundaries

| Component | Responsibility | Communicates With | Boundary Type |
|-----------|----------------|-------------------|---------------|
| **Hook Handler** | Captures agent events, forwards via gRPC | Memory Daemon (gRPC) | External process |
| **Memory Daemon** | Central service: storage, TOC management, query | Hooks, CLI, Agents (gRPC) | Single binary |
| **Service Layer** | gRPC endpoint handling, request validation | Domain Layer (Rust calls) | Module boundary |
| **Domain Layer** | Business logic: TOC building, segmentation, rollup | Storage Layer (Rust calls) | Module boundary |
| **Storage Layer** | RocksDB operations, key encoding, column families | RocksDB (FFI) | Module boundary |
| **Outbox Relay** | Async index updates from outbox queue | Tantivy, HNSW (library calls) | Background task |
| **Tantivy Index** | BM25 keyword search | Outbox Relay (writes), Domain (reads) | Embedded library |
| **HNSW Index** | Vector similarity search | Outbox Relay (writes), Domain (reads) | Embedded library |

---

## Data Flow

### Ingestion Path (Hot Path)

```
Hook Event
    |
    v
[Hook Handler] --gRPC--> [IngestEvent RPC]
                              |
                              v
                    [Validate & Transform]
                              |
                              v
                    [Write to events CF]
                              |
                              +---> [Write to outbox CF] (for index updates)
                              |
                              v
                    [Return EventId]
```

**Key Properties:**
- Single writer to RocksDB (daemon owns all writes)
- Atomic write of event + outbox entry (same WriteBatch)
- Hook handlers are fire-and-forget after acknowledgment

**Key Layout:**
```
events CF:     evt:{ts_ms}:{ulid}  ->  Event (protobuf/msgpack)
outbox CF:     out:{seq}           ->  OutboxEntry
```

### TOC Building Path (Background)

```
[Periodic Timer or Event Threshold]
    |
    v
[BuildToc Job]
    |
    +---> [Read events in time window]
    |
    +---> [Segment by threshold (30min or 4K tokens)]
    |
    +---> [Summarize via Summarizer trait]
    |
    +---> [Write segment TOC nodes]
    |
    +---> [Update parent nodes (day/week/month)]
    |
    +---> [Write checkpoint to checkpoints CF]
    |
    v
[toc_nodes CF updated]
```

**Rollup Hierarchy:**
```
Year
  |
  +-- Month (rollup: week summaries)
        |
        +-- Week (rollup: day summaries)
              |
              +-- Day (rollup: segment summaries)
                    |
                    +-- Segment (30min or token-based, with overlap)
                          |
                          +-- [Events referenced by time_range]
```

**Key Layout:**
```
toc_nodes CF:   toc:{node_id}:{version}  ->  TocNode
toc_latest CF:  latest:{node_id}         ->  version (for fast lookup)
checkpoints CF: ckpt:{job_type}          ->  CheckpointState
```

### Query Path (Agent Navigation)

```
[Agent Query: "what did we discuss yesterday?"]
    |
    v
[GetTocRoot RPC] --> returns Year/Month nodes
    |
    v
[Agent picks: this week]
    |
    v
[GetNode RPC] --> returns Week node with Day children
    |
    v
[Agent picks: yesterday]
    |
    v
[GetNode RPC] --> returns Day node with segments + summary
    |
    v
[Agent reads summary, done OR drills into segment]
    |
    v
[GetEvents RPC] --> returns raw events (last resort)
```

**Progressive Disclosure:**
1. Agent starts with high-level summaries (year/month)
2. Navigates down based on time or keywords in summaries
3. Only fetches raw events when necessary
4. Grips provide excerpts without full event retrieval

### Teleport Path (Phase 2+)

```
[TeleportQuery: "vector database discussion"]
    |
    v
[Query BM25 index] --> returns node_ids/grip_ids
    |
    v
[Query Vector index] --> returns node_ids/grip_ids
    |
    v
[Fuse results (RRF or weighted)]
    |
    v
[Return TOC node entry points, NOT content]
    |
    v
[Agent navigates from entry point via normal TOC ops]
```

**Teleport Properties:**
- Returns pointers, not content (TOC node IDs or grip IDs)
- Agent still uses TOC navigation for context
- Indexes are disposable (rebuilt from outbox/events)

### Outbox Relay Path (Background)

```
[Outbox Relay Loop]
    |
    v
[Read batch from outbox CF]
    |
    +---> [For each entry: update Tantivy index]
    |
    +---> [For each entry: update Vector index]
    |
    v
[Delete processed entries from outbox CF]
    |
    v
[Sleep, repeat]
```

**Outbox Entry Types:**
- `IndexEvent { event_id, content, metadata }`
- `IndexTocNode { node_id, summary, keywords }`
- `IndexGrip { grip_id, excerpt }`

---

## Patterns to Follow

### Pattern 1: Column Family Isolation

**What:** Use RocksDB column families to separate workloads with different access patterns.

**When:** Always. This is a core architectural decision.

**Rationale:**
- Events: append-only, sequential writes, range reads
- TOC nodes: versioned updates, point reads
- Outbox: FIFO queue, deletes after processing
- Grips: point reads by ID
- Checkpoints: infrequent updates, crash recovery

**Configuration:**
```rust
// Each CF can have different:
// - Block cache allocation
// - Compaction strategy (FIFO for outbox, leveled for events)
// - Compression (events highly compressible)
```

### Pattern 2: Append-Only with Versioned TOC

**What:** Events are immutable. TOC nodes are versioned (not mutable in place).

**When:** All writes.

**Rationale:**
- No delete complexity
- Crash recovery is simpler (replay from checkpoint)
- Old TOC versions support debugging and rollback

**Implementation:**
```rust
// Event key includes timestamp + ULID (globally unique, sortable)
let event_key = format!("evt:{}:{}", ts_ms, ulid);

// TOC node key includes version for immutability
let toc_key = format!("toc:{}:{}", node_id, version);
let latest_key = format!("latest:{}", node_id);
```

### Pattern 3: Transactional Outbox for Index Updates

**What:** Write index entries to outbox table atomically with source data. Background relay processes outbox.

**When:** All writes that need index updates (events, TOC nodes, grips).

**Rationale:**
- Solves dual-write problem (RocksDB + external index)
- Indexes can be rebuilt from outbox replay
- Crash-safe: if outbox entry exists, index will eventually update

**Implementation:**
```rust
// Atomic write using WriteBatch
let mut batch = WriteBatch::new();
batch.put_cf(&events_cf, event_key, event_bytes);
batch.put_cf(&outbox_cf, outbox_key, outbox_entry);
db.write(batch)?;
```

### Pattern 4: Checkpoint-Based Crash Recovery

**What:** Periodically save progress markers for background jobs. Resume from checkpoint after crash.

**When:** All background jobs (TOC building, rollup, outbox relay).

**Rationale:**
- Avoid reprocessing entire history on restart
- Checkpoints are cheap (small writes)
- RocksDB guarantees durability after fsync

**Implementation:**
```rust
// Checkpoint structure
struct Checkpoint {
    job_type: String,           // "toc_build", "rollup_day", "outbox_relay"
    last_processed_key: Vec<u8>, // Resume point
    processed_count: u64,
    timestamp: i64,
}

// Save checkpoint after batch completion
db.put_cf(&checkpoints_cf,
          format!("ckpt:{}", job_type),
          checkpoint.encode())?;
```

### Pattern 5: Segment Overlap for Context

**What:** Segments overlap by a small window (e.g., 5 minutes or 500 tokens) to preserve context across boundaries.

**When:** Segmentation during TOC building.

**Rationale:**
- Prevents losing context that spans segment boundaries
- Enables better summarization
- Grips can reference events in overlap zone

**Implementation:**
```
Segment 1: [00:00 -------- 30:00] + [30:00 -- 35:00] overlap
Segment 2:                   [25:00 -- 30:00] overlap + [30:00 -------- 60:00]
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Search-First Architecture

**What:** Building the system around full-text/vector search as the primary query mechanism.

**Why Bad:**
- Indexes can fail, corrupt, or become stale
- No graceful degradation path
- Agentic navigation is more efficient than brute-force search

**Instead:** TOC-first architecture. Indexes are accelerators that return entry points into the TOC, never content directly.

### Anti-Pattern 2: Mutable Event Storage

**What:** Allowing updates or deletes to stored events.

**Why Bad:**
- Complicates crash recovery (need to track deletions)
- Breaks TOC integrity (summaries reference deleted events)
- Prevents deterministic replay for debugging

**Instead:** Append-only. If correction needed, append a correction event.

### Anti-Pattern 3: Synchronous Index Updates

**What:** Updating indexes in the same transaction as primary storage.

**Why Bad:**
- Slows down ingestion hot path
- Creates coupling between storage and indexes
- Index failures can fail ingestion

**Instead:** Outbox pattern. Write to outbox, async relay updates indexes.

### Anti-Pattern 4: Flat Key Namespace

**What:** Using a single column family with prefixed keys for all data types.

**Why Bad:**
- Cannot tune compaction per workload
- Range scans include irrelevant data
- Memory allocation not optimized

**Instead:** Column families per logical data type.

### Anti-Pattern 5: Eager Full Rollup

**What:** Rolling up entire history on every change.

**Why Bad:**
- O(n) on every ingestion
- Blocks ingestion path
- Unnecessary for recent data

**Instead:** Incremental rollup with checkpoints. Only rollup completed time periods.

---

## Scalability Considerations

| Concern | At 1K events/day | At 100K events/day | At 1M events/day |
|---------|------------------|--------------------|--------------------|
| Storage | Single RocksDB, local disk | Single RocksDB, SSD | Consider sharding by time |
| TOC Building | Inline after session end | Background job, 5min batches | Dedicated builder process |
| Index Updates | Near-realtime relay | Batch every 30 seconds | Parallel relay workers |
| Query Latency | <10ms for TOC nav | <50ms for TOC nav | Consider caching hot nodes |
| Memory | 256MB block cache | 1GB block cache | 4GB+ block cache |

---

## Suggested Build Order

Based on the architecture and dependencies, here is the recommended build order:

### Phase 0: Foundation (All MVP Dependencies)

```
[1. Storage Layer]
      |
      +---> [2. Domain Types (Event, TocNode, Grip)]
      |
      +---> [3. Service Layer (gRPC scaffolding)]
      |
      v
[4. IngestEvent RPC] <--- [5. Hook Handler Client]
      |
      v
[6. Basic TOC Building (segment creation)]
      |
      v
[7. Query RPCs (GetTocRoot, GetNode, GetEvents)]
      |
      v
[MVP Complete: End-to-end navigation working]
```

### Phase 1: Quality & Trust

```
[8. Grip Creation & Storage]
      |
      v
[9. Summary-to-Grip Linking]
      |
      v
[10. Better Segmentation (token-aware, topic boundaries)]
```

### Phase 2: Teleports

```
[11. Outbox Infrastructure]
      |
      +---> [12. BM25 Index (Tantivy)]
      |
      +---> [13. Vector Index (HNSW)]
      |
      v
[14. TeleportQuery RPC with Fusion]
```

### Phase 3: Resilience

```
[15. Parallel Scan Infrastructure]
      |
      v
[16. Range-Limited Scan by TOC Bounds]
      |
      v
[17. Fallback Path Integration]
```

### Dependencies Diagram

```
Storage Layer (1)
     |
     +---> Domain Types (2) ---> Service Layer (3)
     |                                  |
     |                                  v
     +---> IngestEvent (4) <------+
     |          |                 |
     |          v                 |
     +---> TOC Building (6) ------+
     |          |                 |
     |          v                 |
     +---> Query RPCs (7) --------+
                                  |
                                  v
                          [MVP Complete]
```

---

## Module Structure for Rust Workspace

```
agent-memory/
├── Cargo.toml                    # Workspace root
├── proto/
│   └── memory.proto              # gRPC service definitions
├── crates/
│   ├── memory-types/             # Shared types (Event, TocNode, Grip, etc.)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── event.rs
│   │       ├── toc.rs
│   │       ├── grip.rs
│   │       └── config.rs
│   │
│   ├── memory-storage/           # RocksDB abstraction layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs             # RocksDB wrapper
│   │       ├── keys.rs           # Key encoding/decoding
│   │       ├── column_families.rs
│   │       └── checkpoint.rs
│   │
│   ├── memory-domain/            # Business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ingest.rs         # Event ingestion logic
│   │       ├── toc_builder.rs    # TOC construction
│   │       ├── rollup.rs         # Time hierarchy rollup
│   │       ├── segmenter.rs      # Segment boundary detection
│   │       ├── summarizer.rs     # Pluggable summarizer trait
│   │       └── query.rs          # Query execution
│   │
│   ├── memory-index/             # Optional teleport indexes
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── bm25.rs           # Tantivy integration
│   │       ├── vector.rs         # HNSW integration
│   │       ├── outbox.rs         # Outbox relay
│   │       └── fusion.rs         # Score fusion
│   │
│   ├── memory-service/           # gRPC service implementation
│   │   ├── Cargo.toml
│   │   ├── build.rs              # tonic-build for proto
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       └── handlers/
│   │           ├── mod.rs
│   │           ├── ingest.rs
│   │           ├── toc.rs
│   │           └── teleport.rs
│   │
│   └── memory-daemon/            # Binary: the daemon
│       ├── Cargo.toml
│       └── src/
│           └── main.rs           # CLI, config loading, startup
│
├── hook-handler/                 # Hook handler client (separate binary)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── client.rs             # gRPC client
│       └── hooks/
│           ├── mod.rs
│           ├── claude.rs
│           └── opencode.rs
│
└── tests/
    ├── integration/              # Integration tests
    └── fixtures/                 # Test data
```

### Workspace Cargo.toml

```toml
[workspace]
resolver = "3"
members = [
    "crates/memory-types",
    "crates/memory-storage",
    "crates/memory-domain",
    "crates/memory-index",
    "crates/memory-service",
    "crates/memory-daemon",
    "hook-handler",
]

[workspace.dependencies]
# Core
tokio = { version = "1.43", features = ["full"] }
tonic = "0.12"
prost = "0.13"

# Storage
rocksdb = "0.23"

# Indexing (Phase 2)
tantivy = "0.22"
hnsw_rs = "0.3"  # Or usearch

# Serialization
serde = { version = "1.0", features = ["derive"] }
rmp-serde = "1.3"  # MessagePack

# Utilities
ulid = "1.1"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
thiserror = "2.0"
anyhow = "1.0"
```

### Crate Dependency Graph

```
memory-types        (leaf: no internal deps)
     |
     v
memory-storage      (depends on: memory-types)
     |
     v
memory-domain       (depends on: memory-types, memory-storage)
     |
     +---> memory-index   (depends on: memory-types, memory-storage)
     |
     v
memory-service      (depends on: memory-types, memory-domain, memory-index)
     |
     v
memory-daemon       (depends on: memory-service)

hook-handler        (depends on: memory-types, generated gRPC client)
```

---

## Key Design Alignment with PROJECT.md

| PROJECT.md Decision | Architecture Alignment |
|---------------------|------------------------|
| TOC as primary navigation | TOC-first query path; teleports return entry points only |
| Append-only storage | Events immutable; TOC nodes versioned |
| Hooks for ingestion | Hook handlers are separate processes, gRPC clients |
| Per-project stores first | Single RocksDB instance per project directory |
| Time-only TOC for MVP | Year/Month/Week/Day/Segment hierarchy |
| gRPC only | tonic server, no HTTP layer |
| Pluggable summarizer | Summarizer trait in memory-domain crate |
| RocksDB column families | events, toc_nodes, toc_latest, grips, outbox, checkpoints |

---

## Sources

### HIGH Confidence (Official Documentation)
- [RocksDB Column Families Wiki](https://github.com/facebook/rocksdb/wiki/column-families)
- [RocksDB Checkpoints Wiki](https://github.com/facebook/rocksdb/wiki/Checkpoints)
- [Tonic gRPC Documentation](https://docs.rs/tonic)
- [Cargo Workspaces - Rust Book](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)

### MEDIUM Confidence (Verified Patterns)
- [Transactional Outbox Pattern - microservices.io](https://microservices.io/patterns/data/transactional-outbox.html)
- [TimescaleDB Hierarchical Continuous Aggregates](https://www.tigerdata.com/docs/use-timescale/latest/continuous-aggregates/hierarchical-continuous-aggregates)
- [Design Patterns for Long-Term Memory in LLM Architectures](https://serokell.io/blog/design-patterns-for-long-term-memory-in-llm-powered-architectures)

### LOW Confidence (Research Papers, Community)
- [TiMem: Temporal-Hierarchical Memory Consolidation](https://arxiv.org/html/2601.02845v1) - January 2026
- [MAGMA: Multi-Graph Agentic Memory Architecture](https://arxiv.org/html/2601.03236v1) - January 2026
- [Hybrid RAG Patterns - BM25 + Vectors](https://medium.com/@Nexumo_/7-hybrid-search-recipes-to-blend-bm25-vectors-without-lag-95ed7481751a)

---

*Generated by GSD Project Researcher, 2026-01-29*
