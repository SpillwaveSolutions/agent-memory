---
phase: 49-copilot-skills-hooks
plan: 01
subsystem: installer
tags: [copilot, converter, hooks, skills, agents]

requires:
  - phase: 46-installer-crate-foundation
    provides: RuntimeConverter trait, tool_maps, helpers, types
  - phase: 48-gemini-codex-converters
    provides: Codex and Gemini converter patterns to follow
provides:
  - CopilotConverter with convert_command, convert_agent, convert_skill, generate_guidance
  - Copilot hook JSON generation (memory-hooks.json)
  - Embedded hook capture script (memory-capture.sh)
affects: [50-skills-converter]

tech-stack:
  added: [include_str! for script embedding]
  patterns: [agent.md format with YAML frontmatter tools array and infer flag]

key-files:
  created: []
  modified:
    - crates/memory-installer/src/converters/copilot.rs
    - crates/memory-installer/src/converter.rs

key-decisions:
  - "target_dir uses .github/ (not .github/copilot/) matching Copilot CLI discovery"
  - "Agents use .agent.md format with tools array and infer:true in YAML frontmatter"
  - "Hook script embedded via include_str! from canonical adapter source"
  - "Hook JSON uses camelCase events and Copilot-specific fields (bash, timeoutSec, comment)"

patterns-established:
  - "Copilot agent format: .github/agents/<name>.agent.md with tools array in frontmatter"
  - "Hook generation via generate_guidance (not per-hook convert_hook)"

requirements-completed: [COP-01, COP-02, COP-03, HOOK-01, HOOK-02, HOOK-03]

duration: 2min
completed: 2026-03-18
---

# Phase 49 Plan 01: Copilot Converter Summary

**CopilotConverter producing .github/skills/, .github/agents/, and .github/hooks/ with camelCase hook events and embedded capture script**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-18T05:02:34Z
- **Completed:** 2026-03-18T05:04:58Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Full CopilotConverter implementation with convert_command, convert_agent, convert_skill, and generate_guidance
- Hook JSON generation with version:1, 5 camelCase events, and Copilot-specific field names (bash, timeoutSec, comment)
- Embedded hook capture script via include_str! from canonical adapter
- 12 unit tests covering all converter methods, hook generation, and field validation

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement CopilotConverter convert_command, convert_agent, convert_skill** - `7dfb891` (feat)
2. **Task 2: Implement hook generation and update stub test** - `1142ad5` (fix)

## Files Created/Modified
- `crates/memory-installer/src/converters/copilot.rs` - Full CopilotConverter implementation with 12 tests
- `crates/memory-installer/src/converter.rs` - Updated stub test to only assert Skills returns empty

## Decisions Made
- target_dir uses `.github/` for project scope (not `.github/copilot/`) to match Copilot CLI discovery conventions
- Agents produce `.agent.md` format with tools array and `infer: true` in YAML frontmatter (not Codex skill-directory pattern)
- Hook capture script embedded via `include_str!` from canonical adapter source for single source of truth
- Hook JSON uses Copilot-specific field names (`bash`, `timeoutSec`, `comment`) distinct from Gemini (`command`, `timeout`, `description`)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy single-element-loop warning**
- **Found during:** Task 2
- **Issue:** After removing Copilot from `[Runtime::Copilot, Runtime::Skills]`, the remaining `[Runtime::Skills]` triggered clippy single_element_loop lint
- **Fix:** Unwound the loop into a direct assertion block
- **Files modified:** crates/memory-installer/src/converter.rs
- **Verification:** clippy passes clean
- **Committed in:** 1142ad5

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary clippy compliance fix. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Copilot converter complete, only Skills converter remains as a stub
- All 97 memory-installer tests passing
- Clippy clean

---
*Phase: 49-copilot-skills-hooks*
*Completed: 2026-03-18*
