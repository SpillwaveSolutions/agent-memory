# Requirements: Agent Memory v3.0

**Defined:** 2026-03-22
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v3.0 Requirements

Requirements for the Competitive Parity & Benchmarks milestone. Each maps to roadmap phases.

### Retrieval Orchestrator (ORCH)

- [x] **ORCH-01**: `memory-orchestrator` crate exists with query expansion, RRF fusion, and rerank pipeline
- [ ] **ORCH-02**: RRF fusion produces different ranking than any single index when scores diverge (unit tested)
- [ ] **ORCH-03**: Orchestrator returns results when one of the four indexes returns empty (fail-open, unit tested)
- [ ] **ORCH-04**: LLM rerank mode invokes configured LLM client and reorders results (integration tested with mock)
- [ ] **ORCH-05**: Cross-encoder reranker extension point stubbed (trait exists, not implemented)
- [ ] **ORCH-06**: `ContextBuilder` converts ranked results into structured `MemoryContext` with summary, events, entities, tokens
- [x] **ORCH-07**: Heuristic query expansion generates lowercase + keyword-stripped variants
- [ ] **ORCH-08**: Existing `memory-retrieval` crate unchanged — orchestrator wraps `RetrievalExecutor`

### CLI API (CLI)

- [ ] **CLI-01**: New `memory` binary with `search`, `context`, `recall`, `add`, `timeline`, `summary` subcommands
- [ ] **CLI-02**: `memory search "query" --format=json` returns JSON envelope with results, meta, confidence
- [ ] **CLI-03**: `memory recall` delegates to search with `--rerank=llm --top=10`
- [ ] **CLI-04**: `memory add` writes via gRPC MemoryClient — exits non-zero with clear error if daemon not running
- [ ] **CLI-05**: TTY detection: JSON when piped, human-readable when interactive
- [ ] **CLI-06**: `memory context` returns structured context for prompt injection
- [ ] **CLI-07**: `memory timeline` and `memory summary` query TOC by entity/range
- [ ] **CLI-08**: `memory-daemon` binary and existing skill hooks unchanged
- [ ] **CLI-09**: All commands exit 0 on success, non-zero on hard failure
- [ ] **CLI-10**: `meta.tokens_estimated` included in JSON envelope for context budget decisions

### Benchmark Suite (BENCH)

- [ ] **BENCH-01**: Custom benchmark harness with TOML fixture files (temporal, multisession, compression)
- [ ] **BENCH-02**: `memory benchmark temporal|multisession|compression|all` subcommands
- [ ] **BENCH-03**: Benchmark reports accuracy, recall@5, token_usage, latency_p50/p95, compression ratio
- [ ] **BENCH-04**: LOCOMO adapter ingests Snap Research dataset and produces `results.json` with aggregate score
- [ ] **BENCH-05**: `--compare` flag reads `benchmarks/baselines.toml` and prints side-by-side competitor table
- [ ] **BENCH-06**: `locomo-data/` in `.gitignore` — dataset never committed
- [ ] **BENCH-07**: CI runs benchmark suite (non-blocking, skips LOCOMO without `--dataset` flag)
- [ ] **BENCH-08**: JSON + markdown report output for all benchmark types

## Future Requirements (v3.1+)

- **ORCH-F01**: Cross-encoder reranking (requires new inference path in memory-embeddings)
- **CLI-F01**: REST/HTTP endpoint wrapping CLI commands
- **CLI-F02**: Python SDK wrapping CLI binary
- **BENCH-F01**: Continuous benchmark regression tracking in CI

## Out of Scope

| Feature | Reason |
|---------|--------|
| REST/HTTP endpoint | Future milestone — CLI-first for v3.0 |
| Python SDK | Future milestone — wraps CLI |
| Memory views UI | Future milestone |
| Cross-encoder reranking | Requires new inference path in memory-embeddings; extension point only |
| Multi-agent shared memory changes | Shipped in v2.1 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ORCH-01 | Phase 51 | Complete |
| ORCH-02 | Phase 51 | Pending |
| ORCH-03 | Phase 51 | Pending |
| ORCH-04 | Phase 51 | Pending |
| ORCH-05 | Phase 51 | Pending |
| ORCH-06 | Phase 51 | Pending |
| ORCH-07 | Phase 51 | Complete |
| ORCH-08 | Phase 51 | Pending |
| CLI-01 | Phase 52 | Pending |
| CLI-02 | Phase 52 | Pending |
| CLI-03 | Phase 52 | Pending |
| CLI-04 | Phase 52 | Pending |
| CLI-05 | Phase 52 | Pending |
| CLI-06 | Phase 52 | Pending |
| CLI-07 | Phase 52 | Pending |
| CLI-08 | Phase 52 | Pending |
| CLI-09 | Phase 52 | Pending |
| CLI-10 | Phase 52 | Pending |
| BENCH-01 | Phase 53 | Pending |
| BENCH-02 | Phase 53 | Pending |
| BENCH-03 | Phase 53 | Pending |
| BENCH-04 | Phase 53 | Pending |
| BENCH-05 | Phase 53 | Pending |
| BENCH-06 | Phase 53 | Pending |
| BENCH-07 | Phase 53 | Pending |
| BENCH-08 | Phase 53 | Pending |

**Coverage:**
- v3.0 requirements: 26 total
- Mapped to phases: 26
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-22*
*Last updated: 2026-03-22 after spec review*
