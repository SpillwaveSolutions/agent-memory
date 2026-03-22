# Phase C: Benchmark Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a two-part benchmark system: a custom harness (temporal recall, multi-session reasoning, compression efficiency) that ships first, then a LOCOMO adapter that produces a score comparable to published MemMachine numbers.

**Architecture:** New `crates/memory-bench/` crate providing a `memory benchmark` subcommand group. Custom harness loads TOML fixture files, ingests setup sessions, runs queries through the `memory` CLI (Phase B), and scores results. LOCOMO adapter wraps the same pipeline for the official Snap Research dataset. Competitor baselines stored in `benchmarks/baselines.toml`. All reports output JSON + markdown.

**Tech Stack:** Rust 2021, `clap`, `serde`, `toml`, `serde_json`, `tokio`, `memory-orchestrator` (Phase A), `memory-cli` binary (Phase B), `anyhow`.

**Spec:** `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md` — Phase C section

**Prerequisite:** Phase A and Phase B complete. `memory` binary available in `$PATH` or `./target/debug/memory`.

---

## File Map

**New crate:** `crates/memory-bench/`

| File | Responsibility |
|------|----------------|
| `crates/memory-bench/Cargo.toml` | Crate manifest |
| `crates/memory-bench/src/main.rs` | Binary entrypoint: `memory benchmark` subcommands |
| `crates/memory-bench/src/cli.rs` | Clap structs for benchmark subcommands |
| `crates/memory-bench/src/fixture.rs` | Load + validate TOML fixture files |
| `crates/memory-bench/src/runner.rs` | Ingest sessions, run queries, collect raw results |
| `crates/memory-bench/src/scorer.rs` | Score raw results: accuracy, recall@k, latency |
| `crates/memory-bench/src/report.rs` | JSON + markdown report generation |
| `crates/memory-bench/src/locomo.rs` | LOCOMO dataset loader + adapter |
| `crates/memory-bench/src/baseline.rs` | Load `benchmarks/baselines.toml`, format comparison table |

**New data files:**

| Path | Responsibility |
|------|----------------|
| `benchmarks/fixtures/temporal-001.toml` | Temporal recall fixture |
| `benchmarks/fixtures/multisession-001.toml` | Multi-session reasoning fixture |
| `benchmarks/fixtures/compression-001.toml` | Token compression fixture |
| `benchmarks/baselines.toml` | Manually-entered competitor scores |
| `benchmarks/scripts/download-locomo.sh` | LOCOMO dataset download script |
| `.gitignore` additions | `locomo-data/` excluded from repo |

**Modified files:**

| File | Change |
|------|--------|
| `Cargo.toml` (workspace root) | Add `crates/memory-bench` to `members` |

---

## Task 1: Scaffold `memory-bench` crate and data files

**Files:**
- Create: `crates/memory-bench/Cargo.toml`
- Create: `crates/memory-bench/src/main.rs`
- Create: `benchmarks/baselines.toml`
- Create: `benchmarks/scripts/download-locomo.sh`
- Modify: `Cargo.toml` (workspace root), `.gitignore`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "memory-bench"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "memory-bench"
path = "src/main.rs"

[dependencies]
clap = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 2: Add to workspace `Cargo.toml`** — add `"crates/memory-bench"` to `members`

- [ ] **Step 3: Create `benchmarks/baselines.toml`**

```toml
# Manually-maintained competitor benchmark scores.
# Update these from published blog posts / papers.
# Sources listed per entry.

[memmachine]
# Source: https://memmachine.ai/blog/2025/12/memmachine-v0.2-delivers-top-scores-and-efficiency-on-locomo-benchmark/
locomo_score = 0.91
token_reduction = 0.80
latency_improvement = 0.75

[mem0]
# Source: https://mem0.ai/research
accuracy_vs_openai_memory = 0.26
token_reduction = 0.90
latency_reduction = 0.91
```

- [ ] **Step 4: Create `benchmarks/scripts/download-locomo.sh`**

