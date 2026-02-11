#!/usr/bin/env bash
# .gemini/hooks/memory-capture.sh
# Captures Gemini CLI lifecycle events into agent-memory.
#
# Fail-open: NEVER blocks Gemini CLI, even if memory-ingest fails or is missing.
# This script always outputs exactly {} to stdout and exits 0.
#
# Supported events:
#   SessionStart  -> SessionStart
#   SessionEnd    -> Stop
#   BeforeAgent   -> UserPromptSubmit  (captures user prompt)
#   AfterAgent    -> AssistantResponse (captures assistant response)
#   BeforeTool    -> PreToolUse        (captures tool name + input)
#   AfterTool     -> PostToolUse       (captures tool name + input)
#
# Requirements:
#   - jq (JSON processor) must be installed
#   - memory-ingest binary must be on PATH (or MEMORY_INGEST_PATH set)
#
# Environment variables:
#   MEMORY_INGEST_PATH    Override path to memory-ingest binary (default: memory-ingest)
#   MEMORY_INGEST_DRY_RUN If set to "1", skip sending to memory-ingest (for testing)

set -euo pipefail

# --- Fail-open wrapper ---
# Wrap all logic in a function so that set -e does not prevent fail-open behavior.
# If anything fails inside main_logic, the trap ensures we still output {} and exit 0.

fail_open() {
  echo '{}'
  exit 0
}

# Trap any error to guarantee fail-open
trap fail_open ERR EXIT

main_logic() {
  # Guard: check jq availability
  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  # Detect jq walk() capability (requires jq 1.6+)
  # Uses runtime check instead of version string parsing for reliability
  JQ_HAS_WALK=false
  if jq -n 'walk(.)' >/dev/null 2>&1; then
    JQ_HAS_WALK=true
  fi

  # Read all of stdin (Gemini sends JSON via stdin)
  INPUT=$(cat) || return 0

  # Guard: empty input
  if [ -z "$INPUT" ]; then
    return 0
  fi

  # Strip ANSI escape sequences from input
  # Gemini CLI can emit colored/streaming output that contaminates JSON
  # Handles CSI sequences (ESC[...X), OSC sequences (ESC]...ST), and other escapes
  if command -v perl >/dev/null 2>&1; then
    INPUT=$(printf '%s' "$INPUT" | perl -pe 's/\e\[[0-9;]*[A-Za-z]//g; s/\e\][^\a\e]*(?:\a|\e\\)//g; s/\e[^[\]].//g') || return 0
  else
    # Fallback: sed handles CSI only (most common case)
    INPUT=$(printf '%s' "$INPUT" | sed $'s/\x1b\[[0-9;]*[a-zA-Z]//g') || return 0
  fi

  # Guard: verify input is valid JSON
  if ! echo "$INPUT" | jq empty 2>/dev/null; then
    return 0
  fi

  # Extract base fields available in all hook events
  HOOK_EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // empty') || return 0
  SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty') || return 0
  TIMESTAMP=$(echo "$INPUT" | jq -r '.timestamp // empty') || return 0
  CWD=$(echo "$INPUT" | jq -r '.cwd // empty') || return 0

  # Skip if no event name (malformed input)
  if [ -z "$HOOK_EVENT" ]; then
    return 0
  fi

  # Redaction filter for sensitive fields in objects
  # Removes keys matching common secret patterns (case-insensitive)
  if [ "$JQ_HAS_WALK" = "true" ]; then
    REDACT_FILTER='walk(if type == "object" then with_entries(select(.key | test("api_key|token|secret|password|credential|authorization"; "i") | not)) else . end)'
  else
    # Fallback for jq < 1.6: delete known sensitive keys at top level and one level deep
    # Does not recurse into nested objects, but catches the common case
    REDACT_FILTER='del(.api_key, .token, .secret, .password, .credential, .authorization, .API_KEY, .TOKEN, .SECRET, .PASSWORD, .CREDENTIAL, .AUTHORIZATION) | if type == "object" then to_entries | map(if (.value | type) == "object" then .value |= del(.api_key, .token, .secret, .password, .credential, .authorization, .API_KEY, .TOKEN, .SECRET, .PASSWORD, .CREDENTIAL, .AUTHORIZATION) else . end) | from_entries else . end'
  fi

  # Build memory-ingest payload based on event type
  local PAYLOAD=""
  case "$HOOK_EVENT" in
    SessionStart)
      PAYLOAD=$(jq -n \
        --arg event "SessionStart" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg agent "gemini" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
      ;;
    SessionEnd)
      PAYLOAD=$(jq -n \
        --arg event "Stop" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg agent "gemini" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
      ;;
    BeforeAgent)
      MESSAGE=$(echo "$INPUT" | jq -r '.prompt // empty')
      # Redact sensitive content from message if it looks like JSON
      if echo "$MESSAGE" | jq empty 2>/dev/null; then
        MESSAGE=$(echo "$MESSAGE" | jq -c "$REDACT_FILTER" 2>/dev/null) || true
      fi
      PAYLOAD=$(jq -n \
        --arg event "UserPromptSubmit" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg msg "$MESSAGE" \
        --arg agent "gemini" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, message: $msg, agent: $agent}')
      ;;
    AfterAgent)
      MESSAGE=$(echo "$INPUT" | jq -r '.prompt_response // empty')
      # Redact sensitive content from message if it looks like JSON
      if echo "$MESSAGE" | jq empty 2>/dev/null; then
        MESSAGE=$(echo "$MESSAGE" | jq -c "$REDACT_FILTER" 2>/dev/null) || true
      fi
      PAYLOAD=$(jq -n \
        --arg event "AssistantResponse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg msg "$MESSAGE" \
        --arg agent "gemini" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, message: $msg, agent: $agent}')
      ;;
    BeforeTool)
      TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
      # Extract and redact tool_input
      TOOL_INPUT=$(echo "$INPUT" | jq -c ".tool_input // {} | $REDACT_FILTER" 2>/dev/null) || TOOL_INPUT='{}'
      PAYLOAD=$(jq -n \
        --arg event "PreToolUse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg tool "$TOOL_NAME" \
        --argjson tinput "$TOOL_INPUT" \
        --arg agent "gemini" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
      ;;
    AfterTool)
      TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
      # Extract and redact tool_input
      TOOL_INPUT=$(echo "$INPUT" | jq -c ".tool_input // {} | $REDACT_FILTER" 2>/dev/null) || TOOL_INPUT='{}'
      PAYLOAD=$(jq -n \
        --arg event "PostToolUse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg tool "$TOOL_NAME" \
        --argjson tinput "$TOOL_INPUT" \
        --arg agent "gemini" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
      ;;
    *)
      # Unknown event type -- skip silently
      return 0
      ;;
  esac

  # Skip if payload construction failed
  if [ -z "$PAYLOAD" ]; then
    return 0
  fi

  # Determine memory-ingest binary path
  local INGEST_BIN="${MEMORY_INGEST_PATH:-memory-ingest}"

  # Dry-run mode for testing (skip actual ingest)
  if [ "${MEMORY_INGEST_DRY_RUN:-0}" = "1" ]; then
    return 0
  fi

  # Send to memory-ingest in background (fail-open, non-blocking)
  # Redirect both stdout and stderr to /dev/null to prevent stdout pollution
  echo "$PAYLOAD" | "$INGEST_BIN" >/dev/null 2>/dev/null &

  return 0
}

# Execute main logic (any failure is caught by trap)
main_logic

# Trap handles the output, but if main_logic succeeds normally,
# the EXIT trap will fire and output {}
