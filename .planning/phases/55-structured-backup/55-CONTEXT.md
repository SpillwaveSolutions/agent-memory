# Phase 55: Structured Backup - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-03-23-memory-export-import-design.md)

<domain>
## Phase Boundary

This phase adds `memory backup` CLI command producing a full JSONL directory structure with incremental support. Also introduces the project's first gRPC streaming RPCs: `ExportBackup` (server-side streaming) to deliver JSONL chunks without buffering the entire dataset in memory. This is the "true backup" — machine-parseable, round-trippable, suitable for git.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- New `backup` subcommand added to existing `memory-cli` crate
- `ExportBackup` is a **server-side streaming RPC** — first streaming RPC in the project
- Tonic streaming support must be wired into the server framework (new infrastructure)
- CLI writes files locally from the streamed data

### Directory Structure
```
memory-backup/
├── manifest.json          # version, export date, layer counts, time range
├── events/                # base layer — one JSONL per day
│   └── YYYY-MM-DD.jsonl
├── toc/                   # derived layers
│   ├── segments.jsonl
│   ├── days.jsonl
│   ├── weeks.jsonl
│   ├── months.jsonl
│   └── years.jsonl
├── grips.jsonl
└── episodes.jsonl
```

### JSONL Format
- Events: one JSON object per line with all Event fields (event_id, session_id, timestamp_ms, event_type, role, text, metadata, agent)
- TOC nodes: one JSON object per line with all TocNode fields (node_id, level, title, summary, bullets, keywords, etc.)
- Grips: one JSON object per line
- Episodes: one JSON object per line

### CLI Flags
- `memory backup` — full backup (default dir: `./memory-backup/`)
- `memory backup --events-only` — base layer only
- `memory backup --since 24h` — incremental (time range)
- `memory backup --since 2026-03-22 --until 2026-03-23` — explicit range
- `memory backup --dir ./custom/` — custom output directory

### Incremental Behavior
- `--since` filters events by timestamp range
- Per-day event files are **overwritten** (not appended) to prevent duplicate JSONL lines
- TOC/grips/episodes files are fully rewritten on each incremental run (small relative to events)
- `manifest.json` records `incremental: true` with time range

### Streaming RPC Design
- `ExportBackup(BackupOptions)` returns `stream BackupChunk`
- Each `BackupChunk` has a type tag (events, toc_segments, toc_days, etc.) and JSONL payload
- CLI receives chunks and writes to appropriate files
- Tonic streaming: `type ExportBackupStream = Pin<Box<dyn Stream<Item = Result<BackupChunk, Status>> + Send>>`

### What's Backed Up
- Events (base layer, source of truth)
- TOC nodes (all 5 levels: segments, days, weeks, months, years)
- Grips (provenance links)
- Episodes (Phase 44 episodic memory)

### What's NOT Backed Up
- BM25/HNSW indexes (platform-specific, rebuilt from events)
- InFlightBuffer state (ephemeral)
- Topic graph data (derived, rebuildable)
- Scheduler checkpoints (rebuilt on daemon start)

### Claude's Discretion
- Exact proto message definition for `BackupChunk`
- Whether to use a single streaming RPC or separate RPCs per data type
- Chunk size for streaming (e.g., 100 records per chunk)
- Error handling for partial backup (e.g., stream interrupted mid-export)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec
- `docs/superpowers/specs/2026-03-23-memory-export-import-design.md` — Full design spec (backup section)

### Existing CLI
- `crates/memory-cli/src/cli.rs` — Add `Backup` variant to `Commands` enum
- `crates/memory-cli/src/client.rs` — Add streaming client method

### gRPC / Proto
- `proto/memory.proto` — Add `ExportBackup` streaming RPC, `BackupOptions`, `BackupChunk` messages
- `crates/memory-service/src/lib.rs` — Server framework (needs streaming support wired)
- `crates/memory-service/src/handlers/` — New handler for backup export

### Storage (read-only access for export)
- `crates/memory-storage/src/db.rs` — `get_events_in_range()`, iterate TOC nodes, grips
- `crates/memory-types/src/event.rs` — Event struct for serialization
- `crates/memory-types/src/toc.rs` — TocNode struct for serialization

### Episodes
- `proto/memory.proto` — Episode-related messages (StartEpisode, etc.)
- `crates/memory-service/src/handlers/` — Episode storage patterns

</canonical_refs>

<specifics>
## Specific Ideas

- This is the most infrastructure-heavy phase — first streaming RPCs require tonic wiring
- Can prototype with unary RPC first (collect all data, return at once) then upgrade to streaming
- manifest.json should include `agent_memory_version` for import compatibility checks
- Consider streaming in order: events first (biggest), then TOC levels, then grips, then episodes

</specifics>

<deferred>
## Deferred Ideas

- Config.toml `[backup]` section for default directory — nice to have, not required
- Compressed backup (gzip the JSONL files) — future optimization
- Parallel streaming (multiple RPC calls for different data types) — premature optimization

</deferred>

---

*Phase: 55-structured-backup*
*Context gathered: 2026-03-23 via PRD Express Path*