```bash
#!/usr/bin/env bash
# Download the LOCOMO benchmark dataset from Snap Research.
# Dataset: https://snap-research.github.io/locomo/
# License: verify terms at the above URL before publishing scores.
set -euo pipefail

DEST="${1:-locomo-data}"
mkdir -p "$DEST"

echo "Downloading LOCOMO dataset to $DEST ..."
# Update URL when Snap Research publishes stable release artifact
curl -L "https://snap-research.github.io/locomo/data/locomo_v1.zip" -o "$DEST/locomo_v1.zip"
unzip -q "$DEST/locomo_v1.zip" -d "$DEST"
echo "Done. Dataset at: $DEST"
echo "NOTE: Verify license terms at https://snap-research.github.io/locomo/ before publishing scores."
```

```bash
chmod +x benchmarks/scripts/download-locomo.sh
```

- [ ] **Step 5: Add `locomo-data/` to `.gitignore`**

```
# LOCOMO benchmark dataset — download separately via benchmarks/scripts/download-locomo.sh
locomo-data/
```

- [ ] **Step 6: Verify crate compiles**

```bash
cargo build -p memory-bench
```

- [ ] **Step 7: Commit**

```bash
git add crates/memory-bench/ benchmarks/ .gitignore Cargo.toml Cargo.lock
git commit -m "feat(bench): scaffold memory-bench crate and baseline data files"
```

---

## Task 2: Define fixture format and loader

**Files:**
- Create: `crates/memory-bench/src/fixture.rs`
- Create: `benchmarks/fixtures/temporal-001.toml`
- Create: `benchmarks/fixtures/multisession-001.toml`
- Create: `benchmarks/fixtures/compression-001.toml`

- [ ] **Step 1: Write fixture loader tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    const FIXTURE_TOML: &str = r#"
[[test]]
id = "temporal-001"
description = "recall decision from prior session"
setup = ["session-a.jsonl"]
query = "what auth approach did we decide on?"
expected_contains = ["JWT", "stateless"]
max_tokens = 500
"#;

    #[test]
    fn test_fixture_parses_valid_toml() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", FIXTURE_TOML).unwrap();
        let fixture = Fixture::load(f.path()).unwrap();
        assert_eq!(fixture.tests.len(), 1);
        assert_eq!(fixture.tests[0].id, "temporal-001");
    }

    #[test]
    fn test_fixture_validates_required_fields() {
        let bad = r#"[[test]]
id = ""
"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", bad).unwrap();
        assert!(Fixture::load(f.path()).is_err());
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

```bash
cargo test -p memory-bench fixture
```

- [ ] **Step 3: Implement `fixture.rs`**

```rust
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Fixture {
    #[serde(rename = "test")]
    pub tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestCase {
    pub id: String,
    pub description: String,
    /// Paths to JSONL session files to ingest before querying.
    pub setup: Vec<String>,
    pub query: String,
    /// Result must contain at least one of these strings (case-insensitive).
    pub expected_contains: Vec<String>,
    pub max_tokens: usize,
}

impl Fixture {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let fixture: Fixture = toml::from_str(&content)?;
        for t in &fixture.tests {
            if t.id.is_empty() {
                bail!("test case has empty id in {}", path.display());
            }
            if t.query.is_empty() {
                bail!("test '{}' has empty query", t.id);
            }
        }
        Ok(fixture)
    }

    pub fn load_dir(dir: &Path) -> Result<Vec<TestCase>> {
        let mut all = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().map(|e| e == "toml").unwrap_or(false) {
                let fixture = Fixture::load(&entry.path())?;
                all.extend(fixture.tests);
            }
        }
        Ok(all)
    }
}
```

- [ ] **Step 4: Create fixture files**

