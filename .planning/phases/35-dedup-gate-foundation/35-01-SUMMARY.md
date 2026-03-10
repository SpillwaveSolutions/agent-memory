---
phase: 35-dedup-gate-foundation
plan: 01
subsystem: types
tags: [ring-buffer, cosine-similarity, dedup, config, serde]

requires:
  - phase: 16-memory-ranking-enhancements
    provides: NoveltyConfig struct and NoveltyChecker service
provides:
  - InFlightBuffer ring buffer with push/find_similar/clear
  - DedupConfig with threshold 0.85, buffer_capacity 256
  - NoveltyConfig backward-compatible type alias
  - Settings.dedup field with serde alias "novelty"
affects: [35-02-dedup-gate-foundation, memory-service-novelty]

tech-stack:
  added: []
  patterns: [ring-buffer-with-modular-wrap, type-alias-for-backward-compat, serde-alias-for-config-migration]

key-files:
  created:
    - crates/memory-types/src/dedup.rs
  modified:
    - crates/memory-types/src/config.rs
    - crates/memory-types/src/lib.rs

key-decisions:
  - "Cosine similarity as dot product (vectors pre-normalized by CandleEmbedder)"
  - "NoveltyConfig kept as type alias, not deprecated -- existing code compiles unchanged"
  - "DedupConfig added to Settings with serde(alias = novelty) for TOML backward compat"

patterns-established:
  - "Type alias migration: rename struct, add `pub type OldName = NewName;` for backward compat"
  - "Ring buffer: Vec<Option<T>> with head/count tracking and modular wrap"

duration: 3min
completed: 2026-03-06
---

# Phase 35 Plan 01: DedupGate Foundation Types Summary

**InFlightBuffer ring buffer and DedupConfig (threshold 0.85, buffer_capacity 256) replacing NoveltyConfig with backward-compatible alias and Settings wiring**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-06T03:05:03Z
- **Completed:** 2026-03-06T03:08:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- InFlightBuffer ring buffer with brute-force cosine similarity search (6 unit tests)
- DedupConfig struct with threshold=0.85, buffer_capacity=256, enabled=false defaults
- NoveltyConfig type alias preserves all existing code (14 crates compile clean)
- Settings.dedup field loads from [dedup] or [novelty] TOML section

## Task Commits

Each task was committed atomically:

1. **Task 1: Create InFlightBuffer ring buffer in memory-types** - `fc93c2b` (feat)
2. **Task 2: Evolve NoveltyConfig to DedupConfig and wire into Settings** - `50291ab` (feat)

## Files Created/Modified
- `crates/memory-types/src/dedup.rs` - InFlightBuffer ring buffer with BufferEntry, cosine similarity search
- `crates/memory-types/src/config.rs` - DedupConfig replacing NoveltyConfig, buffer_capacity field, Settings.dedup
- `crates/memory-types/src/lib.rs` - pub mod dedup, re-exports for BufferEntry, InFlightBuffer, DedupConfig

## Decisions Made
- Cosine similarity implemented as dot product since CandleEmbedder pre-normalizes vectors
- NoveltyConfig kept as type alias (not deprecated) so all existing code compiles unchanged
- Settings.dedup uses serde(alias = "novelty") so existing config.toml files with [novelty] still work

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy collapsible-if in dedup.rs**
- **Found during:** Task 2 (clippy verification)
- **Issue:** Nested `if score >= threshold { if best.is_none_or(...) }` triggered clippy collapsible_if lint
- **Fix:** Collapsed to single `if` with `&&` condition
- **Files modified:** crates/memory-types/src/dedup.rs
- **Verification:** `cargo clippy -p memory-types -- -D warnings` passes clean
- **Committed in:** 50291ab (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug/lint)
**Impact on plan:** Trivial style fix required by strict clippy. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- InFlightBuffer and DedupConfig are ready for Plan 35-02 to wire into NoveltyChecker service layer
- NoveltyConfig type alias ensures seamless migration path
- All 70 memory-types tests + 1 doc-test pass

---
*Phase: 35-dedup-gate-foundation*
*Completed: 2026-03-06*
