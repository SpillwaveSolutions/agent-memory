# Phase 53: Benchmark Suite - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md + docs/superpowers/plans/2026-03-21-v3-phase-c-benchmark-suite.md)

<domain>
## Phase Boundary

This phase creates a two-part benchmark system: a custom harness for internal metrics (temporal recall, multi-session reasoning, compression efficiency) and a LOCOMO adapter for publishable, comparable scores against MemMachine and Mem0. New `crates/memory-bench/` crate with `memory-bench` binary. All reports output JSON + markdown.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- New crate `crates/memory-bench/` with `[[bin]]` producing `memory-bench` binary
- Custom harness loads TOML fixture files from `benchmarks/fixtures/`
- Runner shells out to `memory` binary (Phase 52) via `std::process::Command` — NOT in-process
- LOCOMO adapter wraps the same runner pipeline for the Snap Research dataset
- Competitor baselines stored in `benchmarks/baselines.toml` (manually-entered, not scraped)

### Sub-phase C1: Custom Harness
- Subcommands: `temporal`, `multisession`, `compression`, `all`
- TOML fixture format with `[[test]]` entries containing: id, description, setup (JSONL paths), query, expected_contains, max_tokens
- Metrics: accuracy, recall@5, token_usage (avg), latency_p50/p95, compression_ratio
- `--compare` flag reads baselines.toml and prints side-by-side table
- `--output` flag writes results.json

### Sub-phase C2: LOCOMO Adapter
- Subcommand: `locomo --dataset=./locomo-data/ --output=results.json`
- LOCOMO dataset (Snap Research, ~300-turn multi-session conversations, 4 question types)
- Dataset downloaded separately via `benchmarks/scripts/download-locomo.sh`
- `locomo-data/` in `.gitignore` — never committed
- Adapter feeds conversations through ingestion, runs 4 question types through orchestrator, scores against gold answers
- `--compare=memmachine` reads baselines.toml

### Fixture Files
- `benchmarks/fixtures/temporal-001.toml` — temporal recall tests
- `benchmarks/fixtures/multisession-001.toml` — multi-session reasoning tests
- `benchmarks/fixtures/compression-001.toml` — token compression tests
- `benchmarks/fixtures/sessions/*.jsonl` — stub session data for ingestion
- `benchmarks/baselines.toml` — MemMachine and Mem0 manually-entered scores

### CI Integration
- CI runs benchmark suite (non-blocking — not merge-blocking)
- LOCOMO skipped without `--dataset` flag (flag required to activate)
- Custom harness can run without daemon (fixture-only mode for CI)

### Report Output
- JSON report with all metrics (machine-readable)
- Markdown report with formatted table (human-readable, publishable)
- Both formats available for all benchmark types

### Claude's Discretion
- Whether runner needs daemon running or can operate in fixture-only stub mode for CI
- How to handle missing JSONL session files in fixtures (skip vs fail)
- Whether to add a `--memory-bin` flag to override binary path (useful for CI)
- Exact LOCOMO JSON schema adaptation (varies by dataset version)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec & Plans
- `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md` — Full v3.0 design spec (Phase C section)
- `docs/superpowers/plans/2026-03-21-v3-phase-c-benchmark-suite.md` — Detailed implementation plan with 7 tasks, code snippets, TOML fixtures

### Phase 52 CLI (Dependency)
- `crates/memory-cli/src/main.rs` — `memory` binary that runner shells out to
- `crates/memory-cli/src/output.rs` — `JsonEnvelope` format that runner parses

### Existing Benchmark Infrastructure
- `crates/e2e-tests/src/perf_bench.rs` — Existing perf_bench harness (v2.3, separate from this)

</canonical_refs>

<specifics>
## Specific Ideas

- The implementation plan has 7 tasks with complete Rust code snippets
- Fixture TOML format is fully specified with 3 sample fixtures
- Runner uses `std::process::Command` to shell out to `memory search --format=json`
- Scorer does case-insensitive substring matching against `expected_contains`
- Report generator produces both JSON and markdown table formats
- LOCOMO adapter has typed structs for the Snap Research JSON format
- Baseline comparison reads `benchmarks/baselines.toml` and formats side-by-side

</specifics>

<deferred>
## Deferred Ideas

- Continuous benchmark regression tracking in CI (BENCH-F01) — future milestone
- Automated dataset refresh/download — manual for now
- Side quest: positioning writeup (`docs/positioning/agent-memory-vs-competition.md`) — not a GSD phase, done alongside or after Phase C

</deferred>

---

*Phase: 53-benchmark-suite*
*Context gathered: 2026-03-22 via PRD Express Path*
