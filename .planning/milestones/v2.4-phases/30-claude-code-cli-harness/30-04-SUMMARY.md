---
phase: 30-claude-code-cli-harness
plan: 04
subsystem: testing
tags: [bats, e2e, pipeline, negative-testing, fail-open, cli-harness]

# Dependency graph
requires:
  - "30-01: Shared test helper library (common.bash, cli_wrappers.bash)"
  - "30-02: Fixture JSON files and CI workflow"
provides:
  - "5 E2E pipeline tests proving full hook -> ingest -> gRPC query cycle"
  - "7 negative tests verifying fail-open behavior under error conditions"
affects: [31-gemini-cli-harness, 32-opencode-harness, 33-copilot-codex-harness, 34-cross-cli-matrix]

# Tech tracking
tech-stack:
  added: []
  patterns: [fail-open-verification, event-content-assertion, port-50051-pinning]

key-files:
  created:
    - tests/cli/claude-code/pipeline.bats
    - tests/cli/claude-code/negative.bats
  modified:
    - crates/memory-client/src/client.rs
    - crates/memory-daemon/src/cli.rs

key-decisions:
  - "DEFAULT_ENDPOINT changed from [::1] to 127.0.0.1 to match daemon 0.0.0.0 bind"
  - "Pipeline tests pin to port 50051 (memory-ingest hardcodes DEFAULT_ENDPOINT)"
  - "Assertions verify event content/count rather than session_id (not in query output format)"
  - "Health check uses nc TCP connect before grpcurl (daemon lacks grpc.health service)"

patterns-established:
  - "Pipeline tests: ingest_event + grpc_query events pattern"
  - "Negative tests: pipe to memory-ingest, assert exit 0 + continue:true"
  - "Suppress ingest stdout with >/dev/null to avoid polluting bats output"

# Metrics
duration: 17min
completed: 2026-02-23
---

# Phase 30 Plan 04: Pipeline and Negative Tests Summary

**E2E pipeline tests proving hook-ingest-query cycle and 7 negative tests verifying fail-open resilience**

## Performance

- **Duration:** 17 min
- **Started:** 2026-02-23T06:41:36Z
- **Completed:** 2026-02-23T06:58:00Z
- **Tasks:** 2
- **Files created:** 2
- **Files modified:** 2

## Accomplishments
- pipeline.bats: 5 tests covering complete session lifecycle (6 events), TOC browse query, cwd metadata storage, real Claude Code hook fire, and concurrent session isolation
- negative.bats: 7 tests covering daemon down, malformed JSON, empty stdin, unknown event type, timeout enforcement, wrong port, and large payload (100KB) -- all verify fail-open behavior
- Fixed IPv4/IPv6 mismatch: DEFAULT_ENDPOINT changed from `http://[::1]:50051` to `http://127.0.0.1:50051`
- Fixed clap short flag conflict: removed `-l` short from global `--log-level` to avoid clash with `--limit` in subcommands

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pipeline.bats for full E2E hook-to-query verification** - `75885f9` (feat)
2. **Task 2: Create negative.bats for error handling and edge cases** - `67c601e` (feat)

## Files Created/Modified
- `tests/cli/claude-code/pipeline.bats` - 5 E2E pipeline tests: lifecycle, TOC, cwd, real claude, isolation
- `tests/cli/claude-code/negative.bats` - 7 negative tests: daemon down, malformed, empty, unknown, timeout, wrong port, large payload
- `crates/memory-client/src/client.rs` - DEFAULT_ENDPOINT: [::1] -> 127.0.0.1
- `crates/memory-daemon/src/cli.rs` - All CLI endpoint defaults: [::1] -> 127.0.0.1, removed log_level short flag

## Decisions Made
- Changed DEFAULT_ENDPOINT from IPv6 loopback (`[::1]`) to IPv4 loopback (`127.0.0.1`) because daemon binds to `0.0.0.0` which does not accept IPv6 connections on macOS
- Pipeline tests must use port 50051 because memory-ingest hardcodes DEFAULT_ENDPOINT (no env var override)
- Assertions check event content and count rather than session_id, since the `memory-daemon query events` output format does not include session_id
- Health check in common.bash uses nc TCP connect first because the daemon does not expose `grpc.health.v1.Health` service via reflection

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] IPv4/IPv6 endpoint mismatch prevented ingest from reaching daemon**
- **Found during:** Task 1 (pipeline test development)
- **Issue:** DEFAULT_ENDPOINT was `http://[::1]:50051` (IPv6) but daemon binds to `0.0.0.0` (IPv4 only on macOS). memory-ingest silently failed to connect (fail-open) so events were never ingested.
- **Fix:** Changed DEFAULT_ENDPOINT to `http://127.0.0.1:50051` in memory-client. Updated all CLI default values in memory-daemon/src/cli.rs.
- **Files modified:** crates/memory-client/src/client.rs, crates/memory-daemon/src/cli.rs
- **Committed in:** 75885f9 (Task 1 commit)

**2. [Rule 1 - Bug] Clap short flag conflict between --log-level and --limit**
- **Found during:** Task 1 (pipeline test development)
- **Issue:** Global `--log-level` had `#[arg(short, long)]` giving `-l` short, which conflicted with `--limit` short `-l` in Events/Browse subcommands. Debug builds panic on this assertion.
- **Fix:** Removed `short` attribute from `log_level` in Cli struct. Users can still use `--log-level` (long form).
- **Files modified:** crates/memory-daemon/src/cli.rs
- **Committed in:** 75885f9 (Task 1 commit)

**3. [Rule 3 - Blocking] Health check used grpcurl health service that daemon doesn't expose**
- **Found during:** Task 1 (daemon startup failed health check)
- **Issue:** daemon_health_check tried `grpcurl grpc.health.v1.Health/Check` but daemon only exposes `memory.MemoryService` and `grpc.reflection`. Health check always failed, preventing daemon startup confirmation.
- **Fix:** common.bash health check already updated (from plan 30-03) to use `nc` TCP connect first.
- **Files modified:** None (already fixed in common.bash)
- **Committed in:** N/A (pre-existing fix)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking)
**Impact on plan:** Essential fixes -- without them, the entire pipeline test suite would be unable to verify event ingestion.

## Issues Encountered
None beyond the auto-fixed deviations.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 30 Claude Code bats tests (across 4 .bats files) now pass
- Framework is ready for phases 31-34 (Gemini, OpenCode, Copilot/Codex, cross-CLI matrix)
- IPv4/IPv6 fix benefits all future CLI harness work

## Self-Check: PASSED

- All 2 created files verified present on disk
- Commit 75885f9 (Task 1) verified in git log
- Commit 67c601e (Task 2) verified in git log

---
*Phase: 30-claude-code-cli-harness*
*Completed: 2026-02-23*
