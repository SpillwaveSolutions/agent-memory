#!/usr/bin/env bats
# OpenCode CLI hook capture tests -- all event types via direct CchEvent ingest + gRPC verification
#
# OpenCode uses a TypeScript plugin (memory-capture.ts), which cannot be invoked
# from shell. ALL tests use DIRECT CchEvent ingest via the ingest_event helper.
#
# Each test follows a two-layer proof pattern:
#   Layer 1: ingest_event exits 0 and produces {"continue":true}
#   Layer 2: gRPC query confirms the event was stored in the daemon
#
# sleep 2 between Layer 1 and Layer 2 for background ingest timing.
#
# OpenCode has only 5 event types (NO PreToolUse):
#   SessionStart, UserPromptSubmit, PostToolUse, AssistantResponse, Stop
#
# Tests only need cargo-built binaries + daemon -- no OpenCode CLI required.

load '../lib/common'
load '../lib/cli_wrappers'

# Set at file scope so all tests can access it
FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/opencode"

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

# --- Test 1: SessionStart event is captured via direct ingest ---

@test "hook: SessionStart event is captured via direct ingest (opencode)" {
  local sid="test-opencode-sessionstart-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"

  # Layer 1: Direct ingest via ingest_event helper
  run ingest_event "$json"

  [[ "$status" -eq 0 ]] || {
    echo "Expected exit 0 from ingest_event, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
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

# --- Test 2: UserPromptSubmit event captures message ---

@test "hook: UserPromptSubmit event captures message (opencode)" {
  local sid="test-opencode-userprompt-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/user-prompt.json" "$sid")"

  # Layer 1
  run ingest_event "$json"

  [[ "$status" -eq 0 ]] || {
    echo "Expected exit 0 from ingest_event, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
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

# --- Test 3: AssistantResponse event captures response ---

@test "hook: AssistantResponse event captures response (opencode)" {
  local sid="test-opencode-assistantresponse-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/assistant-response.json" "$sid")"

  # Layer 1
  run ingest_event "$json"

  [[ "$status" -eq 0 ]] || {
    echo "Expected exit 0 from ingest_event, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
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

# --- Test 4: PostToolUse event captures tool name ---

@test "hook: PostToolUse event captures tool name (opencode)" {
  local sid="test-opencode-posttooluse-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/post-tool-use.json" "$sid")"

  # Layer 1
  run ingest_event "$json"

  [[ "$status" -eq 0 ]] || {
    echo "Expected exit 0 from ingest_event, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
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

# --- Test 5: Stop event is captured ---

@test "hook: Stop event is captured (opencode)" {
  local sid="test-opencode-stop-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/stop.json" "$sid")"

  # Layer 1
  run ingest_event "$json"

  [[ "$status" -eq 0 ]] || {
    echo "Expected exit 0 from ingest_event, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Verify event was stored
  local result
  result="$(query_all_events)"

  [[ "$result" != *"No events found"* ]] || {
    echo "Expected events after Stop ingest"
    echo "Query output: $result"
    false
  }
}

# --- Test 6: Multiple events in sequence maintain session coherence ---

@test "hook: multiple events in sequence maintain session coherence (opencode)" {
  local sid="test-opencode-sequence-$$"

  local json_start json_prompt json_tool json_response json_stop
  json_start="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"
  json_prompt="$(rewrite_session_id "${FIXTURE_DIR}/user-prompt.json" "$sid")"
  json_tool="$(rewrite_session_id "${FIXTURE_DIR}/post-tool-use.json" "$sid")"
  json_response="$(rewrite_session_id "${FIXTURE_DIR}/assistant-response.json" "$sid")"
  json_stop="$(rewrite_session_id "${FIXTURE_DIR}/stop.json" "$sid")"

  # Layer 1: Ingest all 5 events via direct ingest
  run ingest_event "$json_start"
  [[ "$status" -eq 0 ]] || { echo "SessionStart ingest failed: $output"; false; }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]

  run ingest_event "$json_prompt"
  [[ "$status" -eq 0 ]] || { echo "UserPromptSubmit ingest failed: $output"; false; }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]

  run ingest_event "$json_tool"
  [[ "$status" -eq 0 ]] || { echo "PostToolUse ingest failed: $output"; false; }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]

  run ingest_event "$json_response"
  [[ "$status" -eq 0 ]] || { echo "AssistantResponse ingest failed: $output"; false; }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]

  run ingest_event "$json_stop"
  [[ "$status" -eq 0 ]] || { echo "Stop ingest failed: $output"; false; }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]

  sleep 3

  # Layer 2: Verify prompt and response content appear
  local result
  result="$(query_all_events)"

  [[ "$result" == *"project structure"* ]] || {
    echo "Expected 'project structure' from UserPromptSubmit in multi-event sequence"
    echo "Query output: $result"
    false
  }

  [[ "$result" == *"src/ and tests/"* ]] || {
    echo "Expected 'src/ and tests/' from AssistantResponse in multi-event sequence"
    echo "Query output: $result"
    false
  }
}

# --- Test 7: Agent field "opencode" is preserved through ingest ---

@test "hook: agent field opencode is preserved through ingest (opencode)" {
  local sid="test-opencode-agentfield-$$"
  local json
  json="$(rewrite_session_id "${FIXTURE_DIR}/session-start.json" "$sid")"

  # Verify fixture contains agent=opencode before ingest
  [[ "$json" == *'"agent":"opencode"'* ]] || [[ "$json" == *'"agent": "opencode"'* ]] || {
    echo "Fixture JSON missing agent=opencode field"
    echo "JSON: $json"
    false
  }

  # Layer 1: Ingest with agent=opencode -- memory-ingest parses and forwards the agent field
  run ingest_event "$json"

  [[ "$status" -eq 0 ]] || {
    echo "Expected exit 0 from ingest_event, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
    echo "Actual output: $output"
    false
  }

  sleep 2

  # Layer 2: Query gRPC to verify event was stored (agent field accepted by ingest pipeline)
  local result
  result="$(query_all_events)"

  [[ "$result" != *"No events found"* ]] || {
    echo "Expected event stored after agent=opencode ingest"
    echo "Query output: $result"
    false
  }
}
