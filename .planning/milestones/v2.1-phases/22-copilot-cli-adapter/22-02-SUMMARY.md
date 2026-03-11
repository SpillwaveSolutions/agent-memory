---
phase: 22-copilot-cli-adapter
plan: 02
subsystem: adapter
tags: [copilot, skills, agent, plugin, SKILL.md, .agent.md, tier-routing, intent-classification]

# Dependency graph
requires:
  - phase: 19-opencode-commands-skills
    provides: "SKILL.md format and content for 5 skills + navigator agent"
  - phase: 21-gemini-cli-adapter
    provides: "Adapter pattern, skill copying strategy, navigator embedding decision"
provides:
  - "5 Copilot CLI skills in .github/skills/ with SKILL.md format"
  - "Navigator agent as proper .agent.md file with infer:true"
  - "plugin.json manifest for /plugin install"
  - "Command-equivalent instructions in memory-query skill (no TOML needed)"
affects: [22-copilot-cli-adapter, 23-cross-agent-discovery]

# Tech tracking
tech-stack:
  added: []
  patterns: [".agent.md for Copilot native agents", "SKILL.md in .github/skills/ (Copilot canonical path)", "plugin.json manifest for Copilot /plugin install"]

key-files:
  created:
    - "plugins/memory-copilot-adapter/.github/skills/memory-query/SKILL.md"
    - "plugins/memory-copilot-adapter/.github/skills/memory-query/references/command-reference.md"
    - "plugins/memory-copilot-adapter/.github/skills/retrieval-policy/SKILL.md"
    - "plugins/memory-copilot-adapter/.github/skills/retrieval-policy/references/command-reference.md"
    - "plugins/memory-copilot-adapter/.github/skills/topic-graph/SKILL.md"
    - "plugins/memory-copilot-adapter/.github/skills/topic-graph/references/command-reference.md"
    - "plugins/memory-copilot-adapter/.github/skills/bm25-search/SKILL.md"
    - "plugins/memory-copilot-adapter/.github/skills/bm25-search/references/command-reference.md"
    - "plugins/memory-copilot-adapter/.github/skills/vector-search/SKILL.md"
    - "plugins/memory-copilot-adapter/.github/skills/vector-search/references/command-reference.md"
    - "plugins/memory-copilot-adapter/.github/agents/memory-navigator.agent.md"
    - "plugins/memory-copilot-adapter/plugin.json"
  modified: []

key-decisions:
  - "Skills use .github/skills/ path (Copilot canonical, not .claude/skills/)"
  - "Navigator agent is a separate .agent.md file with infer:true (unlike Gemini where it was embedded in skill)"
  - "No TOML command files created (Copilot uses skills, not TOML commands)"
  - "Command-equivalent instructions embedded in memory-query skill for search/recent/context operations"
  - "Skills are separate copies from OpenCode plugin (not symlinks) for portability"
  - "plugin.json uses minimal fields (name, version, description, author, repository)"
  - "Agent uses tools: execute, read, search (Copilot tool names)"

patterns-established:
  - "Copilot adapter skills in .github/skills/ with identical SKILL.md format to Claude Code"
  - "Copilot agents in .github/agents/ with .agent.md extension and YAML frontmatter"
  - "Plugin manifest at adapter root for /plugin install support"

# Metrics
duration: 8min
completed: 2026-02-10
---

# Phase 22 Plan 02: Copilot CLI Skills, Navigator Agent, and Plugin Manifest Summary

**5 SKILL.md skills in .github/skills/, navigator .agent.md with tier-aware routing and infer:true, plugin.json manifest for /plugin install**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-10T18:20:44Z
- **Completed:** 2026-02-10T18:28:27Z
- **Tasks:** 2
- **Files created:** 12

## Accomplishments

- Created 5 skill directories with SKILL.md + references/command-reference.md files in .github/skills/
- Enhanced memory-query skill with command-equivalent instructions for search, recent, and context operations (474 lines)
- Created navigator agent as proper .agent.md file with full tier-aware routing, intent classification, fallback chains, and explainability (249 lines)
- Created plugin.json manifest enabling /plugin install from local path or GitHub repo URL

## Task Commits

Each task was committed atomically:

1. **Task 1: Create skills with SKILL.md format for Copilot** - `cfa317b` (feat)
2. **Task 2: Create navigator agent and plugin manifest** - `77a683d` (feat)

## Files Created/Modified

- `plugins/memory-copilot-adapter/.github/skills/memory-query/SKILL.md` - Core query skill with command-equivalent instructions for search, recent, context (474 lines)
- `plugins/memory-copilot-adapter/.github/skills/memory-query/references/command-reference.md` - Full CLI reference for query commands
- `plugins/memory-copilot-adapter/.github/skills/retrieval-policy/SKILL.md` - Tier detection and intent classification skill (271 lines)
- `plugins/memory-copilot-adapter/.github/skills/retrieval-policy/references/command-reference.md` - Retrieval policy CLI reference
- `plugins/memory-copilot-adapter/.github/skills/topic-graph/SKILL.md` - Topic graph exploration skill (268 lines)
- `plugins/memory-copilot-adapter/.github/skills/topic-graph/references/command-reference.md` - Topic graph CLI reference
- `plugins/memory-copilot-adapter/.github/skills/bm25-search/SKILL.md` - BM25 keyword search skill (235 lines)
- `plugins/memory-copilot-adapter/.github/skills/bm25-search/references/command-reference.md` - BM25 search CLI reference
- `plugins/memory-copilot-adapter/.github/skills/vector-search/SKILL.md` - Vector semantic search skill (253 lines)
- `plugins/memory-copilot-adapter/.github/skills/vector-search/references/command-reference.md` - Vector search CLI reference
- `plugins/memory-copilot-adapter/.github/agents/memory-navigator.agent.md` - Navigator agent with tier routing, intent classification, fallback chains (249 lines)
- `plugins/memory-copilot-adapter/plugin.json` - Plugin manifest for /plugin install

## Decisions Made

1. **Skills use .github/skills/ path** -- Copilot's canonical location, not .claude/skills/ (which it reads for backward compatibility). Chose canonical path for clarity.
2. **Navigator agent is a separate .agent.md file** -- Unlike Gemini adapter where navigator logic was embedded in memory-query SKILL.md (Gemini has no agent definition format), Copilot CLI natively supports .agent.md files. This allows proper agent-skill separation.
3. **No TOML command files** -- Copilot CLI does not use TOML commands. Skills auto-activate based on description matching the user's prompt. Command-equivalent instructions were embedded directly in the memory-query skill.
4. **infer:true for auto-invocation** -- Copilot's infer feature allows the navigator agent to be automatically selected when queries match its description, providing seamless user experience without explicit `/agent memory-navigator` invocation.
5. **tools: execute, read, search** -- Used Copilot CLI tool names (not Claude Code/OpenCode names) for the agent's tool list.
6. **Skills are separate copies** -- Not symlinks, matching the Gemini adapter pattern for portability. Each adapter is self-contained.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Skills and agent are ready for Plan 03 (hooks, install skill, README)
- Plugin manifest is in place for hook configuration and README documentation
- All 5 skills + navigator agent provide complete query functionality parity with Claude Code and OpenCode adapters

## Self-Check: PASSED

- All 12 created files verified present on disk
- Both task commits (cfa317b, 77a683d) verified in git log
