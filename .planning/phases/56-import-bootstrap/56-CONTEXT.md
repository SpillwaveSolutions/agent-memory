# Phase 56: Import/Bootstrap - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-03-23-memory-export-import-design.md)

<domain>
## Phase Boundary

This phase adds `memory import` CLI command to restore RocksDB from a backup directory. Uses client-side gRPC streaming to send JSONL records to the daemon. Enables migration, portability, and disaster recovery. Includes round-trip validation test proving export → wipe → import → queries return same results.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- New `import` subcommand added to existing `memory-cli` crate
- `ImportBackup` is a **client-side streaming RPC** — CLI reads files and streams to daemon
- CLI does file reading; daemon does RocksDB writing
- Positional argument for backup directory: `memory import ./backup-dir/`

### Import Process
1. Read `manifest.json` — validate version compatibility
2. Import events first (base layer, chronological order)
3. Import TOC nodes (segments → days → weeks → months → years)
4. Import grips
5. Import episodes
6. Trigger outbox entries for events needing indexing
7. Report: events imported, nodes restored, time elapsed

### CLI Flags
- `memory import ./dir/` — full restore from backup directory
- `memory import ./dir/ --events-only` — events only, rebuild TOC after
- `memory import ./dir/ --dry-run` — show what would be imported without writing

### Safety
- **Idempotent**: events with existing IDs are skipped (dedup by event_id in RocksDB)
- **Additive only**: does NOT delete existing data
- `--dry-run` shows counts without writing
- If events-only, user must trigger TOC rebuild afterward (existing `rebuild-toc` admin command, or create one if it doesn't exist)

### Streaming RPC Design
- `ImportBackup(stream ImportChunk) returns ImportResult`
- Each `ImportChunk` has type tag + JSONL payload (same structure as BackupChunk from Phase 55)
- Daemon processes chunks in order, writing to appropriate column families
- `ImportResult` returns counts per layer, errors, and elapsed time

### Round-Trip Validation
- Export full backup → wipe RocksDB → import full backup → run queries → compare results
- This is an integration test, not a unit test
- Key queries to verify: GetTocRoot, GetEvents (range), hybrid_search, ExpandGrip

### What's Imported
- Events → CF_EVENTS (with outbox entries for re-indexing)
- TOC nodes → CF_TOC_NODES + CF_TOC_LATEST
- Grips → CF_GRIPS
- Episodes → CF_EPISODES

### What's NOT Imported
- BM25/HNSW indexes (rebuilt from events via outbox)
- InFlightBuffer state (ephemeral)
- Scheduler checkpoints (rebuilt on daemon start)
- Config (informational only in backup)

### Claude's Discretion
- Whether `rebuild-toc` command exists or needs to be created
- Exact error reporting for partial imports (e.g., 1000/1200 events imported, 200 skipped as duplicates)
- Whether to validate manifest version before streaming (fail fast vs best effort)
- Progress reporting during import (percentage, records/sec)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec
- `docs/superpowers/specs/2026-03-23-memory-export-import-design.md` — Full design spec (import section)

### Phase 55 Dependency (backup format)
- Phase 55 defines the backup directory structure, manifest.json format, and JSONL schemas
- `BackupChunk` / `ImportChunk` proto messages from Phase 55

### Existing CLI
- `crates/memory-cli/src/cli.rs` — Add `Import` variant to `Commands` enum
- `crates/memory-cli/src/client.rs` — Add client-streaming import method

### gRPC / Proto
- `proto/memory.proto` — Add `ImportBackup` client-streaming RPC, `ImportChunk`, `ImportResult` messages

### Storage (write access for import)
- `crates/memory-storage/src/db.rs` — `put_event()`, TOC node writes, grip writes
- `crates/memory-storage/src/db.rs` — Idempotent write pattern (check exists before insert)

### Existing Admin Commands
- `crates/memory-daemon/src/commands.rs` — Check if `rebuild-toc` exists; if not, create it

</canonical_refs>

<specifics>
## Specific Ideas

- Import is the inverse of backup — same data format, opposite direction
- Client-streaming: CLI reads files line by line, sends as `ImportChunk`, daemon writes
- Round-trip test should be an E2E test in `crates/e2e-tests/` or `crates/memory-cli/tests/`
- Progress bar for large imports (optional, nice UX)

</specifics>

<deferred>
## Deferred Ideas

- `rebuild-toc` as separate admin command if it doesn't exist (IMPORT-F01)
- Selective import (e.g., import only events from a specific date range)
- Import from remote URL (e.g., `memory import https://...`)

</deferred>

---

*Phase: 56-import-bootstrap*
*Context gathered: 2026-03-23 via PRD Express Path*
