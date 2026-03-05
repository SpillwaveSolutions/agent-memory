#!/usr/bin/env bats
# Codex CLI negative tests -- daemon down, malformed input, fail-open behavior (CDEX-04)
#
# Tests memory-ingest fail-open (returns {"continue":true}) in all failure modes.
# Since Codex has NO hook script, hook-dependent tests are SKIPPED with annotation.

load '../lib/common'
load '../lib/cli_wrappers'

# NOTE: Daemon is NOT started -- tests manage connectivity explicitly
setup_file() {
  build_daemon_if_needed
  setup_workspace
  # Daemon is NOT started here -- fail-open tests need no daemon
}

teardown_file() {
  # Stop daemon if any test started one
  stop_daemon 2>/dev/null || true
  teardown_workspace
}

# --- Fixture path ---

FIXTURE_DIR="${BATS_TEST_DIRNAME}/../fixtures/codex"

# =========================================================================
# memory-ingest fail-open tests (assert {"continue":true})
# =========================================================================

# Test 1: memory-ingest with daemon down still returns continue:true
@test "negative: memory-ingest with daemon down still returns continue:true (codex)" {
  # Do NOT start daemon. Use an unused port to ensure no daemon is listening.
  local unused_port=$(( (RANDOM % 10000) + 40000 ))

  run bash -c "echo '{\"hook_event_name\":\"SessionStart\",\"session_id\":\"neg-x1\",\"agent\":\"codex\"}' | MEMORY_DAEMON_ADDR=\"http://127.0.0.1:${unused_port}\" '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  # Output must contain {"continue":true}
  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} but got: $output"
    false
  }
}

# Test 2: memory-ingest with malformed JSON returns continue:true
@test "negative: memory-ingest with malformed JSON returns continue:true (codex)" {
  run bash -c "cat '${FIXTURE_DIR}/malformed.json' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for malformed JSON but got: $output"
    false
  }
}

# Test 3: memory-ingest with empty stdin returns continue:true
@test "negative: memory-ingest with empty stdin returns continue:true (codex)" {
  run bash -c "echo '' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for empty stdin but got: $output"
    false
  }
}

# Test 4: memory-ingest with unknown event type returns continue:true
@test "negative: memory-ingest with unknown event type returns continue:true (codex)" {
  run bash -c "echo '{\"hook_event_name\":\"UnknownEventType\",\"session_id\":\"neg-x4\",\"agent\":\"codex\"}' | '${MEMORY_INGEST_BIN}'"
  [ "$status" -eq 0 ]

  [[ "$output" == *'{"continue":true}'* ]] || {
    echo "Expected {\"continue\":true} for unknown event type but got: $output"
    false
  }
}

# =========================================================================
# Hook-script tests (SKIPPED -- Codex has no hooks)
# =========================================================================

# Test 5: Hook script daemon-down test (skipped)
@test "negative: hook script daemon-down test (SKIPPED - Codex has no hooks)" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# Test 6: Hook script malformed-input test (skipped)
@test "negative: hook script malformed-input test (SKIPPED - Codex has no hooks)" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# Test 7: Hook script empty-stdin test (skipped)
@test "negative: hook script empty-stdin test (SKIPPED - Codex has no hooks)" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}
