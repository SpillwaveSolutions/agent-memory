# Domain Pitfalls: Headless Multi-CLI E2E Testing

**Domain:** Headless Multi-CLI E2E Testing for Agent Memory System
**Researched:** 2026-02-22
**Overall Confidence:** HIGH (verified against official docs, existing codebase patterns, and community reports)

---

## Critical Pitfalls

Mistakes that cause rewrites, CI breakage, or abandoned test suites.

---

### Pitfall 1: Zombie CLI Processes in CI

**What goes wrong:** Spawned CLI processes (Claude Code, OpenCode, Gemini, Copilot, Codex) hang or become zombies when tests timeout, fail, or the harness crashes. In CI containers, no init process reaps orphaned children. Over time, process table fills up and CI runners become unusable.

**Why it happens:** Each E2E test spawns a real CLI process. If the test harness dies (timeout, OOM, signal), the child process is orphaned. The existing hook scripts already use background processes (`echo "$PAYLOAD" | "$INGEST_BIN" >/dev/null 2>/dev/null &` in both Gemini and Copilot adapters), creating grandchild processes the harness cannot track.

**Consequences:** CI runners accumulate zombie processes. Subsequent test runs fail with resource exhaustion. Port conflicts from lingering daemon processes. Flaky "works locally, fails in CI" syndrome.

**Prevention:**
1. Use process groups (`setsid` or `set -m`) so `kill -- -$PGID` kills the entire tree
2. Implement a `trap cleanup EXIT INT TERM` in every test wrapper that kills the process group
3. Add a per-test timeout with the `timeout` command (not just test framework timeout)
4. Run `pkill -f memory-daemon` in test teardown as a safety net
5. In CI, use `--init` flag on Docker containers (or `tini`/`dumb-init`) to reap zombies
6. Track all spawned PIDs in an array and kill them in reverse order during cleanup

**Detection:** CI job durations creeping up over time. "Address already in use" errors in later tests. `ps aux | grep memory` showing stale processes.

**Phase to address:** Framework phase (Claude Code first) -- bake process lifecycle management into the harness from day one.

**Severity:** CRITICAL

---

### Pitfall 2: CLI Authentication Failures in Headless Mode

**What goes wrong:** Every CLI in the matrix requires API authentication, and headless/non-interactive modes have different auth flows than interactive modes. Tests pass locally (tokens cached) but fail in CI (fresh environment, no browser for OAuth).

