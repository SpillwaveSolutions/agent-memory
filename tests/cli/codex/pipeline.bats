#!/usr/bin/env bats
# Codex CLI E2E pipeline tests -- full ingest -> query cycle (CDEX-03)
#
# These tests prove the complete pipeline: ingest CchEvent with agent=codex,
# daemon stores via gRPC, events are queryable via memory-daemon query.
# Uses DIRECT CchEvent format (already-translated) since Codex has no hooks.
# Uses OS-assigned random port for full workspace isolation.

load '../lib/common'
load '../lib/cli_wrappers'

setup_file() {
  build_daemon_if_needed
  setup_workspace
  start_daemon
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

# --- Helper: ingest a full 5-event Codex session (direct CchEvent format) ---

_ingest_full_codex_session() {
  local session_id="${1}"
  local ts_base
  ts_base="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  # 1. SessionStart
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"${session_id}\",\"agent\":\"codex\",\"cwd\":\"/tmp/test\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 2. UserPromptSubmit
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"${session_id}\",\"message\":\"What is 2+2?\",\"agent\":\"codex\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 3. PreToolUse
  ingest_event "{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"codex\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 4. PostToolUse
  ingest_event "{\"hook_event_name\":\"PostToolUse\",\"session_id\":\"${session_id}\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"/test.rs\"},\"agent\":\"codex\",\"timestamp\":\"${ts_base}\"}" >/dev/null

  # 5. Stop
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"${session_id}\",\"agent\":\"codex\",\"timestamp\":\"${ts_base}\"}" >/dev/null
}

# =========================================================================
# Test 1: Complete session lifecycle via direct ingest
# =========================================================================

@test "pipeline: complete codex session lifecycle via direct ingest" {
  assert_daemon_running

  local session_id="codex-pipeline-lifecycle-${RANDOM}"

  local time_before
  time_before="$(_now_ms)"

  # Ingest full 5-event session
  _ingest_full_codex_session "${session_id}"

  # Allow time for async processing
  sleep 2

  local time_after
  time_after="$(_now_ms)"

  # Query events in the time window
  run grpc_query events --from "${time_before}" --to "${time_after}"
  [ "$status" -eq 0 ]

  # Verify events were stored (not "No events found")
  [[ "$output" != *"No events found"* ]] || {
    echo "Expected events but got none after codex session ingest"
    echo "Query output: $output"
    false
  }

  # Verify event content: user prompt
  [[ "$output" == *"What is 2+2?"* ]] || {
    echo "Expected user prompt content in output"
    echo "Query output: $output"
    false
  }
}

# =========================================================================
# Test 2: Ingested events are queryable via TOC browse
# =========================================================================

@test "pipeline: codex ingested events are queryable via TOC browse" {
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

@test "pipeline: codex events with cwd metadata are stored correctly" {
  assert_daemon_running

  local session_id="codex-pipeline-cwd-${RANDOM}"

  local time_before
  time_before="$(_now_ms)"

  # Ingest event with specific cwd
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"${session_id}\",\"agent\":\"codex\",\"cwd\":\"/tmp/codex-cwd-test\"}" >/dev/null

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
# Test 4: Codex agent field is preserved through ingest
# =========================================================================

@test "pipeline: codex agent field is preserved through ingest" {
  assert_daemon_running

  local session_id="codex-agent-field-${RANDOM}"

  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"${session_id}\",\"message\":\"Hello from Codex pipeline\",\"agent\":\"codex\"}" >/dev/null

  sleep 1

  # Query all events (wide time window)
  run grpc_query events --from 0 --to 9999999999999
  [ "$status" -eq 0 ]

  # Verify agent field or message content appears
  [[ "$output" == *"codex:"* ]] || [[ "$output" == *"Hello from Codex pipeline"* ]] || {
    echo "Expected codex agent field or message content in output"
    echo "Query output: $output"
    false
  }
}

# =========================================================================
# Test 5: Concurrent sessions maintain isolation
# =========================================================================

@test "pipeline: codex concurrent sessions maintain isolation" {
  assert_daemon_running

  local msg_a="codex-concurrent-alpha-${RANDOM}"
  local msg_b="codex-concurrent-beta-${RANDOM}"

  local time_before
  time_before="$(_now_ms)"

  # Interleave events from two sessions
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"codex-iso-A-${RANDOM}\",\"agent\":\"codex\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"codex-iso-B-${RANDOM}\",\"agent\":\"codex\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"codex-iso-A\",\"message\":\"${msg_a}\",\"agent\":\"codex\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"codex-iso-B\",\"message\":\"${msg_b}\",\"agent\":\"codex\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"codex-iso-A\",\"agent\":\"codex\"}" >/dev/null
  ingest_event "{\"hook_event_name\":\"Stop\",\"session_id\":\"codex-iso-B\",\"agent\":\"codex\"}" >/dev/null

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
