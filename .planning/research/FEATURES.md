# Feature Landscape: Headless Multi-CLI E2E Testing Harness

**Domain:** Shell-based E2E integration testing for 5 AI coding CLI tools
**Researched:** 2026-02-22
**Overall Confidence:** HIGH

## Executive Summary

This document maps the feature landscape for building a shell-first E2E testing harness that spawns real CLI processes (Claude Code, Gemini CLI, OpenCode, Copilot CLI, Codex CLI) in headless mode. The harness validates hook/event capture, skill/command invocation, and state persistence across the full CLI-to-daemon pipeline. It complements the existing 29 cargo E2E tests (which test handlers directly via tonic::Request) by adding a process-level integration layer.

Codex CLI has NO hook/extension system, so hook-dependent test scenarios must be skipped for it.

---

## Table Stakes

Features the harness must have. Missing = harness is unreliable or unusable.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Isolated workspace per test file | Tests must not pollute each other; fresh temp dir with its own RocksDB, config, and plugin files | Medium | bats `setup_file`/`teardown_file` with `mktemp -d` |
| Process spawning with timeout | Real CLI binaries run headless with kill guard to prevent CI deadlock | Low | `timeout 120s` (gtimeout on macOS) wrapping every CLI invocation |
| Exit code assertion | Verify CLI exits 0 on success, non-zero on failure | Low | bats built-in `[ "$status" -eq 0 ]` |
| Stdout/stderr capture | Capture and assert on CLI output (JSON or text) | Low | bats `run` captures output and status automatically |
| Environment variable injection | Set `MEMORY_INGEST_PATH`, API keys, config paths per test | Low | bats `export` in setup functions |
| CLI availability detection | Skip tests gracefully when a CLI binary is not installed | Low | `command -v claude` check in `setup_file`, then `skip` |
| Daemon lifecycle management | Start memory-daemon before tests, stop after; health check before test runs | Medium | Port 0 for OS-assigned port, health check loop |
| Hook script unit tests | Test each adapter's memory-capture.sh in isolation with mock stdin | Low | Existing `MEMORY_INGEST_DRY_RUN=1` flag supports this |
| Fixture data management | Predefined JSON payloads for hook events, prompts, expected outputs | Low | `tests/e2e-cli/fixtures/` directory |
| JUnit XML reporting | CI-parseable test results | Low | bats `--report-formatter junit --output ./results/` |
| Cleanup on failure preservation | Preserve workspace on failure for debugging, clean on success | Medium | Conditional cleanup in `teardown_file` based on `BATS_SUITE_TEST_FAILED` |

---

## Differentiators

Features that make this harness excellent rather than merely functional.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| CLI x Scenario test matrix | Same logical test across all 5 CLIs with skip rules (e.g., skip hooks for Codex) | Medium | GitHub Actions matrix: `cli: [claude, gemini, opencode, copilot, codex]` with `fail-fast: false` |
| End-to-end hook pipeline verification | Spawn CLI headless -> hook fires -> memory-ingest receives -> verify in RocksDB via gRPC query | High | The "real" E2E test; proves entire pipeline works |
| Structured JSON output parsing | Parse JSON from `--output-format json` for precise field assertions | Medium | `jq` for extraction, bats-assert for validation |
| CI artifact retention on failure | Failed test workspace preserved as tar.gz and uploaded as GitHub Actions artifact | Medium | `actions/upload-artifact@v4` with `if: always()` |
| Shared common.bash helper library | Reusable functions for workspace creation, daemon lifecycle, CLI wrappers, skip logic | Medium | Single source of truth for all test patterns |
| Per-CLI wrapper functions | `run_claude`, `run_gemini`, etc. that encapsulate each CLI's headless flags | Low | Standardizes invocation across test files |

