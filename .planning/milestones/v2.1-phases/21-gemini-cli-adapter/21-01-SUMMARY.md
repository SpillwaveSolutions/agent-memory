---
phase: 21-gemini-cli-adapter
plan: 01
subsystem: adapters
tags: [gemini, hooks, shell, jq, event-capture, fail-open, redaction]

# Dependency graph
requires:
  - phase: 18-agent-tagging-infrastructure
    provides: Event.agent field and with_agent() builder on HookEvent
  - phase: 20-opencode-event-capture
    provides: memory-ingest binary with agent field support
provides:
  - Shell hook handler script (memory-capture.sh) transforming Gemini events to memory-ingest format
  - settings.json hook configuration template for all 6 captured Gemini event types
  - ANSI escape sequence stripping for raw terminal output
  - Redaction of sensitive fields (api_key, token, secret, password, credential, authorization)
affects: [21-02-PLAN, 21-03-PLAN, 23-cross-agent-discovery]

# Tech tracking
tech-stack:
  added: [jq (JSON processor, required dependency for hook script)]
  patterns: [fail-open shell hooks with trap/function wrapping, backgrounded binary invocation, ANSI stripping via sed]

key-files:
  created:
    - plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh
    - plugins/memory-gemini-adapter/.gemini/settings.json
  modified:
    - .gitignore

key-decisions:
  - "Use function wrapping with trap ERR EXIT for fail-open behavior instead of global || true"
  - "Use $HOME/.gemini/hooks/memory-capture.sh as default global install path (env var expansion in settings.json)"
  - "MEMORY_INGEST_DRY_RUN env var for testing without memory-ingest binary"
  - "MEMORY_INGEST_PATH env var to override binary location"
  - "Redact message and tool_input fields only (not structural fields like session_id)"
  - "Added .gemini/ override to .gitignore (global gitignore blocks .gemini/ directories)"

patterns-established:
  - "Fail-open shell hook pattern: wrap logic in function, trap ERR/EXIT to output {} and exit 0"
  - "Gemini hook settings.json structure: nested array-of-objects with matcher for tool hooks"
  - "ANSI stripping before JSON parsing: sed with escape sequence pattern"
  - "Sensitive field redaction via jq walk() with case-insensitive key pattern matching"

# Metrics
duration: 5min
completed: 2026-02-10
---

# Phase 21 Plan 01: Gemini CLI Event Capture Summary

**Shell hook handler and settings.json configuration capturing 6 Gemini lifecycle events into agent-memory with fail-open behavior, ANSI stripping, and sensitive field redaction**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-10T15:47:39Z
- **Completed:** 2026-02-10T15:52:20Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Created memory-capture.sh shell hook that transforms all 6 Gemini CLI lifecycle events (SessionStart, SessionEnd, BeforeAgent, AfterAgent, BeforeTool, AfterTool) into memory-ingest JSON format
- Created settings.json with correct nested array-of-objects hook configuration for all 6 event types with matcher support for tool hooks
- Implemented fail-open behavior: script always returns {} and exits 0, even on malformed/empty input or jq failures
- Added ANSI escape sequence stripping to handle colored terminal output from Gemini CLI
- Added sensitive field redaction (api_key, token, secret, password, credential, authorization) using jq walk filter
- All payloads include `agent: "gemini"` tag for cross-agent query support

## Task Commits

Each task was committed atomically:

1. **Task 1: Create memory-capture.sh hook handler script** - `30cd240` (feat) - committed by previous agent execution
2. **Task 2: Create settings.json hook configuration** - `78484fa` (feat)

## Files Created/Modified
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` - Shell hook handler transforming Gemini lifecycle events to memory-ingest format (185 lines)
- `plugins/memory-gemini-adapter/.gemini/settings.json` - Gemini CLI hook configuration registering all 6 event types with correct nested structure
- `.gitignore` - Added .gemini/ override to counter global gitignore rule

## Decisions Made
- **Function wrapping for fail-open:** Wrapped all logic in `main_logic()` function with `trap fail_open ERR EXIT` to guarantee {} output even when `set -euo pipefail` is active. This is more robust than per-line `|| true`.
- **$HOME path for global install:** Used `$HOME/.gemini/hooks/memory-capture.sh` in settings.json command field since Gemini CLI supports env var expansion. Install skill in Plan 03 will adjust for project-level installs.
- **DRY_RUN and PATH env vars:** Added `MEMORY_INGEST_DRY_RUN=1` for testing without the binary, and `MEMORY_INGEST_PATH` to override the default binary location.
- **Redaction scope:** Redact sensitive keys from `tool_input` objects and JSON-formatted `message` fields only. Structural fields (session_id, timestamp, etc.) are never redacted since they're needed for correct event storage.
- **Unique hook names:** Each hook entry has a descriptive unique name (e.g., "memory-capture-session-start", "memory-capture-user-prompt") for identification in logs and debugging.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added .gemini/ override to .gitignore**
- **Found during:** Task 1 (committing memory-capture.sh)
- **Issue:** User's global gitignore at `~/.gitignore_global` has `.gemini/` rule that prevents tracking Gemini adapter files
- **Fix:** Added `!.gemini`, `!.gemini/`, `!**/.gemini`, `!**/.gemini/` overrides to project .gitignore, matching existing .opencode override pattern
- **Files modified:** .gitignore
- **Verification:** `git check-ignore` returns exit 1 (not ignored) after fix
- **Committed in:** 30cd240 (previous agent execution bundled with Task 1)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Essential fix to allow tracking Gemini adapter files. No scope creep.

## Issues Encountered
- Task 1 (memory-capture.sh) was already committed by a previous agent execution in commit 30cd240 (which incorrectly bundled it with Plan 02's TOML command files). The script was verified to be complete and correct, so no duplicate commit was created.

## User Setup Required
None - no external service configuration required. The hook script requires `jq` to be installed on the system (the install skill in Plan 03 will verify this).

## Next Phase Readiness
- Hook handler and settings.json template are ready for the install skill (Plan 03) to deploy
- TOML commands already created (committed in Plan 02's previous execution)
- Plan 02 (skills) and Plan 03 (install skill + README) can proceed

## Self-Check: PASSED

- FOUND: plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh
- FOUND: plugins/memory-gemini-adapter/.gemini/settings.json
- FOUND: .planning/phases/21-gemini-cli-adapter/21-01-SUMMARY.md
- FOUND: commit 30cd240
- FOUND: commit 78484fa

---
*Phase: 21-gemini-cli-adapter*
*Completed: 2026-02-10*
