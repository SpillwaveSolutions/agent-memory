---
phase: 22-copilot-cli-adapter
plan: 01
subsystem: infra
tags: [copilot, shell, hooks, session-id, event-capture, memory-ingest, jq]

# Dependency graph
requires:
  - phase: 18-agent-tagging-infrastructure
    provides: Event.agent field and agent:copilot tagging support
  - phase: 20-opencode-event-capture
    provides: memory-ingest binary with agent field and CchEvent parsing
provides:
  - Shell hook handler that synthesizes session IDs and transforms Copilot JSON to memory-ingest format
  - memory-hooks.json configuration for 5 Copilot CLI lifecycle events
  - Copilot-specific session ID synthesis via CWD-keyed temp files
  - Bug #991 workaround (sessionStart per-prompt reuse logic)
affects: [22-copilot-cli-adapter, 23-cross-agent-discovery]

# Tech tracking
tech-stack:
  added: []
  patterns: [session-id-synthesis-via-temp-files, event-type-as-argument, millisecond-timestamp-conversion, toolargs-double-parse]

key-files:
  created:
    - plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh
    - plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json
  modified: []

key-decisions:
  - "Single script with event type as $1 argument (matching Gemini adapter pattern, less code duplication)"
  - "Runtime jq walk() capability test (same approach as Gemini adapter, more portable than version parsing)"
  - "Perl preferred for ANSI stripping with sed fallback (CSI+OSC+SS2/SS3 coverage)"
  - "del()-based fallback redaction for jq < 1.6 (top level + one level deep)"
  - "Session file cleanup only on user_exit or complete reasons (preserves resumed sessions)"
  - "No stdout output from hook script (Copilot ignores stdout for most events)"

patterns-established:
  - "Session ID synthesis: uuidgen with /proc/sys/kernel/random/uuid and date+pid fallbacks"
  - "CWD hashing: md5sum with md5 (macOS) fallback for temp file keying"
  - "Timestamp conversion: date -r (macOS) then date -d (Linux) then current time fallback"

# Metrics
duration: 2min
completed: 2026-02-10
---

# Phase 22 Plan 01: Copilot CLI Event Capture Summary

**Shell hook handler with session ID synthesis and memory-hooks.json for 5 Copilot CLI lifecycle events using fail-open pattern and agent:copilot tagging**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-10T18:20:26Z
- **Completed:** 2026-02-10T18:23:10Z
- **Tasks:** 2
- **Files created:** 2

## Accomplishments
- Created memory-capture.sh (238 lines) that synthesizes session IDs, converts millisecond timestamps, double-parses toolArgs, and handles Bug #991 per-prompt sessionStart firing
- Created memory-hooks.json with all 5 event registrations in Copilot CLI hook format (version: 1)
- Fail-open pattern with trap ERR EXIT, backgrounded memory-ingest, and zero stdout output
- Cross-platform compatibility: macOS/Linux for md5, date, uuidgen; perl/sed for ANSI stripping; jq walk()/del() for redaction

## Task Commits

Each task was committed atomically:

1. **Task 1: Create memory-capture.sh hook handler script with session ID synthesis** - `3797769` (feat)
2. **Task 2: Create memory-hooks.json hook configuration** - `c10f35a` (feat)

## Files Created/Modified
- `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` - Shell hook handler (238 lines): receives event type as $1, reads JSON from stdin, synthesizes session ID via temp files, converts timestamps, builds memory-ingest payloads with agent:copilot
- `plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json` - Copilot hook config (45 lines): registers sessionStart, sessionEnd, userPromptSubmitted, preToolUse, postToolUse with 10s timeouts

## Decisions Made
- **Single script with $1 argument:** Matches Gemini adapter pattern; event type passed as argument rather than separate scripts per event. Less code duplication, easier maintenance.
- **Runtime jq walk() test:** Same approach as Gemini adapter (Phase 21 decision). Tests `jq -n 'walk(.)'` rather than parsing version strings.
- **Perl preferred for ANSI stripping:** Covers CSI, OSC, and SS2/SS3 sequences. sed fallback for minimal systems handles CSI and basic OSC only.
- **del()-based redaction fallback:** For jq < 1.6, deletes known sensitive keys at top level and one level deep. Matches Gemini adapter approach.
- **Session cleanup on terminal reasons only:** Only removes session temp file when sessionEnd reason is "user_exit" or "complete". Preserves session ID for resumed/continued sessions (Bug #991 workaround).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Hook infrastructure complete, ready for Plan 02 (skills and agent definitions) and Plan 03 (install skill and README)
- memory-capture.sh and memory-hooks.json are the foundation that the install skill will deploy to target projects

## Self-Check: PASSED

- FOUND: plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh
- FOUND: plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json
- FOUND: .planning/phases/22-copilot-cli-adapter/22-01-SUMMARY.md
- FOUND: commit 3797769 (Task 1)
- FOUND: commit c10f35a (Task 2)

---
*Phase: 22-copilot-cli-adapter*
*Completed: 2026-02-10*
