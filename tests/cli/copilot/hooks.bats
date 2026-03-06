#!/usr/bin/env bats
# Copilot CLI hook capture tests -- all event types via memory-capture.sh + gRPC verification
#
# Each test follows a two-layer proof pattern:
#   Layer 1: memory-capture.sh exits 0 (Copilot hook produces NO stdout, unlike Gemini's {})
#   Layer 2: gRPC query confirms the event was stored in the daemon
#
# CRITICAL COPILOT DIFFERENCES:
#   - Event type passed as $1 argument (not in JSON body)
#   - Hook produces NO stdout output (assert only on exit code 0)
#   - Hook runs memory-ingest in background (&), so sleep 2 before gRPC query
#   - Session ID synthesized via CWD hash (not provided in JSON input)
#   - Each test uses unique CWD to avoid session file collisions
#
# Tests only need cargo-built binaries + daemon -- no Copilot CLI required.

load '../lib/common'
load '../lib/cli_wrappers'

# Set at file scope so all tests can access it
FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/copilot"
HOOK_SCRIPT="${PROJECT_ROOT}/plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh"

setup_file() {
  build_daemon_if_needed
  setup_workspace
  start_daemon
}

teardown_file() {
  stop_daemon
  teardown_workspace
  # Clean up any stale session files from tests
  rm -f /tmp/copilot-memory-session-* 2>/dev/null || true
}

# --- Helpers ---

compute_cwd_hash() {
  local cwd="$1"
  printf '%s' "$cwd" | md5sum 2>/dev/null | cut -d' ' -f1 || \
  printf '%s' "$cwd" | md5 2>/dev/null
}

query_all_events() {
  run grpc_query events --from 0 --to 9999999999999 --limit 1000
  echo "$output"
}

teardown() {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null || true
}

# --- Test 1: sessionStart event creates session file ---

@test "hook: sessionStart event creates session file" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  local json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script, got $status"
    echo "Output: $output"
    false
  }

  # Verify session file was created
  [ -f "/tmp/copilot-memory-session-${cwd_hash}" ] || {
    echo "Session file not found at /tmp/copilot-memory-session-${cwd_hash}"
    false
  }

  # Verify session ID has copilot- prefix
  local sid
  sid=$(cat "/tmp/copilot-memory-session-${cwd_hash}")
  [[ "$sid" == copilot-* ]] || {
    echo "Expected session ID with copilot- prefix, got: $sid"
    false
  }
}

# --- Test 2: sessionStart event is captured via gRPC ---

@test "hook: sessionStart event is captured via gRPC" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  local json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"

  [ "$status" -eq 0 ]

  # Wait for background ingest to complete
  sleep 2

  # Layer 2: Query gRPC and verify event was stored
  local result
  result="$(query_all_events)"

  [[ "$result" != *"No events found"* ]] || {
    echo "Expected at least one event after sessionStart ingest"
    echo "Query output: $result"
    false
  }
}

# --- Test 3: userPromptSubmitted event captures prompt ---

@test "hook: userPromptSubmitted event captures prompt" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  # First create a session
  local start_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$start_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Submit user prompt
  local json='{"cwd":"'"${test_cwd}"'","timestamp":1709640001000,"prompt":"Explain the project structure"}'
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' userPromptSubmitted"

  [ "$status" -eq 0 ]

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

# --- Test 4: preToolUse event captures tool name ---

@test "hook: preToolUse event captures tool name" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  # First create a session
  local start_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$start_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Send preToolUse event
  local json
  json=$(cat "${FIXTURE_DIR}/pre-tool-use.json" | jq -c --arg cwd "$test_cwd" '.cwd = $cwd')
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' preToolUse"

  [ "$status" -eq 0 ]

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

# --- Test 5: postToolUse event captures tool name ---

@test "hook: postToolUse event captures tool name" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  # First create a session
  local start_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$start_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Send postToolUse event
  local json
  json=$(cat "${FIXTURE_DIR}/post-tool-use.json" | jq -c --arg cwd "$test_cwd" '.cwd = $cwd')
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' postToolUse"

  [ "$status" -eq 0 ]

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

# --- Test 6: sessionEnd event maps to Stop ---

@test "hook: sessionEnd event maps to Stop" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  # First create a session
  local start_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$start_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Send sessionEnd event
  local json='{"cwd":"'"${test_cwd}"'","timestamp":1709640005000,"reason":"user_exit"}'
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionEnd"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from hook script for sessionEnd, got $status"
    echo "Output: $output"
    false
  }
}

# --- Test 7: session ID synthesis is deterministic ---

