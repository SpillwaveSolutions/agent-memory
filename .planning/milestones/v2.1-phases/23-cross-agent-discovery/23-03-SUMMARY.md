---
phase: 23-cross-agent-discovery
plan: 03
subsystem: documentation
tags: [clod, toml, cli, documentation, adapters, cross-agent]

# Dependency graph
requires:
  - phase: 23-cross-agent-discovery (plan 01)
    provides: ListAgents/GetAgentActivity RPCs, agents CLI commands
  - phase: 18-agent-tagging-infrastructure
    provides: Event.agent field, AgentAdapter trait, --agent CLI filter
  - phase: 19-opencode-commands-skills
    provides: OpenCode plugin adapter
  - phase: 21-gemini-cli-adapter
    provides: Gemini adapter with TOML commands and shell hooks
  - phase: 22-copilot-cli-adapter
    provides: Copilot adapter with skills and shell hooks
provides:
  - CLOD format specification (docs/adapters/clod-format.md)
  - CLOD parser and converter module (crates/memory-daemon/src/clod.rs)
  - Four adapter generators (claude, opencode, gemini, copilot)
  - CLI `clod convert` and `clod validate` subcommands
  - Cross-agent usage guide (docs/adapters/cross-agent-guide.md)
  - Adapter authoring guide (docs/adapters/authoring-guide.md)
  - Updated top-level README with Supported Agents table and docs links
  - Updated UPGRADING.md with v2.2 multi-agent ecosystem section
affects: [milestone-completion, community-adoption, adapter-development]

# Tech tracking
tech-stack:
  added: [toml 0.8]
  patterns: [CLOD format for cross-adapter command generation, adapter_dir/adapter_ext resolution helpers]

key-files:
  created:
    - docs/adapters/clod-format.md
    - docs/adapters/cross-agent-guide.md
    - docs/adapters/authoring-guide.md
    - crates/memory-daemon/src/clod.rs
  modified:
    - Cargo.toml
    - crates/memory-daemon/Cargo.toml
    - crates/memory-daemon/src/cli.rs
    - crates/memory-daemon/src/commands.rs
    - crates/memory-daemon/src/lib.rs
    - crates/memory-daemon/src/main.rs
    - docs/README.md
    - docs/UPGRADING.md

key-decisions:
  - "CLOD uses TOML format (matches Gemini commands, Rust ecosystem, human-readable)"
  - "Generate adapter files with simple format! macros instead of template engine (10-30 lines each)"
  - "UPGRADING.md uses v2.2.0 numbering for multi-agent ecosystem (v2.1 was ranking enhancements)"
  - "toml 0.8 crate added to workspace dependencies for CLOD parsing"

patterns-established:
  - "CLOD definition pattern: command + parameters + process + output + adapters sections"
  - "Per-adapter generator pattern: adapter_dir/adapter_ext resolution with defaults"
  - "yaml_escape helper for safe YAML frontmatter values in generated markdown"

# Metrics
duration: 14min
completed: 2026-02-10
---

# Phase 23 Plan 03: CLOD Spec, Converter CLI, and Documentation Summary

**CLOD format specification with TOML-based converter CLI, cross-agent usage guide, adapter authoring guide, and updated top-level docs for the multi-agent ecosystem**

## Performance

- **Duration:** 14 min
- **Started:** 2026-02-10T22:54:22Z
- **Completed:** 2026-02-10T23:08:41Z
- **Tasks:** 3
- **Files modified:** 12

## Accomplishments
- Defined the CLOD (Cross-Language Operation Definition) format with complete specification and 2 examples
- Implemented CLOD parser and converter module with 4 generator functions (claude, opencode, gemini, copilot)
- Added `clod convert` and `clod validate` CLI subcommands
- Created comprehensive cross-agent usage guide (319 lines) covering all 4 adapters, agent discovery, filtered queries, and common workflows
- Created adapter authoring guide (582 lines) covering AgentAdapter trait, event capture patterns, fail-open, redaction, skills, commands, agent tagging, config precedence, and testing
- Updated docs/README.md with Supported Agents table, Cross-Agent Discovery section, and documentation links
- Updated docs/UPGRADING.md with v2.2 Multi-Agent Ecosystem section

## Task Commits

Each task was committed atomically:

1. **Task 1: Create CLOD format specification and converter CLI subcommand** - `1f7e027` (feat)
2. **Task 2: Create cross-agent usage guide and adapter authoring guide** - `99e5d7b` (docs)
3. **Task 3: Update top-level documentation with cross-agent references** - `a4a073c` (docs)

## Files Created/Modified
- `docs/adapters/clod-format.md` - NEW: CLOD format specification with 2 complete examples and generated output samples
- `crates/memory-daemon/src/clod.rs` - NEW: CLOD parser, validator, and 4 adapter generators with 11 tests
- `crates/memory-daemon/src/cli.rs` - Added ClodCliCommand enum (Convert, Validate) with 3 CLI tests
- `crates/memory-daemon/src/commands.rs` - Added handle_clod_command with convert/validate logic
- `crates/memory-daemon/src/lib.rs` - Exported clod module and ClodCliCommand
- `crates/memory-daemon/src/main.rs` - Wired Clod command dispatch
- `Cargo.toml` - Added toml 0.8 to workspace dependencies
- `crates/memory-daemon/Cargo.toml` - Added toml dependency
- `docs/adapters/cross-agent-guide.md` - NEW: Cross-agent usage guide (4 adapters, discovery, filtering, workflows)
- `docs/adapters/authoring-guide.md` - NEW: Adapter authoring guide (trait, hooks, fail-open, redaction, testing)
- `docs/README.md` - Added Supported Agents table, Cross-Agent Discovery section, documentation links
- `docs/UPGRADING.md` - Added v2.2.0 Multi-Agent Ecosystem section with migration steps

## Decisions Made
- **TOML for CLOD format**: Chosen because Gemini already uses TOML commands, TOML is native to Rust ecosystem (Cargo.toml), supports comments (unlike JSON), and is more readable than YAML for nested structures.
- **Simple format! generators**: Each adapter generator is 10-30 lines using format! macros. No template engine needed -- the output formats are simple enough that direct string construction is clearer and has zero dependencies.
- **v2.2.0 for multi-agent release**: The existing UPGRADING.md already had v2.1.0 for Phase 16-17 (ranking enhancements). The multi-agent ecosystem is a separate release at v2.2.0.
- **toml 0.8 workspace dependency**: Added as workspace dependency for shared use. Currently only used by memory-daemon for CLOD parsing.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Build cache from parallel Plan 23-02 execution caused a stale proto artifact error (`GetTopTopicsRequest` missing `agent_filter`). Resolved by cleaning the build cache (`cargo clean -p memory-service`) to force a fresh proto rebuild. This was a transient issue from parallel plan execution, not a code problem.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 23 is now complete (all 3 plans executed)
- v2.1 Multi-Agent Ecosystem milestone deliverables are done
- All documentation is comprehensive and cross-referenced
- Full workspace clippy and doc build pass
- CLOD converter is ready for community adapter development

---
*Phase: 23-cross-agent-discovery*
*Completed: 2026-02-10*