---

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Mock CLI simulators | Simulating CLI behavior defeats E2E purpose; tests the mock, not the CLI | Spawn real CLI binaries; skip when unavailable |
| Interactive/TUI test mode | Driving interactive sessions with keystroke simulation is extremely brittle | Only test headless/non-interactive modes |
| Full LLM round-trip in every test | Real LLM calls are slow, expensive, and non-deterministic | Test mechanical pipeline (spawn -> hook -> ingest -> verify); LLM quality is out of scope |
| API key management in tests | Hardcoding or committing keys is a security risk | Use CI secrets; skip tests locally when keys absent |
| Custom test framework | Building a bespoke runner adds maintenance and breaks tool integration | Use bats-core with standard helpers |
| Cross-platform shell abstraction | Windows cmd/PowerShell compatibility adds massive complexity | Target macOS/Linux only; Windows is out of scope for v2.4 |
| Shared state between tests | Shared daemons or databases create ordering dependencies and flakiness | Each test file gets its own workspace and daemon |
| Performance benchmarking | Response time measurement belongs in perf_bench, not E2E correctness tests | Keep existing perf_bench harness separate |
| Testing CLI authentication | Auth (OAuth, API keys) is the CLI vendor's responsibility | Assume pre-authenticated; skip with message if auth fails |

---

## Feature Dependencies

```
CLI Detection (command -v)
    |
    +--- Workspace Isolation (mktemp -d)
    |       |
    |       +--- Daemon Lifecycle (start/stop/health)
    |       |       |
    |       |       +--- Hook Script Unit Tests
    |       |       |
    |       |       +--- CLI Headless Invocation
    |       |               |
    |       |               +--- E2E Pipeline Tests
    |       |
    |       +--- Fixture Data
    |
    +--- Per-CLI Wrapper Functions
    |
    +--- Common Helper Library (common.bash)
            |
            +--- JUnit Reporting (bats --report-formatter junit)
            |
            +--- CI Matrix (GitHub Actions)
            |
            +--- Artifact Retention (tar.gz on failure)
```

**Critical path (build order):**
1. common.bash with workspace + daemon lifecycle helpers
2. CLI detection + skip logic
3. Per-CLI wrapper functions (run_claude, run_gemini, etc.)
4. Hook script unit tests (mock stdin, verify output)
5. Smoke tests (basic headless invocation per CLI)
6. E2E pipeline tests (hook -> ingest -> query -> verify)
7. JUnit reporting + CI matrix + artifact retention

---

## Test Scenario Categories

### Category 1: Smoke Tests (All 5 CLIs)

Verify basic headless invocation works.

| Scenario | What It Tests | Assertion | Skip Rule |
|----------|--------------|-----------|-----------|
| CLI binary exists | Binary is installed and on PATH | `command -v` succeeds | Skip file if not found |
| Headless invocation | CLI runs with non-interactive flags and exits | Exit code 0, some stdout produced | Skip if CLI unavailable |
| JSON output mode | CLI produces parseable JSON in headless mode | `jq empty` succeeds on stdout | Skip if CLI has no JSON output (Copilot, Codex) |
| Plugin recognition | CLI recognizes memory adapter commands/skills | Output references memory commands | Skip if CLI unavailable |

### Category 2: Hook Capture Tests (Skip Codex -- NO hooks)

Verify hook scripts fire and produce correct payloads.

| Scenario | What It Tests | Assertion | Skip Rule |
|----------|--------------|-----------|-----------|
| SessionStart payload | Hook produces valid SessionStart JSON | JSON has event, session_id, timestamp, agent fields | Skip Codex |
| UserPromptSubmit payload | User message captured via hook | Payload contains message field | Skip Codex |
| PostToolUse payload | Tool use event has tool_name and tool_input | JSON has tool_name field | Skip Codex |
| Stop/SessionEnd payload | Session end produces Stop event | Correct event type | Skip Codex |
| Fail-open on missing binary | Hook exits 0 when memory-ingest not on PATH | Exit code 0, safe output | Skip Codex |
| Redaction filter | Sensitive fields (api_key, token) stripped | Payload lacks redacted keys | Skip Codex |
| ANSI stripping | Input with escape sequences produces clean JSON | Valid JSON output | Skip Codex |

