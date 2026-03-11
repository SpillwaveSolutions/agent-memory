---
phase: 01-foundation
plan: 02
subsystem: types
tags: [rust, serde, chrono, domain-types, config, toml]

# Dependency graph
requires:
  - phase: 01-00
    provides: workspace scaffolding, crate structure
provides:
  - Event, EventRole, EventType for conversation storage
  - TocNode, TocLevel, TocBullet for TOC hierarchy
  - Grip for provenance anchoring
  - OutboxEntry for async index updates
  - Settings with layered configuration loading
  - MemoryError unified error type
affects: [01-storage, 01-service, 01-daemon, 02-toc-building, 03-grips]

# Tech tracking
tech-stack:
  added: [config, directories]
  patterns: [serde-serialization, builder-pattern, layered-config]

key-files:
  created:
    - crates/memory-types/src/event.rs
    - crates/memory-types/src/outbox.rs
    - crates/memory-types/src/toc.rs
    - crates/memory-types/src/grip.rs
    - crates/memory-types/src/config.rs
    - crates/memory-types/src/error.rs
  modified:
    - crates/memory-types/src/lib.rs
    - crates/memory-types/Cargo.toml

key-decisions:
  - "Used directories crate instead of dirs (already in workspace)"
  - "Environment vars prefixed with MEMORY_ for config override"
  - "Timestamps stored as milliseconds for consistency"

patterns-established:
  - "All domain types implement Serialize/Deserialize"
  - "Builder pattern with with_* methods for optional fields"
  - "to_bytes/from_bytes methods for JSON serialization"
  - "Layered config: defaults -> file -> env vars"

# Metrics
duration: 12min
completed: 2026-01-29
---

# Phase 1 Plan 02: Domain Types Summary

**Domain types with serde serialization: Event/TocNode/Grip/Settings with layered config loading via config crate**

## Performance

- **Duration:** 12 min
- **Started:** 2026-01-29T21:45:00Z
- **Completed:** 2026-01-29T21:56:37Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- Event struct with all required fields per ING-02 (session_id, timestamp, role, text, metadata)
- TocNode with full hierarchy support (Year -> Month -> Week -> Day -> Segment)
- Grip for provenance anchoring with event range references
- Settings with layered config loading (defaults -> file -> env vars)
- MultiAgentMode enum (Separate/Unified) per STOR-06

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Event and OutboxEntry types** - `c715a2d` (feat)
2. **Task 2: Create TocNode and Grip types** - `53dd9db` (feat)
3. **Task 3: Create Settings configuration** - `f9dce93` (feat)

## Files Created/Modified

- `crates/memory-types/src/event.rs` - Event, EventRole, EventType for conversation storage
- `crates/memory-types/src/outbox.rs` - OutboxEntry for async index updates
- `crates/memory-types/src/toc.rs` - TocNode, TocLevel, TocBullet for TOC hierarchy
- `crates/memory-types/src/grip.rs` - Grip for provenance anchoring
- `crates/memory-types/src/config.rs` - Settings, SummarizerSettings, MultiAgentMode
- `crates/memory-types/src/error.rs` - MemoryError unified error type
- `crates/memory-types/src/lib.rs` - Module exports and re-exports
- `crates/memory-types/Cargo.toml` - Added config, directories dependencies

## Decisions Made

1. **directories crate instead of dirs** - The workspace already had `directories = "6.0"`, which is functionally equivalent to `dirs`. Used the existing workspace dependency.

2. **Environment variable prefix MEMORY_** - Config crate loads env vars with MEMORY_ prefix (e.g., MEMORY_DB_PATH, MEMORY_GRPC_PORT) for clear namespacing.

3. **Timestamps as milliseconds** - All DateTime<Utc> fields serialized via `chrono::serde::ts_milliseconds` for consistent integer representation in storage.

4. **MemoryError created early** - Created error.rs with unified error type as part of Task 1 since config.rs needs it. This was a necessary addition not explicitly specified in the plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created error.rs for MemoryError type**
- **Found during:** Task 1 (Event types)
- **Issue:** config.rs requires MemoryError type but error.rs wasn't in Task 1 scope
- **Fix:** Created minimal error.rs with MemoryError enum in Task 1
- **Files modified:** crates/memory-types/src/error.rs
- **Verification:** cargo build -p memory-types compiles
- **Committed in:** c715a2d (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for config.rs to compile. No scope creep.

## Issues Encountered

None - plan executed smoothly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All domain types ready for storage layer (01-01) and service layer (01-03)
- Event type ready for IngestEvent RPC
- TocNode ready for TOC building in Phase 2
- Settings ready for daemon configuration in 01-04
- No blockers

---
*Phase: 01-foundation*
*Completed: 2026-01-29*
