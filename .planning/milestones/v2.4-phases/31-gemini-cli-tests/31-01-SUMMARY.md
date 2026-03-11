---
phase: 31-gemini-cli-tests
plan: 01
subsystem: testing
tags: [bats, gemini, cli, hooks, e2e, jq]

# Dependency graph
requires:
  - phase: 30-claude-code-cli-harness
    provides: "bats-core framework, common.bash, cli_wrappers.bash, daemon lifecycle helpers"
provides:
  - "7 Gemini-format fixture JSON files for hook testing"
  - "smoke.bats with 8 tests covering binary detection, daemon health, ingest, gemini CLI"
  - "hooks.bats with 8 tests covering all 6 Gemini event types via memory-capture.sh"
  - "Fix: memory-capture.sh compact JSON output for memory-ingest compatibility"
affects: [31-02-pipeline-negative, gemini-adapter]

# Tech tracking
tech-stack:
  added: []
  patterns: [two-layer-hook-proof, gemini-event-translation-testing]

key-files:
  created:
    - tests/cli/fixtures/gemini/session-start.json
    - tests/cli/fixtures/gemini/session-end.json
    - tests/cli/fixtures/gemini/before-agent.json
    - tests/cli/fixtures/gemini/after-agent.json
    - tests/cli/fixtures/gemini/before-tool.json
    - tests/cli/fixtures/gemini/after-tool.json
    - tests/cli/fixtures/gemini/malformed.json
    - tests/cli/gemini/smoke.bats
    - tests/cli/gemini/hooks.bats
  modified:
    - plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh

key-decisions:
  - "Fixed jq -n to jq -nc in memory-capture.sh (multi-line JSON broke memory-ingest line-by-line reading)"
  - "sleep 2 between hook invocation and gRPC query (background & ingest needs time)"

patterns-established:
  - "Two-layer hook proof: Layer 1 asserts exit 0 + {} output; Layer 2 queries gRPC for stored content"
  - "Gemini fixtures use compact single-line JSON matching memory-ingest read_line requirement"

# Metrics
duration: 6min
completed: 2026-02-26
---

# Phase 31 Plan 01: Gemini CLI Tests Summary

**16 bats tests (smoke + hooks) validating Gemini CLI hook capture via memory-capture.sh translation layer with jq -nc compact output fix**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-26T04:22:22Z
- **Completed:** 2026-02-26T04:29:18Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- 7 Gemini-format fixture JSON files covering all 6 event types + malformed input
- smoke.bats: 8 tests covering binary detection, daemon health, hook script existence, ingest, gemini CLI
- hooks.bats: 8 tests with two-layer proof (hook script exit 0 + {} output, then gRPC query verification)
- Fixed critical bug in memory-capture.sh: jq -n produced multi-line JSON that memory-ingest silently dropped

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Gemini fixture JSON files** - `19cbafe` (feat)
2. **Task 2: Create smoke.bats and hooks.bats** - `1235372` (feat)

## Files Created/Modified
- `tests/cli/fixtures/gemini/*.json` - 7 Gemini-format fixture files (compact single-line)
- `tests/cli/gemini/smoke.bats` - GEMI-01 smoke tests (8 tests)
- `tests/cli/gemini/hooks.bats` - GEMI-02 hook capture tests (8 tests)
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` - Fixed jq -n to jq -nc

## Decisions Made
- Fixed `jq -n` to `jq -nc` in memory-capture.sh because memory-ingest uses `read_line` (line-by-line), and jq default output is multi-line which silently fails
- Used `sleep 2` between hook invocation and gRPC query to handle background ingest (`&` in hook script)
- All hook tests assert `{}` output (not `{"continue":true}` which is Claude Code specific)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed jq multi-line output in memory-capture.sh**
- **Found during:** Task 2 (hooks.bats testing)
- **Issue:** `jq -n` in memory-capture.sh produced multi-line JSON payloads, but memory-ingest reads stdin with `read_line` (single line only), causing all hook-ingested events to silently fail
- **Fix:** Changed all 6 `jq -n` calls to `jq -nc` for compact single-line output
- **Files modified:** plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh
- **Verification:** Manual end-to-end test confirmed events now stored and queryable via gRPC
- **Committed in:** 1235372 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix -- without compact JSON output, the entire Gemini hook pipeline silently dropped all events. No scope creep.

## Issues Encountered
None beyond the jq multi-line bug documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Gemini fixture files and core test files ready for Phase 31-02 (pipeline + negative tests)
- Hook script fix ensures all future Gemini hook testing works correctly
- All 16 tests pass reliably

## Self-Check: PASSED

All 9 created files verified on disk. Both commit hashes (19cbafe, 1235372) confirmed in git log.

---
*Phase: 31-gemini-cli-tests*
*Completed: 2026-02-26*
