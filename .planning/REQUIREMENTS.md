# Requirements: Agent Memory

**Defined:** 2026-03-22 (v3.0); extended 2026-09-01 (v3.2)
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v3.0 Requirements

Requirements for the Competitive Parity & Benchmarks milestone. Each maps to roadmap phases.

### Retrieval Orchestrator (ORCH)

- [x] **ORCH-01**: `memory-orchestrator` crate exists with query expansion, RRF fusion, and rerank pipeline
- [x] **ORCH-02**: RRF fusion produces different ranking than any single index when scores diverge (unit tested)
- [x] **ORCH-03**: Orchestrator returns results when one of the four indexes returns empty (fail-open, unit tested)
- [x] **ORCH-04**: LLM rerank mode invokes configured LLM client and reorders results (integration tested with mock)
- [x] **ORCH-05**: Cross-encoder reranker extension point stubbed (trait exists, not implemented)
- [x] **ORCH-06**: `ContextBuilder` converts ranked results into structured `MemoryContext` with summary, events, entities, tokens
- [x] **ORCH-07**: Heuristic query expansion generates lowercase + keyword-stripped variants
- [x] **ORCH-08**: Existing `memory-retrieval` crate unchanged — orchestrator wraps `RetrievalExecutor`

### CLI API (CLI)

- [x] **CLI-01**: New `memory` binary with `search`, `context`, `recall`, `add`, `timeline`, `summary` subcommands
- [x] **CLI-02**: `memory search "query" --format=json` returns JSON envelope with results, meta, confidence
- [x] **CLI-03**: `memory recall` delegates to search with `--rerank=llm --top=10`
- [x] **CLI-04**: `memory add` writes via gRPC MemoryClient — exits non-zero with clear error if daemon not running
- [x] **CLI-05**: TTY detection: JSON when piped, human-readable when interactive
- [x] **CLI-06**: `memory context` returns structured context for prompt injection
- [x] **CLI-07**: `memory timeline` and `memory summary` query TOC by entity/range
- [x] **CLI-08**: `memory-daemon` binary and existing skill hooks unchanged
- [x] **CLI-09**: All commands exit 0 on success, non-zero on hard failure
- [x] **CLI-10**: `meta.tokens_estimated` included in JSON envelope for context budget decisions

### Benchmark Suite (BENCH)

- [x] **BENCH-01**: Custom benchmark harness with TOML fixture files (temporal, multisession, compression)
- [x] **BENCH-02**: `memory benchmark temporal|multisession|compression|all` subcommands
- [x] **BENCH-03**: Benchmark reports accuracy, recall@5, token_usage, latency_p50/p95, compression ratio
- [x] **BENCH-04**: LOCOMO adapter ingests Snap Research dataset and produces `results.json` with aggregate score
- [x] **BENCH-05**: `--compare` flag reads `benchmarks/baselines.toml` and prints side-by-side competitor table
- [x] **BENCH-06**: `locomo-data/` in `.gitignore` — dataset never committed
- [x] **BENCH-07**: CI runs benchmark suite (non-blocking, skips LOCOMO without `--dataset` flag)
- [x] **BENCH-08**: JSON + markdown report output for all benchmark types

## v3.2 Requirements (Prove It)

Requirements for making v3.1's claims provable and operable. Each maps to
exactly one plan in `docs/plans/v3.2-prove-it-plan.md`.

### Release pipeline (REL)

- [x] **REL-01**: Tagged commit must be an ancestor of `origin/main` (59-01)
- [x] **REL-02**: `workspace.package.version` must equal the tag minus `v` (59-01)
- [x] **REL-03**: Any failed platform build → no GitHub release (59-01)
- [x] **REL-04**: Release body is the matching CHANGELOG section (59-01)

### Benchmarks (BENCH)

