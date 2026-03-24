---
phase: 55-structured-backup
plan: 01
subsystem: backup
tags: [grpc, streaming, jsonl, tokio-stream, backup, protobuf]

requires:
  - phase: 54-daily-export
    provides: ExportDaily RPC pattern, query module, domain-to-proto helpers
provides:
  - ExportBackup server-side streaming RPC (first streaming RPC in project)
  - BackupOptions, BackupChunk, BackupChunkType proto messages
  - list_all_grips and list_all_episodes storage methods
  - backup.rs streaming handler module
affects: [55-02 client/CLI consumer, 56-import-bootstrap]

tech-stack:
  added: [tokio-stream]
  patterns: [tokio mpsc + ReceiverStream for server-side streaming, JSONL chunk batching]

key-files:
  created:
    - crates/memory-service/src/backup.rs
  modified:
    - proto/memory.proto
    - Cargo.toml
    - crates/memory-service/Cargo.toml
    - crates/memory-client/Cargo.toml
    - crates/memory-storage/src/db.rs
    - crates/memory-storage/src/episodes.rs
    - crates/memory-service/src/lib.rs
    - crates/memory-service/src/ingest.rs

key-decisions:
  - "tokio mpsc channel (buffer=64) + ReceiverStream pattern for streaming RPC"
  - "CHUNK_SIZE=100 records per BackupChunk for balanced memory/throughput"
  - "Domain types serialized directly to JSONL (not proto types) for round-trip fidelity"
  - "Grip deserialization uses from_bytes matching existing storage pattern"
  - "Manifest sent last as completion signal with counts and version metadata"

patterns-established:
  - "Server-side streaming: tokio::spawn + mpsc::channel + ReceiverStream"
  - "JSONL batching: collect lines into Vec, flush at CHUNK_SIZE boundary"
  - "list_all_* storage methods: forward iteration with no limit for backup"

requirements-completed: [BACKUP-01, BACKUP-02, BACKUP-03, BACKUP-05, BACKUP-06, BACKUP-07, GRPC-02, GRPC-04]

duration: 5min
completed: 2026-03-24
---

# Phase 55 Plan 01: Structured Backup Server-Side Summary

**ExportBackup streaming RPC with tokio mpsc + ReceiverStream, JSONL chunk batching for events/TOC/grips/episodes, and manifest completion signal**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-24T02:33:35Z
- **Completed:** 2026-03-24T02:38:35Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- First server-side streaming RPC in the project (ExportBackup) compiles and is wired
- BackupOptions, BackupChunk, BackupChunkType proto messages defined with all chunk types
- Storage iteration methods (list_all_grips, list_all_episodes) for backup export
- Streaming handler batches all data types as JSONL chunks with events_only and time-range filtering
- Manifest chunk sent last with counts, version, incremental flag

## Task Commits

Each task was committed atomically:

1. **Task 1: Proto definitions + tokio-stream dependency + storage methods** - `858532b` (feat)
2. **Task 2: Streaming backup handler + service wiring** - `2f2b148` (feat)

## Files Created/Modified
- `proto/memory.proto` - ExportBackup RPC, BackupOptions, BackupChunk, BackupChunkType
- `Cargo.toml` - tokio-stream workspace dependency
- `crates/memory-service/Cargo.toml` - tokio-stream dependency
- `crates/memory-client/Cargo.toml` - tokio-stream dependency
- `crates/memory-storage/src/db.rs` - list_all_grips method (filters node: index keys)
- `crates/memory-storage/src/episodes.rs` - list_all_episodes method (forward iteration)
- `crates/memory-service/src/backup.rs` - Streaming backup handler with all stream_* helpers
- `crates/memory-service/src/lib.rs` - backup module registration
- `crates/memory-service/src/ingest.rs` - ExportBackupStream type + export_backup method wiring

## Decisions Made
- Used tokio mpsc channel (buffer=64) + ReceiverStream for the streaming pattern
- Set CHUNK_SIZE=100 records per BackupChunk for balanced memory/throughput
- Serialize domain types (not proto types) to JSONL for round-trip fidelity with Phase 56 import
- Use Grip::from_bytes (not serde_json::from_slice) to match existing storage deserialization pattern
- Manifest sent last as backup completion signal

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused BackupChunk import from ingest.rs**
- **Found during:** Task 2 (service wiring)
- **Issue:** BackupChunk import in ingest.rs triggered unused_imports warning (clippy -D warnings)
- **Fix:** Removed BackupChunk from pb imports since trait impl uses BackupOptions and Self::ExportBackupStream
- **Files modified:** crates/memory-service/src/ingest.rs
- **Verification:** cargo clippy passes clean
- **Committed in:** 2f2b148 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor import cleanup required for clippy compliance. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Server-side streaming infrastructure complete
- Plan 02 can build client-side consumer and CLI backup command on top
- ReceiverStream + BackupChunk types are exported for client crate use

---
*Phase: 55-structured-backup*
*Completed: 2026-03-24*
