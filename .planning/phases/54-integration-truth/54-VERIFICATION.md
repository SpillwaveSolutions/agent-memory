---
phase: 54-integration-truth
verified: 2026-08-30
status: passed
---

# Phase 54: Integration Truth Verification

**Phase Goal:** every merged feature is reachable from a public entry point or is explicitly removed/relabeled. No silent success-while-doing-nothing.

## Execution evidence (v3.1 process rule)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `memory-orchestrator` has a binary-reachable dependent | VERIFIED | `cargo tree -i memory-orchestrator` → `memory-service` → `memory-daemon`, `memory-cli`, `e2e-tests` |
| 2 | RouteQuery names fusion stage `rank_fusion` | VERIFIED | unit `test_route_query_names_fusion_stage`; e2e `test_full_pipeline_ingest_toc_grip_route_query` asserts `explanation.fusion_stage == "rank_fusion"` |
| 3 | LLM rerank reorders vs heuristic | VERIFIED | `test_llm_rerank_reorders_bm25_hits` (mock Completer reverses prompt order); orchestrator `test_llm_reranker_honors_completer_order` |
| 4 | BM25 outbox indexes events | VERIFIED | `test_process_index_event_makes_event_findable`; missing events warn + `BM25_SKIPPED_NOOP` |
| 5 | Hybrid fusion differs from either input | VERIFIED | `test_hybrid_fusion_differs_from_either_input`; `test_weighted_fusion_differs_from_either_input` |
| 6 | One RRF site | VERIFIED | `rg -l 'reciprocal\|rrf' crates/` → `crates/memory-orchestrator/src/fusion.rs` only |
| 7 | `--background` is honest | VERIFIED | CLI parse tests; `start_daemon` `anyhow::bail!` when `background=true` |
| 8 | Lock poison recovers | VERIFIED | `recover_lock_recovers_from_poison`; production `Mutex`/`RwLock` sites in vector/hnsw/registry/usage use `recover_lock` |

## Crate reachability

```
memory-orchestrator v2.7.0
└── memory-service v2.7.0
    ├── e2e-tests
    ├── memory-cli
    ├── memory-client
    └── memory-daemon
```

`RouteQuery` calls `MemoryOrchestrator::query_ranked`. CLI `memory search --rerank` forwards `rerank_mode` through `route_query_ex`.

## Human verification (blockers)

- [x] `cargo tree -i memory-orchestrator` shows memory-service / memory-daemon
- [x] Clippy `-D warnings` green on `--workspace --all-targets --all-features`
- [x] E2E `pipeline_test` fusion_stage assertion
- [ ] Full `cargo test --workspace --all-features` (CI); unit tests of all Phase 54 crates passed locally
