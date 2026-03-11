---
phase: 21-gemini-cli-adapter
plan: 04
subsystem: adapter
tags: [bash, jq, ansi, redaction, gemini-cli, shell-hooks]

# Dependency graph
requires:
  - phase: 21-gemini-cli-adapter (plans 01-03)
    provides: hook handler script, install skill, README, settings.json
provides:
  - jq version detection with walk() capability fallback
  - broader ANSI stripping (CSI + OSC + other escapes via perl)
  - per-project install path rewriting instructions and documentation
affects: [22-copilot-cli-adapter]

# Tech tracking
tech-stack:
  added: []
  patterns: [runtime capability check over version string parsing, perl+sed ANSI strip chain]

key-files:
  created: []
  modified:
    - plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh
    - plugins/memory-gemini-adapter/.gemini/skills/memory-gemini-install/SKILL.md
    - plugins/memory-gemini-adapter/README.md

key-decisions:
  - "Runtime jq walk() test (jq -n 'walk(.)') instead of version string parsing for portability"
  - "del()-based fallback covers top level and one level deep (pragmatic, not recursive)"
  - "perl preferred over sed for ANSI stripping; sed fallback retained for minimal systems"

patterns-established:
  - "JQ_HAS_WALK capability flag pattern: test at startup, branch on feature availability"
  - "Perl-first ANSI strip with sed fallback for POSIX-minimal environments"

# Metrics
duration: 3min
completed: 2026-02-10
---

# Phase 21 Plan 04: Gap Closure Summary

**jq 1.5 fallback redaction via del(), perl-based ANSI stripping for CSI+OSC+SS2/SS3, and per-project path rewriting in install skill and README**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-10T18:05:03Z
- **Completed:** 2026-02-10T18:08:05Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Hook script now detects jq walk() capability at runtime and uses del()-based fallback redaction for jq < 1.6
- ANSI stripping upgraded from sed CSI-only to perl handling CSI, OSC, and other two-byte escape sequences
- Install skill SKILL.md documents per-project path rewriting with jq walk and sed fallback
- README documents jq 1.6+ recommendation, provides concrete sed command for per-project installs, and adds jq version troubleshooting

## Task Commits

Each task was committed atomically:

1. **Task 1: Harden memory-capture.sh -- jq version check and ANSI stripping** - `cc195f4` (fix)
2. **Task 2: Fix per-project install paths and update documentation** - `753bc2f` (docs)

## Files Created/Modified
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` - Added JQ_HAS_WALK detection, conditional REDACT_FILTER, perl+sed ANSI strip
- `plugins/memory-gemini-adapter/.gemini/skills/memory-gemini-install/SKILL.md` - Per-project mode, path rewriting, jq walk version note
- `plugins/memory-gemini-adapter/README.md` - jq 1.6+ prereq note, concrete sed command for per-project, troubleshooting section

## Decisions Made
- Used runtime capability check (`jq -n 'walk(.)'`) instead of version string parsing -- more reliable across distros that may backport features
- Fallback redaction uses `del()` on explicit key names at top level and one nested level -- pragmatic compromise vs full recursive walk
- Perl is preferred for ANSI stripping since it handles CSI, OSC, and SS2/SS3 in a single pass; sed fallback handles CSI-only for systems without perl

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All 3 UAT findings from Phase 21 review are now resolved
- Phase 22 (Copilot CLI Adapter) can proceed with lessons learned already incorporated
- The jq version check pattern and perl ANSI stripping approach should be reused in Phase 22

---
*Phase: 21-gemini-cli-adapter*
*Completed: 2026-02-10*
