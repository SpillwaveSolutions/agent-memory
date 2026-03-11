---
phase: 19-opencode-commands-and-skills
plan: 02
subsystem: plugins
tags: [opencode, skills, memory-query, retrieval-policy, topic-graph, portable-skills]

# Dependency graph
requires:
  - phase: 19-01
    provides: "OpenCode plugin directory structure and .gitignore override"
provides:
  - "memory-query skill in OpenCode format"
  - "retrieval-policy skill in OpenCode format"
  - "topic-graph skill in OpenCode format"
affects: [19-03, 19-04, 19-05, 20]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Portable skill format: SKILL.md + references/ works across Claude Code and OpenCode"]

key-files:
  created:
    - plugins/memory-opencode-plugin/.opencode/skill/memory-query/SKILL.md
    - plugins/memory-opencode-plugin/.opencode/skill/memory-query/references/command-reference.md
    - plugins/memory-opencode-plugin/.opencode/skill/retrieval-policy/SKILL.md
    - plugins/memory-opencode-plugin/.opencode/skill/retrieval-policy/references/command-reference.md
    - plugins/memory-opencode-plugin/.opencode/skill/topic-graph/SKILL.md
    - plugins/memory-opencode-plugin/.opencode/skill/topic-graph/references/command-reference.md
  modified: []

key-decisions:
  - "Direct copy of SKILL.md files - skill format is fully portable between Claude Code and OpenCode"

patterns-established:
  - "Skill portability: same SKILL.md with YAML frontmatter works in both .claude/skills/ and .opencode/skill/"
  - "Reference subdirectory: each skill has references/command-reference.md for CLI details"

# Metrics
duration: 2min
completed: 2026-02-09
---

# Phase 19 Plan 02: Port Core Skills to OpenCode Summary

**Three core skills (memory-query, retrieval-policy, topic-graph) ported to OpenCode format with SKILL.md and command references**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-09T21:03:51Z
- **Completed:** 2026-02-09T21:05:35Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- Ported memory-query skill with tier-aware retrieval documentation and full CLI reference
- Ported retrieval-policy skill with intent classification and fallback chain documentation
- Ported topic-graph skill with time-decayed importance scoring and lifecycle documentation
- All skill names match directory names (lowercase, hyphenated)
- All descriptions contain trigger phrases within 1-1024 character limit

## Task Commits

Each task was committed atomically:

1. **Task 1: Port memory-query skill** - `0608a8e` (feat)
2. **Task 2: Port retrieval-policy skill** - `160dd40` (feat)
3. **Task 3: Port topic-graph skill** - `01f20bf` (feat)

## Files Created/Modified
- `plugins/memory-opencode-plugin/.opencode/skill/memory-query/SKILL.md` - Core memory query skill with tier-aware retrieval
- `plugins/memory-opencode-plugin/.opencode/skill/memory-query/references/command-reference.md` - Full CLI and gRPC reference for query commands
- `plugins/memory-opencode-plugin/.opencode/skill/retrieval-policy/SKILL.md` - Retrieval policy with intent classification and fallback chains
- `plugins/memory-opencode-plugin/.opencode/skill/retrieval-policy/references/command-reference.md` - CLI and gRPC reference for retrieval commands
- `plugins/memory-opencode-plugin/.opencode/skill/topic-graph/SKILL.md` - Topic graph exploration with time-decayed scoring
- `plugins/memory-opencode-plugin/.opencode/skill/topic-graph/references/command-reference.md` - CLI and gRPC reference for topic commands

## Decisions Made
- Direct copy of SKILL.md files without modification - the skill format (YAML frontmatter + markdown body + references/) is fully portable between Claude Code and OpenCode plugin formats

## Deviations from Plan

None - plan executed exactly as written.

Note: The `.gitignore` override for `.opencode` directories was already handled in plan 19-01, so no blocking issue occurred.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Three core skills now available in OpenCode plugin format
- Ready for plan 19-03 (additional skills or commands)
- OpenCode skill directory now contains: bm25-search (from 19-01), memory-query, retrieval-policy, topic-graph

## Self-Check: PASSED

- All 6 created files verified present on disk
- All 3 task commits verified in git log (0608a8e, 160dd40, 01f20bf)

---
*Phase: 19-opencode-commands-and-skills*
*Completed: 2026-02-09*
