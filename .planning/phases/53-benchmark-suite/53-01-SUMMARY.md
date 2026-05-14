---
phase: 53-benchmark-suite
plan: 01
subsystem: testing
tags: [benchmark, toml, fixtures, serde, memory-bench]

requires:
  - phase: 52-simple-cli-api
    provides: "memory CLI binary for benchmark queries"
provides:
  - "memory-bench crate with fixture loader"
  - "TOML fixture format (Fixture, TestCase types)"
  - "3 benchmark fixture files (temporal, multisession, compression)"
  - "7 stub JSONL session files"
  - "Competitor baselines in benchmarks/baselines.toml"
  - "LOCOMO download script"
affects: [53-02, 53-03]

tech-stack:
  added: [memory-bench crate]
  patterns: [TOML fixture loading with validation, JSONL session stubs]

key-files:
  created:
    - crates/memory-bench/Cargo.toml
    - crates/memory-bench/src/lib.rs
    - crates/memory-bench/src/main.rs
    - crates/memory-bench/src/fixture.rs
    - benchmarks/baselines.toml
    - benchmarks/scripts/download-locomo.sh
    - benchmarks/fixtures/temporal-001.toml
    - benchmarks/fixtures/multisession-001.toml
    - benchmarks/fixtures/compression-001.toml
    - benchmarks/fixtures/sessions/auth-decision.jsonl
    - benchmarks/fixtures/sessions/bug-fix.jsonl
    - benchmarks/fixtures/sessions/follow-up.jsonl
    - benchmarks/fixtures/sessions/session-a.jsonl
    - benchmarks/fixtures/sessions/session-b.jsonl
    - benchmarks/fixtures/sessions/session-c.jsonl
    - benchmarks/fixtures/sessions/long-session.jsonl
  modified:
    - Cargo.toml
    - .gitignore

key-decisions:
  - "TOML fixture format with [[test]] arrays for multi-case files"
  - "Fixture::load validates id and query non-empty at parse time"
  - "Fixture::load_dir sorts entries for deterministic ordering"

patterns-established:
  - "Fixture TOML: [[test]] with id, description, setup, query, expected_contains, max_tokens"
  - "Session stubs: JSONL with {role, content} objects per line"

requirements-completed: [BENCH-01, BENCH-06]

duration: 4min
completed: 2026-03-23
---

# Phase 53 Plan 01: Benchmark Suite Foundation Summary

**memory-bench crate with TOML fixture loader, 3 benchmark fixtures, 7 session stubs, and competitor baselines**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-23T02:13:14Z
- **Completed:** 2026-03-23T02:17:00Z
- **Tasks:** 2
- **Files modified:** 18

## Accomplishments
- Scaffolded memory-bench crate as workspace member with all required dependencies
- Implemented Fixture and TestCase types with TOML deserialization and validation
- Created 3 fixture files covering temporal recall, multisession reasoning, and compression
- Created 7 realistic JSONL session stubs for benchmark ingestion
- Added competitor baselines (MemMachine, Mem0) and LOCOMO download script

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold memory-bench crate and benchmark data files** - `5d7cd54` (feat)
2. **Task 2: Implement fixture format and TOML loader with tests** - `4355227` (feat)

## Files Created/Modified
- `crates/memory-bench/Cargo.toml` - Crate manifest with workspace deps
- `crates/memory-bench/src/main.rs` - Minimal binary entrypoint
- `crates/memory-bench/src/lib.rs` - Module declarations
- `crates/memory-bench/src/fixture.rs` - TOML fixture loader with Fixture/TestCase types and 4 tests
- `benchmarks/baselines.toml` - MemMachine and Mem0 competitor scores
- `benchmarks/scripts/download-locomo.sh` - LOCOMO dataset download script
- `benchmarks/fixtures/temporal-001.toml` - Temporal recall test cases
- `benchmarks/fixtures/multisession-001.toml` - Multi-session reasoning test case
- `benchmarks/fixtures/compression-001.toml` - Compression efficiency test case
- `benchmarks/fixtures/sessions/*.jsonl` - 7 session stub files
- `Cargo.toml` - Added memory-bench to workspace members
- `.gitignore` - Added locomo-data/ exclusion

## Decisions Made
- TOML fixture format uses `[[test]]` arrays allowing multiple test cases per file
- Fixture::load validates id and query non-empty at parse time (fail-fast)
- Fixture::load_dir sorts directory entries for deterministic test ordering
- Session stubs use realistic multi-turn conversations about auth, bug fixes, and caching

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Fixture loader types (Fixture, TestCase) ready for runner and scorer in Plan 02
- Session JSONL stubs ready for ingestion testing
- Baselines ready for comparison report generation in Plan 03

---
*Phase: 53-benchmark-suite*
*Completed: 2026-03-23*