- [x] **BENCH-10**: Per-conversation isolation on `--backend cli` (60-01)
- [x] **BENCH-11**: Deterministic drain wait (poll checkpoints, no blind sleep) (60-01)
- [ ] **BENCH-12**: Committed `locomo_llm_judge` full-dataset result (60-02 / #39)
- [ ] **BENCH-13**: Layer switch `bm25|vector|hybrid` on the custom harness (60-03)

### Quality evidence (QUAL)

- [ ] **QUAL-01**: Semantic fixture set, ≥15 tests (60-03 / #40)
- [ ] **QUAL-02**: Topic clustering purity + ARI artifact (60-03 / #47)
- [ ] **QUAL-03**: README "Solid" rows cite committed artifacts (60-03)

### Operate it (OPS)

- [ ] **OPS-01**: `admin backfill-index` resumable, idempotent (61-01 / #41)
- [ ] **OPS-02**: `install-service`/`uninstall-service` macOS+Linux (61-02 / #42)
- [ ] **OPS-03**: Zero fallible `unwrap` on request paths (61-03)
- [ ] **OPS-04**: Hostile-input e2e over all RPCs (61-03)
- [ ] **OPS-05**: `admin rebuild-toc` real (61-04 / #43)

### Installer (INST)

- [ ] **INST-01**: Claude Code plugin registration (CREG/META) (61-05)
- [ ] **INST-02**: `memory-installer uninstall` (61-05 / #48)
- [ ] **INST-03**: `memory-installer status` (61-05 / #48)

## Future Requirements (v3.3+)

- **ORCH-F01**: Cross-encoder reranking — Phase 62 *if* 60-02 shows retrieval is the bottleneck (#44)
- **CLI-F01**: REST/HTTP endpoint wrapping CLI commands
- **CLI-F02**: Python SDK wrapping CLI binary
- **BENCH-F01**: Continuous benchmark regression tracking in CI
- **REG-F01**: Gemini/Codex/Copilot plugin registration
- **INST-F01**: `--for all` / `--all` installer flags
- **OPS-F01**: Windows service install; true double-fork daemonization

## Out of Scope

| Feature | Reason |
|---------|--------|
| REST/HTTP endpoint | Future milestone — CLI-first |
| Python SDK | Future milestone — wraps CLI |
| Memory views UI | Future milestone |
| Cross-encoder reranking | Conditional Phase 62; extension point only until then |
| Multi-agent shared memory changes | Shipped in v2.1 |

## Traceability

### v3.0 (complete)

| Requirement | Phase | Status |
|-------------|-------|--------|
| ORCH-01..08 | Phase 51 | Complete |
| CLI-01..10 | Phase 52 | Complete |
| BENCH-01..08 | Phase 53 | Complete |

### v3.2

| Requirement | Plan | Status |
|-------------|------|--------|
| REL-01 | 59-01 | Complete (#45) |
| REL-02 | 59-01 | Complete (#45) |
| REL-03 | 59-01 | Complete (#45) |
| REL-04 | 59-01 | Complete (#45) |
| BENCH-10 | 60-01 | In progress |
| BENCH-11 | 60-01 | In progress |
| BENCH-12 | 60-02 | Open (#39) |
| BENCH-13 | 60-03 | Open (#40) |
| QUAL-01 | 60-03 | Open (#40) |
| QUAL-02 | 60-03 | Open (#47) |
| QUAL-03 | 60-03 | Open |
| OPS-01 | 61-01 | Open (#41) |
| OPS-02 | 61-02 | Open (#42) |
| OPS-03 | 61-03 | Open |
| OPS-04 | 61-03 | Open |
| OPS-05 | 61-04 | Open (#43) |
| INST-01 | 61-05 | Open |
| INST-02 | 61-05 | Open (#48) |
| INST-03 | 61-05 | Open (#48) |

**Coverage:**
- v3.0 requirements: 26 total, all complete
- v3.2 requirements: 19 total, 4 complete (REL), 15 open
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-22*
*Last updated: 2026-09-01 — v3.2 IDs added from the adopted Prove It plan*

