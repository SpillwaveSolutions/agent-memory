---
phase: 53-benchmark-suite
verified: 2026-03-22T00:00:00Z
status: passed
score: 17/17 must-haves verified
re_verification: false
gaps: []
human_verification:
  - test: "Run memory-bench all against a live memory daemon"
    expected: "Fixture ingestion, query, scoring, and markdown report output with real latency values"
    why_human: "Requires running memory daemon; CI smoke test is --help only"
  - test: "Run memory-bench locomo --dataset=./locomo-data/ after downloading the LOCOMO dataset"
    expected: "Conversations ingested, questions scored, aggregate JSON printed with overall_score and per-type breakdown"
    why_human: "LOCOMO dataset not committed; requires manual download via download-locomo.sh"
---

# Phase 53: Benchmark Suite Verification Report

**Phase Goal:** Users can measure and compare Agent Memory retrieval quality with reproducible benchmarks and a publishable LOCOMO score
**Verified:** 2026-03-22
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | memory-bench crate compiles as part of the workspace | VERIFIED | `cargo build -p memory-bench` succeeds; "crates/memory-bench" in root Cargo.toml |
| 2 | TOML fixture files parse into typed Rust structs | VERIFIED | `fixture.rs` defines `Fixture`/`TestCase`; 4 passing tests including `test_fixture_parses_valid_toml` |
| 3 | Fixture loader validates required fields and rejects invalid fixtures | VERIFIED | `Fixture::load` bails on empty id or empty query; tests `test_fixture_validates_empty_id` and `test_fixture_validates_empty_query` pass |
| 4 | locomo-data/ is gitignored | VERIFIED | `locomo-data/` entry confirmed in `.gitignore` |
| 5 | Baseline competitor scores are stored in benchmarks/baselines.toml | VERIFIED | `[memmachine]` with `locomo_score = 0.91` and `[mem0]` with scores present |
| 6 | Runner shells out to memory binary and captures JSON output + latency | VERIFIED | `runner.rs` uses `Command::new(&config.memory_bin).args(["search", query, "--format=json"])`, captures elapsed time, parses `meta.tokens_estimated` |
| 7 | Scorer computes accuracy and recall@5 from expected_contains matching | VERIFIED | `scorer.rs` exports `score_result`, `compute_accuracy`, `compute_recall_at_k`; 11 passing unit tests |
| 8 | Scorer computes compression_ratio from context_tokens vs raw_tokens | VERIFIED | `compute_compression_ratio(context_tokens, raw_tokens)` and `estimate_raw_tokens` implemented; tests pass |
| 9 | Report generates both JSON and markdown table formats | VERIFIED | `report.rs` exports `to_json` and `to_markdown`; round-trip test and header test pass |
| 10 | Baseline loader reads benchmarks/baselines.toml into typed structs | VERIFIED | `baseline.rs` defines `Baselines`/`CompetitorScore`; `test_baselines_load` asserts `memmachine.locomo_score = Some(0.91)` |
| 11 | CLI exposes temporal, multisession, compression, all, and locomo subcommands | VERIFIED | `cargo run -p memory-bench -- --help` shows all 5 subcommands; clap definitions in `cli.rs` |
| 12 | --compare flag reads baselines and prints side-by-side table | VERIFIED | `all` and `locomo` subcommands both have `--compare` flag; `to_markdown` generates 4-column table when `baselines` is `Some` |
| 13 | LOCOMO JSON dataset files parse into typed Rust structs | VERIFIED | `locomo.rs` defines `LocomoConversation`, `Turn`, `Question`; `test_locomo_conversation_parses` and `test_locomo_conversation_multiple_types` pass |
| 14 | LOCOMO adapter loads conversations with 4 question types | VERIFIED | `question_type` field supports single_hop, multi_hop, temporal, open_domain; test verifies all 4 types parse |
| 15 | LOCOMO results include per-type scores and aggregate score | VERIFIED | `LocomoResult.by_type` HashMap + `LocomoAggregateResult.by_type`; `test_aggregate_results` verifies per-type sums |
| 16 | CI can run custom harness without LOCOMO (locomo subcommand requires --dataset flag) | VERIFIED | `--dataset` is required arg (no default); `--help` checks only in CI smoke test |
| 17 | CI runs benchmark smoke test as non-blocking step (continue-on-error: true) | VERIFIED | `benchmark-smoke` job at line 183 of ci.yml with `continue-on-error: true` and `needs: [test]` |

