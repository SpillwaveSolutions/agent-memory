# Phase 54: Daily Markdown Export - Research

**Researched:** 2026-03-23
**Domain:** CLI command + unary gRPC RPC for markdown daily export
**Confidence:** HIGH

## Summary

Phase 54 adds a `memory daily` CLI command that produces browsable markdown files (`memory/YYYY-MM-DD.md`) from TOC day nodes, plus an `ExportDaily` unary gRPC RPC that returns structured day data (CLI renders markdown, daemon returns data). This is a well-bounded feature that extends existing patterns: a new CLI subcommand (like `summary` or `timeline`), a new unary RPC (like `GetNode` or `BrowseToc`), and a new client method (like `browse_toc`).

The codebase has clear, repeatable patterns for all three layers. The CLI uses clap derive with a `Commands` enum, dispatches in `main.rs`, and each command lives in `commands/*.rs`. The gRPC service delegates to handler functions in `query.rs` (or similar modules), and the `MemoryClient` wraps tonic calls with typed result structs. No external libraries are needed beyond what already exists.

**Primary recommendation:** Follow the existing `summary` command pattern for TOC navigation and the `timeline` command pattern for event range queries. The `ExportDaily` RPC handler assembles data from existing storage methods (`get_toc_node`, `get_child_nodes`, `get_events_in_range`, `get_grips_for_node`). Markdown rendering is a new pure function in the CLI crate.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- New `daily` subcommand added to existing `memory-cli` crate (NOT a new crate)
- `ExportDaily` is a unary RPC (NOT streaming) -- day data fits in a single response
- CLI renders markdown from structured RPC response (daemon returns data, not markdown)
- Output directory default: `./memory/` (configurable via `--dir` flag)
- One file per day: `memory/YYYY-MM-DD.md`
- Structure: Sessions -> Summary bullets -> Keywords -> Grip excerpts
- Session markers include start/end times and agent name
- Footer: "Exported from agent-memory" + timestamp + "this file is a derived view"
- Days without rollup get partial output: event timeline + "*Summary pending -- day rollup not yet complete*"
- CLI Flags: `memory daily` (today), `memory daily --range 7d` (last 7 days), `memory daily --dir ./memory/`
- No `--format` flag (always writes markdown files, not stdout)
- Request: `DateRange { start_date, end_date }` (ISO date strings)
- Response: structured `DailyExportResponse { days: [DayExport] }`
- Days with no events: skip (no empty files) -- DAILY-04
- Multiple agents in one day: group by session, each session shows agent name

### Claude's Discretion
- Whether to add `--overwrite` flag or always overwrite existing daily files
- How to format grip excerpts in markdown (blockquote style vs inline)
- Whether to include event counts per session

### Deferred Ideas (OUT OF SCOPE)
- Automatic daemon scheduler integration (DAILY-F01) -- v3.2
- Configurable export time/dir in config.toml (DAILY-F02) -- v3.2
- Markdown styling options (themes, templates) -- not planned
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DAILY-01 | `memory daily` produces browsable markdown files from TOC day nodes | CLI subcommand pattern (Commands enum + DailyArgs), TocNode day format (`toc:day:YYYY-MM-DD`), file write to `memory/YYYY-MM-DD.md` |
| DAILY-02 | Daily markdown includes session markers, summary bullets, keywords, and grip excerpts | TocNode.bullets, TocNode.keywords, TocBullet.grip_ids for provenance, segment nodes for sessions, `get_grips_for_node` for excerpts |
| DAILY-03 | `--range 7d` exports multiple days; handles days without rollup | `parse_range` pattern from timeline.rs, day node detection via `get_toc_node("toc:day:YYYY-MM-DD")`, partial output when no bullets/summary |
| DAILY-04 | Skips days with no events | Check `get_events_in_range` for day boundaries; skip file write if empty |
| DAILY-05 | Footer includes "derived view" notice and export timestamp | Pure string formatting in markdown renderer |
| GRPC-01 | `ExportDaily` unary RPC returns structured day data | New proto messages + handler in query.rs pattern, assembles from existing storage methods |
</phase_requirements>

## Standard Stack

