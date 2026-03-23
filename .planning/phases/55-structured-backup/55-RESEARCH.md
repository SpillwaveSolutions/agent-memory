# Phase 55: Structured Backup - Research

**Researched:** 2026-03-23
**Domain:** gRPC server-side streaming, JSONL backup, tonic streaming infrastructure
**Confidence:** HIGH

## Summary

Phase 55 introduces the project's first server-side streaming gRPC RPC (`ExportBackup`) and a `memory backup` CLI command that produces a structured JSONL directory. The research confirms that tonic 0.12 (already in use) fully supports server-side streaming via `tokio-stream` and either `tokio::mpsc` channels or the `async-stream` crate. Two new workspace dependencies are needed: `tokio-stream` and (optionally) `async-stream`.

All four domain types (Event, TocNode, Grip, Episode) already derive `Serialize + Deserialize`, so JSONL serialization is straightforward via `serde_json::to_string`. Storage iteration methods exist for events (`get_events_in_range`) and TOC nodes (`get_toc_nodes_by_level`), episodes (`list_episodes`), but a new `list_all_grips` method is needed since grips are stored with mixed key prefixes in CF_GRIPS (both `grip_id` keys and `node:*` index keys).

**Primary recommendation:** Use `tokio::mpsc` channel + `ReceiverStream` pattern (not `async-stream`) to minimize new dependencies. Add `tokio-stream` to workspace. Implement streaming handler in a new `backup.rs` module following the existing handler delegation pattern from `ingest.rs`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- New `backup` subcommand added to existing `memory-cli` crate
- `ExportBackup` is a **server-side streaming RPC** -- first streaming RPC in the project
- Tonic streaming support must be wired into the server framework (new infrastructure)
- CLI writes files locally from the streamed data
- Directory structure: `memory-backup/` with `manifest.json`, `events/YYYY-MM-DD.jsonl`, `toc/*.jsonl`, `grips.jsonl`, `episodes.jsonl`
- JSONL format: one JSON object per line for all data types
- CLI flags: `memory backup`, `--events-only`, `--since`, `--until`, `--dir`
- Incremental: `--since` filters events by timestamp; per-day event files overwritten (not appended); TOC/grips/episodes fully rewritten
- Streaming RPC: `ExportBackup(BackupOptions)` returns `stream BackupChunk`
- Backed up: Events, TOC nodes (5 levels), Grips, Episodes
- NOT backed up: BM25/HNSW indexes, InFlightBuffer, Topic graph, Scheduler checkpoints

### Claude's Discretion
- Exact proto message definition for `BackupChunk`
- Whether to use a single streaming RPC or separate RPCs per data type
- Chunk size for streaming (e.g., 100 records per chunk)
- Error handling for partial backup (e.g., stream interrupted mid-export)

### Deferred Ideas (OUT OF SCOPE)
- Config.toml `[backup]` section for default directory
- Compressed backup (gzip the JSONL files)
- Parallel streaming (multiple RPC calls for different data types)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BACKUP-01 | `memory backup` exports all layers as JSONL directory with `manifest.json` | CLI pattern from daily.rs; serde_json serialization on domain types |
| BACKUP-02 | `memory backup --events-only` exports just base event layer | BackupOptions proto message with `events_only` bool field |
| BACKUP-03 | `memory backup --since 24h` exports only recent data | Reuse `parse_range_to_days` from daily.rs; BackupOptions with timestamp fields |
| BACKUP-04 | Incremental backups overwrite per-day event files | CLI file writing pattern: `std::fs::write` per day file |
| BACKUP-05 | `manifest.json` includes version, counts, time range, incremental flag | Serde serialization of a ManifestInfo struct |
| BACKUP-06 | Backup includes events, TOC nodes (all levels), grips, episodes | Storage methods: get_events_in_range, get_toc_nodes_by_level, new list_all_grips, list_episodes |
| BACKUP-07 | `ExportBackup` uses server-side gRPC streaming | tonic 0.12 streaming with tokio-stream ReceiverStream pattern |
| GRPC-02 | `ExportBackup` server-side streaming RPC delivers JSONL chunks | Proto `stream BackupChunk` return type; BackupChunk with type tag + payload |
| GRPC-04 | Streaming support wired into tonic server framework | type ExportBackupStream = ReceiverStream<...>; MemoryService trait impl |
</phase_requirements>

## Standard Stack

