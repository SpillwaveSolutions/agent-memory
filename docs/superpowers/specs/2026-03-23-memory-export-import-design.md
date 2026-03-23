# Memory Export/Import System — Design Spec

**Date:** 2026-03-23
**Status:** Approved
**Milestone:** v3.1 (follows v3.0 Competitive Parity & Benchmarks)

---

## Goal

Add three capabilities to agent-memory: human-readable daily markdown export (the "warm safety blanket"), full structured JSONL backup with incremental support, and bootstrap import for migration/portability. RocksDB remains the source of truth. Exported files are derived views that can be checked into GitHub.

**Positioning:**
> Agent-Memory gives you the OpenClaw-style comfort of browsable markdown dailies
> AND the safety of a complete, round-trippable backup — without surrendering
> the structured retrieval that makes it more than a file system.

---

## Two User Experiences

| Experience | Format | Audience | Priority |
|------------|--------|----------|----------|
| Warm fuzzy (OpenClaw-style) | Pretty markdown dailies | Developer browsing memory in GitHub/editor | Human-readable |
| True backup | JSONL directory structure | CI/cron jobs, migration, disaster recovery | Machine-parseable, round-trip fidelity |

---

## Three Commands

### 1. `memory daily` — Warm Fuzzy Markdown Export

**Automatic by default.** The daemon scheduler runs daily export at end-of-day
(configurable time, default 23:59 local). Users can also invoke manually.

```bash
# Manual invocation
memory daily                      # export today
memory daily --range 7d           # last 7 days
memory daily --dir ./memory/      # output directory (default: ./memory/)

# Configuration in config.toml
[daily_export]
enabled = true                    # on by default
time = "23:59"                    # when to run (local time)
dir = "./memory"                  # output directory
```

**Output:** `memory/YYYY-MM-DD.md` per day.

**Markdown format:**

```markdown
# 2026-03-23

## Sessions

### Session 1 (09:15 — 11:42) [agent: claude]

**Summary:** Implemented RRF fusion for the retrieval orchestrator.

**Key points:**
- Chose Reciprocal Rank Fusion with k=60 as the merge strategy
- Cross-encoder reranking deferred — trait stubbed for future use
- All 4 index layers wired with fail-open behavior

**Keywords:** RRF, fusion, orchestrator, reranking, fail-open

**Key moments:**
> "The key insight is that RRF is parameter-free — no per-corpus tuning needed"
> — grip:01HN4QXKN6...

### Session 2 (14:00 — 16:30) [agent: claude]

**Summary:** Added CLI binary with 6 subcommands.

**Key points:**
- All commands route through gRPC (RocksDB exclusive lock)
- JsonEnvelope with TTY-aware output
- `memory recall` delegates to search with LLM rerank

**Keywords:** CLI, gRPC, JSON, TTY, recall

---
*Exported from agent-memory on 2026-03-23T23:59:00*
*Source of truth: RocksDB — this file is a derived view*
```

**Data sources per daily file:**
- Day-level TocNode (title, bullets, keywords) via `BrowseToc`
- Segment-level TocNodes (session boundaries, summaries) via `BrowseToc`
- Grips (key moment excerpts) via `ExpandGrip`
- Session metadata (agent, start/end times) from events

**Automatic scheduling:**
- New scheduler job alongside existing rollup jobs
- Runs after day rollup completes (so day-level summary is available)
- Skips if no events for the day
- Idempotent — re-running overwrites the file for that day

### 2. `memory backup` — Full Structured Backup

**Supports incremental export by time range** for cron-based daily backups.

```bash
# Full backup (everything)
memory backup                                  # default: ./memory-backup/
memory backup --dir ./memory-backup/

# Events-only (minimal, can rebuild everything from these)
memory backup --events-only

# Incremental (time range for cron jobs)
memory backup --since 2026-03-22              # everything since date
memory backup --since 24h                      # last 24 hours
memory backup --since 2026-03-01 --until 2026-03-23

# Cron example: daily incremental at 2am
# 0 2 * * * cd /project && memory backup --since 24h --dir ./memory-backup/
```

**Output directory structure:**

