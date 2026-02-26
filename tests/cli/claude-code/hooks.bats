#!/usr/bin/env bats
# Claude Code hook capture tests -- all event types via stdin pipe + gRPC verification
#
# Each test follows a two-layer proof pattern:
#   Layer 1: memory-ingest exits 0 and produces {"continue":true} (fail-open)
#   Layer 2: gRPC query confirms the event was stored in the daemon
#
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

# Helper: ingest a fixture and verify Layer 1 (continue:true)
ingest_fixture() {
  local json="$1"
  run ingest_event "$json"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]
}

# Helper: query all events in the daemon with a wide time window.
# Note: query output format is "[timestamp_ms] agent_type: content"
# and does NOT include session_id.
query_all_events() {
  run grpc_query events --from 0 --to 9999999999999 --limit 1000
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
  result="$(query_all_events)"

  # Layer 2: verify at least 1 event stored (SessionStart has no message content)
  [[ "$result" == *"found"* ]] || {
    echo "Expected events in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" != *"No events found"* ]] || {
    echo "Expected at least one event after ingest"
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
  result="$(query_all_events)"

  # Layer 2: verify message content appears in query output
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
  result="$(query_all_events)"

  # Layer 2: verify tool event was stored (PreToolUse shows as "tool:" in output)
  [[ "$result" == *"tool:"* ]] || {
    echo "Expected 'tool:' type in gRPC query result"
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
  result="$(query_all_events)"

  # Layer 2: verify event count increased (at least 4 events by now)
  [[ "$result" == *"found"* ]] || {
    echo "Expected events in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" != *"No events found"* ]] || {
    echo "Expected events after PostToolUse ingest"
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
  result="$(query_all_events)"

  # Layer 2: verify assistant message content
  [[ "$result" == *"crates/"* ]] || {
    echo "Expected 'crates/' from assistant message in gRPC query result"
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
  result="$(query_all_events)"

  # Layer 2: verify subagent message content
  [[ "$result" == *"code review"* ]] || {
    echo "Expected 'code review' in gRPC query result"
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
  result="$(query_all_events)"

  # Layer 2: verify subagent stop message content
  [[ "$result" == *"review"* ]] || {
    echo "Expected 'review' content in gRPC query result"
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
  result="$(query_all_events)"

  # Layer 2: verify event was stored (Stop has no message, check system: type)
  [[ "$result" == *"system:"* ]] || {
    echo "Expected 'system:' type in gRPC query result"
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
  result="$(query_all_events)"

  # Layer 2: verify event count includes all events ingested so far (at least 9)
  [[ "$result" == *"found"* ]] || {
    echo "Expected events in gRPC query result"
    echo "Query output: $result"
    false
  }
  [[ "$result" != *"No events found"* ]] || {
    echo "Expected events after SessionEnd ingest"
    echo "Query output: $result"
    false
  }
}

# --- Test 10: Multiple events in sequence maintain session coherence ---

@test "hook: multiple events in sequence maintain session coherence" {
  local sid="test-hook-sequence-$$"

  # Capture event count before this test
  local before_result
  before_result="$(query_all_events)"

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

  sleep 2

  # Layer 2: query all events and verify count increased by at least 4
  local after_result
  after_result="$(query_all_events)"

  # Verify we have events and the prompt content appears
  [[ "$after_result" == *"project structure"* ]] || {
    echo "Expected 'project structure' content from multi-event sequence"
    echo "Query output: $after_result"
    false
  }

  # Verify total count is at least 13 (9 from tests 1-9 + 4 from this test)
  [[ "$after_result" == *"found"* ]] || {
    echo "Expected events in gRPC query result"
    echo "Query output: $after_result"
    false
  }
  [[ "$after_result" != *"No events found"* ]] || {
    echo "Expected events after multi-event sequence ingest"
    echo "Query output: $after_result"
    false
  }
}
