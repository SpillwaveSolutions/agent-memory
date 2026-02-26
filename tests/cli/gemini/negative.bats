#!/usr/bin/env bats
# Gemini CLI negative tests -- daemon down, malformed input, fail-open behavior (GEMI-04)
#
# Tests BOTH memory-ingest fail-open (returns {"continue":true}) and
# memory-capture.sh fail-open (returns {}) in all failure modes.
# The assertion is always exit 0 with appropriate fail-open output.

load '../lib/common'
load '../lib/cli_wrappers'

# NOTE: Daemon is NOT started -- tests manage connectivity explicitly
setup_file() {
  build_daemon_if_needed
  setup_workspace
  # Daemon is NOT started here -- tests that need it start/stop explicitly
}

teardown_file() {
  # Stop daemon if any test started one
  stop_daemon 2>/dev/null || true
  teardown_workspace
}

# --- Fixture and hook script paths ---

FIXTURE_DIR="${BATS_TEST_DIRNAME}/../fixtures/gemini"
HOOK_SCRIPT="${PROJECT_ROOT}/plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh"

# =========================================================================
# memory-ingest fail-open tests (assert {"continue":true})
# =========================================================================

# Test 1: memory-ingest with daemon down still returns continue:true
@test "negative: memory-ingest with daemon down still returns continue:true (gemini)" {
  # Do NOT start daemon. Use an unused port to ensure no daemon is listening.
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-g1\",\"agent\":\"gemini\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${unused_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  # Output must contain {"continue":true}
  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} but got: $output"
    false
  }
}

# Test 2: memory-ingest with malformed JSON returns continue:true
@test "negative: memory-ingest with malformed JSON returns continue:true (gemini)" {
  run bash -c "cat '${FIXTURE_DIR}/malformed.json' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for malformed JSON but got: $output"
    false
  }
}

# Test 3: memory-ingest with empty stdin returns continue:true
@test "negative: memory-ingest with empty stdin returns continue:true (gemini)" {
  run bash -c "echo '' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for empty stdin but got: $output"
    false
  }
}

# Test 4: memory-ingest with unknown event type returns continue:true
@test "negative: memory-ingest with unknown event type returns continue:true (gemini)" {
  run bash -c "echo '{\"hook_event_name\":\"UnknownEventType\",\"session_id\":\"neg-g4\",\"agent\":\"gemini\"}' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for unknown event type but got: $output"
    false
  }
}

# =========================================================================
# memory-capture.sh fail-open tests (assert {} output)
# =========================================================================

# Test 5: memory-capture.sh with daemon down still returns {}
@test "negative: memory-capture.sh with daemon down still returns {} (gemini)" {
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-g5\"}' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${unused_port}' bash '${HOOK_SCRIPT}'"
  [ "$status" -eq 0 ]

  # Hook script must output exactly {}
  [[ "$output" == *'{}'* ]] || {
    echo "Expected {} from hook script with daemon down but got: $output"
    false
  }
}

# Test 6: memory-capture.sh with malformed input still returns {}
@test "negative: memory-capture.sh with malformed input still returns {} (gemini)" {
  run bash -c "echo '{not valid json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' bash '${HOOK_SCRIPT}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{}'* ]] || {
    echo "Expected {} from hook script with malformed input but got: $output"
    false
  }
}

# Test 7: memory-capture.sh with empty stdin still returns {}
@test "negative: memory-capture.sh with empty stdin still returns {} (gemini)" {
  run bash -c "echo '' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' bash '${HOOK_SCRIPT}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{}'* ]] || {
    echo "Expected {} from hook script with empty stdin but got: $output"
    false
  }
}
