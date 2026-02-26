---
phase: 32-opencode-cli-tests
verified: 2026-02-26T08:30:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
---

# Phase 32: OpenCode CLI Tests Verification Report

**Phase Goal:** Developers can run isolated shell-based E2E tests for OpenCode CLI, handling its less mature headless mode with appropriate skip/warn patterns
**Verified:** 2026-02-26T08:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `bats tests/cli/opencode/` executes all OpenCode tests in isolated workspaces, reusing Phase 30 common helpers | VERIFIED | 4 bats files exist (smoke.bats, hooks.bats, pipeline.bats, negative.bats); all load `'../lib/common'` and `'../lib/cli_wrappers'`; total 25 tests confirmed by `bats --count` (8+7+5+5=25) |
| 2 | OpenCode invocation uses `opencode run --format json` and timeout guards prevent hangs | VERIFIED | `run_opencode()` in `tests/cli/lib/cli_wrappers.bash` uses `("opencode" "run" "--format" "json" "$@")` wrapped in `${TIMEOUT_CMD} ${CLI_TIMEOUT}s`; smoke.bats test 8 and negative.bats test 5 both guard with exit 124/137 skip or require_cli skip |
| 3 | OpenCode hook capture produces events with agent field "opencode" queryable via gRPC pipeline test | VERIFIED | All 5 fixture JSON files contain `"agent":"opencode"`; hooks.bats uses direct ingest_event with two-layer proof (exit 0 + continue:true, then gRPC query); pipeline.bats uses `"agent":"opencode"` in all 13 ingest calls |
| 4 | Negative tests cover daemon-down and timeout scenarios specific to OpenCode's headless behavior | VERIFIED | negative.bats has 5 tests: daemon-down (test 1), malformed JSON (test 2), empty stdin (test 3), unknown event type (test 4), OpenCode headless timeout with skip-friendly exit 0/124/137 (test 5) |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `tests/cli/fixtures/opencode/session-start.json` | SessionStart CchEvent fixture with agent=opencode | VERIFIED | Single-line compact JSON, `"agent":"opencode"` present |
| `tests/cli/fixtures/opencode/user-prompt.json` | UserPromptSubmit CchEvent fixture | VERIFIED | Single-line compact JSON, `"agent":"opencode"` present |
| `tests/cli/fixtures/opencode/assistant-response.json` | AssistantResponse CchEvent fixture | VERIFIED | Single-line compact JSON, `"agent":"opencode"` present |
| `tests/cli/fixtures/opencode/post-tool-use.json` | PostToolUse CchEvent fixture | VERIFIED | Single-line compact JSON, `"agent":"opencode"` present |
| `tests/cli/fixtures/opencode/stop.json` | Stop CchEvent fixture | VERIFIED | Single-line compact JSON, `"agent":"opencode"` present |
| `tests/cli/fixtures/opencode/malformed.json` | Intentionally broken JSON for negative tests | VERIFIED | Content: `{not valid json at all -- this is intentionally broken` |
| `tests/cli/opencode/smoke.bats` | 8 smoke tests (binary check, daemon health, ingest validation, opencode CLI skip) | VERIFIED | `bats --count` returns 8; require_cli opencode used in 2 tests; hard assertions throughout |
| `tests/cli/opencode/hooks.bats` | 7 hook capture tests (all 5 event types + sequence + agent field) | VERIFIED | `bats --count` returns 7; 20 ingest_event calls; FIXTURE_DIR wired to fixtures/opencode; no HOOK_SCRIPT; no PreToolUse in code |
| `tests/cli/opencode/pipeline.bats` | 5 E2E pipeline tests (session lifecycle, TOC browse, cwd metadata, agent field, concurrent sessions) | VERIFIED | `bats --count` returns 5; 13 `"agent":"opencode"` references; `5 found` and `6 found` assertions present; no PreToolUse in code |
| `tests/cli/opencode/negative.bats` | 5-6 negative tests (daemon down, malformed, empty stdin, unknown event, timeout scenario) | VERIFIED | `bats --count` returns 5; 7 `{"continue":true}` assertions; 4 memory-ingest fail-open tests + 1 timeout test; no memory-capture.sh references |
| `tests/cli/lib/cli_wrappers.bash` | run_opencode wrapper with timeout guard | VERIFIED | `run_opencode()` function present; uses `opencode run --format json`; TIMEOUT_CMD guard present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tests/cli/opencode/smoke.bats` | `tests/cli/lib/common.bash` | `load '../lib/common'` | WIRED | Pattern confirmed in all 4 bats files |
| `tests/cli/opencode/hooks.bats` | `tests/cli/fixtures/opencode/*.json` | `FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/opencode"` | WIRED | All 5 event type fixtures accessed via rewrite_session_id |
| `tests/cli/opencode/hooks.bats` | `memory-ingest binary` | `ingest_event` helper from common.bash | WIRED | 20 ingest_event calls in hooks.bats |
| `tests/cli/opencode/pipeline.bats` | `memory-ingest + memory-daemon` | `ingest_event` + `grpc_query` from common.bash | WIRED | 13 agent=opencode ingest calls; grpc_query in all 5 tests |
| `tests/cli/opencode/negative.bats` | `memory-ingest binary` | direct pipe to `MEMORY_INGEST_BIN` | WIRED | 4 MEMORY_INGEST_BIN references in fail-open tests |

### Requirements Coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| OPEN-01: `bats tests/cli/opencode/` runs all OpenCode tests in isolated workspaces reusing Phase 30 helpers | SATISFIED | 25 tests across 4 files, all using setup_workspace/teardown_workspace and common.bash helpers |
| OPEN-02: opencode run --format json + timeout guards | SATISFIED | run_opencode wrapper uses correct syntax; skip/warn patterns for exit 124/137 |
| OPEN-03: Hook capture with agent="opencode" queryable via gRPC | SATISFIED | pipeline.bats proves complete ingest-to-query cycle; hooks.bats two-layer proof pattern |
| OPEN-04: Negative tests for daemon-down and timeout scenarios | SATISFIED | negative.bats covers all 4 fail-open modes + timeout |

### Anti-Patterns Found

No TODO/FIXME/HACK/placeholder patterns found in opencode test files or fixtures.

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `hooks.bats`, `pipeline.bats` | "PreToolUse" appears in comments only (`# OpenCode has 5 event types (no PreToolUse)`) | Info | Not a stub — comment is documenting intentional design choice, no test code uses PreToolUse |

### Human Verification Required

#### 1. Live Test Suite Execution

**Test:** Run `bats tests/cli/opencode/` with a running daemon and verify all 25 tests pass without daemon tests timing out.
**Expected:** All 25 tests pass (tests 7-8 in smoke.bats and test 5 in negative.bats skip gracefully if opencode binary is not installed).
**Why human:** Requires a running memory-daemon and built binaries; CI environment needed for full confirmation.

#### 2. OpenCode Binary Installed Paths

**Test:** If opencode binary is installed, run smoke.bats test 8 ("opencode headless mode produces output") and negative.bats test 5 ("opencode headless timeout produces skip-friendly exit").
**Expected:** Test 8 either succeeds (exit 0) or skips with "OpenCode headless mode timed out (known quirk)"; test 5 exits 0, 124, or 137.
**Why human:** Depends on whether the opencode binary is available and its actual headless behavior.

### Gaps Summary

No gaps found. All 4 observable truths are verified, all 11 artifacts exist and are substantive, all 5 key links are wired, and no anti-patterns block goal achievement.

The phase achieved its goal: developers can run `bats tests/cli/opencode/` to execute 25 isolated shell-based E2E tests for OpenCode CLI. The tests correctly handle OpenCode's headless mode quirks via skip patterns for exit 124/137, use `opencode run --format json` as required, prove agent field "opencode" through the ingest-to-gRPC pipeline, and cover daemon-down and timeout negative scenarios.

---

_Verified: 2026-02-26T08:30:00Z_
_Verifier: Claude (gsd-verifier)_
