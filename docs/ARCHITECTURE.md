# Architecture

## Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Hook Handler                             │
│     (captures conversation events from Claude Code hooks)        │
└───────────────────────────┬─────────────────────────────────────┘
                            │ IngestEvent RPC
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                        memory-daemon                             │
│  ┌───────────┐ ┌───────────┐ ┌────────────┐ ┌─────────────────┐ │
│  │   gRPC    │ │    TOC    │ │ Summarizer │ │    Scheduler    │ │
│  │   Server  │ │  Builder  │ │ (LLM API)  │ │ (Background Jobs)│ │
│  └─────┬─────┘ └─────┬─────┘ └──────┬─────┘ └────────┬────────┘ │
│        │             │              │                │           │
│        └─────────────┴──────────────┴────────────────┘           │
│                               │                                   │
│  ┌────────────────────────────┼────────────────────────────────┐ │
│  │                     Search Layer                             │ │
│  │  ┌─────────────────────┐  │  ┌────────────────────────────┐ │ │
│  │  │   BM25 Full-Text    │  │  │    Vector HNSW Index       │ │ │
│  │  │     (Tantivy)       │  │  │      (usearch)             │ │ │
│  │  └─────────────────────┘  │  └────────────────────────────┘ │ │
│  └───────────────────────────┼─────────────────────────────────┘ │
│                              │                                    │
│  ┌───────────────────────────┼─────────────────────────────────┐ │
│  │                    Topic Graph Layer                         │ │
│  │  ┌──────────────┐ ┌───────────────┐ ┌─────────────────────┐ │ │
│  │  │   HDBSCAN    │ │  LLM Labels   │ │ Topic Relationships │ │ │
│  │  │  Clustering  │ │  & Importance │ │    & Scoring        │ │ │
│  │  └──────────────┘ └───────────────┘ └─────────────────────┘ │ │
│  └───────────────────────────┼─────────────────────────────────┘ │
│                              ▼                                    │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │                      memory-storage                           ││
│  │                        (RocksDB)                              ││
│  │  ┌───────┐ ┌─────┐ ┌───────┐ ┌───────────┐ ┌───────────────┐ ││
│  │  │Events │ │ TOC │ │ Grips │ │Checkpoints│ │ Topics/Vectors│ ││
│  │  └───────┘ └─────┘ └───────┘ └───────────┘ └───────────────┘ ││
│  └──────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Crate Structure

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `memory-types` | Domain models (Event, TocNode, Grip, Settings) | None (leaf crate) |
| `memory-storage` | RocksDB persistence layer | memory-types |
| `memory-toc` | Segmentation, summarization, TOC building | memory-types, memory-storage |
| `memory-search` | Tantivy BM25 full-text search (indexes TOC nodes and grips) | memory-types |
| `memory-embeddings` | Candle ML model loading with all-MiniLM-L6-v2 (384-dim embeddings) | memory-types |
| `memory-vector` | usearch HNSW vector index wrapper | memory-types, memory-embeddings |
| `memory-indexing` | Outbox consumer pipeline with checkpoint-based crash recovery | memory-types, memory-storage, memory-search, memory-vector |
| `memory-topics` | HDBSCAN topic clustering, LLM labeling, importance scoring, relationships | memory-types, memory-storage, memory-embeddings |
| `memory-scheduler` | tokio-cron-scheduler wrapper for background jobs (rollup, compaction, indexing) | memory-types, memory-storage, memory-toc, memory-indexing |
| `memory-service` | gRPC service implementation | memory-types, memory-storage, memory-toc, memory-search, memory-vector, memory-topics, memory-scheduler |
| `memory-client` | Client library for hook handlers | memory-types, memory-service |
| `memory-daemon` | CLI binary | All crates |

### Dependency Graph

