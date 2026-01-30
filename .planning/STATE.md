# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-30)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** Phase 2 Complete - Ready for Phase 3

## Current Position

Phase: 2 of 6 (TOC Building) - COMPLETE
Plan: 3 of 3 in current phase (completed: 02-01, 02-02, 02-03)
Status: Phase 2 Complete
Last activity: 2026-01-30 -- Completed 02-03-PLAN.md (TOC Hierarchy Builder)

Progress: [########----------] 44% (8/18 plans)

## Performance Metrics

**Velocity:**
- Total plans completed: 8
- Average duration: ~12min
- Total execution time: ~95min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Foundation | 5/5 | 47min | 9min |
| 2. TOC Building | 3/3 | ~48min | ~16min |
| 3. Grips & Provenance | 0/3 | - | - |
| 4. Query Layer | 0/2 | - | - |
| 5. Integration | 0/3 | - | - |
| 6. End-to-End Demo | 0/2 | - | - |

**Recent Trend:**
- Last 5 plans: 01-03 (12min), 01-04 (4min), 02-01 (~15min), 02-02 (~15min), 02-03 (~18min)
- Trend: Stable with slight increase for more complex plans

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

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-01-30
Stopped at: Completed 02-03-PLAN.md (Phase 2 Complete)
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
