# Phase 37: StaleFilter - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Downrank stale and superseded query results at retrieval time so agents get fresher, more relevant answers. This is a query-time scoring adjustment applied post-merge in the RetrievalExecutor, before results are returned. No changes to storage or ingest pipeline.

</domain>

<decisions>
## Implementation Decisions

### Time-decay curve
- Reference point: relative to the newest result in the result set (not query time). If all results are old, no penalty applies.
- Formula: `score_adj = score * (1.0 - max_penalty * (1 - e^(-age/half_life)))` where age = newest_timestamp - this_timestamp
- Asymptotic approach to max penalty (never reaches 30%, smoothly approaches it)
- At 1 half-life (~14 days): ~19% penalty. At 2 half-lives: ~26%. At 3 half-lives: ~28.6%.
- Default half-life: 14 days (configurable via config.toml)
- Max penalty: 30% score reduction (asymptotic bound, not hard floor)

### Supersession logic
- Superseded results get a fixed 15% additional penalty on top of time-decay
- No transitivity: each result marked superseded at most once, even if multiple newer results are similar
- Add `superseded_by: <doc_id>` to result's metadata HashMap for explainability (no proto change needed)
- Supersession similarity threshold: 0.80 (lower than dedup's 0.85 — catches "evolved versions of same topic")

### Kind exemptions
- Exempt from BOTH time-decay AND supersession: Constraint, Definition, Procedure, Preference
- Only Observation gets full decay treatment
- Hardcoded enum match (not configurable) — simple, testable, can be made configurable later

### Claude's Discretion
- Whether to apply uniform decay across retrieval layers or reduce decay for semantic results
- Whether to add a per-query `skip_staleness` flag (likely defer — no proto change this phase)
- How to handle high-salience Observations (threshold exemption vs full decay)
- Supersession detection method: pairwise cosine check vs topic-based grouping

</decisions>

<specifics>
## Specific Ideas

- Combined max theoretical penalty: ~45% (30% decay + 15% supersession) for a very old Observation superseded by a newer result
- Follows DedupConfig pattern for config struct design (flat struct, serde defaults)
- Enabled by default (unlike dedup which is opt-in) — safe because worst case is minor score adjustment

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `MemoryKind` enum in `memory-types/src/salience.rs`: Already has Observation, Preference, Procedure, Constraint, Definition — exact variants needed for exemption matching
- `DedupConfig` in `memory-types/src/config.rs`: Pattern for how config sections are structured (serde defaults, flat struct)
- `SearchResult` in `memory-retrieval/src/executor.rs`: Has `score: f32`, `metadata: HashMap<String, String>`, and `source_layer` — all fields needed for staleness adjustment
- `importance::TimeDecayCalculator` in `memory-topics/src/importance.rs`: Existing time-decay implementation for topics (different use case but similar math)

### Established Patterns
- Config: `[section_name]` in config.toml with `default_*()` helper functions for serde defaults
- Scoring: Write-time salience (SalienceScorer) vs query-time adjustment (StaleFilter) — different lifecycle
- Fail-open: DedupConfig pattern of passing through unchanged on errors

### Integration Points
- `RetrievalExecutor::execute()` in `memory-retrieval/src/executor.rs`: Post-merge, pre-return is where StaleFilter hooks in
- `ExecutionResult.results: Vec<SearchResult>`: The result set that gets filtered
- `SearchResult.metadata`: Where `superseded_by` flag gets added
- `StalenessConfig` loads from `MemoryConfig` alongside `DedupConfig`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 37-stale-filter*
*Context gathered: 2026-03-06*
