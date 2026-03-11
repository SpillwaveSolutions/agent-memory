# 13-01 Summary: Outbox Consumer Foundation

## Completed

### Task 1: Add memory-indexing crate to workspace
- Added `memory-indexing` to workspace members in root `Cargo.toml`
- Created `crates/memory-indexing/Cargo.toml` with dependencies:
  - memory-storage (workspace)
  - memory-types (workspace)
  - tokio (workspace, features: rt-multi-thread, sync)
  - tracing (workspace)
  - serde (workspace, features: derive)
  - serde_json (workspace)
  - chrono (workspace, features: serde)
  - thiserror (workspace)
  - tempfile (dev-dependency)
- Added `memory-indexing` to workspace dependencies

### Task 2: Implement outbox entry reading in Storage
- Added `get_outbox_entries(start_sequence: u64, limit: usize)` method
  - Returns `Vec<(u64, OutboxEntry)>` tuples in sequence order
  - Uses iterator with `IteratorMode::From` for efficient range scans
  - Supports pagination via start_sequence and limit
- Added `delete_outbox_entries(up_to_sequence: u64)` method
  - Uses `WriteBatch` for atomic deletion
  - Returns count of deleted entries
  - Includes up_to_sequence (inclusive)
- Added 7 new tests for outbox operations:
  - `test_get_outbox_entries_empty`
  - `test_get_outbox_entries_after_event`
  - `test_get_outbox_entries_with_limit`
  - `test_get_outbox_entries_from_offset`
  - `test_delete_outbox_entries`
  - `test_delete_outbox_entries_none`
  - `test_delete_outbox_entries_all`

### Task 3: Create IndexCheckpoint and error types
- Created `error.rs` with `IndexingError` enum:
  - `Storage(StorageError)` - wraps storage errors
  - `Checkpoint(String)` - checkpoint load/save issues
  - `Serialization(String)` - JSON encoding errors
  - `Index(String)` - generic index operation errors
- Created `checkpoint.rs` with:
  - `IndexType` enum: `Bm25`, `Vector`, `Combined`
  - `IndexCheckpoint` struct with:
    - `index_type: IndexType`
    - `last_sequence: u64`
    - `last_processed_time: DateTime<Utc>` (serialized as milliseconds)
    - `processed_count: u64`
    - `created_at: DateTime<Utc>` (serialized as milliseconds)
  - Methods: `new()`, `with_sequence()`, `checkpoint_key()`, `update()`, `to_bytes()`, `from_bytes()`
- Created `lib.rs` with public exports

## Files Modified
- `/Cargo.toml` - Added memory-indexing to workspace members and dependencies
- `/crates/memory-storage/src/db.rs` - Added get_outbox_entries, delete_outbox_entries, and tests
- `/crates/memory-indexing/Cargo.toml` - New file
- `/crates/memory-indexing/src/lib.rs` - New file
- `/crates/memory-indexing/src/error.rs` - New file
- `/crates/memory-indexing/src/checkpoint.rs` - New file

## Verification
- `cargo build -p memory-indexing` - Compiles without warnings
- `cargo test -p memory-indexing` - 10 tests pass
- `cargo test -p memory-storage -- outbox` - 8 tests pass
- `cargo test -p memory-storage` - All 25 tests pass

## Key Decisions
- Used `chrono::serde::ts_milliseconds` for timestamp serialization (JSON-compatible)
- Checkpoint keys follow pattern: `index_bm25`, `index_vector`, `index_combined`
- Outbox iteration uses `IteratorMode::From` with forward direction for efficient range scans
- Delete operations use `WriteBatch` for atomicity

## Ready for Next Phase
The foundation is now in place for Plan 13-02 which will add:
- BM25 index integration
- Vector index integration
- The actual `IndexingPipeline` that consumes entries and updates indexes
