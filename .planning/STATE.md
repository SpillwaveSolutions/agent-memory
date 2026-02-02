# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-30)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.0 in progress - Phases 10.5-14 COMPLETE - Phase 15 ready for execution

## Current Position

Milestone: v2.0 Scheduler+Teleport (in progress)
Current: Phase 15 - Configuration Wizard Skills (planning complete)
Status: Phases 10.5-14 complete, Phase 15 plans ready for execution
Last activity: 2026-02-02 -- Completed Phases 10.5, 11, 12, 13, and 14

Progress Phase 10.5: [====================] 100% (3/3 plans)
Progress Phase 11: [====================] 100% (4/4 plans)
Progress Phase 12: [====================] 100% (5/5 plans)
Progress Phase 13: [====================] 100% (4/4 plans)
Progress Phase 14: [====================] 100% (6/6 plans)
Progress Phase 15: [                    ] 0% (0/5 plans)

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

### Roadmap Evolution

- Phase 15 added: Configuration Wizard Skills (AskUserQuestion-based interactive config wizards for storage, LLM, multi-agent)

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-02-02
Stopped at: Completed Phases 10.5, 11, 12, 13, and 14 execution
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

## Phase 10.5 Plans (v2.0 Agentic TOC Search)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 10.5-01 | 1 | Core search algorithm (memory-toc/src/search.rs) | Complete |
| 10.5-02 | 2 | gRPC integration (SearchNode/SearchChildren RPCs) | Complete |
| 10.5-03 | 3 | CLI search command | Complete |

## Phase 11 Plans (v2.0 Teleport - BM25)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 11-01 | 1 | Tantivy integration (memory-search crate, schema, index) | Complete |
| 11-02 | 2 | Indexing pipeline (TOC node and grip document mapping) | Complete |
| 11-03 | 2 | Search API (gRPC TeleportSearch RPC, BM25 scoring) | Complete |
| 11-04 | 3 | CLI and testing (teleport command, commit job) | Complete |

## Phase 12 Plans (v2.0 Teleport - Vector)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 12-01 | 1 | Embedding infrastructure (memory-embeddings crate, Candle all-MiniLM-L6-v2) | Complete |
| 12-02 | 1 | Vector index (memory-vector crate, usearch HNSW) | Complete |
| 12-03 | 2 | Vector indexing pipeline | Complete |
| 12-04 | 3 | gRPC integration (VectorTeleport/HybridSearch RPCs) | Complete |
| 12-05 | 4 | CLI and documentation (vector teleport commands) | Complete |

## Phase 13 Plans (v2.0 Teleport - Outbox)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 13-01 | 1 | Outbox consumer for indexing (checkpoint tracking) | Complete |
| 13-02 | 1 | Incremental index updates (add/update documents) | Complete |
| 13-03 | 2 | Full rebuild command (admin rebuild-indexes) | Complete |
| 13-04 | 3 | Async indexing pipeline (scheduled via Phase 10) | Complete |

## Phase 14 Plans (Topic Graph Memory)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 14-01 | 1 | Topic extraction (memory-topics crate, HDBSCAN clustering) | Complete |
| 14-02 | 2 | Topic labeling (LLM integration with keyword fallback) | Complete |
| 14-03 | 3 | Importance scoring (time decay with configurable half-life) | Complete |
| 14-04 | 4 | Topic relationships (similarity, hierarchy discovery) | Complete |
| 14-05 | 5 | Navigation RPCs (topic gRPC endpoints) | Complete |
| 14-06 | 6 | Lifecycle management (pruning, resurrection, CLI) | Complete |

## Phase 15 Plans (Configuration Wizard Skills)

| Plan | Wave | Description | Status |
|------|------|-------------|--------|
| 15-01 | 1 | memory-storage skill (storage, retention, cleanup, GDPR) | Ready |
| 15-02 | 1 | memory-llm skill (provider, model discovery, cost, API test) | Ready |
| 15-03 | 2 | memory-agents skill (multi-agent, tagging, query scope) | Ready |
| 15-04 | 2 | Reference documentation (all reference/*.md files) | Ready |
| 15-05 | 3 | Plugin integration (marketplace.json, memory-setup updates) | Ready |
