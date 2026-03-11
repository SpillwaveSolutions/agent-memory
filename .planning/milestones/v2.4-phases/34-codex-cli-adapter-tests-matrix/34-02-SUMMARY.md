---
phase: 34-codex-cli-adapter-tests-matrix
plan: 02
one_liner: "Codex pipeline tests (5 E2E direct-ingest) and negative tests (4 fail-open + 3 skipped hook tests)"
subsystem: cli-testing
tags: [codex, bats, pipeline, negative, fail-open, CchEvent, direct-ingest]
dependency_graph:
  requires:
    - phase: 34-01
      provides: Codex adapter, fixtures, smoke tests, cli_wrappers run_codex
    - phase: 30-cli-harness
      provides: common.bash, cli_wrappers.bash, bats infrastructure
  provides:
    - codex-pipeline-tests
    - codex-negative-tests
  affects: [34-03-matrix-report]
tech_stack:
  added: []
  patterns: [direct-CchEvent-ingest, skip-annotation-for-no-hooks]
key_files:
  created:
    - tests/cli/codex/pipeline.bats
    - tests/cli/codex/negative.bats
  modified: []
key-decisions:
  - "Pipeline tests mirror copilot pattern exactly with agent=codex substitution"
  - "Negative tests use memory-ingest only (no hook script tests since Codex has none)"
  - "Hook-skipped tests annotate GitHub Discussion #2150 as reason"
patterns-established:
  - "No-hooks CLI adapter: skip hook tests with Discussion reference annotation"
metrics:
  duration: "2min"
  completed: "2026-03-05"
  tasks: 2
  files_created: 2
  files_modified: 0
---

# Phase 34 Plan 02: Codex Pipeline and Negative Tests Summary

**Codex pipeline tests (5 E2E direct-ingest) and negative tests (4 fail-open + 3 skipped hook tests)**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-05T23:14:54Z
- **Completed:** 2026-03-05T23:16:21Z
- **Tasks:** 2
- **Files created:** 2

## Accomplishments

- 5 E2E pipeline tests covering session lifecycle, TOC browse, cwd metadata, agent field preservation, concurrent session isolation
- 4 memory-ingest fail-open tests covering daemon-down, malformed JSON, empty stdin, unknown event type
- 3 skipped hook-script tests with clear annotation that Codex has no hooks

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pipeline.bats for Codex E2E ingest-to-query tests** - `d69ae01` (feat)
2. **Task 2: Create negative.bats for Codex fail-open and error handling tests** - `9586a0f` (feat)

## Files Created/Modified

- `tests/cli/codex/pipeline.bats` - 5 E2E pipeline tests with direct CchEvent ingest, agent=codex
- `tests/cli/codex/negative.bats` - 4 memory-ingest fail-open tests + 3 skipped hook tests

## Decisions Made

- Pipeline tests mirror copilot pattern exactly with agent=codex substitution (consistent cross-adapter test structure)
- Negative tests use memory-ingest only since Codex has no hook script
- Hook-skipped tests annotate GitHub Discussion #2150 as the reason for no hooks

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All Codex CLI tests complete (smoke, hooks-skipped, pipeline, negative)
- Ready for Plan 03: Matrix report generation across all 4 CLI adapters

## Self-Check: PASSED

All key files verified present. Both task commits (d69ae01, 9586a0f) confirmed in git log. Test counts: pipeline.bats=5, negative.bats=7.

---
*Phase: 34-codex-cli-adapter-tests-matrix*
*Completed: 2026-03-05*
