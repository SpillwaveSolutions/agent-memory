# Phase 56: Import/Bootstrap - Research

**Researched:** 2026-03-24
**Domain:** gRPC client-side streaming, RocksDB bulk write, JSONL import/restore
**Confidence:** HIGH

## Summary

Phase 56 adds a `memory import` CLI command that restores a backup directory to RocksDB via a client-side streaming gRPC RPC. This is the inverse of Phase 55's `ExportBackup` server-side streaming. The CLI reads JSONL files from disk and streams `ImportChunk` messages to the daemon, which writes events, TOC nodes, grips, and episodes to their respective column families.

The codebase already has all the write primitives needed: `put_event` (idempotent by event_id), `put_toc_node` (versioned upsert), `put_grip`, and `store_episode`. The backup format from Phase 55 is well-defined with `BackupChunkType` enum covering all data layers. The existing `rebuild-toc` admin command already exists in `memory-daemon`, so `--events-only` import can reference it.

**Primary recommendation:** Mirror Phase 55's backup.rs pattern but in reverse. Add `ImportBackup` client-streaming RPC to proto, implement server handler receiving `tonic::Streaming<ImportChunk>`, CLI reads backup directory and sends chunks.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- New `import` subcommand added to existing `memory-cli` crate
- `ImportBackup` is a **client-side streaming RPC** -- CLI reads files and streams to daemon
- CLI does file reading; daemon does RocksDB writing
- Positional argument for backup directory: `memory import ./backup-dir/`
- Import order: events first, then TOC nodes (segments -> days -> weeks -> months -> years), grips, episodes
- Trigger outbox entries for events needing indexing
- CLI flags: `memory import ./dir/`, `memory import ./dir/ --events-only`, `memory import ./dir/ --dry-run`
- Idempotent: events with existing IDs are skipped (dedup by event_id in RocksDB)
- Additive only: does NOT delete existing data
- Streaming RPC: `ImportBackup(stream ImportChunk) returns ImportResult`
- Each `ImportChunk` has type tag + JSONL payload (same structure as BackupChunk)
- Round-trip validation: export -> wipe -> import -> queries return same results
- Events -> CF_EVENTS with outbox entries for re-indexing
- TOC nodes -> CF_TOC_NODES + CF_TOC_LATEST
- Grips -> CF_GRIPS
- Episodes -> CF_EPISODES
- BM25/HNSW indexes NOT imported (rebuilt from events via outbox)

### Claude's Discretion
- Whether `rebuild-toc` command exists or needs to be created (RESOLVED: it already exists in `memory-daemon/src/commands.rs`)
- Exact error reporting for partial imports (e.g., 1000/1200 events imported, 200 skipped as duplicates)
- Whether to validate manifest version before streaming (fail fast vs best effort)
- Progress reporting during import (percentage, records/sec)

### Deferred Ideas (OUT OF SCOPE)
- `rebuild-toc` as separate admin command if it doesn't exist (IMPORT-F01) -- already exists
- Selective import (e.g., import only events from a specific date range)
- Import from remote URL (e.g., `memory import https://...`)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| IMPORT-01 | `memory import ./dir/` restores a full backup to RocksDB | Storage write methods confirmed: `put_event`, `put_toc_node`, `put_grip`, `store_episode` |
| IMPORT-02 | Round-trip test: export -> wipe -> import -> queries return same results | E2E test crate exists at `crates/e2e-tests/`; can create `import_roundtrip_test.rs` |
| IMPORT-03 | `memory import --dry-run` shows what would be imported without writing | CLI reads files and counts records; daemon handler can accept dry_run flag in ImportResult logic |
| IMPORT-04 | Idempotent -- events with existing IDs are skipped | `put_event` already checks `db.get_cf` before write, returns `created=false` |
| IMPORT-05 | `ImportBackup` uses client-side gRPC streaming | Tonic 0.12 supports `rpc Import(stream Msg) returns (Resp)` natively |
| IMPORT-06 | Events-only import works; user triggers TOC rebuild after | `rebuild-toc` admin command exists in `memory-daemon/src/commands.rs` |
| GRPC-03 | `ImportBackup` client-side streaming RPC accepts JSONL chunks | Proto syntax confirmed; tonic generates `Streaming<T>` parameter |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tonic | 0.12 | gRPC framework (client-streaming RPC) | Already used project-wide |
| tokio-stream | workspace | Stream utilities for client-side streaming | Already in memory-client Cargo.toml |
| serde_json | workspace | JSONL parsing for import chunks | Already used throughout |
| clap | workspace | CLI argument parsing (Import variant) | Already used in memory-cli |
| anyhow | workspace | Error handling in CLI command | Already used in backup command |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chrono | workspace | Timestamp handling in events | Already used |
| tracing | workspace | Structured logging | Already used |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Client-streaming RPC | Batch unary RPCs | Streaming avoids large message size limits; better for large backups |
| JSONL import format | Binary protobuf | JSONL matches Phase 55 export format; human-readable |

