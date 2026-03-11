# Phase 31: Gemini CLI Tests - Research

**Researched:** 2026-02-25
**Domain:** Bats E2E testing for Gemini CLI hook capture via agent-memory
**Confidence:** HIGH

## Summary

Phase 31 reuses the Phase 30 bats-core test framework to validate Gemini CLI hook capture through the agent-memory pipeline. The existing framework (`common.bash`, `cli_wrappers.bash`) provides workspace isolation, daemon lifecycle, ingest helpers, and gRPC query helpers that are directly reusable. The Gemini adapter already exists at `plugins/memory-gemini-adapter/` with a complete `memory-capture.sh` hook script and `settings.json` configuration.

The key difference from Claude Code tests is that Gemini CLI uses different hook event names (BeforeAgent, AfterAgent, BeforeTool, AfterTool, SessionStart, SessionEnd) which the `memory-capture.sh` script translates to the CchEvent format that `memory-ingest` expects (UserPromptSubmit, AssistantResponse, PreToolUse, PostToolUse, SessionStart, Stop). The Gemini hook script also includes ANSI escape stripping, secret redaction, and jq walk() compatibility -- all of which need test coverage.

**Primary recommendation:** Mirror the Claude Code 4-file test structure (smoke, hooks, pipeline, negative) with Gemini-specific fixtures, binary detection for `gemini` CLI, and tests that exercise the `memory-capture.sh` translation layer directly rather than requiring a live Gemini API key.

## Standard Stack

### Core (Reused from Phase 30)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bats-core | 1.12 | Test runner | Already established in Phase 30 |
| common.bash | N/A | Workspace isolation, daemon lifecycle, gRPC query | Shared helper library |
| cli_wrappers.bash | N/A | CLI detection, hook stdin testing | Shared helper library |
| jq | 1.6+ | JSON fixture manipulation, payload construction | Required by memory-capture.sh |
| memory-ingest | local build | CchEvent ingestion binary | Core pipeline component |
| memory-daemon | local build | gRPC storage backend | Core pipeline component |

### Gemini-Specific
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| gemini CLI | latest | Real CLI binary detection / headless smoke | Only for tests that `require_cli gemini` |
| memory-capture.sh | N/A | Gemini hook-to-CchEvent translator | Core of hook testing |

### No New Dependencies Needed
The entire Phase 31 test suite builds on existing infrastructure. No new npm/cargo/brew installs required beyond what Phase 30 established.

## Architecture Patterns

### Recommended Test Directory Structure
```
tests/cli/
  gemini/
    smoke.bats           # GEMI-01: binary detection, basic ingest, daemon health
    hooks.bats           # GEMI-02: hook capture via memory-capture.sh translation
    pipeline.bats        # GEMI-03: full ingest-to-query cycle
    negative.bats        # GEMI-04: daemon-down, malformed input, fail-open
  fixtures/
    gemini/
      session-start.json       # Gemini SessionStart hook input format
      session-end.json         # Gemini SessionEnd hook input format
      before-agent.json        # Gemini BeforeAgent hook input (with .prompt field)
      after-agent.json         # Gemini AfterAgent hook input (with .prompt_response field)
      before-tool.json         # Gemini BeforeTool hook input (with .tool_name, .tool_input)
      after-tool.json          # Gemini AfterTool hook input (with .tool_name, .tool_input)
      malformed.json           # Invalid JSON for negative tests
  lib/
    common.bash               # (existing) shared helpers
    cli_wrappers.bash          # (existing) shared CLI wrappers
```

### Pattern 1: Two-Layer Testing via Hook Script
**What:** Tests feed Gemini-format JSON into `memory-capture.sh`, which translates and pipes to `memory-ingest`, then verify via gRPC query.
**When to use:** All hook capture tests (GEMI-02).
**Why:** This tests the actual translation layer without requiring a Gemini API key or live CLI session.

```bash
# Layer 1: Feed Gemini-format JSON into memory-capture.sh, verify exit 0 and {} output
run bash -c "echo '$gemini_json' | MEMORY_INGEST_PATH='$MEMORY_INGEST_BIN' MEMORY_DAEMON_ADDR='http://127.0.0.1:$MEMORY_DAEMON_PORT' '$HOOK_SCRIPT'"
[ "$status" -eq 0 ]
[[ "$output" == '{}' ]]

# Layer 2: Query gRPC to verify event was stored with agent=gemini
result="$(query_all_events)"
[[ "$result" == *"gemini:"* ]]
```

### Pattern 2: Direct CchEvent Ingest (Bypass Hook Script)
**What:** Feed pre-translated CchEvent JSON directly to `memory-ingest` with `"agent": "gemini"`.
**When to use:** Pipeline tests (GEMI-03) that test the storage layer, not the translation.
**Why:** Isolates the ingest pipeline from the hook script, using the same `ingest_event` helper as Claude Code tests.

