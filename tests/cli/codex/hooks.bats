#!/usr/bin/env bats
# Codex CLI hook capture tests -- ALL SKIPPED
#
# Codex CLI does NOT support lifecycle hooks (GitHub Discussion #2150).
# These tests exist as placeholders to document the gap and maintain
# structural parity with other CLI test suites (Claude Code, Gemini, Copilot).
#
# If/when Codex adds hook support, these tests should be implemented
# following the same two-layer proof pattern used by other adapters.

load '../lib/common'
load '../lib/cli_wrappers'

# --- Test 1: SessionStart event ---

@test "hook: SessionStart event captures session" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# --- Test 2: UserPromptSubmit event ---

@test "hook: UserPromptSubmit event captures prompt" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# --- Test 3: PreToolUse event ---

@test "hook: PreToolUse event captures tool name" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# --- Test 4: PostToolUse event ---

@test "hook: PostToolUse event captures tool result" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# --- Test 5: SessionEnd event ---

@test "hook: SessionEnd event maps to Stop" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

# --- Test 6: session ID synthesis ---

@test "hook: session ID synthesis is deterministic" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}