## Architecture Patterns

### Recommended Project Structure
```
crates/memory-cli/src/commands/
    import.rs              # NEW: import CLI command (mirror of backup.rs)
crates/memory-client/src/client.rs
    import_backup()        # NEW: client-streaming method
crates/memory-service/src/
    import.rs              # NEW: import handler (mirror of backup.rs)
    ingest.rs              # MODIFY: add ImportBackup to MemoryService impl
proto/memory.proto         # MODIFY: add ImportBackup RPC, ImportChunk, ImportResult messages
crates/e2e-tests/tests/
    import_roundtrip_test.rs  # NEW: round-trip test
```

### Pattern 1: Client-Side Streaming RPC (Tonic 0.12)

**What:** Client sends a stream of messages; server receives all, returns single response.
**When to use:** Importing large datasets where total payload exceeds single message limits.

**Proto definition:**
```protobuf
// Client-streaming RPC for import
rpc ImportBackup(stream ImportChunk) returns (ImportResult);

// Reuse BackupChunkType from Phase 55
message ImportChunk {
    BackupChunkType chunk_type = 1;
    string jsonl_data = 2;
    uint32 record_count = 3;
    bool dry_run = 4;
    bool events_only = 5;
}

message ImportResult {
    uint64 events_imported = 1;
    uint64 events_skipped = 2;
    uint64 toc_nodes_imported = 3;
    uint64 grips_imported = 4;
    uint64 episodes_imported = 5;
    uint64 errors = 6;
    double elapsed_seconds = 7;
    bool dry_run = 8;
}
```

**Server handler signature (tonic generates):**
```rust
// In the MemoryService trait, tonic generates:
async fn import_backup(
    &self,
    request: Request<Streaming<ImportChunk>>,
) -> Result<Response<ImportResult>, Status>;
```

**Server implementation pattern:**
```rust
async fn import_backup(
    &self,
    request: Request<Streaming<ImportChunk>>,
) -> Result<Response<ImportResult>, Status> {
    let mut stream = request.into_inner();
    let mut counts = ImportCounts::default();

    while let Some(chunk) = stream.message().await? {
        match chunk.chunk_type() {
            BackupChunkType::Events => {
                import_events(&self.storage, &chunk.jsonl_data, chunk.dry_run, &mut counts)?;
            }
            BackupChunkType::TocSegments | BackupChunkType::TocDays | ... => {
                import_toc_nodes(&self.storage, &chunk.jsonl_data, chunk.dry_run, &mut counts)?;
            }
            BackupChunkType::Grips => {
                import_grips(&self.storage, &chunk.jsonl_data, chunk.dry_run, &mut counts)?;
            }
            BackupChunkType::Episodes => {
                import_episodes(&self.storage, &chunk.jsonl_data, chunk.dry_run, &mut counts)?;
            }
            _ => { /* skip manifest, unspecified */ }
        }
    }
    Ok(Response::new(counts.into_result()))
}
```

**Client-side streaming pattern:**
```rust
// In MemoryClient:
pub async fn import_backup(
    &mut self,
    chunks: impl Stream<Item = ImportChunk> + Send + 'static,
) -> Result<ImportResult, ClientError> {
    let response = self.inner.import_backup(chunks).await?;
    Ok(response.into_inner())
}
```

### Pattern 2: CLI File Reader (Mirror of backup.rs)

**What:** CLI reads backup directory, constructs ImportChunk stream, sends to daemon.
**Example:**
```rust
pub async fn run(args: ImportArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = connect_client(&global.endpoint).await?;
    let base = PathBuf::from(&args.dir);

    // 1. Read and validate manifest
    let manifest = read_manifest(&base)?;
    validate_manifest_version(&manifest)?;

    // 2. Build chunk stream from files
    let chunks = build_import_stream(&base, args.events_only, args.dry_run)?;

    // 3. Stream to daemon
    let result = client.import_backup(chunks).await?;

    // 4. Report results
    report_import_result(&result);
    Ok(())
}
```

### Pattern 3: Idempotent Event Import with Outbox

**What:** For events, use `put_event` (not `put_event_only`) so events get outbox entries for re-indexing.
**Key insight:** `put_event` already handles idempotency -- if event_id exists, it returns `(key, false)` and skips the write. New events get outbox entries, triggering BM25/HNSW/TOC indexing.

