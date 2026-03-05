# Phase 32: OpenCode CLI Tests - Research

**Researched:** 2026-02-26
**Domain:** Bats E2E testing for OpenCode CLI hook capture via agent-memory
**Confidence:** HIGH

## Summary

Phase 32 reuses the Phase 30 bats-core test framework to validate OpenCode CLI hook capture through the agent-memory pipeline. The existing framework (`common.bash`, `cli_wrappers.bash`) provides workspace isolation, daemon lifecycle, ingest helpers, and gRPC query helpers that are directly reusable. The OpenCode plugin already exists at `plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` -- a TypeScript plugin that hooks into OpenCode lifecycle events and pipes CchEvent-format JSON to `memory-ingest`.

The critical difference from Gemini tests (Phase 31) is that OpenCode uses a **TypeScript plugin** (`memory-capture.ts`) rather than a **bash hook script** (`memory-capture.sh`). This means hook capture tests cannot pipe JSON through a shell script directly. Instead, the TypeScript plugin translates OpenCode events (session.created, message.updated, tool.execute.after, session.idle) into CchEvent format and shells out to `memory-ingest`. For test purposes, the hook capture tests should feed **pre-translated CchEvent JSON** (with `"agent": "opencode"`) directly to `memory-ingest`, since testing the TypeScript plugin requires a running OpenCode process. The hook event mapping is: `session.created` -> SessionStart, `session.idle` -> Stop, `message.updated` (role=user) -> UserPromptSubmit, `message.updated` (role=assistant) -> AssistantResponse, `tool.execute.after` -> PostToolUse.

There is a complication with headless mode flags. The REQUIREMENTS.md specifies `-p -q -f json` which matches the **archived opencode-ai/opencode** project. The **current active OpenCode** (anomalyco/opencode, formerly sst/opencode) uses `opencode run "prompt" --format json`. Since the project plugin imports `@opencode-ai/plugin`, the tests should handle both variants and skip gracefully when the binary is not installed.

**Primary recommendation:** Mirror the Claude Code/Gemini 4-file test structure (smoke, hooks, pipeline, negative) with OpenCode-specific fixtures. Hook capture tests feed CchEvent JSON directly to `memory-ingest` with `"agent": "opencode"` (bypassing TypeScript plugin). Smoke tests detect the `opencode` binary and test both flag variants with graceful skip. Add OpenCode-specific timeout guards for headless mode quirks.

## Standard Stack

### Core (Reused from Phase 30)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bats-core | 1.12 | Test runner | Already established in Phase 30 |
| common.bash | N/A | Workspace isolation, daemon lifecycle, gRPC query | Shared helper library |
| cli_wrappers.bash | N/A | CLI detection, hook stdin testing | Shared helper library |
| jq | 1.6+ | JSON fixture manipulation, payload construction | Required for compact JSON |
| memory-ingest | local build | CchEvent ingestion binary | Core pipeline component |
| memory-daemon | local build | gRPC storage backend | Core pipeline component |

### OpenCode-Specific
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| opencode CLI | latest | Real CLI binary detection / headless smoke | Only for tests that `require_cli opencode` |
| memory-capture.ts | N/A | OpenCode plugin event translator (TypeScript) | NOT directly testable in shell; test via direct CchEvent ingest |

### No New Dependencies Needed
The entire Phase 32 test suite builds on existing infrastructure. No new npm/cargo/brew installs required beyond what Phase 30 established.

## Architecture Patterns

### Recommended Test Directory Structure
```
tests/cli/
  opencode/
    smoke.bats           # OPEN-01: binary detection, basic ingest, daemon health
    hooks.bats           # OPEN-02: hook capture via direct CchEvent ingest with agent=opencode
    pipeline.bats        # OPEN-03: full ingest-to-query cycle
    negative.bats        # OPEN-04: daemon-down, malformed input, timeout, fail-open
  fixtures/
    opencode/
      session-start.json       # CchEvent: SessionStart with agent=opencode
      user-prompt.json         # CchEvent: UserPromptSubmit with agent=opencode
      assistant-response.json  # CchEvent: AssistantResponse with agent=opencode
      post-tool-use.json       # CchEvent: PostToolUse with agent=opencode
      stop.json                # CchEvent: Stop with agent=opencode
      malformed.json           # Invalid JSON for negative tests
  lib/
    common.bash               # (existing) shared helpers
    cli_wrappers.bash          # (existing) shared CLI wrappers -- needs run_opencode wrapper
```

