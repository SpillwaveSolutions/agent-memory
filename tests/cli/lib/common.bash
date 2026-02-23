#!/usr/bin/env bash
# common.bash -- Shared test helper library for bats CLI E2E tests.
#
# Provides:
#   - Workspace isolation (temp dirs per test run)
#   - Daemon lifecycle (build, start, stop, health check)
#   - gRPC query helper
#   - Ingest helper
#
# Usage in .bats files:
#   load ../lib/common

# --- Project root detection ---

detect_project_root() {
    local dir
    dir="$(cd "$(dirname "${BATS_TEST_DIRNAME:-$0}")" && pwd)"
    # Walk up until we find Cargo.toml at workspace root
    while [[ "$dir" != "/" ]]; do
        if [[ -f "$dir/Cargo.toml" ]] && grep -q '\[workspace\]' "$dir/Cargo.toml" 2>/dev/null; then
            echo "$dir"
            return 0
        fi
        dir="$(dirname "$dir")"
    done
    # Fallback: try git rev-parse
    git rev-parse --show-toplevel 2>/dev/null || {
        echo "ERROR: Cannot detect project root" >&2
        return 1
    }
}

PROJECT_ROOT="$(detect_project_root)"
export PROJECT_ROOT

# --- Binary paths ---

MEMORY_DAEMON_BIN="${PROJECT_ROOT}/target/debug/memory-daemon"
MEMORY_INGEST_BIN="${PROJECT_ROOT}/target/debug/memory-ingest"
export MEMORY_DAEMON_BIN
export MEMORY_INGEST_BIN
# Alias used by ingest helper and hooks
export MEMORY_INGEST_PATH="${MEMORY_INGEST_BIN}"

# --- Configurable timeouts ---

DAEMON_HEALTH_TIMEOUT="${DAEMON_HEALTH_TIMEOUT:-10}"
DAEMON_POLL_INTERVAL="${DAEMON_POLL_INTERVAL:-0.5}"

# --- Workspace isolation ---

setup_workspace() {
    local run_id
    run_id="$(date +%s)-$$"
    TEST_WORKSPACE="${PROJECT_ROOT}/tests/cli/.runs/${run_id}"
    TEST_DB_PATH="${TEST_WORKSPACE}/db"
    TEST_LOG_FILE="${TEST_WORKSPACE}/logs/daemon.log"

    mkdir -p "${TEST_WORKSPACE}/db" "${TEST_WORKSPACE}/logs" "${TEST_WORKSPACE}/data"

    export TEST_WORKSPACE
    export TEST_DB_PATH
    export TEST_LOG_FILE
}

teardown_workspace() {
    # Stop daemon if still running
    if [[ -n "${DAEMON_PID:-}" ]]; then
        stop_daemon
    fi

    # Preserve workspace on failure for debugging
    if [[ "${BATS_TEST_COMPLETED:-}" == "1" ]] || [[ "${BATS_ERROR_STATUS:-0}" == "0" ]]; then
        if [[ -n "${TEST_WORKSPACE:-}" ]] && [[ -d "${TEST_WORKSPACE}" ]]; then
            rm -rf "${TEST_WORKSPACE}"
        fi
    else
        if [[ -n "${TEST_WORKSPACE:-}" ]]; then
            echo "# Test failed -- workspace preserved at: ${TEST_WORKSPACE}" >&3 2>/dev/null || true
        fi
    fi
}

# --- Daemon build ---

build_daemon_if_needed() {
    local daemon_bin="${MEMORY_DAEMON_BIN}"
    local needs_build=0

    if [[ ! -f "${daemon_bin}" ]]; then
        needs_build=1
    else
        # Rebuild if any source file is newer than the binary
        local src_dir="${PROJECT_ROOT}/crates/memory-daemon/src"
        if [[ -d "${src_dir}" ]]; then
            while IFS= read -r -d '' src_file; do
                if [[ "${src_file}" -nt "${daemon_bin}" ]]; then
                    needs_build=1
                    break
                fi
            done < <(find "${src_dir}" -name '*.rs' -print0 2>/dev/null)
        fi
    fi

    if [[ "${needs_build}" == "1" ]]; then
        echo "# Building memory-daemon..." >&3 2>/dev/null || true
        if ! (cd "${PROJECT_ROOT}" && cargo build -p memory-daemon -p memory-ingest 2>&1); then
            # Build failed -- if binaries exist from a previous build, use them
            if [[ -f "${daemon_bin}" ]]; then
                echo "# Build failed but existing binary found, continuing..." >&3 2>/dev/null || true
            else
                echo "ERROR: cargo build failed and no existing binary found" >&2
                return 1
            fi
        fi
    fi
}

# --- Port selection ---

pick_random_port() {
    # Pick a random port in the range 10000-60000
    local port
    port=$(( (RANDOM % 50000) + 10000 ))
    echo "${port}"
}

# --- Daemon lifecycle ---

