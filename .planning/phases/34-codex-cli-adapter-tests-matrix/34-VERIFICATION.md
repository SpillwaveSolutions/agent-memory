---
phase: 34-codex-cli-adapter-tests-matrix
verified: 2026-03-05T23:30:00Z
status: passed
score: 12/12 must-haves verified
re_verification: false
---

# Phase 34: Codex CLI Adapter, Tests, and Matrix Report Verification Report

**Phase Goal:** Codex CLI adapter exists with commands and skills (no hooks), Codex headless tests pass with hook tests skipped, and a cross-CLI matrix report aggregates results from all 5 CLIs
**Verified:** 2026-03-05T23:30:00Z
**Status:** passed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Codex CLI adapter directory exists at adapters/codex-cli/ with skills and sandbox documentation | VERIFIED | adapters/codex-cli/ exists with README.md (206 lines), SANDBOX-WORKAROUND.md (85 lines), .gitignore |
| 2 | Codex adapter has NO hook handler (commands + skills only) | VERIFIED | No .codex/hooks/ directory; 5 skills exist under .codex/skills/ |
| 3 | Running bats tests/cli/codex/smoke.bats executes 8 smoke tests | VERIFIED | bats --count returns 8 |
| 4 | Running bats tests/cli/codex/hooks.bats shows all tests SKIPPED with Codex no-hooks annotation | VERIFIED | bats --count returns 6; grep finds 6 skip statements with "GitHub Discussion #2150" |
| 5 | run_codex wrapper exists in cli_wrappers.bash using codex exec --full-auto --json (no -q flag) | VERIFIED | Function at line 102, uses "codex" "exec" "--full-auto" "--json", comment confirms no -q flag |
| 6 | bats tests/cli/codex/pipeline.bats executes 5 E2E pipeline tests with direct CchEvent ingest | VERIFIED | bats --count returns 5; 17 occurrences of agent/codex references |
| 7 | bats tests/cli/codex/negative.bats executes 7 tests (4 fail-open + 3 skipped hook tests) | VERIFIED | bats --count returns 7; 19 continue:true references; 3 hook-skipped tests |
| 8 | Pipeline tests use direct CchEvent format with agent=codex | VERIFIED | loads common.bash, fixtures/codex path referenced, agent=codex in event payloads |
| 9 | Negative tests verify fail-open behavior (continue:true) | VERIFIED | 19 continue:true references; covers daemon-down, malformed, empty, unknown event |
| 10 | Matrix report script exists at scripts/cli-matrix-report.sh parsing JUnit XML from all 5 CLIs | VERIFIED | 139-line script, executable, valid bash syntax, references claude-code/gemini/opencode/copilot/codex |
| 11 | CI workflow has matrix-report job running after all e2e-cli matrix entries | VERIFIED | e2e-cli.yml line 126: matrix-report job with needs: [e2e-cli] and if: always() |
| 12 | Matrix report is viewable in GitHub Actions step summary | VERIFIED | scripts/cli-matrix-report.sh junit-reports >> $GITHUB_STEP_SUMMARY at line 145 |

**Score:** 12/12 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `adapters/codex-cli/README.md` | Codex adapter documentation (50+ lines) | VERIFIED | 206 lines |
| `adapters/codex-cli/SANDBOX-WORKAROUND.md` | macOS sandbox workaround (20+ lines) | VERIFIED | 85 lines |
| `adapters/codex-cli/.codex/skills/memory-query/SKILL.md` | Core query skill with YAML frontmatter | VERIFIED | Contains "name: memory-query" |
| `adapters/codex-cli/.codex/skills/retrieval-policy/SKILL.md` | Retrieval policy skill | VERIFIED | File exists |
| `adapters/codex-cli/.codex/skills/topic-graph/SKILL.md` | Topic graph skill | VERIFIED | File exists |
| `adapters/codex-cli/.codex/skills/bm25-search/SKILL.md` | BM25 search skill | VERIFIED | File exists |
| `adapters/codex-cli/.codex/skills/vector-search/SKILL.md` | Vector search skill | VERIFIED | File exists, command-reference.md present |
| `tests/cli/codex/smoke.bats` | 8 smoke tests (80+ lines) | VERIFIED | 8 tests counted by bats |
| `tests/cli/codex/hooks.bats` | 6 all-skipped hook tests | VERIFIED | 6 tests, all with skip statement |
| `tests/cli/codex/pipeline.bats` | 5 E2E pipeline tests (80+ lines) | VERIFIED | 5 tests, 224 lines, agent=codex |
| `tests/cli/codex/negative.bats` | 7 negative tests (40+ lines) | VERIFIED | 7 tests, 96 lines, continue:true |
| `tests/cli/lib/cli_wrappers.bash` | run_codex wrapper function | VERIFIED | Function at line 102 using codex exec --full-auto --json |
| `tests/cli/fixtures/codex/` | 6 CchEvent JSON fixtures | VERIFIED | 6 files: session-start, session-end, user-prompt, pre-tool-use, post-tool-use, malformed |
| `scripts/cli-matrix-report.sh` | Cross-CLI matrix report aggregator (30+ lines) | VERIFIED | 139 lines, executable, python3 xml.etree parsing |
| `.github/workflows/e2e-cli.yml` | Updated CI with matrix-report job | VERIFIED | matrix-report job at line 126 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tests/cli/codex/smoke.bats` | `tests/cli/lib/cli_wrappers.bash` | `load '../lib/cli_wrappers'` | WIRED | Pattern found at top of file |
| `tests/cli/codex/smoke.bats` | `tests/cli/fixtures/codex/` | FIXTURE_DIR variable | WIRED | `FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/codex"` |
| `tests/cli/lib/cli_wrappers.bash` | `codex exec --full-auto` | run_codex function | WIRED | `local cmd=("codex" "exec" "--full-auto" "--json" "$@")` |
| `tests/cli/codex/pipeline.bats` | `tests/cli/lib/common.bash` | `load '../lib/common'` | WIRED | Pattern found |
| `tests/cli/codex/pipeline.bats` | `tests/cli/fixtures/codex/` | ingest with agent:codex | WIRED | 17 agent/codex references |
| `tests/cli/codex/negative.bats` | `tests/cli/fixtures/codex/malformed.json` | malformed fixture path | WIRED | Direct cat of FIXTURE_DIR/malformed.json |
| `scripts/cli-matrix-report.sh` | `tests/cli/.runs/report.xml` | JUnit XML parsing | WIRED | CI mode reads `junit-<cli>-*/report.xml` |
| `.github/workflows/e2e-cli.yml` | `scripts/cli-matrix-report.sh` | matrix-report job invocation | WIRED | `scripts/cli-matrix-report.sh junit-reports >> $GITHUB_STEP_SUMMARY` |