### Pattern 1: Direct CchEvent Ingest (Primary Test Pattern)
**What:** Tests feed pre-translated CchEvent JSON directly to `memory-ingest` with `"agent": "opencode"`, then verify via gRPC query.
**When to use:** All hook capture tests (OPEN-02) and pipeline tests (OPEN-03).
**Why:** The OpenCode plugin is TypeScript, not a bash script. Testing the TypeScript plugin requires a running OpenCode session with API keys. Direct CchEvent ingest tests the storage layer that the plugin writes to.

```bash
# Layer 1: Feed CchEvent JSON to memory-ingest, verify continue:true
run ingest_event '{"hook_event_name":"UserPromptSubmit","session_id":"opencode-test-001","message":"What is the project structure?","agent":"opencode"}'
[ "$status" -eq 0 ]
[[ "$output" == *'"continue":true'* ]]

# Layer 2: Query gRPC to verify event was stored with agent=opencode
sleep 1
local result
result="$(query_all_events)"
[[ "$result" == *"opencode:"* ]] || [[ "$result" == *"project structure"* ]]
```

### Pattern 2: OpenCode Binary Detection with Graceful Skip
**What:** Use `require_cli opencode "OpenCode"` for tests needing the actual binary.
**When to use:** Smoke tests for binary presence and headless mode.
**Why:** CI environments and most dev machines will NOT have OpenCode installed; tests must skip gracefully.

```bash
@test "opencode binary detection works (skip if not installed)" {
  require_cli opencode "OpenCode"
  run opencode --version
  [ "$status" -eq 0 ]
}
```

### Pattern 3: Headless Invocation with Timeout Guards
**What:** Wrap OpenCode headless calls in timeout to prevent hangs.
**When to use:** Any smoke test that invokes the real `opencode` binary.
**Why:** OpenCode's headless mode (`opencode run`) is less mature than Claude Code's `-p` mode. Timeout guards prevent test suite hangs.

```bash
@test "opencode headless mode produces output (skip if not installed)" {
  require_cli opencode "OpenCode"
  local timeout_cmd
  timeout_cmd="$(detect_timeout_cmd)"
  [[ -n "$timeout_cmd" ]] || skip "No timeout command available"

  # Use generous timeout for headless mode
  run "$timeout_cmd" 30s opencode run --format json "echo hello"
  # Accept exit 0 (success) or 124/137 (timeout) -- both are informative
  if [[ "$status" -eq 124 ]] || [[ "$status" -eq 137 ]]; then
    skip "OpenCode headless mode timed out (known quirk)"
  fi
  [ "$status" -eq 0 ]
}
```

### Pattern 4: cli_wrappers.bash Extension -- run_opencode Wrapper
**What:** Add `run_opencode` function to `cli_wrappers.bash` for OpenCode headless invocation.
**When to use:** Smoke tests that invoke the real CLI.
**Why:** Centralizes timeout handling, flag selection, and stderr capture.

```bash
run_opencode() {
  # Usage: run_opencode <prompt> [extra args...]
  local test_stderr="${TEST_WORKSPACE:-/tmp}/opencode_stderr.log"
  export TEST_STDERR="${test_stderr}"

  local cmd=("opencode" "run" "--format" "json" "$@")

  if [[ -n "${TIMEOUT_CMD}" ]]; then
    "${TIMEOUT_CMD}" "${CLI_TIMEOUT}s" "${cmd[@]}" 2>"${test_stderr}"
  else
    "${cmd[@]}" 2>"${test_stderr}"
  fi
}
```

### Anti-Patterns to Avoid
- **Trying to test memory-capture.ts directly via shell:** The TypeScript plugin runs inside the OpenCode process. It cannot be invoked standalone. Test the CchEvent pipeline instead.
- **Assuming `-p -q -f json` flags:** The current anomalyco/opencode uses `opencode run --format json "prompt"`. The old opencode-ai/opencode used `-p -q -f json`. The tests should use the current `run` subcommand syntax.
- **Duplicating common.bash helpers:** Reuse `setup_workspace`, `start_daemon`, `stop_daemon`, `ingest_event`, `grpc_query` without modification.
- **Requiring OpenCode binary for hook tests:** Most tests should work without the `opencode` binary by feeding CchEvent JSON directly to `memory-ingest`.

