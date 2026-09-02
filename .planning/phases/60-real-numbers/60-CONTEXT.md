# Phase 60: Real Numbers - Context

**Gathered:** 2026-09-02
**Status:** 60-01 in execution
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
