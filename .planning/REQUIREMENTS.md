# Requirements: Agent Memory v2.5

**Defined:** 2026-03-05
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v2.5 Requirements

Requirements for semantic dedup and retrieval quality milestone. Each maps to roadmap phases.

### Dedup Gate

- [ ] **DEDUP-01**: System embeds incoming events and checks in-flight buffer (256 entries) for within-session duplicates
- [ ] **DEDUP-02**: System checks HNSW vector index for cross-session duplicates against all history
- [ ] **DEDUP-03**: Duplicate events are stored in RocksDB but skip outbox/indexing (append-only preserved)
- [ ] **DEDUP-04**: Structural events (session_start, session_end) bypass dedup entirely
- [ ] **DEDUP-05**: Similarity threshold is configurable (default 0.85) via config.toml
- [ ] **DEDUP-06**: Dedup gate is fail-open — embedding/search failures pass events through unchanged

### Retrieval Quality

- [ ] **RETRV-01**: Stale results are downranked via time-decay relative to the newest result in the result set
- [ ] **RETRV-02**: Supersession detection marks older results semantically similar to newer ones as superseded
- [ ] **RETRV-03**: High-salience events (above threshold) are exempt from time-decay
- [ ] **RETRV-04**: Time-decay half-life is configurable (default 14 days) via config.toml

### Testing

- [ ] **TEST-01**: E2E tests prove dedup drops duplicate events from indexing while preserving storage
- [ ] **TEST-02**: E2E tests prove stale filtering downranks old results relative to newer ones
- [ ] **TEST-03**: E2E tests prove fail-open behavior when dedup gate encounters errors

## Future Requirements

### Dedup Enhancements

- **DEDUP-F01**: Admin RPC to view dedup statistics (events skipped, threshold hits, buffer utilization)
- **DEDUP-F02**: Calibration test fixture with known text pairs for threshold tuning
- **DEDUP-F03**: Per-agent dedup scoping (only dedup within same agent's history)

### Cross-Project Memory

- **XPROJ-01**: Unified memory queries across multiple project stores
- **XPROJ-02**: Cross-project dedup for shared context

## Out of Scope

| Feature | Reason |
|---------|--------|
| LLM-based dedup (Mem0 pattern) | Requires API calls; violates local-first, 50ms timeout constraint |
| Event deletion for dedup | Violates append-only invariant; store-and-skip-outbox instead |
| Separate dedup vector index | In-flight buffer + existing HNSW sufficient; avoids storage/complexity overhead |
| Query-time dedup (post-retrieval) | Ingest-time dedup is the agreed approach; supersession handles query-side |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEDUP-01 | Phase 35 | Pending |
| DEDUP-02 | Phase 36 | Pending |
| DEDUP-03 | Phase 36 | Pending |
| DEDUP-04 | Phase 36 | Pending |
| DEDUP-05 | Phase 35 | Pending |
| DEDUP-06 | Phase 35 | Pending |
| RETRV-01 | Phase 37 | Pending |
| RETRV-02 | Phase 37 | Pending |
| RETRV-03 | Phase 37 | Pending |
| RETRV-04 | Phase 37 | Pending |
| TEST-01 | Phase 38 | Pending |
| TEST-02 | Phase 38 | Pending |
| TEST-03 | Phase 38 | Pending |

**Coverage:**
- v2.5 requirements: 13 total
- Mapped to phases: 13
- Unmapped: 0

---
*Requirements defined: 2026-03-05*
*Last updated: 2026-03-05 after roadmap creation (traceability complete)*