## OpenCode Event Mapping (Critical)

The `memory-capture.ts` plugin translates OpenCode lifecycle events to CchEvent format:

| OpenCode Plugin Event | CchEvent hook_event_name | Key Input Fields | Key Mapping |
|----------------------|--------------------------|------------------|-------------|
| session.created | SessionStart | input.id or input.sessionID | extractSessionId() |
| session.idle | Stop | input.id or input.sessionID | Maps to Stop (checkpoint) |
| message.updated (role=user) | UserPromptSubmit | properties.message.content | String or JSON.stringify(content) |
| message.updated (role=assistant) | AssistantResponse | properties.message.content | String or JSON.stringify(content) |
| tool.execute.after | PostToolUse | input.tool, input.args | tool -> tool_name, args -> tool_input |

All translated events set `"agent": "opencode"` -- this is the key differentiator from Claude Code and Gemini events.

**NOTE:** Unlike Gemini which has PreToolUse (BeforeTool), OpenCode only captures PostToolUse (tool.execute.after). There is no tool.execute.before hook in OpenCode's plugin API. This means OpenCode fixtures should NOT include a PreToolUse fixture.

## OpenCode-Format Fixture JSON Schemas (CchEvent format)

All fixtures use the CchEvent format that `memory-ingest` directly consumes, with `"agent": "opencode"`.

### SessionStart
```json
{"hook_event_name":"SessionStart","session_id":"opencode-test-001","timestamp":"2026-02-26T10:00:00Z","cwd":"/tmp/test-workspace","agent":"opencode"}
```

### UserPromptSubmit
```json
{"hook_event_name":"UserPromptSubmit","session_id":"opencode-test-001","timestamp":"2026-02-26T10:00:05Z","message":"What is the project structure?","agent":"opencode"}
```

### AssistantResponse
```json
{"hook_event_name":"AssistantResponse","session_id":"opencode-test-001","timestamp":"2026-02-26T10:00:15Z","message":"The project contains src/ and tests/ directories.","agent":"opencode"}
```

### PostToolUse
```json
{"hook_event_name":"PostToolUse","session_id":"opencode-test-001","timestamp":"2026-02-26T10:00:10Z","tool_name":"read_file","tool_input":{"file_path":"/tmp/test-workspace/README.md"},"agent":"opencode"}
```

### Stop
```json
{"hook_event_name":"Stop","session_id":"opencode-test-001","timestamp":"2026-02-26T10:00:30Z","cwd":"/tmp/test-workspace","agent":"opencode"}
```

