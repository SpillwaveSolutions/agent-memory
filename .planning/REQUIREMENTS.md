# Requirements: v2.2 Production Hardening

**Defined:** 2026-02-10
**Core Value:** An agent can answer "what were we talking about last week?" without scanning everything

## v1 Requirements

Requirements for this milestone. Each maps to roadmap phases.

### E2E Testing

- [ ] **E2E-01**: Full pipeline test: ingest events -> TOC segment build -> grip creation -> query route returns correct results
- [ ] **E2E-02**: Teleport index test: ingest -> BM25 index build -> bm25_search returns matching events
- [ ] **E2E-03**: Vector teleport test: ingest -> vector index build -> vector_search returns semantically similar events
- [ ] **E2E-04**: Topic graph test: ingest -> topic clustering -> get_top_topics returns relevant topics
- [ ] **E2E-05**: Multi-agent test: ingest from multiple agents -> cross-agent query returns all -> filtered query returns one
- [ ] **E2E-06**: Graceful degradation test: query with missing indexes still returns results via TOC fallback
- [ ] **E2E-07**: Grip provenance test: ingest -> segment with grips -> expand_grip returns source events with context
- [ ] **E2E-08**: Error path test: malformed events handled gracefully, invalid queries return useful errors

### Tech Debt

- [ ] **DEBT-01**: Wire GetRankingStatus RPC to return current ranking configuration
- [ ] **DEBT-02**: Wire PruneVectorIndex RPC to trigger vector index cleanup
- [ ] **DEBT-03**: Wire PruneBm25Index RPC to trigger BM25 index cleanup
- [ ] **DEBT-04**: Fix ListAgents session_count via event scanning (currently 0 from TOC only)
- [ ] **DEBT-05**: Add agent field to TeleportResult proto message and populate from event metadata
- [ ] **DEBT-06**: Add agent field to VectorTeleportMatch proto message and populate from event metadata

### CI/CD

- [ ] **CI-01**: E2E test suite runs in GitHub Actions CI pipeline
- [ ] **CI-02**: E2E tests run on PR submissions (not just main branch pushes)
- [ ] **CI-03**: CI reports test count and pass/fail status for E2E suite separately

## v2 Requirements

Deferred to future release.

### Performance
- **PERF-01**: Ingest throughput benchmark (events/sec)
- **PERF-02**: Query latency benchmark (p50/p95/p99)
- **PERF-03**: Index rebuild time benchmark

### Cross-Project
- **XPROJ-01**: Shared memory across projects
- **XPROJ-02**: Project-scoped vs global memory queries

## Out of Scope

| Feature | Reason |
|---------|--------|
| Performance benchmarks | Deferred to v2.3; not needed for production readiness |
| Cross-project memory | Future milestone; requires architectural changes |
| MCP server | Future milestone; different deployment model |
| New adapter plugins | v2.1 shipped 4 adapters; sufficient for now |
| UI/dashboard | CLI-only tool, no UI planned |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| E2E-01 | Phase 25 | Pending |
| E2E-02 | Phase 25 | Pending |
| E2E-03 | Phase 25 | Pending |
| E2E-04 | Phase 25 | Pending |
| E2E-05 | Phase 26 | Pending |
| E2E-06 | Phase 26 | Pending |
| E2E-07 | Phase 25 | Pending |
| E2E-08 | Phase 26 | Pending |
| DEBT-01 | Phase 24 | Pending |
| DEBT-02 | Phase 24 | Pending |
| DEBT-03 | Phase 24 | Pending |
| DEBT-04 | Phase 24 | Pending |
| DEBT-05 | Phase 24 | Pending |
| DEBT-06 | Phase 24 | Pending |
| CI-01 | Phase 27 | Pending |
| CI-02 | Phase 27 | Pending |
| CI-03 | Phase 27 | Pending |

**Coverage:**
- v1 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0

---
*Requirements defined: 2026-02-10*
*Last updated: 2026-02-10 after roadmap creation*
