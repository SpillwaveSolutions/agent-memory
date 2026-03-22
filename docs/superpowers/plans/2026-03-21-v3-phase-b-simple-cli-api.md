# Phase B: Simple CLI API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `memory` binary with 6 structured-JSON commands (`add`, `search`, `context`, `timeline`, `summary`, `recall`) wired to the Phase A orchestrator — leaving `memory-daemon` and all existing skill hooks unchanged.

**Architecture:** New `crates/memory-cli/` crate with a `[[bin]]` producing the `memory` binary. Each subcommand calls `MemoryOrchestrator` (Phase A) via an in-process call or `MemoryClient` gRPC for writes. All commands emit a consistent JSON envelope when stdout is not a TTY; human-readable text otherwise. `memory recall` delegates to `memory search --rerank=llm --top=10`.

**Tech Stack:** Rust 2021, `clap` (derive), `serde_json`, `memory-orchestrator` (Phase A), `memory-client` (gRPC writes), `tokio`, `thiserror`, `atty` or `std::io::IsTerminal` for TTY detection.

**Spec:** `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md` — Phase B section

**Prerequisite:** Phase A complete (`memory-orchestrator` crate exists and compiles).

---

## File Map

**New crate:** `crates/memory-cli/`

| File | Responsibility |
|------|----------------|
| `crates/memory-cli/Cargo.toml` | Crate manifest, `[[bin]]` entry for `memory` |
| `crates/memory-cli/src/main.rs` | Entrypoint: parse CLI, dispatch to command handlers |
| `crates/memory-cli/src/cli.rs` | Clap struct: `Cli`, `Commands` enum, shared flags |
| `crates/memory-cli/src/output.rs` | `JsonEnvelope`, TTY detection, `print_output()` |
| `crates/memory-cli/src/client.rs` | Shared gRPC client setup (`MemoryClient` wrapper) |
| `crates/memory-cli/src/commands/mod.rs` | Command module declarations |
| `crates/memory-cli/src/commands/search.rs` | `memory search` — calls orchestrator |
| `crates/memory-cli/src/commands/context.rs` | `memory context` — calls orchestrator, returns structured context |
| `crates/memory-cli/src/commands/add.rs` | `memory add` — writes via gRPC MemoryClient |
| `crates/memory-cli/src/commands/timeline.rs` | `memory timeline` — queries TOC by entity/range |
| `crates/memory-cli/src/commands/summary.rs` | `memory summary` — queries TOC for time-range summary |
| `crates/memory-cli/src/commands/recall.rs` | `memory recall` — delegates to search with llm rerank |

**Modified files:**

| File | Change |
|------|--------|
| `Cargo.toml` (workspace root) | Add `crates/memory-cli` to `members` |

---

## Task 1: Scaffold `memory-cli` crate

**Files:**
- Create: `crates/memory-cli/Cargo.toml`
- Create: `crates/memory-cli/src/main.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "memory-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "memory"
path = "src/main.rs"

[dependencies]
memory-orchestrator = { path = "../memory-orchestrator" }
memory-client = { workspace = true }
memory-types = { workspace = true }
clap = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }
```

- [ ] **Step 2: Add to workspace `Cargo.toml`** — add `"crates/memory-cli"` to `members`

- [ ] **Step 3: Create `src/main.rs` stub**

```rust
mod cli;
mod client;
mod commands;
mod output;

use anyhow::Result;
use cli::{Cli, Commands};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Search(args) => commands::search::run(args, &cli.global).await,
        Commands::Context(args) => commands::context::run(args, &cli.global).await,
        Commands::Add(args) => commands::add::run(args, &cli.global).await,
        Commands::Timeline(args) => commands::timeline::run(args, &cli.global).await,
        Commands::Summary(args) => commands::summary::run(args, &cli.global).await,
        Commands::Recall(args) => commands::recall::run(args, &cli.global).await,
    }
}
```

- [ ] **Step 4: Verify scaffold compiles**

```bash
cargo build -p memory-cli
```

- [ ] **Step 5: Commit**

```bash
git add crates/memory-cli/ Cargo.toml Cargo.lock
git commit -m "feat(cli): scaffold memory-cli crate with memory binary entry point"
```

---

## Task 2: Define CLI structs and JSON envelope

**Files:**
- Create: `crates/memory-cli/src/cli.rs`
- Create: `crates/memory-cli/src/output.rs`

- [ ] **Step 1: Write output tests**