### malformed.json
```json
{not valid json at all -- this is intentionally broken
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Workspace isolation | Custom temp dir logic | `setup_workspace` / `teardown_workspace` from common.bash | Battle-tested, preserves on failure |
| Daemon lifecycle | Manual start/stop | `build_daemon_if_needed`, `start_daemon`, `stop_daemon` from common.bash | Random port, health check, PID mgmt |
| gRPC queries | Raw grpcurl commands | `grpc_query` from common.bash | Wraps memory-daemon query CLI |
| Event ingestion | Raw echo-pipe | `ingest_event` from common.bash | Sets MEMORY_DAEMON_ADDR correctly |
| CLI detection | Custom PATH checks | `require_cli` / `has_cli` from cli_wrappers.bash | Consistent skip messages |
| Timeout wrapping | Custom timer | `detect_timeout_cmd` + `TIMEOUT_CMD` from cli_wrappers.bash | macOS/Linux compatible |
| Session ID rewriting | sed hacks | `rewrite_session_id` helper (from hooks.bats pattern) | jq-based, compact output |

**Key insight:** Phase 32 should write zero new shared helpers beyond adding `run_opencode` to `cli_wrappers.bash`. The only new code is test files (4 .bats files), fixtures (6 JSON files), and the `run_opencode` wrapper.

## Common Pitfalls

### Pitfall 1: TypeScript Plugin Not Shell-Testable
**What goes wrong:** Developer tries to invoke `memory-capture.ts` directly from bash, as was done with Gemini's `memory-capture.sh`.
**Why it happens:** Pattern from Phase 31 (Gemini) used `echo JSON | bash hook_script`. OpenCode's plugin is TypeScript, not bash.
**How to avoid:** All hook capture tests (OPEN-02) use direct CchEvent ingest via `ingest_event` helper. Do not attempt to invoke the TypeScript plugin.
**Warning signs:** Tests referencing `.opencode/plugin/memory-capture.ts` in bash `run` commands.

### Pitfall 2: Wrong Headless Mode Flags
**What goes wrong:** Tests use `-p -q -f json` (old opencode-ai/opencode) but the installed binary is anomalyco/opencode (current active project).
**Why it happens:** REQUIREMENTS.md references `-p -q -f json` which was the syntax for the old archived project.
**How to avoid:** Use `opencode run --format json "prompt"` as the primary headless invocation. The `run_opencode` wrapper should encapsulate the correct flags. Add a flag-detection smoke test that tries both syntaxes.
**Warning signs:** "unknown flag" errors from opencode CLI.

### Pitfall 3: No PreToolUse in OpenCode
**What goes wrong:** Fixtures include a PreToolUse event expecting it to match Gemini's BeforeTool or Claude Code's PreToolUse.
**Why it happens:** OpenCode only has `tool.execute.after` hook, no `tool.execute.before`.
**How to avoid:** OpenCode fixture set should have only 5 event types (SessionStart, UserPromptSubmit, AssistantResponse, PostToolUse, Stop). No PreToolUse fixture.
**Warning signs:** Test expecting "PreToolUse" events from OpenCode source.

### Pitfall 4: OpenCode Binary Rarely Available in CI
**What goes wrong:** Smoke tests that require the `opencode` binary fail in CI, causing the entire test suite to appear broken.
**Why it happens:** OpenCode must be separately installed (`curl -fsSL https://opencode.ai/install | bash`), and CI environments typically do not have it.
**How to avoid:** All tests requiring `opencode` binary must use `require_cli opencode "OpenCode"` to skip gracefully. Keep the number of CLI-dependent tests to a minimum (2-3 smoke tests).
**Warning signs:** CI failures with "opencode not found on PATH".

### Pitfall 5: Headless Mode Hangs
**What goes wrong:** `opencode run "prompt"` hangs indefinitely, blocking the entire bats test suite.
**Why it happens:** OpenCode headless mode is less mature than Claude Code. It may wait for input, spawn a TUI, or hang on API errors.
**How to avoid:** Always wrap `opencode run` in a timeout (30s max). Use the `run_opencode` wrapper with `TIMEOUT_CMD`. If timeout fires, skip the test rather than fail.
**Warning signs:** Tests that take 120+ seconds.

### Pitfall 6: OpenCode Ecosystem Fragmentation
**What goes wrong:** Developer installs the wrong `opencode` binary (there are multiple projects with similar names).
**Why it happens:** There are at least two major "OpenCode" projects: opencode-ai/opencode (archived, became Crush) and anomalyco/opencode (active, formerly sst/opencode).
**How to avoid:** Smoke test should detect which variant is installed by checking `opencode --version` output. Document the expected binary source in test file comments.
**Warning signs:** Unexpected --version output or unknown subcommands.

## Code Examples

### Example 1: OpenCode Smoke Test (Binary Detection)
```bash
@test "opencode binary detection works (skip if not installed)" {
  require_cli opencode "OpenCode"
  run opencode --version
  [ "$status" -eq 0 ]
}
```

### Example 2: OpenCode Hook Capture via Direct CchEvent Ingest
```bash
@test "hook: UserPromptSubmit event captures message with agent=opencode" {
  local sid="test-opencode-prompt-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/user-prompt.json" "$sid")"

  ingest_fixture "$json"
  sleep 1

  local result
  result="$(query_all_events)"

  [[ "$result" == *"project structure"* ]] || {
    echo "Expected 'project structure' in gRPC query result"
    echo "Query output: $result"
    false
  }
}
```