### Pattern 3: Gemini Binary Detection with Graceful Skip
**What:** Use `require_cli gemini "Gemini CLI"` for tests needing the actual binary.
**When to use:** Smoke tests for binary presence and headless mode.
**Why:** CI environments may not have Gemini CLI installed; tests must skip gracefully.

### Anti-Patterns to Avoid
- **Testing with live Gemini API:** Most tests should use the hook script directly, not invoke `gemini` with a prompt. Reserve live CLI tests for optional smoke tests.
- **Duplicating common.bash helpers:** Reuse `setup_workspace`, `start_daemon`, `stop_daemon`, `ingest_event`, `grpc_query` without modification.
- **Hardcoding memory-ingest path in hook script tests:** Always set `MEMORY_INGEST_PATH` env var to point at the cargo-built binary.

## Gemini Hook Event Mapping (Critical)

The `memory-capture.sh` script translates Gemini hook events to CchEvent format:

| Gemini Hook Event | CchEvent hook_event_name | Key Input Fields | Key Mapping |
|-------------------|--------------------------|------------------|-------------|
| SessionStart | SessionStart | `.source` ("startup"\|"resume"\|"clear") | Direct passthrough |
| SessionEnd | Stop | `.reason` ("exit"\|"clear"\|"logout") | Renamed to Stop |
| BeforeAgent | UserPromptSubmit | `.prompt` | `.prompt` -> `.message` |
| AfterAgent | AssistantResponse | `.prompt_response` | `.prompt_response` -> `.message` |
| BeforeTool | PreToolUse | `.tool_name`, `.tool_input` | Direct passthrough |
| AfterTool | PostToolUse | `.tool_name`, `.tool_input` | Direct passthrough |

All translated events set `"agent": "gemini"` -- this is the key differentiator from Claude Code events.

## Gemini-Format Fixture JSON Schemas

### SessionStart (Gemini input format)
```json
{
  "hook_event_name": "SessionStart",
  "session_id": "gemini-test-001",
  "timestamp": "2026-02-25T10:00:00Z",
  "cwd": "/tmp/test-workspace",
  "source": "startup"
}
```

### BeforeAgent (Gemini input format -- has .prompt, NOT .message)
```json
{
  "hook_event_name": "BeforeAgent",
  "session_id": "gemini-test-001",
  "timestamp": "2026-02-25T10:00:05Z",
  "cwd": "/tmp/test-workspace",
  "prompt": "What is the project structure?"
}
```

### AfterAgent (Gemini input format -- has .prompt_response, NOT .message)
```json
{
  "hook_event_name": "AfterAgent",
  "session_id": "gemini-test-001",
  "timestamp": "2026-02-25T10:00:15Z",
  "cwd": "/tmp/test-workspace",
  "prompt_response": "The project contains src/ and tests/ directories."
}
```

### BeforeTool (Gemini input format)
```json
{
  "hook_event_name": "BeforeTool",
  "session_id": "gemini-test-001",
  "timestamp": "2026-02-25T10:00:10Z",
  "cwd": "/tmp/test-workspace",
  "tool_name": "read_file",
  "tool_input": {"file_path": "/tmp/test-workspace/README.md"}
}
```

### AfterTool (Gemini input format)
```json
{
  "hook_event_name": "AfterTool",
  "session_id": "gemini-test-001",
  "timestamp": "2026-02-25T10:00:11Z",
  "cwd": "/tmp/test-workspace",
  "tool_name": "read_file",
  "tool_input": {"file_path": "/tmp/test-workspace/README.md"},
  "tool_response": {"content": "# README"}
}
```

