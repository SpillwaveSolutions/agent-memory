---
phase: 53-benchmark-suite
plan: 02
subsystem: testing
tags: [benchmark, scorer, runner, clap, serde, toml, markdown]

requires:
  - phase: 53-benchmark-suite plan 01
    provides: "Fixture loader, TOML format, test case structs"
provides:
  - "Benchmark runner shelling out to memory binary"
  - "Scorer: accuracy, recall@k, percentile, compression ratio"
  - "Report generator: JSON + markdown with optional baseline comparison"
  - "Baseline TOML loader for competitor scores"
  - "CLI with temporal, multisession, compression, all, locomo subcommands"
affects: [53-benchmark-suite plan 03, benchmark-integration]

tech-stack:
  added: []
  patterns: ["shell-out runner pattern via std::process::Command", "BenchmarkReport as shared metric struct"]

key-files:
  created:
    - crates/memory-bench/src/scorer.rs
    - crates/memory-bench/src/runner.rs
    - crates/memory-bench/src/report.rs
    - crates/memory-bench/src/baseline.rs
    - crates/memory-bench/src/cli.rs
  modified:
    - crates/memory-bench/src/lib.rs
    - crates/memory-bench/src/main.rs

key-decisions:
  - "Runner shells out via std::process::Command (no in-process coupling)"
  - "Compression ratio: 1.0 - (context_tokens / raw_tokens), raw_tokens from chars/4"
  - "CLI uses clap subcommands with global --memory-bin flag"
  - "Report supports single-column and comparison table modes"

patterns-established:
  - "BenchmarkReport as shared metric struct across scorer/report modules"
  - "Category filtering by test ID prefix (temporal, multi, compress)"

requirements-completed: [BENCH-02, BENCH-03, BENCH-05, BENCH-08]

duration: 3min
completed: 2026-03-23
---

# Phase 53 Plan 02: Runner, Scorer, Report, and CLI Summary

**Benchmark engine with scorer (accuracy/recall/compression), report generator (JSON+markdown), baseline comparison, and 5-subcommand CLI**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-23T02:19:10Z
- **Completed:** 2026-03-23T02:22:00Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Scorer module with accuracy, recall@k, percentile, compression ratio, token estimation (11 unit tests)
- Runner module shelling out to memory binary for search and session ingestion
- Report module generating JSON (round-trippable) and markdown tables with optional baseline columns
- Baseline TOML loader for MemMachine and Mem0 competitor scores
- CLI with temporal, multisession, compression, all, locomo subcommands and --compare flag
- Full pipeline: load fixtures -> filter by category -> ingest sessions -> query -> score -> report

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement runner, scorer, report, and baseline modules** - `4d63f2b` (feat)
2. **Task 2: Wire CLI subcommands and run pipeline** - `dc37de6` (feat)

## Files Created/Modified
- `crates/memory-bench/src/scorer.rs` - Accuracy, recall@k, percentile, compression ratio, BenchmarkReport struct
- `crates/memory-bench/src/runner.rs` - Shell out to memory binary for queries and session ingestion
- `crates/memory-bench/src/report.rs` - JSON and markdown report generation with baseline comparison
- `crates/memory-bench/src/baseline.rs` - TOML loader for competitor benchmark scores
- `crates/memory-bench/src/cli.rs` - Clap CLI with 5 subcommands and global --memory-bin flag
- `crates/memory-bench/src/main.rs` - Full pipeline wiring: fixtures -> run -> score -> report
- `crates/memory-bench/src/lib.rs` - Module declarations

## Decisions Made
- Runner shells out via std::process::Command (clean separation, no in-process coupling)
- Compression ratio formula: 1.0 - (context_tokens / raw_tokens), with raw_tokens estimated as total_chars / 4
- CLI uses clap subcommands with global --memory-bin flag for binary path override
- Report supports both single-column (no baselines) and multi-column (with baselines) table modes
- Category filtering done by test ID prefix matching

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All core modules ready for Plan 03 (LOCOMO adapter)
- LOCOMO subcommand placeholder wired in CLI, ready for implementation
- BenchmarkReport struct shared across scorer and report modules

---
*Phase: 53-benchmark-suite*
*Completed: 2026-03-23*