### Core (already in workspace)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tonic | 0.12 | gRPC framework (already used) | Project standard |
| prost | 0.13 | Protobuf codegen (already used) | Project standard |
| serde_json | 1.0 | JSONL serialization (already used) | All domain types derive Serialize |
| chrono | 0.4 | Date parsing for --since/--until (already used) | Project standard |
| clap | 4.5 | CLI arg parsing (already used) | Project standard |

### New Dependencies Required
| Library | Version | Purpose | Why Needed |
|---------|---------|---------|------------|
| tokio-stream | 0.1 | `ReceiverStream` wrapper for tonic streaming | Required by tonic for server-side streaming; wraps `tokio::mpsc::Receiver` into a `Stream` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio-stream ReceiverStream | async-stream crate | async-stream adds another dependency; ReceiverStream is sufficient and more idiomatic for tonic |
| Single streaming RPC | Separate RPCs per data type | Single RPC is simpler; CLI handles routing chunks to files via type tag |

## Architecture Patterns

### Recommended Project Structure (new files)
```
proto/memory.proto                          # Add ExportBackup RPC + messages
crates/memory-service/src/backup.rs         # NEW: Streaming backup handler
crates/memory-storage/src/db.rs             # ADD: list_all_grips(), list_all_episodes()
crates/memory-client/src/client.rs          # ADD: export_backup() streaming method
crates/memory-cli/src/cli.rs               # ADD: Backup variant + BackupArgs
crates/memory-cli/src/commands/backup.rs    # NEW: Backup command implementation
crates/memory-cli/src/commands/mod.rs       # ADD: pub mod backup;
```

### Pattern 1: Proto Definition for Server-Side Streaming
**What:** Add `ExportBackup` RPC with `stream` return type
**When to use:** This is the first streaming RPC in the project
**Example:**
```protobuf
// In the MemoryService service block:
rpc ExportBackup(BackupOptions) returns (stream BackupChunk);

// Request message
message BackupOptions {
    bool events_only = 1;        // Only export events (BACKUP-02)
    int64 since_ms = 2;          // Start timestamp for incremental (0 = all)
    int64 until_ms = 3;          // End timestamp (0 = now)
}

// Each chunk in the stream
message BackupChunk {
    BackupChunkType chunk_type = 1;
    string jsonl_data = 2;       // One or more JSONL lines
    uint32 record_count = 3;     // Records in this chunk
}

enum BackupChunkType {
    BACKUP_CHUNK_TYPE_UNSPECIFIED = 0;
    BACKUP_CHUNK_TYPE_EVENTS = 1;
    BACKUP_CHUNK_TYPE_TOC_SEGMENTS = 2;
    BACKUP_CHUNK_TYPE_TOC_DAYS = 3;
    BACKUP_CHUNK_TYPE_TOC_WEEKS = 4;
    BACKUP_CHUNK_TYPE_TOC_MONTHS = 5;
    BACKUP_CHUNK_TYPE_TOC_YEARS = 6;
    BACKUP_CHUNK_TYPE_GRIPS = 7;
    BACKUP_CHUNK_TYPE_EPISODES = 8;
    BACKUP_CHUNK_TYPE_MANIFEST = 9;
}
```

### Pattern 2: Tonic Server-Side Streaming Handler
**What:** Handler that produces a stream of BackupChunks via tokio channel
**When to use:** Implementing the `export_backup` method on MemoryServiceImpl
**Example:**
```rust
// In crates/memory-service/src/backup.rs
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use memory_storage::Storage;
use crate::pb::{BackupOptions, BackupChunk, BackupChunkType};

pub type ExportBackupStream = ReceiverStream<Result<BackupChunk, Status>>;

pub async fn export_backup(
    storage: Arc<Storage>,
    request: Request<BackupOptions>,
) -> Result<Response<ExportBackupStream>, Status> {
    let opts = request.into_inner();
    let (tx, rx) = mpsc::channel(64); // buffered channel

    tokio::spawn(async move {
        // Stream events in day-sized chunks
        if let Err(e) = stream_events(&storage, &opts, &tx).await {
            let _ = tx.send(Err(Status::internal(e.to_string()))).await;
            return;
        }
        // Stream TOC nodes (unless events_only)
        if !opts.events_only {
            // stream_toc_level(...) for each level
            // stream_grips(...)
            // stream_episodes(...)
        }
        // Stream manifest last
        // tx.send(Ok(manifest_chunk)).await
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}
```

