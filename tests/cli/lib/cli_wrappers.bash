#!/usr/bin/env bash
# cli_wrappers.bash -- CLI-specific wrapper functions for bats E2E tests.
#
# Provides:
#   - CLI availability detection (require_cli, has_cli)
#   - Claude Code wrappers (run_claude, run_claude_with_hooks)
#   - Dry-run hook testing (run_hook_stdin, run_hook_stdin_dry)
#   - Timeout command detection (macOS/Linux)
#
# Usage in .bats files:
#   load ../lib/common
#   load ../lib/cli_wrappers

# --- Timeout configuration ---

CLI_TIMEOUT="${CLI_TIMEOUT:-120}"
export CLI_TIMEOUT

detect_timeout_cmd() {
    # Returns the appropriate timeout command for the platform.
    # Linux: timeout (coreutils)
    # macOS: gtimeout (from coreutils via brew) or timeout if available
    if command -v timeout &>/dev/null; then
        echo "timeout"
    elif command -v gtimeout &>/dev/null; then
        echo "gtimeout"
    else
        # No timeout command available -- return empty to skip timeout wrapping
        echo ""
    fi
}

TIMEOUT_CMD="$(detect_timeout_cmd)"
export TIMEOUT_CMD

# --- CLI availability detection ---

has_cli() {
    # Usage: has_cli <binary_name>
    # Returns 0 if binary exists on PATH, 1 otherwise. Non-skipping.
    local binary_name="${1}"
    command -v "${binary_name}" &>/dev/null
}

require_cli() {
    # Usage: require_cli <binary_name> [<human_name>]
    # Skips the test with an informative message if binary is not found.
    local binary_name="${1}"
    local human_name="${2:-${binary_name}}"

    if ! has_cli "${binary_name}"; then
        skip "Skipping: ${human_name} not installed (${binary_name} not found on PATH)"
    fi
}

# --- Claude Code wrappers ---

run_claude() {
    # Usage: run_claude <prompt> [extra args...]
    # Wraps claude CLI in headless/print mode with timeout and JSON output.
    # Sets $output (stdout) and $TEST_STDERR (stderr file) per bats convention.
    local test_stderr="${TEST_WORKSPACE:-/tmp}/claude_stderr.log"
    export TEST_STDERR="${test_stderr}"

    local cmd=("claude" "-p" "$@" "--output-format" "json")

    if [[ -n "${TIMEOUT_CMD}" ]]; then
        "${TIMEOUT_CMD}" "${CLI_TIMEOUT}s" "${cmd[@]}" 2>"${test_stderr}"
    else
        "${cmd[@]}" 2>"${test_stderr}"
    fi
}

run_claude_with_hooks() {
    # Usage: run_claude_with_hooks <prompt> [extra args...]
    # Same as run_claude but ensures hook env vars point at the test workspace.
    export MEMORY_INGEST_PATH="${MEMORY_INGEST_BIN:-${PROJECT_ROOT}/target/debug/memory-ingest}"
    export MEMORY_DAEMON_ADDR="http://127.0.0.1:${MEMORY_DAEMON_PORT:-50051}"

    run_claude "$@"
}

# --- Hook / ingest pipeline testing (no Claude Code needed) ---

run_hook_stdin() {
    # Usage: echo '{"hook_event_name":"SessionStart","session_id":"s1"}' | run_hook_stdin
    # Pipes stdin to memory-ingest binary directly. Tests the hook-to-ingest pipeline
    # without requiring a Claude Code API key.
    local ingest_bin="${MEMORY_INGEST_BIN:-${PROJECT_ROOT}/target/debug/memory-ingest}"

    if [[ ! -f "${ingest_bin}" ]]; then
        echo "ERROR: memory-ingest binary not found at ${ingest_bin}" >&2
        return 1
    fi

    MEMORY_DAEMON_ADDR="http://127.0.0.1:${MEMORY_DAEMON_PORT:-50051}" "${ingest_bin}"
}

run_hook_stdin_dry() {
    # Usage: echo '{"hook_event_name":"SessionStart","session_id":"s1"}' | run_hook_stdin_dry
    # Same as run_hook_stdin but with MEMORY_INGEST_DRY_RUN=1 for fast unit-level checks.
    local ingest_bin="${MEMORY_INGEST_BIN:-${PROJECT_ROOT}/target/debug/memory-ingest}"

    if [[ ! -f "${ingest_bin}" ]]; then
        echo "ERROR: memory-ingest binary not found at ${ingest_bin}" >&2
        return 1
    fi

    MEMORY_INGEST_DRY_RUN=1 \
    MEMORY_DAEMON_ADDR="http://127.0.0.1:${MEMORY_DAEMON_PORT:-50051}" \
        "${ingest_bin}"
}

# --- Utility ---

wait_for_output_contains() {
    # Usage: wait_for_output_contains <file> <pattern> [timeout_seconds]
    # Polls a file until it contains the given pattern.
    local file="${1}"
    local pattern="${2}"
    local timeout="${3:-10}"
    local elapsed=0

    while (( $(echo "${elapsed} < ${timeout}" | bc -l 2>/dev/null || echo 0) )); do
        if grep -q "${pattern}" "${file}" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
        elapsed="$(echo "${elapsed} + 0.5" | bc -l 2>/dev/null || echo "${timeout}")"
    done
    return 1
}
