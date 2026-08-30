# Phase 55: Performance Truth - Context

**Gathered:** 2026-08-30
**Status:** In execution
**Source:** docs/plans/v3.1-make-it-true-plan.md

## Phase Boundary

Make recorded perf numbers support — rather than contradict — the core value
claim. Measurement methodology must survive scrutiny. No new retrieval
capabilities.

## Root cause (55-01) — confirmed in code

`crates/e2e-tests/src/bin/perf_bench.rs` times **setup + query** under query
step names:

- `*.toc` wraps `build_toc_segment` (MockSummarizer rollup of the whole
  corpus) plus two `get_toc_node` lookups. That is the 64.6s "TOC navigation"
  number. Real navigation is the lookups.
- `*.vector` wraps Candle embed + HNSW index build plus one search. That is
  the 7.2s "vector" number.
- `*.bm25` wraps Tantivy index build plus one search (~245ms).
- `*.route_query` already times only the RPC (~2.3ms) — the honest query path.

## Decisions

- Split every step into `*_build`/`*_index` (setup, ingest-time) vs query.
- Default query iterations = 30. p90 only if n≥10; p99 only if n≥30; otherwise
  min/median/max and say so.
- Warm: setup once, warmup one query, then N query samples.
- Cold: new harness per iteration; still split setup vs query timers.
- `vector_model_load` is a one-shot setup metric, never folded into query.
- Re-baseline `baseline.json` schema version 2; rewrite `docs/benchmarks.md`.