### Core (already in workspace)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.x (derive) | CLI argument parsing | Already used for all subcommands |
| tonic | 0.12.x | gRPC server/client | Already powers all RPCs |
| prost | 0.13.x | Protobuf codegen | Already used via tonic-build |
| chrono | 0.4.x | Date parsing/formatting | Already used for TOC time boundaries |
| tokio | 1.x | Async runtime | Already used everywhere |
| serde_json | 1.x | JSON serialization | Used in CLI output module |

### Supporting (already in workspace)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| anyhow | 1.x | Error handling | CLI command error returns |
| tracing | 0.1.x | Logging | Debug logging in handler and CLI |

### No New Dependencies Needed
This phase uses only existing workspace crates. No `cargo add` required.

## Architecture Patterns

### Recommended Project Structure (changes only)
```
proto/
  memory.proto                    # Add ExportDaily RPC + messages
crates/memory-service/src/
  ingest.rs                       # Add ExportDaily dispatch to MemoryService trait impl
  query.rs                        # Add export_daily handler function
crates/memory-client/src/
  client.rs                       # Add export_daily() method + ExportDailyResult struct
crates/memory-cli/src/
  cli.rs                          # Add Daily variant to Commands enum + DailyArgs
  commands/mod.rs                 # Add `pub mod daily;`
  commands/daily.rs               # New: daily command implementation + markdown renderer
  main.rs                         # Add Commands::Daily dispatch
```

### Pattern 1: Adding a New Unary RPC (end-to-end)

**What:** The canonical path for adding a new gRPC endpoint in this project.

**Step 1 -- Proto definition** (`proto/memory.proto`):
```protobuf
// Inside service MemoryService { ... }
// ===== Daily Export RPCs (Phase 54) =====
rpc ExportDaily(ExportDailyRequest) returns (ExportDailyResponse);

// New messages (add at end of file):
message ExportDailyRequest {
    string start_date = 1;  // ISO date "YYYY-MM-DD"
    string end_date = 2;    // ISO date "YYYY-MM-DD"
}

message DayExport {
    string date = 1;                     // "YYYY-MM-DD"
    optional TocNode day_node = 2;       // Day-level TOC node (may be absent if no rollup)
    repeated TocNode segments = 3;       // Segment-level children
    repeated Event events = 4;           // All events for that day
    repeated Grip grips = 5;             // Grips referenced by bullets
    bool has_rollup = 6;                 // Whether day rollup has completed
}

message ExportDailyResponse {
    repeated DayExport days = 1;
}
```

**Step 2 -- Handler** (`crates/memory-service/src/query.rs`):
```rust
pub async fn export_daily(
    storage: Arc<Storage>,
    request: Request<ExportDailyRequest>,
) -> Result<Response<ExportDailyResponse>, Status> {
    let req = request.into_inner();
    // Parse dates, iterate days, assemble DayExport for each
    // Uses: storage.get_toc_node(), storage.get_child_nodes(),
    //       storage.get_events_in_range(), storage.get_grips_for_node()
}
```

**Step 3 -- Trait impl dispatch** (`crates/memory-service/src/ingest.rs`):
```rust
async fn export_daily(
    &self,
    request: Request<ExportDailyRequest>,
) -> Result<Response<ExportDailyResponse>, Status> {
    query::export_daily(self.storage.clone(), request).await
}
```

**Step 4 -- Client method** (`crates/memory-client/src/client.rs`):
```rust
pub async fn export_daily(
    &mut self,
    start_date: &str,
    end_date: &str,
) -> Result<ExportDailyResult, ClientError> {
    let request = tonic::Request::new(ExportDailyRequest {
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
    });
    let response = self.inner.export_daily(request).await?;
    let resp = response.into_inner();
    Ok(ExportDailyResult { days: resp.days })
}
```

**Step 5 -- CLI subcommand** (`crates/memory-cli/src/commands/daily.rs`):
```rust
pub async fn run(args: DailyArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = connect_client(&global.endpoint).await?;
    let (start, end) = compute_date_range(&args.range);
    let result = client.export_daily(&start, &end).await?;
    let dir = args.dir.unwrap_or_else(|| "./memory".to_string());
    std::fs::create_dir_all(&dir)?;
    for day in &result.days {
        let md = render_day_markdown(day);
        let path = format!("{}/{}.md", dir, day.date);
        std::fs::write(&path, md)?;
        println!("Wrote {}", path);
    }
    Ok(())
}
```

