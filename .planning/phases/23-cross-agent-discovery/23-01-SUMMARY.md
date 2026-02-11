---
phase: 23-cross-agent-discovery
plan: 01
subsystem: api
tags: [grpc, proto, chrono, toc, agent-discovery, cli]

# Dependency graph
requires:
  - phase: 18-agent-tagging-infrastructure
    provides: Event.agent field, TocNode.contributing_agents, agent_filter on queries
  - phase: 22-copilot-cli-adapter
    provides: Multi-agent ingestion pipeline (Claude, OpenCode, Gemini, Copilot)
provides:
  - ListAgents RPC aggregating agents from TocNode.contributing_agents
  - GetAgentActivity RPC with time-bounded event scans and chrono bucketing
  - AgentDiscoveryHandler service module in memory-service
  - CLI `agents list` and `agents activity` commands in memory-daemon
  - Agent Discovery section in docs/README.md
affects: [23-02, 23-03, documentation, milestone-completion]

# Tech tracking
tech-stack:
  added: []
  patterns: [TOC-based O(k) agent aggregation, chrono day/week bucketing, parse_time_arg dual format]

key-files:
  created:
    - crates/memory-service/src/agents.rs
  modified:
    - proto/memory.proto
    - crates/memory-service/src/lib.rs
    - crates/memory-service/src/ingest.rs
    - crates/memory-service/Cargo.toml
    - crates/memory-storage/src/db.rs
    - crates/memory-daemon/src/cli.rs
    - crates/memory-daemon/src/commands.rs
    - crates/memory-daemon/src/main.rs
    - crates/memory-daemon/src/lib.rs
    - docs/README.md

key-decisions:
  - "Approximate event_count from TOC node count (O(k)) instead of O(n) event scan"
  - "session_count = 0 since not available from TOC alone; exact counts deferred"
  - "Fixed get_toc_nodes_by_level versioned key prefix bug in storage (Rule 1 auto-fix)"
  - "parse_time_arg accepts both YYYY-MM-DD and epoch ms for CLI flexibility"

patterns-established:
  - "AgentDiscoveryHandler pattern: Arc<Storage> handler with gRPC request/response delegation"
  - "compute_bucket pattern: chrono day/week truncation for time-series bucketing"

# Metrics
duration: 7min
completed: 2026-02-10
---

# Phase 23 Plan 01: Agent Discovery RPCs Summary

**ListAgents and GetAgentActivity gRPC RPCs with O(k) TOC-based agent aggregation, chrono day/week bucketing, and CLI `agents list`/`agents activity` commands**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-10T15:43:50Z
- **Completed:** 2026-02-10T15:51:40Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments
- Added ListAgents and GetAgentActivity RPCs to proto/memory.proto with full message definitions
- Implemented AgentDiscoveryHandler with TOC-based agent aggregation (O(k) over TOC nodes)
- Added CLI `agents list` and `agents activity` commands with human-readable table output
- Fixed pre-existing bug in `get_toc_nodes_by_level` versioned key prefix
- 9 unit tests for agent discovery + 10 CLI/helper tests = 19 new tests total

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ListAgents and GetAgentActivity RPCs to proto and implement service logic** - `d94c2e2` (feat)
2. **Task 2: Add Agents CLI subcommand group to memory-daemon** - `a258f02` (feat)
3. **Task 3: Add tests for agent discovery and update documentation** - `7879610` (docs)

## Files Created/Modified
- `proto/memory.proto` - Added ListAgents, GetAgentActivity RPCs and 6 new message types
- `crates/memory-service/src/agents.rs` - NEW: AgentDiscoveryHandler with list_agents, get_agent_activity, compute_bucket + 9 tests
- `crates/memory-service/src/lib.rs` - Registered agents module and AgentDiscoveryHandler export
- `crates/memory-service/src/ingest.rs` - Wired agent_service into MemoryServiceImpl (all 8 constructors)
- `crates/memory-service/Cargo.toml` - Added serde_json dependency
- `crates/memory-storage/src/db.rs` - Fixed get_toc_nodes_by_level versioned key prefix
- `crates/memory-daemon/src/cli.rs` - AgentsCommand enum (List, Activity) + 5 parse tests
- `crates/memory-daemon/src/commands.rs` - handle_agents_command, agents_list, agents_activity, parse_time_arg + 5 tests
- `crates/memory-daemon/src/main.rs` - Wired Agents command dispatch
- `crates/memory-daemon/src/lib.rs` - Exported AgentsCommand and handle_agents_command
- `docs/README.md` - Agent Discovery section with CLI examples

## Decisions Made
- **Approximate counts via TOC:** event_count in AgentSummary counts TOC nodes an agent appears in (not actual events). session_count is 0. This gives O(k) performance where k is total TOC nodes (typically hundreds) instead of O(n) over all events.
- **Fixed storage bug:** get_toc_nodes_by_level had a key format mismatch (missing "toc:" prefix in versioned key lookup). Fixed as Rule 1 auto-fix since it blocked agent discovery functionality.
- **Dual time format:** parse_time_arg accepts both YYYY-MM-DD strings and Unix epoch milliseconds, providing CLI flexibility for human and scripted usage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed get_toc_nodes_by_level versioned key prefix**
- **Found during:** Task 1 (service implementation testing)
- **Issue:** get_toc_nodes_by_level used `format!("{}:v{:06}", node_id, version)` but put_toc_node stored with `format!("toc:{}:v{:06}", node.node_id, new_version)`. The retrieval key was missing the "toc:" prefix, so nodes were never found by level iteration.
- **Fix:** Changed get_toc_nodes_by_level to use `format!("toc:{}:v{:06}", node_id, version)` matching the storage format.
- **Files modified:** crates/memory-storage/src/db.rs
- **Verification:** 9 agent tests pass, including test_list_agents_aggregates_from_toc_nodes
- **Committed in:** d94c2e2 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed ULID-based event IDs in tests**
- **Found during:** Task 1 (test failures)
- **Issue:** Tests used string event IDs like "evt-1" but storage.put_event requires valid ULID strings (parsed by EventKey::from_event_id)
- **Fix:** Updated create_test_event helper to generate ULID-based event IDs using ulid::Ulid::from_parts(timestamp_ms, rand::random())
- **Files modified:** crates/memory-service/src/agents.rs (test module)
- **Verification:** All 9 agent tests pass
- **Committed in:** d94c2e2 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both auto-fixes necessary for correctness. The storage bug was pre-existing. No scope creep.

## Issues Encountered
- C++ compilation errors (esaxx-rs cstdint not found) on macOS required sourcing env.sh before cargo commands. This is an environment setup issue documented in the project.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Agent discovery RPCs are live and wired into the gRPC server
- CLI commands are ready for use with a running daemon
- Ready for Plan 02 (documentation) and Plan 03 (remaining cross-agent features)
- Full workspace clippy and tests pass

---
*Phase: 23-cross-agent-discovery*
*Completed: 2026-02-10*
