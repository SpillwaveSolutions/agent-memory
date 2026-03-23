# Requirements: Agent Memory v3.1

**Defined:** 2026-03-23
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v3.1 Requirements

Requirements for the Memory Export/Import milestone. Each maps to roadmap phases.

### Daily Markdown Export (DAILY)

- [ ] **DAILY-01**: `memory daily` produces browsable markdown files (`memory/YYYY-MM-DD.md`) from TOC day nodes
- [ ] **DAILY-02**: Daily markdown includes session markers, summary bullets, keywords, and grip excerpts
- [ ] **DAILY-03**: `memory daily --range 7d` exports multiple days; handles days without rollup (partial output with pending note)
- [ ] **DAILY-04**: Skips days with no events (no empty files)
- [ ] **DAILY-05**: Footer includes "derived view" notice and export timestamp

### Structured Backup (BACKUP)

- [ ] **BACKUP-01**: `memory backup` exports all layers as JSONL directory structure with `manifest.json`
- [ ] **BACKUP-02**: `memory backup --events-only` exports just the base event layer
- [ ] **BACKUP-03**: `memory backup --since 24h` exports only recent data (incremental by time range)
- [ ] **BACKUP-04**: Incremental backups overwrite per-day event files (no duplicate JSONL lines)
- [ ] **BACKUP-05**: `manifest.json` includes version, counts, time range, and incremental flag
- [ ] **BACKUP-06**: Backup includes events, TOC nodes (all levels), grips, and episodes
- [ ] **BACKUP-07**: `ExportBackup` uses server-side gRPC streaming (first streaming RPC in the project)

### Import/Bootstrap (IMPORT)

- [ ] **IMPORT-01**: `memory import ./dir/` restores a full backup to RocksDB
- [ ] **IMPORT-02**: Round-trip test: export → wipe → import → all queries return same results
- [ ] **IMPORT-03**: `memory import --dry-run` shows what would be imported without writing
- [ ] **IMPORT-04**: Idempotent — events with existing IDs are skipped (dedup by event_id)
- [ ] **IMPORT-05**: `ImportBackup` uses client-side gRPC streaming
- [ ] **IMPORT-06**: Events-only import works; user triggers TOC rebuild after

### gRPC Infrastructure (GRPC)

- [ ] **GRPC-01**: `ExportDaily` unary RPC returns structured day data (CLI renders markdown)
- [ ] **GRPC-02**: `ExportBackup` server-side streaming RPC delivers JSONL chunks
- [ ] **GRPC-03**: `ImportBackup` client-side streaming RPC accepts JSONL chunks
- [ ] **GRPC-04**: Streaming support wired into tonic server framework (new infrastructure)

## Future Requirements (v3.2+)

- **DAILY-F01**: Automatic daemon scheduler integration (DailyExportJob after day rollup)
- **DAILY-F02**: Configurable export time and directory via `[daily_export]` in config.toml
- **IMPORT-F01**: `rebuild-toc` command if it doesn't exist at implementation time

## Out of Scope

| Feature | Reason |
|---------|--------|
| Live sync / file watching | Not bidirectional — files are derived views |
| Markdown import | Only JSONL backup imports; markdown dailies are read-only |
| Index file backup (BM25/HNSW) | Platform-specific; rebuild from events |
| Automatic cloud backup | Use cron + `memory backup --since 24h` + git push |
| Editing markdown dailies reflected in RocksDB | Files are read-only derived views |
| Automatic daemon scheduler for daily export | Deferred to v3.2; use cron for v3.1 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DAILY-01 | Phase 54 | Pending |
| DAILY-02 | Phase 54 | Pending |
| DAILY-03 | Phase 54 | Pending |
| DAILY-04 | Phase 54 | Pending |
| DAILY-05 | Phase 54 | Pending |
| BACKUP-01 | Phase 55 | Pending |
| BACKUP-02 | Phase 55 | Pending |
| BACKUP-03 | Phase 55 | Pending |
| BACKUP-04 | Phase 55 | Pending |
| BACKUP-05 | Phase 55 | Pending |
| BACKUP-06 | Phase 55 | Pending |
| BACKUP-07 | Phase 55 | Pending |
| IMPORT-01 | Phase 56 | Pending |
| IMPORT-02 | Phase 56 | Pending |
| IMPORT-03 | Phase 56 | Pending |
| IMPORT-04 | Phase 56 | Pending |
| IMPORT-05 | Phase 56 | Pending |
| IMPORT-06 | Phase 56 | Pending |
| GRPC-01 | Phase 54 | Pending |
| GRPC-02 | Phase 55 | Pending |
| GRPC-03 | Phase 56 | Pending |
| GRPC-04 | Phase 55 | Pending |

**Coverage:**
- v3.1 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-23*
*Last updated: 2026-03-23 after spec review*
