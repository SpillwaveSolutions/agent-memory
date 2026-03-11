---
phase: 34-codex-cli-adapter-tests-matrix
plan: 01
one_liner: "Codex CLI adapter with 5 skills (no hooks), 6 CchEvent fixtures, run_codex wrapper, 8 smoke tests, 6 all-skipped hooks tests"
subsystem: cli-testing
tags: [codex, adapter, skills, bats, fixtures, smoke-tests]
dependency_graph:
  requires: [phase-30-cli-harness, phase-31-claude-code-tests]
  provides: [codex-adapter, codex-fixtures, codex-smoke-tests, run_codex-wrapper]
  affects: [cli_wrappers.bash]
tech_stack:
  added: [codex-cli-skills]
  patterns: [skills-only-adapter, no-hooks-pattern, CchEvent-direct-ingest]
key_files:
  created:
    - adapters/codex-cli/README.md
    - adapters/codex-cli/SANDBOX-WORKAROUND.md
    - adapters/codex-cli/.gitignore
    - adapters/codex-cli/.codex/skills/memory-query/SKILL.md
    - adapters/codex-cli/.codex/skills/memory-query/references/command-reference.md
    - adapters/codex-cli/.codex/skills/retrieval-policy/SKILL.md
    - adapters/codex-cli/.codex/skills/retrieval-policy/references/command-reference.md
    - adapters/codex-cli/.codex/skills/topic-graph/SKILL.md
    - adapters/codex-cli/.codex/skills/topic-graph/references/command-reference.md
    - adapters/codex-cli/.codex/skills/bm25-search/SKILL.md
    - adapters/codex-cli/.codex/skills/bm25-search/references/command-reference.md
    - adapters/codex-cli/.codex/skills/vector-search/SKILL.md
    - adapters/codex-cli/.codex/skills/vector-search/references/command-reference.md
    - tests/cli/fixtures/codex/session-start.json
    - tests/cli/fixtures/codex/session-end.json
    - tests/cli/fixtures/codex/user-prompt.json
    - tests/cli/fixtures/codex/pre-tool-use.json
    - tests/cli/fixtures/codex/post-tool-use.json
    - tests/cli/fixtures/codex/malformed.json
    - tests/cli/codex/smoke.bats
    - tests/cli/codex/hooks.bats
  modified:
    - tests/cli/lib/cli_wrappers.bash
decisions:
  - "Codex adapter placed in adapters/ (not plugins/) because it has no hooks"
  - "Skills use YAML frontmatter with name + description (Codex SKILL.md format)"
  - "run_codex uses codex exec --full-auto --json (no -q flag -- does not exist in Codex)"
  - "Smoke test 6 verifies adapter skills instead of hook script (Codex has no hooks)"
  - "Global gitignore blocks .codex/ -- used git add -f to override"
metrics:
  duration: "7min"
  completed: "2026-03-05"
  tasks: 2
  files_created: 22
  files_modified: 1
---

# Phase 34 Plan 01: Codex CLI Adapter, Fixtures, and Smoke Tests Summary

Codex CLI adapter with 5 skills (no hooks), 6 CchEvent fixtures, run_codex wrapper, 8 smoke tests, 6 all-skipped hooks tests.

## What Was Done

### Task 1: Codex Adapter Directory with Skills and Documentation

Created the Codex CLI adapter at `adapters/codex-cli/` with:

- **5 skills** under `.codex/skills/` -- memory-query, retrieval-policy, topic-graph, bm25-search, vector-search
- Each skill has YAML frontmatter with `name` and `description` fields (Codex format) and a `references/command-reference.md`
- **SANDBOX-WORKAROUND.md** documenting the macOS Seatbelt sandbox issue (GitHub Issue #5041) with workarounds for both Linux (Landlock config) and macOS (danger-full-access mode)
- **README.md** explaining the no-hooks limitation (Discussion #2150), installation via .codex/skills/ copy, cross-agent query examples, and manual CchEvent ingestion
- **.gitignore** for logs, macOS artifacts, and editor files
- **No hooks directory** -- Codex CLI does not support lifecycle hooks

### Task 2: Fixtures, Wrapper, Smoke Tests, and Hooks Tests

- **6 fixture JSONs** in `tests/cli/fixtures/codex/` in CchEvent format with `agent:"codex"`: session-start, session-end, user-prompt, pre-tool-use, post-tool-use, malformed
- **run_codex wrapper** appended to `cli_wrappers.bash` using `codex exec --full-auto --json` (no `-q` flag per research findings)
- **smoke.bats** with 8 tests:
  - Tests 1-3: Binary and daemon checks (always run)
  - Test 4: Valid CchEvent ingest produces continue:true
  - Test 5: Malformed JSON ingest produces continue:true (fail-open)
  - Test 6: Adapter skills exist with valid YAML frontmatter
  - Tests 7-8: Codex binary detection and headless mode (skip if not installed)
- **hooks.bats** with 6 all-skipped tests annotating "Codex CLI does not support hooks (GitHub Discussion #2150)"

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | a2e6d1f | Create Codex CLI adapter with 5 skills and sandbox docs |
| 2 | 740a4ae | Add Codex fixtures, run_codex wrapper, smoke and hooks tests |

## Verification Results

| Check | Expected | Actual | Status |
|-------|----------|--------|--------|
| SKILL.md count | 5 | 5 | PASS |
| No hooks directory | true | true | PASS |
| Fixture JSON count | 6 | 6 | PASS |
| run_codex wrapper exists | true | true | PASS |
| No -q flag in run_codex | true | true | PASS |
| smoke.bats test count | 8 | 8 | PASS |
| hooks.bats test count | 6 | 6 | PASS |
| All hooks tests skipped | 6 | 6 | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Global gitignore blocks .codex/ directory**
- **Found during:** Task 1 commit
- **Issue:** User's global `~/.gitignore_global` contains `.codex/` rule, preventing git add of skill files
- **Fix:** Used `git add -f` to force-add the files, overriding the global gitignore
- **Files affected:** All files under `adapters/codex-cli/.codex/`
- **Commit:** a2e6d1f

## Self-Check: PASSED

All key files verified present. Both task commits (a2e6d1f, 740a4ae) confirmed in git log.
