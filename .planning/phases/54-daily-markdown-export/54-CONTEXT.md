# Phase 54: Daily Markdown Export - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-03-23-memory-export-import-design.md)

<domain>
## Phase Boundary

This phase adds `memory daily` CLI command that produces human-readable markdown files from TOC day nodes. Also adds `ExportDaily` unary gRPC RPC that returns structured day data (CLI renders markdown). This is the "warm safety blanket" — OpenClaw-style daily files that developers can browse, edit, and version-control with Git.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- New `daily` subcommand added to existing `memory-cli` crate (NOT a new crate)
- `ExportDaily` is a unary RPC (NOT streaming) — day data fits in a single response
- CLI renders markdown from structured RPC response (daemon returns data, not markdown)
- Output directory default: `./memory/` (configurable via `--dir` flag)

### Markdown Format
- One file per day: `memory/YYYY-MM-DD.md`
- Structure: Sessions → Summary bullets → Keywords → Grip excerpts
- Session markers include start/end times and agent name
- Footer: "Exported from agent-memory" + timestamp + "this file is a derived view"
- Days without rollup get partial output: event timeline + "*Summary pending — day rollup not yet complete*"

### Data Sources
- Day-level TocNode (title, bullets, keywords) via `BrowseToc`
- Segment-level TocNodes (session boundaries, summaries) via `BrowseToc`
- Grips (key moment excerpts) via `ExpandGrip`
- Session metadata (agent, start/end times) from events
- Must paginate through all events for a day (existing `GetEvents` has `limit` + `has_more`)

### CLI Flags
- `memory daily` — export today
- `memory daily --range 7d` — last 7 days
- `memory daily --dir ./memory/` — output directory
- No `--format` flag (always writes markdown files, not stdout)

### ExportDaily RPC
- Request: `DateRange { start_date, end_date }` (ISO date strings)
- Response: structured `DailyExportResponse { days: [DayExport] }` where each `DayExport` contains day node, segment nodes, events, grip excerpts
- Unary (not streaming) — one day's data fits comfortably in a single response

### Edge Cases
- Days with no events: skip (no empty files) — DAILY-04
- Days without rollup: partial output with pending note — DAILY-03
- Multiple agents in one day: group by session, each session shows agent name

### Claude's Discretion
- Whether to add `--overwrite` flag or always overwrite existing daily files
- How to format grip excerpts in markdown (blockquote style vs inline)
- Whether to include event counts per session

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec
- `docs/superpowers/specs/2026-03-23-memory-export-import-design.md` — Full design spec (daily export section)

### Existing CLI (extend, don't duplicate)
- `crates/memory-cli/src/cli.rs` — Add `Daily` variant to `Commands` enum
- `crates/memory-cli/src/output.rs` — Reuse `estimate_tokens`, may need markdown rendering helpers
- `crates/memory-cli/src/commands/summary.rs` — Similar TOC navigation pattern to reuse
- `crates/memory-cli/src/commands/timeline.rs` — Similar event range query to reuse

### Existing gRPC
- `proto/memory.proto` — Add `ExportDaily` RPC to `MemoryService`
- `crates/memory-service/src/handlers/` — Add new handler following existing patterns
- `crates/memory-client/src/client.rs` — Add `export_daily()` method

### TOC Structure
- `crates/memory-toc/src/node_id.rs` — Day node ID format: `toc:day:YYYY-MM-DD`
- `crates/memory-types/src/toc.rs` — TocNode, TocLevel, TocBullet types

</canonical_refs>

<specifics>
## Specific Ideas

- Follow Phase 52 pattern: add subcommand to existing memory-cli crate
- Markdown rendering is a new module in memory-cli (`commands/daily.rs`)
- `ExportDaily` handler reads day node + children (segments) + events + grips — assembles into response
- Test with existing TOC data from development usage

</specifics>

<deferred>
## Deferred Ideas

- Automatic daemon scheduler integration (DAILY-F01) — v3.2
- Configurable export time/dir in config.toml (DAILY-F02) — v3.2
- Markdown styling options (themes, templates) — not planned

</deferred>

---

*Phase: 54-daily-markdown-export*
*Context gathered: 2026-03-23 via PRD Express Path*