```
                              memory-types
                                   ↑
       ┌───────────┬───────────────┼───────────────┬───────────────┐
       │           │               │               │               │
memory-storage  memory-toc   memory-search   memory-embeddings     │
       ↑           ↑               ↑               ↑               │
       │           │               │               │               │
       │           │               │         memory-vector         │
       │           │               │               ↑               │
       │           │               │               │               │
       │           │         memory-indexing───────┘               │
       │           │               ↑                               │
       │           │               │                               │
       │           │         memory-topics─────────────────────────┘
       │           │               ↑
       │           │               │
       ├───────────┴───────────────┤
       │                           │
  memory-scheduler                 │
       │                           │
       └───────────────────────────┤
                                   │
                             memory-service
                                   ↑
                                   │
                             memory-client
                                   │
                                   ▼
                             memory-daemon
```

### Crate Responsibilities

**Core Layer:**
- `memory-types`: Shared domain models, traits, and error types
- `memory-storage`: RocksDB column families, atomic writes, range scans

**TOC Layer:**
- `memory-toc`: Segmentation rules, LLM summarization, hierarchical TOC building

**Search Layer:**
- `memory-search`: Tantivy index management, BM25 ranking, teleport search
- `memory-embeddings`: all-MiniLM-L6-v2 model loading via Candle, embedding generation
- `memory-vector`: usearch HNSW index, approximate nearest neighbor queries

**Pipeline Layer:**
- `memory-indexing`: Outbox consumer, incremental indexing, checkpoint recovery
- `memory-topics`: HDBSCAN clustering, topic labeling, importance scoring, topic relationships
- `memory-scheduler`: Cron-based job scheduling (rollup, compaction, index maintenance)

**API Layer:**
- `memory-service`: gRPC handlers, request validation, response mapping
- `memory-client`: Hook handler integration, event mapping
- `memory-daemon`: CLI, configuration loading, graceful shutdown

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

### Search Pipeline

```
1. TeleportSearch or VectorTeleport called with query
   │
2. Query routed to appropriate index:
   ├── BM25: Tantivy tokenizes and ranks by term frequency
   └── Vector: Candle generates embedding, usearch finds nearest neighbors
   │
3. Results merged and deduplicated (for HybridSearch)
   │
4. Top-K results returned with scores and node references
   │
5. Client can ExpandGrip for full context
```

### Topic Graph Construction

```
1. Outbox indexing job collects new embeddings
   │
2. HDBSCAN clusters embeddings into topic groups
   │
3. LLM generates labels for each cluster
   │
4. Importance scoring based on:
   ├── Cluster size (more members = higher importance)
   ├── Recency (recent content boosts importance)
   └── Centrality (well-connected topics score higher)
   │
5. Topic relationships computed:
   ├── Co-occurrence in TOC nodes
   └── Embedding similarity between cluster centroids
   │
6. Topics and relationships persisted to RocksDB
```

### Indexing Pipeline

```
1. Outbox consumer reads pending entries
   │
2. For each entry:
   ├── Extract text from TOC node summaries and grips
   ├── Index text in Tantivy (BM25)
   ├── Generate embedding via Candle
   └── Add embedding to usearch HNSW index
   │
3. Update checkpoint for crash recovery
   │
4. Periodic compaction and optimization
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
| `bm25_index` | `bm25:meta` | Index metadata | Pointer to Tantivy directory path |
| `vector_index` | `vec:meta` | Index metadata | Pointer to usearch directory path |
| `topics` | `topic:{topic_id}` | Topic JSON | Topic clusters with labels and importance |
| `topic_links` | `tlink:{topic_id}:{node_id}` | Link JSON | Topic-to-TOC node associations |
| `topic_rels` | `trel:{topic_id_a}:{topic_id_b}` | Relationship JSON | Inter-topic relationships and strength |

### External Index Directories

Some indexes are managed outside RocksDB for specialized libraries:

| Index | Location | Library | Purpose |
|-------|----------|---------|---------|
| Tantivy BM25 | `{db_path}/tantivy/` | Tantivy | Full-text search with BM25 ranking |
| usearch HNSW | `{db_path}/usearch/` | usearch | Approximate nearest neighbor vectors |

RocksDB column families store metadata and pointers to these external directories. The indexing pipeline coordinates writes to ensure consistency between RocksDB and external indexes.

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

## Background Jobs

The scheduler manages periodic maintenance tasks via tokio-cron-scheduler.

### Job Types

| Job | Schedule | Purpose |
|-----|----------|---------|
| `outbox_processor` | Every 30s | Process pending outbox entries for TOC updates |
| `index_sync` | Every 5m | Sync new content to BM25 and vector indexes |
| `topic_refresh` | Every 1h | Re-cluster embeddings and update topic graph |
| `rollup` | Daily 3am | Roll up day nodes into week summaries |
| `compaction` | Weekly Sun 4am | Optimize RocksDB and Tantivy indexes |

### Job Lifecycle

```
1. Scheduler starts on daemon init
   │
