---
phase: 46-installer-crate-foundation
plan: 01
subsystem: installer
tags: [rust, clap, cli, converter-trait, gray_matter, walkdir, plugin-installer]

requires:
  - phase: 45-canonical-source-consolidation
    provides: Plugin source directories with marketplace.json manifests
provides:
  - memory-installer workspace crate with CLI skeleton
  - RuntimeConverter trait with 7 methods
  - 6 converter stubs (Claude, OpenCode, Gemini, Codex, Copilot, Skills)
  - select_converter dispatch table
  - PluginBundle, ConvertedFile, InstallConfig, Runtime types
  - Managed-section marker constants
affects: [46-02, 46-03, 47, 48, 49]

tech-stack:
  added: [gray_matter 0.3.2, walkdir 2.5, shellexpand 3.1]
  patterns: [RuntimeConverter trait dispatch, stateless converter stubs, managed-section markers]

key-files:
  created:
    - crates/memory-installer/Cargo.toml
    - crates/memory-installer/src/main.rs
    - crates/memory-installer/src/lib.rs
    - crates/memory-installer/src/types.rs
    - crates/memory-installer/src/converter.rs
    - crates/memory-installer/src/converters/mod.rs
    - crates/memory-installer/src/converters/claude.rs
    - crates/memory-installer/src/converters/opencode.rs
    - crates/memory-installer/src/converters/gemini.rs
    - crates/memory-installer/src/converters/codex.rs
    - crates/memory-installer/src/converters/copilot.rs
    - crates/memory-installer/src/converters/skills.rs
  modified:
    - Cargo.toml

key-decisions:
  - "Used owned Strings in all types (not borrowed) for simplicity with Box<dyn RuntimeConverter>"
  - "Used Box<dyn RuntimeConverter> trait objects for extensibility over enum dispatch"
  - "Managed-section markers defined as const strings in types.rs as compatibility contracts"

patterns-established:
  - "RuntimeConverter: stateless trait with 7 methods, one impl per runtime"
  - "Converter stubs: return empty Vec/None, implementations filled in Phases 47-49"
  - "select_converter: exhaustive match on Runtime enum returning Box<dyn RuntimeConverter>"

requirements-completed: [INST-01, INST-03]

duration: 4min
completed: 2026-03-17
---

# Phase 46 Plan 01: Installer Crate Foundation Summary

**memory-installer crate with clap CLI, RuntimeConverter trait, 6 converter stubs, and full type system -- zero tokio dependency**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-17T19:49:34Z
- **Completed:** 2026-03-17T19:53:33Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments
- Created memory-installer workspace crate with standalone binary (no tokio, no gRPC, no RocksDB)
- Defined RuntimeConverter trait with 7 methods: name, target_dir, convert_command, convert_agent, convert_skill, convert_hook, generate_guidance
- Implemented 6 converter stubs with correct target_dir paths per runtime
- CLI skeleton with install subcommand and all required flags (--agent, --project, --global, --dir, --dry-run, --source)
- Added gray_matter and walkdir as workspace dependencies for Plans 02-03

## Task Commits

Each task was committed atomically:

1. **Task 1: Create memory-installer crate with Cargo.toml, types, and CLI skeleton** - `be6661e` (feat)
2. **Task 2: Create RuntimeConverter trait, 6 converter stubs, and dispatch table** - `c69a21c` (test)
3. **Formatting fix** - `aa80fff` (chore)

## Files Created/Modified
- `Cargo.toml` - Added memory-installer to workspace members, gray_matter and walkdir to workspace deps
- `crates/memory-installer/Cargo.toml` - Crate definition with all dependencies, no tokio
- `crates/memory-installer/src/main.rs` - CLI entry point with clap derive parser, scope validation
- `crates/memory-installer/src/lib.rs` - Public module declarations for types, converter, converters
- `crates/memory-installer/src/types.rs` - Runtime, InstallScope, InstallConfig, PluginBundle, ConvertedFile, managed-section constants
- `crates/memory-installer/src/converter.rs` - RuntimeConverter trait definition + 8 dispatch tests
- `crates/memory-installer/src/converters/mod.rs` - select_converter dispatch table
- `crates/memory-installer/src/converters/claude.rs` - ClaudeConverter stub (.claude/plugins/memory-plugin)
- `crates/memory-installer/src/converters/opencode.rs` - OpenCodeConverter stub (.opencode)
- `crates/memory-installer/src/converters/gemini.rs` - GeminiConverter stub (.gemini)
- `crates/memory-installer/src/converters/codex.rs` - CodexConverter stub (.codex)
- `crates/memory-installer/src/converters/copilot.rs` - CopilotConverter stub (.github/copilot)
- `crates/memory-installer/src/converters/skills.rs` - SkillsConverter stub (skills/)

## Decisions Made
- Used owned Strings in all PluginBundle types (not borrowed) to avoid lifetime complexity with trait objects
- Used Box<dyn RuntimeConverter> trait objects for select_converter dispatch (extensibility over enum dispatch)
- Defined managed-section marker constants in types.rs with explicit compatibility contract documentation
- shellexpand added as direct crate dependency (not workspace) since it was not in workspace deps

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed cargo fmt formatting**
- **Found during:** Task 2 verification
- **Issue:** generate_guidance method signature was split across multiple lines inconsistently
- **Fix:** Ran cargo fmt to normalize formatting
- **Files modified:** converter.rs, all 6 converter stub files
- **Verification:** cargo fmt --check passes
- **Committed in:** aa80fff

---

**Total deviations:** 1 auto-fixed (1 formatting bug)
**Impact on plan:** Trivial formatting fix, no scope change.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- RuntimeConverter trait and all types ready for Plan 02 (parser) and Plan 03 (tool_maps + writer)
- Converter stubs ready for Phases 47-49 to fill in real conversion logic
- CLI skeleton ready to wire parser and writer in subsequent plans

## Self-Check: PASSED

All 12 created files verified on disk. All 3 commits (be6661e, c69a21c, aa80fff) verified in git log.

---
*Phase: 46-installer-crate-foundation*
*Completed: 2026-03-17*
