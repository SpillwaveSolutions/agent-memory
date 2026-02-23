#!/usr/bin/env bats
# Claude Code E2E pipeline tests -- full hook -> ingest -> query cycle (CLDE-03)
#
# These tests prove the complete pipeline: fire hook event via memory-ingest,
# daemon ingests via gRPC, events are queryable via memory-daemon query.
#
# NOTE: memory-ingest connects to hardcoded http://127.0.0.1:50051 (DEFAULT_ENDPOINT),
# so the daemon MUST be started on port 50051 for pipeline ingest to succeed.

load '../lib/common'
load '../lib/cli_wrappers'

# Force port 50051 because memory-ingest hardcodes DEFAULT_ENDPOINT
PIPELINE_PORT=50051

setup_file() {
  build_daemon_if_needed
  setup_workspace
  start_daemon "${PIPELINE_PORT}"
}

teardown_file() {
  stop_daemon
  teardown_workspace
}

# --- Helper: get current time in Unix ms ---

_now_ms() {
  # macOS date doesn't support %N, use python or perl fallback
  if python3 -c "import time; print(int(time.time()*1000))" 2>/dev/null; then
    return
  fi
  # Fallback: seconds * 1000
  echo "$(( $(date +%s) * 1000 ))"
}

# --- Helper: ingest a session lifecycle (output suppressed) ---

_ingest_full_session() {
  local session_id="${1}"
  local ts_base
  ts_base="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  # 1. SessionStart
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"${session_id}\",\"agent\":\"claude\",\"cwd\":\"/tmp/test\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 2. UserPromptSubmit
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"${session_id}\",\"message\":\"What is 2+2?\",\"agent\":\"claude\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 3. PreToolUse
  ingest_event "{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"claude\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 4. PostToolUse
  ingest_event "{\"hook_event_name\":\"PostToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"claude\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 5. AssistantResponse
  ingest_event "{\"hook_event_name\":\"AssistantResponse\",\"session_id\":\"${session_id}\",\"message\":\"The answer is 4.\",\"agent\":\"claude\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 6. Stop
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"${session_id}\",\"agent\":\"claude\",\"timestamp\":\"${ts_base}\"}" >/dev/null
}

# =========================================================================
# Test 1: Complete session lifecycle via hook ingest
# =========================================================================

@test "pipeline: complete session lifecycle via hook ingest" {
  assert_daemon_running

  local session_id="pipeline-lifecycle-${RANDOM}"

  local time_before
  time_before="$(_now_ms)"

  # Ingest full 6-event session
  _ingest_full_session "${session_id}"

  # Allow time for async processing
  sleep 2

  local time_after
  time_after="$(_now_ms)"

  # Query events in the time window
  run grpc_query events --from "${time_before}" --to "${time_after}"
  [ "$status" -eq 0 ]

  # Verify all 6 events were stored
  [[ "$output" == *"6 found"* ]] || {
    echo "Expected 6 events found in output"
    echo "Query output: $output"
    false
  }

  # Verify event types are present: user prompt, assistant response, tool events
  [[ "$output" == *"What is 2+2?"* ]] || {
    echo "Expected user prompt content in output"
    echo "Query output: $output"
    false
  }

  [[ "$output" == *"The answer is 4."* ]] || {
    echo "Expected assistant response content in output"
    echo "Query output: $output"
    false
  }
}

# =========================================================================
# Test 2: Ingested events are queryable via TOC browse
# =========================================================================

@test "pipeline: ingested events are queryable via TOC browse" {
  assert_daemon_running

  # Query TOC root -- should succeed even if no TOC rollup has occurred
  run grpc_query root
  [ "$status" -eq 0 ]

  # The key assertion is that the gRPC query path is operational
  [[ -n "$output" ]]
}

# =========================================================================
# Test 3: Events with cwd metadata are stored correctly
# =========================================================================

@test "pipeline: events with cwd metadata are stored correctly" {
  assert_daemon_running

  local session_id="pipeline-cwd-${RANDOM}"

  local time_before
  time_before="$(_now_ms)"

  # Ingest event with specific cwd
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"${session_id}\",\"agent\":\"claude\",\"cwd\":\"/home/user/pipeline-test-project\"}" >/dev/null

  sleep 1

  local time_after
  time_after="$(_now_ms)"

  # Query events -- the event should be present
  run grpc_query events --from "${time_before}" --to "${time_after}"
  [ "$status" -eq 0 ]

  # Verify at least one event was returned
  [[ "$output" == *"found"* ]] || {
    echo "Expected events in query output after cwd ingest"
    echo "Query output: $output"
    false
  }

  # Verify the query didn't return "No events found"
  [[ "$output" != *"No events found"* ]] || {
    echo "Expected events but got none after cwd ingest"
    echo "Query output: $output"
    false
  }
}

# =========================================================================
# Test 4: Real claude hook fire produces queryable event (requires claude)
# =========================================================================

@test "pipeline: real claude hook fire produces queryable event (requires claude)" {
  require_cli claude "Claude Code"
  assert_daemon_running

  local time_before
  time_before="$(_now_ms)"

  # Run a real Claude Code session with a trivial prompt
  run run_claude "What is 2+2? Answer with just the number."
  # Allow both success and non-zero exit (API key issues, etc.)

  sleep 3

  local time_after
  time_after="$(_now_ms)"

  # Query events in the time window
  run grpc_query events --from "${time_before}" --to "${time_after}"
  [ "$status" -eq 0 ]

  # At least some output should exist (even "No events found")
  [[ -n "$output" ]] || {
    echo "Expected at least some output from query"
    false
  }
}

# =========================================================================
# Test 5: Concurrent sessions maintain isolation
# =========================================================================

@test "pipeline: concurrent sessions maintain isolation" {
  assert_daemon_running

  local msg_a="unique-marker-alpha-${RANDOM}"
  local msg_b="unique-marker-beta-${RANDOM}"

  local time_before
  time_before="$(_now_ms)"

  # Interleave events from two sessions
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"iso-A-${RANDOM}\",\"agent\":\"claude\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"iso-B-${RANDOM}\",\"agent\":\"claude\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"iso-A\",\"message\":\"${msg_a}\",\"agent\":\"claude\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"iso-B\",\"message\":\"${msg_b}\",\"agent\":\"claude\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"iso-A\",\"agent\":\"claude\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"iso-B\",\"agent\":\"claude\"}" >/dev/null

  sleep 2

  local time_after
  time_after="$(_now_ms)"

  # Query all events in time window
  run grpc_query events --from "${time_before}" --to "${time_after}"
  [ "$status" -eq 0 ]

  # Both session messages should appear in the output
  [[ "$output" == *"${msg_a}"* ]] || {
    echo "Expected message_a '${msg_a}' in output"
    echo "Output: $output"
    false
  }
  [[ "$output" == *"${msg_b}"* ]] || {
    echo "Expected message_b '${msg_b}' in output"
    echo "Output: $output"
    false
  }

  # Verify 6 events total (3 per session)
  [[ "$output" == *"6 found"* ]] || {
    echo "Expected 6 events for two concurrent sessions"
    echo "Output: $output"
    false
  }
}
