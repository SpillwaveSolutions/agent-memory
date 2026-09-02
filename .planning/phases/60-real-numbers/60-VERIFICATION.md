---
phase: 60-real-numbers
verified: 2026-09-02
status: 60-01-verified
---

# Phase 60: Real Numbers Verification

## 60-01 (this PR)

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