```
memory-backup/
├── manifest.json              # version, export date, layer counts, time range
├── events/                    # base layer (source of truth)
│   ├── 2026-03-22.jsonl       # one file per day
│   └── 2026-03-23.jsonl
├── toc/                       # derived layers (saves rebuild time)
│   ├── segments.jsonl         # all segment nodes
│   ├── days.jsonl             # all day nodes
│   ├── weeks.jsonl            # all week nodes
│   ├── months.jsonl           # all month nodes
│   └── years.jsonl            # all year nodes
├── grips.jsonl                # all grips (provenance links)
└── episodes.jsonl             # episodic memory (Phase 44)
```

**manifest.json:**

```json
{
  "version": "1.0",
  "agent_memory_version": "3.1.0",
  "exported_at": "2026-03-23T02:00:00Z",
  "time_range": {
    "since": "2026-03-22T00:00:00Z",
    "until": "2026-03-23T02:00:00Z"
  },
  "counts": {
    "events": 1247,
    "segments": 43,
    "days": 1,
    "weeks": 0,
    "months": 0,
    "years": 0,
    "grips": 87,
    "episodes": 12
  },
  "incremental": true,
  "events_only": false
}
```

**JSONL record format (events):**

```jsonl
{"event_id":"01HN4Q...","session_id":"sess-001","timestamp_ms":1711180800000,"event_type":"user_message","role":"user","text":"We should use JWT","metadata":{"topic":"auth"},"agent":"claude"}
```

**JSONL record format (TOC nodes):**

```jsonl
{"node_id":"toc:day:2026-03-23","level":"day","title":"2026-03-23","summary":"Implemented retrieval orchestrator...","bullets":["RRF fusion with k=60","CLI binary with 6 commands"],"keywords":["RRF","CLI","orchestrator"],"child_node_ids":["toc:segment:..."],"start_time_ms":1711152000000,"end_time_ms":1711238400000,"salience_score":0.7,"contributing_agents":["claude"]}
```

**Incremental behavior:**
- `--since` filters events by timestamp range
- TOC nodes included if their time range overlaps the export range
- Grips included if their source events fall within the range
- Episodes included if their start time falls within the range
- `manifest.json` records `incremental: true` with the time range
- Incremental files are per-day for events. On overlap, the entire day file is **overwritten** (not appended) to avoid duplicate JSONL lines. This is safe because each day file is self-contained.
- TOC/grips/episodes JSONL files are fully rewritten on each incremental run (these are small relative to events)

**Topic graph:** Topic graph data is derived from the TOC hierarchy and rebuilt via the topic clustering job. It is NOT included in backups — it is rebuilt from events like BM25/HNSW indexes.

### 3. `memory import` — Bootstrap from Backup

```bash
# Full restore from backup directory
memory import ./memory-backup/

# Events only (rebuild TOC/indexes from scratch)
memory import ./memory-backup/ --events-only

# Dry run — show what would be imported
memory import ./memory-backup/ --dry-run
```

**Import process:**
1. Read `manifest.json` — validate version compatibility
2. Import events first (base layer, chronological order)
3. Import TOC nodes (segments → days → weeks → months → years)
4. Import grips
5. Import episodes
6. Trigger outbox entries for any events that need indexing
6. Report: events imported, nodes restored, time elapsed

**Safety:**
- Idempotent — events with existing IDs are skipped (dedup by event_id)
- `--dry-run` shows counts without writing
- Does NOT delete existing data — additive merge only
- If importing events-only, user must trigger TOC rebuild. The daemon's existing `rebuild-toc` admin command handles this. If the command does not exist at implementation time, add it as a new `memory-daemon rebuild-toc` subcommand that triggers the full rollup pipeline (segment → day → week → month → year).

**What is NOT imported:**
- BM25/HNSW indexes (rebuilt from events via outbox)
- InFlightBuffer state (ephemeral, within-session only)
- Scheduler checkpoints (rebuild on next daemon start)

---

## Architecture

### New gRPC RPCs

All three commands route through gRPC to the daemon.

| RPC | Direction | Purpose |
|-----|-----------|---------|
| `ExportDaily(DateRange)` | Unary | Returns structured day data (nodes, segments, events, grips). **CLI renders markdown.** |
| `ExportBackup(BackupOptions)` | Server streaming | Streams all data in layer order as JSONL chunks |
| `ImportBackup(stream)` | Client streaming | Accepts JSONL stream, writes to RocksDB |

