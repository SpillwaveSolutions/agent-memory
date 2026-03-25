---
phase: 58-claude-registration-metadata
plan: 01
subsystem: installer
tags: [claude-code, registry, json, plugin-metadata, chrono]

# Dependency graph
requires:
  - phase: 47-claude-opencode-converters
    provides: RuntimeConverter trait, ClaudeConverter scaffold, path rewriting
provides:
  - Claude Code runtime registration via 3 JSON registry files
  - plugin.json metadata with version as single source of truth
  - Old version directory cleanup on re-install
affects: [59-uninstall-status]

# Tech tracking
tech-stack:
  added: [chrono (timestamps for registry files)]
  patterns: [JSON merge-upsert for registry files, version-from-plugin.json pattern]

key-files:
  created:
    - plugins/memory-query-plugin/.claude-plugin/plugin.json
  modified:
    - crates/memory-installer/src/converters/claude.rs
    - crates/memory-installer/Cargo.toml

key-decisions:
  - "Registry helpers accept &Path for home dir to enable isolated tempdir testing"
  - "installedAt preserved from existing entry on re-install; lastUpdated always refreshed"
  - "Corrupt/missing JSON falls back to empty object rather than erroring"

patterns-established:
  - "build_* helpers for each registry file: read-merge-write with serde_json"
  - "cleanup_old_versions: iterate sibling dirs, remove non-matching versions"
  - "read_plugin_version: read version from plugin.json, fallback to 1.0.0"

requirements-completed: [CREG-01, CREG-02, CREG-03, CREG-04, CREG-05, CREG-06, META-01, META-02, META-03]

# Metrics
duration: 3min
completed: 2026-03-25
---

# Phase 58 Plan 01: Claude Registration Metadata Summary

**Claude Code runtime registration via 3 JSON registry files (known_marketplaces, installed_plugins, settings) with plugin.json version source-of-truth and old-version cleanup**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-25T22:08:45Z
- **Completed:** 2026-03-25T22:12:21Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Created plugin.json with name=memory-query, version=1.0.0 as single source of truth for plugin metadata
- Implemented generate_guidance() producing 3 registry ConvertedFile entries for Global scope
- Plugin key "memory-query@agent-memory" used consistently across installed_plugins.json and settings.json
- Old version directory cleanup prevents cache accumulation on updates
- 13 new tests covering all 9 requirements plus edge cases (corrupt JSON, missing plugin.json, scope filtering)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create plugin.json and add chrono dependency** - `b806364` (feat)
2. **Task 2: Implement Claude registration in generate_guidance with tests** - `aedbfb9` (feat)

## Files Created/Modified
- `plugins/memory-query-plugin/.claude-plugin/plugin.json` - Plugin metadata with name, version, description
- `crates/memory-installer/Cargo.toml` - Added chrono workspace dependency
- `crates/memory-installer/src/converters/claude.rs` - Registration logic with 7 helper functions and 13 new tests

## Decisions Made
- Registry helpers accept `&Path` for home dir to enable isolated tempdir testing without mocking
- installedAt preserved from existing entry on re-install; lastUpdated always refreshed (matches Python reference)
- Corrupt/missing JSON falls back to empty object rather than erroring (graceful degradation)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 59 (uninstall + status) can now proceed -- knows exactly which 3 registry files to clean up
- Registration format matches Python codebase-mentor reference implementation
- All tests pass, clippy clean, fmt clean

---
*Phase: 58-claude-registration-metadata*
*Completed: 2026-03-25*
