# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v2.5 — Semantic Dedup & Retrieval Quality

**Shipped:** 2026-03-10
**Phases:** 4 | **Plans:** 11 | **Sessions:** ~11

### What Was Built
- Ingest-time semantic dedup via InFlightBuffer + HNSW CompositeVectorIndex with configurable 0.85 threshold
- Store-and-skip-outbox pattern preserving append-only invariant while keeping indexes clean
- StaleFilter with exponential time-decay, supersession detection, and high-salience kind exemptions
- StalenessConfig propagation from config.toml through daemon to RetrievalHandler
- 10 E2E tests proving dedup, stale filtering, and fail-open end-to-end

### What Worked
- Gap closure plans (35-02, 36-03, 37-03) caught integration issues before E2E validation
- Phase 37 (StaleFilter) was independent of 35-36 (dedup), enabling parallel conceptual progress
- Fail-open design philosophy carried forward cleanly from v1.0 hooks
- Milestone audit caught unchecked REQUIREMENTS.md boxes before completion — prevented false gaps
- Average plan execution was 4 minutes — consistent with v2.4 cadence

### What Was Inefficient
- SUMMARY.md files for phases 35/37/38 lacked `one_liner` frontmatter — required manual extraction during milestone completion
- Nyquist VALIDATION.md files missing for all 4 phases — informational but adds noise to audit
- ROADMAP.md progress table had formatting inconsistencies (missing plan counts for phases 37-38)

### Patterns Established
- CompositeVectorIndex pattern: search multiple backends, return highest score — reusable for future index types
- Store-and-skip-outbox: dedup without violating append-only — architectural pattern for any future filtering
- StalenessConfig via `with_services` parameter threading — no global state, explicit dependency injection
- DedupResult carrying embedding alongside should_store — avoids redundant embedding computation

### Key Lessons
1. Config propagation (RETRV-04) needed its own plan — end-to-end wiring is never "just plumbing"
2. `std::sync::RwLock` is fine for sub-microsecond operations — don't default to tokio async locks
3. Supersession iterates newest-first and breaks on first match — simpler than transitive closure, works in practice
4. ULID-based event_ids are required in E2E tests (storage validates format) — caught late but not blocking

### Cost Observations
- Model mix: 100% opus (quality profile)
- Sessions: ~11 (one per plan)
- Notable: 5-day milestone with 42 commits — efficient for 4 phases of new architecture

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~20 | 8 | Foundation patterns established |
| v2.0 | ~42 | 9 | Cognitive layers, first index work |
| v2.1 | ~22 | 6 | Multi-agent adapters, CLOD format |
| v2.2 | ~10 | 4 | E2E testing, CI gates |
| v2.3 | ~2 | 2 | Docs and benchmarks |
| v2.4 | ~15 | 5 | CLI test harnesses |
| v2.5 | ~11 | 4 | Dedup + stale filter, quality layer |

### Cumulative Quality

| Milestone | E2E Tests | CLI Tests | LOC Rust |
|-----------|-----------|-----------|----------|
| v1.0 | 0 | 0 | 9,135 |
| v2.0 | 0 | 0 | ~27,000 |
| v2.1 | 0 | 0 | ~40,800 |
| v2.2 | 29 | 0 | 43,932 |
| v2.3 | 29 | 0 | 44,912 |
| v2.4 | 29 | 144 | 44,917 |
| v2.5 | 39 | 144 | 48,282 |

### Top Lessons (Verified Across Milestones)

1. Fail-open design is essential for agent tooling — proven in v1.0 hooks, v2.5 dedup gate
2. Gap closure plans are worth the extra phase — catch integration issues early (v2.5 phases 36-03, 37-03)
3. Milestone audits before completion prevent false completion — caught requirement checkbox gaps in v2.5
4. One plan per meaningful decision boundary keeps execution predictable (avg 4-5 min/plan across v2.4-v2.5)