**Streaming RPCs are new infrastructure.** The current proto has zero streaming RPCs — all existing RPCs are unary. Adding server-side streaming (`ExportBackup`) and client streaming (`ImportBackup`) requires wiring tonic streaming support into the server framework. This is the first use of gRPC streaming in the project and represents meaningful new work (not just "new RPCs follow existing patterns").

**Markdown rendering happens in the CLI, not the daemon.** `ExportDaily` returns structured data (day nodes, segment nodes, events, grip excerpts). The CLI's `daily` command renders this into the markdown format shown above, following the same pattern as Phase 52 where the CLI handles all output formatting (`output.rs`).

### Daemon Scheduler Integration (v3.1 scope: CLI only)

**v3.1 ships manual `memory daily` CLI command only.** Automatic daemon scheduler integration is deferred to v3.2 — it adds complexity (filesystem permissions from daemon context, config for time/dir, dependency ordering with rollup jobs) that isn't needed when users can automate with system cron:

```bash
# Cron example: daily markdown export at midnight
0 0 * * * cd /project && memory daily --range 1d --dir ./memory/
```

**v3.2 (deferred):** Add `DailyExportJob` to daemon scheduler, running after `DayRollupJob` completes. Config via `[daily_export]` section in config.toml.

**Manual invocation note:** `memory daily --range 7d` may hit days whose rollup hasn't completed yet. For those days, the command generates a partial markdown file from raw events (no summary bullets, just event timeline). The file includes a note: `*Summary pending — day rollup not yet complete*`.

### CLI → Daemon Flow

```
memory daily    → gRPC ExportDaily    → read TocNodes + Events → render markdown → write files
memory backup   → gRPC ExportBackup   → stream all layers as JSONL → write directory
memory import   → read JSONL files    → gRPC ImportBackup stream → write to RocksDB
```

Note: `memory import` reads files locally and streams to daemon. The CLI does the file reading; the daemon does the writing.

---

## Configuration

New section in `config.toml`:

```toml
[backup]
default_dir = "./memory-backup"   # default backup directory

# [daily_export] — deferred to v3.2 (automatic scheduler integration)
# enabled = true
# time = "23:59"
# dir = "./memory"
```

---

## What This Does NOT Include

- Live sync / file watching (not bidirectional)
- Markdown import (only JSONL backup imports; markdown dailies are read-only views)
- Index file backup (BM25/HNSW are platform-specific; rebuild from events)
- Automatic cloud backup (use cron + `memory backup --since 24h` + git push)
- Editing markdown dailies and having changes reflected in RocksDB

---

## Success Criteria

**Daily export:**
- [ ] `memory daily` produces browsable markdown that a developer would check into GitHub
- [ ] `memory daily --range 7d` handles days without rollup (partial output with pending note)
- [ ] Markdown format includes sessions, summaries, keywords, and grip excerpts
- [ ] Skips days with no events

**Backup:**
- [ ] `memory backup` exports all layers as JSONL directory structure
- [ ] `memory backup --since 24h` exports only recent data (incremental)
- [ ] `memory backup --events-only` exports just the base layer
- [ ] `manifest.json` includes counts, version, time range for validation
- [ ] Incremental backups merge into existing directory without data loss

**Import:**
- [ ] `memory import` restores a full backup to a fresh RocksDB
- [ ] Round-trip test: export → wipe → import → all queries return same results
- [ ] `--dry-run` shows what would be imported without writing
- [ ] Idempotent — safe to re-run (dedup by event_id)

**All:**
- [ ] All commands pass `task pr-precheck`
- [ ] No changes to existing commands or daemon behavior

---

## Relationship to Existing System

This extends the existing architecture without modifying it:

- **RocksDB remains source of truth** — exports are derived views
- **TOC hierarchy unchanged** — daily export reads existing day/segment nodes
- **Summarizer unchanged** — daily export consumes summaries, doesn't generate them
- **gRPC extended** — new RPCs include first streaming RPCs (tonic streaming support)
- **Scheduler unchanged for v3.1** — automatic daily export deferred to v3.2
- **CLI extended** — new subcommands follow Phase 52 patterns

The OpenClaw-style daily files give users a human-readable window into their agent's memory without requiring them to understand gRPC, RocksDB, or the TOC hierarchy. The backup/import system gives them complete portability and disaster recovery.
