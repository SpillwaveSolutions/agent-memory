---
phase: 50-integration-testing-migration
plan: 02
subsystem: infra
tags: [archival, migration, adapters, installer]

requires:
  - phase: 50-integration-testing-migration-01
    provides: "E2E converter tests proving installer generates correct output"
provides:
  - "3 archived adapter directories with README stubs pointing to memory-installer"
  - "12K+ LOC removed from obsolete adapter files"
affects: [future-cleanup, release-notes]

tech-stack:
  added: []
  patterns: [archive-with-stub-readme, preserve-compile-dependencies]

key-files:
  created: []
  modified:
    - plugins/memory-copilot-adapter/README.md
    - plugins/memory-gemini-adapter/README.md
    - plugins/memory-opencode-plugin/README.md

key-decisions:
  - "Preserved memory-capture.sh for include_str! compile dependency in CopilotConverter"

patterns-established:
  - "Archive pattern: replace adapter contents with README stub, retain for one release cycle"

requirements-completed: [MIG-03]

duration: 1min
completed: 2026-03-22
---

# Phase 50 Plan 02: Archive Old Adapter Directories Summary

**Archived 3 legacy adapter directories (copilot, gemini, opencode) with README stubs pointing to memory-installer, removing 12K+ lines of obsolete plugin files while preserving the include_str! compile dependency**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-22T02:44:09Z
- **Completed:** 2026-03-22T02:45:31Z
- **Tasks:** 2
- **Files modified:** 51 (48 deleted, 3 modified)

## Accomplishments
- Archived memory-copilot-adapter: deleted plugin.json, .gitignore, agents, hooks config, all skills; preserved memory-capture.sh
- Archived memory-gemini-adapter: deleted .gitignore, entire .gemini/ directory tree (commands, hooks, settings, skills)
- Archived memory-opencode-plugin: deleted .gitignore, entire .opencode/ directory tree (agents, commands, plugin, skills)
- Full workspace QA validation passed (format, clippy, all tests, docs)

## Task Commits

Each task was committed atomically:

1. **Task 1: Archive 3 adapter directories with README stubs** - `988216e` (chore)
2. **Task 2: Final workspace validation after archival** - no code changes (validation-only task)

## Files Created/Modified
- `plugins/memory-copilot-adapter/README.md` - Archive stub pointing to memory-installer (notes preserved compile dependency)
- `plugins/memory-gemini-adapter/README.md` - Archive stub pointing to memory-installer
- `plugins/memory-opencode-plugin/README.md` - Archive stub pointing to memory-installer
- 48 files deleted across copilot skills, gemini .gemini/, opencode .opencode/ trees

## Decisions Made
- Preserved memory-capture.sh as required by CopilotConverter's include_str! macro path
- Active plugin directories (memory-query-plugin, memory-setup-plugin) and installer-sources.json left untouched

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 50 complete: all converter E2E tests pass (Plan 01) and old adapters archived (Plan 02)
- Old adapter directories retained for one release cycle; can be fully removed in future version
- memory-capture.sh must remain until CopilotConverter include_str! reference is refactored

---
*Phase: 50-integration-testing-migration*
*Completed: 2026-03-22*