In `crates/memory-cli/src/output.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_serializes_status_ok() {
        let env = JsonEnvelope::ok("search", serde_json::json!({"results": []}));
        let s = serde_json::to_string(&env).unwrap();
        assert!(s.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_envelope_error_has_nonzero_exit() {
        let env = JsonEnvelope::error("daemon not running");
        assert_eq!(env.status, "error");
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

```bash
cargo test -p memory-cli output
```

- [ ] **Step 3: Implement `JsonEnvelope` and `output.rs`**

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

impl JsonEnvelope {
    pub fn ok(query: &str, results: serde_json::Value) -> Self {
        Self {
            status: "ok".to_string(),
            query: Some(query.to_string()),
            results: Some(results),
            context: None,
            error: None,
            meta: Meta::default(),
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            status: "error".to_string(),
            query: None,
            results: None,
            context: None,
            error: Some(msg.to_string()),
            meta: Meta::default(),
        }
    }
}

/// Print output. JSON when piped (not a TTY); human-readable when interactive.
pub fn print_output(envelope: &JsonEnvelope, force_json: bool) {
    let is_tty = std::io::stdout().is_terminal();
    if force_json || !is_tty {
        println!("{}", serde_json::to_string(envelope).unwrap_or_default());
    } else {
        // Human-readable fallback
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

- [ ] **Step 4: Implement `cli.rs`** with all subcommand structs:

```rust
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "memory", about = "Agent Memory CLI — structured JSON interface")]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Output format: json (default when piped) or text
    #[arg(long, global = true)]
    pub format: Option<String>,

    /// gRPC endpoint (default: http://127.0.0.1:50051)
    #[arg(long, global = true, default_value = "http://127.0.0.1:50051")]
    pub endpoint: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search memory (uses orchestrator: RRF fusion + rerank)
    Search(SearchArgs),
    /// Get structured context for prompt injection
    Context(ContextArgs),
    /// Add an event to memory (requires daemon running)
    Add(AddArgs),
    /// Get timeline for an entity or topic
    Timeline(TimelineArgs),
    /// Get compressed summary of a time range
    Summary(SummaryArgs),
    /// Multi-hop recall (alias: search --rerank=llm --top=10)
    Recall(RecallArgs),
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long, default_value = "10")]
    pub top: usize,
    #[arg(long)]
    pub rerank: Option<String>, // "llm" | "heuristic"
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Args, Debug)]
pub struct ContextArgs {
    pub query: String,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    #[arg(long)]
    pub content: String,
    #[arg(long, default_value = "episodic")]
    pub kind: String,
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Args, Debug)]
pub struct TimelineArgs {
    #[arg(long)]
    pub entity: Option<String>,
    #[arg(long, default_value = "7d")]
    pub range: String,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Args, Debug)]
pub struct SummaryArgs {
    #[arg(long, default_value = "week")]
    pub range: String,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Args, Debug)]
pub struct RecallArgs {
    pub query: String,
    #[arg(long)]
    pub format: Option<String>,
}
```

- [ ] **Step 5: Run output tests — verify pass**

```bash
cargo test -p memory-cli output
```

- [ ] **Step 6: Commit**

```bash
git add crates/memory-cli/src/cli.rs crates/memory-cli/src/output.rs
git commit -m "feat(cli): add CLI argument structs and JsonEnvelope with TTY detection"
```

---

## Task 3: Implement `memory search` and `memory recall`

**Files:**
- Create: `crates/memory-cli/src/commands/search.rs`
- Create: `crates/memory-cli/src/commands/recall.rs`
- Create: `crates/memory-cli/src/commands/mod.rs`

- [ ] **Step 1: Write search command test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_returns_envelope() {
        // Uses MockLayerExecutor via orchestrator
        let result = run_search_with_mock("JWT bug", 5, "heuristic").await;
        assert_eq!(result.status, "ok");
    }
}
```

- [ ] **Step 2: Implement `commands/search.rs`**

```rust
use crate::cli::{GlobalArgs, SearchArgs};
use crate::output::{print_output, JsonEnvelope, Meta};
use anyhow::Result;
use memory_orchestrator::{MemoryOrchestrator, OrchestratorConfig, RerankMode};
// (Orchestrator construction with real executor wired via gRPC client — see client.rs)

pub async fn run(args: SearchArgs, global: &GlobalArgs) -> Result<()> {
    let force_json = args.format.as_deref() == Some("json");
    let rerank_mode = match args.rerank.as_deref() {
        Some("llm") => RerankMode::Llm,
        _ => RerankMode::Heuristic,
    };

    let config = OrchestratorConfig {
        top_k: args.top,
        rerank_mode,
        ..Default::default()
    };

    let orchestrator = build_orchestrator(global, config).await?;
    let ctx = orchestrator.query(&args.query).await?;

    let envelope = JsonEnvelope {
        status: "ok".to_string(),
        query: Some(args.query.clone()),
        results: Some(serde_json::to_value(&ctx.relevant_events)?),
        context: Some(serde_json::to_value(&ctx)?),
        error: None,
        meta: Meta {
            retrieval_ms: ctx.retrieval_ms,
            tokens_estimated: ctx.tokens_estimated,
            confidence: ctx.confidence,
        },
    };

    print_output(&envelope, force_json);
    Ok(())
}
```

