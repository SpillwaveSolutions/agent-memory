---
phase: 30-claude-code-cli-harness
plan: 01
subsystem: testing
tags: [bats, bash, e2e, cli, grpc, daemon-lifecycle]

# Dependency graph
requires: []
provides:
  - "Shared bats test library (common.bash) with workspace isolation and daemon lifecycle"
  - "CLI wrapper library (cli_wrappers.bash) with Claude Code headless invocation helpers"
  - ".gitignore for test run artifacts"
affects: [30-02, 30-03, 30-04, 31, 32, 33, 34]

# Tech tracking
tech-stack:
  added: [bats-core]
  patterns: [setup_file/teardown_file daemon scope, random port selection, fail-open ingest]

key-files:
  created:
    - tests/cli/lib/common.bash
    - tests/cli/lib/cli_wrappers.bash
    - tests/cli/.gitignore

key-decisions:
  - "Random port selection instead of --port 0 (daemon logs requested addr, not bound addr)"
  - "grpcurl preferred for health checks with nc and /dev/tcp fallbacks"
  - "Workspace preserved on test failure for debugging, cleaned on success"

patterns-established:
  - "load ../lib/common pattern: every .bats file sources common.bash for infra"
  - "setup_file scope for daemon: one daemon per .bats file, not per test"
  - "require_cli skip pattern: missing CLI binary skips test with informative message"

# Metrics
duration: 3min
completed: 2026-02-23
---

# Phase 30 Plan 01: Shared Test Helper Library Summary

**Bats test infrastructure with workspace isolation, daemon lifecycle management, CLI detection, and hook pipeline helpers**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-23T06:36:34Z
- **Completed:** 2026-02-23T06:39:16Z
- **Tasks:** 2
- **Files created:** 3

## Accomplishments
- common.bash with 13 functions: workspace setup/teardown, daemon build/start/stop/health, gRPC query, ingest event, assertion helpers
- cli_wrappers.bash with CLI detection (require_cli/has_cli), Claude Code wrappers (run_claude/run_claude_with_hooks), hook pipeline testing (run_hook_stdin/run_hook_stdin_dry), and cross-platform timeout detection
- .gitignore excludes .runs/ directory from version control

## Task Commits

Each task was committed atomically:

1. **Task 1: Create common.bash helper library** - `34b92a2` (feat)
2. **Task 2: Create cli_wrappers.bash and .gitignore** - `cf9626a` (feat)

## Files Created/Modified
- `tests/cli/lib/common.bash` - Shared test helper: workspace isolation, daemon lifecycle, gRPC query, ingest
- `tests/cli/lib/cli_wrappers.bash` - CLI wrappers: detection, Claude Code headless, hook pipeline testing
- `tests/cli/.gitignore` - Ignores .runs/ test workspace artifacts

## Decisions Made
- Used random port selection (RANDOM % 50000 + 10000) instead of --port 0, because the daemon server logs the *requested* address, not the OS-assigned bound address, making port discovery from logs unreliable
- Health check uses grpcurl as primary, with nc and bash /dev/tcp as fallbacks for environments without grpcurl
- Workspaces are preserved on test failure for post-mortem debugging

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Random port instead of --port 0 for port detection**
- **Found during:** Task 1 (common.bash creation)
- **Issue:** Plan specified `--port 0` for OS-assigned port with log parsing. Investigation of server code showed `run_server_with_scheduler` logs the *requested* addr (e.g., `[::1]:0`), not the actual bound port. Port discovery from logs would always show 0.
- **Fix:** Implemented `pick_random_port()` using `$RANDOM` in range 10000-60000. The randomly chosen port is passed to `--port` directly.
- **Files modified:** tests/cli/lib/common.bash
- **Verification:** bash -n syntax check passes; function defined and used in start_daemon
- **Committed in:** 34b92a2 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix -- original approach would not work. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- common.bash and cli_wrappers.bash ready for all subsequent .bats test files
- Plan 30-02 can source these libraries and create actual test files
- Daemon build and lifecycle fully automated

---
*Phase: 30-claude-code-cli-harness*
*Completed: 2026-02-23*
