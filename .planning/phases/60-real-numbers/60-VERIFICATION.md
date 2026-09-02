---
phase: 60-real-numbers
verified: 2026-09-02
status: 60-01-verified; 60-03-executing
---

# Phase 60: Real Numbers Verification

## 60-01 (PR #49)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `--backend cli` isolation is per-conversation daemon | UNIT | `isolation_default_label`; locomo result `isolation` field |
| 2 | Two isolated daemons do not bleed | RUN | `cli_isolated_daemons_do_not_bleed` under `MEMORY_BENCH_LIVE=1` |
| 3 | Drain wait polls checkpoints | UNIT | `drain_caught_up_*`; `parse_checkpoint_json_roundtrip` |
| 4 | No `std::thread::sleep` in cli-backend path | CODE | `poll_pause` uses `recv_timeout` |
| 5 | CI `bench-cli-smoke` | CI | `.github/workflows/ci.yml` job after `build` |
| 6 | `drain_wait_ms` per conversation | CODE | `LocomoConversationResult.drain_wait_ms` |
| 7 | Query reader sees indexer commits | CODE | `TeleportSearcher::search` calls `reload()` |
| 8 | Sequence 0 advances BM25 checkpoint | UNIT | `test_process_batch_sequence_zero_advances_checkpoint` |

## 60-03 (this branch)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ≥15 semantic tests; hit files share no query tokens | UNIT | `semantic_hits_do_not_contain_query_tokens` |
| 2 | BM25 recall@5 < 0.4 on the paraphrase set | RUN | `semantic-bm25.json` recall@5 = 0.00 (0/16); `semantic_fixtures_bm25_below_point_four_vector_wins` |
| 3 | Vector / hybrid beat BM25 | RUN | `semantic-vector.json` 1.00 16/16; `semantic-hybrid.json` 1.00 16/16 |
| 4 | `--layers` is a custom-harness mock switch | CODE | `RetrievalLayer`; CLI search still RouteQuery |
| 5 | Purity + ARI on `TopicExtractor::cluster` | RUN | `topics-quality.json`; `metrics` hand-computed 3-cluster (purity 8/9, ARI 4.5/7) |
| 6 | README vector/topic rows cite artifacts | DOCS | README status table; positioning Claims Ledger |