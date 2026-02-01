# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-30)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v1.0.0 SHIPPED - v2.0 planned (Phase 10 Scheduler + Phases 11-13 Teleport)

## Current Position

Milestone: v2.0 Scheduler+Teleport (in progress)
Current: Phase 10 - Background Scheduler
Status: Phase complete
Last activity: 2026-01-31 -- Completed 10-04-PLAN.md

Progress Phase 10: [####################] 100% (4/4 plans)

## Performance Metrics

**Velocity:**
- Total plans completed: 26
- Average duration: ~10min
- Total execution time: ~249min

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
| 9. Setup Plugin | 4/4 | ~19min | ~5min |
| 10. Background Scheduler | 4/4 | ~61min | ~15min |

**Recent Trend:**
- Last 5 plans: 10-01 (~8min), 10-02 (~10min), 10-03 (~19min), 10-04 (~24min)
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

**From 09-01:**
- Plugin structure follows memory-query-plugin pattern
- Progressive Disclosure Architecture for SKILL.md
- Three slash commands: /memory-setup, /memory-status, /memory-config
- setup-troubleshooter agent with safe/permission-required fix tiers

**From 09-02:**
- 6-step progressive wizard with conditional skip logic
- State detection before asking questions to skip completed steps
- Three flag modes: --fresh (reset), --minimal (defaults), --advanced (full options)
- Consistent output formatting with [check]/[x] status indicators
- Backup-before-overwrite pattern for --fresh flag

**From 09-03:**
- Install helper script as sourced functions for flexibility
- User-level services (launchd/systemd/Task Scheduler) not system services
- SHA256 checksum verification for binary downloads
- Platform detection via detect_os/detect_arch/detect_platform functions

**From 09-04:**
- 7 health checks in sequence: binary, daemon, port, gRPC, database, events, CCH
- 4 status levels: healthy, degraded, unhealthy, not installed
- Config validation with restart-required matrix for side effect handling
- Troubleshooter uses 6 diagnostic categories: INSTALLATION, STARTUP, CONNECTION, INGESTION, SUMMARIZATION, RUNTIME
- Safe auto-fixes vs permission-required fixes tier system

**From 10-01:**
- SchedulerService wraps tokio-cron-scheduler's JobScheduler
- shutdown() requires &mut self due to underlying API
- Timezone validation at SchedulerService::new() for fail-fast
- Jobs receive CancellationToken for graceful shutdown integration
- validate_cron_expression() for upfront cron syntax checking

**From 10-02:**
- JobRegistry uses RwLock<HashMap> for thread-safe status tracking
- OverlapPolicy::Skip is the default - prevents job pileup
- OverlapGuard uses AtomicBool for lock-free running state
- RunGuard RAII pattern ensures running flag is released on drop/panic
- JitterConfig generates random delay in milliseconds
- register_job() checks is_paused before acquiring overlap guard

**From 10-03:**
- Jobs module in memory-scheduler with optional "jobs" feature (default on)
- RollupJobConfig configures day/week/month cron schedules
- create_rollup_jobs() wires existing memory-toc::rollup::RollupJob to scheduler
- CompactionJobConfig configures weekly RocksDB compaction
- run_server_with_scheduler() starts scheduler-aware gRPC server
- MemoryServiceImpl::with_scheduler() wires scheduler gRPC handlers
- MockSummarizer used by default; production should load ApiSummarizer from config

**From 10-04:**
- JobStatusProto uses Proto suffix to avoid name conflict with domain JobStatus
- Scheduler RPCs return success/error response rather than gRPC errors for pause/resume
- CLI uses gRPC client to query daemon rather than direct storage access
- Timestamps formatted as local time for human readability in CLI
- SchedulerGrpcService delegates from MemoryServiceImpl when scheduler is configured

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-01-31
Stopped at: Completed 10-04-PLAN.md (Job observability)
Resume file: None

## Milestone History

See: .planning/MILESTONES.md for complete history
- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans, 91 files, 9,135 LOC)

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

## Phase 9 Plans

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 09-01 | 1 | Setup Plugin Structure | Complete |
| 09-02 | 2 | Interactive Wizard Flow | Complete |
| 09-03 | 2 | Installation Automation | Complete |
| 09-04 | 3 | Health Check and Troubleshooting | Complete |

## Phase 10 Plans (v2.0 Scheduler)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 10-01 | 1 | Scheduler infrastructure (tokio-cron-scheduler, cron parsing, TZ) | Complete |
| 10-02 | 1 | Job registry and lifecycle (register, pause, overlap policy) | Complete |
| 10-03 | 2 | TOC rollup jobs (wire existing rollups to scheduler) | Complete |
| 10-04 | 3 | Job observability (status RPC, CLI, metrics) | Complete |

## Phase 11 Plans (v2.0 Teleport - BM25)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 11-01 | 1 | Tantivy integration (embedded index, schema design) | Planned |
| 11-02 | 1 | Indexing pipeline (TOC node and grip text extraction) | Planned |
| 11-03 | 2 | Search API (gRPC TeleportSearch RPC, scoring) | Planned |
| 11-04 | 3 | CLI and testing (teleport command, benchmark) | Planned |

## Phase 12 Plans (v2.0 Teleport - Vector)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 12-01 | 1 | HNSW index setup (usearch or hnsw-rs integration) | Planned |
| 12-02 | 1 | Local embedding model (sentence-transformers or candle) | Planned |
| 12-03 | 2 | Vector search API (gRPC VectorTeleport RPC) | Planned |
| 12-04 | 3 | Hybrid ranking (BM25 + vector fusion) | Planned |

## Phase 13 Plans (v2.0 Teleport - Outbox)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 13-01 | 1 | Outbox consumer for indexing (checkpoint tracking) | Planned |
| 13-02 | 1 | Incremental index updates (add/update documents) | Planned |
| 13-03 | 2 | Full rebuild command (admin rebuild-indexes) | Planned |
| 13-04 | 3 | Async indexing pipeline (scheduled via Phase 10) | Planned |
