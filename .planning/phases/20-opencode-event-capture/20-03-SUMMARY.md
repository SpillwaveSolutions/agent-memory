---
phase: 20-opencode-event-capture
plan: 03
subsystem: cli, docs
tags: [cli, agent-display, retrieval, opencode, event-capture, documentation]

# Dependency graph
requires:
  - phase: 18-agent-tagging
    provides: "RetrievalResult.agent field in proto, --agent CLI flag definition"
  - phase: 20-01
    provides: "Agent pipeline wiring (ingest, retrieval, gRPC)"
provides:
  - "CLI retrieval route output displays agent source for each result"
  - "--agent filter wired from CLI to gRPC RouteQueryRequest"
  - "Event capture documentation in OpenCode plugin README"
affects: [23-cross-agent-discovery, documentation]

# Tech tracking
tech-stack:
  added: []
  patterns: ["conditional agent display in CLI output", "CLI flag passthrough to gRPC request"]

key-files:
  created: []
  modified:
    - "crates/memory-daemon/src/commands.rs"
    - "plugins/memory-opencode-plugin/README.md"
    - "plugins/memory-opencode-plugin/.gitignore"

key-decisions:
  - "Only display agent for RetrievalResult (proto has agent field); skip TeleportResult/VectorTeleportMatch/HybridSearchResult (no agent field in proto)"
  - "Agent line shown conditionally via if-let to preserve backward compatibility"

patterns-established:
  - "Conditional metadata display: use if-let for optional proto fields in CLI output"

# Metrics
duration: 9min
completed: 2026-02-09
---

# Phase 20 Plan 03: Agent Display and Plugin README Summary

**CLI retrieval route wired with --agent filter passthrough and agent display in output, plus event capture documentation in OpenCode plugin README**

## Performance

- **Duration:** 9 min
- **Started:** 2026-02-09T22:05:13Z
- **Completed:** 2026-02-09T22:15:08Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Wired `--agent` CLI flag through to `RouteQueryRequest.agent_filter` in the gRPC call
- Added conditional `Agent: <name>` display in CLI retrieval route output for results with agent metadata
- Documented event capture system in OpenCode plugin README with hooks table, prerequisites, fail-open behavior, configuration, and verification steps
- Updated .gitignore with `node_modules/` and compiled JS exclusions

## Task Commits

Each task was committed atomically:

1. **Task 1: Add agent display to CLI query output formatters** - `4d4e5d0` (feat)
2. **Task 2: Update plugin README with event capture documentation** - `eaa7b72` (docs)

## Files Created/Modified
- `crates/memory-daemon/src/commands.rs` - Wired --agent passthrough + agent display in retrieval results
- `plugins/memory-opencode-plugin/README.md` - Added Event Capture section with hooks, behavior, config docs
- `plugins/memory-opencode-plugin/.gitignore` - Added node_modules/ and *.js exclusions

## Decisions Made
- Only added agent display for `RetrievalResult` which has `optional string agent = 7` in proto. TeleportResult, VectorTeleportMatch, and HybridSearchResult do not have agent fields -- agent display for those types requires future index metadata propagation.
- Used `if let Some(ref agent) = result.agent` for conditional display, preserving backward compatibility when agent is absent.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- C++ toolchain issue on macOS prevented `cargo build` and required `CPATH` workaround for SDK headers. This is an environment-level issue, not a code issue. `cargo check`, `cargo clippy`, and `cargo test` all pass with the workaround.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 20 is now complete (3/3 plans executed)
- CLI displays agent metadata and wires agent filter for cross-agent queries
- Plugin README documents event capture for OpenCode users
- Ready for Phase 21 (Gemini CLI Adapter) or Phase 23 (Cross-Agent Discovery)

## Self-Check: PASSED

- FOUND: crates/memory-daemon/src/commands.rs
- FOUND: plugins/memory-opencode-plugin/README.md
- FOUND: plugins/memory-opencode-plugin/.gitignore
- FOUND: .planning/phases/20-opencode-event-capture/20-03-SUMMARY.md
- FOUND: commit 4d4e5d0 (Task 1)
- FOUND: commit eaa7b72 (Task 2)

---
*Phase: 20-opencode-event-capture*
*Completed: 2026-02-09*