**Score:** 17/17 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/memory-bench/Cargo.toml` | Crate manifest with workspace deps | VERIFIED | Contains "memory-bench"; clap, tokio, serde, toml, thiserror, anyhow, tracing wired |
| `crates/memory-bench/src/fixture.rs` | TOML fixture loader with validation | VERIFIED | `Fixture`, `TestCase` exported; `load` validates id/query; `load_dir` sorts entries |
| `crates/memory-bench/src/scorer.rs` | Result scoring functions + BenchmarkReport | VERIFIED | `score_result`, `compute_accuracy`, `compute_recall_at_k`, `percentile`, `compute_compression_ratio`, `estimate_raw_tokens`, `BenchmarkReport` all present |
| `crates/memory-bench/src/runner.rs` | Shell-out runner for memory binary | VERIFIED | `run_query` uses `std::process::Command`; `ingest_session` reads JSONL; `QueryResult` captures latency + tokens |
| `crates/memory-bench/src/report.rs` | JSON + markdown report generation | VERIFIED | `to_json` round-trips; `to_markdown` generates single-column or 4-column comparison table |
| `crates/memory-bench/src/baseline.rs` | Competitor baseline TOML loader | VERIFIED | `Baselines`, `CompetitorScore` defined; `load` reads TOML from path |
| `crates/memory-bench/src/cli.rs` | Clap CLI definition with all subcommands | VERIFIED | `Cli`, `Commands` with Temporal/Multisession/Compression/All/Locomo variants |
| `crates/memory-bench/src/locomo.rs` | LOCOMO dataset loader and scorer | VERIFIED | `LocomoConversation`, `Turn`, `Question`, `LocomoResult`, `LocomoAggregateResult`; `load_dataset`, `score_conversation`, `aggregate_results` all implemented with 6 tests |
| `crates/memory-bench/src/main.rs` | Full pipeline wiring | VERIFIED | All 5 CLI commands dispatch to correct modules; `run_category`, `run_all`, `run_tests` implement the scoring pipeline end-to-end |
| `crates/memory-bench/src/lib.rs` | Module declarations | VERIFIED | Exports `baseline`, `fixture`, `locomo`, `report`, `runner`, `scorer` |
| `benchmarks/fixtures/temporal-001.toml` | Temporal recall test fixtures | VERIFIED | Contains `[[test]]` with id temporal-001 and temporal-002 |
| `benchmarks/fixtures/multisession-001.toml` | Multi-session test fixtures | VERIFIED | Contains `[[test]]` with id multi-001 |
| `benchmarks/fixtures/compression-001.toml` | Compression test fixtures | VERIFIED | Contains `[[test]]` with id compress-001 |
| `benchmarks/fixtures/sessions/*.jsonl` | 7 JSONL session stub files | VERIFIED | auth-decision (6 lines), bug-fix, follow-up, session-a/b/c, long-session (30 lines) all present |
| `benchmarks/baselines.toml` | Competitor baseline scores | VERIFIED | `[memmachine]` locomo_score=0.91; `[mem0]` accuracy_vs_openai_memory=0.26 |
| `benchmarks/scripts/download-locomo.sh` | LOCOMO download script | VERIFIED | Executable (`chmod +x`); curl + unzip pipeline present |
| `.github/workflows/ci.yml` | Non-blocking benchmark CI step | VERIFIED | `benchmark-smoke` job with `continue-on-error: true`, `needs: [test]`, runs --help only |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `fixture.rs` | `benchmarks/fixtures/*.toml` | `toml::from_str` in `Fixture::load` | WIRED | `Fixture::load` and `Fixture::load_dir` both call `toml::from_str`; tests use temp fixtures |
| `Cargo.toml` (root) | `crates/memory-bench` | workspace members | WIRED | `"crates/memory-bench"` present in members array |
| `runner.rs` | memory binary | `Command::new(&config.memory_bin).args(["search", ...])` | WIRED | Line 34-36 of runner.rs; graceful fallback when binary not found |
| `scorer.rs` | `runner.rs` | `score_result` called on `result.raw_output` | WIRED | `main.rs` line 144: `scorer::score_result(&result.raw_output, &test.expected_contains)` |
| `scorer.rs` | fixture setup paths | `compute_compression_ratio` uses `estimate_raw_tokens(&test.setup)` | WIRED | `main.rs` line 150: `scorer::estimate_raw_tokens(&test.setup)` |
| `report.rs` | `scorer.rs` | `BenchmarkReport` struct passed to `to_json`/`to_markdown` | WIRED | `report.rs` imports `crate::scorer::BenchmarkReport` |
| `main.rs` | `cli.rs` | Clap Parser dispatch via `Commands::` | WIRED | `main.rs` line 2-3: `mod cli; use cli::Cli; cli::Cli::parse()` |
| `locomo.rs` | JSON dataset files | `serde_json::from_str` in `load_dataset` | WIRED | `locomo.rs` line 97; `test_load_dataset_from_dir` confirms with temp dir |
| `main.rs` | `locomo.rs` | `Commands::Locomo` dispatch | WIRED | `main.rs` line 53: `locomo::load_dataset(...)`, `locomo::score_conversation(...)`, `locomo::aggregate_results(...)` |
| `ci.yml` | `cargo run -p memory-bench` | benchmark-smoke job | WIRED | Lines 204-211 of ci.yml: build + help checks for --help, all --help, locomo --help |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BENCH-01 | 53-01-PLAN.md | Custom benchmark harness with TOML fixture files (temporal, multisession, compression) | SATISFIED | `fixture.rs` + 3 `.toml` fixture files + 7 session stubs verified on disk |
| BENCH-02 | 53-02-PLAN.md | `memory benchmark temporal/multisession/compression/all` subcommands | SATISFIED | CLI exposes all subcommands; `cargo run -- --help` confirms all present |
| BENCH-03 | 53-02-PLAN.md | Benchmark reports accuracy, recall@5, token_usage, latency_p50/p95, compression ratio | SATISFIED | `BenchmarkReport` struct and all 6 metric fields verified in `scorer.rs` |
| BENCH-04 | 53-03-PLAN.md | LOCOMO adapter ingests Snap Research dataset and produces results.json with aggregate score | SATISFIED | `locomo.rs` with `load_dataset`, `score_conversation`, `aggregate_results`; JSON output in main.rs |
| BENCH-05 | 53-02-PLAN.md | --compare flag reads benchmarks/baselines.toml and prints side-by-side competitor table | SATISFIED | `--compare` flag on `all` and `locomo` subcommands; `to_markdown` generates 4-column table |
| BENCH-06 | 53-01-PLAN.md | locomo-data/ in .gitignore — dataset never committed | SATISFIED | `locomo-data/` confirmed in `.gitignore` |
| BENCH-07 | 53-03-PLAN.md | CI runs benchmark suite (non-blocking, skips LOCOMO without --dataset flag) | SATISFIED | `benchmark-smoke` CI job with `continue-on-error: true`; --dataset is required arg with no default |
| BENCH-08 | 53-02-PLAN.md | JSON + markdown report output for all benchmark types | SATISFIED | `to_json` and `to_markdown` in `report.rs`; both invoked from main.rs for all subcommands |

All 8 requirements mapped. No orphaned requirements detected.

### Anti-Patterns Found

No anti-patterns detected in any memory-bench source file. No TODO/FIXME/placeholder comments, no empty return stubs, no console-only handlers.

### Human Verification Required

#### 1. Live Benchmark Run Against Memory Daemon

**Test:** Start memory daemon, then run `cargo run -p memory-bench -- all --fixtures benchmarks/fixtures`
**Expected:** Fixture JSONL sessions ingest via `memory add`, queries execute via `memory search --format=json`, BenchmarkReport printed as markdown table with real latency/token values
**Why human:** Requires running memory daemon; CI smoke test only verifies --help output, not actual benchmark execution

#### 2. LOCOMO Dataset Benchmark

**Test:** Download LOCOMO dataset via `benchmarks/scripts/download-locomo.sh`, then run `cargo run -p memory-bench -- locomo --dataset=./locomo-data/ --compare`
**Expected:** Conversations loaded and ingested, questions scored against memory search results, aggregate JSON with `overall_score` and per-type breakdown (single_hop, multi_hop, temporal, open_domain) printed; comparison table shows Agent-Memory vs MemMachine vs Mem0
**Why human:** LOCOMO dataset is gitignored and requires separate download; publishable score depends on real retrieval quality

### Gaps Summary

No gaps. All 17 must-have truths verified. All 8 requirement IDs (BENCH-01 through BENCH-08) are satisfied with concrete implementation evidence. All key links are wired. 24 unit tests pass. Clippy passes with no warnings.

The two human verification items are not blockers — they require external runtime dependencies (memory daemon, LOCOMO dataset) that cannot be verified programmatically.

---

_Verified: 2026-03-22_
_Verifier: Claude (gsd-verifier)_
