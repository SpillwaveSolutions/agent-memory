---
phase: 48-gemini-codex-converters
plan: 02
subsystem: installer
tags: [codex, converter, skill-directories, agents-md, sandbox]

requires:
  - phase: 46-installer-crate-foundation
    provides: RuntimeConverter trait, ConvertedFile type, tool_maps, types
  - phase: 47-claude-opencode-converters
    provides: Shared helpers (reconstruct_md, rewrite_paths), converter pattern reference
provides:
  - Full CodexConverter implementation (commands, agents, skills, guidance)
  - AGENTS.md generation with sandbox permission guidance
  - Sandbox permission mapping per agent
affects: [49-skills-converter, installer-integration]

tech-stack:
  added: []
  patterns: [command-to-skill-directory, agents-md-generation, sandbox-permission-mapping]

key-files:
  created: []
  modified:
    - crates/memory-installer/src/converters/codex.rs
    - crates/memory-installer/src/converter.rs

key-decisions:
  - "Codex commands become skills/{name}/SKILL.md with YAML frontmatter (name, description)"
  - "Codex agents become orchestration skills with mapped tools and sandbox sections"
  - "AGENTS.md generated with skills list, agent descriptions, and sandbox recommendations"
  - "Tool deduplication applied after mapping (Write and Edit both map to edit)"

patterns-established:
  - "Codex skill directory: skills/{name}/SKILL.md with YAML frontmatter"
  - "Sandbox permission helper: sandbox_for_agent() returns workspace-write or read-only"

requirements-completed: [CDX-01, CDX-02, CDX-03, CDX-04]

duration: 2min
completed: 2026-03-18
---

# Phase 48 Plan 02: Codex Converter Summary

**CodexConverter producing SKILL.md directories for commands/agents plus AGENTS.md with sandbox permission guidance**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-18T02:41:01Z
- **Completed:** 2026-03-18T02:42:51Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- Implemented full CodexConverter with convert_command, convert_agent, convert_skill, convert_hook, generate_guidance
- Commands become skills/{name}/SKILL.md with YAML frontmatter (name, description) and rewritten paths
- Agents become orchestration skills with mapped tools (MCP excluded, deduped), sandbox section
- AGENTS.md generated with skills list, agent descriptions, and per-agent sandbox recommendations
- Sandbox mapping: setup-troubleshooter gets workspace-write, all others get read-only
- Updated stub test to only check Copilot and Skills (Codex now implemented)
- Added 7 unit tests covering all CDX-01 through CDX-04 requirements

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement CodexConverter and update stub test** - `daafdf8` (feat)

## Files Created/Modified
- `crates/memory-installer/src/converters/codex.rs` - Full CodexConverter implementation with 7 unit tests
- `crates/memory-installer/src/converter.rs` - Updated stub test to only check Copilot and Skills

## Decisions Made
- Codex commands become skills/{name}/SKILL.md with YAML frontmatter (name, description)
- Agents become orchestration skills with tools section (deduped after mapping) and sandbox section
- AGENTS.md contains header, available skills list, agents section with sandbox recommendations
- Tool deduplication applied after mapping since Write and Edit both map to "edit" in Codex

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CodexConverter complete, 4 of 6 runtimes now implemented (Claude, OpenCode, Gemini, Codex)
- Copilot and Skills remain stubs for Phase 49
- All 88 memory-installer tests pass, clippy clean

---
*Phase: 48-gemini-codex-converters*
*Completed: 2026-03-18*