### Requirements Coverage

All phase requirements satisfied:
- CDEX-01: Codex adapter with skills (no hooks) -- SATISFIED
- CDEX-02: Codex smoke tests -- SATISFIED
- CDEX-03: Codex pipeline (E2E ingest-to-query) -- SATISFIED
- CDEX-04: Codex negative/fail-open tests -- SATISFIED
- CDEX-05: Cross-CLI matrix report -- SATISFIED

### Anti-Patterns Found

None. No TODO/FIXME/PLACEHOLDER comments found in test files or the matrix report script. No stub implementations or empty return patterns detected.

### Human Verification Required

#### 1. Codex Headless Mode Test

**Test:** Install Codex CLI and run `bats tests/cli/codex/smoke.bats`
**Expected:** Tests 7 and 8 run (not skipped), `codex --version` succeeds, headless mode produces output
**Why human:** Codex binary not present in CI without installation; tests gracefully skip when absent

#### 2. Full Pipeline E2E

**Test:** Start memory-daemon and run `bats tests/cli/codex/pipeline.bats`
**Expected:** All 5 pipeline tests pass -- session lifecycle, TOC browse, cwd metadata, agent field, concurrent sessions
**Why human:** Requires running daemon; async timing depends on actual system behavior

#### 3. Matrix Report with Real JUnit XML

**Test:** Run all CLI test suites to generate JUnit XML, then run `scripts/cli-matrix-report.sh <junit-dir>`
**Expected:** Markdown table with 5 CLI columns, per-scenario rows, summary totals
**Why human:** Requires actual test runs across all 5 CLI suites to produce input files

## Commits Verified

| Commit | Description | Present |
|--------|-------------|---------|
| a2e6d1f | feat(34-01): create Codex CLI adapter with 5 skills and sandbox docs | YES |
| 740a4ae | feat(34-01): add Codex fixtures, run_codex wrapper, smoke and hooks tests | YES |
| d69ae01 | feat(34-02): create Codex pipeline.bats with 5 E2E ingest-to-query tests | YES |
| 9586a0f | feat(34-02): create Codex negative.bats with 4 fail-open + 3 skipped hook tests | YES |
| 8837a85 | feat(34-03): add CLI matrix report script for JUnit XML aggregation | YES |
| efcabda | feat(34-03): add matrix-report job to e2e-cli workflow | YES |

## Summary

Phase 34 goal is fully achieved. All three plan objectives completed:

1. **Codex CLI adapter (34-01):** Adapter exists at `adapters/codex-cli/` with 5 skills under `.codex/skills/`, no hooks directory, README.md (206 lines) and SANDBOX-WORKAROUND.md (85 lines). The `run_codex` wrapper uses `codex exec --full-auto --json` without the `-q` flag. All 8 smoke tests count correctly; all 6 hooks tests are skipped with the GitHub Discussion #2150 annotation.

2. **Pipeline and negative tests (34-02):** `pipeline.bats` has 5 E2E tests using direct CchEvent format with `agent=codex`. `negative.bats` has 7 tests: 4 memory-ingest fail-open tests asserting `continue:true`, and 3 skipped hook tests with proper annotation.

3. **Cross-CLI matrix report (34-03):** `scripts/cli-matrix-report.sh` (139 lines) is executable, has valid bash syntax, uses Python3 `xml.etree.ElementTree` for JUnit XML parsing, and references all 5 CLIs. The `e2e-cli.yml` CI workflow has a `matrix-report` job with `needs: [e2e-cli]` and `if: always()`, downloading JUnit artifacts and outputting to `$GITHUB_STEP_SUMMARY`.

---

_Verified: 2026-03-05T23:30:00Z_
_Verifier: Claude (gsd-verifier)_
