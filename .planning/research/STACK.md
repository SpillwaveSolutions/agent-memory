# Technology Stack: Headless CLI E2E Testing Harness

**Project:** Agent Memory v2.4 -- Headless CLI E2E Testing
**Researched:** 2026-02-22
**Overall Confidence:** HIGH

> This document covers ONLY the v2.4 stack additions. The existing Rust stack (tokio, tonic, rocksdb, tantivy, etc.) is validated and unchanged.

---

## Recommended Stack

### Shell Testing Framework

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| bats-core | 1.12.x | Primary test runner | TAP-compliant, native JUnit output (`--formatter junit`), parallel execution (`--jobs`), `setup_file`/`teardown_file` for workspace lifecycle, `bats::on_failure` hook (v1.12). The only serious Bash testing framework with CI-native reporting. |
| bats-support | 0.3.x | Core assertion helpers | `assert`, `refute`, `assert_equal` -- required foundation for bats-assert |
| bats-assert | 2.1.x | Output assertions | `assert_output --partial`, `assert_line`, `refute_output` -- validates CLI stdout/stderr content |
| bats-file | 0.4.x | Filesystem assertions | `assert_file_exists`, `assert_dir_exists` -- validates workspace artifacts after CLI runs |

### CLI-Specific Dependencies

| Technology | Purpose | Why |
|------------|---------|-----|
| jq (1.7+) | JSON parsing in tests | Already a project dependency (hooks use jq). Validates JSON output from `--output-format json` modes across all CLIs. |
| timeout / gtimeout | Process kill guard | CLIs can hang if API keys are invalid or prompts trigger interactive fallback. `timeout 60s claude -p ...` prevents CI deadlock. On macOS, use `gtimeout` from `coreutils`. |

### Reporting and CI

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| bats JUnit formatter | built-in | CI artifact | `bats --report-formatter junit --output ./results/` produces JUnit XML natively. No external converter needed. |
| test-summary/action | v2 | GitHub Actions summary | Parses JUnit XML and renders pass/fail table in PR checks. |

### NOT Adding

| Technology | Why Not |
|------------|---------|
| shunit2 | No parallel execution, no native JUnit, no helper libraries. bats-core dominates shell testing. |
| Python pytest | User preference is shell-first. Python adds runtime dependency, virtualenv management, language boundary. |
| Bun/Deno test | Same objection as Python. Unnecessary for what is fundamentally shell process management. |
| tap-xunit | bats-core has native JUnit output since v1.7. External TAP-to-JUnit conversion is no longer needed. |
| Docker sandbox | Gemini CLI uses Docker sandbox with `--yolo --sandbox`, but our tests validate real local behavior. Docker adds CI complexity. Temp directory isolation is sufficient. |

---

## Headless CLI Invocation Reference

This is the critical research: how to run each CLI non-interactively.

### Claude Code

**Confidence:** HIGH (verified via official docs at code.claude.com/docs/en/headless)

| Flag | Purpose |
|------|---------|
| `-p "prompt"` / `--print "prompt"` | Non-interactive mode. Runs prompt, prints result, exits. |
| `--output-format json` | Structured JSON with `result`, `session_id`, metadata |
| `--output-format stream-json` | NDJSON streaming (for real-time monitoring) |
| `--output-format text` | Plain text (default) |
| `--allowedTools "Bash,Read,Edit"` | Auto-approve specific tools (no confirmation prompts) |
| `--append-system-prompt "..."` | Add instructions while keeping defaults |
| `--continue` | Continue most recent conversation |
| `--resume SESSION_ID` | Continue specific conversation |
| `--model MODEL` | Select model |
| `--json-schema '{...}'` | Constrain output to schema (with `--output-format json`) |

**Test invocation pattern:**
```bash
timeout 120s claude -p "Read the file test.txt and tell me its contents" \
  --output-format json \
  --allowedTools "Read" \
  2>"$TEST_STDERR"
```

