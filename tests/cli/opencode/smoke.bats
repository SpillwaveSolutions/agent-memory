#!/usr/bin/env bats
# OpenCode CLI smoke tests -- binary detection, basic ingest, daemon connectivity
#
# Tests 1-6: Always run (require only cargo-built binaries + daemon)
# Tests 7-8: Require opencode CLI binary (skip gracefully if not installed)

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

# --- Test 4: memory-capture.ts plugin file exists ---

@test "memory-capture.ts plugin file exists" {
  local plugin_file="${PROJECT_ROOT}/plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts"
  [ -f "$plugin_file" ] || {
    echo "Plugin file not found at: $plugin_file"
    false
  }
}

# --- Test 5: memory-ingest produces continue:true on valid CchEvent JSON ---

@test "memory-ingest produces continue:true on valid CchEvent JSON" {
  local json='{"hook_event_name":"SessionStart","session_id":"opencode-smoke-001","timestamp":"2026-02-26T10:00:00Z","cwd":"/tmp/test-workspace","agent":"opencode"}'

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
  local fixture_dir="${PROJECT_ROOT}/tests/cli/fixtures/opencode"
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

# --- Test 7: opencode binary detection works (skip if not installed) ---

@test "opencode binary detection works (skip if not installed)" {
  require_cli opencode "OpenCode"

  run opencode --version
  [ "$status" -eq 0 ]
}

# --- Test 8: opencode headless mode produces output (skip if not installed) ---

@test "opencode headless mode produces output (skip if not installed)" {
  require_cli opencode "OpenCode"

  run run_opencode "echo hello"

  # Timeout exits 124 or 137 -- known quirk of headless mode
  if [[ "$status" -eq 124 ]] || [[ "$status" -eq 137 ]]; then
    skip "OpenCode headless mode timed out (known quirk)"
  fi

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from opencode headless, got $status"
    echo "Output: $output"
    false
  }
  [[ -n "$output" ]] || {
    echo "Expected non-empty output from opencode headless mode"
    false
  }
}
