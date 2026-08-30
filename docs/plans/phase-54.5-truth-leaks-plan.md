# Phase 54.5 — Truth leaks + CI toolchain pin

**Date:** 2026-08-30
**Status:** Implemented (PR pending)
**Depends on:** Phase 54 (merged #32), Phase 55 (merged #33)
**Branch:** `feature/phase-54.5-truth-leaks`

## Why

Phase 54's public claims were true. A review of the wiring found second-order
honesty bugs in the explainability payload and two landmines that would
contaminate Phase 55 measurements. Main is also red on Clippy because CI
tracks floating `stable` (Rust 1.98 `result_large_err` on generated tonic stubs).

## Scope

1. **CI:** `#[allow(clippy::result_large_err)]` on `include_proto!`; pin
   `rust-toolchain.toml` to 1.97; CI/release/e2e workflows honor the pin.
2. **Explainability tells the truth**
   - LLM fail-open (completer error or unparseable order) reports `rerank=heuristic`
   - `layers_attempted` only lists layers that were supported and invoked
   - `stop_conditions` and `mode_override` are forwarded into the orchestrator
   - unknown `rerank_mode` is `InvalidArgument` (CLI clap-rejects too)
3. **LLM rerank actually reranks**
   - BM25 hits expose indexed `text` (events have empty keywords)
   - salience ranking `preserve_order` after a successful LLM rerank
4. **Don't plant Phase 55 landmines**
   - drop per-event full grip scan on BM25 outbox drain
   - fusion fan-out is concurrent (`join_all`) unless mode is Sequential
   - query path, prune job, and dedup share one HNSW `Arc<RwLock<_>>`
   - dimension comes from the embedder / `EMBEDDING_DIM`, not a magic 384

## Non-goals

- Phase 55/56 work
- Implementing cross-encoder reranking
- Background daemonization