### Pattern 3: MemoryService Trait Integration
**What:** Wire the streaming RPC into the existing service impl
**When to use:** tonic generates an associated type for the stream
**Example:**
```rust
// tonic codegen will generate in the MemoryService trait:
//   type ExportBackupStream: Stream<Item = Result<BackupChunk, Status>> + Send;
//   async fn export_backup(...) -> Result<Response<Self::ExportBackupStream>, Status>;

// In MemoryServiceImpl (ingest.rs):
use crate::backup;

// Add to MemoryService impl block:
type ExportBackupStream = backup::ExportBackupStream;

async fn export_backup(
    &self,
    request: Request<BackupOptions>,
) -> Result<Response<Self::ExportBackupStream>, Status> {
    backup::export_backup(self.storage.clone(), request).await
}
```

### Pattern 4: Streaming Client Method
**What:** Client consumes the stream and yields chunks
**When to use:** `MemoryClient::export_backup()` method
**Example:**
```rust
// In crates/memory-client/src/client.rs:
use tonic::Streaming;

pub async fn export_backup(
    &mut self,
    events_only: bool,
    since_ms: i64,
    until_ms: i64,
) -> Result<Streaming<BackupChunk>, ClientError> {
    let request = tonic::Request::new(BackupOptions {
        events_only,
        since_ms,
        until_ms,
    });
    let response = self.inner.export_backup(request).await?;
    Ok(response.into_inner()) // Streaming<BackupChunk>
}
```

### Pattern 5: CLI File Writing from Stream
**What:** CLI receives stream chunks and routes to files
**When to use:** `memory backup` command implementation
**Example:**
```rust
// In crates/memory-cli/src/commands/backup.rs:
use tonic::Streaming;

let mut stream = client.export_backup(args.events_only, since_ms, until_ms).await?;

while let Some(chunk) = stream.message().await? {
    match chunk.chunk_type() {
        BackupChunkType::Events => {
            // Parse JSONL to extract date, write to events/YYYY-MM-DD.jsonl
        }
        BackupChunkType::TocSegments => {
            // Append to toc/segments.jsonl
        }
        BackupChunkType::Manifest => {
            // Write manifest.json
        }
        // ... etc
    }
}
```

### Anti-Patterns to Avoid
- **Buffering entire dataset in memory:** The whole point of streaming is to avoid this. Never collect all events into a Vec before sending.
- **Using unary RPC for backup:** Would require loading all data into one response message, defeating the purpose.
- **Appending to JSONL files on incremental:** Leads to duplicate lines. Always overwrite per-day event files.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Stream wrapping | Custom Stream impl | `ReceiverStream` from tokio-stream | Battle-tested, tonic-idiomatic |
| JSONL serialization | Custom line formatter | `serde_json::to_string` + newline | All domain types already derive Serialize |
| Date range parsing | New parser | Reuse `parse_range_to_days` from daily.rs | Already tested in Phase 54 |
| Protobuf streaming | Manual HTTP/2 frames | tonic codegen `stream` keyword | tonic handles all framing |

## Common Pitfalls

### Pitfall 1: Missing `tokio-stream` Dependency
**What goes wrong:** Compilation fails with missing `ReceiverStream` type
**Why it happens:** tokio-stream is NOT included in tokio even with `full` features; it's a separate crate
**How to avoid:** Add `tokio-stream = "0.1"` to workspace Cargo.toml AND to memory-service/Cargo.toml and memory-client/Cargo.toml
**Warning signs:** "cannot find type ReceiverStream" compiler error

### Pitfall 2: Grip Key Prefix Collision in CF_GRIPS
**What goes wrong:** When scanning all grips, you pick up `node:*` index entries alongside actual grip entries
**Why it happens:** CF_GRIPS stores both grip data (keyed by `grip_id`) and index entries (keyed by `node:{node_id}:{grip_id}`)
**How to avoid:** New `list_all_grips` method must filter out keys starting with `node:` -- only return entries where key does NOT start with `node:`. Alternatively, use forward iteration from Start and skip `node:` prefixed keys.
**Warning signs:** Deserialization errors when trying to parse index entries as Grip objects

### Pitfall 3: Event JSONL Must Include All Fields for Round-Trip
**What goes wrong:** Backup JSONL loses data that Phase 56 import cannot restore
**Why it happens:** Serializing proto Event loses precision vs serializing domain Event (proto uses enums as i32)
**How to avoid:** Serialize domain types (memory_types::Event, etc.) directly to JSONL, NOT proto types. Domain types have proper serde derives with readable enum names.
**Warning signs:** Import produces events with wrong types or missing metadata

