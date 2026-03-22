# Phase 52: Simple CLI API - Research

**Researched:** 2026-03-21
**Domain:** Rust CLI binary, clap derive API, orchestrator integration, gRPC client
**Confidence:** HIGH

## Summary

Phase 52 creates a new `memory` binary (crate: `memory-cli`) with 6 subcommands (`search`, `context`, `recall`, `add`, `timeline`, `summary`) backed by the Phase 51 `MemoryOrchestrator`. The workspace already has `clap 4.5.56` with derive features, `serde_json`, `tokio`, and all other needed dependencies. The binary uses the `MemoryOrchestrator<E: LayerExecutor>` from `memory-orchestrator` for read commands and the `MemoryClient` gRPC client for writes.

The critical design decision is how read commands construct the orchestrator. The `SimpleLayerExecutor` in `memory-service::retrieval` (currently private/`struct`) wires BM25, Vector, Topics, and Agentic layers using `Arc<Storage>`, `Arc<TeleportSearcher>`, `Arc<VectorTeleportHandler>`, and `Arc<TopicGraphHandler>`. For the CLI to run in-process reads, it needs either: (a) make `SimpleLayerExecutor` public and depend on `memory-service`, or (b) route all read commands through gRPC to the daemon. **Recommendation: Route all commands through gRPC** (including reads). This avoids the CLI needing direct RocksDB access (which would conflict with the daemon's exclusive lock), avoids pulling in the heavy `memory-service` dependency tree (embeddings, vector, topics, etc.), and keeps the binary lean. The CLI becomes a thin gRPC client wrapper with JSON envelope formatting.

**Primary recommendation:** All 6 commands route through `MemoryClient` gRPC, requiring daemon to be running. The `memory-orchestrator` crate is NOT used directly in the CLI binary -- instead, the daemon's existing `RouteQuery` RPC provides orchestrated search results, and `GetTocRoot`/`BrowseToc`/`GetEvents` RPCs serve timeline/summary queries.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- New crate `crates/memory-cli/` with `[[bin]]` entry producing `memory` binary
- Each subcommand calls `MemoryOrchestrator` (Phase 51) via in-process call or `MemoryClient` gRPC for writes
- `memory-daemon` binary and existing skill hooks unchanged
- `memory recall` is a named alias for `memory search --rerank=llm --top=10` (same code path)
- New `memory` binary -- NOT renaming `memory-daemon`
- JSON Envelope: status, query, results, context, error, meta fields
- `meta` includes `retrieval_ms`, `tokens_estimated`, `confidence`
- `--format=json` default when stdout is not a TTY; human-readable when interactive
- Uses `std::io::IsTerminal` for TTY detection (no `atty` dep)
- All commands exit 0 on success, non-zero on hard failure
- `clap` derive API with `Cli`, `Commands` enum, `GlobalArgs`
- Global args: `--format`, `--endpoint` (default `http://127.0.0.1:50051`)
- `memory add` routes through `MemoryClient` over gRPC -- daemon must be running
- If daemon not running, exits non-zero with message: `"memory daemon not running -- start with: memory-daemon start"`

### Claude's Discretion
- How to construct `MemoryOrchestrator` with real storage for read commands (may need gRPC client or direct storage access)
- Whether `timeline` and `summary` call orchestrator or directly query TOC gRPC RPCs
- Error handling strategy for partial failures (e.g., orchestrator returns results but with degraded indexes)
- Whether to add `--verbose` or `--debug` flag for tracing output

### Deferred Ideas (OUT OF SCOPE)
- REST/HTTP endpoint (CLI-F01) -- future milestone
- Python SDK (CLI-F02) -- wraps CLI binary, future milestone
- `--verbose` / `--debug` tracing flags -- nice to have, not required
- Updated canonical plugin source to reference `memory` binary in new hooks -- future integration task
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | New `memory` binary with 6 subcommands | New `memory-cli` crate with `[[bin]] name = "memory"`, clap derive |
| CLI-02 | `memory search` returns JSON envelope with results, meta, confidence | `MemoryClient::hybrid_search()` or `RouteQuery` RPC -> `JsonEnvelope` |
| CLI-03 | `memory recall` delegates to search with `--rerank=llm --top=10` | Trivial: construct `SearchArgs` and call search handler |
| CLI-04 | `memory add` writes via gRPC, exits non-zero if daemon down | `MemoryClient::ingest()`, catch connection error with context message |
| CLI-05 | TTY detection: JSON when piped, human-readable when interactive | `std::io::IsTerminal` on `std::io::stdout()` (stable Rust 1.70+) |
| CLI-06 | `memory context` returns structured context for prompt injection | Route through daemon's `RouteQuery` RPC, format as MemoryContext-style JSON |
| CLI-07 | `memory timeline` and `memory summary` query TOC by entity/range | `MemoryClient::get_toc_root()`, `browse_toc()`, `get_events()` RPCs |
| CLI-08 | `memory-daemon` binary and existing skill hooks unchanged | No modifications to `memory-daemon` crate |
| CLI-09 | All commands exit 0 on success, non-zero on hard failure | `std::process::exit(1)` on error, `main() -> Result<()>` |
| CLI-10 | `meta.tokens_estimated` in JSON envelope | Estimate from response text: `chars * 0.75 + 50` (per Phase 51 decision) |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5.56 | CLI argument parsing (derive API) | Already in workspace, derive API is idiomatic Rust |
| serde_json | 1.0.149 | JSON serialization for envelope | Already in workspace |
| tokio | 1.49.0 | Async runtime for gRPC calls | Already in workspace |
| memory-client | workspace | gRPC client to daemon | Existing crate with all RPCs already implemented |
| memory-types | workspace | Event type for `add` command | Existing, needed for event construction |
| thiserror | 2.0.18 | Error types | Already in workspace |
| anyhow | 1.0 | Error handling in main | Already in workspace |
| tracing | 0.1 | Logging | Already in workspace |
| tracing-subscriber | 0.3 | Log output setup | Already in workspace |

### NOT Needed (Discretion Decision)
| Library | Why NOT | Alternative |
|---------|---------|-------------|
| memory-orchestrator | RocksDB lock conflict, heavy deps | Use daemon's `RouteQuery` gRPC RPC |
| memory-service | Private `SimpleLayerExecutor`, massive dep tree | Route through gRPC |
| memory-storage | Would conflict with daemon's DB lock | Not needed |
| atty | Deprecated | Use `std::io::IsTerminal` (stable since Rust 1.70) |

**Installation:**
```bash
# No new external dependencies needed -- all are in workspace already
# Just add workspace members reference
```

## Architecture Patterns

### Recommended Project Structure
```
crates/memory-cli/
  Cargo.toml           # [[bin]] name = "memory"
  src/
    main.rs            # Entry: parse CLI, dispatch commands
    cli.rs             # Clap structs: Cli, Commands, GlobalArgs, *Args
    output.rs          # JsonEnvelope, Meta, print_output(), TTY detection
    client.rs          # connect_client() helper wrapping MemoryClient
    commands/
      mod.rs           # Module declarations
      search.rs        # memory search -- calls RouteQuery RPC
      context.rs       # memory context -- calls RouteQuery RPC, formats MemoryContext
      recall.rs        # memory recall -- delegates to search with llm+top=10
      add.rs           # memory add -- calls IngestEvent RPC
      timeline.rs      # memory timeline -- calls GetEvents/BrowseToc RPCs
      summary.rs       # memory summary -- calls GetTocRoot/BrowseToc RPCs
```

### Pattern 1: gRPC-Only Architecture
**What:** All 6 commands go through `MemoryClient` gRPC to the running daemon. No in-process storage access.
**When to use:** Always (for this phase).
**Why:** RocksDB uses exclusive file locks. If the daemon has the DB open, the CLI cannot open it simultaneously. The daemon already has all retrieval layers (BM25, Vector, Topics, Agentic) wired through `SimpleLayerExecutor` and the `RetrievalHandler.route_query()` RPC.

**Critical evidence:** The daemon's `start_daemon()` in `commands.rs:373` opens `Storage::open(&db_path)` which acquires exclusive RocksDB locks. A second process cannot open the same path.

```rust
// client.rs
use anyhow::{Context, Result};
use memory_client::MemoryClient;

pub async fn connect_client(endpoint: &str) -> Result<MemoryClient> {
    MemoryClient::connect(endpoint)
        .await
        .context(format!(
            "memory daemon not running -- start with: memory-daemon start\n(endpoint: {endpoint})"
        ))
}
```

### Pattern 2: JsonEnvelope Output
**What:** Consistent JSON wrapper for all command outputs.
**When to use:** Every command response.

```rust
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEnvelope {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub meta: Meta,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Meta {
    pub retrieval_ms: u64,
    pub tokens_estimated: usize,
    pub confidence: f64,
}
```

### Pattern 3: TTY-Aware Output
**What:** JSON when piped/non-TTY, human-readable when interactive terminal.

```rust
pub fn print_output(envelope: &JsonEnvelope, force_json: bool) {
    let is_tty = std::io::stdout().is_terminal();
    if force_json || !is_tty {
        println!("{}", serde_json::to_string(envelope).unwrap_or_default());
    } else {
        // Human-readable rendering
        if envelope.status == "ok" {
            if let Some(q) = &envelope.query {
                println!("Query: {q}");
            }
            if let Some(r) = &envelope.results {
                println!("{}", serde_json::to_string_pretty(r).unwrap_or_default());
            }
        } else {
            eprintln!("Error: {}", envelope.error.as_deref().unwrap_or("unknown"));
        }
    }
}
```

### Pattern 4: Recall as Search Alias
**What:** `memory recall "query"` constructs `SearchArgs { rerank: Some("llm"), top: 10 }` and calls `search::run()`.

### Anti-Patterns to Avoid
- **Direct Storage access from CLI:** RocksDB exclusive lock prevents dual-process access. Always go through gRPC.
- **In-process orchestrator without daemon:** Requires pulling in memory-service, memory-embeddings, memory-vector, memory-topics, memory-search, memory-storage -- massive binary bloat and lock conflicts.
- **Using `atty` crate:** Deprecated. Use `std::io::IsTerminal` (stable since Rust 1.70).
- **Printing errors to stdout:** Errors go to stderr, JSON goes to stdout. Critical for piped usage.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI parsing | Custom arg parser | `clap` 4.5 derive API | Already in workspace, full-featured |
| gRPC communication | Custom HTTP/protobuf | `MemoryClient` from `memory-client` crate | Already implements all RPCs needed |
| TTY detection | Manual fd checks | `std::io::IsTerminal` | Standard library, stable since Rust 1.70 |
| JSON output | Manual string formatting | `serde_json::to_string()` with `JsonEnvelope` | Type-safe, consistent |
| Token estimation | Complex tokenizer | `chars * 0.75 + 50` heuristic | Per Phase 51 decision, sufficient accuracy |
| Error context | Generic error messages | `anyhow::Context` with actionable messages | "daemon not running -- start with: memory-daemon start" |

## Common Pitfalls

### Pitfall 1: RocksDB Lock Conflict
**What goes wrong:** CLI tries to open Storage directly while daemon has it locked.
**Why it happens:** RocksDB uses exclusive file locks by default.
**How to avoid:** All commands go through gRPC to the daemon. No direct storage access.
**Warning signs:** "lock" or "LOCK" errors when running CLI with daemon active.

### Pitfall 2: Connection Error Messaging
**What goes wrong:** gRPC connection failure produces cryptic tonic error.
**Why it happens:** Raw `tonic::transport::Error` is not user-friendly.
**How to avoid:** Wrap with `anyhow::Context` providing actionable message: "memory daemon not running -- start with: memory-daemon start".
**Warning signs:** Users see "transport error" instead of clear instructions.

### Pitfall 3: TTY Detection in Tests
**What goes wrong:** Tests always see non-TTY (piped), so TTY branch never tested.
**Why it happens:** Test runners pipe stdout.
**How to avoid:** Test the `print_output()` function with explicit `force_json` parameter. Unit test the `JsonEnvelope` serialization separately.

### Pitfall 4: Binary Name Collision
**What goes wrong:** `memory` binary name could conflict with system commands.
**Why it happens:** Generic binary name.
**How to avoid:** This is intentional per spec. The `[[bin]] name = "memory"` is the desired developer-facing name. No action needed, but verify `cargo install` path.

### Pitfall 5: Exit Code Handling
**What goes wrong:** `main() -> Result<()>` with `?` returns exit code 1 but with ugly debug output.
**Why it happens:** `anyhow` prints Debug format on error.
**How to avoid:** Catch errors in main, print JSON error envelope, then `std::process::exit(1)`.

### Pitfall 6: memory-client MemoryClient Requires `&mut self`
**What goes wrong:** `MemoryClient` methods take `&mut self` (tonic client pattern).
**Why it happens:** Tonic's generated client uses `&mut self` for all RPC calls.
**How to avoid:** Create the client once per command invocation. No need for shared/concurrent access since CLI runs one command then exits.

## Code Examples

### Existing MemoryClient RPCs Available for CLI Commands

```rust
// From crates/memory-client/src/client.rs -- these are the RPCs we wire to:

// For `memory add`:
client.ingest(event).await                    // -> (event_id, created)

// For `memory search` / `memory context` / `memory recall`:
// Option A: Use RouteQuery RPC (full orchestrated search)
// Option B: Use hybrid_search() for simpler search
client.hybrid_search(query, top_k, mode, bm25_w, vec_w, target).await

// For `memory timeline`:
client.get_events(from_ms, to_ms, limit).await     // -> GetEventsResult
client.browse_toc(parent_id, limit, token).await    // -> BrowseTocResult

// For `memory summary`:
client.get_toc_root().await                         // -> Vec<TocNode>
client.get_node(node_id).await                      // -> Option<TocNode>
```

### RouteQuery RPC for Orchestrated Search

The daemon's `RetrievalHandler::route_query()` (in `memory-service/src/retrieval.rs`) provides the full orchestrated pipeline: intent classification -> tier detection -> fallback chain -> execution -> ranking -> staleness filtering. This is the closest equivalent to calling `MemoryOrchestrator.query()` in-process.

```rust
// The RouteQuery RPC returns RouteQueryResponse with:
// - results: Vec<RetrievalResult> (doc_id, doc_type, score, text_preview, metadata)
// - explainability: ExplainabilityPayload (intent, tier, layers_tried, etc.)
// - execution_time_ms, result_count

// CLI maps this to JsonEnvelope:
// envelope.results = results mapped to JSON
// envelope.meta.retrieval_ms = response.execution_time_ms
// envelope.meta.tokens_estimated = sum of text lengths * 0.75 + 50
// envelope.meta.confidence = top result score or explainability confidence
```

### Constructing Events for `memory add`

```rust
use memory_types::{Event, EventType, EventRole};
use chrono::Utc;
use ulid::Ulid;

let event = Event::new(
    Ulid::new().to_string(),
    format!("cli-{}", Ulid::new()),  // session_id for CLI-originated events
    Utc::now(),
    EventType::UserMessage,  // or map from --kind flag
    EventRole::User,
    args.content.clone(),
);
// Optionally set agent: event.agent = args.agent;
```

### Timeline Range Parsing

```rust
// Parse "7d", "30d", "1w", etc. into millisecond range
fn parse_range(range: &str) -> (i64, i64) {
    let now = chrono::Utc::now().timestamp_millis();
    let duration_ms = if range.ends_with('d') {
        let days: i64 = range.trim_end_matches('d').parse().unwrap_or(7);
        days * 86_400_000
    } else if range.ends_with('w') {
        let weeks: i64 = range.trim_end_matches('w').parse().unwrap_or(1);
        weeks * 7 * 86_400_000
    } else {
        7 * 86_400_000 // default 7 days
    };
    (now - duration_ms, now)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `atty` crate for TTY | `std::io::IsTerminal` | Rust 1.70 (June 2023) | No external dep needed |
| `structopt` for CLI | `clap` 4.x derive API | 2022 | `structopt` merged into clap |
| Print debug on error | JSON error envelope | This phase | Machine-parseable errors |

## Open Questions

1. **RouteQuery RPC vs HybridSearch for search/context/recall**
   - What we know: `RouteQuery` provides full orchestrated pipeline (intent -> tier -> chain -> execute -> rank -> filter). `HybridSearch` provides simpler BM25+Vector fusion.
   - What's unclear: Whether `RouteQuery` response format maps cleanly to the `JsonEnvelope` expected by the spec.
   - Recommendation: Use `RouteQuery` RPC for search/context/recall commands. It provides the most complete retrieval including the explainability payload which contains confidence and retrieval_ms. If `RouteQuery` doesn't return enough text for token estimation, supplement with `get_node()` lookups.

2. **Summary command content source**
   - What we know: TOC nodes have summaries at day/week/month levels. `get_toc_root()` returns year-level nodes, `browse_toc()` returns children.
   - What's unclear: Whether existing TOC summaries are populated (depends on summarizer config).
   - Recommendation: Navigate TOC hierarchy (root -> year -> month/week -> day) and collect `summary` fields from nodes in the requested range. Return whatever summaries exist; empty results are valid.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio::test |
| Config file | Workspace Cargo.toml (existing) |
| Quick run command | `cargo test -p memory-cli` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | 6 subcommands parse correctly | unit | `cargo test -p memory-cli cli` | Wave 0 |
| CLI-02 | search returns JSON envelope | unit | `cargo test -p memory-cli search` | Wave 0 |
| CLI-03 | recall delegates to search | unit | `cargo test -p memory-cli recall` | Wave 0 |
| CLI-04 | add errors when daemon down | unit | `cargo test -p memory-cli add` | Wave 0 |
| CLI-05 | TTY detection logic | unit | `cargo test -p memory-cli output` | Wave 0 |
| CLI-06 | context returns structured JSON | unit | `cargo test -p memory-cli context` | Wave 0 |
| CLI-07 | timeline/summary query TOC | unit | `cargo test -p memory-cli timeline` | Wave 0 |
| CLI-08 | daemon binary unchanged | manual | Verify no changes to memory-daemon crate | N/A |
| CLI-09 | Exit codes 0/non-zero | unit | `cargo test -p memory-cli exit` | Wave 0 |
| CLI-10 | tokens_estimated in meta | unit | `cargo test -p memory-cli meta` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-cli && cargo clippy -p memory-cli -- -D warnings`
- **Per wave merge:** `task pr-precheck`
- **Phase gate:** Full `task pr-precheck` green before verification

### Wave 0 Gaps
- [ ] `crates/memory-cli/` -- entire crate does not exist yet
- [ ] Unit tests for `JsonEnvelope` serialization
- [ ] Unit tests for CLI argument parsing (clap derive tests are usually snapshot-style)
- [ ] Unit tests for range parsing utility
- [ ] Note: Integration tests requiring running daemon should be in `crates/e2e-tests/` or marked `#[ignore]`

## Sources

### Primary (HIGH confidence)
- `crates/memory-orchestrator/src/orchestrator.rs` -- MemoryOrchestrator API, LayerExecutor generic
- `crates/memory-orchestrator/src/types.rs` -- OrchestratorConfig, MemoryContext, RankedResult
- `crates/memory-client/src/client.rs` -- All existing gRPC client RPCs
- `crates/memory-service/src/retrieval.rs` -- SimpleLayerExecutor (private), RetrievalHandler RPCs
- `crates/memory-daemon/src/commands.rs:373` -- Storage::open() exclusive lock pattern
- `crates/memory-retrieval/src/executor.rs` -- LayerExecutor trait definition
- `Cargo.toml` (workspace) -- Resolved dependency versions

### Secondary (MEDIUM confidence)
- `docs/superpowers/plans/2026-03-21-v3-phase-b-simple-cli-api.md` -- Implementation plan with code snippets
- `.planning/phases/52-simple-cli-api/52-CONTEXT.md` -- User decisions

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies already in workspace, versions verified via cargo metadata
- Architecture: HIGH - gRPC-only approach verified by RocksDB lock behavior in daemon code
- Pitfalls: HIGH - based on direct code inspection of existing crate patterns
- Discretion decisions: MEDIUM - gRPC-only is a deviation from CONTEXT.md "in-process" suggestion, but technically necessary due to RocksDB locks

**Research date:** 2026-03-21
**Valid until:** 2026-04-21 (stable domain, no fast-moving dependencies)