### Category 3: E2E Pipeline Tests (Skip Codex for hook-dependent)

Full pipeline: spawn CLI -> hook fires -> daemon ingests -> query verifies.

| Scenario | What It Tests | Assertion | Skip Rule |
|----------|--------------|-----------|-----------|
| Hook ingest -> daemon storage | Event via hook appears in gRPC query | Query returns ingested event | Skip Codex |
| Agent tag propagation | Hook sets correct agent field per CLI | Event has correct agent tag | Skip Codex |
| Command invocation via CLI | Memory commands work through CLI | Valid response from command | Skip if CLI unavailable |

### Category 4: Negative Tests (All 5 CLIs)

Graceful error handling.

| Scenario | What It Tests | Assertion | Skip Rule |
|----------|--------------|-----------|-----------|
| Daemon not running | CLI/hook handles missing daemon | Exit 0 (fail-open), error logged | Skip Codex for hook tests |
| Malformed stdin to hook | Hook receives invalid JSON | Exit 0, no crash | Skip Codex |
| Timeout enforcement | CLI with hung prompt is killed | Process terminated by timeout | Skip if CLI unavailable |

---

## CLI-Specific Skip Matrix

| Scenario Category | Claude Code | Gemini CLI | OpenCode | Copilot CLI | Codex CLI |
|-------------------|:-----------:|:----------:|:--------:|:-----------:|:---------:|
| Smoke Tests | RUN | RUN | RUN | RUN | RUN |
| Hook Capture | RUN | RUN | RUN | RUN | **SKIP** |
| E2E Pipeline (hooks) | RUN | RUN | RUN | RUN | **SKIP** |
| E2E Pipeline (commands) | RUN | RUN | RUN | RUN | RUN |
| Negative Tests | RUN | RUN | RUN | RUN | PARTIAL |

---

## MVP Recommendation

### Phase 1 (Claude Code -- framework phase):

Build in this order:
1. **common.bash** -- workspace isolation, daemon lifecycle, CLI detection, skip helpers
2. **Per-CLI wrappers** -- `run_claude` function encapsulating `-p --output-format json --allowedTools`
3. **Hook script unit tests** -- mock stdin -> verify JSON output (uses existing `MEMORY_INGEST_DRY_RUN`)
4. **Smoke tests** -- basic headless invocation
5. **E2E pipeline test** -- hook capture -> daemon query verification
6. **CI integration** -- JUnit reporting, artifact retention, matrix job

### Defer to subsequent CLI phases:
- CLI-specific quirk workarounds (Copilot session synthesis, OpenCode headless bugs)
- Cross-CLI comparative tests

### Defer to post-v2.4:
- Windows support
- Performance regression tracking in shell tests
- GUI/dashboard for results

---

## Sources

- [Claude Code headless docs](https://code.claude.com/docs/en/headless) -- HIGH confidence
- [Gemini CLI headless docs](https://google-gemini.github.io/gemini-cli/docs/cli/headless.html) -- HIGH confidence
- [Codex CLI non-interactive docs](https://developers.openai.com/codex/noninteractive) -- HIGH confidence
- [Copilot CLI docs](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/use-copilot-cli) -- HIGH confidence
- [OpenCode CLI docs](https://opencode.ai/docs/cli/) -- MEDIUM confidence
- [bats-core docs](https://bats-core.readthedocs.io/en/latest/usage.html) -- HIGH confidence

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Table Stakes | HIGH | Standard CLI testing patterns; bats-core well-documented |
| Test Scenarios | HIGH | Derived from existing adapter hook scripts in this repo |
| Skip Matrix | HIGH | Codex no-hooks constraint documented; other CLIs have verified hooks |
| Differentiators | HIGH | JUnit reporting, CI matrix, artifact retention are proven patterns |
| Anti-Features | HIGH | Each backed by concrete reasoning and project constraints |
