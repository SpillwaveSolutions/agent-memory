# Phase 25: E2E Core Pipeline Tests - Context

**Gathered:** 2026-02-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Automated tests that verify the full ingest-to-query pipeline across all search layers: TOC navigation, BM25 keyword search, vector semantic search, topic clustering, and grip provenance. Tests prove the core pipeline works end-to-end. Multi-agent scenarios, graceful degradation, and error paths are Phase 26.

</domain>

<decisions>
## Implementation Decisions

### Test organization
- **Dual location:** Dedicated `crates/e2e-tests/` crate for full pipeline scenarios AND integration tests in `memory-daemon` crate for direct API coverage
- E2E crate has its own Cargo.toml pulling in all necessary workspace dependencies
- Daemon-side tests exercise the daemon's public API directly

### Test file structure
- Claude's discretion on how to group test files (per-layer vs per-scenario)
- Choose based on dependency patterns and what minimizes boilerplate

### Test harness
- Claude's discretion on shared TestHarness struct vs per-test setup
- Goal: reduce boilerplate without over-abstracting

### Test naming
- Claude's discretion — pick a convention consistent with existing workspace tests

### Test data strategy
- Claude's discretion on builder helpers vs JSON fixtures — pick what works with existing patterns
- Claude's discretion on summarizer approach (mock vs template) — use what the Summarizer trait supports
- Claude's discretion on event volume per test — use minimum needed per scenario
- Claude's discretion on single-agent vs multi-agent in core tests — respect Phase 25 vs 26 boundary

### Assertion depth
- **Structural + content assertions:** Verify non-empty results AND check specific field values (agent, timestamps, text snippets)
- **Verify ordering:** BM25 returns higher-relevance results first, vector returns closest semantic matches first
- **Full provenance chain verification:** Trace from ingested event through grip creation to TOC bullet and back via expand_grip
- Use `pretty_assertions` crate for diff output on failure — consistent with workspace conventions

### Test isolation
- Claude's discretion on temp directory per test vs shared store with namespacing
- Claude's discretion on parallel safety (cargo test default) vs serial execution
- Claude's discretion on per-test timeouts vs relying on CI-level timeouts
- Claude's discretion on cleanup strategy (always clean vs keep-on-failure)

### Claude's Discretion
- Test harness design (shared struct vs per-test)
- Test file organization and naming convention
- Test data creation approach (builders vs fixtures)
- Summarizer mock strategy
- Event volume per test
- Agent scope in core tests (single vs dual)
- Storage isolation strategy
- Parallelism approach
- Timeout strategy
- Cleanup behavior

</decisions>

<specifics>
## Specific Ideas

- E2E tests use `cargo test` infrastructure (decided in v2.2 milestone planning — not a separate framework)
- Phase 24 wired all RPCs and added agent fields, so E2E tests can assert complete data
- Existing workspace uses `pretty_assertions` — maintain consistency
- Success criteria require 5 specific test scenarios (pipeline, BM25, vector, topics, grip expand)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 25-e2e-core-pipeline-tests*
*Context gathered: 2026-02-11*
