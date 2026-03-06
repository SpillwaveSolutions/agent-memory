#!/usr/bin/env bats
# Copilot CLI smoke tests -- binary detection, basic ingest, daemon connectivity
#
# Tests 1-6: Always run (require only cargo-built binaries + daemon)
# Tests 7-8: Require copilot CLI binary (skip gracefully if not installed)

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

# --- Test 1: memory-daemon binary exists ---

@test "memory-daemon binary exists and is executable" {
  [ -f "$MEMORY_DAEMON_BIN" ]
  [ -x "$MEMORY_DAEMON_BIN" ]
}

# --- Test 2: memory-ingest binary exists ---

@test "memory-ingest binary exists and is executable" {
  [ -f "$MEMORY_INGEST_PATH" ]
  [ -x "$MEMORY_INGEST_PATH" ]
}

# --- Test 3: daemon is running and healthy ---

@test "daemon is running and healthy" {
  assert_daemon_running
  daemon_health_check
}

# --- Test 4: memory-capture.sh exists and is executable ---

@test "memory-capture.sh exists and is executable" {
  local hook_script="${PROJECT_ROOT}/plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh"
  [ -f "$hook_script" ] || {
    echo "Hook script not found at: $hook_script"
    false
  }
  [ -x "$hook_script" ] || {
    echo "Hook script not executable: $hook_script"
    false
  }
}

# --- Test 5: memory-ingest produces continue:true on valid CchEvent JSON ---

@test "memory-ingest produces continue:true on valid CchEvent JSON" {
  local json='{"hook_event_name":"SessionStart","session_id":"copilot-smoke-001","timestamp":"2026-03-05T10:00:00Z","cwd":"/tmp/test-workspace","agent":"copilot"}'

  run ingest_event "$json"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from memory-ingest, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true in output"
    echo "Actual output: $output"
    false
  }
}

# --- Test 6: memory-ingest produces continue:true on malformed JSON ---

@test "memory-ingest produces continue:true on malformed JSON" {
  local fixture_dir="${PROJECT_ROOT}/tests/cli/fixtures/copilot"
  local json
  json="$(cat "${fixture_dir}/malformed.json")"

  run ingest_event "$json"

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from memory-ingest on malformed input, got $status"
    false
  }
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]] || {
    echo "Expected continue:true on malformed JSON (fail-open)"
    echo "Actual output: $output"
    false
  }
}

# --- Test 7: copilot binary detection works (skip if not installed) ---

@test "copilot binary detection works (skip if not installed)" {
  require_cli copilot "Copilot CLI"

  run copilot --version
  [ "$status" -eq 0 ]
}

# --- Test 8: copilot headless mode produces output (skip if not installed) ---

@test "copilot headless mode produces output (skip if not installed)" {
  require_cli copilot "Copilot CLI"

  run run_copilot "echo hello"

  # Timeout exits 124 or 137 -- skip gracefully
  if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    skip "Copilot headless mode timed out"
  fi

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from copilot headless mode, got $status"
    echo "Output: $output"
    false
  }
  [[ -n "$output" ]] || {
    echo "Expected non-empty output from copilot headless mode"
    false
  }
}
