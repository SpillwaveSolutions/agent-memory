---
phase: 33-copilot-cli-tests
plan: 01
subsystem: testing
tags: [bats, copilot, hooks, session-synthesis, cli-testing]

requires:
  - phase: 30-claude-code-cli-harness
    provides: "bats test framework, common.bash, cli_wrappers.bash, daemon lifecycle helpers"
provides:
  - "6 Copilot-native fixture JSON files (ms timestamps, no hook_event_name/session_id)"
  - "run_copilot wrapper in cli_wrappers.bash"
  - "smoke.bats with 8 tests (binary checks, daemon health, ingest, copilot CLI skip)"
  - "hooks.bats with 10 tests (all 5 event types, session synthesis, Bug #991, cleanup)"
affects: [33-copilot-cli-tests, 34-aider-cli-tests]

tech-stack:
  added: []
  patterns: ["Copilot hook $1 argument pattern for event types", "Session ID synthesis via CWD hash temp files"]

key-files:
  created:
    - tests/cli/fixtures/copilot/session-start.json
    - tests/cli/fixtures/copilot/session-end.json
    - tests/cli/fixtures/copilot/user-prompt.json
    - tests/cli/fixtures/copilot/pre-tool-use.json
    - tests/cli/fixtures/copilot/post-tool-use.json
    - tests/cli/fixtures/copilot/malformed.json
    - tests/cli/copilot/smoke.bats
    - tests/cli/copilot/hooks.bats
  modified:
    - tests/cli/lib/cli_wrappers.bash
    - plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh

key-decisions:
  - "Fixed jq -n to jq -nc in Copilot memory-capture.sh (same bug as Phase 31-01 Gemini fix)"

patterns-established:
  - "Copilot hook tests use unique CWD per test via TEST_WORKSPACE/copilot-test-BATS_TEST_NUMBER"
  - "Per-test teardown cleans session files to prevent cross-test leakage"

duration: 4min
completed: 2026-03-05
---

# Phase 33 Plan 01: Copilot CLI Tests Summary

**18 bats tests for Copilot CLI covering smoke, hook capture for all 5 event types, session ID synthesis via CWD hashing, Bug #991 reuse, and session file cleanup**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-05T21:32:42Z
- **Completed:** 2026-03-05T21:36:51Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- 6 Copilot-native fixture JSON files with ms timestamps, no hook_event_name/session_id/agent fields
- run_copilot wrapper added to cli_wrappers.bash with timeout guard
- smoke.bats: 8 tests covering binary detection, daemon health, ingest, and graceful copilot CLI skip
- hooks.bats: 10 tests covering all 5 event types via hook script with $1 argument pattern, session ID synthesis, Bug #991 reuse verification, and session cleanup on terminal/non-terminal reasons

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Copilot-native fixture JSON files and run_copilot wrapper** - `a302816` (feat)
2. **Task 2: Create smoke.bats and hooks.bats for Copilot** - `dab12b8` (feat)

## Files Created/Modified
- `tests/cli/fixtures/copilot/session-start.json` - Copilot-native SessionStart fixture
- `tests/cli/fixtures/copilot/session-end.json` - Copilot-native SessionEnd fixture
- `tests/cli/fixtures/copilot/user-prompt.json` - Copilot-native UserPromptSubmit fixture
- `tests/cli/fixtures/copilot/pre-tool-use.json` - Copilot-native PreToolUse fixture
- `tests/cli/fixtures/copilot/post-tool-use.json` - Copilot-native PostToolUse fixture
- `tests/cli/fixtures/copilot/malformed.json` - Intentionally broken JSON for fail-open testing
- `tests/cli/copilot/smoke.bats` - CPLT-01 smoke tests (8 tests)
- `tests/cli/copilot/hooks.bats` - CPLT-02 hook capture tests (10 tests)
- `tests/cli/lib/cli_wrappers.bash` - Added run_copilot wrapper
- `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` - Fixed jq -n to jq -nc

## Decisions Made
- Fixed jq -n to jq -nc in Copilot memory-capture.sh -- same multi-line JSON bug as Phase 31-01 Gemini fix; memory-ingest reads stdin line-by-line so compact output is required

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed jq -n to jq -nc in memory-capture.sh**
- **Found during:** Task 2 (hooks.bats Layer 2 gRPC verification)
- **Issue:** Hook script used `jq -n` which produces multi-line JSON. memory-ingest reads stdin line-by-line, so only the first line `{` was ingested, silently failing to store events.
- **Fix:** Changed all 5 `jq -n` calls to `jq -nc` for compact single-line output
- **Files modified:** plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh
- **Verification:** All 10 hooks.bats tests pass including gRPC Layer 2 verification
- **Committed in:** dab12b8 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix for hook script correctness. Same pattern as Phase 31-01 Gemini fix.

## Issues Encountered
None beyond the jq compact output bug documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Copilot CLI test coverage complete (18 tests)
- Ready for Phase 33 Plan 02 (pipeline and negative tests) or Phase 34 (Aider CLI tests)

---
*Phase: 33-copilot-cli-tests*
*Completed: 2026-03-05*
