---
phase: 56-import-bootstrap
plan: 02
subsystem: cli
tags: [import, cli, grpc, client-streaming, jsonl, round-trip-test]

requires:
  - phase: 56-import-bootstrap plan 01
    provides: ImportBackup RPC handler, ImportChunk/ImportResult proto types, client import_backup method
  - phase: 55-structured-backup plan 02
    provides: backup CLI command patterns, BackupChunkType, JSONL file structure
provides:
  - memory import CLI command with manifest validation and file reading
  - import_chunks testable API for direct chunk import without tonic Streaming
  - Round-trip integration tests proving import correctness and idempotency
affects: [migration-tooling, disaster-recovery, future-backup-enhancements]

tech-stack:
  added: [tokio-stream (memory-cli dependency)]
  patterns: [read_jsonl_chunks batching, validate_manifest fail-fast, import_chunks testable extraction]

key-files:
  created:
    - crates/memory-cli/src/commands/import.rs
    - crates/memory-service/tests/import_round_trip.rs
  modified:
    - crates/memory-cli/src/cli.rs
    - crates/memory-cli/src/commands/mod.rs
    - crates/memory-cli/src/main.rs
    - crates/memory-cli/Cargo.toml
    - crates/memory-service/src/import.rs

key-decisions:
  - "Extracted import_chunks public fn from import_backup for testable import without tonic Streaming construction"
  - "Event IDs in tests use deterministic ULIDs via ulid::Ulid::from_parts for reproducible test data"

patterns-established:
  - "import_chunks pattern: public testable core logic extracted from gRPC handler for integration testing"
  - "ULID-based test event construction for storage round-trip tests"

requirements-completed: [IMPORT-01, IMPORT-02, IMPORT-03, IMPORT-06]

duration: 10min
completed: 2026-03-24
---

# Phase 56 Plan 02: Import CLI Command + Round-Trip Tests Summary

**CLI `memory import` command with manifest validation, JSONL chunk streaming, and 3 round-trip integration tests proving import correctness, idempotency, and dry-run safety**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-24T20:02:14Z
- **Completed:** 2026-03-24T20:12:14Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments
- `memory import ./dir/` CLI command with --dry-run and --events-only flags
- Manifest version validation (fails fast on unsupported versions)
- File reading in correct order: events (chronological), TOC (segments to years), grips, episodes
- 3 round-trip integration tests: events survive import, duplicates skipped, dry_run prevents writes
- Extracted `import_chunks` as testable public API (avoids tonic Streaming construction complexity)

## Task Commits

Each task was committed atomically:

1. **Task 1: CLI ImportArgs + import command + wiring** - `ea9ee1d` (feat)
2. **Task 2: Round-trip integration test** - `53fa9fc` (test)
3. **Task 3: Full workspace validation + pr-precheck** - `acbd6ae` (chore)

## Files Created/Modified
- `crates/memory-cli/src/commands/import.rs` - CLI import command with manifest validation, file reading, chunk streaming
- `crates/memory-cli/src/cli.rs` - ImportArgs struct with dir, --dry-run, --events-only
- `crates/memory-cli/src/commands/mod.rs` - Module registration for import
- `crates/memory-cli/src/main.rs` - Command dispatch for Import variant
- `crates/memory-cli/Cargo.toml` - Added tokio-stream dependency
- `crates/memory-service/src/import.rs` - Added import_chunks public fn, refactored to share core logic
- `crates/memory-service/tests/import_round_trip.rs` - 3 integration tests (round-trip, idempotency, dry-run)

## Decisions Made
- Extracted `import_chunks` as a `pub` function separate from `import_backup` to enable integration testing without constructing tonic::Streaming (which has no public constructor in tonic 0.12)
- Used deterministic ULIDs via `ulid::Ulid::from_parts` for test events since Storage requires valid ULID event IDs

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added tokio-stream dependency to memory-cli**
- **Found during:** Task 1 (import command creation)
- **Issue:** tokio-stream was not in memory-cli Cargo.toml, needed for streaming chunks to daemon
- **Fix:** Added tokio-stream workspace dependency to memory-cli/Cargo.toml
- **Files modified:** crates/memory-cli/Cargo.toml
- **Verification:** cargo build succeeds
- **Committed in:** ea9ee1d (Task 1 commit)

**2. [Rule 3 - Blocking] Extracted import_chunks for testability**
- **Found during:** Task 2 (integration test creation)
- **Issue:** tonic 0.12 Streaming has no public constructor, preventing direct import_backup testing
- **Fix:** Extracted process_import_chunks core logic, exposed via public import_chunks function
- **Files modified:** crates/memory-service/src/import.rs
- **Verification:** All 3 integration tests pass using import_chunks
- **Committed in:** 53fa9fc (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes necessary for compilation and testing. No scope creep.

## Issues Encountered
- Event IDs must be valid ULIDs for storage retrieval (EventKey::from_event_id parses ULID). Fixed by using ulid::Ulid::from_parts for deterministic test data.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Import CLI feature is complete end-to-end (proto, handler, client, CLI)
- Phase 56 (import-bootstrap) is fully delivered
- Ready for v3.1 milestone completion

---
*Phase: 56-import-bootstrap*
*Completed: 2026-03-24*
