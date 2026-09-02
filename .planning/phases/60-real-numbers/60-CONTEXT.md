# Phase 60: Real Numbers - Context

**Gathered:** 2026-09-02
**Status:** 60-01 in PR #49; 60-03 executing
**Source:** docs/plans/v3.2-prove-it-plan.md

## Phase Boundary

A committed `locomo_llm_judge` number and quality artifacts behind every
"Solid". 60-01 is the harness; 60-02 is the maintainer run; 60-03 is
vector/topic fixtures.

## 60-01 decisions

- Design A: spawn-per-conversation. Design B (AdminReset) rejected.
- `--isolation daemon-per-conversation` is the default for `--backend cli`.
- `--pid-file` on daemon start/stop so spawned daemons do not clobber a
  user's PID file.
- GetIndexCheckpoints is read-only. Drain waits on BM25; vector is required
  only when that checkpoint exists (the outbox pipeline currently registers
  BM25 only).
- Poll interval is `mpsc::recv_timeout`, not `std::thread::sleep`.
- `--limit-questions` exists so 60-02 can dry-run.

## 60-03 decisions

- `--layers` is a **mock-backend** switch. Live `memory search` is always
  RouteQuery hybrid (indexing on a fresh daemon is BM25-only).
- Mock BM25 = token overlap. Mock vector = committed paraphrase lexicon +
  TF-IDF cosine (not Candle). Mock hybrid = RRF k=60.
- `memory-bench all` excludes `semantic` so `custom-harness-mock.json` is
  not tanked.
- Topic quality is `TopicExtractor::cluster` on capped TF-IDF (top 32,
  df≥2) of an 80-doc / 8-cluster synthetic corpus, not live TOC / Candle.
- Honest caveats are required in result JSON and README.