### Example 3: OpenCode Headless Smoke with Timeout Guard
```bash
@test "opencode headless mode produces output (skip if not installed)" {
  require_cli opencode "OpenCode"

  local timeout_cmd
  timeout_cmd="$(detect_timeout_cmd)"
  [[ -n "$timeout_cmd" ]] || skip "No timeout command available"

  run "$timeout_cmd" 30s opencode run --format json "echo hello"

  if [[ "$status" -eq 124 ]] || [[ "$status" -eq 137 ]]; then
    skip "OpenCode headless mode timed out (known quirk)"
  fi

  [ "$status" -eq 0 ]
  [[ -n "$output" ]]
}
```

### Example 4: OpenCode Negative Test (Daemon Down)
```bash
@test "negative: memory-ingest with daemon down still returns continue:true (opencode)" {
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-o1\",\"agent\":\"opencode\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${unused_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} but got: $output"
    false
  }
}
```

### Example 5: OpenCode Pipeline (Full Session Lifecycle)
```bash
@test "pipeline: complete opencode session lifecycle via hook ingest" {
  assert_daemon_running

  local session_id="opencode-pipeline-lifecycle-${RANDOM}"
  local time_before
  time_before="$(_now_ms)"

  # 5-event session (no PreToolUse in OpenCode)
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"${session_id}\",\"agent\":\"opencode\",\"cwd\":\"/tmp/test\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"${session_id}\",\"message\":\"What is 2+2?\",\"agent\":\"opencode\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"PostToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"opencode\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"AssistantResponse\",\"session_id\":\"${session_id}\",\"message\":\"The answer is 4.\",\"agent\":\"opencode\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"${session_id}\",\"agent\":\"opencode\"}" >/dev/null

  sleep 2

  local time_after
  time_after="$(_now_ms)"

  run grpc_query events --from "${time_before}" --to "${time_after}"
  [ "$status" -eq 0 ]

  [[ "$output" == *"5 found"* ]] || {
    echo "Expected 5 events found in output"
    echo "Query output: $output"
    false
  }
}
```

## Key Differences from Claude Code and Gemini Tests

| Aspect | Claude Code | Gemini | OpenCode |
|--------|-------------|--------|----------|
| Hook mechanism | memory-ingest reads stdin directly | memory-capture.sh (bash) translates | memory-capture.ts (TypeScript plugin) |
| Hook script testable from shell? | N/A (direct ingest) | Yes (pipe JSON to .sh) | **No** (TypeScript requires OpenCode runtime) |
| Hook output | `{"continue":true}` | `{}` | N/A (plugin is async, no direct output) |
| Agent field value | `"claude"` | `"gemini"` | `"opencode"` |
| Binary name | `claude` | `gemini` | `opencode` |
| Headless invocation | `claude -p "prompt" --output-format json` | `gemini "prompt" --output-format json` | `opencode run --format json "prompt"` |
| PreToolUse support | Yes | Yes (BeforeTool) | **No** (only PostToolUse) |
| Background ingest | No (synchronous) | Yes (`&` in hook script) | N/A (plugin handles async) |
| Event types tested | 9 (incl. SubagentStart/Stop) | 6 (Gemini hook types) | **5** (no PreToolUse, no Subagent) |
| Install method | `npm install -g @anthropic-ai/claude-code` | `npm install -g @google/gemini-cli` | `curl -fsSL https://opencode.ai/install \| bash` |

## Test Count Estimate

| File | Test Count | Requires opencode CLI | Requires Daemon |
|------|-----------|----------------------|-----------------|
| smoke.bats | 6-8 | 2-3 tests (skip if missing) | Yes |
| hooks.bats | 6-8 | No | Yes |
| pipeline.bats | 5-6 | No | Yes |
| negative.bats | 6-7 | 1 test (timeout with CLI) | Partial (some tests intentionally no daemon) |
| **Total** | **23-29** | **3-4 optional** | **Most** |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| opencode-ai/opencode with `-p -q -f json` | anomalyco/opencode with `opencode run --format json` | Sept 2025 (opencode-ai archived) | Tests must use `run` subcommand, not `-p` flag |
| opencode-ai/opencode @opencode-ai/plugin | anomalyco/opencode plugin system | Ongoing | Plugin API may differ; existing memory-capture.ts imports `@opencode-ai/plugin` type |

