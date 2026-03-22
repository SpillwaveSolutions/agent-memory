---
phase: 48-gemini-codex-converters
plan: 01
subsystem: installer
tags: [gemini, toml, converter, shell-escaping, settings-json, hooks]

requires:
  - phase: 46-installer-crate-foundation
    provides: RuntimeConverter trait, ConvertedFile, tool_maps, writer infrastructure
  - phase: 47-claude-opencode-converters
    provides: ClaudeConverter reference pattern, helpers.rs (reconstruct_md, rewrite_paths, value_to_yaml)
provides:
  - GeminiConverter with TOML command output, agent-to-skill conversion, settings.json generation
  - escape_shell_vars helper for ${VAR} to $VAR conversion
affects: [49-skills-converter, gemini-adapter-archive]

tech-stack:
  added: []
  patterns: [toml-serialization-for-commands, agent-to-skill-embedding, shell-var-escaping]

key-files:
  created: []
  modified:
    - crates/memory-installer/src/converters/gemini.rs
    - crates/memory-installer/src/converters/helpers.rs

key-decisions:
  - "Agents become skill directories with SKILL.md (Gemini has no separate agent format)"
  - "Tool list appended as ## Tools section in SKILL.md body (not frontmatter)"
  - "convert_hook returns None (hooks deferred to Phase 49)"
  - "settings.json uses _comment array and __managed_by marker for safe merge"

patterns-established:
  - "TOML command format: description + prompt fields via toml::to_string_pretty"
  - "Agent-to-skill embedding: agents converted to skill directories for runtimes without agent concept"
  - "Shell variable escaping: ${VAR} to $VAR for Gemini template compatibility"

requirements-completed: [GEM-01, GEM-02, GEM-03, GEM-04, GEM-05, GEM-06]

duration: 2min
completed: 2026-03-18
---

# Phase 48 Plan 01: Gemini Converter Summary

**GeminiConverter producing TOML commands, agent-to-skill SKILL.md, and settings.json with 6 PascalCase hook events plus escape_shell_vars helper**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-18T02:40:57Z
- **Completed:** 2026-03-18T02:43:17Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- Full GeminiConverter implementation with all 6 GEM-* requirements satisfied
- TOML command conversion with description + prompt fields using toml crate
- Agent-to-skill conversion with color/skills field stripping and MCP/Task tool exclusion
- settings.json generation with 6 PascalCase hook event types and managed JSON marker
- escape_shell_vars helper converting ${VAR} to $VAR for Gemini template compatibility
- 13 unit tests (8 Gemini converter + 5 escape_shell_vars helper)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add escape_shell_vars helper and implement GeminiConverter** - `9509c62` (feat)

## Files Created/Modified
- `crates/memory-installer/src/converters/gemini.rs` - Full GeminiConverter implementation (convert_command, convert_agent, convert_skill, convert_hook, generate_guidance) with 8 tests
- `crates/memory-installer/src/converters/helpers.rs` - Added escape_shell_vars function with 5 unit tests

## Decisions Made
- Agents become skill directories with SKILL.md since Gemini has no separate agent file format
- Tool list appended as "## Tools" section in SKILL.md body rather than frontmatter (Gemini skills don't have a tools frontmatter field)
- convert_hook returns None -- hook script copying deferred to Phase 49
- settings.json uses _comment array and __managed_by marker for safe merge on future installs
- build_gemini_tools reads from agent's allowed-tools frontmatter array (not KNOWN_TOOLS constant)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- GeminiConverter complete, ready for Phase 48 Plan 02 (CodexConverter)
- escape_shell_vars helper available for any future converter that needs shell variable escaping
- Phase 49 can implement hook script copying (convert_hook currently returns None)

---
*Phase: 48-gemini-codex-converters*
*Completed: 2026-03-18*