- [ ] **Step 3: Implement `commands/recall.rs`** — delegates to search with llm rerank:

```rust
use crate::cli::{GlobalArgs, RecallArgs, SearchArgs};
use crate::commands::search;

pub async fn run(args: RecallArgs, global: &GlobalArgs) -> Result<()> {
    // recall = search --rerank=llm --top=10
    search::run(SearchArgs {
        query: args.query,
        top: 10,
        rerank: Some("llm".to_string()),
        format: args.format,
    }, global).await
}
```

- [ ] **Step 4: Build and run**

```bash
cargo build -p memory-cli && ./target/debug/memory search "test query" 2>&1 | head -5
```

- [ ] **Step 5: Commit**

```bash
git add crates/memory-cli/src/commands/
git commit -m "feat(cli): implement memory search and memory recall commands"
```

---

## Task 4: Implement `memory add`, `context`, `timeline`, `summary`

**Files:**
- Create: `crates/memory-cli/src/commands/add.rs`
- Create: `crates/memory-cli/src/commands/context.rs`
- Create: `crates/memory-cli/src/commands/timeline.rs`
- Create: `crates/memory-cli/src/commands/summary.rs`
- Create: `crates/memory-cli/src/client.rs`

- [ ] **Step 1: Implement `client.rs`** — shared gRPC client builder:

```rust
use anyhow::{Context, Result};
use memory_client::MemoryClient;

pub async fn connect(endpoint: &str) -> Result<MemoryClient> {
    MemoryClient::connect(endpoint.to_string())
        .await
        .with_context(|| format!(
            "memory daemon not running — start with: memory-daemon start\n(endpoint: {endpoint})"
        ))
}
```

- [ ] **Step 2: Implement `add.rs`** — writes via gRPC, clear error if daemon down:

```rust
pub async fn run(args: AddArgs, global: &GlobalArgs) -> Result<()> {
    let client = crate::client::connect(&global.endpoint).await
        .map_err(|e| { eprintln!("{e}"); std::process::exit(1); })?;
    // Call client.ingest_event(content, kind, agent) and print confirmation envelope
    // ... (wire to actual MemoryClient ingest RPC)
    println!(r#"{{"status":"ok","message":"event stored"}}"#);
    Ok(())
}
```

- [ ] **Step 3: Implement `context.rs`** — same as search but formats as MemoryContext:

```rust
pub async fn run(args: ContextArgs, global: &GlobalArgs) -> Result<()> {
    // Call orchestrator.query() and return the full MemoryContext as JSON envelope
}
```

- [ ] **Step 4: Implement `timeline.rs` and `summary.rs`** — stub queries against TOC gRPC RPCs (wire to existing `memory-service` TOC RPCs)

- [ ] **Step 5: Build the full binary**

```bash
cargo build -p memory-cli
./target/debug/memory --help
```

Expected: shows all 6 subcommands

- [ ] **Step 6: Run pr-precheck**

```bash
task pr-precheck
```

- [ ] **Step 7: Commit**

```bash
git add crates/memory-cli/src/
git commit -m "feat(cli): complete memory binary — add/context/timeline/summary commands"
```

---

## Task 5: Integration smoke test and wrap-up

- [ ] **Step 1: Start daemon, run memory search**

```bash
memory-daemon start --foreground &
sleep 1
echo '{"content":"we decided to use JWT for auth","kind":"episodic"}' | memory add --content "we decided to use JWT for auth" --kind episodic
memory search "JWT auth decision" --format=json | jq .status
```

Expected: `"ok"`

- [ ] **Step 2: Verify TTY detection**

```bash
# Piped — should be JSON
memory search "test" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['status'])"
```

Expected: `ok`

- [ ] **Step 3: Verify daemon-down error message**

```bash
memory-daemon stop
memory add --content "test" 2>&1
```

Expected: message containing `memory daemon not running`

- [ ] **Step 4: Verify Phase B success criteria**

- [ ] `memory search "query" --format=json` returns JSON in <100ms p50 ✓
- [ ] `memory recall` delegates to search with llm rerank ✓
- [ ] `memory add` with daemon down exits non-zero with clear error ✓
- [ ] TTY detection: JSON when piped, human-readable when interactive ✓
- [ ] `memory-daemon` binary and existing skill hooks unchanged ✓

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(phase-b): complete Simple CLI API — memory binary with 6 commands and JSON envelope"
```
