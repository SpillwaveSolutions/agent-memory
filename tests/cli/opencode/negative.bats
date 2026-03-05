#!/usr/bin/env bats
# OpenCode CLI negative tests -- daemon down, malformed input, fail-open behavior (OPEN-04).
#
# Tests memory-ingest fail-open ONLY (no hook script layer -- OpenCode uses TypeScript plugin).
# The assertion is always exit 0 with {"continue":true} for all failure modes.

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

# --- Fixture path ---

FIXTURE_DIR="${BATS_TEST_DIRNAME}/../fixtures/opencode"

# =========================================================================
# memory-ingest fail-open tests (assert {"continue":true})
# =========================================================================

# Test 1: memory-ingest with daemon down still returns continue:true
@test "negative: memory-ingest with daemon down still returns continue:true (opencode)" {
  # Do NOT start daemon. Use an unused port to ensure no daemon is listening.
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-o1\",\"agent\":\"opencode\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${unused_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  # Output must contain {"continue":true}
  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} but got: $output"
    false
  }
}

# Test 2: memory-ingest with malformed JSON returns continue:true
@test "negative: memory-ingest with malformed JSON returns continue:true (opencode)" {
  run bash -c "cat '${FIXTURE_DIR}/malformed.json' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for malformed JSON but got: $output"
    false
  }
}

# Test 3: memory-ingest with empty stdin returns continue:true
@test "negative: memory-ingest with empty stdin returns continue:true (opencode)" {
  run bash -c "echo '' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for empty stdin but got: $output"
    false
  }
}

# Test 4: memory-ingest with unknown event type returns continue:true
@test "negative: memory-ingest with unknown event type returns continue:true (opencode)" {
  run bash -c "echo '{\"hook_event_name\":\"UnknownEventType\",\"session_id\":\"neg-o4\",\"agent\":\"opencode\"}' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for unknown event type but got: $output"
    false
  }
}

# =========================================================================
# OpenCode-specific timeout/skip test
# =========================================================================

# Test 5: opencode headless timeout produces skip-friendly exit
@test "negative: opencode headless timeout produces skip-friendly exit (skip if not installed)" {
  require_cli opencode "OpenCode"

  if [[ -z "${TIMEOUT_CMD}" ]]; then
    skip "Skipping: no timeout command available (timeout/gtimeout)"
  fi

  # Run opencode with a very short timeout -- we expect it to time out
  # Exit codes 0 (completed fast), 124 (timeout), 137 (killed) are all acceptable
  run "${TIMEOUT_CMD}" 2s opencode run --format json "echo test" 2>/dev/null
  [[ "$status" -eq 0 || "$status" -eq 124 || "$status" -eq 137 ]] || {
    echo "Expected exit 0, 124, or 137 but got: $status"
    echo "Output: $output"
    false
  }
}