@test "hook: session ID synthesis is deterministic (same CWD = same hash)" {
  local test_cwd_a="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}-a"
  local test_cwd_b="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}-b"
  mkdir -p "$test_cwd_a" "$test_cwd_b"

  local hash_a hash_b
  hash_a=$(compute_cwd_hash "$test_cwd_a")
  hash_b=$(compute_cwd_hash "$test_cwd_b")
  rm -f "/tmp/copilot-memory-session-${hash_a}" "/tmp/copilot-memory-session-${hash_b}" 2>/dev/null

  # Create session for CWD A
  local json_a='{"cwd":"'"${test_cwd_a}"'","timestamp":1709640000000}'
  run bash -c "echo '$json_a' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Create session for CWD B
  local json_b='{"cwd":"'"${test_cwd_b}"'","timestamp":1709640000000}'
  run bash -c "echo '$json_b' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Verify different CWDs produce different session files
  local sid_a sid_b
  sid_a=$(cat "/tmp/copilot-memory-session-${hash_a}")
  sid_b=$(cat "/tmp/copilot-memory-session-${hash_b}")

  [[ "$sid_a" != "$sid_b" ]] || {
    echo "Expected different session IDs for different CWDs"
    echo "CWD A: $test_cwd_a -> $sid_a"
    echo "CWD B: $test_cwd_b -> $sid_b"
    false
  }

  # Cleanup extra session file
  rm -f "/tmp/copilot-memory-session-${hash_b}" 2>/dev/null

  # Verify same CWD reuses same hash (invoke again for CWD A)
  rm -f "/tmp/copilot-memory-session-${hash_a}" 2>/dev/null
  run bash -c "echo '$json_a' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Session file should be at the same hash path
  [ -f "/tmp/copilot-memory-session-${hash_a}" ] || {
    echo "Session file not found at expected hash path after second invocation"
    false
  }

  # Cleanup
  rm -f "/tmp/copilot-memory-session-${hash_a}" 2>/dev/null
}

# --- Test 8: Bug #991 -- second sessionStart reuses existing session ID ---

@test "hook: Bug #991 -- second sessionStart reuses existing session ID" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  local json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'

  # First sessionStart -- creates session
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  local sid_first
  sid_first=$(cat "/tmp/copilot-memory-session-${cwd_hash}")

  # Second sessionStart -- should reuse existing session ID (Bug #991)
  run bash -c "echo '$json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  local sid_second
  sid_second=$(cat "/tmp/copilot-memory-session-${cwd_hash}")

  [[ "$sid_first" == "$sid_second" ]] || {
    echo "Bug #991: Expected same session ID on second sessionStart"
    echo "First:  $sid_first"
    echo "Second: $sid_second"
    false
  }
}

# --- Test 9: session file cleaned up on terminal reason ---

@test "hook: session file cleaned up on terminal reason" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  # Create session
  local start_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$start_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Verify session file exists
  [ -f "/tmp/copilot-memory-session-${cwd_hash}" ] || {
    echo "Session file should exist after sessionStart"
    false
  }

  # End session with terminal reason (user_exit)
  local end_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640005000,"reason":"user_exit"}'
  run bash -c "echo '$end_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionEnd"
  [ "$status" -eq 0 ]

  # Session file should be removed on terminal reason
  [ ! -f "/tmp/copilot-memory-session-${cwd_hash}" ] || {
    echo "Session file should be removed after sessionEnd with reason=user_exit"
    false
  }
}

# --- Test 10: session file preserved on non-terminal reason ---

@test "hook: session file preserved on non-terminal reason" {
  local test_cwd="${TEST_WORKSPACE}/copilot-test-${BATS_TEST_NUMBER}"
  mkdir -p "$test_cwd"

  local cwd_hash
  cwd_hash=$(compute_cwd_hash "$test_cwd")
  rm -f "/tmp/copilot-memory-session-${cwd_hash}" 2>/dev/null

  # Create session
  local start_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640000000}'
  run bash -c "echo '$start_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionStart"
  [ "$status" -eq 0 ]

  # Verify session file exists
  [ -f "/tmp/copilot-memory-session-${cwd_hash}" ] || {
    echo "Session file should exist after sessionStart"
    false
  }

  # End session with non-terminal reason (keepalive)
  local end_json='{"cwd":"'"${test_cwd}"'","timestamp":1709640005000,"reason":"keepalive"}'
  run bash -c "echo '$end_json' | \
    MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' \
    MEMORY_DAEMON_ADDR='http://127.0.0.1:${MEMORY_DAEMON_PORT}' \
    '$HOOK_SCRIPT' sessionEnd"
  [ "$status" -eq 0 ]

  # Session file should still exist on non-terminal reason
  [ -f "/tmp/copilot-memory-session-${cwd_hash}" ] || {
    echo "Session file should be preserved after sessionEnd with reason=keepalive"
    false
  }
}