### Pattern 2: TOC Day Node Navigation

**What:** How to find and iterate day-level TOC nodes for a date range.

**Key insight:** Day node IDs are deterministic: `toc:day:YYYY-MM-DD`. You can construct the ID from a date string and call `storage.get_toc_node()` directly -- no need to traverse the hierarchy from root.

```rust
// In the ExportDaily handler:
let mut current = start_date;
while current <= end_date {
    let node_id = format!("toc:day:{}", current.format("%Y-%m-%d"));
    let day_node = storage.get_toc_node(&node_id)?;
    // day_node is Some if TOC built for this day, None if not
    current += chrono::Duration::days(1);
}
```

### Pattern 3: Pagination for Events Within a Day

**What:** `GetEvents` has a `limit` field and `has_more` flag. A single day may have more events than one page.

**How to handle:** The `ExportDaily` handler should paginate internally:
```rust
// Day boundaries: start of day (00:00:00.000) to end of day (23:59:59.999)
let day_start_ms = day.and_hms(0, 0, 0).timestamp_millis();
let day_end_ms = day.and_hms(23, 59, 59).timestamp_millis() + 999;

// Storage.get_events_in_range returns ALL events (no limit at storage level)
// The limit is only in the gRPC GetEvents response
let all_events = storage.get_events_in_range(day_start_ms, day_end_ms)?;
```

**Key finding:** `storage.get_events_in_range()` returns ALL events in the range (it scans the full RocksDB range). The `limit` is only applied in the gRPC layer (`query.rs` line 146-149). So the `ExportDaily` handler can call storage directly and get all events for a day without pagination.

### Pattern 4: Collecting Grips for a Day

**What:** Grips are linked to TOC nodes via `TocBullet.grip_ids`. To collect all grips for a day:
1. Get day node and its segment children
2. For each node, collect `grip_ids` from bullets
3. Fetch each grip via `storage.get_grip(grip_id)`

Alternative: `storage.get_grips_for_node(node_id)` returns all grips indexed under a node. This is more efficient as it does a prefix scan.

### Anti-Patterns to Avoid
- **Making ExportDaily RPC return markdown:** The daemon returns structured data; CLI renders markdown. This separation allows future consumers (GUI, API) to render differently.
- **Streaming for day export:** A single day's data (TOC node + segments + events + grips) fits comfortably in a unary response. Streaming adds complexity for no benefit here.
- **Writing to stdout:** The `daily` command writes files, not stdout. This is different from other commands.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Date range iteration | Custom loop with days math | `chrono::NaiveDate` + `Duration::days(1)` | Handles month/year boundaries correctly |
| Day node ID construction | Manual string formatting | `format!("toc:day:{}", date.format("%Y-%m-%d"))` | Consistent with `node_id.rs` patterns |
| Range parsing ("7d", "30d") | New parser | Reuse `timeline::parse_range()` | Already handles all formats |
| File path construction | Unsafe path joining | `std::path::PathBuf` | Cross-platform path handling |

## Common Pitfalls

