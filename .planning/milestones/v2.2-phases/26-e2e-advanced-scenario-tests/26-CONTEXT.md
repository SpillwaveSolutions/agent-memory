# Phase 26: E2E Advanced Scenario Tests - Context

**Gathered:** 2026-02-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Verify edge cases and multi-agent scenarios through automated E2E tests. Covers cross-agent queries (unfiltered and filtered), graceful degradation when indexes are missing, and error handling for malformed inputs. Builds on Phase 25's core test infrastructure.

</domain>

<decisions>
## Implementation Decisions

### Multi-agent test scenarios
- Agent count: Claude's discretion (pick what best exercises the code paths)
- Search layer coverage: Focus on primary retrieval path (route_query/TOC) for multi-agent tests, not all layers
- Agent filter strictness: Claude's discretion based on current filtering implementation
- Content overlap between agents: Claude's discretion — pick the data pattern that best exercises cross-agent vs filtered queries

### Graceful degradation behavior
- Fallback signaling: Claude's discretion based on current response structure
- Missing index scenarios: Test BOTH individual missing indexes (BM25 missing, vector missing, topic graph missing separately) AND all indexes missing together (worst case: TOC only)
- Fallback result quality: Assert correct provenance — verify results come from expected sessions/segments with valid provenance chains, not just non-empty
- Stale/partial index states: Claude's discretion on whether this adds value

### Error handling expectations
- Malformed input types: Claude's discretion on which scenarios are most likely to cause real issues
- gRPC status code assertions: Claude's discretion on assertion precision
- Error message quality: Assert that error messages contain useful context — messages must mention the problematic field/value for better debugging
- Concurrency error scenarios: Claude's discretion on whether this belongs in this phase

### Test data & infrastructure
- Test location: Claude's discretion based on existing e2e-tests crate structure
- Agent naming in tests: Use realistic agent names (e.g., 'claude', 'copilot', 'gemini') — doubles as documentation of real multi-agent scenarios
- Test gating: All Phase 26 tests run by default — no #[ignore]. These are essential E2E tests that should always pass
- File organization: Claude's discretion based on Phase 25 patterns

### Claude's Discretion
- Number of test agents for multi-agent scenarios
- Filter behavior assertions (strict exclusion vs prioritization)
- Content overlap pattern for test data
- Whether to test stale/partial index states
- Specific malformed input categories to test
- gRPC status code assertion granularity
- Whether to include concurrency error scenarios
- Same crate vs new crate for test location
- File-per-scenario vs grouped organization

</decisions>

<specifics>
## Specific Ideas

- Use realistic agent names like 'claude', 'copilot', 'gemini' in test data — makes tests serve as documentation
- Degradation tests must cover each missing index individually AND all missing together
- Error messages must include field-level context (which field was invalid, what was wrong)
- All tests must run without #[ignore] — no special flags needed to exercise advanced scenarios

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 26-e2e-advanced-scenario-tests*
*Context gathered: 2026-02-11*
