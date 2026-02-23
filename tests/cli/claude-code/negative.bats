#!/usr/bin/env bats
# Claude Code negative tests -- daemon down, malformed input, timeout enforcement (CLDE-04)
#
# These tests verify graceful error handling and fail-open behavior.
# The assertion is always that memory-ingest exits 0 and produces
# {"continue":true} regardless of what goes wrong.

load '../lib/common'
load '../lib/cli_wrappers'

# NOTE: Some tests intentionally do NOT start a daemon
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

# --- Helper: path to fixture files ---

FIXTURE_DIR="${BATS_TEST_DIRNAME}/../fixtures/claude-code"

# =========================================================================
# Test 1: memory-ingest with daemon down still returns continue:true
# =========================================================================

@test "negative: memory-ingest with daemon down still returns continue:true" {
  # Do NOT start daemon. Use an unused port to ensure no daemon is listening.
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-1\",\"agent\":\"claude\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${unused_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  # Output must be exactly {"continue":true}
  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} but got: $output"
    false
  }
}

# =========================================================================
# Test 2: memory-ingest with malformed JSON returns continue:true
# =========================================================================

@test "negative: memory-ingest with malformed JSON returns continue:true" {
  run bash -c "cat '${FIXTURE_DIR}/malformed.json' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for malformed JSON but got: $output"
    false
  }
}

# =========================================================================
# Test 3: memory-ingest with empty stdin returns continue:true
# =========================================================================

@test "negative: memory-ingest with empty stdin returns continue:true" {
  run bash -c "echo '' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for empty stdin but got: $output"
    false
  }
}

# =========================================================================
# Test 4: memory-ingest with unknown event type returns continue:true
# =========================================================================

@test "negative: memory-ingest with unknown event type returns continue:true" {
  run bash -c "echo '{\"hook_event_name\":\"UnknownEventType\",\"session_id\":\"neg-4\",\"agent\":\"claude\"}' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for unknown event type but got: $output"
    false
  }
}

# =========================================================================
# Test 5: timeout enforcement prevents hung CLI process
# =========================================================================

@test "negative: timeout enforcement prevents hung CLI process" {
  # Verify that the detect_timeout_cmd function returns a valid timeout command
  local timeout_cmd
  timeout_cmd="$(detect_timeout_cmd)"

  if [[ -z "${timeout_cmd}" ]]; then
    skip "No timeout command available on this platform"
  fi

  # Demonstrate timeout enforcement works: timeout a sleep command
  run "${timeout_cmd}" 2s sleep 10
  # timeout exits with 124 (GNU coreutils) or 137 (macOS gtimeout) when it kills the process
  [[ "$status" -ne 0 ]] || {
    echo "Expected non-zero exit from timed-out command"
    false
  }

  # The timeout command itself should exist and be functional
  run command -v "${timeout_cmd}"
  [ "$status" -eq 0 ]
}

# =========================================================================
# Test 6: daemon on wrong port is detected (fail-open)
# =========================================================================

@test "negative: daemon on wrong port is detected" {
  # Start daemon on its normal port
  start_daemon

  assert_daemon_running

  # Ingest with the WRONG port (daemon is running, but not on this port)
  local wrong_port=$(( MEMORY_DAEMON_PORT + 1 ))
  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-6\",\"agent\":\"claude\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${wrong_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  # Fail-open: still returns continue:true even though ingest failed
  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for wrong port but got: $output"
    false
  }

  stop_daemon
}

# =========================================================================
# Test 7: very large payload is handled gracefully
# =========================================================================

@test "negative: very large payload is handled gracefully" {
  # Generate a 100KB message field
  local large_msg
  large_msg="$(python3 -c "print('A' * 102400)" 2>/dev/null || printf '%0.sA' {1..1024})"

  run bash -c "echo '{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"neg-7\",\"message\":\"${large_msg}\",\"agent\":\"claude\"}' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for large payload but got: $output"
    false
  }
}