### Pitfall 4: Events Column Family Uses Custom Key Format
**What goes wrong:** Cannot iterate all events with a simple prefix scan
**Why it happens:** Events use `EventKey` with timestamp-based prefix encoding
**How to avoid:** Use `get_events_in_range(0, i64::MAX)` for full export, or `get_events_in_range(since_ms, until_ms)` for incremental. Both return `(EventKey, Vec<u8>)` tuples that need `Event::from_bytes` deserialization.
**Warning signs:** Empty results when trying raw prefix iteration

### Pitfall 5: TOC Nodes Have Version History
**What goes wrong:** Backing up all TOC_NODES CF entries duplicates historical versions
**Why it happens:** TOC nodes store versioned entries (`toc:{node_id}:v{version}`) plus latest pointers
**How to avoid:** Use `get_toc_nodes_by_level(level, None, None)` for each level -- this resolves to latest versions only via the toc_latest CF
**Warning signs:** Backup contains duplicate nodes with different versions

### Pitfall 6: Stream Channel Backpressure
**What goes wrong:** Memory usage spikes if sender produces faster than client consumes
**Why it happens:** Unbounded channel or too-large buffer
**How to avoid:** Use bounded channel (`mpsc::channel(64)`) -- sender will await when buffer is full
**Warning signs:** High memory usage during backup of large stores

### Pitfall 7: Proto Field Numbers
**What goes wrong:** Proto field number conflicts with existing messages
**Why it happens:** Field numbers >200 reserved for future phases per project convention
**How to avoid:** New messages (BackupOptions, BackupChunk, BackupChunkType) get their own message blocks -- no conflict since they are new top-level messages. For the service block, just add the RPC line.

## Code Examples

### Storage: New list_all_grips Method
```rust
// In crates/memory-storage/src/db.rs
/// List all grips in storage (for backup export).
///
/// Iterates CF_GRIPS from start, skipping `node:` index entries.
pub fn list_all_grips(&self) -> Result<Vec<memory_types::Grip>, StorageError> {
    let grips_cf = self.db.cf_handle(CF_GRIPS)
        .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_GRIPS.to_string()))?;

    let mut grips = Vec::new();
    let iter = self.db.iterator_cf(&grips_cf, IteratorMode::Start);

    for item in iter {
        let (key, value) = item?;
        let key_str = String::from_utf8_lossy(&key);
        // Skip index entries (node:{node_id}:{grip_id})
        if key_str.starts_with("node:") {
            continue;
        }
        let grip = memory_types::Grip::from_bytes(&value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        grips.push(grip);
    }

    Ok(grips)
}
```

### Storage: list_all_episodes Method
```rust
// In crates/memory-storage/src/episodes.rs
/// List ALL episodes (no limit, for backup export).
pub fn list_all_episodes(&self) -> Result<Vec<Episode>, StorageError> {
    let cf = self.db.cf_handle(CF_EPISODES)
        .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EPISODES.to_string()))?;

    let mut episodes = Vec::new();
    let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

    for item in iter {
        let (_, value) = item?;
        let episode: Episode = serde_json::from_slice(&value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        episodes.push(episode);
    }

    Ok(episodes)
}
```

### Manifest JSON Structure
```rust
#[derive(Serialize)]
struct Manifest {
    agent_memory_version: String,
    export_date: String,
    incremental: bool,
    since_ms: Option<i64>,
    until_ms: Option<i64>,
    counts: ManifestCounts,
}

#[derive(Serialize)]
struct ManifestCounts {
    events: u64,
    toc_segments: u64,
    toc_days: u64,
    toc_weeks: u64,
    toc_months: u64,
    toc_years: u64,
    grips: u64,
    episodes: u64,
}
```

