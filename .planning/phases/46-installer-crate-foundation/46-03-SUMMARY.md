---
phase: 46-installer-crate-foundation
plan: 03
subsystem: installer
tags: [rust, tool-maps, writer, dry-run, managed-sections, cli-pipeline]

requires:
  - phase: 46-installer-crate-foundation-01
    provides: "CLI skeleton, types, converter trait, converter stubs"
  - phase: 46-installer-crate-foundation-02
    provides: "Plugin parser with parse_sources() and parse_md_file()"
provides:
  - "map_tool() function for 11 tools x 6 runtimes"
  - "write_files() with dry-run support and WriteReport"
  - "merge_managed_section() with 3-case merge logic"
  - "remove_managed_section() for future uninstall"
  - "MANAGED_BEGIN/MANAGED_END compatibility contract constants"
  - "Working end-to-end install pipeline in main.rs"
affects: [47-claude-opencode-converters, 48-gemini-codex-converters, 49-generic-skills-hooks]

tech-stack:
  added: []
  patterns: [static-match-tool-mapping, write-interceptor-dry-run, managed-section-markers]

key-files:
  created:
    - crates/memory-installer/src/tool_maps.rs
    - crates/memory-installer/src/writer.rs
  modified:
    - crates/memory-installer/src/main.rs
    - crates/memory-installer/src/lib.rs

key-decisions:
  - "Used match expression for tool maps instead of HashMap (compile-time exhaustive, zero overhead)"
  - "Callers handle mcp__* prefix check before calling map_tool (keeps static return type)"
  - "Re-exported marker constants from writer.rs for API convenience"

patterns-established:
  - "map_tool(runtime, claude_name) -> Option<&'static str> for centralized tool name translation"
  - "write_files(files, dry_run) write-interceptor pattern for all converter output"
  - "merge_managed_section 3-case pattern: new file, existing with markers, existing without"

requirements-completed: [INST-04, INST-05, INST-06, INST-07]

duration: 4min
completed: 2026-03-17
---

# Phase 46 Plan 03: Converter Trait & Tool Maps Summary

**Static match-based tool mapping for 11 tools x 6 runtimes, file writer with dry-run and managed-section markers, and working end-to-end install pipeline**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-17T20:05:08Z
- **Completed:** 2026-03-17T20:08:59Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Centralized tool mapping table covering all 11 Claude tools across all 6 runtimes with Gemini Task correctly excluded
- File writer with dry-run mode (prints CREATE/OVERWRITE report without writing) and real write mode (creates parent dirs, writes content)
- Managed-section merge/remove logic for safe injection into shared config files
- main.rs wired as complete pipeline: parse_sources -> select_converter -> convert_* -> write_files
- 32 new unit tests (18 tool_maps + 14 writer), all passing

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement tool_maps.rs with 11-tool x 6-runtime mapping table** - `4edfc70` (feat)
2. **Task 2: Implement writer.rs with dry-run, managed-section markers, and wire main.rs pipeline** - `b12ed8f` (feat)

## Files Created/Modified
- `crates/memory-installer/src/tool_maps.rs` - Static match-based map_tool() for 11 tools x 6 runtimes with KNOWN_TOOLS constant
- `crates/memory-installer/src/writer.rs` - write_files(), merge_managed_section(), remove_managed_section(), WriteReport, marker constants
- `crates/memory-installer/src/main.rs` - Full install pipeline: parse -> convert -> write with auto-discovery and error handling
- `crates/memory-installer/src/lib.rs` - Added tool_maps and writer module declarations

## Decisions Made
- Used match expression for tool maps instead of HashMap -- compile-time exhaustive coverage, zero runtime overhead, unknown tools naturally fall through to None
- Callers handle mcp__* prefix check before calling map_tool -- keeps the return type as simple Option<&'static str> without Cow complexity
- Re-exported MANAGED_BEGIN/MANAGED_END from writer.rs for API convenience while keeping canonical definitions in types.rs

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 46 foundation is complete: CLI, parser, types, converter trait+stubs, tool maps, writer, and pipeline wiring
- Phase 47 (Claude & OpenCode converters) and Phase 48 (Gemini & Codex converters) can proceed independently
- All converter stubs implement RuntimeConverter trait and return empty vecs
- End-to-end dry-run verified: `memory-installer install --agent claude --project --dry-run` runs successfully

---
*Phase: 46-installer-crate-foundation*
*Completed: 2026-03-17*
