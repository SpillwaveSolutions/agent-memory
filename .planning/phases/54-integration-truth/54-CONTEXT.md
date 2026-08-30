# Phase 54: Integration Truth - Context

**Gathered:** 2026-08-30
**Status:** Implemented (PR pending)
**Source:** docs/plans/v3.1-make-it-true-plan.md

<domain>
## Phase Boundary

Close the v3.0 claim/reality gap for wiring: every merged retrieval feature is
reachable from a public entry point or is explicitly removed/relabeled. No
code path silently succeeds while doing nothing.

No new capabilities. Sequential first phase of milestone v3.1 "Make It True".
</domain>

<decisions>
## Implementation Decisions

- Wire `memory-orchestrator` on the daemon/service side behind `RouteQuery`
  so CLI and gRPC callers both benefit (not a CLI-only dep).
- LLM reranker uses `Completer` trait; production injects `ApiSummarizer`.
- CrossEncoder hard-errors (`RerankError::NotImplemented`); never warn-and-fallback.
- BM25 outbox `IndexEvent`/`UpdateToc` index the event (`DocType::Event`) rather
  than deleting the action. Missing events warn + `BM25_SKIPPED_NOOP`.
- Canonical fusion lives in `memory-orchestrator::fusion` (`fuse` / `fuse_weighted`).
  Hybrid layer and HybridSearch RPC both call it. `rg -l "reciprocal|rrf" crates/`
  is fusion.rs only.
- `CrateLayer::Hybrid` fetches BM25 + vector and fuses with equal weights.
- Daemon `start` is honest foreground. `--background` exits non-zero.
- Lock policy: `recover_lock` (unwrap_or_else into_inner + metric). No parking_lot.
- Daemon `run_server_with_scheduler` attaches BM25/vector/topics via `QueryIndexBundle`.
- Process rules added to `.planning/config.json`: execution-evidence, crate
  reachability, human_verification as blockers.
</decisions>