### Pitfall 1: Days with No TOC Node vs Days with No Events
**What goes wrong:** Confusing "no TOC node" (rollup hasn't run) with "no events" (nothing happened).
**Why it happens:** A day can have events but no TOC node if the scheduler hasn't rolled up yet.
**How to avoid:** Check events independently from TOC. If events exist but no day node, output partial markdown (DAILY-03). If no events exist, skip entirely (DAILY-04).
**Warning signs:** Empty files being generated, or missing files for recent days with activity.

### Pitfall 2: Event Pagination in gRPC vs Storage
**What goes wrong:** Using `client.get_events()` (which has a limit) instead of direct storage access in the handler.
**Why it happens:** Assuming the client-side pagination applies server-side too.
**How to avoid:** The `ExportDaily` handler runs server-side with direct `storage.get_events_in_range()` access -- no limit needed.
**Warning signs:** Missing events in exported files for busy days.

### Pitfall 3: Proto Import Updates
**What goes wrong:** Adding new messages to `memory.proto` but forgetting to import them in `ingest.rs` use block and add the trait method dispatch.
**Why it happens:** Many files need coordinated updates: proto, ingest.rs (imports + trait impl), query.rs (handler), client.rs (method + result struct + imports).
**How to avoid:** Follow the 5-step pattern documented above. After proto change, run `cargo build` to regenerate, then fix compile errors.
**Warning signs:** "unresolved import" or "method not found" compile errors.

### Pitfall 4: Segment vs Session Grouping
**What goes wrong:** Treating segments as sessions 1:1. Segments are TOC nodes that may not perfectly align with session boundaries.
**Why it happens:** TOC segments are time-based slices, while sessions are bounded by session_start/session_end events.
**How to avoid:** Group events by `session_id` field for session markers. Use segment nodes for summary/bullet data. The markdown structure should show sessions (from events) annotated with segment summaries.
**Warning signs:** Session start/end times not matching segment boundaries.

### Pitfall 5: Missing `--dir` Directory Creation
**What goes wrong:** File write fails because output directory doesn't exist.
**Why it happens:** Assuming `./memory/` already exists.
**How to avoid:** Always call `std::fs::create_dir_all(&dir)` before writing files.

## Code Examples

### CLI Args Definition
```rust
// Source: Pattern from existing cli.rs
/// Arguments for the `daily` subcommand.
#[derive(Parser, Debug)]
pub struct DailyArgs {
    /// Time range for export (e.g., "7d", "30d"). Default: today only.
    #[arg(long)]
    pub range: Option<String>,

    /// Output directory for markdown files.
    #[arg(long, default_value = "./memory")]
    pub dir: String,
}
```

### Date Range Computation
```rust
// Source: Pattern from timeline.rs parse_range + chrono NaiveDate
fn compute_date_range(range: &Option<String>) -> (String, String) {
    let today = chrono::Utc::now().date_naive();
    match range {
        None => {
            let date_str = today.format("%Y-%m-%d").to_string();
            (date_str.clone(), date_str)
        }
        Some(r) => {
            let days = parse_range_to_days(r); // e.g., "7d" -> 7
            let start = today - chrono::Duration::days(days - 1);
            (
                start.format("%Y-%m-%d").to_string(),
                today.format("%Y-%m-%d").to_string(),
            )
        }
    }
}
```

### Markdown Rendering (core function)
```rust
// Source: New code, follows project patterns
fn render_day_markdown(day: &DayExport) -> String {
    let mut md = String::new();

    // Title
    md.push_str(&format!("# {}\n\n", day.date));

    if day.has_rollup {
        if let Some(ref node) = day.day_node {
            // Summary bullets
            if !node.bullets.is_empty() {
                md.push_str("## Summary\n\n");
                for bullet in &node.bullets {
                    md.push_str(&format!("- {}\n", bullet.text));
                }
                md.push('\n');
            }
            // Keywords
            if !node.keywords.is_empty() {
                md.push_str(&format!("**Keywords:** {}\n\n", node.keywords.join(", ")));
            }
        }
    } else {
        md.push_str("*Summary pending -- day rollup not yet complete*\n\n");
    }

    // Sessions (grouped by session_id from events)
    // ... group events, render session markers, grip excerpts

    // Footer
    md.push_str("---\n\n");
    md.push_str(&format!(
        "*Exported from agent-memory at {} -- this file is a derived view*\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    md
}
```

### Session Grouping from Events
```rust
// Source: Derived from timeline.rs event mapping patterns
fn group_events_by_session(events: &[ProtoEvent]) -> Vec<SessionGroup> {
    let mut sessions: IndexMap<String, SessionGroup> = IndexMap::new();
    for event in events {
        let entry = sessions
            .entry(event.session_id.clone())
            .or_insert_with(|| SessionGroup {
                session_id: event.session_id.clone(),
                agent: event.agent.clone().unwrap_or_default(),
                start_ms: event.timestamp_ms,
                end_ms: event.timestamp_ms,
                event_count: 0,
            });
        entry.end_ms = entry.end_ms.max(event.timestamp_ms);
        entry.start_ms = entry.start_ms.min(event.timestamp_ms);
        entry.event_count += 1;
    }
    sessions.into_values().collect()
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No export capability | File-based daily markdown export | Phase 54 (this phase) | First export feature |
| N/A | Unary RPC for day data | Phase 54 | Foundation for Phase 55 streaming |

**Existing patterns to follow:**
- `summary` command: TOC navigation via `get_toc_root` + `browse_toc`
- `timeline` command: Event range queries via `get_events`
- `query.rs`: Handler pattern with `Arc<Storage>` parameter

## Open Questions

1. **Overwrite behavior for existing daily files**
   - What we know: User marked this as Claude's discretion
   - Recommendation: Always overwrite (simpler, idempotent). No `--overwrite` flag needed. The files are derived views and should always reflect current state.

2. **Grip excerpt formatting in markdown**
   - What we know: User marked this as Claude's discretion
   - Recommendation: Use blockquote style (`> excerpt text`) with a "Key moment:" prefix. This is visually distinct and standard markdown.

3. **Event counts per session**
   - What we know: User marked this as Claude's discretion
   - Recommendation: Include event counts. They're cheap to compute and give a sense of session activity. Format: `### Session: claude (14:30-15:45, 23 events)`

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | Cargo.toml workspace |
| Quick run command | `cargo test -p memory-cli -p memory-service -p memory-client --lib` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DAILY-01 | CLI produces markdown files from TOC day nodes | unit + integration | `cargo test -p memory-cli daily` | No - Wave 0 |
| DAILY-02 | Markdown includes session markers, bullets, keywords, grips | unit | `cargo test -p memory-cli daily::tests` | No - Wave 0 |
| DAILY-03 | Range export + partial output for unrolled days | unit | `cargo test -p memory-cli daily::tests::partial` | No - Wave 0 |
| DAILY-04 | Skips days with no events | unit | `cargo test -p memory-cli daily::tests::skip_empty` | No - Wave 0 |
| DAILY-05 | Footer includes derived view notice | unit | `cargo test -p memory-cli daily::tests::footer` | No - Wave 0 |
| GRPC-01 | ExportDaily RPC returns structured day data | unit + integration | `cargo test -p memory-service query::tests::export_daily` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-cli -p memory-service -p memory-client --lib`
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`
- **Phase gate:** Full `task pr-precheck` before PR

### Wave 0 Gaps
- [ ] `crates/memory-cli/src/commands/daily.rs` -- unit tests for markdown rendering, date range parsing, session grouping
- [ ] `crates/memory-service/src/query.rs` -- integration test for `export_daily` handler with test storage
- [ ] `crates/memory-client/src/client.rs` -- unit test for `export_daily` method (compile-level, no live server)
- [ ] Proto changes: `cargo build` must succeed after proto modifications (tonic-build regeneration)

## Sources

### Primary (HIGH confidence)
- `proto/memory.proto` -- Full proto definition reviewed (all existing RPCs, messages, enums)
- `crates/memory-service/src/query.rs` -- Handler patterns for all 5 QRY RPCs
- `crates/memory-service/src/ingest.rs` -- MemoryServiceImpl struct and trait dispatch pattern
- `crates/memory-cli/src/cli.rs` -- Commands enum with 6 existing subcommands
- `crates/memory-cli/src/commands/summary.rs` -- TOC navigation pattern
- `crates/memory-cli/src/commands/timeline.rs` -- Event range + parse_range pattern
- `crates/memory-client/src/client.rs` -- Client method pattern (browse_toc, get_events, expand_grip)
- `crates/memory-types/src/toc.rs` -- TocNode, TocBullet, TocLevel definitions
- `crates/memory-toc/src/node_id.rs` -- Day node ID format: `toc:day:YYYY-MM-DD`
- `crates/memory-storage/src/db.rs` -- Storage methods: get_toc_node, get_child_nodes, get_events_in_range, get_grips_for_node

### Secondary (MEDIUM confidence)
- `crates/memory-cli/src/output.rs` -- JsonEnvelope pattern (daily command won't use this for output but may for error reporting)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in workspace, no new deps
- Architecture: HIGH - exact patterns verified from 6+ existing commands and RPCs
- Pitfalls: HIGH - identified from direct code inspection of storage layer and gRPC patterns
- Proto changes: HIGH - verified message structures and field numbering conventions

**Research date:** 2026-03-23
**Valid until:** 2026-04-23 (stable project, no external dependency changes)
