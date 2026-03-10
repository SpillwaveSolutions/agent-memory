# Phase 33: Copilot CLI Tests - Research

**Researched:** 2026-03-05
**Domain:** Shell-based E2E testing for GitHub Copilot CLI adapter
**Confidence:** HIGH

## Summary

Phase 33 adds isolated BATS-based E2E tests for the Copilot CLI adapter, following the established 2-plan pattern from Phases 31 (Gemini) and 32 (OpenCode). The Copilot adapter has a unique characteristic that distinguishes it from all other adapters: it does NOT receive session IDs from the CLI. Instead, the hook script (`memory-capture.sh`) synthesizes deterministic session IDs by hashing the CWD via md5 and storing them in `/tmp/copilot-memory-session-{hash}` files. This session ID synthesis is the primary testing differentiator for Phase 33.

The Copilot hook script receives the event type as a `$1` argument (not inside the JSON payload), reads Copilot-native JSON from stdin (with Unix millisecond timestamps, not ISO 8601), and transforms it into CchEvent format before piping to `memory-ingest`. The hook runs `memory-ingest` in the background with stdout/stderr redirected to `/dev/null` (fail-open pattern). This means Copilot hook tests must invoke `memory-capture.sh` with the event type as a CLI argument, which differs from Gemini (where hook_event_name is in the JSON).

The existing infrastructure from Phase 30 (common.bash, cli_wrappers.bash, workspace isolation, daemon lifecycle) is fully reusable. Copilot-specific fixtures need to use Copilot-native JSON format (Unix ms timestamps, `.prompt` instead of `.message`, `.toolName`/`.toolArgs` instead of `tool_name`/`tool_input`). The 2-plan pattern is: Plan 01 (fixtures + smoke + hooks), Plan 02 (pipeline + negative).

