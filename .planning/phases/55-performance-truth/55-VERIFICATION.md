---
phase: 55-performance-truth
verified: 2026-08-30
status: passed
---

# Phase 55: Performance Truth Verification

**Phase Goal:** recorded perf numbers support the core value claim; methodology survives scrutiny.

## Execution evidence

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | 64.6s "TOC navigation" was rollup | RUN | `single.toc_build` p50 = 76714 ms on 240 events; old `single.toc` was this timer |
| 2 | Query `*.toc` is navigation | RUN | `single.toc` p50 = **0.13 ms**, n=30, medium/warm |
| 3 | Vector model load not in `*.vector` | RUN | `vector_model_load` 156 ms; `vector_index` 12.6 s; `vector` 4.15 s (query embed) |
| 4 | p90/p99 withheld below 10/30 samples | UNIT + RUN | unit tests; setup steps in latest.json omit p90/p99 (samples=1) |
| 5 | Warm vs cold are different loops | CODE | Warm: one setup + N queries; cold: new store per iteration |
| 6 | docs name corpus, samples, caveats | DOCS | `docs/benchmarks.md` committed-result table |
| 7 | Re-baselined latest.json / baseline.json | RUN | schema 2, 2026-08-30T17:30:20Z, linux/x86_64, 240 events, 30 samples |

## Human verification (blockers)

- [x] Committed `latest.json` from a real medium/warm/30 run
- [x] `single.toc` warm p50 < 500ms (0.13 ms)
- [x] Vector split: model load vs index vs query
