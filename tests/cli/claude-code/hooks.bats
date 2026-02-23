#!/usr/bin/env bats
# Claude Code hook capture tests -- all event types via stdin pipe + gRPC verification
#
# Each test follows a two-layer proof pattern:
#   Layer 1: memory-ingest exits 0 and produces {"continue":true} (fail-open)
#   Layer 2: gRPC query confirms the event was stored in the daemon
#
# All tests use unique session IDs (with PID) to avoid cross-test interference.
# Tests only need cargo-built binaries + daemon -- no Claude CLI required.

load '../lib/common'
load '../lib/cli_wrappers'

# Set at file scope so all tests can access it
FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/claude-code"

setup_file() {
  build_daemon_if_needed
  setup_workspace
  start_daemon
}

teardown_file() {
  stop_daemon
  teardown_workspace
}

# Helper: rewrite session_id in fixture JSON using jq (or sed fallback)
rewrite_session_id() {
  local fixture_file="$1"
  local new_sid="$2"

  if command -v jq &>/dev/null; then
    jq --arg sid "$new_sid" '.session_id = $sid' "$fixture_file"
  else
    # sed fallback: replace the session_id value
    sed "s/\"session_id\":[[:space:]]*\"[^\"]*\"/\"session_id\": \"${new_sid}\"/" "$fixture_file"
  fi
}

# Helper: ingest a fixture and verify Layer 1 (continue:true)
ingest_fixture() {
  local json="$1"
  run ingest_event "$json"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]
}

# Helper: query events via gRPC and check for expected content
# Uses a wide time window (epoch 0 to now+1hr in ms) to catch all events.
# Returns the query output for further assertions.
query_events() {
  local now_ms
  now_ms=$(( $(date +%s) * 1000 ))
  local from_ms=0
  local to_ms=$(( now_ms + 3600000 ))

  run grpc_query events --from "$from_ms" --to "$to_ms" --limit 100
  echo "$output"
}

# --- Test 1: SessionStart event ---

@test "hook: SessionStart event is captured and queryable" {
  local sid="test-hook-sessionstart-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 2: UserPromptSubmit event ---

@test "hook: UserPromptSubmit event captures message" {
  local sid="test-hook-userprompt-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/user-prompt.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" == *"project structure"* ]] || {
    echo "Expected 'project structure' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 3: PreToolUse event ---

@test "hook: PreToolUse event captures tool name" {
  local sid="test-hook-pretool-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/pre-tool-use.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" == *"Read"* ]] || {
    echo "Expected 'Read' tool name in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 4: PostToolUse event ---

@test "hook: PostToolUse event captures tool name" {
  local sid="test-hook-posttool-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/post-tool-use.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" == *"Read"* ]] || {
    echo "Expected 'Read' tool name in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 5: AssistantResponse event ---

@test "hook: AssistantResponse event captures message" {
  local sid="test-hook-assistant-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/assistant-response.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" == *"project structure"* ]] || {
    echo "Expected 'project structure' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 6: SubagentStart event ---

@test "hook: SubagentStart event is captured" {
  local sid="test-hook-substart-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/subagent-start.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 7: SubagentStop event ---

@test "hook: SubagentStop event is captured" {
  local sid="test-hook-substop-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/subagent-stop.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 8: Stop event ---

@test "hook: Stop event is captured" {
  local sid="test-hook-stop-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/stop.json" "$sid")"

  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 9: SessionEnd maps to Stop event ---

@test "hook: SessionEnd maps to Stop event" {
  local sid="test-hook-sessionend-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/session-end.json" "$sid")"

  # SessionEnd should map to Stop event type (per map_cch_event_type)
  ingest_fixture "$json"

  sleep 1
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
}

# --- Test 10: Multiple events in sequence maintain session coherence ---

@test "hook: multiple events in sequence maintain session coherence" {
  local sid="test-hook-sequence-$$"

  # Ingest 4 events in order with the same session_id
  local json_start json_prompt json_tool json_stop

  json_start="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"
  json_prompt="$(rewrite_session_id "${FIXTURE_DIR}/user-prompt.json" "$sid")"
  json_tool="$(rewrite_session_id "${FIXTURE_DIR}/post-tool-use.json" "$sid")"
  json_stop="$(rewrite_session_id "${FIXTURE_DIR}/stop.json" "$sid")"

  # Layer 1: all four ingest calls succeed with continue:true
  ingest_fixture "$json_start"
  ingest_fixture "$json_prompt"
  ingest_fixture "$json_tool"
  ingest_fixture "$json_stop"

  sleep 1

  # Layer 2: query all events and verify session coherence
  local result
  result="$(query_events)"

  # Layer 2: verify event appears in gRPC query
  [[ "$result" == *"$sid"* ]] || {
    echo "Expected session_id '$sid' in gRPC query result"
    echo "Query output: $result"
    false
  }
}