`benchmarks/fixtures/temporal-001.toml`:
```toml
[[test]]
id = "temporal-001"
description = "Recall an architectural decision made in a prior session"
setup = ["sessions/auth-decision.jsonl"]
query = "what authentication approach did we decide on?"
expected_contains = ["JWT", "token"]
max_tokens = 500

[[test]]
id = "temporal-002"
description = "Recall a specific bug fix from two sessions ago"
setup = ["sessions/bug-fix.jsonl", "sessions/follow-up.jsonl"]
query = "how did we fix the null pointer exception?"
expected_contains = ["null check", "Option"]
max_tokens = 400
```

`benchmarks/fixtures/multisession-001.toml`:
```toml
[[test]]
id = "multi-001"
description = "Connect a decision from session A with an outcome from session B"
setup = ["sessions/session-a.jsonl", "sessions/session-b.jsonl", "sessions/session-c.jsonl"]
query = "what was the outcome of the approach we chose last week?"
expected_contains = ["performance", "latency"]
max_tokens = 600
```

`benchmarks/fixtures/compression-001.toml`:
```toml
[[test]]
id = "compress-001"
description = "Verify context is compressed vs raw session dump"
setup = ["sessions/long-session.jsonl"]
query = "summarize the key decisions from this project"
expected_contains = ["decision", "architecture"]
max_tokens = 800
```

Also create stub JSONL session files in `benchmarks/fixtures/sessions/`:
```jsonl
{"role":"user","content":"We should use JWT for our auth system because it's stateless"}
{"role":"assistant","content":"Agreed. JWT gives us stateless auth which scales horizontally."}
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cargo test -p memory-bench fixture
```

- [ ] **Step 6: Commit**

```bash
git add crates/memory-bench/src/fixture.rs benchmarks/fixtures/
git commit -m "feat(bench): add fixture format, loader, and sample test cases"
```

---

## Task 3: Implement runner and scorer

**Files:**
- Create: `crates/memory-bench/src/runner.rs`
- Create: `crates/memory-bench/src/scorer.rs`

- [ ] **Step 1: Write scorer tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_hit_when_expected_present() {
        let result = "We chose JWT for stateless auth";
        let expected = &["JWT".to_string(), "token".to_string()];
        assert!(score_result(result, expected));
    }

    #[test]
    fn test_score_miss_when_none_present() {
        let result = "We chose sessions with cookies";
        let expected = &["JWT".to_string(), "token".to_string()];
        assert!(!score_result(result, expected));
    }

    #[test]
    fn test_accuracy_all_hits() {
        let results = vec![true, true, true];
        assert!((compute_accuracy(&results) - 1.0).abs() < 0.001);
    }
}
```

- [ ] **Step 2: Implement `scorer.rs`**

```rust
/// Returns true if the result text contains at least one expected string (case-insensitive).
pub fn score_result(result: &str, expected_contains: &[String]) -> bool {
    let lower = result.to_lowercase();
    expected_contains.iter().any(|e| lower.contains(&e.to_lowercase()))
}

pub fn compute_accuracy(hits: &[bool]) -> f64 {
    if hits.is_empty() { return 0.0; }
    hits.iter().filter(|&&h| h).count() as f64 / hits.len() as f64
}

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkReport {
    pub accuracy: f64,
    pub recall_at_5: f64,
    pub token_usage_avg: usize,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub compression_ratio: f64,
    pub test_count: usize,
    pub pass_count: usize,
}
```

- [ ] **Step 3: Implement `runner.rs`** — calls `memory search` via `std::process::Command`:

```rust
use std::process::Command;
use std::time::Instant;

pub struct QueryResult {
    pub raw_output: String,
    pub latency_ms: u64,
    pub tokens_estimated: usize,
    pub success: bool,
}

