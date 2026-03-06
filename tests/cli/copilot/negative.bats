#!/usr/bin/env bats
# Copilot CLI negative tests -- daemon down, malformed input, fail-open behavior (CPLT-04)
#
# Tests BOTH memory-ingest fail-open (returns {"continue":true}) and
# memory-capture.sh fail-open (exits 0, NO stdout) in all failure modes.
#
# CRITICAL DIFFERENCE from Gemini: Copilot hook produces NO stdout.
# Where Gemini tests assert [[ "$output" == '{}' ]], Copilot tests
# assert [ -z "$output" ] or just exit code 0.

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
  # Clean up any Copilot session temp files
  rm -f /tmp/copilot-memory-session-* 2>/dev/null || true
}

# --- Fixture and hook script paths ---

FIXTURE_DIR="${BATS_TEST_DIRNAME}/../fixtures/copilot"
HOOK_SCRIPT="${PROJECT_ROOT}/plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh"

# =========================================================================
# memory-ingest fail-open tests (assert {"continue":true})
# =========================================================================

# Test 1: memory-ingest with daemon down still returns continue:true
@test "negative: memory-ingest with daemon down still returns continue:true (copilot)" {
  # Do NOT start daemon. Use an unused port to ensure no daemon is listening.
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-c1\",\"agent\":\"copilot\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${unused_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  # Output must contain {"continue":true}
  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} but got: $output"
    false
  }
}

# Test 2: memory-ingest with malformed JSON returns continue:true
@test "negative: memory-ingest with malformed JSON returns continue:true (copilot)" {
  run bash -c "cat '${FIXTURE_DIR}/malformed.json' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for malformed JSON but got: $output"
    false
  }
}

# Test 3: memory-ingest with empty stdin returns continue:true
@test "negative: memory-ingest with empty stdin returns continue:true (copilot)" {
  run bash -c "echo '' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for empty stdin but got: $output"
    false
  }
}

# Test 4: memory-ingest with unknown event type returns continue:true
@test "negative: memory-ingest with unknown event type returns continue:true (copilot)" {
  run bash -c "echo '{\"hook_event_name\":\"UnknownEventType\",\"session_id\":\"neg-c4\",\"agent\":\"copilot\"}' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for unknown event type but got: $output"
    false
  }
}

# =========================================================================
# memory-capture.sh fail-open tests (assert exit 0, NO stdout)
# =========================================================================

# Test 5: memory-capture.sh with daemon down still exits 0
@test "negative: memory-capture.sh with daemon down still exits 0 (copilot)" {
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"cwd\":\"/tmp/neg-test\",\"timestamp\":1709640000000}' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' MEMORY_DAEMON_ADDR='http://127.0.0.1:${unused_port}' bash '${HOOK_SCRIPT}' sessionStart"
  [ "$status" -eq 0 ]

  # Copilot hook produces NO stdout (unlike Gemini's {})
  # We do NOT assert on output content -- just exit code
}

# Test 6: memory-capture.sh with malformed input still exits 0
@test "negative: memory-capture.sh with malformed input still exits 0 (copilot)" {
  run bash -c "echo '{not valid json' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' bash '${HOOK_SCRIPT}' sessionStart"
  [ "$status" -eq 0 ]

  # Copilot hook produces NO stdout on malformed input
}

# Test 7: memory-capture.sh with empty stdin still exits 0
@test "negative: memory-capture.sh with empty stdin still exits 0 (copilot)" {
  run bash -c "echo '' | MEMORY_INGEST_PATH='${MEMORY_INGEST_BIN}' bash '${HOOK_SCRIPT}' sessionStart"
  [ "$status" -eq 0 ]

  # Copilot hook produces NO stdout on empty input
}