**Environment:**
- `ANTHROPIC_API_KEY` -- required for auth
- No TTY required (but see bug #9026: some versions hang without TTY; `--output-format json` mitigates)

**Hook mechanism:** Claude Code Context Hooks (CCH) -- event-driven, hook scripts in `.claude/hooks/`

**Sources:**
- [Official headless docs](https://code.claude.com/docs/en/headless)
- [Bug #9026: TTY hang](https://github.com/anthropics/claude-code/issues/9026)

---

### Gemini CLI

**Confidence:** HIGH (verified via official docs and GitHub repo)

| Flag | Purpose |
|------|---------|
| `"prompt"` (positional arg) | Non-interactive mode |
| `--question "prompt"` | Alternative prompt flag |
| `--output-format text` | Plain text output (default) |
| `--output-format json` | Structured JSON at completion |
| `--output-format stream-json` | NDJSON event stream |
| `--yolo` | Auto-approve all tool calls |
| `--sandbox` | Run tools in Docker sandbox (auto-enabled with `--yolo`) |
| `--sandbox=false` | Disable sandbox even with `--yolo` |

**Test invocation pattern:**
```bash
timeout 120s gemini --yolo --sandbox=false \
  --output-format json \
  "Read the file test.txt and tell me its contents" \
  2>"$TEST_STDERR"
```

**Environment:**
- `GEMINI_API_KEY` or cached auth credentials
- Hooks in `.gemini/hooks/` directory (shell scripts receiving JSON on stdin)

**Hook mechanism:** File-based hooks in `.gemini/hooks/`, JSON on stdin, `{}` on stdout

**Sources:**
- [Official headless docs](https://google-gemini.github.io/gemini-cli/docs/cli/headless.html)
- [GitHub source](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/headless.md)

---

### OpenCode CLI

**Confidence:** MEDIUM (docs confirm `-p` flag; headless mode is newer, less battle-tested)

| Flag | Purpose |
|------|---------|
| `-p "prompt"` | Non-interactive mode |
| `run "prompt"` | Alternative non-interactive subcommand |
| `-q` / `--quiet` | Disable spinner (essential for script parsing) |
| `-f json` / `--format json` | JSON output format |

**Test invocation pattern:**
```bash
timeout 120s opencode -p "Read the file test.txt and tell me its contents" \
  -q -f json \
  2>"$TEST_STDERR"
```

**Environment:**
- API key env vars (provider-dependent: `OPENAI_API_KEY` or `ANTHROPIC_API_KEY`)
- All permissions auto-approved in non-interactive mode

**Hook mechanism:** OpenCode hooks system (event-based, similar to Gemini)

**CAVEAT:** OpenCode issue #10411 requests improved non-interactive mode for `opencode run`. The `-p` flag works but may have rough edges. Test early, validate behavior.

**Sources:**
- [CLI docs](https://opencode.ai/docs/cli/)
- [Issue #10411](https://github.com/anomalyco/opencode/issues/10411)
- [Issue #953: headless mode request](https://github.com/sst/opencode/issues/953)

---

### GitHub Copilot CLI

**Confidence:** HIGH (verified via GitHub official docs)

| Flag | Purpose |
|------|---------|
| `-p "prompt"` / `--prompt "prompt"` | Non-interactive mode |
| `--yes` | Skip confirmation prompts |
| `--allow-all-tools` | Grant all tool permissions |
| `--allow-all` / `--yolo` | Enable all permissions at once |
| `--allow-tool TOOL` | Permit specific tool |
| `--deny-tool TOOL` | Block specific tool |
| `--model MODEL` | Select model |

**Test invocation pattern:**
```bash
timeout 120s copilot -p "Read the file test.txt and tell me its contents" \
  --yes --allow-all-tools \
  2>"$TEST_STDERR"
```

**Environment:**
- `GITHUB_TOKEN` -- required for auth
- Use `--allow-all-tools` to prevent permission-related hangs in non-interactive mode

**Hook mechanism:** Hooks config in `.github/hooks/`, scripts receive event type as `$1`, JSON on stdin

**CAVEAT:** Issue #633 notes MCP servers are not run in non-interactive mode. Issue #550 notes hanging with `-p` on permission errors. Always use `--allow-all-tools`.

**Sources:**
- [Official CLI docs](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/use-copilot-cli)
- [Issue #633: MCP in non-interactive](https://github.com/github/copilot-cli/issues/633)
- [Issue #550: hang on permission error](https://github.com/github/copilot-cli/issues/550)

---

### OpenAI Codex CLI

**Confidence:** HIGH (verified via official developer docs)

| Flag / Command | Purpose |
|----------------|---------|
| `codex exec "prompt"` | Non-interactive execution (no TUI) |
| `-q` / `--quiet` | Quiet mode -- non-interactive, final output only |
| `--full-auto` | Auto-approve with `on-request` approval + workspace-write sandbox |
| `-a never` | Disable all approval prompts |
| `--dangerously-bypass-approvals-and-sandbox` | Full unrestricted access (use only in test sandbox) |

**Test invocation pattern:**
```bash
timeout 120s codex exec -q --full-auto \
  "Read the file test.txt and tell me its contents" \
  2>"$TEST_STDERR"
```

**Environment:**
- `OPENAI_API_KEY` -- required for auth
- **No hook support.** Codex CLI has no hooks/extension system. Adapter is commands+skills only.

**CAVEAT:** Issue #1340 notes `-q` mode is not truly non-interactive when git warnings appear. Use `--full-auto` alongside `-q` to suppress all prompts.

**Hook mechanism:** NONE. Codex CLI has no hook system. Hook-dependent E2E tests must be skipped for Codex.

**Sources:**
- [Official non-interactive docs](https://developers.openai.com/codex/noninteractive)
- [CLI reference](https://developers.openai.com/codex/cli/reference/)
- [Issue #1340: quiet mode git warning](https://github.com/openai/codex/issues/1340)

---

## Test Isolation Strategy

Use bats-core `setup_file` / `teardown_file` for per-file workspace lifecycle:

```bash
setup_file() {
  # Create isolated workspace
  export TEST_WORKSPACE="$(mktemp -d)"
  export TEST_STDERR="$TEST_WORKSPACE/stderr.log"
  export TEST_PROJECT="$TEST_WORKSPACE/project"
  mkdir -p "$TEST_PROJECT"

  # Seed workspace with test fixtures
  cp -r "$BATS_TEST_DIRNAME/../fixtures/plugin-files/." "$TEST_PROJECT/"
  echo "Hello from test fixture" > "$TEST_PROJECT/test.txt"

  # Build and start memory daemon against isolated DB
  export DAEMON_LOG="$TEST_WORKSPACE/daemon.log"
  "$PROJECT_ROOT/target/release/memory-daemon" \
    --db-path "$TEST_WORKSPACE/db" \
    --port 0 > "$DAEMON_LOG" 2>&1 &
  export DAEMON_PID=$!

  # Wait for daemon ready (up to 3 seconds)
  for i in $(seq 1 30); do
    if grpcurl -plaintext "localhost:$DAEMON_PORT" grpc.health.v1.Health/Check >/dev/null 2>&1; then
      break
    fi
    sleep 0.1
  done
}

teardown_file() {
  # Kill daemon
  if [ -n "${DAEMON_PID:-}" ]; then
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi

  # Preserve workspace on failure for CI artifact collection
  if [ "${BATS_SUITE_TEST_FAILED:-0}" -gt 0 ]; then
    local artifact_dir="$PROJECT_ROOT/test-artifacts"
    mkdir -p "$artifact_dir"
    tar czf "$artifact_dir/${BATS_TEST_FILENAME##*/}.tar.gz" \
      -C "$TEST_WORKSPACE" . 2>/dev/null || true
  else
    rm -rf "$TEST_WORKSPACE"
  fi
}
```

**Key isolation properties:**
- Each `.bats` file gets its own temp directory, daemon instance, and RocksDB database
- CLI plugins are copied into the workspace (not symlinked) so tests cannot pollute each other
- Workspace is preserved on failure for debugging (uploaded as CI artifact)
- Daemon uses port 0 (OS-assigned) to avoid port conflicts in parallel runs

---

## CI Integration Pattern

### GitHub Actions Matrix

```yaml
jobs:
  cli-e2e:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false  # Run all CLIs even if one fails
      matrix:
        cli: [claude, gemini, opencode, copilot, codex]
    steps:
      - uses: actions/checkout@v4

      - name: Install bats-core and helpers
        run: |
          git clone --depth 1 --branch v1.12.0 \
            https://github.com/bats-core/bats-core.git /tmp/bats
          sudo /tmp/bats/install.sh /usr/local
          mkdir -p tests/e2e-cli/test_helper
          git clone --depth 1 https://github.com/bats-core/bats-support.git \
            tests/e2e-cli/test_helper/bats-support
          git clone --depth 1 https://github.com/bats-core/bats-assert.git \
            tests/e2e-cli/test_helper/bats-assert
          git clone --depth 1 https://github.com/bats-core/bats-file.git \
            tests/e2e-cli/test_helper/bats-file

      - name: Build memory-daemon
        run: cargo build --release -p memory-daemon

      - name: Run ${{ matrix.cli }} E2E tests
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          bats tests/e2e-cli/${{ matrix.cli }}/ \
            --report-formatter junit \
            --output ./test-results/${{ matrix.cli }}/ \
            --jobs 2

      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: e2e-results-${{ matrix.cli }}
          path: |
            test-results/${{ matrix.cli }}/
            test-artifacts/

      - uses: test-summary/action@v2
        if: always()
        with:
          paths: test-results/${{ matrix.cli }}/**/*.xml
```

---

## Proposed Directory Structure

```
tests/
  e2e-cli/
    test_helper/
      bats-support/     # git clone (gitignored, installed in CI)
      bats-assert/      # git clone (gitignored, installed in CI)
      bats-file/        # git clone (gitignored, installed in CI)
      common.bash       # shared: workspace helpers, daemon lifecycle, CLI wrappers
    fixtures/
      plugin-files/     # minimal adapter configs for each CLI
      test-prompts.bash # standard prompt strings (read file, list files, etc.)
    claude/
      smoke.bats        # basic headless invocation + output validation
      hooks.bats        # hook capture: ingest event -> verify in daemon
      memory.bats       # full pipeline: hook capture -> query via skill
    gemini/
      smoke.bats
      hooks.bats
      memory.bats
    opencode/
      smoke.bats
      hooks.bats
      memory.bats
    copilot/
      smoke.bats
      hooks.bats
      memory.bats
    codex/
      smoke.bats
      commands.bats     # commands only (no hooks -- Codex has none)
      memory.bats       # query-only via explicit ingest (no passive capture)
    setup-bats.sh       # installs bats + helpers locally for dev
```

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Test framework | bats-core 1.12 | shunit2 | No parallel execution, no JUnit output, smaller ecosystem |
| Test framework | bats-core 1.12 | pytest + subprocess | Adds Python dependency, user prefers shell-first |
| Reporting | JUnit XML (native bats) | TAP + tap-xunit | Extra conversion step; bats has native JUnit since v1.7 |
| Isolation | mktemp + per-file daemon | Docker per-test | Massive CI overhead, Docker-in-Docker complexity |
| Process guard | timeout/gtimeout | custom trap | coreutils `timeout` handles SIGKILL escalation correctly |
| Helper install | git clone in CI | npm install bats | bats npm package exists but git clone is simpler, no node dependency |
| Artifact format | tar.gz on failure | always preserve | Disk waste on success; failures need debugging, successes do not |

---

## Installation

```bash
# === macOS ===
brew install bats-core
brew install coreutils  # provides gtimeout
brew install jq

# === Linux (from source) ===
git clone --depth 1 --branch v1.12.0 https://github.com/bats-core/bats-core.git /tmp/bats
sudo /tmp/bats/install.sh /usr/local
sudo apt-get install -y jq coreutils

# === Helper libraries (both platforms) ===
cd tests/e2e-cli/test_helper
git clone --depth 1 https://github.com/bats-core/bats-support.git
git clone --depth 1 https://github.com/bats-core/bats-assert.git
git clone --depth 1 https://github.com/bats-core/bats-file.git

# === Or use the setup script ===
./tests/e2e-cli/setup-bats.sh
```

---

## Headless Invocation Summary Matrix

| CLI | Non-Interactive Flag | Auto-Approve | JSON Output | Hooks | Confidence |
|-----|---------------------|--------------|-------------|-------|------------|
| Claude Code | `-p "prompt"` | `--allowedTools "..."` | `--output-format json` | `.claude/hooks/` | HIGH |
| Gemini CLI | `"prompt"` (positional) | `--yolo --sandbox=false` | `--output-format json` | `.gemini/hooks/` | HIGH |
| OpenCode | `-p "prompt"` | auto in non-interactive | `-f json` | hooks system | MEDIUM |
| Copilot CLI | `-p "prompt"` | `--yes --allow-all-tools` | N/A (text only) | `.github/hooks/` | HIGH |
| Codex CLI | `codex exec "prompt"` | `-q --full-auto` | N/A (text only) | NONE | HIGH |

---

## Sources

- [bats-core GitHub](https://github.com/bats-core/bats-core) -- v1.12 confirmed, HIGH confidence
- [bats-core docs: usage](https://bats-core.readthedocs.io/en/latest/usage.html) -- JUnit formatter confirmed, HIGH confidence
- [Claude Code headless docs](https://code.claude.com/docs/en/headless) -- all flags verified, HIGH confidence
- [Gemini CLI headless docs](https://google-gemini.github.io/gemini-cli/docs/cli/headless.html) -- all flags verified, HIGH confidence
- [OpenCode CLI docs](https://opencode.ai/docs/cli/) -- `-p` confirmed, MEDIUM confidence (newer feature)
- [Copilot CLI docs](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/use-copilot-cli) -- verified, HIGH confidence
- [Codex CLI non-interactive docs](https://developers.openai.com/codex/noninteractive) -- `codex exec` verified, HIGH confidence
- [test-summary/action](https://github.com/test-summary/action) -- JUnit XML rendering, HIGH confidence