```rust
fn import_event_line(storage: &Storage, line: &str) -> Result<bool, ImportError> {
    let event: Event = serde_json::from_str(line)?;
    let event_bytes = event.to_bytes()?;
    let outbox_entry = OutboxEntry::new_event(event.event_id.clone());
    let outbox_bytes = serde_json::to_vec(&outbox_entry)?;
    let (_key, created) = storage.put_event(&event.event_id, &event_bytes, &outbox_bytes)?;
    Ok(created)
}
```

### Anti-Patterns to Avoid
- **Don't batch-write without idempotency checks:** Each event must be checked individually via `put_event`'s built-in idempotency.
- **Don't import TOC nodes before events:** Events are the base layer; TOC is derived.
- **Don't delete existing data before import:** Import is additive-only per requirements.
- **Don't import manifest as data:** Manifest is metadata for validation, not a RocksDB record.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Event idempotency | Custom dedup logic | `Storage::put_event` (checks before write) | Already handles ING-03 |
| TOC versioning | Manual version tracking | `Storage::put_toc_node` (auto-increments version) | Already handles TOC-06 |
| Grip index entries | Manual `node:` index keys | `Storage::put_grip` (creates index entry) | Handles toc_node_id linking |
| Stream construction | Manual async stream | `tokio_stream::iter()` or `async_stream::stream!` | Standard stream adapter |
| Outbox entries | Custom queue logic | `Storage::put_event` (atomic event+outbox write) | Already handles ING-05 |

## Common Pitfalls

### Pitfall 1: ImportChunk dry_run flag per-chunk vs per-stream
**What goes wrong:** Putting dry_run on every chunk is redundant and error-prone if chunks disagree.
**Why it happens:** Natural impulse to put flags on the message that carries data.
**How to avoid:** Send dry_run and events_only as fields on the FIRST chunk (or as a separate initial control message), or include them on every chunk but validate consistency. Simplest: include on every chunk, daemon uses first chunk's value.
**Warning signs:** Tests where some chunks are dry_run=true and others false.

### Pitfall 2: TOC node version conflicts on import
**What goes wrong:** `put_toc_node` auto-increments version from current latest. If importing a node that already exists, the imported data gets a new version number (not the original).
**Why it happens:** TOC versioning is designed for append-only mutation, not bulk restore.
**How to avoid:** For import, this is actually correct behavior -- the imported node content overwrites with a new version. The data is preserved even if version numbers differ from the original.
**Warning signs:** Round-trip test asserting exact version numbers (don't do this -- assert content equality instead).

### Pitfall 3: Event timestamp format mismatch
**What goes wrong:** Phase 55 serializes domain `Event` (with `DateTime<Utc>` timestamp), but `put_event` expects `EventKey::from_event_id` which parses the ULID-based event_id.
**Why it happens:** Events are serialized as domain types (per Phase 55 decision), not proto types.
**How to avoid:** Deserialize JSONL line into `memory_types::Event`, then use `event.to_bytes()` for storage. The `event.event_id` contains the timestamp-encoded ULID.
**Warning signs:** Events stored but not findable by time range queries.

### Pitfall 4: Missing outbox entry serialization
**What goes wrong:** `put_event` requires `outbox_bytes` parameter, but import code forgets to construct it.
**Why it happens:** Export doesn't include outbox data (it's transient).
**How to avoid:** Construct `OutboxEntry` for each imported event, serialize it, pass to `put_event`.
**Warning signs:** Events stored but never indexed by BM25/HNSW.

### Pitfall 5: Stream type mismatch in tonic client
**What goes wrong:** `import_backup` on the generated client expects a specific stream type.
**Why it happens:** Tonic 0.12 client methods accept `impl IntoStreamingRequest<Message = T>`.
**How to avoid:** Use `tokio_stream::iter(chunks_vec)` to convert a `Vec<ImportChunk>` into a stream, or use `async_stream::stream!` for lazy file reading.
**Warning signs:** Compile errors about stream trait bounds.

## Code Examples

### Example 1: Adding ImportBackup to MemoryService trait impl
```rust
// In ingest.rs, add to the impl block:
#[tonic::async_trait]
impl MemoryService for MemoryServiceImpl {
    // Existing:
    type ExportBackupStream = backup::ExportBackupStream;

    // ... existing methods ...

    async fn import_backup(
        &self,
        request: Request<Streaming<ImportChunk>>,
    ) -> Result<Response<ImportResult>, Status> {
        import::import_backup(self.storage.clone(), request).await
    }
}
```

### Example 2: Client-side stream construction
```rust
// In CLI import command:
use tokio_stream::iter as stream_iter;

let chunks: Vec<ImportChunk> = Vec::new();
// ... read files, build chunks ...

let stream = stream_iter(chunks);
let result = client.import_backup(stream).await?;
```