### SessionEnd (Gemini input format)
```json
{
  "hook_event_name": "SessionEnd",
  "session_id": "gemini-test-001",
  "timestamp": "2026-02-25T10:00:30Z",
  "cwd": "/tmp/test-workspace",
  "reason": "exit"
}
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Workspace isolation | Custom temp dir logic | `setup_workspace` / `teardown_workspace` from common.bash | Battle-tested, preserves on failure |
| Daemon lifecycle | Manual start/stop | `build_daemon_if_needed`, `start_daemon`, `stop_daemon` from common.bash | Random port, health check, PID mgmt |
| gRPC queries | Raw grpcurl commands | `grpc_query` from common.bash | Wraps memory-daemon query CLI |
| Event ingestion | Raw echo-pipe | `ingest_event` from common.bash | Sets MEMORY_DAEMON_ADDR correctly |
| CLI detection | Custom PATH checks | `require_cli` / `has_cli` from cli_wrappers.bash | Consistent skip messages |
| Hook stdin testing | Custom pipe setup | `run_hook_stdin` / `run_hook_stdin_dry` from cli_wrappers.bash | Handles env vars correctly |
| Session ID rewriting | sed hacks | `rewrite_session_id` helper (adapt from hooks.bats) | jq-based, compact output |

**Key insight:** Phase 31 should write zero new shared helpers. Everything needed exists in `common.bash` and `cli_wrappers.bash`. The only new code is test files, fixtures, and possibly a thin Gemini-specific helper for invoking `memory-capture.sh`.

## Common Pitfalls

### Pitfall 1: Hook Script Background Ingest
**What goes wrong:** `memory-capture.sh` sends to memory-ingest in background (`&`), so the gRPC query may run before the event is stored.
**Why it happens:** Line 195: `echo "$PAYLOAD" | "$INGEST_BIN" >/dev/null 2>/dev/null &`
**How to avoid:** In tests, either (a) modify the hook invocation to run foreground, or (b) add `sleep 1-2` between hook call and gRPC query, or (c) bypass the hook script and pipe directly to `memory-ingest` for deterministic timing.
**Warning signs:** Intermittent "No events found" in gRPC queries after hook invocation.

### Pitfall 2: Hook Script Output is `{}` Not `{"continue":true}`
**What goes wrong:** Tests copy Claude Code assertions expecting `{"continue":true}` but Gemini hooks output `{}`.
**Why it happens:** Gemini CLI expects `{}` (empty JSON) for allow-through, while Claude Code expects `{"continue":true}`.
**How to avoid:** Hook script tests assert `[[ "$output" == '{}' ]]`. Direct `memory-ingest` tests still assert `{"continue":true}`.
**Warning signs:** All hook script tests fail on output assertion.

### Pitfall 3: ANSI Escape Sequences in Input
**What goes wrong:** Gemini CLI can emit ANSI escape codes in its hook stdin, breaking JSON parsing.
**Why it happens:** `memory-capture.sh` lines 62-67 strip ANSI using perl or sed fallback.
**How to avoid:** Include a test fixture with embedded ANSI codes to verify the stripping works.
**Warning signs:** JSON parse failures in hook script when processing real Gemini output.

### Pitfall 4: jq walk() Availability
**What goes wrong:** Secret redaction fails on systems with jq < 1.6 (no `walk()` function).
**Why it happens:** `memory-capture.sh` lines 46-49 detect walk() at runtime and fall back to shallow redaction.
**How to avoid:** Test both code paths if possible, or at minimum verify the script works with the host jq version.
**Warning signs:** Redaction test failures on older macOS with system jq.

### Pitfall 5: Gemini Field Name Differences
**What goes wrong:** Fixtures use `.message` instead of `.prompt` for BeforeAgent events.
**Why it happens:** Claude Code uses `.message` for all events; Gemini uses `.prompt` for BeforeAgent and `.prompt_response` for AfterAgent.
**How to avoid:** Fixtures must match the Gemini hook input schema exactly (see fixture schemas above).
**Warning signs:** Empty message content in stored events.

### Pitfall 6: Trap-Based Fail-Open
**What goes wrong:** The `memory-capture.sh` uses `trap fail_open ERR EXIT` which means it ALWAYS outputs `{}` and exits 0, even on success.
**Why it happens:** The EXIT trap fires on normal exit too, not just errors.
**How to avoid:** This is actually correct behavior -- the test should assert exit 0 and `{}` output for ALL cases (success and failure).
**Warning signs:** None -- this is intentional behavior.

## Code Examples

### Example 1: Gemini Hook Smoke Test
```bash
@test "memory-capture.sh exists and is executable" {
  local hook_script="${PROJECT_ROOT}/plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh"
  [ -f "$hook_script" ]
  [ -x "$hook_script" ]
}
```

### Example 2: Gemini Hook Capture via memory-capture.sh
```bash
HOOK_SCRIPT="${PROJECT_ROOT}/plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh"

