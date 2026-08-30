---
phase: 56-honest-benchmarks
verified: 2026-08-30
status: passed
---

# Phase 56: Honest Benchmarks Verification

**Phase Goal:** a benchmark story that survives hostile review — or no story at all.

## Execution evidence

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | recall@k ≠ accuracy | RUN | `compute_recall_at_k` unit test; committed `custom-harness-mock.json`: accuracy=0.88, recall_at_k=0.86 |
| 2 | compression uses file contents | UNIT | `estimate_raw_tokens_reads_file_contents_not_paths`; report notes "raw = setup file *contents*" |
| 3 | CLI failures abort | UNIT | `cli_ingest_fails_loud_when_binary_missing` |
| 4 | Per-test / per-conversation isolation | UNIT + RUN | `mock_stores_do_not_bleed`; `isolation_no_cross_conversation_bleed`; smoke `isolation` field |
| 5 | ≥25 fixtures | RUN | 25 tests in `custom-harness-mock.json`; `committed_fixtures_are_at_least_25` |
| 6 | Real locomo10.json schema | UNIT | `real_schema_parses_including_numeric_answer`; `invented_v1_schema_is_rejected` |
| 7 | Substring is not a LOCOMO score | RUN | smoke `metric=context_hit_rate`; `--compare` exits non-zero for mock scorer |
| 8 | CI smoke executes the pipeline | CODE + RUN | `memory-bench smoke` writes results; local run 1 conversation / 4 questions |
| 9 | Download URL is GitHub locomo10.json | CODE | `benchmarks/scripts/download-locomo.sh` fetches LICENSE.txt then data/locomo10.json |

## Committed artifacts

- [`benchmarks/results/custom-harness-mock.json`](../../../benchmarks/results/custom-harness-mock.json) — 25 tests, backend=mock, 22/25, failed=`compress-001,compress-003,multi-004`. **Not a production quality number.**
- [`benchmarks/results/locomo-smoke.json`](../../../benchmarks/results/locomo-smoke.json) — 1 conversation, `metric=context_hit_rate`, overall_score=0.5 (2/4). **Not a LOCOMO score.**

## Decision gate

**HOLD comparison marketing.** No `locomo_llm_judge` artifact. Full `locomo10.json` + API judge was not run (no key). README must not claim a LOCOMO score.

## Human verification (blockers)

- [x] Committed results JSON produced by actually running the adapter
- [x] Metric field is `context_hit_rate`, not an unlabeled LOCOMO score
- [x] `--compare` refused for mock scorer
