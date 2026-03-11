---
phase: 30-claude-code-cli-harness
verified: 2026-02-23T21:00:00Z
status: passed
score: 5/5 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 3/5
  gaps_closed:
    - "memory-ingest now reads MEMORY_DAEMON_ADDR env var (std::env::var check at main.rs:137)"
    - "hooks.bats Layer 2 assertions are now hard failures: [[ expr ]] || { echo msg; false; } pattern — no || true escape hatches remain"
    - "ROADMAP success criterion 5 now reads tests/cli/lib/common.bash (not test_helper/common.bash)"
  gaps_remaining: []
  regressions: []
---

# Phase 30: Claude Code CLI Harness Verification Report

**Phase Goal:** Developers can run isolated shell-based E2E tests for Claude Code that validate the full hook-to-query pipeline, with reusable framework infrastructure for all subsequent CLI phases
**Verified:** 2026-02-23T21:00:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (plans 30-05 and 30-06)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `bats tests/cli/claude-code/` executes tests in isolated temp workspaces each with its own daemon on an OS-assigned port | VERIFIED | setup_workspace() creates unique .runs/<timestamp-PID>/ dirs. Both hooks.bats and pipeline.bats call start_daemon() with no port argument — pick_random_port() is used. No hardcoded 50051 in either file. ingest_event() in common.bash passes MEMORY_DAEMON_ADDR=http://127.0.0.1:${MEMORY_DAEMON_PORT} to memory-ingest, which now reads it (main.rs:137). True per-workspace isolation is now achieved. |
| 2 | Tests that require `claude` binary skip gracefully with informative message when binary is not installed | VERIFIED | require_cli() in cli_wrappers.bash calls bats skip with message. smoke.bats tests 7-8, pipeline.bats test 4 all use require_cli. Consistent across all bats files. |
| 3 | Claude Code hook fires produce events visible via gRPC query in the same test workspace | VERIFIED | Gap 1 closed: memory-ingest reads MEMORY_DAEMON_ADDR env var (main.rs lines 137-141). Gap 2 closed: all 10 hooks.bats Layer 2 assertions use hard failure pattern [[ "$result" == *"$sid"* ]] || { echo ...; false; }. No || true escape hatches remain in test assertions. grpc_query uses --endpoint http://127.0.0.1:${MEMORY_DAEMON_PORT} correctly targeting the per-test daemon. |
| 4 | JUnit XML report is generated and CI matrix job uploads failure artifacts (logs, workspace tarballs) | VERIFIED | e2e-cli.yml has bats --report-formatter junit --output tests/cli/.runs. upload-artifact for report.xml runs always. upload-artifact for failure workspace runs with if: failure() condition. 5-CLI matrix [claude-code, gemini, opencode, copilot, codex]. |
| 5 | A `tests/cli/lib/common.bash` library exists that other CLI test phases can source (via `load ../lib/common`) for workspace setup, daemon lifecycle, and CLI wrappers | VERIFIED | Gap 3 closed: ROADMAP success criterion 5 now reads tests/cli/lib/common.bash (ROADMAP.md:104). Library at tests/cli/lib/common.bash: 290 lines, 13 functions. cli_wrappers.bash: 133 lines, 8 functions. Future phases use load ../lib/common pattern. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `tests/cli/lib/common.bash` | Shared test helper: workspace isolation, daemon lifecycle, gRPC query, ingest | VERIFIED | 290 lines, 13 functions. ingest_event() correctly passes MEMORY_DAEMON_ADDR env var (line 266). |
| `tests/cli/lib/cli_wrappers.bash` | CLI wrappers: detection, Claude Code headless, hook pipeline testing | VERIFIED | 133 lines, 8 functions. require_cli(), run_claude(), run_hook_stdin() all present. |
| `tests/cli/.gitignore` | Ignores .runs/ directory | VERIFIED | Contains .runs/ entry. |
| `tests/cli/fixtures/claude-code/*.json` | All 10 fixture files (9 event types + malformed) | VERIFIED | All 10 files present. 9 valid JSON, 1 intentionally malformed. |
| `.github/workflows/e2e-cli.yml` | CI workflow with 5-CLI matrix, bats, JUnit, artifacts | VERIFIED | Valid YAML. 5-CLI matrix. bats --report-formatter junit configured. Upload artifacts on failure. |
| `tests/cli/claude-code/smoke.bats` | 8 smoke tests | VERIFIED | 8 @test blocks. Tests 7-8 use require_cli. |
| `tests/cli/claude-code/hooks.bats` | 10 hook capture tests with hard Layer 2 gRPC assertions | VERIFIED | 10 @test blocks. All Layer 2 assertions use hard failure pattern. No || true in test assertion logic (only valid || true in teardown cleanup operations). |
| `tests/cli/claude-code/pipeline.bats` | 5 E2E pipeline tests | VERIFIED | 5 @test blocks. Uses start_daemon() with random port (no hardcoded 50051). Hard assertions throughout. |
| `tests/cli/claude-code/negative.bats` | 7 negative tests | VERIFIED | 7 @test blocks. Hard assertions on exit code and continue:true output. The only || true in file is stop_daemon 2>/dev/null || true in teardown_file — a cleanup guard, not an assertion escape hatch. |
| `crates/memory-ingest/src/main.rs` | Reads MEMORY_DAEMON_ADDR env var for gRPC connection | VERIFIED | Lines 137-141: if let Ok(addr) = std::env::var("MEMORY_DAEMON_ADDR") { MemoryClient::connect(&addr).await } else { MemoryClient::connect_default().await }. Fallback to default preserves backward compatibility. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tests/cli/lib/common.bash` (`ingest_event`) | `target/debug/memory-ingest` | `MEMORY_DAEMON_ADDR env var + binary invocation` | WIRED | common.bash:266 sets MEMORY_DAEMON_ADDR=http://127.0.0.1:${MEMORY_DAEMON_PORT}. memory-ingest main.rs:137 reads that env var and connects to the specified address. Full ingest-to-random-port routing is now functional. |
| `tests/cli/claude-code/hooks.bats` | daemon (gRPC query Layer 2) | `grpc_query events + hard assert content` | WIRED | All 10 tests use [[ "$result" == *"$sid"* ]] || { echo ...; false; } pattern. grpc_query targets --endpoint http://127.0.0.1:${MEMORY_DAEMON_PORT}. No || true escape hatches remain in assertions. |
| `tests/cli/claude-code/hooks.bats` | `tests/cli/lib/common.bash` | `load '../lib/common'` | WIRED | Line 11 loads common, line 12 loads cli_wrappers. |
| `tests/cli/claude-code/pipeline.bats` | random-port daemon | `start_daemon() via common.bash` | WIRED | setup_file() calls start_daemon() with no port argument. pick_random_port() selects the port. ingest_event() routes to that port via MEMORY_DAEMON_ADDR. No PIPELINE_PORT=50051 hardcode remains. |
| `.github/workflows/e2e-cli.yml` | `tests/cli/` | `bats --report-formatter junit tests/cli/${{ matrix.cli }}/` | WIRED | CI runs bats against correct directory with JUnit output. |

### Requirements Coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| HARN-01: Isolated temp workspace per test run | SATISFIED | setup_workspace creates unique .runs/<timestamp-PID>/ directories |
| HARN-02: Daemon lifecycle (start/stop/health) | SATISFIED | build_daemon_if_needed, start_daemon, stop_daemon, daemon_health_check all implemented |
| HARN-03: OS-assigned port per daemon | SATISFIED | pick_random_port() used in both hooks.bats and pipeline.bats. ingest_event() routes to correct port via env var. True isolation achieved. |
| HARN-04: Graceful skip for missing CLI binary | SATISFIED | require_cli() and has_cli() implemented, used consistently |
| HARN-05: JUnit XML reporting | SATISFIED | e2e-cli.yml uses --report-formatter junit with artifact upload |
| HARN-06: Failure artifact upload | SATISFIED | upload-artifact with if: failure() and 7-day retention |
| HARN-07: Reusable library for subsequent phases | SATISFIED | tests/cli/lib/common.bash and cli_wrappers.bash. ROADMAP criterion matches actual path. |
| CLDE-01: Headless claude invocation | SATISFIED | run_claude() wraps claude -p --output-format json with timeout |
| CLDE-02: All event types captured | SATISFIED | hooks.bats covers all 10 event types with hard Layer 2 gRPC verification |
| CLDE-03: Full hook-to-query pipeline | SATISFIED | pipeline.bats implements real gRPC verification on random-port daemon. No hardcoded port workaround needed. |
| CLDE-04: Negative tests (fail-open) | SATISFIED | negative.bats has 7 tests with hard assertions on fail-open behavior |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `tests/cli/claude-code/negative.bats` | 20 | `stop_daemon 2>/dev/null \|\| true` in teardown_file | INFO | This is correct cleanup behavior — teardown should not fail even if no daemon was started. Not an assertion escape hatch. |

No blocker or warning anti-patterns remain.

### Human Verification Required

#### 1. Smoke Tests Pass (Tests 1-6)

**Test:** Run `bats tests/cli/claude-code/smoke.bats` after `cargo build -p memory-daemon -p memory-ingest`
**Expected:** 6 tests pass (daemon binary exists, ingest binary exists, daemon healthy, valid/malformed/empty JSON returns continue:true). Tests 7-8 skip with "Skipping: Claude Code not installed"
**Why human:** Requires built Rust binaries. Verifies actual daemon startup, port binding, and health check polling.

#### 2. Hooks Tests Pass (with built binaries)

**Test:** Run `bats tests/cli/claude-code/hooks.bats` after `cargo build -p memory-daemon -p memory-ingest`
**Expected:** All 10 tests pass. Each test verifies its session_id appears in gRPC query results. Tests 2-5 also check content-specific strings (project structure, Read tool name).
**Why human:** Requires built binaries. Validates that the now-functional ingest-to-random-port pipeline actually stores events and returns them in gRPC queries.

#### 3. Pipeline Tests Pass (Random Port)

**Test:** Run `bats tests/cli/claude-code/pipeline.bats`
**Expected:** Tests 1, 2, 3, 5 pass with real gRPC assertions on random-port daemon. Test 4 skips if claude not installed. No port conflict expected (random port).
**Why human:** Validates the complete gRPC pipeline end-to-end on a randomly-assigned port.

#### 4. Negative Tests Pass

**Test:** Run `bats tests/cli/claude-code/negative.bats`
**Expected:** All 7 tests pass. Daemon-down test verifies fail-open without daemon running.
**Why human:** Tests runtime behavior of memory-ingest fail-open mode under error conditions.

## Re-verification: Gap Closure Summary

### Gap 1 Closed: memory-ingest reads MEMORY_DAEMON_ADDR

Plan 30-05 added the env var check at `crates/memory-ingest/src/main.rs` lines 137-141:

```rust
let client_result = if let Ok(addr) = std::env::var("MEMORY_DAEMON_ADDR") {
    MemoryClient::connect(&addr).await
} else {
    MemoryClient::connect_default().await
};
```

The `ingest_event()` helper in `common.bash` line 266 sets this env var:

```bash
echo "${json}" | MEMORY_DAEMON_ADDR="http://127.0.0.1:${MEMORY_DAEMON_PORT}" "${MEMORY_INGEST_BIN}"
```

These two connect: ingest now routes to whichever port the test daemon is running on.

### Gap 2 Closed: hooks.bats Layer 2 assertions are hard failures

Plan 30-06 replaced all 10 `|| true` escape hatches with:

```bash
[[ "$result" == *"$sid"* ]] || {
  echo "Expected session_id '$sid' in gRPC query result"
  echo "Query output: $result"
  false
}
```

Grep of hooks.bats for `|| true` returns no output. The only `|| true` in the entire test suite appears in `negative.bats` teardown cleanup — a correct usage.

### Gap 3 Closed: ROADMAP path corrected

Plan 30-06 updated ROADMAP.md. Success criterion 5 now reads:

> A `tests/cli/lib/common.bash` library exists that other CLI test phases can source (via `load ../lib/common`) for workspace setup, daemon lifecycle, and CLI wrappers

This matches the actual file path `tests/cli/lib/common.bash`.

### Regression Check: Previously Passing Items

Items that were VERIFIED in the initial pass were spot-checked:

- `tests/cli/lib/cli_wrappers.bash` — still present, 133 lines, 8 functions
- `.github/workflows/e2e-cli.yml` — still present, 5-CLI matrix, JUnit artifact upload intact
- `tests/cli/claude-code/negative.bats` — still 7 @test blocks, hard assertions on fail-open
- `tests/cli/claude-code/pipeline.bats` — 5 @test blocks, no hardcoded 50051, now uses random port

No regressions found.

---

_Verified: 2026-02-23T21:00:00Z_
_Verifier: Claude (gsd-verifier)_
