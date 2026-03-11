---
phase: 23-cross-agent-discovery
plan: 02
subsystem: api
tags: [grpc, proto, topics, agent-filter, cli, cross-agent]

# Dependency graph
requires:
  - phase: 23-cross-agent-discovery
    plan: 01
    provides: ListAgents/GetAgentActivity RPCs, AgentsCommand CLI enum, agent discovery handler
  - phase: 14-topic-graph
    provides: TopicStorage, TopicLink, Topic types, GetTopTopics RPC
  - phase: 18-agent-tagging-infrastructure
    provides: TocNode.contributing_agents field
provides:
  - Agent-filtered GetTopTopics RPC via optional agent_filter field (201)
  - get_topics_for_agent() aggregation using TopicLink -> TocNode -> contributing_agents
  - CLI `agents topics --agent <id>` command with formatted output
  - get_top_topics_for_agent() in memory-client
affects: [23-03, documentation, cross-agent-queries]

# Tech tracking
tech-stack:
  added: []
  patterns: [TopicLink -> TocNode -> contributing_agents indirect agent-topic linking]

key-files:
  created: []
  modified:
    - proto/memory.proto
    - crates/memory-topics/src/storage.rs
    - crates/memory-service/src/topics.rs
    - crates/memory-client/src/client.rs
    - crates/memory-daemon/src/cli.rs
    - crates/memory-daemon/src/commands.rs

key-decisions:
  - "TopicGraphHandler now accepts main_storage for TocNode lookups (required for agent-topic linking)"
  - "agent_filter field uses proto field number 201 (>200 per Phase 23 convention to avoid conflicts)"
  - "Agent-topic aggregation uses indirect path through TopicLink -> TocNode -> contributing_agents (no new storage structures needed)"
  - "Combined score = importance_score * max_relevance for agent-filtered topic ranking"

patterns-established:
  - "Indirect agent-topic linking: Topic -> TopicLink -> TocNode -> contributing_agents (no denormalization)"
  - "Proto optional field pattern with empty-string check for backward compatibility"

# Metrics
duration: 18min
completed: 2026-02-10
---

# Phase 23 Plan 02: Agent-Aware Topic Queries Summary

**Agent-filtered GetTopTopics RPC with TopicLink -> TocNode -> contributing_agents aggregation and `agents topics --agent <id>` CLI command**

## Performance

- **Duration:** 18 min
- **Started:** 2026-02-10T22:54:15Z
- **Completed:** 2026-02-10T23:12:15Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- Added optional agent_filter field to GetTopTopicsRequest proto (field 201, backward compatible)
- Implemented get_topics_for_agent() aggregation in TopicStorage using the indirect TopicLink -> TocNode -> contributing_agents path
- Added `agents topics --agent <id> [--limit N]` CLI command with formatted table output
- Added get_top_topics_for_agent() to memory-client for agent-filtered queries
- 8 new tests (5 unit + 3 integration) covering matching, no-match, limit, sort order, case-insensitivity, and backward compatibility

## Task Commits

Each task was committed atomically:

1. **Task 1: Add agent filter to GetTopTopics RPC and implement topic-agent aggregation** - `762c8f3` (feat)
2. **Task 2: Add `agents topics` CLI command** - `7152940` (feat)
3. **Task 3: Add tests for agent-filtered topic queries** - `c319542` (test)

## Files Created/Modified
- `proto/memory.proto` - Added optional agent_filter field (201) to GetTopTopicsRequest
- `crates/memory-topics/src/storage.rs` - Added get_topics_for_agent() with indirect agent-topic linking + 5 unit tests
- `crates/memory-service/src/topics.rs` - Updated TopicGraphHandler to accept main_storage, wired agent filter in get_top_topics handler + 3 integration tests
- `crates/memory-client/src/client.rs` - Added get_top_topics_for_agent() method, updated existing get_top_topics with agent_filter: None
- `crates/memory-daemon/src/cli.rs` - Added Topics variant to AgentsCommand enum + 2 CLI parse tests
- `crates/memory-daemon/src/commands.rs` - Implemented agents_topics() with formatted output and empty-result handling

## Decisions Made
- **TopicGraphHandler constructor change:** Added main_storage parameter to TopicGraphHandler::new() for TocNode lookups. This is needed because the agent-topic linking path requires reading TocNode.contributing_agents from the main storage. No existing code calls TopicGraphHandler::new() directly (always passed in as Arc parameter), so this is a non-breaking change.
- **Proto field numbering:** Used field number 201 (>200 convention from Phase 23 research) to avoid potential conflicts with future base fields.
- **Indirect linking approach:** Used Topic -> TopicLink -> TocNode -> contributing_agents path rather than adding a denormalized agent-to-topic index. This keeps the storage schema simple and leverages existing data structures. Performance is bounded by topic count (typically < 1000), not event count.
- **Combined score ranking:** Agent-filtered topics are ranked by importance_score * max_relevance, giving weight to both overall topic importance and how relevant the agent's contributions are.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Agent-filtered topic queries are fully implemented and tested
- CLI `agents topics --agent <id>` is ready for use with a running daemon
- Ready for Plan 03 (remaining cross-agent documentation)
- Full workspace test, clippy, and doc pass

## Self-Check: PASSED

All files verified present. All 4 commits verified (762c8f3, 7152940, c319542, 491edcf).

---
*Phase: 23-cross-agent-discovery*
*Completed: 2026-02-10*