### Example 3: CLI ImportArgs definition
```rust
/// Arguments for the `import` subcommand.
#[derive(Parser, Debug)]
pub struct ImportArgs {
    /// Backup directory to import from.
    pub dir: String,

    /// Only import events (skip TOC, grips, episodes).
    #[arg(long)]
    pub events_only: bool,

    /// Show what would be imported without writing.
    #[arg(long)]
    pub dry_run: bool,
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Unary batch RPCs | Client-side streaming RPC | tonic 0.12 stable | No message size limits for large imports |
| Custom idempotency | Built-in `put_event` check | Phase 1 (ING-03) | Import gets idempotency for free |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in) + E2E harness |
| Config file | `crates/e2e-tests/Cargo.toml` |
| Quick run command | `cargo test -p memory-service --lib import` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IMPORT-01 | Full backup restore to RocksDB | integration | `cargo test -p e2e-tests import_roundtrip` | No - Wave 0 |
| IMPORT-02 | Round-trip: export -> wipe -> import -> verify | integration | `cargo test -p e2e-tests import_roundtrip` | No - Wave 0 |
| IMPORT-03 | Dry-run shows counts without writing | unit | `cargo test -p memory-service --lib import::tests::test_dry_run` | No - Wave 0 |
| IMPORT-04 | Idempotent skip of existing events | unit | `cargo test -p memory-service --lib import::tests::test_idempotent` | No - Wave 0 |
| IMPORT-05 | Client-streaming RPC works end-to-end | integration | `cargo test -p e2e-tests import_roundtrip` | No - Wave 0 |
| IMPORT-06 | Events-only import | unit | `cargo test -p memory-service --lib import::tests::test_events_only` | No - Wave 0 |
| GRPC-03 | ImportBackup RPC in proto compiles | build | `cargo build -p memory-service` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-service --lib import && cargo test -p memory-cli --lib`
- **Per wave merge:** `cargo test --workspace --all-features`
- **Phase gate:** Full suite green + `task pr-precheck`

### Wave 0 Gaps
- [ ] `proto/memory.proto` -- add ImportBackup RPC, ImportChunk, ImportResult messages
- [ ] `crates/memory-service/src/import.rs` -- import handler with unit tests
- [ ] `crates/e2e-tests/tests/import_roundtrip_test.rs` -- round-trip validation test

## Open Questions

1. **Stream vs Vec for large imports**
   - What we know: `tokio_stream::iter(vec)` works but loads all chunks into memory first
   - What's unclear: Whether backups can be large enough to OOM with Vec approach
   - Recommendation: Use lazy streaming via `async_stream::stream!` that reads files on-demand; if too complex, Vec is fine for v3.1 (backups are typically <100MB)

2. **Progress reporting granularity**
   - What we know: CLI can print to stderr during streaming
   - What's unclear: Whether to count records as they're sent or wait for final ImportResult
   - Recommendation: Print per-file progress on send (stderr), then final summary from ImportResult

3. **Manifest version validation**
   - What we know: manifest.json has `"version": "1.0"`
   - What's unclear: Whether to fail on version mismatch or warn
   - Recommendation: Validate before streaming (fail fast). Print error and exit if version unsupported.

## Sources

### Primary (HIGH confidence)
- `proto/memory.proto` -- existing BackupChunk/BackupChunkType messages, ExportBackup streaming RPC
- `crates/memory-service/src/backup.rs` -- ExportBackup handler pattern (channel + spawn)
- `crates/memory-storage/src/db.rs` -- `put_event` (idempotent, atomic with outbox), `put_toc_node` (versioned), `put_grip`, lines 84-155, 317-365, 491-512
- `crates/memory-storage/src/episodes.rs` -- `store_episode` method
- `crates/memory-client/src/client.rs` -- `export_backup` client method pattern
- `crates/memory-cli/src/commands/backup.rs` -- backup CLI command (file I/O patterns)
- `crates/memory-cli/src/cli.rs` -- Commands enum, BackupArgs pattern
- `crates/memory-daemon/src/commands.rs` -- `rebuild-toc` admin command exists (line 1068)

### Secondary (MEDIUM confidence)
- Tonic 0.12 client-streaming: handler receives `Request<Streaming<T>>`, client accepts `impl IntoStreamingRequest`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in workspace, no new dependencies
- Architecture: HIGH - mirrors Phase 55 export pattern exactly (inverse direction)
- Pitfalls: HIGH - identified from reading actual storage code (idempotency, versioning, outbox)

**Research date:** 2026-03-24
**Valid until:** 2026-04-24 (stable patterns, no external dependencies)