2. Jobs registered with cron expressions
   │
3. Each job execution:
   ├── Load checkpoint from CF_CHECKPOINTS
   ├── Process work from checkpoint position
   ├── Write progress checkpoints periodically
   └── Write final checkpoint on completion
   │
4. On shutdown: graceful stop with checkpoint flush
```

### Job Control API

- `ListJobs`: Returns all registered jobs with status and next run time
- `GetJob`: Returns detailed status for a single job
- `PauseJob`: Stops scheduled execution (in-flight work completes)
- `ResumeJob`: Resumes scheduled execution from checkpoint

## gRPC Service

### Endpoints

#### Core TOC Operations

| RPC | Request | Response | Purpose |
|-----|---------|----------|---------|
| `IngestEvent` | `IngestEventRequest` | `IngestEventResponse` | Store conversation event |
| `GetTocRoot` | `GetTocRootRequest` | `GetTocRootResponse` | Get year-level nodes |
| `GetNode` | `GetNodeRequest` | `GetNodeResponse` | Get specific node |
| `BrowseToc` | `BrowseTocRequest` | `BrowseTocResponse` | Paginated children |
| `GetEvents` | `GetEventsRequest` | `GetEventsResponse` | Events in time range |
| `ExpandGrip` | `ExpandGripRequest` | `ExpandGripResponse` | Context around grip |
| `SearchNode` | `SearchNodeRequest` | `SearchNodeResponse` | Search within a TOC node |
| `SearchChildren` | `SearchChildrenRequest` | `SearchChildrenResponse` | Search node's children |

#### BM25 Full-Text Search

| RPC | Request | Response | Purpose |
|-----|---------|----------|---------|
| `TeleportSearch` | `TeleportSearchRequest` | `TeleportSearchResponse` | BM25 search across all content |
| `GetTeleportStatus` | `GetTeleportStatusRequest` | `GetTeleportStatusResponse` | BM25 index health and stats |

#### Vector Search

| RPC | Request | Response | Purpose |
|-----|---------|----------|---------|
| `VectorTeleport` | `VectorTeleportRequest` | `VectorTeleportResponse` | Semantic similarity search |
| `HybridSearch` | `HybridSearchRequest` | `HybridSearchResponse` | Combined BM25 + vector search |
| `GetVectorIndexStatus` | `GetVectorIndexStatusRequest` | `GetVectorIndexStatusResponse` | Vector index health and stats |

#### Topic Graph

| RPC | Request | Response | Purpose |
|-----|---------|----------|---------|
| `GetTopicGraphStatus` | `GetTopicGraphStatusRequest` | `GetTopicGraphStatusResponse` | Topic graph health and stats |
| `GetTopicsByQuery` | `GetTopicsByQueryRequest` | `GetTopicsByQueryResponse` | Find topics matching query |
| `GetRelatedTopics` | `GetRelatedTopicsRequest` | `GetRelatedTopicsResponse` | Get related topics by ID |
| `GetTopTopics` | `GetTopTopicsRequest` | `GetTopTopicsResponse` | Get most important topics |

#### Scheduler Management

| RPC | Request | Response | Purpose |
|-----|---------|----------|---------|
| `ListJobs` | `ListJobsRequest` | `ListJobsResponse` | List all scheduled jobs |
| `GetJob` | `GetJobRequest` | `GetJobResponse` | Get job status and schedule |
| `PauseJob` | `PauseJobRequest` | `PauseJobResponse` | Pause a scheduled job |
| `ResumeJob` | `ResumeJobRequest` | `ResumeJobResponse` | Resume a paused job |

### Health Check

tonic-health endpoint marks MemoryService as SERVING when ready.

### Reflection

tonic-reflection enables grpcurl and other tools to discover the API.
