# Phase 51: Retrieval Orchestrator - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md + docs/superpowers/plans/2026-03-21-v3-phase-a-retrieval-orchestrator.md)

<domain>
## Phase Boundary

This phase creates a new `memory-orchestrator` crate that coordinates all existing indexes into a single, ranked, confidence-scored result set. Existing crates unchanged. The orchestrator wraps `RetrievalExecutor` — calls it per layer, then applies RRF fusion across per-layer result sets.

Pipeline: Query Expansion → Multi-Index Fan-out → RRF Fusion → Reranking → Context Builder

</domain>

<decisions>
## Implementation Decisions

### Architecture
- New crate `crates/memory-orchestrator/` — sits between `memory-retrieval` and the CLI
- All existing crates unchanged — orchestrator is purely additive
- Wraps `RetrievalExecutor` from `memory-retrieval` for each index layer
- Fail-open: if any index fails during fan-out, orchestrator continues with available results

### Query Expansion (expand.rs)
- Heuristic variants only in Phase A (default): lowercase, strip question words, keyword bias
- LLM-assisted expansion deferred (opt-in via --expand=llm flag, not wired in Phase A)
- Always includes original query plus 1-2 variants

### RRF Fusion (fusion.rs)
- Reciprocal Rank Fusion: for each doc, sum 1/(k+rank) across all lists
- RRF constant k=60 (standard literature value)
- Parameter-free, no per-corpus tuning needed
- Deduplicates same doc_id across lists
- Salience/recency from Phase 40 scores adjust ranking post-fusion

### Reranking (rerank.rs)
- `Reranker` trait with async `rerank()` method
- `HeuristicReranker`: uses RRF scores as-is, trims to top 10 (default)
- `LlmReranker`: placeholder — falls back to heuristic in Phase A (client not wired)
- `CrossEncoderReranker`: extension point stubbed, NOT implemented (different inference path needed)

### Context Builder (context_builder.rs)
- Converts ranked results into structured `MemoryContext`
- Fields: summary, relevant_events, key_entities, open_questions, retrieval_ms, tokens_estimated, confidence
- Token estimate: ~0.75 tokens per character + overhead
- key_entities and open_questions populated in Phase C

### Types (types.rs)
- `OrchestratorConfig`: top_k (10), rerank_mode (Heuristic), expand_query (false), rrf_k (60.0)
- `RerankMode`: Heuristic | Llm enum
- `RankedResult`: score, doc_id, text, source_layer, confidence
- `MemoryContext`: summary, relevant_events, key_entities, open_questions, retrieval_ms, tokens_estimated, confidence

### Claude's Discretion
- Exact error handling strategy in orchestrator.rs (anyhow vs thiserror)
- Whether to use tracing::instrument on key methods
- Test helper organization within modules vs separate test files

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec & Plans
- `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md` — Full v3.0 design spec (Phase A section)
- `docs/superpowers/plans/2026-03-21-v3-phase-a-retrieval-orchestrator.md` — Detailed implementation plan with TDD tasks, code snippets, and test cases

### Existing Retrieval Crate (DO NOT MODIFY)
- `crates/memory-retrieval/src/executor.rs` — RetrievalExecutor, LayerExecutor trait, SearchResult, FallbackChain
- `crates/memory-retrieval/src/types.rs` — RetrievalLayer enum, CapabilityTier, ExecutionMode, StopConditions

### Existing Types
- `crates/memory-types/src/lib.rs` — Core types used across crates

</canonical_refs>

<specifics>
## Specific Ideas

- The implementation plan includes complete TDD-style task breakdown (8 tasks with test-first approach)
- Each task has specific Rust code snippets for types, functions, and tests
- RRF fusion algorithm is fully specified with test cases for consensus boosting, deduplication, and empty list handling
- MockLayerExecutor pattern specified for orchestrator integration tests

</specifics>

<deferred>
## Deferred Ideas

- Cross-encoder reranking (ORCH-F01) — requires new inference path in memory-embeddings
- LLM-assisted query expansion (--expand=llm) — extension point exists, impl deferred
- LLM reranker client wiring — trait exists, actual LLM client connection deferred to Phase B
- NER for key_entities in ContextBuilder — populated in Phase C

</deferred>

---

*Phase: 51-retrieval-orchestrator*
*Context gathered: 2026-03-22 via PRD Express Path*