pub fn run_query(query: &str, memory_bin: &str) -> QueryResult {
    let start = Instant::now();
    let output = Command::new(memory_bin)
        .args(["search", query, "--format=json"])
        .output()
        .expect("failed to run memory binary");

    let latency_ms = start.elapsed().as_millis() as u64;
    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    let tokens_estimated = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v["meta"]["tokens_estimated"].as_u64())
        .unwrap_or(0) as usize;

    QueryResult {
        raw_output: raw,
        latency_ms,
        tokens_estimated,
        success: output.status.success(),
    }
}
```

- [ ] **Step 4: Run scorer tests**

```bash
cargo test -p memory-bench scorer
```

- [ ] **Step 5: Commit**

```bash
git add crates/memory-bench/src/runner.rs crates/memory-bench/src/scorer.rs
git commit -m "feat(bench): add benchmark runner (shells out to memory binary) and scorer"
```

---

## Task 4: Implement report generation and baseline comparison

**Files:**
- Create: `crates/memory-bench/src/report.rs`
- Create: `crates/memory-bench/src/baseline.rs`

- [ ] **Step 1: Implement `baseline.rs`**

```rust
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Baselines {
    pub memmachine: Option<CompetitorScore>,
    pub mem0: Option<CompetitorScore>,
}

#[derive(Debug, Deserialize)]
pub struct CompetitorScore {
    pub locomo_score: Option<f64>,
    pub token_reduction: Option<f64>,
    pub latency_improvement: Option<f64>,
    pub accuracy_vs_openai_memory: Option<f64>,
    pub latency_reduction: Option<f64>,
}