### Event JSONL Line (domain type serialization)
```rust
// Deserialize from storage bytes, serialize to JSONL
let event = Event::from_bytes(&bytes)?;
let line = serde_json::to_string(&event)?; // One JSON object, no newline
// Write: format!("{}\n", line) to file
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Unary RPC only | Server-side streaming | Phase 55 (first) | Enables backup without OOM on large stores |
| No backup capability | Structured JSONL backup | Phase 55 | Machine-readable, git-friendly export |

**Project firsts in this phase:**
- First streaming RPC (ExportBackup)
- First use of `tokio-stream` crate
- First `ReceiverStream` pattern in the codebase

## Open Questions

1. **Chunk size for streaming**
   - What we know: Need to batch records into chunks (not one-per-message for efficiency, not all-at-once for memory)
   - What's unclear: Optimal batch size
   - Recommendation: Use 100 records per chunk. This balances gRPC message overhead (small messages = more framing) vs memory (large messages = more buffering). The JSONL text per event is ~200-500 bytes, so 100 records = ~20-50KB per message which is well within gRPC limits.

2. **Event day-partitioning in stream vs CLI**
   - What we know: Events need to be split into per-day JSONL files
   - What's unclear: Should the server group events by day in separate chunks, or should the CLI parse timestamps and route?
   - Recommendation: Server sends events in time-sorted order. CLI parses `timestamp_ms` from each JSONL line to determine which `events/YYYY-MM-DD.jsonl` file to write to. This keeps the server simple (just iterate and send) and the CLI already handles file routing for daily.rs.

3. **Partial backup recovery**
   - What we know: Stream could be interrupted mid-export
   - What's unclear: Should partial backup be treated as valid?
   - Recommendation: Manifest is sent LAST. If manifest.json doesn't exist, the backup is incomplete. CLI can warn "backup appears incomplete -- no manifest.json found" but still write partial data. This is simple and robust.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | Cargo.toml workspace |
| Quick run command | `cargo test -p memory-service --lib backup` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BACKUP-01 | Full backup produces correct directory structure | integration | `cargo test -p memory-cli backup_full` | No - Wave 0 |
| BACKUP-02 | --events-only flag skips TOC/grips/episodes | unit | `cargo test -p memory-service backup::tests::events_only` | No - Wave 0 |
| BACKUP-03 | --since filters events by timestamp | unit | `cargo test -p memory-service backup::tests::incremental` | No - Wave 0 |
| BACKUP-04 | Per-day files overwritten on incremental | integration | `cargo test -p memory-cli backup_incremental_overwrite` | No - Wave 0 |
| BACKUP-05 | manifest.json has required fields | unit | `cargo test -p memory-service backup::tests::manifest` | No - Wave 0 |
| BACKUP-06 | All data types included in backup | unit | `cargo test -p memory-service backup::tests::all_types` | No - Wave 0 |
| BACKUP-07 | ExportBackup uses streaming RPC | unit | `cargo test -p memory-service backup::tests::streaming` | No - Wave 0 |
| GRPC-02 | BackupChunk messages flow correctly | unit | `cargo test -p memory-service backup::tests::chunk_types` | No - Wave 0 |
| GRPC-04 | Streaming wired into service framework | unit | `cargo test -p memory-service ingest::tests::export_backup` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-service --lib backup && cargo test -p memory-storage --lib`
- **Per wave merge:** `cargo test --workspace --all-features`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/memory-service/src/backup.rs` -- new module with handler + tests
- [ ] `crates/memory-cli/src/commands/backup.rs` -- new CLI command + tests
- [ ] Storage methods: `list_all_grips()`, `list_all_episodes()` with tests
- [ ] `tokio-stream` dependency added to workspace and relevant crate Cargo.toml files

## Sources

### Primary (HIGH confidence)
- Project source code: proto/memory.proto, crates/memory-service/src/ingest.rs, crates/memory-storage/src/db.rs, crates/memory-storage/src/episodes.rs -- direct code inspection
- [tonic routeguide tutorial](https://github.com/hyperium/tonic/blob/master/examples/routeguide-tutorial.md) -- official server-side streaming example
- [tonic 0.12 minimal streaming example](https://techoverflow.net/2025/09/08/rust-grpc-server-stream-minimal-example/) -- verified streaming types and imports

### Secondary (MEDIUM confidence)
- [Bidirectional gRPC streaming with tonic](https://oneuptime.com/blog/post/2026-01-25-bidirectional-grpc-streaming-tonic-rust/view) -- confirms ReceiverStream pattern for tonic 0.12

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in workspace except tokio-stream (well-established crate)
- Architecture: HIGH - tonic streaming pattern is well-documented and consistent across sources
- Pitfalls: HIGH - identified from direct code inspection of storage key formats and column family layouts
- Storage methods: HIGH - verified exact CF names, key formats, and iteration patterns from db.rs source

**Research date:** 2026-03-23
**Valid until:** 2026-04-23 (stable domain, tonic 0.12 not changing)
