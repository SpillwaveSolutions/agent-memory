# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-30)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** All phases complete - v1 ready

## Current Position

Phase: 8 of 8 (CCH Hook Integration) - COMPLETE
Plan: 1 of 1 in current phase (completed: 08-01)
Status: Phase 8 complete - CCH integration ready
Last activity: 2026-01-31 -- Completed 08-01-PLAN.md (CCH Hook Handler Binary)

Progress: [####################] 100% (19/19 plans)

## Performance Metrics

**Velocity:**
- Total plans completed: 19
- Average duration: ~10min
- Total execution time: ~173min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Foundation | 5/5 | 47min | 9min |
| 2. TOC Building | 3/3 | ~48min | ~16min |
| 3. Grips & Provenance | 3/3 | ~30min | ~10min |
| 4. Query Layer | 2/2 | ~20min | ~10min |
| 5. Integration | 3/3 | ~30min | ~10min |
| 6. End-to-End Demo | 2/2 | ~20min | ~10min |
| 8. CCH Integration | 1/1 | ~4min | ~4min |

**Recent Trend:**
- Last 5 plans: 05-02 (~10min), 05-03 (~10min), 06-01 (~10min), 06-02 (~10min), 08-01 (~4min)
- Trend: Consistent velocity with well-defined plans

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- TOC as primary navigation (agentic search beats brute-force)
- Append-only storage (immutable truth, no deletion complexity)
- gRPC only (no HTTP server)
- Per-project stores first (simpler mental model)

**From 01-00:**
- Workspace resolver=2 for modern Cargo features
- Dependencies defined in workspace.dependencies for DRY
- Proto compilation deferred to Phase 1 Plan 03
- Layer separation: types -> storage -> service -> daemon

**From 01-01:**
- Key format: {prefix}:{timestamp_ms:013}:{ulid} for time-range scans
- 6 column families: events, toc_nodes, toc_latest, grips, outbox, checkpoints
- Atomic batch writes for event + outbox entries
- ULID event_id with embedded timestamp for reconstruction

**From 01-02:**
- All domain types implement Serialize/Deserialize
- Timestamps stored as milliseconds (chrono::serde::ts_milliseconds)
- Config env vars prefixed with MEMORY_
- Builder pattern with with_* methods for optional fields

**From 01-03:**
- Proto enums use EVENT_ROLE_ and EVENT_TYPE_ prefixes for protobuf compatibility
- Graceful shutdown via run_server_with_shutdown for daemon use
- Health reporter marks MemoryService as serving for monitoring
- Proto-to-domain conversion via separate convert_* methods
- Service holds Arc<Storage> for thread-safe access

**From 01-04:**
- PID file location via directories::BaseDirs::runtime_dir() with fallback
- Process checking via libc::kill(pid, 0) on Unix
- Background daemonization deferred; use process managers (systemd, launchd)
- CLI structure: global flags -> subcommand -> subcommand options

**From 02-01:**
- tiktoken-rs for accurate token counting (OpenAI cl100k_base encoding)
- Time-gap boundary: 30 min default
- Token-threshold boundary: 4000 tokens default
- Overlap: 5 min or 500 tokens for context continuity

**From 02-02:**
- Summarizer trait is async and Send + Sync for concurrent use
- ApiSummarizer supports both OpenAI and Anthropic APIs
- MockSummarizer generates deterministic summaries for testing
- JSON response parsing handles markdown code blocks

**From 02-03:**
- TOC node IDs encode level and time: "toc:{level}:{time_identifier}"
- Versioned storage: new versions appended, not mutated (TOC-06)
- Parent nodes created automatically up to Year level
- Rollup jobs use configurable min_age to avoid incomplete periods
- Checkpoints stored per job name for crash recovery

**From 03-01:**
- Grip ID format: "grip:{timestamp_ms}:{ulid}" for time-ordered iteration
- Grips stored in CF_GRIPS with node index: "node:{node_id}:{grip_id}"
- Grip validation ensures 26-char ULID with alphanumeric characters

**From 03-02:**
- GripExtractor uses term-overlap scoring (>30% match threshold)
- Grips linked to bullets via grip_ids Vec<String> field
- Excerpt truncation with configurable max length

**From 03-03:**
- Grip expansion retrieves context events around excerpt
- Configurable context window (events_before/after, time limits)
- Events partitioned into before/excerpt/after for structured access

**From 05-01:**
- memory-client crate provides client API for hook handlers
- HookEvent maps to Event with 1:1 type mapping (HOOK-03)
- MemoryClient::connect/ingest for gRPC communication

**From 05-02:**
- Query RPCs added to proto (GetTocRoot, GetNode, BrowseToc, GetEvents, ExpandGrip)
- Query CLI subcommand with root/node/browse/events/expand commands
- Pagination via continuation_token for large result sets

**From 05-03:**
- Admin CLI opens storage directly (not via gRPC) for local operations
- StorageStats provides event/node/grip counts and disk usage
- Compact triggers RocksDB compaction on all or specific CFs
- RebuildToc placeholder - full impl requires memory-toc integration

**From 08-01:**
- memory-ingest binary reads CCH JSON from stdin
- Fail-open behavior: always return {"continue":true}
- Reuse memory-client types (HookEvent, map_hook_event)
- Event types: SessionStart, UserPromptSubmit, PostToolUse, Stop, SubagentStart/Stop

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-01-31
Stopped at: Completed 08-01-PLAN.md (CCH Hook Handler Binary)
Resume file: None

## Phase 1 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 01-00 | 1 | Workspace scaffolding, docs/README.md | Complete |
| 01-01 | 2 | RocksDB storage layer | Complete |
| 01-02 | 2 | Domain types (Event, TocNode, Grip, Settings) | Complete |
| 01-03 | 3 | gRPC service + IngestEvent RPC | Complete |
| 01-04 | 4 | CLI daemon binary | Complete |

## Phase 2 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 02-01 | 1 | Segmentation engine (time/token boundaries) | Complete |
| 02-02 | 1 | Summarizer trait and implementation | Complete |
| 02-03 | 2 | TOC hierarchy builder with rollups | Complete |

## Phase 3 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 03-01 | 1 | Grip storage and data model | Complete |
| 03-02 | 1 | Summarizer grip extraction integration | Complete |
| 03-03 | 2 | Grip expansion/context retrieval | Complete |

## Phase 4 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 04-01 | 1 | TOC navigation RPCs (GetTocRoot, GetNode, BrowseToc) | Complete |
| 04-02 | 2 | Event retrieval RPCs (GetEvents, ExpandGrip) | Complete |

## Phase 5 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 05-01 | 1 | Client library and hook mapping | Complete |
| 05-02 | 1 | Query CLI commands | Complete |
| 05-03 | 2 | Admin commands (rebuild-toc, compact, status) | Complete |

## Phase 6 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 06-01 | 1 | Integration test harness and demo script | Complete |
| 06-02 | 2 | Documentation and usage examples | Complete |

## Phase 8 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 08-01 | 1 | CCH hook handler binary | Complete |
