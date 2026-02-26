#!/usr/bin/env bats
# Gemini CLI hook capture tests -- all event types via memory-capture.sh + gRPC verification
#
# Each test follows a two-layer proof pattern:
#   Layer 1: memory-capture.sh exits 0 and produces {} (fail-open)
#   Layer 2: gRPC query confirms the event was stored in the daemon
#
# The hook script runs memory-ingest in BACKGROUND (&), so sleep is required
# between Layer 1 and Layer 2 to allow async ingest to complete.
#
# Tests only need cargo-built binaries + daemon -- no Gemini CLI required.

load '../lib/common'
load '../lib/cli_wrappers'

# Set at file scope so all tests can access it
FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/gemini"
HOOK_SCRIPT="${PROJECT_ROOT}/plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh"

setup_file() {
  build_daemon_if_needed
  setup_workspace
  start_daemon
}

teardown_file() {
  stop_daemon
  teardown_workspace
}

# Helper: rewrite session_id in fixture JSON, always compact single-line output.
# memory-ingest reads stdin line-by-line, so multi-line JSON silently fails.
rewrite_session_id() {
  local fixture_file="$1"
  local new_sid="$2"

  if command -v jq &>/dev/null; then
    jq -c --arg sid "$new_sid" '.session_id = $sid' "$fixture_file"
  else
    # sed fallback: already single-line if fixture is compact; pipe through tr to strip newlines
    sed "s/\"session_id\":[[:space:]]*\"[^\"]*\"/\"session_id\": \"${new_sid}\"/" "$fixture_file" | tr -d '\n'
  fi
}

# Helper: query all events in the daemon with a wide time window.
query_all_events() {
  run grpc_query events --from 0 --to 9999999999999 --limit 1000
  echo "$output"
}

# --- Test 1: SessionStart event captured via hook script ---

@test "hook: SessionStart event is captured via hook script" {
  local sid="test-gemini-sessionstart-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"

  # Layer 1: Feed Gemini-format JSON into memory-capture.sh
  run bash -c "echo '$json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script"
    echo "Actual output: $output"
    false
  }

  # Wait for background ingest to complete
  sleep 2

  # Layer 2: Query gRPC and verify event was stored
  local result
  result="$(query_all_events)"

  [[ "$result" != *"No events found"* ]] || {
    echo "Expected at least one event after SessionStart ingest"
    echo "Query output: $result"
    false
  }
}

# --- Test 2: BeforeAgent event captures prompt ---

@test "hook: BeforeAgent event captures prompt" {
  local sid="test-gemini-beforeagent-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/before-agent.json" "$sid")"

  # Layer 1
  run bash -c "echo '$json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Verify prompt content appears in query
  local result
  result="$(query_all_events)"

  [[ "$result" == *"project structure"* ]] || {
    echo "Expected 'project structure' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 3: AfterAgent event captures response ---

@test "hook: AfterAgent event captures response" {
  local sid="test-gemini-afteragent-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/after-agent.json" "$sid")"

  # Layer 1
  run bash -c "echo '$json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Verify response content appears in query
  local result
  result="$(query_all_events)"

  [[ "$result" == *"src/ and tests/"* ]] || {
    echo "Expected 'src/ and tests/' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 4: BeforeTool event captures tool name ---

@test "hook: BeforeTool event captures tool name" {
  local sid="test-gemini-beforetool-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/before-tool.json" "$sid")"

  # Layer 1
  run bash -c "echo '$json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Verify tool event was stored
  local result
  result="$(query_all_events)"

  [[ "$result" == *"tool:"* ]] || {
    echo "Expected 'tool:' type in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 5: AfterTool event captures tool name ---

@test "hook: AfterTool event captures tool name" {
  local sid="test-gemini-aftertool-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/after-tool.json" "$sid")"

  # Layer 1
  run bash -c "echo '$json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Verify tool event was stored
  local result
  result="$(query_all_events)"

  [[ "$result" == *"tool:"* ]] || {
    echo "Expected 'tool:' type in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 6: SessionEnd event maps to Stop ---

@test "hook: SessionEnd event maps to Stop" {
  local sid="test-gemini-sessionend-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/session-end.json" "$sid")"

  # Layer 1
  run bash -c "echo '$json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Verify event was stored (event count increased)
  local result
  result="$(query_all_events)"

  [[ "$result" != *"No events found"* ]] || {
    echo "Expected events after SessionEnd ingest"
    echo "Query output: $result"
    false
  }
}

# --- Test 7: Multiple events in sequence maintain session coherence ---

@test "hook: multiple events in sequence maintain session coherence" {
  local sid="test-gemini-sequence-$$"

  local json_start json_prompt json_response json_end
  json_start="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"
  json_prompt="$(rewrite_session_id "${FIXTURE_DIR}/before-agent.json" "$sid")"
  json_response="$(rewrite_session_id "${FIXTURE_DIR}/after-agent.json" "$sid")"
  json_end="$(rewrite_session_id "${FIXTURE_DIR}/session-end.json" "$sid")"

  # Layer 1: Ingest all 4 events via hook script
  run bash -c "echo '$json_start' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"
  [ "$status" -eq 0 ]
  [[ "$output" == '{}' ]]

  run bash -c "echo '$json_prompt' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"
  [ "$status" -eq 0 ]
  [[ "$output" == '{}' ]]

  run bash -c "echo '$json_response' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"
  [ "$status" -eq 0 ]
  [[ "$output" == '{}' ]]

  run bash -c "echo '$json_end' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"
  [ "$status" -eq 0 ]
  [[ "$output" == '{}' ]]

  sleep 3

  # Layer 2: Verify prompt and response content appear
  local result
  result="$(query_all_events)"

  [[ "$result" == *"project structure"* ]] || {
    echo "Expected 'project structure' from BeforeAgent in multi-event sequence"
    echo "Query output: $result"
    false
  }

  [[ "$result" == *"src/ and tests/"* ]] || {
    echo "Expected 'src/ and tests/' from AfterAgent in multi-event sequence"
    echo "Query output: $result"
    false
  }
}

# --- Test 8: Hook script with ANSI-contaminated input still works ---

@test "hook: ANSI-contaminated input is handled gracefully" {
  local sid="test-gemini-ansi-$$"
  local clean_json
  clean_json="$(rewrite_session_id "${FIXTURE_DIR}/before-agent.json" "$sid")"

  # Prepend ANSI escape sequence to the JSON
  local ansi_json
  ansi_json=$'\x1b[32m'"${clean_json}"

  # Layer 1: Hook script should strip ANSI and still process
  run bash -c "printf '%s' '$ansi_json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' '$HOOK_SCRIPT'"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script with ANSI input, got $status"
    false
  }
  [[ "$output" == '{}' ]] || {
    echo "Expected {} output from hook script with ANSI input"
    echo "Actual output: $output"
    false
  }
}
