---
phase: 20-opencode-event-capture
plan: 02
subsystem: plugins
tags: [typescript, opencode, event-capture, plugin, session-lifecycle, fail-open]

# Dependency graph
requires:
  - phase: 20-01
    provides: "CchEvent.agent field, HookEvent.agent propagation, memory-ingest agent support"
  - phase: 19-opencode-commands
    provides: "OpenCode plugin directory structure, commands, skills, agent"
provides:
  - "TypeScript plugin capturing OpenCode session lifecycle events into agent-memory"
  - "Four event hooks: session.created, session.idle, message.updated, tool.execute.after"
  - "Automatic agent:opencode tagging on all captured events"
  - "Project directory context included in every event"
affects: [20-03, 21-gemini-adapter, 22-copilot-adapter, 23-cross-agent-discovery]

# Tech tracking
tech-stack:
  added: ["@opencode-ai/plugin (type import only)"]
  patterns: ["Fail-open event capture via try/catch with silent error swallowing", "Defensive session ID extraction across polymorphic event shapes"]

key-files:
  created:
    - "plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts"
  modified: []

key-decisions:
  - "Used Bun $ shell API to pipe JSON to memory-ingest binary (no gRPC dependency in TypeScript)"
  - "Hardcoded agent:opencode in payload since plugin IS OpenCode (no detection needed)"
  - "Mapped session.idle to Stop hook event for R1.4.1/R1.4.2 (session end and checkpoint)"
  - "Defensive extractSessionId() checks multiple field names per research Pitfall 3"

patterns-established:
  - "OpenCode plugin event capture: TypeScript plugin -> JSON -> memory-ingest binary -> gRPC"
  - "Fail-open pattern: try/catch with empty catch block, .quiet() on shell calls"

# Metrics
duration: 1min
completed: 2026-02-09
---

# Phase 20 Plan 02: OpenCode Event Capture Plugin Summary

**TypeScript OpenCode plugin capturing session lifecycle events (start, idle, messages, tool use) via memory-ingest binary with agent:opencode tagging and fail-open pattern**

## Performance

- **Duration:** 1 min
- **Started:** 2026-02-09T22:05:11Z
- **Completed:** 2026-02-09T22:06:39Z
- **Tasks:** 1
- **Files created:** 1

## Accomplishments
- Created OpenCode event capture plugin with four lifecycle hooks covering session start, session end/checkpoint, message capture, and tool execution
- All events tagged with `agent: "opencode"` and project directory (`cwd: directory`) for R1.4.3 and R1.4.4
- Fail-open pattern ensures OpenCode is never blocked by memory-ingest failures
- Defensive session ID extraction handles polymorphic event input shapes (id, sessionID, session_id, properties.sessionID)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create OpenCode event capture plugin** - `cb828eb` (feat)

## Files Created/Modified
- `plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` - TypeScript plugin with four lifecycle event handlers for capturing OpenCode sessions into agent-memory

## Decisions Made
- Used Bun `$` shell API to pipe JSON to memory-ingest binary rather than compiling proto for TypeScript gRPC -- simpler, proven, no build pipeline needed
- Hardcoded `agent: "opencode"` in the event payload since the plugin IS OpenCode by definition (no environment detection needed)
- Mapped `session.idle` event to `Stop` hook event name, fulfilling both R1.4.1 (session end capture) and R1.4.2 (checkpoint capture)
- Created `extractSessionId()` helper that checks multiple field names (`id`, `sessionID`, `session_id`, `properties.sessionID`) to handle different OpenCode event shapes per research Pitfall 3
- Used `JSON.stringify()` as fallback for non-string message content (handles both plain text and content block arrays)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. The plugin auto-activates when placed in the `.opencode/plugin/` directory. Requires `memory-ingest` binary in PATH (can be overridden with `MEMORY_INGEST_PATH` environment variable).

## Next Phase Readiness
- Event capture plugin complete, ready for Plan 03 (unified query CLI enhancements)
- The full OpenCode integration is now wired: commands/skills/agent (Phase 19), agent pipeline (Plan 01), event capture (this plan)
- Gemini/Copilot adapters (Phases 21, 22) can follow the same pattern: capture events, pipe JSON to memory-ingest

## Self-Check: PASSED

- [x] plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts exists
- [x] Commit cb828eb (Task 1) verified
- [x] .planning/phases/20-opencode-event-capture/20-02-SUMMARY.md exists

---
*Phase: 20-opencode-event-capture*
*Completed: 2026-02-09*
