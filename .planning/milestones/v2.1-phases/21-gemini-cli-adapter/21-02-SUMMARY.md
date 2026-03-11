---
phase: 21-gemini-cli-adapter
plan: 02
subsystem: plugins
tags: [gemini-cli, toml-commands, skills, navigator, retrieval, agent-adapter]

# Dependency graph
requires:
  - phase: 19-opencode-commands-skills
    provides: SKILL.md files and command-reference.md files for memory-query, retrieval-policy, topic-graph, bm25-search, vector-search
  - phase: 18-agent-tagging
    provides: Agent field on events and queries for multi-agent filtering
provides:
  - 3 TOML command files for Gemini CLI slash commands (memory-search, memory-recent, memory-context)
  - 5 skill directories with SKILL.md and references/command-reference.md for Gemini CLI
  - Enhanced memory-query skill with embedded Navigator agent logic
affects: [21-03-PLAN, 23-cross-agent-discovery]

# Tech tracking
tech-stack:
  added: [gemini-cli-toml-commands]
  patterns: [navigator-as-skill, separate-skill-copies, toml-prompt-commands]

key-files:
  created:
    - plugins/memory-gemini-adapter/.gemini/commands/memory-search.toml
    - plugins/memory-gemini-adapter/.gemini/commands/memory-recent.toml
    - plugins/memory-gemini-adapter/.gemini/commands/memory-context.toml
    - plugins/memory-gemini-adapter/.gemini/skills/memory-query/SKILL.md
    - plugins/memory-gemini-adapter/.gemini/skills/memory-query/references/command-reference.md
    - plugins/memory-gemini-adapter/.gemini/skills/retrieval-policy/SKILL.md
    - plugins/memory-gemini-adapter/.gemini/skills/retrieval-policy/references/command-reference.md
    - plugins/memory-gemini-adapter/.gemini/skills/topic-graph/SKILL.md
    - plugins/memory-gemini-adapter/.gemini/skills/topic-graph/references/command-reference.md
    - plugins/memory-gemini-adapter/.gemini/skills/bm25-search/SKILL.md
    - plugins/memory-gemini-adapter/.gemini/skills/bm25-search/references/command-reference.md
    - plugins/memory-gemini-adapter/.gemini/skills/vector-search/SKILL.md
    - plugins/memory-gemini-adapter/.gemini/skills/vector-search/references/command-reference.md
  modified: []

key-decisions:
  - "Navigator agent logic embedded in memory-query SKILL.md (Gemini has no separate agent definition format)"
  - "Skills are separate copies from OpenCode plugin (not symlinks) for portability"
  - "TOML commands are self-contained with full instructions since Gemini does not auto-load skills from commands"
  - "Parallel invocation instructions included in Navigator Mode for Gemini to minimize query latency"

patterns-established:
  - "Navigator-as-skill: Embed agent behavior in SKILL.md when platform lacks separate agent definitions"
  - "TOML prompt pattern: Full workflow instructions in prompt field with {{args}} substitution"
  - "Separate skill copies: Each adapter gets its own copy of shared skills for portability"

# Metrics
duration: 8min
completed: 2026-02-10
---

# Phase 21 Plan 02: Gemini CLI Commands and Skills Summary

**3 TOML slash commands and 5 skills with embedded Navigator logic for tier-aware memory retrieval in Gemini CLI**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-10T15:47:54Z
- **Completed:** 2026-02-10T15:56:09Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Created 3 TOML command files (memory-search, memory-recent, memory-context) with complete Gemini CLI instructions including argument parsing, memory-daemon CLI commands, output formatting, and error handling
- Copied 4 skills from OpenCode plugin (retrieval-policy, topic-graph, bm25-search, vector-search) as separate portable copies
- Enhanced memory-query skill with full Navigator Mode including trigger patterns, intent classification, parallel invocation strategy, tier-aware layer routing, and explainability output format
- All TOML files validated with Python tomllib parser; all SKILL.md files have correct YAML frontmatter

## Task Commits

Each task was committed atomically:

1. **Task 1: Create TOML command files for Gemini** - `30cd240` (feat)
2. **Task 2: Copy skills and embed navigator logic in memory-query** - `5738458` (feat)

## Files Created/Modified

- `plugins/memory-gemini-adapter/.gemini/commands/memory-search.toml` - Search slash command with tier-aware retrieval route and TOC fallback
- `plugins/memory-gemini-adapter/.gemini/commands/memory-recent.toml` - Recent conversations command with TOC navigation
- `plugins/memory-gemini-adapter/.gemini/commands/memory-context.toml` - Grip expansion command for conversation context
- `plugins/memory-gemini-adapter/.gemini/skills/memory-query/SKILL.md` - Core query skill with embedded Navigator Mode (508 lines)
- `plugins/memory-gemini-adapter/.gemini/skills/memory-query/references/command-reference.md` - Query CLI reference
- `plugins/memory-gemini-adapter/.gemini/skills/retrieval-policy/SKILL.md` - Tier detection and intent classification skill
- `plugins/memory-gemini-adapter/.gemini/skills/retrieval-policy/references/command-reference.md` - Retrieval CLI reference
- `plugins/memory-gemini-adapter/.gemini/skills/topic-graph/SKILL.md` - Topic graph exploration skill
- `plugins/memory-gemini-adapter/.gemini/skills/topic-graph/references/command-reference.md` - Topics CLI reference
- `plugins/memory-gemini-adapter/.gemini/skills/bm25-search/SKILL.md` - BM25 keyword search skill
- `plugins/memory-gemini-adapter/.gemini/skills/bm25-search/references/command-reference.md` - BM25 CLI reference
- `plugins/memory-gemini-adapter/.gemini/skills/vector-search/SKILL.md` - Vector semantic search skill
- `plugins/memory-gemini-adapter/.gemini/skills/vector-search/references/command-reference.md` - Vector CLI reference

## Decisions Made

- **Navigator as skill:** Embedded navigator agent logic inside memory-query SKILL.md because Gemini CLI has no separate agent definition format. This gives Gemini the full retrieval intelligence when the skill activates.
- **Separate copies:** Skills are full copies from the OpenCode plugin, not symlinks. This ensures each adapter is self-contained and portable.
- **Self-contained commands:** TOML command prompts include complete workflow instructions because Gemini does not auto-load skills when a command is invoked. The prompt must be sufficient on its own.
- **Parallel invocation guidance:** Added explicit instructions for Gemini to invoke retrieval steps in parallel (e.g., status + classify simultaneously) to minimize round-trip latency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added .gemini override to .gitignore**
- **Found during:** Task 1 (TOML command commit)
- **Issue:** `.gemini` directory was gitignored by a global gitignore rule, preventing git add
- **Fix:** Added `!.gemini` and `!**/.gemini` overrides to project `.gitignore` (matching existing `.opencode` pattern)
- **Files modified:** `.gitignore`
- **Verification:** git add succeeded after override was added
- **Committed in:** 30cd240 (part of Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Auto-fix was necessary for git to track Gemini adapter files. No scope creep.

## Issues Encountered

None beyond the gitignore blocking issue documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- TOML commands and skills are ready for Gemini CLI users
- Plan 03 (hook handler, install skill, README) can proceed to complete the adapter
- All 5 skills are in place for Phase 23 cross-agent discovery

## Self-Check: PASSED

- All 13 created files verified on disk
- Commit 30cd240 (Task 1) verified in git log
- Commit 5738458 (Task 2) verified in git log

---
*Phase: 21-gemini-cli-adapter*
*Completed: 2026-02-10*
