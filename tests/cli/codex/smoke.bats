#!/usr/bin/env bats
# Codex CLI smoke tests -- binary detection, basic ingest, daemon connectivity
#
# Tests 1-6: Always run (require only cargo-built binaries + daemon)
# Tests 7-8: Require codex CLI binary (skip gracefully if not installed)
#
# NOTE: Codex CLI does NOT support hooks (GitHub Discussion #2150).
# The adapter provides skills/commands only. Event capture requires
# direct CchEvent JSON ingestion via memory-ingest.

load '../lib/common'
load '../lib/cli_wrappers'

FIXTURE_DIR="${PROJECT_ROOT}/tests/cli/fixtures/codex"

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

# --- Test 4: memory-ingest produces continue:true on valid CchEvent JSON ---

@test "memory-ingest produces continue:true on valid CchEvent JSON" {
  local json='{"hook_event_name":"SessionStart","session_id":"codex-smoke-001","timestamp":"2026-03-05T10:00:00Z","cwd":"/tmp/test-workspace","agent":"codex"}'

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

# --- Test 5: memory-ingest produces continue:true on malformed JSON ---

@test "memory-ingest produces continue:true on malformed JSON" {
  local json
  json="$(cat "${FIXTURE_DIR}/malformed.json")"

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

# --- Test 6: Codex adapter skills exist and have valid SKILL.md format ---

@test "codex adapter skills exist and have valid SKILL.md format" {
  local skills_dir="${PROJECT_ROOT}/adapters/codex-cli/.codex/skills"

  # Verify skills directory exists
  [ -d "$skills_dir" ] || {
    echo "Skills directory not found at: $skills_dir"
    false
  }

  # Verify all 5 skills exist
  local expected_skills=("memory-query" "retrieval-policy" "topic-graph" "bm25-search" "vector-search")
  for skill in "${expected_skills[@]}"; do
    [ -f "${skills_dir}/${skill}/SKILL.md" ] || {
      echo "Missing SKILL.md for: ${skill}"
      false
    }
  done

  # Verify YAML frontmatter has name field in each skill
  for skill in "${expected_skills[@]}"; do
    grep -q "name: ${skill}" "${skills_dir}/${skill}/SKILL.md" || {
      echo "Missing 'name: ${skill}' in SKILL.md frontmatter"
      false
    }
  done

  # Verify no hooks directory exists (Codex has no hooks)
  [ ! -d "${PROJECT_ROOT}/adapters/codex-cli/.codex/hooks" ] || {
    echo "Hooks directory should NOT exist for Codex adapter"
    false
  }
}

# --- Test 7: codex binary detection works (skip if not installed) ---

@test "codex binary detection works (skip if not installed)" {
  require_cli codex "Codex CLI"

  run codex --version
  [ "$status" -eq 0 ]
}

# --- Test 8: codex headless mode produces output (skip if not installed) ---

@test "codex headless mode produces output (skip if not installed)" {
  require_cli codex "Codex CLI"

  run run_codex "echo hello"

  # Timeout exits 124 or 137 -- skip gracefully
  if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    skip "Codex headless mode timed out"
  fi

  [ "$status" -eq 0 ] || {
    echo "Expected exit 0 from codex headless mode, got $status"
    echo "Output: $output"
    false
  }
  [[ -n "$output" ]] || {
    echo "Expected non-empty output from codex headless mode"
    false
  }
}
