---
phase: 33-copilot-cli-tests
verified: 2026-03-05T21:44:05Z
status: passed
score: 10/10 must-haves verified
re_verification: false
---

# Phase 33: Copilot CLI Tests Verification Report

**Phase Goal:** Developers can run isolated shell-based E2E tests for Copilot CLI that validate session ID synthesis and the hook-to-query pipeline
**Verified:** 2026-03-05T21:44:05Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `bats tests/cli/copilot/` executes all Copilot tests in isolated workspaces, reusing Phase 30 common helpers | VERIFIED | All 4 files load `'../lib/common'` and `'../lib/cli_wrappers'`; bats --count confirms 8+10+5+7=30 tests |
| 2 | Copilot binary detection uses correct binary name and `--allow-all-tools` prevents interactive prompts | VERIFIED | `run_copilot` in cli_wrappers.bash uses `copilot -p "$@" --allow-all-tools`; tests 7-8 in smoke.bats use `require_cli copilot` for graceful skip |
| 3 | Copilot session ID synthesis produces deterministic session IDs from workspace context, verified in captured events | VERIFIED | hooks.bats tests 1, 7, 8 verify session file creation at `/tmp/copilot-memory-session-{hash}`, `copilot-*` prefix, deterministic hash, and Bug #991 reuse |
| 4 | Negative tests verify daemon-down and malformed-input handling for Copilot-specific edge cases | VERIFIED | negative.bats has 7 tests: 4 for memory-ingest fail-open asserting `{"continue":true}`, 3 for memory-capture.sh asserting exit 0 with no stdout assertion (correct Copilot behavior) |
| 5 | pipeline.bats proves full ingest-to-query cycle with agent=copilot events | VERIFIED | pipeline.bats has 5 tests using direct CchEvent ingest with `"agent":"copilot"` (17 copilot agent references); helper `_ingest_full_copilot_session` ingests 5-event session |
| 6 | Negative tests assert exit 0 only for memory-capture.sh (Copilot hook produces no stdout) | VERIFIED | Tests 5-7 in negative.bats assert `[ "$status" -eq 0 ]` only, with explicit comment "We do NOT assert on output content" |
| 7 | All Copilot fixture files use Copilot-native format (ms timestamps, no hook_event_name/session_id/agent fields) | VERIFIED | grep confirms 0 files with CchEvent fields; all 5 valid fixtures have `"timestamp":1709` ms timestamps; malformed.json is intentionally broken |
| 8 | memory-capture.sh uses `jq -nc` (compact output) for all event payloads | VERIFIED | 5 occurrences of `jq -nc` in memory-capture.sh (lines 149, 158, 172, 186, 201); `jq -n` only appears in capability check at line 59 |
| 9 | Hook tests use $1 argument pattern (event type passed as argument, not in JSON body) | VERIFIED | All 10 hook invocations in hooks.bats end with `'$HOOK_SCRIPT' sessionStart/userPromptSubmitted/preToolUse/postToolUse/sessionEnd` |
| 10 | All 4 commits from summaries exist in git history | VERIFIED | Commits a302816, dab12b8, 02da769, 93ad5b4 all confirmed in git log |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `tests/cli/fixtures/copilot/session-start.json` | Copilot-native SessionStart fixture (ms timestamp) | VERIFIED | Single line, valid JSON, `{"cwd":..., "timestamp":1709640000000}` |
| `tests/cli/fixtures/copilot/user-prompt.json` | Copilot-native UserPromptSubmit fixture with `.prompt` field | VERIFIED | Contains `.prompt` field, ms timestamp, no CchEvent fields |
| `tests/cli/fixtures/copilot/pre-tool-use.json` | Copilot-native PreToolUse fixture with `.toolName` and `.toolArgs` (JSON string) | VERIFIED | Contains `.toolName:"Read"`, `.toolArgs:"{\"path\":\"/test.rs\"}"` |
| `tests/cli/fixtures/copilot/post-tool-use.json` | Copilot-native PostToolUse fixture with `.toolName` and `.toolArgs` (JSON string) | VERIFIED | Same format as pre-tool-use.json |
| `tests/cli/fixtures/copilot/session-end.json` | Copilot-native SessionEnd fixture with `.reason` field | VERIFIED | Contains `.reason:"user_exit"` |
| `tests/cli/fixtures/copilot/malformed.json` | Intentionally broken JSON for fail-open testing | VERIFIED | `{not valid json at all -- this is intentionally broken` |
| `tests/cli/copilot/smoke.bats` | CPLT-01 smoke tests (8 tests) | VERIFIED | bats --count returns 8; 125 lines; 2 `require_cli copilot` guards |
| `tests/cli/copilot/hooks.bats` | CPLT-02 hook capture tests (10 tests) | VERIFIED | bats --count returns 10; 458 lines; all 5 event types, session synthesis, Bug #991 |
| `tests/cli/copilot/pipeline.bats` | CPLT-03 E2E pipeline tests (min 80 lines) | VERIFIED | bats --count returns 5; 224 lines (exceeds minimum) |
| `tests/cli/copilot/negative.bats` | CPLT-04 negative tests (min 80 lines) | VERIFIED | bats --count returns 7; 115 lines (exceeds minimum) |
| `tests/cli/lib/cli_wrappers.bash` | Contains `run_copilot` wrapper | VERIFIED | `run_copilot()` function at line 102 with timeout guard and `--allow-all-tools` |
| `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` | Executable hook script with jq -nc fix | VERIFIED | Executable (-rwxr-xr-x), 5 `jq -nc` calls for all event types |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tests/cli/copilot/smoke.bats` | `tests/cli/lib/common.bash` | `load '../lib/common'` | WIRED | Line 7: `load '../lib/common'` |
| `tests/cli/copilot/hooks.bats` | `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` | `HOOK_SCRIPT` variable | WIRED | Line 22: `HOOK_SCRIPT="${PROJECT_ROOT}/plugins/.../memory-capture.sh"` used in 10 hook invocations |
| `tests/cli/copilot/hooks.bats` | `tests/cli/fixtures/copilot/*.json` | `FIXTURE_DIR` variable | WIRED | Line 21: `FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/copilot"` used to load pre-tool-use.json and post-tool-use.json |
| `tests/cli/copilot/pipeline.bats` | `tests/cli/lib/common.bash` | `ingest_event` helper | WIRED | `ingest_event` called 13 times in pipeline.bats; loaded via `load '../lib/common'` |
| `tests/cli/copilot/negative.bats` | `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` | `HOOK_SCRIPT` variable | WIRED | Line 32: `HOOK_SCRIPT="${PROJECT_ROOT}/plugins/.../memory-capture.sh"` used in tests 5-7 |

### Requirements Coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| CPLT-01: Smoke tests with binary detection and daemon health | SATISFIED | smoke.bats: 8 tests cover binary existence, daemon health, ingest, memory-capture.sh detection, graceful copilot CLI skip |
| CPLT-02: Hook capture with session ID synthesis | SATISFIED | hooks.bats: 10 tests cover all 5 event types via $1 pattern, session synthesis, deterministic hash, Bug #991 reuse, terminal/non-terminal cleanup |
| CPLT-03: E2E pipeline ingest-to-query cycle | SATISFIED | pipeline.bats: 5 tests prove full lifecycle, TOC browse, cwd metadata, agent field preservation, concurrent session isolation |
| CPLT-04: Negative/fail-open tests | SATISFIED | negative.bats: 7 tests cover daemon-down, malformed JSON, empty stdin, unknown event type for memory-ingest (continue:true); daemon-down, malformed, empty for memory-capture.sh (exit 0 only) |

### Anti-Patterns Found

No anti-patterns detected across all 4 test files, 6 fixture files, updated cli_wrappers.bash, or memory-capture.sh.

### Human Verification Required

#### 1. Live bats test run

**Test:** Run `bats tests/cli/copilot/` from the repository root with a running daemon
**Expected:** All 30 tests pass; tests 7-8 in smoke.bats skip gracefully if copilot CLI not installed
**Why human:** Requires a live memory-daemon binary and running gRPC service for Layer 2 gRPC verification tests

#### 2. Copilot binary detection behavior

**Test:** Install copilot CLI and run `bats tests/cli/copilot/smoke.bats`
**Expected:** Tests 7-8 execute (not skip) and test 8 either passes or skips gracefully on timeout (exit 124/137)
**Why human:** Requires the actual copilot binary installed in PATH

## Summary

Phase 33 goal is fully achieved. All 30 tests (8 smoke + 10 hooks + 5 pipeline + 7 negative) are substantive, correctly wired, and cover all 4 CPLT requirements.

Key correctness properties verified directly in code:

1. **Copilot-native fixture format** — All 6 fixture files lack `hook_event_name`, `session_id`, and `agent` fields; 5 valid fixtures use Unix millisecond timestamps.

2. **Session ID synthesis implementation** — `memory-capture.sh` synthesizes IDs via `CWD_HASH=$(printf '%s' "${CWD:-unknown}" | md5sum ...)` with session file at `/tmp/copilot-memory-session-${CWD_HASH}`. Bug #991 reuse tested in hooks.bats test 8.

3. **jq compact output fix** — `jq -nc` used in all 5 event payload constructions in memory-capture.sh (line 59's `jq -n` is only the capability probe, not payload generation).

4. **No-stdout hook behavior** — negative.bats tests 5-7 correctly assert only `[ "$status" -eq 0 ]` with explicit comments explaining Copilot hook produces no stdout (unlike Gemini's `{}`).

5. **$1 argument pattern** — All 10 hook invocations in hooks.bats pass event type as `$1` argument to `$HOOK_SCRIPT`, not embedded in JSON.

---

_Verified: 2026-03-05T21:44:05Z_
_Verifier: Claude (gsd-verifier)_