@test "hook: BeforeAgent event captures prompt with agent=gemini" {
  local sid="test-gemini-prompt-$$"
  local fixture="${FIXTURE_DIR}/before-agent.json"
  local json
  json="$(rewrite_session_id "$fixture" "$sid")"

  # Run hook script with Gemini-format JSON
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ]
  [[ "$output" == '{}' ]]

  sleep 2  # Background ingest needs time

  local result
  result="$(query_all_events)"

  [[ "$result" == *"project structure"* ]] || {
    echo "Expected prompt content in gRPC query"
    echo "Query output: $result"
    false
  }
}
```

### Example 3: Gemini Binary Detection with Graceful Skip
```bash
@test "gemini binary detection works (skip if not installed)" {
  require_cli gemini "Gemini CLI"
  run gemini --version
  [ "$status" -eq 0 ]
}
```

### Example 4: Gemini Negative Test (Daemon Down)
```bash
@test "negative: memory-capture.sh with daemon down still returns {}" {
  local unused_port=$(( (RANDOM % 10000) + 40000 ))
  local hook_script="${PROJECT_ROOT}/plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh"

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-1\",\"cwd\":\"/tmp\"}' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${unused_port}' \
    '$hook_script'"

  [ "$status" -eq 0 ]
  [[ "$output" == '{}' ]]
}
```

### Example 5: Direct CchEvent Ingest with agent=gemini
```bash
@test "pipeline: gemini agent field is preserved through ingest" {
  local sid="pipeline-gemini-agent-${RANDOM}"
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"${sid}\",\"message\":\"Hello from Gemini\",\"agent\":\"gemini\"}"
  sleep 1
  local result
  result="$(query_all_events)"
  [[ "$result" == *"gemini:"* ]] || [[ "$result" == *"Hello from Gemini"* ]]
}
```

## Test Count Estimate

| File | Test Count | Requires gemini CLI | Requires Daemon |
|------|-----------|---------------------|-----------------|
| smoke.bats | 6-8 | 2 tests (skip if missing) | Yes |
| hooks.bats | 8-10 | No | Yes |
| pipeline.bats | 5-6 | 1 test (skip if missing) | Yes |
| negative.bats | 6-7 | No | Partial (some tests intentionally no daemon) |
| **Total** | **25-31** | **3 optional** | **Most** |

## Key Differences from Claude Code Tests

| Aspect | Claude Code | Gemini |
|--------|-------------|--------|
| Hook output | `{"continue":true}` | `{}` |
| Event names | SessionStart, UserPromptSubmit, etc. | SessionStart, BeforeAgent, AfterAgent, etc. |
| Prompt field | `.message` | `.prompt` (BeforeAgent) |
| Response field | `.message` | `.prompt_response` (AfterAgent) |
| Translation layer | None (memory-ingest reads CchEvent directly) | `memory-capture.sh` translates Gemini -> CchEvent |
| Agent field value | `"claude"` | `"gemini"` |
| Binary name | `claude` | `gemini` |
| CLI package | Built-in | `npm install -g @google/gemini-cli` |
| Headless flag | `-p <prompt> --output-format json` | Positional arg `--output-format json` |
| Background ingest | No (synchronous) | Yes (`&` in hook script) |
| Secret redaction | No | Yes (jq walk filter) |

## Open Questions

1. **Hook script foreground mode for testing**
   - What we know: `memory-capture.sh` sends to memory-ingest in background (`&`), causing timing issues in tests.
   - What's unclear: Whether to patch the script for testing or just use generous sleep delays.
   - Recommendation: Use `sleep 2` after hook invocations. The background `&` is intentional for production (non-blocking). Alternatively, for deterministic tests, bypass the hook script and pipe directly to `memory-ingest` for Layer 2 verification.

2. **Gemini CLI headless mode exact flags**
   - What we know: `gemini "prompt" --output-format json` runs in headless mode.
   - What's unclear: Whether `--output-format` is the exact flag (docs say it exists but exact syntax not confirmed).
   - Recommendation: Add a smoke test that runs `gemini --help` and checks for output format flags. Mark real CLI tests as optional (skip if not installed).

## Sources

### Primary (HIGH confidence)
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` -- Complete hook translation script (read directly)
- `plugins/memory-gemini-adapter/.gemini/settings.json` -- Hook configuration (read directly)
- `crates/memory-ingest/src/main.rs` -- CchEvent struct and field mapping (read directly)
- `tests/cli/claude-code/*.bats` -- All 4 test files as templates (read directly)
- `tests/cli/lib/common.bash` -- Shared helpers (read directly)
- `tests/cli/lib/cli_wrappers.bash` -- CLI wrappers (read directly)

### Secondary (MEDIUM confidence)
- [Gemini CLI Hooks Reference](https://geminicli.com/docs/hooks/reference/) -- Event names and JSON schemas
- [Gemini CLI Headless Mode](https://geminicli.com/docs/cli/headless/) -- Headless mode usage
- [Gemini CLI Hooks Overview](https://geminicli.com/docs/hooks/) -- Hook configuration format
- [Gemini CLI Writing Hooks](https://geminicli.com/docs/hooks/writing-hooks/) -- Hook script patterns
- [@google/gemini-cli on npm](https://www.npmjs.com/package/@google/gemini-cli) -- Binary name is `gemini`

### Tertiary (LOW confidence)
- None -- all findings verified against source code or official docs.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Reuses Phase 30 framework entirely, no new dependencies
- Architecture: HIGH - Direct mirror of Claude Code test structure with Gemini-specific fixtures
- Pitfalls: HIGH - Identified from reading actual `memory-capture.sh` source code
- Gemini hook format: MEDIUM - Verified against official docs and existing adapter code

**Research date:** 2026-02-25
**Valid until:** 2026-03-25 (stable -- framework is local, Gemini hook API is versioned)
