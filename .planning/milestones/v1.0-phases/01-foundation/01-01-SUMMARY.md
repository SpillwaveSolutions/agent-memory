---
phase: 01-foundation
plan: 01
subsystem: database
tags: [rocksdb, storage, key-encoding, column-families, atomic-writes]

# Dependency graph
requires:
  - phase: 01-00
    provides: workspace scaffolding with memory-storage crate stub
provides:
  - RocksDB wrapper with 6 column families
  - Time-prefixed key encoding for efficient range scans
  - Atomic write batches (event + outbox)
  - Idempotent event writes
  - Checkpoint storage for crash recovery
affects: [01-03-grpc-service, 02-toc-building, 03-grips]

# Tech tracking
tech-stack:
  added: [rocksdb, ulid, tempfile, rand]
  patterns: [column-family-isolation, time-prefixed-keys, atomic-batch-writes, idempotent-upserts]

key-files:
  created:
    - crates/memory-storage/src/column_families.rs
    - crates/memory-storage/src/error.rs
    - crates/memory-storage/src/keys.rs
    - crates/memory-storage/src/db.rs
  modified:
    - crates/memory-storage/src/lib.rs
    - crates/memory-storage/Cargo.toml

key-decisions:
  - "FifoCompactOptions for outbox CF queue workload (STOR-05)"
  - "Zstd compression for events CF space efficiency"
  - "13-digit zero-padded timestamps for lexicographic sorting"
  - "ULID for event_id with embedded timestamp"

patterns-established:
  - "Key format: {prefix}:{timestamp_ms:013}:{ulid} for time-range scans"
  - "StorageError with From impls for RocksDB and serde_json errors"
  - "Atomic batch writes for event + outbox entries"

# Metrics
duration: 15min
completed: 2026-01-29
---

# Phase 01 Plan 01: RocksDB Storage Layer Summary

**RocksDB storage layer with 6 column families, time-prefixed keys (evt:{ts}:{ulid}), atomic batch writes, and idempotent event storage**

## Performance

- **Duration:** 15 min
- **Started:** 2026-01-29T21:44:56Z
- **Completed:** 2026-01-29T22:00:00Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- Defined 6 column families with appropriate compaction (Universal for events, FIFO for outbox)
- Implemented time-prefixed key encoding enabling efficient range scans
- Built Storage struct with atomic event+outbox writes and idempotent duplicate handling
- Added checkpoint storage for crash recovery support
- Comprehensive test suite with 9 passing tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Column families and storage errors** - `b5384fe` (feat)
2. **Task 2: Time-prefixed key encoding** - `3d8c6e0` (feat)
3. **Task 3: Storage struct with RocksDB operations** - `50313ba` (feat)

## Files Created/Modified
- `crates/memory-storage/src/column_families.rs` - CF constants and build_cf_descriptors()
- `crates/memory-storage/src/error.rs` - StorageError enum with RocksDB/key/serialization variants
- `crates/memory-storage/src/keys.rs` - EventKey, OutboxKey, CheckpointKey encoding
- `crates/memory-storage/src/db.rs` - Storage struct with open/put/get/range operations
- `crates/memory-storage/src/lib.rs` - Module exports and re-exports
- `crates/memory-storage/Cargo.toml` - Added rocksdb, ulid, tempfile, rand dependencies

## Decisions Made
- Used `FifoCompactOptions` (not FifoCompactionOptions) per rocksdb 0.22 API
- 13-digit zero-padded timestamps ensure lexicographic ordering works correctly
- ULID contains embedded timestamp, enabling from_event_id reconstruction
- Sequential outbox sequence counter with AtomicU64 for thread-safe monotonic ordering

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed RocksDB API naming**
- **Found during:** Task 1 (Column families module)
- **Issue:** Plan used `FifoCompactionOptions` but rocksdb 0.22 uses `FifoCompactOptions`
- **Fix:** Changed to correct struct name
- **Files modified:** crates/memory-storage/src/column_families.rs
- **Verification:** cargo check passes
- **Committed in:** b5384fe

**2. [Rule 3 - Blocking] Resolved C++ toolchain compilation issue**
- **Found during:** Initial build
- **Issue:** librocksdb-sys build failed due to missing C++ headers (x86_64 Rust on ARM Mac)
- **Fix:** Set SDKROOT, CXXFLAGS, CFLAGS environment variables for proper include paths
- **Files modified:** None (build environment)
- **Verification:** RocksDB compiles successfully

---

**Total deviations:** 2 auto-fixed (both blocking)
**Impact on plan:** Both were necessary for compilation. No scope creep.

## Issues Encountered
- x86_64 Rust toolchain on ARM Mac caused C++ include path issues for librocksdb-sys
  - Resolution: Set explicit SDK and C++ include flags via environment variables
  - Note: This affects all future builds on this system

## Next Phase Readiness
- Storage layer complete and tested
- Ready for:
  - 01-03 gRPC service to use Storage for event persistence
  - Phase 2 TOC building to use events and toc_nodes column families
- Dependency: memory-types Event struct needed for full integration (provided by 01-02)

---
*Phase: 01-foundation*
*Completed: 2026-01-29*
