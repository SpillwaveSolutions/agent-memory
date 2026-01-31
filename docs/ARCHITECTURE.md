# Architecture

## Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Hook Handler                            │
│  (captures conversation events from Claude Code hooks)       │
└────────────────────────┬────────────────────────────────────┘
                         │ IngestEvent RPC
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     memory-daemon                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   gRPC      │  │    TOC      │  │    Summarizer       │  │
│  │   Server    │  │   Builder   │  │    (LLM API)        │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │              │
│         └────────────────┼─────────────────────┘              │
│                          ▼                                    │
│  ┌───────────────────────────────────────────────────────┐   │
│  │                    memory-storage                      │   │
│  │                      (RocksDB)                         │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐  │   │
│  │  │ Events  │ │   TOC   │ │  Grips  │ │ Checkpoints │  │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────────┘  │   │
│  └───────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Crate Structure

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `memory-types` | Domain models (Event, TocNode, Grip, Settings) | None (leaf crate) |
| `memory-storage` | RocksDB persistence layer | memory-types |
| `memory-toc` | Segmentation, summarization, TOC building | memory-types, memory-storage |
| `memory-service` | gRPC service implementation | memory-types, memory-storage |
| `memory-client` | Client library for hook handlers | memory-types, memory-service |
| `memory-daemon` | CLI binary | All crates |

### Dependency Graph

```
memory-types
     ↑
     ├─────────────────────┬───────────────────┐
     │                     │                   │
memory-storage        memory-toc          memory-service
     ↑                     ↑                   ↑
     ├─────────────────────┤                   │
     │                     │                   │
     │                memory-client────────────┘
     │                     │
     └─────────────────────┴───────────────────┐
                                               │
                                         memory-daemon
```

## Data Flow

### Event Ingestion

```
1. Hook handler captures event (user message, assistant response, etc.)
   │
2. memory-client maps HookEvent to domain Event
   │
3. IngestEvent RPC sends to daemon
   │
4. Daemon validates and serializes event
   │
5. Storage writes atomically:
   ├── Event to CF_EVENTS
   └── OutboxEntry to CF_OUTBOX
   │
6. Background job processes outbox for TOC updates
```

### TOC Construction

```
1. Outbox processor reads pending events
   │
2. Segmenter groups events by time/token boundaries
   ├── Time gap: 30 min default
   ├── Token threshold: 4000 tokens default
   └── Overlap: 5 min or 500 tokens for context
   │
3. Summarizer generates summaries with grips
   ├── Calls LLM API (OpenAI/Anthropic)
   └── Extracts grips from excerpt-bullet matches
   │
4. TocBuilder creates/updates node hierarchy
   ├── Year → Month → Week → Day → Segment
   └── Versioned updates (TOC-06)
   │
5. Checkpoint written for crash recovery
```

### Query Resolution

```
1. Client calls GetTocRoot for year nodes
   │
2. Agent reads summaries, decides to drill down
   │
3. GetNode or BrowseToc retrieves children
   │
4. Agent finds relevant time period
   │
5. GetEvents retrieves raw events
   │
6. ExpandGrip provides source evidence for verification
```

## Storage Schema

### Column Families

| CF | Key Format | Value | Purpose |
|----|------------|-------|---------|
| `events` | `evt:{ts_ms:013}:{ulid}` | Event JSON | Raw conversation events |
| `toc_nodes` | `toc:{node_id}:v{version}` | TocNode JSON | Versioned TOC nodes |
| `toc_latest` | `latest:{node_id}` | Version (u32) | Latest version pointer |
| `grips` | `grip:{ts_ms:013}:{ulid}` | Grip JSON | Excerpt provenance |
| `outbox` | `out:{sequence:016}` | OutboxEntry JSON | Pending TOC updates |
| `checkpoints` | `chk:{job_name}` | Checkpoint bytes | Job crash recovery |

### Key Design

Time-prefixed keys enable efficient range scans:

```
All events for January 2026:
  Start: evt:1704067200000:00000000000000000000000000
  End:   evt:1706745599999:ZZZZZZZZZZZZZZZZZZZZZZZZZZ

Single event lookup:
  evt:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
      │             │
      │             └── ULID (26 chars, alphanumeric)
      └── Timestamp in ms (13 digits, zero-padded)
```

ULID suffix ensures:
- Uniqueness within millisecond
- Embedded timestamp for reconstruction
- Lexicographic ordering

## Crash Recovery

1. **Atomic batch writes** ensure event + outbox consistency
2. **Outbox entries** survive crash and are reprocessed on restart
3. **Checkpoints** store job progress (processed event ID, timestamp)
4. **Idempotent operations** handle duplicate processing safely
5. **RocksDB durability** guarantees written data persists

### Recovery Flow

```
On Startup:
1. Open RocksDB (recovers WAL)
   │
2. Load checkpoint for each background job
   │
3. Resume from checkpoint position
   │
4. Process outbox entries (idempotent)
```

## Configuration

### Loading Order (highest priority first)

1. Command-line flags (`--port`, `--db-path`)
2. Environment variables (`MEMORY_PORT`, `MEMORY_DB_PATH`)
3. Config file (~/.config/memory-daemon/config.toml)
4. Defaults

### Key Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `port` | 50051 | gRPC server port |
| `db_path` | ~/.memory-store | RocksDB data directory |
| `log_level` | info | Logging verbosity |
| `segment_time_gap_minutes` | 30 | Time gap for segmentation |
| `segment_token_threshold` | 4000 | Token threshold for segmentation |
| `overlap_minutes` | 5 | Context overlap in segments |
| `overlap_tokens` | 500 | Token overlap in segments |

## gRPC Service

### Endpoints

| RPC | Request | Response | Purpose |
|-----|---------|----------|---------|
| `IngestEvent` | `IngestEventRequest` | `IngestEventResponse` | Store conversation event |
| `GetTocRoot` | `GetTocRootRequest` | `GetTocRootResponse` | Get year-level nodes |
| `GetNode` | `GetNodeRequest` | `GetNodeResponse` | Get specific node |
| `BrowseToc` | `BrowseTocRequest` | `BrowseTocResponse` | Paginated children |
| `GetEvents` | `GetEventsRequest` | `GetEventsResponse` | Events in time range |
| `ExpandGrip` | `ExpandGripRequest` | `ExpandGripResponse` | Context around grip |

### Health Check

tonic-health endpoint marks MemoryService as SERVING when ready.

### Reflection

tonic-reflection enables grpcurl and other tools to discover the API.
