#!/usr/bin/env bash
# .github/hooks/scripts/memory-capture.sh
# Captures Copilot CLI lifecycle events into agent-memory.
#
# Fail-open: NEVER blocks Copilot CLI, even if memory-ingest fails or is missing.
# This script always exits 0. No stdout output (Copilot ignores stdout for most events).
#
# CRITICAL DIFFERENCES FROM GEMINI ADAPTER:
#   1. No session_id in hook input -- synthesized via temp file keyed by CWD hash
#   2. No hook_event_name in hook input -- passed as $1 argument from hooks config
#   3. Timestamps are Unix milliseconds, not ISO 8601
#   4. toolArgs is a JSON string, not an object (double-parse required)
#   5. sessionStart may fire per-prompt (Bug #991) -- reuse existing session ID
#
# Supported events (via $1 argument):
#   sessionStart          -> SessionStart
#   sessionEnd            -> Stop
#   userPromptSubmitted   -> UserPromptSubmit  (captures user prompt)
#   preToolUse            -> PreToolUse        (captures tool name + input)
#   postToolUse           -> PostToolUse       (captures tool name + input)
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
# If anything fails inside main_logic, the trap ensures we exit 0.

fail_open() {
  exit 0
}

# Trap any error to guarantee fail-open
trap fail_open ERR EXIT

main_logic() {
  EVENT_TYPE="${1:-}"

  # Guard: check jq availability
  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  # Guard: need event type
  if [ -z "$EVENT_TYPE" ]; then
    return 0
  fi

  # Detect jq walk() capability (requires jq 1.6+)
  # Uses runtime check instead of version string parsing for reliability
  JQ_HAS_WALK=false
  if jq -n 'walk(.)' >/dev/null 2>&1; then
    JQ_HAS_WALK=true
  fi

  # Read all of stdin (Copilot sends JSON via stdin)
  INPUT=$(cat) || return 0

  # Guard: empty input
  if [ -z "$INPUT" ]; then
    return 0
  fi

  # Strip ANSI escape sequences from input
  # Handles CSI sequences (ESC[...X), OSC sequences (ESC]...BEL and ESC]...ST), and other escapes
  if command -v perl >/dev/null 2>&1; then
    INPUT=$(printf '%s' "$INPUT" | perl -pe 's/\e\[[0-9;]*[A-Za-z]//g; s/\e\][^\a\e]*(?:\a|\e\\)//g; s/\e[^[\]].//g') || return 0
  else
    # Fallback: sed handles CSI and basic OSC sequences
    INPUT=$(printf '%s' "$INPUT" | sed $'s/\x1b\[[0-9;]*[a-zA-Z]//g; s/\x1b\][^\x07]*\x07//g; s/\x1b\][^\x1b]*\x1b\\\\//g') || return 0
  fi

  # Guard: verify input is valid JSON
  if ! echo "$INPUT" | jq empty 2>/dev/null; then
    return 0
  fi

  # Extract base fields available in all hook events
  CWD=$(echo "$INPUT" | jq -r '.cwd // empty') || return 0
  TS_MS=$(echo "$INPUT" | jq -r '.timestamp // 0') || return 0

  # Convert timestamp from Unix milliseconds to ISO 8601
  # date -r is macOS, date -d is Linux
  if [ "$TS_MS" != "0" ] && [ -n "$TS_MS" ]; then
    TS_SEC=$((TS_MS / 1000))
    TIMESTAMP=$(date -r "$TS_SEC" -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || \
                date -d "@$TS_SEC" -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || \
                date -u +"%Y-%m-%dT%H:%M:%SZ")
  else
    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  fi

  # --- Session ID synthesis via temp file ---
  # Copilot does NOT provide session_id. We synthesize one keyed by CWD hash.
  # md5sum is Linux, md5 is macOS
  CWD_HASH=$(printf '%s' "${CWD:-unknown}" | md5sum 2>/dev/null | cut -d' ' -f1 || \
             printf '%s' "${CWD:-unknown}" | md5 2>/dev/null || \
             echo "default")
  SESSION_FILE="/tmp/copilot-memory-session-${CWD_HASH}"

  case "$EVENT_TYPE" in
    sessionStart)
      # Bug #991: sessionStart fires per-prompt in interactive mode.
      # Reuse existing session ID if session file already exists.
      if [ -f "$SESSION_FILE" ]; then
        SESSION_ID=$(cat "$SESSION_FILE")
      else
        SESSION_ID="copilot-$(uuidgen 2>/dev/null | tr '[:upper:]' '[:lower:]' || \
                    cat /proc/sys/kernel/random/uuid 2>/dev/null || \
                    echo "$(date +%s)-$$")"
        echo "$SESSION_ID" > "$SESSION_FILE"
      fi
      ;;
    sessionEnd)
      SESSION_ID=$(cat "$SESSION_FILE" 2>/dev/null || echo "copilot-unknown")
      # Only clean up session file on terminal reasons (user_exit or complete).
      # Preserve for resumed sessions (Bug #991 workaround).
      REASON=$(echo "$INPUT" | jq -r '.reason // empty')
      if [ "$REASON" = "user_exit" ] || [ "$REASON" = "complete" ]; then
        rm -f "$SESSION_FILE" 2>/dev/null
      fi
      ;;
    *)
      SESSION_ID=$(cat "$SESSION_FILE" 2>/dev/null || echo "copilot-unknown")
      ;;
  esac

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
  case "$EVENT_TYPE" in
    sessionStart)
      PAYLOAD=$(jq -n \
        --arg event "SessionStart" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
      ;;
    sessionEnd)
      PAYLOAD=$(jq -n \
        --arg event "Stop" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
      ;;
    userPromptSubmitted)
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
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, message: $msg, agent: $agent}')
      ;;
    preToolUse)
      TOOL_NAME=$(echo "$INPUT" | jq -r '.toolName // empty')
      # toolArgs is a JSON-encoded STRING, not an object -- double-parse required
      TOOL_ARGS_STR=$(echo "$INPUT" | jq -r '.toolArgs // "{}"')
      TOOL_INPUT=$(echo "$TOOL_ARGS_STR" | jq -c "$REDACT_FILTER" 2>/dev/null || echo '{}')
      PAYLOAD=$(jq -n \
        --arg event "PreToolUse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg tool "$TOOL_NAME" \
        --argjson tinput "$TOOL_INPUT" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
      ;;
    postToolUse)
      TOOL_NAME=$(echo "$INPUT" | jq -r '.toolName // empty')
      # toolArgs is a JSON-encoded STRING, not an object -- double-parse required
      TOOL_ARGS_STR=$(echo "$INPUT" | jq -r '.toolArgs // "{}"')
      TOOL_INPUT=$(echo "$TOOL_ARGS_STR" | jq -c "$REDACT_FILTER" 2>/dev/null || echo '{}')
      PAYLOAD=$(jq -n \
        --arg event "PostToolUse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg tool "$TOOL_NAME" \
        --argjson tinput "$TOOL_INPUT" \
        --arg agent "copilot" \
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
main_logic "$@"