**Primary recommendation:** Follow the Phase 31/32 pattern exactly, but add Copilot-specific tests for session ID synthesis (temp file creation, deterministic hashing, Bug #991 reuse behavior, cleanup on terminal session end).

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bats-core | 1.12 | Test framework | Established in Phase 30 |
| common.bash | - | Workspace isolation, daemon lifecycle | Phase 30 shared library |
| cli_wrappers.bash | - | CLI detection, hook/ingest helpers | Phase 30 shared library |
| jq | 1.6+ | JSON fixture manipulation | Used by rewrite_session_id helper |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| memory-capture.sh | - | Copilot hook script under test | Hook capture tests (CPLT-02) |
| memory-ingest | - | Binary that receives CchEvent JSON | All ingest tests |
| memory-daemon | - | gRPC daemon for query verification | Pipeline tests (CPLT-03) |
| grpcurl | - | Optional gRPC health check | Daemon health verification |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Direct CchEvent ingest | Hook script invocation | Copilot DOES have a testable shell hook script (unlike OpenCode's TypeScript plugin), so BOTH patterns apply |

## Architecture Patterns

### Recommended Project Structure
```
tests/cli/
  copilot/
    smoke.bats           # CPLT-01: Binary detection, basic ingest, daemon connectivity
    hooks.bats           # CPLT-02: Hook script capture, session ID synthesis verification
    pipeline.bats        # CPLT-03: Full ingest -> query cycle
    negative.bats        # CPLT-04: Daemon down, malformed input, fail-open
  fixtures/
    copilot/
      session-start.json       # Copilot-native format (ms timestamps, no session_id)
      session-end.json         # With .reason field for cleanup logic
      user-prompt.json         # Uses .prompt field (not .message)
      pre-tool-use.json        # Uses .toolName, .toolArgs (JSON string)
      post-tool-use.json       # Uses .toolName, .toolArgs (JSON string)
      malformed.json           # Invalid JSON for fail-open testing
  lib/
    common.bash          # Reuse from Phase 30
    cli_wrappers.bash    # Reuse from Phase 30 (add run_copilot wrapper)
```

### Pattern 1: Copilot Hook Script Invocation
**What:** Invoke memory-capture.sh with event type as $1, JSON on stdin
**When to use:** CPLT-02 hook capture tests
**Example:**
```bash
# Source: plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh
# Copilot hook receives event type as argument, not in JSON body
run bash -c "echo '$json' | \
  MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
  MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
  '$HOOK_SCRIPT' sessionStart"
```

### Pattern 2: Copilot-Native Fixture Format
**What:** Fixtures use Copilot's native JSON format (different from CchEvent format)
**When to use:** All hook capture tests that go through memory-capture.sh
**Example:**
```json
{
  "cwd": "/tmp/test-workspace",
  "timestamp": 1709640000000,
  "prompt": "Explain the project structure",
  "toolName": "Read",
  "toolArgs": "{\"path\":\"/test.rs\"}"
}
```
Note: No `hook_event_name`, no `session_id`, no `agent` field -- these are all added by the hook script.

### Pattern 3: Session ID Synthesis Verification
**What:** Verify session ID is deterministically derived from CWD hash
**When to use:** CPLT-02 session synthesis tests
**Example:**
```bash
# Hash the CWD the same way the hook script does
local expected_hash
expected_hash=$(printf '%s' "/tmp/test-workspace" | md5sum 2>/dev/null | cut -d' ' -f1 || \
                printf '%s' "/tmp/test-workspace" | md5 2>/dev/null)
local session_file="/tmp/copilot-memory-session-${expected_hash}"

# After invoking hook with sessionStart, verify session file was created
[ -f "$session_file" ]

# Verify session ID starts with "copilot-"
local sid
sid=$(cat "$session_file")
[[ "$sid" == copilot-* ]]
```

### Pattern 4: Direct CchEvent Ingest (for pipeline tests)
**What:** Bypass hook script, ingest CchEvent JSON directly via ingest_event helper
**When to use:** CPLT-03 pipeline tests (same as OpenCode Phase 32 pattern)
**Example:**
```bash
# Direct CchEvent ingest with agent=copilot
ingest_event '{"hook_event_name":"SessionStart","session_id":"copilot-pipeline-001","agent":"copilot","cwd":"/tmp/test","timestamp":"2026-03-05T10:00:00Z"}'
```

### Pattern 5: Two-Layer Proof (from Phase 31)
**What:** Layer 1 = hook/ingest exits 0 with correct output; Layer 2 = gRPC query verifies storage
**When to use:** All hook and pipeline tests
**Note:** sleep 2 between layers for background ingest timing (established in Phase 31)

### Anti-Patterns to Avoid
- **Using CchEvent format in hook tests:** Copilot hook expects native Copilot JSON (ms timestamps, .prompt, .toolName). Using CchEvent format will silently produce wrong results.
- **Expecting stdout from hook script:** Copilot hook produces NO stdout (unlike Gemini which outputs `{}`). The hook runs memory-ingest in background with all output redirected to /dev/null.
- **Forgetting session file cleanup:** Tests that create session files in /tmp must clean them up in teardown to avoid cross-test contamination.
- **Testing without jq:** The hook script requires jq and silently exits 0 if jq is missing. Tests must `require_cli jq` or skip.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Workspace isolation | Custom temp dirs | `setup_workspace` / `teardown_workspace` from common.bash | Handles cleanup on failure, preserves for debugging |
| Daemon lifecycle | Manual start/stop | `start_daemon` / `stop_daemon` from common.bash | Handles port assignment, health checks, timeout |
| CLI detection | Manual `which` checks | `require_cli` / `has_cli` from cli_wrappers.bash | Graceful skip with informative messages |
| Session ID hashing | Custom md5 logic | Replicate exact logic from memory-capture.sh | Must match for verification |
| JSON fixture rewriting | sed hacks | `rewrite_session_id` helper with jq | Handles compact output, special characters |

**Key insight:** The Phase 30 infrastructure handles all shared concerns. Copilot-specific code is only the fixture format, hook invocation pattern, and session synthesis verification.

## Common Pitfalls

### Pitfall 1: Copilot-Native vs CchEvent JSON Format Confusion
**What goes wrong:** Using CchEvent format fixtures (with `hook_event_name`, `session_id`, `agent`) when testing through the hook script, or using Copilot-native format when testing direct ingest.
**Why it happens:** Other CLIs (Claude Code, Gemini) include hook_event_name in their JSON. Copilot does not -- event type comes as $1 argument.
**How to avoid:** Two separate fixture sets: Copilot-native (for hook tests) and CchEvent (for pipeline tests). Label clearly.
**Warning signs:** Hook script silently exits 0 but no events appear in daemon.

### Pitfall 2: Session File Leakage Between Tests
**What goes wrong:** A session file from a previous test persists in /tmp, causing the hook to reuse a stale session ID instead of generating a new one.
**Why it happens:** The hook uses `/tmp/copilot-memory-session-{hash}` which persists across test runs if not cleaned up.
**How to avoid:** Each hook test must use a unique CWD (use TEST_WORKSPACE path) and clean up session files in teardown.
**Warning signs:** Session IDs don't match expectations; non-deterministic test failures.

### Pitfall 3: Hook Script Runs memory-ingest in Background
**What goes wrong:** Layer 2 gRPC query returns "No events found" because background ingest hasn't completed.
**Why it happens:** The Copilot hook script uses `echo "$PAYLOAD" | "$INGEST_BIN" >/dev/null 2>/dev/null &` -- background execution.
**How to avoid:** Always `sleep 2` between hook invocation and gRPC query (established pattern from Phase 31).
**Warning signs:** Intermittent test failures where events are "sometimes" found.

### Pitfall 4: No Stdout from Copilot Hook
**What goes wrong:** Tests assert on stdout content (like Gemini's `{}` output) and fail.
**Why it happens:** Copilot hook explicitly redirects all output to /dev/null. Comment in script: "No stdout output (Copilot ignores stdout for most events)."
**How to avoid:** For hook tests, assert only on exit code (always 0) and gRPC query results. Do NOT assert on stdout.
**Warning signs:** `[[ "$output" == '{}' ]]` fails for Copilot hook tests.

### Pitfall 5: Copilot Binary Name
**What goes wrong:** Tests look for `gh copilot` instead of `copilot`.
**Why it happens:** Historical Copilot was a `gh` extension. Current Copilot CLI is standalone binary named `copilot`.
**How to avoid:** Use `require_cli copilot "Copilot CLI"` -- the binary name is `copilot`.
**Warning signs:** Binary detection test always skips even when Copilot is installed.

### Pitfall 6: Unix Millisecond Timestamps in Copilot Fixtures
**What goes wrong:** Using ISO 8601 timestamps in Copilot-native fixtures.
**Why it happens:** CchEvent uses ISO 8601, but Copilot's raw hook input uses Unix milliseconds.
**How to avoid:** Copilot-native fixtures must use integer ms timestamps (e.g., `1709640000000`). The hook script converts them to ISO 8601.
**Warning signs:** Timestamp conversion in hook produces wrong/current-time fallback values.

### Pitfall 7: toolArgs is a JSON String, Not an Object
**What goes wrong:** Copilot fixtures use `"toolArgs": {"path": "/test.rs"}` (object) instead of `"toolArgs": "{\"path\":\"/test.rs\"}"` (string).
**Why it happens:** Most JSON conventions use objects for nested data. Copilot quirk: toolArgs is a JSON-encoded string requiring double-parse.
**How to avoid:** Always use string-encoded toolArgs in Copilot-native fixtures.
**Warning signs:** Hook script's jq redaction filter fails silently on toolArgs.

### Pitfall 8: Bug #991 -- sessionStart Fires Per-Prompt
**What goes wrong:** Tests assume sessionStart creates a new session each time.
**Why it happens:** Copilot Bug #991: sessionStart fires on every prompt in interactive mode. Hook script reuses existing session ID if session file exists.
**How to avoid:** Test the reuse behavior explicitly -- invoke sessionStart twice with same CWD, verify same session ID.
**Warning signs:** Multiple session IDs when expecting one.

## Code Examples

### Copilot-Native Fixture: session-start.json
```json
{"cwd":"/tmp/test-workspace","timestamp":1709640000000}
```
Note: No hook_event_name, no session_id, no agent. Event type passed as $1.

### Copilot-Native Fixture: user-prompt.json
```json
{"cwd":"/tmp/test-workspace","timestamp":1709640001000,"prompt":"Explain the project structure"}
```
Note: Uses `.prompt` field, not `.message`.

### Copilot-Native Fixture: post-tool-use.json
```json
{"cwd":"/tmp/test-workspace","timestamp":1709640002000,"toolName":"Read","toolArgs":"{\"path\":\"/test.rs\"}"}
```
Note: `.toolArgs` is a JSON string, not an object.

### Copilot-Native Fixture: session-end.json
```json
{"cwd":"/tmp/test-workspace","timestamp":1709640005000,"reason":"user_exit"}
```
Note: `.reason` field controls session file cleanup.

### Hook Script Invocation Pattern
```bash
# Source: Established pattern from Phase 31 Gemini hooks.bats
HOOK_SCRIPT="${PROJECT_ROOT}/plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh"

# Invoke hook with unique CWD to get predictable session file
run bash -c "echo '$json' | \
  MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
  MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
  '$HOOK_SCRIPT' sessionStart"

# Assert exit 0 (fail-open, no stdout expected)
[ "$status" -eq 0 ]
```

### Session ID Synthesis Verification
```bash
# Compute expected session file path from CWD
local test_cwd="${TEST_WORKSPACE}/copilot-project"
mkdir -p "$test_cwd"

local cwd_hash
cwd_hash=$(printf '%s' "$test_cwd" | md5sum 2>/dev/null | cut -d' ' -f1 || \
           printf '%s' "$test_cwd" | md5 2>/dev/null)
local session_file="/tmp/copilot-memory-session-${cwd_hash}"

# Clean up any stale session file
rm -f "$session_file" 2>/dev/null

# Invoke hook with cwd pointing to test workspace
local json="{\"cwd\":\"${test_cwd}\",\"timestamp\":1709640000000}"
run bash -c "echo '$json' | \
  MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
  MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
  '$HOOK_SCRIPT' sessionStart"

[ "$status" -eq 0 ]

# Verify session file was created
[ -f "$session_file" ]

# Verify session ID format
local sid
sid=$(cat "$session_file")
[[ "$sid" == copilot-* ]]

# Cleanup
rm -f "$session_file"
```

### run_copilot Wrapper (to add to cli_wrappers.bash)
```bash
run_copilot() {
    # Usage: run_copilot <prompt> [extra args...]
    # Wraps copilot CLI in headless mode with timeout.
    # Note: Copilot does NOT have JSON output mode.
    local test_stderr="${TEST_WORKSPACE:-/tmp}/copilot_stderr.log"
    export TEST_STDERR="${test_stderr}"

    local cmd=("copilot" "-p" "$@" "--allow-all-tools")

    if [[ -n "${TIMEOUT_CMD}" ]]; then
        "${TIMEOUT_CMD}" "${CLI_TIMEOUT}s" "${cmd[@]}" 2>"${test_stderr}"
    else
        "${cmd[@]}" 2>"${test_stderr}"
    fi
}
```

### CchEvent Direct Ingest (for pipeline tests)
```bash
# 5 Copilot event types mapped to CchEvent names:
#   sessionStart     -> SessionStart
#   sessionEnd       -> Stop
#   userPromptSubmitted -> UserPromptSubmit
#   preToolUse       -> PreToolUse
#   postToolUse      -> PostToolUse

_ingest_full_copilot_session() {
  local session_id="${1}"
  local ts_base
  ts_base="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"${session_id}\",\"agent\":\"copilot\",\"cwd\":\"/tmp/test\",\"timestamp\":\"${ts_base}\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"${session_id}\",\"message\":\"What is 2+2?\",\"agent\":\"copilot\",\"timestamp\":\"${ts_base}\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"copilot\",\"timestamp\":\"${ts_base}\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"PostToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"copilot\",\"timestamp\":\"${ts_base}\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"${session_id}\",\"agent\":\"copilot\",\"timestamp\":\"${ts_base}\"}" >/dev/null
}
```

## Copilot-Specific Test Differentiators

These are tests unique to Copilot that don't exist in other CLI phases:

| Test | What It Verifies | Why Copilot-Specific |
|------|-----------------|---------------------|
| Session ID synthesis | Session file created at `/tmp/copilot-memory-session-{hash}` | Only Copilot synthesizes session IDs |
| Deterministic CWD hashing | Same CWD always produces same session file path | md5-based hashing is Copilot-only |
| Bug #991 reuse | Second sessionStart with same CWD reuses session ID | Copilot-specific bug workaround |
| Session cleanup on terminal reason | Session file removed on "user_exit" or "complete" | Only Copilot has this cleanup logic |
| Session preserved on non-terminal | Session file NOT removed on other reasons | Bug #991 resumed session support |
| Event type via $1 argument | Hook receives event type as CLI arg, not in JSON | Unique to Copilot among all adapters |
| ms timestamp conversion | Unix milliseconds converted to ISO 8601 | Only Copilot uses ms timestamps |
| toolArgs double-parse | JSON string toolArgs correctly parsed | Copilot-specific JSON encoding quirk |
| No stdout from hook | Hook produces no output on stdout | Copilot hook redirects all output to /dev/null |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `gh copilot` (gh extension) | `copilot` (standalone binary) | 2025 | Binary detection uses `copilot`, not `gh copilot` |
| No headless mode | `-p` with `--allow-all-tools` | 2025 | Enables non-interactive testing |

**Important notes:**
- Copilot CLI does NOT have a JSON output mode (unlike Claude Code `--output-format json` or OpenCode `--format json`)
- Copilot CLI uses `--allow-all-tools` for non-interactive tool approval (similar to `--yes` in some contexts)
- The `-p` flag passes a prompt for headless execution

## Open Questions

1. **Does `copilot -p` exit cleanly after completing a task?**
   - What we know: Claude Code and OpenCode both have headless modes that exit. Copilot has `-p` flag.
   - What's unclear: Whether timeout handling is needed (like OpenCode quirks)
   - Recommendation: Include timeout handling in smoke tests, skip gracefully on timeout exit codes (124, 137)

2. **What does Copilot stdout look like in headless mode?**
   - What we know: No JSON output mode documented. Output is likely plain text.
   - What's unclear: Exact format, whether it's parseable
   - Recommendation: Smoke test asserts non-empty output only, no format parsing. Focus on hook/ingest pipeline.

## 2-Plan Pattern (from Phase 31/32)

| Plan | Contents | Requirements |
|------|----------|-------------|
| Plan 01 | Fixtures (Copilot-native + CchEvent) + smoke.bats + hooks.bats | CPLT-01, CPLT-02 |
| Plan 02 | pipeline.bats + negative.bats | CPLT-03, CPLT-04 |

### Plan 01 Scope
- Create `tests/cli/fixtures/copilot/` with Copilot-native JSON fixtures (ms timestamps, .prompt, .toolName/.toolArgs)
- Create `tests/cli/copilot/smoke.bats` (binary detection, daemon health, basic ingest, Copilot CLI detection)
- Create `tests/cli/copilot/hooks.bats` (all 5 event types through hook script, session ID synthesis, Bug #991 reuse, session cleanup)
- Add `run_copilot` wrapper to cli_wrappers.bash

### Plan 02 Scope
- Create `tests/cli/copilot/pipeline.bats` (full session lifecycle via direct CchEvent ingest, TOC browse, cwd metadata, agent field preservation, concurrent sessions)
- Create `tests/cli/copilot/negative.bats` (daemon down, malformed JSON, empty stdin, unknown event type, Copilot headless timeout)

## Sources

### Primary (HIGH confidence)
- `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` -- Copilot hook implementation (238 lines), session synthesis logic, event type mapping, fail-open pattern
- `plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json` -- 5 registered events with $1 argument pattern
- `tests/cli/lib/common.bash` -- Phase 30 shared infrastructure
- `tests/cli/lib/cli_wrappers.bash` -- CLI wrappers, hook testing helpers
- `tests/cli/opencode/` -- Phase 32 test pattern (4 files: smoke, hooks, pipeline, negative)
- `tests/cli/gemini/hooks.bats` -- Phase 31 hook script invocation pattern (reference for Copilot)
- `.planning/phases/22-copilot-cli-adapter/22-01-SUMMARY.md` -- Copilot adapter implementation decisions

### Secondary (MEDIUM confidence)
- [GitHub Blog: Copilot CLI](https://github.blog/ai-and-ml/github-copilot/power-agentic-workflows-in-your-terminal-with-github-copilot-cli/) -- Confirmed binary name `copilot`, `-p` flag, `--allow-all-tools` flag
- [GitHub Docs: Using Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli) -- Confirmed `copilot` binary, `--allow-all` and `--yolo` permission flags

### Tertiary (LOW confidence)
- Copilot CLI headless exit behavior -- Not fully documented; recommend defensive timeout handling

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- Reusing established Phase 30 infrastructure, well-documented
- Architecture: HIGH -- Following proven 2-plan pattern from Phase 31/32, Copilot adapter code is in-repo and readable
- Pitfalls: HIGH -- Copilot-specific quirks (session synthesis, ms timestamps, toolArgs double-parse, no stdout) all documented in hook script comments
- Copilot CLI flags: MEDIUM -- `-p` and `--allow-all-tools` confirmed via GitHub blog; exact headless behavior needs runtime validation

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (stable -- test patterns and adapter code are project-internal)
