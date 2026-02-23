---
phase: 30-claude-code-cli-harness
plan: 02
subsystem: testing
tags: [bats, fixtures, ci, github-actions, e2e, cli-harness]

# Dependency graph
requires:
  - phase: 30-claude-code-cli-harness
    provides: "Phase context and plan structure for CLI harness"
provides:
  - "10 Claude Code event fixture JSON files for deterministic bats testing"
  - "e2e-cli.yml GitHub Actions workflow with 5-CLI x 2-OS matrix"
  - "JUnit XML report generation and failure artifact uploads"
affects: [30-03, 30-04, 31-gemini-cli-harness, 32-opencode-harness, 33-copilot-codex-harness, 34-cross-cli-matrix]

# Tech tracking
tech-stack:
  added: [bats-core, bats-support, bats-assert, junit-formatter]
  patterns: [fixture-based-testing, matrix-ci, skip-on-missing-cli]

key-files:
  created:
    - tests/cli/fixtures/claude-code/session-start.json
    - tests/cli/fixtures/claude-code/user-prompt.json
    - tests/cli/fixtures/claude-code/pre-tool-use.json
    - tests/cli/fixtures/claude-code/post-tool-use.json
    - tests/cli/fixtures/claude-code/assistant-response.json
    - tests/cli/fixtures/claude-code/subagent-start.json
    - tests/cli/fixtures/claude-code/subagent-stop.json
    - tests/cli/fixtures/claude-code/stop.json
    - tests/cli/fixtures/claude-code/session-end.json
    - tests/cli/fixtures/claude-code/malformed.json
    - .github/workflows/e2e-cli.yml
  modified: []

key-decisions:
  - "Fixtures match CchEvent struct fields from memory-ingest/src/main.rs"
  - "Bats helper libraries installed via git clone in CI (reliable across Linux/macOS)"
  - "Missing CLI test directory results in skip annotation, not failure"
  - "JUnit XML retained 14 days, failure artifacts retained 7 days"

patterns-established:
  - "Fixture convention: tests/cli/fixtures/{cli-name}/{event-type}.json"
  - "CI matrix: fail-fast false with continue-on-error for bats + post-check step"
  - "BATS_LIB_PATH env var points to tests/cli/lib for helper libraries"

# Metrics
duration: 1min
completed: 2026-02-23
---

# Phase 30 Plan 02: Fixtures and CI Workflow Summary

**10 Claude Code event fixture JSONs plus e2e-cli.yml with 5-CLI x 2-OS bats matrix and JUnit reporting**

## Performance

- **Duration:** 1 min
- **Started:** 2026-02-23T06:36:39Z
- **Completed:** 2026-02-23T06:37:50Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Created 10 fixture JSON files covering all 9 Claude Code event types plus 1 malformed fixture for negative tests
- Created e2e-cli.yml GitHub Actions workflow with 5-CLI (claude-code, gemini, opencode, copilot, codex) x 2-OS matrix
- Configured JUnit XML report generation, failure artifact uploads, and step summary reporting
- Skip-safe design: missing CLI test directories produce annotations, not failures

## Task Commits

Each task was committed atomically:

1. **Task 1: Create fixture JSON payloads for all Claude Code event types** - `94c7e47` (feat)
2. **Task 2: Create e2e-cli.yml GitHub Actions workflow with 5-CLI matrix** - `43120ba` (feat)

## Files Created/Modified
- `tests/cli/fixtures/claude-code/session-start.json` - SessionStart event fixture
- `tests/cli/fixtures/claude-code/user-prompt.json` - UserPromptSubmit event fixture
- `tests/cli/fixtures/claude-code/pre-tool-use.json` - PreToolUse event fixture
- `tests/cli/fixtures/claude-code/post-tool-use.json` - PostToolUse event fixture
- `tests/cli/fixtures/claude-code/assistant-response.json` - AssistantResponse event fixture
- `tests/cli/fixtures/claude-code/subagent-start.json` - SubagentStart event fixture
- `tests/cli/fixtures/claude-code/subagent-stop.json` - SubagentStop event fixture
- `tests/cli/fixtures/claude-code/stop.json` - Stop event fixture
- `tests/cli/fixtures/claude-code/session-end.json` - SessionEnd event fixture
- `tests/cli/fixtures/claude-code/malformed.json` - Intentionally invalid JSON for negative testing
- `.github/workflows/e2e-cli.yml` - E2E CLI test workflow with 5-CLI matrix

## Decisions Made
- Fixtures use same field names as CchEvent struct in memory-ingest for compatibility
- Bats helper libraries (bats-support, bats-assert) installed via git clone rather than npm for cross-platform reliability
- Missing CLI test directory triggers skip annotation (not failure) so matrix jobs pass gracefully
- JUnit XML reports retained 14 days; failure artifacts retained 7 days

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Fixtures ready for bats test scripts in plan 03
- CI workflow ready to execute once test scripts exist in tests/cli/{cli-name}/
- BATS_LIB_PATH and binary path env vars set for test consumption

## Self-Check: PASSED

- All 11 created files verified present on disk
- Commit 94c7e47 (Task 1) verified in git log
- Commit 43120ba (Task 2) verified in git log

---
*Phase: 30-claude-code-cli-harness*
*Completed: 2026-02-23*
