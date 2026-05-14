---
phase: 52-simple-cli-api
plan: 02
subsystem: cli
tags: [grpc, search, context, recall, json-envelope, route-query]

requires:
  - phase: 52-simple-cli-api-01
    provides: "CLI skeleton with GlobalArgs, SearchArgs, ContextArgs, RecallArgs, JsonEnvelope, connect_client"
provides:
  - "search command mapping RouteQueryResponse to JsonEnvelope with results array"
  - "context command returning structured MemoryContext-shaped JSON"
  - "recall command as named alias for search --rerank=llm --top=10"
  - "Re-exported RetrievalResult, ExplainabilityPayload, ProtoEvent from memory-client"
affects: [52-simple-cli-api-03, benchmark-suite]

tech-stack:
  added: []
  patterns: ["RouteQuery -> JsonEnvelope mapping", "Named alias delegation (recall -> search)"]

key-files:
  created: []
  modified:
    - crates/memory-cli/src/commands/search.rs
    - crates/memory-cli/src/commands/recall.rs
    - crates/memory-cli/src/commands/context.rs
    - crates/memory-cli/src/commands/timeline.rs
    - crates/memory-client/src/lib.rs

key-decisions:
  - "RetrievalLayer mapped by i32 value per proto enum (topics=1, hybrid=2, vector=3, bm25=4, agentic=5)"
  - "Context key_entities extracted as doc_id+doc_type pairs (simple heuristic)"
  - "Recall rerank flag is informational-only until daemon supports rerank mode in RPC"

patterns-established:
  - "Proto enum to string mapping via match on i32 values"
  - "build_results_json / build_meta as public helpers reused by context command"

requirements-completed: [CLI-02, CLI-03, CLI-06, CLI-08]

duration: 3min
completed: 2026-03-22
---

# Phase 52 Plan 02: Read-Path Commands Summary

**Search, context, and recall commands mapping RouteQuery gRPC response to JsonEnvelope with meta (retrieval_ms, tokens_estimated, confidence)**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-22T05:11:23Z
- **Completed:** 2026-03-22T05:14:44Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Search command queries daemon via RouteQuery RPC, maps results to JSON with source_layer, doc_id, score, metadata
- Context command builds structured MemoryContext-shaped JSON with summary, relevant_events, key_entities, open_questions
- Recall command delegates to search with rerank=llm, top=10 as named alias
- 11 unit tests covering result mapping, meta extraction, layer string conversion, empty results, context shape

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement memory search command via RouteQuery RPC** - `1206a17` (feat)
2. **Task 2: Implement memory recall and memory context commands** - `b15b82b` (feat)

## Files Created/Modified
- `crates/memory-cli/src/commands/search.rs` - Search command with RouteQuery RPC, result mapping, meta extraction
- `crates/memory-cli/src/commands/recall.rs` - Recall command delegating to search with llm rerank
- `crates/memory-cli/src/commands/context.rs` - Context command building structured MemoryContext JSON
- `crates/memory-cli/src/commands/timeline.rs` - Fixed import to use memory_client re-export
- `crates/memory-client/src/lib.rs` - Re-exported RetrievalResult, ExplainabilityPayload, ProtoEvent

## Decisions Made
- RetrievalLayer enum values mapped to strings per proto definition (not per plan's original Bm25=1 numbering which was incorrect)
- Context key_entities uses doc_id + doc_type pairs as simple entity references
- build_results_json and build_meta made public for reuse by context command

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed timeline.rs import of memory_service::pb::Event**
- **Found during:** Task 2 (compilation blocked)
- **Issue:** timeline.rs imported `memory_service::pb::Event` directly but memory-cli doesn't depend on memory-service
- **Fix:** Re-exported `Event as ProtoEvent` from memory-client; updated timeline.rs import
- **Files modified:** crates/memory-client/src/lib.rs, crates/memory-cli/src/commands/timeline.rs
- **Verification:** cargo test and clippy pass
- **Committed in:** b15b82b (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Import fix was necessary for compilation. No scope creep.

## Issues Encountered
- Proto RetrievalLayer enum values differ from plan's description (plan said Bm25=1, Vector=2; proto has Topics=1, Hybrid=2, Vector=3, Bm25=4, Agentic=5). Used actual proto values.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Read-path commands (search, context, recall) complete and tested
- Write-path commands (add, timeline, summary) ready for Plan 03
- All meta fields populated: retrieval_ms, tokens_estimated, confidence

---
*Phase: 52-simple-cli-api*
*Completed: 2026-03-22*
