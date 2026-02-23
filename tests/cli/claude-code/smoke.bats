#!/usr/bin/env bats
# Claude Code smoke tests -- binary detection, basic ingest, daemon connectivity
#
# Tests 1-6: Always run (require only cargo-built binaries + daemon)
# Tests 7-8: Require claude CLI binary (skip gracefully if not installed)

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

# --- Test 4: memory-ingest produces continue:true on valid JSON ---

@test "memory-ingest produces continue:true on valid JSON" {
  local fixture_dir="${PROJECT_ROOT}/tests/cli/fixtures/claude-code"
  local json
  json="$(cat "${fixture_dir}/session-start.json")"

  run ingest_event "$json"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]
}

# --- Test 5: memory-ingest produces continue:true on malformed JSON ---

@test "memory-ingest produces continue:true on malformed JSON" {
  local fixture_dir="${PROJECT_ROOT}/tests/cli/fixtures/claude-code"
  local json
  json="$(cat "${fixture_dir}/malformed.json")"

  run ingest_event "$json"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]
}

# --- Test 6: memory-ingest produces continue:true on empty stdin ---

@test "memory-ingest produces continue:true on empty stdin" {
  run ingest_event ""

  [ "$status" -eq 0 ]
  [[ "$output" == *'"continue":true'* ]] || [[ "$output" == *'"continue": true'* ]]
}

# --- Test 7: claude binary detection (skip if not installed) ---

@test "claude binary detection works (skip if not installed)" {
  require_cli claude "Claude Code"

  run claude --version
  [ "$status" -eq 0 ]
}

# --- Test 8: claude headless mode produces JSON output (requires claude) ---

@test "claude headless mode produces JSON output (requires claude)" {
  require_cli claude "Claude Code"

  # Skip if running inside a Claude Code session (nested sessions not allowed)
  if [[ -n "${CLAUDECODE:-}" ]]; then
    skip "Skipping: cannot run Claude Code inside an existing Claude Code session"
  fi

  run run_claude "echo hello"

  [ "$status" -eq 0 ]
  # Output should be valid JSON (starts with { or [)
  [[ "$output" == "{"* ]] || [[ "$output" == "["* ]]
}