**Why it happens:**
- **Claude Code:** OAuth flow requires a browser. Headless mode (`-p` flag) needs `ANTHROPIC_API_KEY` env var or pre-configured OAuth token. Trust verification is disabled in `-p` mode (helpful for testing but a security concern per [issue #20253](https://github.com/anthropics/claude-code/issues/20253)).
- **Codex CLI:** Needs `OPENAI_API_KEY`. The `codex exec` command works headlessly but still requires valid credentials.
- **Gemini CLI:** Needs Google API credentials. Non-TTY detection triggers headless mode automatically.
- **OpenCode:** Needs provider API key configured. `opencode -p` for non-interactive mode.
- **Copilot CLI:** `GH_TOKEN` authentication reportedly [does not work reliably](https://github.com/orgs/community/discussions/167158) in headless contexts.

**Consequences:** Entire test matrix fails in CI. Tests become "local only" which defeats the purpose. Secrets management becomes a blocking issue before any real test logic is written.

**Prevention:**
1. Design tests to work WITHOUT real API keys where possible (test hook capture, not LLM responses)
2. For hook-only tests: mock the CLI output, test the shell scripts directly by piping JSON to stdin
3. For full integration tests: use CI secrets with clear documentation of required env vars
4. Create a `check-prerequisites.sh` that validates all CLIs and credentials before running, marking unavailable CLIs as SKIPPED not FAILED
5. Separate "hook capture tests" (no API key needed) from "full round-trip tests" (API key required)
6. Use `MEMORY_INGEST_DRY_RUN=1` for tests that only validate hook script logic (both Gemini and Copilot adapters already support this)

**Detection:** All tests fail in CI with auth errors. New contributor onboarding takes hours because of credential setup.

**Phase to address:** Framework phase -- prerequisite checking and skip-vs-fail distinction must be in the harness core.

**Severity:** CRITICAL

---

### Pitfall 3: Workspace State Leaking Between Tests

**What goes wrong:** Tests share state through filesystem artifacts, temp files, daemon state, or session files. One test's output corrupts another test's expectations.

**Why it happens:** The Copilot adapter uses shared temp files for session synthesis (`/tmp/copilot-memory-session-${CWD_HASH}`). The memory daemon uses per-project RocksDB stores keyed by CWD. If two tests use the same directory, they share the same store. If cleanup fails, stale data persists.

**Consequences:** Non-deterministic test failures. Tests pass in isolation but fail when run together. Test order matters (a hidden dependency). Debugging requires running the exact sequence that failed.

**Prevention:**
1. Create a unique temp directory per test: `WORKSPACE=$(mktemp -d "/tmp/e2e-test-XXXXXX")`
2. Set `CWD` to the unique workspace so each test gets its own RocksDB store
3. Clean up session files in teardown (`rm -f /tmp/copilot-memory-session-*`)
4. Run the memory daemon with a test-specific config pointing to the workspace
5. Use `trap` to ensure cleanup happens even on failure
6. NEVER use `/tmp` directly for session files in tests -- use `$WORKSPACE/tmp/`
7. Override `SESSION_FILE` location via environment variable in the Copilot adapter

**Detection:** Tests fail intermittently. Running a single test passes, but the full suite fails. Test output contains data from other tests.

**Phase to address:** Framework phase -- workspace isolation is the foundation everything else builds on.

**Severity:** CRITICAL

---

### Pitfall 4: Hook Timing and Async Event Delivery

**What goes wrong:** Tests assert on events captured by hooks, but hooks fire asynchronously. The test checks for events before the hook has delivered them to the daemon. Or the daemon has not yet processed the ingested event.

**Why it happens:** Both Gemini and Copilot hook scripts send events to `memory-ingest` in the background (`&`). The hook returns immediately (fail-open design). The ingest binary sends a gRPC call. The daemon processes it asynchronously. There are at least 3 async boundaries between "hook fires" and "event is queryable."

**Consequences:** Tests that assert "event was captured" fail intermittently. Adding `sleep 2` "fixes" them (classic flaky test antipattern). Test suite runtime balloons because of defensive sleeps.

**Prevention:**
1. Implement a poll-with-timeout pattern: `wait_for_event(predicate, timeout_secs)` that polls the daemon
2. Use the daemon's gRPC API to check event count, not filesystem artifacts
3. Set a reasonable poll interval (100ms) with a hard timeout (10s)
4. For hook-only tests (no daemon): capture the ingest payload to a file instead of sending to daemon, then assert on the file contents synchronously
5. Set `MEMORY_INGEST_DRY_RUN=1` for tests that only validate hook script logic
6. Create a `capture-ingest` mock binary that writes payloads to a file for assertion

**Detection:** Tests pass locally (fast machine) but fail in CI (slower). Adding sleeps makes them pass. Different failure rates on different machines.

**Phase to address:** Framework phase for the polling utility. Each CLI phase uses it.

**Severity:** CRITICAL

---

## Moderate Pitfalls

Mistakes that cause delays, flakiness, or accumulated maintenance burden.

---

### Pitfall 5: Headless Mode Behavioral Differences Per CLI

**What goes wrong:** Each CLI has subtly different headless behavior. Tests written assuming one CLI's behavior break when applied to another.

**Specific differences discovered:**
- **Claude Code (`-p` flag):** User-invoked skills and built-in commands are NOT available. [Large stdin (7000+ chars) returns empty output](https://github.com/anthropics/claude-code/issues/7263). Trust verification is disabled. Sessions do not persist between invocations.
- **Codex CLI (`codex exec`):** JSON Lines output with `--json` flag (stream of events, NOT single JSON). Event types include thread/turn/item events. `--full-auto` enables low-friction automation. Sandbox blocks network by default.
- **Gemini CLI:** Auto-detects non-TTY for headless. JSON output via `--output-format json`. Single prompt, then exits. [Known freezing in non-interactive with debug enabled](https://github.com/google-gemini/gemini-cli/pull/14580).
- **OpenCode (`opencode -p` / `opencode run`):** Supports `-f json` for JSON output. `-q` for quiet mode. [Known bug: exits after auto-compaction if token overflow](https://github.com/anomalyco/opencode/issues/13946) (Feb 2026).
- **Copilot CLI:** GH_TOKEN auth unreliable in headless. No session_id provided (must synthesize via temp file). sessionStart fires per-prompt (Bug #991). toolArgs is a JSON string, not object (double-parse required).

**Prevention:**
1. Create a per-CLI configuration file that documents: invocation command, output format flag, authentication env var, known limitations, and skip conditions
2. Abstract CLI invocation behind a `run_cli.sh` wrapper that normalizes output format
3. Test one CLI at a time in separate phases so behavioral assumptions do not bleed across
4. Mark known-broken scenarios as SKIPPED with a reference to the upstream bug

**Phase to address:** Each CLI gets its own phase. Claude Code first to build the abstraction layer.

**Severity:** HIGH

---

### Pitfall 6: Golden File Fragility

**What goes wrong:** Tests compare CLI output against stored golden files. Any change in CLI version, output formatting, timestamp format, or field ordering breaks tests. Golden files become a maintenance burden.

**Why it happens:** CLI tools update frequently (weekly/monthly). Output format changes are rarely documented in changelogs. Version strings in output change on every update. Timestamps are inherently non-deterministic.

**Consequences:** Tests break after any CLI update, even when actual behavior is correct. Team spends time updating golden files instead of finding bugs. Trust in the test suite erodes.

**Prevention:**
1. DO NOT use golden files for CLI output comparison. Instead:
   - Assert on structural properties: "output contains key X", "JSON has field Y with value matching pattern Z"
   - Use `jq` to extract specific fields and compare those
   - Normalize timestamps, version strings, and paths before comparison
2. If golden files are truly needed (e.g., for hook payload format validation):
   - Store only the schema/structure, not exact values
   - Use an `--update` flag pattern to regenerate golden files intentionally
   - Pin CLI versions in CI to reduce churn
3. Prefer semantic assertions: "event was ingested with agent=gemini" over "output matches this exact JSON blob"

**Phase to address:** Framework phase -- establish assertion patterns early. Avoid golden files from the start.

**Severity:** HIGH

---

### Pitfall 7: CI Environment Differences

**What goes wrong:** Tests pass locally but fail in CI due to missing CLIs, different PATH, different OS behavior, or resource constraints.

**Specific risks for this project:**
- Not all 5 CLIs will be installed in CI (especially Copilot, which requires GitHub app auth)
- macOS vs Linux differences in shell commands: `date -r` vs `date -d` for timestamp conversion (already handled in Copilot adapter but a pattern that will recur across all test scripts)
- `md5sum` vs `md5`, `uuidgen` availability and output case (already handled in Copilot adapter)
- CI containers run under resource constraints -- CLI startup is slower
- GitHub Actions runners have limited concurrent process capacity
- `jq` version differences: `walk()` requires jq 1.6+ (adapters already handle this with runtime check)

**Prevention:**
1. Prerequisite check script that marks missing CLIs as SKIPPED
2. Use GitHub Actions matrix strategy to test on both macOS and Ubuntu
3. Pin CLI versions in CI using exact version install scripts
4. Use conditional test execution: `if command -v claude >/dev/null; then run_claude_tests; else skip "Claude Code not found"; fi`
5. Set generous timeouts for CI (2x local timeouts)
6. Create a `lib/compat.sh` sourced by all test scripts with portable wrappers for OS-specific commands
7. Document exact CI setup requirements in a setup action

**Phase to address:** Framework phase for the prerequisite system. CI integration as a dedicated concern.

**Severity:** HIGH

---

### Pitfall 8: Codex CLI Constraints Beyond Missing Hooks

**What goes wrong:** Codex CLI is assumed to work like the other 4 CLIs minus hooks, but it has additional constraints that surface during testing.

**Known Codex constraints (from [official docs](https://developers.openai.com/codex/cli/reference/)):**
- No hook system at all -- cannot capture events passively
- Sandbox mode blocks network by default (memory daemon gRPC calls would fail in sandbox)
- `.codex/` directory is read-only in workspace-write mode
- `on-failure` approval policy is deprecated -- must use `on-request` or `never`
- `codex exec` is the headless mode (NOT a `-p` flag like Claude Code)
- JSON Lines output (stream of events, not single JSON object) -- requires different parsing
- Uses `notify` config for external program notification (NOT the same as lifecycle hooks)
- The `notify` system runs an external program but only for specific notification types, not full lifecycle events

**Consequences:** Tests written for other CLIs cannot be trivially adapted for Codex. Sandbox restrictions prevent the adapter from communicating with the daemon. The "no hooks" constraint is deeper than just "skip hook tests."

**Prevention:**
1. Design Codex adapter to use explicit command invocation (not passive capture)
2. For testing: use `--full-auto` with appropriate sandbox settings, or disable sandbox for test scenarios
3. Parse JSON Lines output with `while IFS= read -r line` not `jq .`
4. Test Codex separately with its own assertion patterns
5. Document that Codex tests validate command/skill execution, NOT event capture
6. Consider using Codex's `notify` config as a limited notification substitute for testing (but do not conflate with hooks)

**Phase to address:** Codex adapter phase (last CLI phase, after framework is proven with the other 4).

**Severity:** MEDIUM

---

### Pitfall 9: Test Matrix Explosion

**What goes wrong:** 5 CLIs x 7 scenarios = 35 tests. Adding OS matrix (macOS + Linux) doubles to 70. Adding retry logic for flaky tests triples effective CI time. Suite takes 30+ minutes.

**Why it happens:** Naive approach tests every CLI against every scenario. Some scenarios are irrelevant for some CLIs (hooks for Codex, session synthesis for Claude Code). No prioritization of which combinations matter.

**Consequences:** CI becomes a bottleneck. Developers skip running E2E locally. Test maintenance cost exceeds test value. Team pushes to disable tests.

**Prevention:**
1. Define a test taxonomy:
   - **Universal tests** (all CLIs): basic invocation, daemon communication, command execution
   - **Hook tests** (4 CLIs, not Codex): event capture, payload format, fail-open behavior
   - **CLI-specific tests**: session synthesis (Copilot), headless output (Codex exec), etc.
2. Target 20-25 tests total, not 35+
3. Run "smoke" subset on PRs (5-10 tests), full matrix nightly
4. Use test tagging for selective execution
5. Parallelize across CLIs (each CLI's tests are independent)
6. Set a hard CI time budget: 15 minutes for E2E, period

**Phase to address:** Framework phase for the taxonomy. Each CLI phase adds only relevant tests.

**Severity:** MEDIUM

---

## Minor Pitfalls

Mistakes that cause annoyance but are fixable without major rework.

---

### Pitfall 10: Shell Script Portability Across macOS and Linux

**What goes wrong:** Test harness shell scripts use bash-isms or OS-specific commands that fail on the other platform.

**Already observed in codebase:**
- Copilot adapter handles `date -r` (macOS) vs `date -d` (Linux)
- Copilot adapter handles `md5sum` (Linux) vs `md5` (macOS)
- Copilot adapter handles `uuidgen` vs `/proc/sys/kernel/random/uuid`
- Both adapters handle ANSI stripping via perl (preferred) with sed fallback

**Prevention:**
1. Create a `lib/compat.sh` sourced by all test scripts with portable wrappers
2. Use `#!/usr/bin/env bash` (already done in adapters)
3. Test on both macOS and Linux in CI
4. Avoid GNU-specific flags (`sed -i ''` on macOS vs `sed -i` on Linux)

**Phase to address:** Framework phase -- create the compatibility library first.

**Severity:** LOW

---

### Pitfall 11: Daemon Port Conflicts in Parallel Tests

**What goes wrong:** Multiple test processes try to start the memory daemon on the same gRPC port. Only one succeeds; others fail with "address already in use."

**Prevention:**
1. Assign unique ports per test using a counter or random port allocation
2. Use port 0 (OS-assigned) and capture the actual port from daemon startup output
3. Or use a single shared daemon instance for all tests (simpler but reduces isolation)
4. Prefer Unix domain sockets over TCP for test-local communication (faster, no port conflicts)

**Phase to address:** Framework phase.

**Severity:** LOW

---

### Pitfall 12: ANSI Escape Sequence Contamination

**What goes wrong:** CLI output includes ANSI color codes, cursor movement, or spinner animations that corrupt JSON parsing in test assertions.

**Already observed in codebase:** Both Gemini and Copilot adapters include ANSI stripping logic using perl/sed. This same problem will affect test output parsing.

**Prevention:**
1. Set `NO_COLOR=1` or `TERM=dumb` environment variables when spawning CLIs
2. Use `--no-color` or equivalent flags if available per CLI
3. Strip ANSI before JSON parsing (reuse existing adapter pattern)
4. Pipe CLI output through a normalizer as a safety net

**Phase to address:** Framework phase -- set environment variables in the harness core.

**Severity:** LOW

---

### Pitfall 13: Memory Daemon Startup Race Condition

**What goes wrong:** Test starts daemon and immediately sends requests. Daemon is not yet listening, requests fail.

**Prevention:**
1. Health check loop: poll `grpc_health_v1.Health/Check` until ready
2. Read daemon stdout for "listening on" message
3. Implement `wait_for_daemon(port, timeout)` helper in the test framework
4. Set a maximum startup timeout (5 seconds) with clear error message

**Phase to address:** Framework phase.

**Severity:** LOW

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Framework (Claude Code) | Zombie processes, workspace isolation, daemon startup race | Process group kill, mktemp per test, health check loop |
| Framework (Claude Code) | Golden file fragility | Semantic assertions from day one, no golden files |
| Claude Code tests | Auth failure in CI, `-p` mode limitations (no skills, large input bug) | API key in CI secrets, test hook scripts not interactive features |
| OpenCode tests | `opencode run` compaction exit bug, different output format | Pin version, use `-q` flag, guard against unexpected exit 0 |
| Gemini tests | Debug mode freezing in non-interactive, ANSI contamination | Set `NO_COLOR=1`, never enable debug in tests, strip ANSI |
| Copilot tests | No session_id (synthesis via temp file), sessionStart per-prompt bug, GH_TOKEN unreliable | Use workspace-scoped session files, handle duplicate sessionStart, test with app-level auth |
| Codex tests | No hooks at all, sandbox blocks network, JSON Lines output format | Skip all hook tests, disable sandbox for tests, parse JSONL not JSON |
| CI integration | Missing CLIs, OS differences, timeout flakiness | Prerequisite skip logic, generous timeouts, matrix for macOS + Linux |
| Matrix reporting | False failures from infra issues counted against CLIs | Distinguish infra failures from test failures in reporting |

---

## Integration Pitfalls (Adding to Existing System)

These pitfalls are specific to adding E2E CLI testing on top of an existing system with 29 cargo-based E2E tests.

### Existing Test Interference

**What goes wrong:** New shell-based CLI tests and existing cargo E2E tests compete for daemon resources, ports, or RocksDB stores when run in the same CI job.

**Prevention:**
- Run shell E2E tests in a separate CI job (the existing system already has a dedicated E2E job)
- Use different port ranges for cargo tests vs shell tests
- Never share workspaces between the two test layers

### Config File Pollution

**What goes wrong:** CLI tests create or modify config files (`~/.config/agent-memory/`, `~/.claude/`, `.gemini/`, `.codex/`) that affect subsequent tests or the developer's local environment.

**Prevention:**
- Override all config paths with environment variables pointing to the workspace
- Set `HOME` to a temp directory for CLI tests
- Or use `XDG_CONFIG_HOME` override to redirect config discovery

### Hook Script Modification During Testing

**What goes wrong:** Tests that validate hook installation or modification accidentally alter the source hook scripts in the repository, causing git dirty state.

**Prevention:**
- Always copy hook scripts to the workspace, never modify in-place
- Use `git diff --exit-code` as a post-test assertion that no source files changed
- Run tests in a git worktree or clean copy

---

## Sources

### CLI-Specific Official Documentation
- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks) - Hook event types and handler documentation
- [Claude Code Headless Mode](https://code.claude.com/docs/en/headless) - Non-interactive `-p` flag documentation and limitations
- [Claude Code Large Input Bug #7263](https://github.com/anthropics/claude-code/issues/7263) - Empty output with large stdin in headless
- [Claude Code Security Issue #20253](https://github.com/anthropics/claude-code/issues/20253) - Trust verification disabled in `-p` mode
- [Codex CLI Non-Interactive Mode](https://developers.openai.com/codex/noninteractive) - `codex exec` documentation
- [Codex CLI Reference](https://developers.openai.com/codex/cli/reference/) - Command line options and sandbox modes
- [Codex Advanced Configuration](https://developers.openai.com/codex/config-advanced/) - Sandbox, notify, and approval settings
- [Codex Changelog](https://developers.openai.com/codex/changelog/) - Recent updates including Feb 2026 changes
- [Gemini CLI Headless Reference](https://geminicli.com/docs/cli/headless/) - Non-interactive mode
- [Gemini CLI Freezing Bug Fix #14580](https://github.com/google-gemini/gemini-cli/pull/14580) - Debug mode freezing
- [Gemini CLI Non-Interactive Commands #5435](https://github.com/google-gemini/gemini-cli/issues/5435) - Slash commands in headless
- [OpenCode CLI Documentation](https://opencode.ai/docs/cli/) - Non-interactive mode and flags
- [OpenCode Run Compaction Bug #13946](https://github.com/anomalyco/opencode/issues/13946) - Exit after compaction overflow
- [OpenCode Headless Mode Request #953](https://github.com/sst/opencode/issues/953) - Non-interactive mode history
- [Copilot CLI Auth Discussion #167158](https://github.com/orgs/community/discussions/167158) - GH_TOKEN authentication problems

### E2E Testing Best Practices
- [E2E Testing Best Practices 2025](https://www.bunnyshell.com/blog/best-practices-for-end-to-end-testing-in-2025/) - Flakiness prevention, test pyramid
- [Shell Scripting Best Practices](https://oneuptime.com/blog/post/2026-02-13-shell-scripting-best-practices/view) - Trap patterns, cleanup, portability
- [Fixing Flaky E2E Tests in CI](https://medium.com/@Adekola_Olawale/fixing-flaky-end-to-end-cypress-tests-in-ci-environments-71902f12dbb9) - CI environment differences
- [Golden File Testing Introduction](https://ro-che.info/articles/2017-12-04-golden-tests) - Fragility and maintenance concerns
- [Zombie Process Fixes](https://oneuptime.com/blog/post/2026-01-24-fix-zombie-process-issues/view) - Process reaping in containers

### Codebase References
- `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh` - Gemini hook patterns, fail-open, ANSI stripping
- `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` - Copilot hook patterns, session synthesis, OS compatibility
