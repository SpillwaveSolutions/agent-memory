---
phase: 29-performance-benchmarks
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/e2e-tests/src/bin/perf_bench.rs
  - crates/e2e-tests/benchmarks/baseline.json
  - docs/benchmarks.md
autonomous: true

must_haves:
  truths:
    - "Benchmark harness reports ingest throughput and per-layer query latency for cold and warm runs on small and medium datasets."
    - "Benchmark output includes p50/p90/p99 table and JSON written to stdout and deterministic files."
    - "Benchmark run compares results against committed baseline and exits non-zero only for severe regressions with warning summary."
  artifacts:
    - path: "crates/e2e-tests/src/bin/perf_bench.rs"
      provides: "Benchmark harness for ingest, TOC navigation, teleport, topic graph, and route_query"
      min_lines: 200
    - path: "crates/e2e-tests/benchmarks/baseline.json"
      provides: "Committed baseline metrics and thresholds"
      contains: "baseline"
    - path: "docs/benchmarks.md"
      provides: "Usage and output conventions for perf harness"
      contains: "perf_bench"
  key_links:
    - from: "crates/e2e-tests/src/bin/perf_bench.rs"
      to: "crates/e2e-tests/benchmarks/baseline.json"
      via: "load + compare baseline"
      pattern: "baseline.*json"
    - from: "crates/e2e-tests/src/bin/perf_bench.rs"
      to: "memory_service::RetrievalHandler"
      via: "route_query benchmark"
      pattern: "route_query"
    - from: "crates/e2e-tests/src/bin/perf_bench.rs"
      to: "memory_service::VectorTeleportHandler"
      via: "vector search benchmark"
      pattern: "VectorTeleportHandler"
---

<objective>
Create a benchmark harness under crates/e2e-tests that measures ingest throughput and retrieval latency across TOC navigation, teleport (BM25/vector), topic graph, and route_query with deterministic datasets and baseline comparison.

Purpose: Establish repeatable, comparable performance baselines for core retrieval layers.
Output: perf_bench binary, baseline JSON, and benchmark usage documentation.
</objective>

<execution_context>
@/Users/richardhightower/.config/opencode/get-shit-done/workflows/execute-plan.md
@/Users/richardhightower/.config/opencode/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@crates/e2e-tests/src/lib.rs
@crates/e2e-tests/tests/pipeline_test.rs
@crates/e2e-tests/tests/bm25_teleport_test.rs
@crates/e2e-tests/tests/vector_search_test.rs
@crates/e2e-tests/tests/topic_graph_test.rs
@crates/e2e-tests/tests/multi_agent_test.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Build benchmark harness with deterministic datasets and cold/warm runs</name>
  <files>crates/e2e-tests/src/bin/perf_bench.rs</files>
  <action>
Create a new perf benchmark binary under crates/e2e-tests that:
- Accepts CLI flags for dataset tier (small|medium), run mode (cold|warm), iterations, optional trace input, and output directory.
- Generates deterministic synthetic events using a fixed seed with mixed content and multi-agent tags (single-agent + minimal multi-agent). Use separate datasets for small and medium tiers.
- Supports optional real trace input via a JSONL file of Event payloads; fall back to synthetic when not provided.
- Runs the benchmark steps in a fixed order and records per-iteration durations:
  - ingest events
  - TOC build and navigation (build_toc_segment plus at least one navigation read such as get_toc_root/get_node or storage lookups)
  - BM25 index build + BM25 search
  - Vector index build + vector search (follow vector_search_test patterns, including OnceLock for CandleEmbedder and spawn_blocking for embeddings)
  - Topic graph storage + get_top_topics/search
  - route_query (RetrievalHandler with configured BM25 + vector + topic handlers)
- Implements cold runs with a fresh TestHarness per iteration; warm runs reuse storage and indexes to measure cached access.
- Keep the implementation in the benchmark binary (no new product code paths) and reuse existing test patterns for index setup and handlers.
  </action>
  <verify>cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 3 --out-dir crates/e2e-tests/benchmarks</verify>
  <done>Running the small cold benchmark prints per-step timings for ingest, TOC, BM25, vector, topic graph, and route_query with both single-agent and multi-agent cases.</done>
</task>

<task type="auto">
  <name>Task 2: Add percentile reporting, deterministic outputs, and baseline comparison</name>
  <files>crates/e2e-tests/src/bin/perf_bench.rs, crates/e2e-tests/benchmarks/baseline.json</files>
  <action>
Extend the benchmark harness to:
- Compute p50/p90/p99 percentiles per step from iteration samples and calculate ingest throughput (events/sec).
- Emit a human-readable table and a JSON payload to stdout and to deterministic files in the output directory (for example, latest.txt and latest.json).
- Define a baseline JSON schema that includes dataset tier, mode, per-step metrics, and threshold configuration (relative + absolute) for warnings vs severe regressions.
- Load the committed baseline file by default and compare current metrics against it, emitting warnings for soft regressions and returning a non-zero exit only for severe regressions.
- Support a flag (for example --write-baseline) that writes the current run to the baseline file for initial capture or updates.
Populate crates/e2e-tests/benchmarks/baseline.json by running the harness in write-baseline mode for small/medium and cold/warm once the harness is implemented.
  </action>
  <verify>cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 3 --write-baseline --out-dir crates/e2e-tests/benchmarks</verify>
  <done>Baseline file exists with thresholds and metrics, and rerunning without --write-baseline produces a comparison summary with exit code 0 unless severe regressions are detected.</done>
</task>

<task type="auto">
  <name>Task 3: Document benchmark usage and output conventions</name>
  <files>docs/benchmarks.md</files>
  <action>
Document the performance benchmark workflow, including:
- How to run small/medium and cold/warm benchmarks, with example commands.
- Optional real-trace input format and how it overrides synthetic data.
- Output file naming/location conventions and what the table/JSON contain.
- How to update the baseline with --write-baseline and how thresholds affect exit codes.
- Note on vector model download costs and expected runtime for vector benchmarks.
  </action>
  <verify>rg "perf_bench" docs/benchmarks.md</verify>
  <done>docs/benchmarks.md describes usage, outputs, baseline update, and regression handling.</done>
</task>

</tasks>

<verification>
- cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 3 --out-dir crates/e2e-tests/benchmarks
- cargo run -p e2e-tests --bin perf_bench -- --tier small --mode warm --iterations 3 --out-dir crates/e2e-tests/benchmarks
</verification>

<success_criteria>
- perf_bench runs for small/medium tiers and cold/warm modes, producing table + JSON with p50/p90/p99 metrics.
- Baseline comparison reports warnings vs severe regressions with correct exit behavior.
- Benchmark usage and output conventions are documented in docs/benchmarks.md.
</success_criteria>

<output>
After completion, create `.planning/phases/29-performance-benchmarks/29-01-SUMMARY.md`
</output>