impl Baselines {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
```

- [ ] **Step 2: Implement `report.rs`** — markdown table generation:

```rust
use crate::scorer::BenchmarkReport;

pub fn to_markdown(report: &BenchmarkReport, compare: Option<(&str, f64)>) -> String {
    let mut md = String::new();
    md.push_str("## Benchmark Results\n\n");
    md.push_str(&format!("| Metric | Agent-Memory |"));
    if let Some((name, _)) = compare {
        md.push_str(&format!(" {} |", name));
    }
    md.push('\n');
    md.push_str("|--------|-------------|");
    if compare.is_some() { md.push_str("----------|"); }
    md.push('\n');
    md.push_str(&format!("| Accuracy | {:.1}% |", report.accuracy * 100.0));
    if let Some((_, score)) = compare {
        md.push_str(&format!(" {:.1}% |", score * 100.0));
    }
    md.push('\n');
    md.push_str(&format!("| Recall@5 | {:.2} |\n", report.recall_at_5));
    md.push_str(&format!("| Avg tokens | {} |\n", report.token_usage_avg));
    md.push_str(&format!("| Latency p50 | {}ms |\n", report.latency_p50_ms));
    md
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/memory-bench/src/report.rs crates/memory-bench/src/baseline.rs
git commit -m "feat(bench): add markdown report and baseline comparison"
```

---

## Task 5: Wire `memory benchmark` CLI commands (C1)

**Files:**
- Create: `crates/memory-bench/src/cli.rs`
- Modify: `crates/memory-bench/src/main.rs`

- [ ] **Step 1: Implement CLI and wire subcommands**

```rust
// main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "memory-bench")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run temporal recall benchmark
    Temporal { #[arg(long, default_value = "benchmarks/fixtures")] fixtures: String },
    /// Run multi-session reasoning benchmark
    Multisession { #[arg(long, default_value = "benchmarks/fixtures")] fixtures: String },
    /// Run compression efficiency benchmark
    Compression { #[arg(long, default_value = "benchmarks/fixtures")] fixtures: String },
    /// Run full custom suite
    All {
        #[arg(long, default_value = "benchmarks/fixtures")] fixtures: String,
        #[arg(long)] output: Option<String>,
        #[arg(long)] compare: Option<String>,
    },
    /// Run LOCOMO adapter benchmark
    Locomo {
        #[arg(long)] dataset: String,
        #[arg(long)] output: Option<String>,
        #[arg(long)] compare: Option<String>,
    },
}
```

- [ ] **Step 2: Run help to verify**

```bash
cargo run -p memory-bench -- --help
```

Expected: shows `temporal`, `multisession`, `compression`, `all`, `locomo`

- [ ] **Step 3: Run full custom suite smoke test**

```bash
cargo run -p memory-bench -- all --fixtures benchmarks/fixtures 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/memory-bench/src/
git commit -m "feat(bench): wire memory-bench CLI with all subcommands"
```

---

## Task 6: Implement LOCOMO adapter (C2)

**Files:**
- Create: `crates/memory-bench/src/locomo.rs`

- [ ] **Step 1: Write LOCOMO adapter test (with fixture data)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locomo_conversation_parses() {
        let json = r#"{"conversation_id":"c1","turns":[{"role":"user","content":"What is your name?"},{"role":"assistant","content":"I am an AI."}],"questions":[{"question":"What did the user ask?","answer":"name","type":"single_hop"}]}"#;
        let conv: LocomoConversation = serde_json::from_str(json).unwrap();
        assert_eq!(conv.questions.len(), 1);
    }
}
```

- [ ] **Step 2: Implement `locomo.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct LocomoConversation {
    pub conversation_id: String,
    pub turns: Vec<Turn>,
    pub questions: Vec<Question>,
}

#[derive(Debug, Deserialize)]
pub struct Turn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct Question {
    pub question: String,
    pub answer: String,
    #[serde(rename = "type")]
    pub question_type: String, // single_hop | multi_hop | temporal | open_domain
}

#[derive(Debug, Serialize)]
pub struct LocomoResult {
    pub conversation_id: String,
    pub total_questions: usize,
    pub correct: usize,
    pub score: f64,
    pub by_type: std::collections::HashMap<String, f64>,
}

/// Load conversations from the LOCOMO dataset directory.
pub fn load_dataset(dir: &Path) -> Result<Vec<LocomoConversation>> {
    let mut conversations = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            let conv: LocomoConversation = serde_json::from_str(&content)?;
            conversations.push(conv);
        }
    }
    Ok(conversations)
}
```

- [ ] **Step 3: Run LOCOMO tests**

```bash
cargo test -p memory-bench locomo
```

- [ ] **Step 4: Commit**

```bash
git add crates/memory-bench/src/locomo.rs
git commit -m "feat(bench): add LOCOMO adapter with conversation loader and scorer"
```

---

## Task 7: Final QA and Phase C wrap-up

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace --all-features
```

- [ ] **Step 2: Run pr-precheck**

```bash
task pr-precheck
```

- [ ] **Step 3: Verify custom harness runs end-to-end**

```bash
cargo run -p memory-bench -- all --fixtures benchmarks/fixtures --output results.json
cat results.json | jq .accuracy
```

- [ ] **Step 4: Verify `--compare` reads baselines**

```bash
cargo run -p memory-bench -- all --fixtures benchmarks/fixtures --compare memmachine
```

Expected: side-by-side table in output

- [ ] **Step 5: Verify locomo-data is gitignored**

```bash
mkdir -p locomo-data && git status | grep locomo
```

Expected: `locomo-data/` not tracked

- [ ] **Step 6: Verify Phase C success criteria**

- [ ] Custom benchmark suite runs end-to-end with fixture files ✓
- [ ] LOCOMO adapter loads dataset and produces aggregate score ✓
- [ ] `--compare=memmachine` reads baselines.toml ✓
- [ ] `locomo-data/` confirmed in `.gitignore` ✓
- [ ] CI runs benchmark suite non-blocking (skips LOCOMO without `--dataset`) ✓
- [ ] All code passes `task pr-precheck` ✓

- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "feat(phase-c): complete Benchmark Suite — custom harness + LOCOMO adapter"
```

---

## After Phase C: Side Quest

Create `docs/positioning/agent-memory-vs-competition.md` with:
- Head-to-head table (Agent-Memory vs Mem0 vs MemMachine, 6 dimensions)
- LOCOMO score from `results.json` filled in
- "Beyond RAG: Cognitive Memory Architecture" narrative
- Publishable as blog post with minor editing