start_daemon() {
    local port="${1:-}"

    if [[ -z "${port}" ]]; then
        port="$(pick_random_port)"
    fi

    if [[ ! -f "${MEMORY_DAEMON_BIN}" ]]; then
        echo "ERROR: memory-daemon binary not found at ${MEMORY_DAEMON_BIN}" >&2
        echo "ERROR: Run build_daemon_if_needed first" >&2
        return 1
    fi

    MEMORY_DAEMON_PORT="${port}"
    export MEMORY_DAEMON_PORT
    export MEMORY_DAEMON_ADDR="http://127.0.0.1:${MEMORY_DAEMON_PORT}"

    # Start daemon in foreground mode, in background
    "${MEMORY_DAEMON_BIN}" start \
        --foreground \
        --port "${MEMORY_DAEMON_PORT}" \
        --db-path "${TEST_DB_PATH}" \
        >"${TEST_LOG_FILE}" 2>&1 &
    DAEMON_PID=$!
    export DAEMON_PID

    # Wait for daemon to become healthy
    if ! wait_for_daemon; then
        echo "ERROR: Daemon failed to start within ${DAEMON_HEALTH_TIMEOUT}s" >&2
        echo "ERROR: PID=${DAEMON_PID}, port=${MEMORY_DAEMON_PORT}" >&2
        echo "ERROR: Log file contents:" >&2
        cat "${TEST_LOG_FILE}" >&2 2>/dev/null || true
        # Kill the process if it is still running
        kill "${DAEMON_PID}" 2>/dev/null || true
        wait "${DAEMON_PID}" 2>/dev/null || true
        unset DAEMON_PID
        return 1
    fi
}

stop_daemon() {
    if [[ -z "${DAEMON_PID:-}" ]]; then
        return 0
    fi

    # Send SIGTERM for graceful shutdown
    kill "${DAEMON_PID}" 2>/dev/null || true

    # Wait up to 5 seconds for process to exit
    local wait_count=0
    while kill -0 "${DAEMON_PID}" 2>/dev/null && [[ ${wait_count} -lt 10 ]]; do
        sleep 0.5
        wait_count=$((wait_count + 1))
    done

    # Force kill if still alive
    if kill -0 "${DAEMON_PID}" 2>/dev/null; then
        kill -9 "${DAEMON_PID}" 2>/dev/null || true
    fi

    wait "${DAEMON_PID}" 2>/dev/null || true
    unset DAEMON_PID
}

daemon_health_check() {
    # Try TCP connectivity check first (most reliable, no protocol dependency)
    if command -v nc &>/dev/null; then
        nc -z 127.0.0.1 "${MEMORY_DAEMON_PORT}" &>/dev/null
        return $?
    fi

    # Use grpcurl to list services (daemon exposes reflection, not grpc.health)
    if command -v grpcurl &>/dev/null; then
        grpcurl -plaintext "127.0.0.1:${MEMORY_DAEMON_PORT}" list &>/dev/null
        return $?
    fi

    # Bash /dev/tcp fallback
    if command -v bash &>/dev/null; then
        (echo >/dev/tcp/127.0.0.1/"${MEMORY_DAEMON_PORT}") &>/dev/null
        return $?
    fi

    # Last resort: check if the PID is still alive (weak check)
    kill -0 "${DAEMON_PID}" 2>/dev/null
}

wait_for_daemon() {
    local elapsed=0
    local timeout="${DAEMON_HEALTH_TIMEOUT}"

    while (( $(echo "${elapsed} < ${timeout}" | bc -l 2>/dev/null || echo 0) )); do
        # First check: is the process still alive?
        if ! kill -0 "${DAEMON_PID}" 2>/dev/null; then
            echo "ERROR: Daemon process (PID ${DAEMON_PID}) died during startup" >&2
            return 1
        fi

        if daemon_health_check; then
            return 0
        fi

        sleep "${DAEMON_POLL_INTERVAL}"
        elapsed="$(echo "${elapsed} + ${DAEMON_POLL_INTERVAL}" | bc -l 2>/dev/null || echo "${timeout}")"
    done

    return 1
}

# --- gRPC query helper ---

grpc_query() {
    # Usage: grpc_query <subcommand> [args...]
    # Example: grpc_query events --from 1000 --to 2000
    if [[ ! -f "${MEMORY_DAEMON_BIN}" ]]; then
        echo "ERROR: memory-daemon binary not found" >&2
        return 1
    fi

    "${MEMORY_DAEMON_BIN}" query \
        --endpoint "http://127.0.0.1:${MEMORY_DAEMON_PORT}" \
        "$@"
}

# --- Ingest helper ---

ingest_event() {
    # Usage: ingest_event '{"hook_event_name":"SessionStart","session_id":"test-1"}'
    # Pipes JSON to memory-ingest with correct daemon address
    local json="${1}"

    if [[ ! -f "${MEMORY_INGEST_BIN}" ]]; then
        echo "ERROR: memory-ingest binary not found at ${MEMORY_INGEST_BIN}" >&2
        return 1
    fi

    echo "${json}" | MEMORY_DAEMON_ADDR="http://127.0.0.1:${MEMORY_DAEMON_PORT}" "${MEMORY_INGEST_BIN}"
}

# --- Assertions ---

assert_daemon_running() {
    if [[ -z "${DAEMON_PID:-}" ]]; then
        echo "ERROR: DAEMON_PID is not set" >&2
        return 1
    fi
    if ! kill -0 "${DAEMON_PID}" 2>/dev/null; then
        echo "ERROR: Daemon process (PID ${DAEMON_PID}) is not running" >&2
        return 1
    fi
    return 0
}

assert_daemon_healthy() {
    if ! daemon_health_check; then
        echo "ERROR: Daemon health check failed on port ${MEMORY_DAEMON_PORT}" >&2
        return 1
    fi
    return 0
}
