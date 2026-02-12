# Phase 29: Performance Benchmarks - Context

**Gathered:** 2026-02-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish baseline ingest throughput and query latency benchmarks across core retrieval layers using a dedicated benchmark harness. No new product capabilities.

</domain>

<decisions>
## Implementation Decisions

### Benchmark scope
- Benchmark ingest + TOC navigation + teleport (BM25/vector) + topic graph + route_query.
- Run both cold and warm benchmarks.
- Include single-agent plus minimal multi-agent cases.
- Primary benchmark location: bench harness under `crates/e2e-tests`.

### Benchmark format and outputs
- Output both a human-readable table and JSON.
- Write results to stdout and to a file.
- Report per-step + per-layer metrics (not end-to-end only).
- Include p50/p90/p99 percentiles.

### Dataset and workload shape
- Use synthetic baseline data with optional real trace input.
- Include small + medium dataset tiers.
- Mixed content with multi-agent tags.
- Deterministic runs (fixed seed).

### Success criteria and thresholds
- Baseline + soft warning thresholds (not hard fail on all regressions).
- Compare against a committed baseline file.
- Emit warning + non-zero exit only for severe regressions.
- Use both relative and absolute thresholds.

### Claude's Discretion
- Exact benchmark fixture content and naming.
- File naming conventions for benchmark outputs, as long as they are documented and deterministic.

</decisions>

<specifics>
## Specific Ideas

- None beyond decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 29-performance-benchmarks*
*Context gathered: 2026-02-12*