**Deprecated/outdated:**
- **opencode-ai/opencode:** Archived September 2025, succeeded by "Crush". The existing plugin type import (`@opencode-ai/plugin`) may need updating for the anomalyco/opencode variant.
- **`-p -q -f json` flags:** These were from the archived opencode-ai/opencode. The REQUIREMENTS.md reference to these flags should be treated as needing adjustment.

## Open Questions

1. **Which OpenCode variant are we targeting?**
   - What we know: The plugin imports `@opencode-ai/plugin` (archived project). The current active project is anomalyco/opencode with different CLI flags.
   - What's unclear: Whether the existing plugin still works with the anomalyco/opencode or needs migration.
   - Recommendation: Tests should be written against the **current** anomalyco/opencode `run` subcommand syntax. The plugin compatibility is a separate concern (not Phase 32's scope). Smoke tests should detect which variant is installed and adapt.

2. **Should REQUIREMENTS.md flags be updated?**
   - What we know: OPEN-01 specifies `-p -q -f json` but the current OpenCode uses `opencode run --format json`.
   - What's unclear: Whether the project intends to target the old or new OpenCode.
   - Recommendation: Update smoke tests to use `opencode run --format json "prompt"` with timeout guards. Note the discrepancy but do not block on it -- the tests are valid with either variant since hook tests bypass the CLI entirely.

3. **MCP tool calls do not trigger plugin hooks**
   - What we know: GitHub issue #2319 on anomalyco/opencode confirms MCP tool calls do not trigger `tool.execute.before` or `tool.execute.after` hooks.
   - What's unclear: Whether this is fixed in the latest release.
   - Recommendation: This is a known limitation. Tests that verify tool capture should note that only built-in tool calls (not MCP) trigger the plugin. For testing purposes, this does not matter since we test direct CchEvent ingest.

## Sources

### Primary (HIGH confidence)
- `plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` -- Complete TypeScript plugin (read directly)
- `crates/memory-ingest/src/main.rs` -- CchEvent struct and field mapping (read directly)
- `tests/cli/claude-code/*.bats` -- All 4 test files as templates (read directly)
- `tests/cli/gemini/*.bats` -- All 4 Gemini test files as templates (read directly)
- `tests/cli/lib/common.bash` -- Shared helpers (read directly)
- `tests/cli/lib/cli_wrappers.bash` -- CLI wrappers (read directly)
- `.planning/phases/31-gemini-cli-tests/31-RESEARCH.md` -- Gemini research (read directly)

### Secondary (MEDIUM confidence)
- [OpenCode CLI docs](https://opencode.ai/docs/cli/) -- `run` subcommand, `--format json`, `serve` for headless
- [anomalyco/opencode GitHub](https://github.com/anomalyco/opencode) -- Active project, plugin system
- [anomalyco/opencode Issue #2923](https://github.com/anomalyco/opencode/issues/2923) -- `--format json` bug with `--command` (fixed)
- [anomalyco/opencode Issue #2319](https://github.com/sst/opencode/issues/2319) -- MCP tool calls don't trigger plugin hooks
- [SpillwaveSolutions/opencode_cli skill](https://github.com/SpillwaveSolutions/opencode_cli) -- `opencode run --model <provider/model> "prompt"` syntax

### Tertiary (LOW confidence)
- [opencode-ai/opencode GitHub](https://github.com/opencode-ai/opencode) -- Archived Sept 2025, `-p -q -f json` flags. Plugin type `@opencode-ai/plugin` originates here.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Reuses Phase 30 framework entirely, no new dependencies
- Architecture: HIGH - Direct mirror of Claude Code/Gemini test structure with OpenCode-specific fixtures
- Pitfalls: HIGH - Identified from reading actual plugin source code and researching CLI ecosystem
- OpenCode headless flags: MEDIUM - Current anomalyco/opencode docs verified but REQUIREMENTS.md references old flags
- Plugin compatibility: LOW - Existing plugin imports `@opencode-ai/plugin` from archived project; compatibility with anomalyco/opencode unverified

**Research date:** 2026-02-26
**Valid until:** 2026-03-26 (mostly stable -- framework is local; OpenCode ecosystem moving fast)
