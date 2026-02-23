# Architecture Patterns: Headless CLI E2E Testing Harness

**Domain:** Shell-first headless CLI E2E testing harness for Agent Memory
**Researched:** 2026-02-22

## Recommended Architecture

The E2E harness is a **bats-core test framework** that sits alongside (not inside) the existing Rust workspace. It spawns real CLI processes in headless mode, validates that events flow through the memory-daemon pipeline, and reports results via JUnit XML in a CI matrix.

### High-Level Architecture

```
tests/e2e-cli/
    test_helper/
      bats-support/     (git clone, .gitignored)
      bats-assert/      (git clone, .gitignored)
      bats-file/        (git clone, .gitignored)
      common.bash       (THE shared library -- workspace, daemon, CLI wrappers)
    fixtures/
      hook-payloads/    (JSON stdin for hook script testing)
      plugin-files/     (minimal adapter configs per CLI)
      hello-project/    (README + single source file)
      rust-project/     (Cargo.toml, src/main.rs)
    claude/
      smoke.bats
      hooks.bats
      pipeline.bats
    gemini/
      smoke.bats
      hooks.bats
      pipeline.bats
    opencode/
      smoke.bats
      hooks.bats
      pipeline.bats
    copilot/
      smoke.bats
      hooks.bats
      pipeline.bats
    codex/
      smoke.bats
      commands.bats     (no hooks -- commands/skills only)
    setup-bats.sh       (installs bats + helpers locally)
```

### Relationship to Existing Architecture

```
EXISTING (unchanged)                    NEW (additive)
========================               ========================
crates/e2e-tests/                      tests/e2e-cli/
  tests/pipeline_test.rs                 claude/smoke.bats
  tests/bm25_teleport_test.rs           claude/hooks.bats
  tests/multi_agent_test.rs             claude/pipeline.bats
  (29 Rust integration tests)           (Real CLI processes)
  (Direct handler calls)                (Real daemon, real gRPC)
  (No daemon, no gRPC)                  (bats-core + JUnit XML)

plugins/                               tests/e2e-cli/fixtures/
  memory-gemini-adapter/                 (copies adapter files into workspace)
  memory-copilot-adapter/                (validates hook behavior E2E)
  memory-opencode-plugin/

crates/memory-daemon/                  tests/e2e-cli/test_helper/common.bash
  (Production daemon)                    (Starts/stops daemon per test file)

crates/memory-ingest/                  tests/e2e-cli/{cli}/pipeline.bats
  (Production ingest binary)             (Validates ingest via real CLIs)
```

**Key principle:** The two test layers are complementary, not overlapping.

| Layer | What it tests | How it tests | Speed |
|-------|--------------|--------------|-------|
| `crates/e2e-tests/` | Internal pipeline correctness | Direct Rust handler calls, no daemon | Fast (seconds) |
| `tests/e2e-cli/` (new) | End-to-end CLI integration | Real daemon + real CLI processes via bats | Slow (minutes) |

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| **common.bash** | Workspace isolation, daemon lifecycle, CLI wrappers, skip helpers | All .bats files source it |
| **fixtures/** | Static test data: JSON payloads, plugin configs, project templates | Read by .bats files |
| **{cli}/smoke.bats** | Basic headless invocation, binary detection, output validation | common.bash, CLI binary |
| **{cli}/hooks.bats** | Hook script unit tests: mock stdin, verify JSON payload | Hook scripts, common.bash |
| **{cli}/pipeline.bats** | Full E2E: CLI headless -> hook -> daemon -> gRPC query -> verify | CLI, daemon, hook scripts |
| **{cli}/commands.bats** | Codex-only: command invocation without hooks | CLI binary, common.bash |
| **setup-bats.sh** | One-time install of bats-core + helper libraries | git, filesystem |
| **GitHub Actions CI** | Matrix runner: 5 CLIs, JUnit XML, artifact collection | bats, test-summary/action |

### Data Flow

```
1. bats tests/e2e-cli/claude/pipeline.bats
   |
2. setup_file() from common.bash:
   |-- mktemp -d -> $TEST_WORKSPACE
   |-- cp fixtures into workspace
   |-- start_daemon (port 0 -> OS assigns)
   |-- wait_for_daemon health check
   |-- setup adapter hooks in workspace
   |
3. @test "headless prompt ingests events":
   |-- run_claude "What files are in this project?"
   |   |-- timeout 120s claude -p "..." --output-format json --allowedTools "Read"
   |   |-- hooks fire in background -> memory-ingest -> daemon
   |-- sleep 2  # allow async hook processing
   |-- grpcurl query daemon for event count
   |-- assert event_count >= 1
   |-- assert agent field == "claude"
   |
4. teardown_file():
   |-- kill daemon
   |-- if BATS_SUITE_TEST_FAILED > 0: tar.gz workspace -> test-artifacts/
   |-- else: rm -rf workspace
   |
5. bats outputs JUnit XML to test-results/claude/
   |
6. GitHub Actions: test-summary/action renders JUnit in PR checks
```

## Per-CLI Wrapper Functions

Each CLI has different headless flags. Wrappers centralize this in common.bash:

```bash
run_claude() {
  local prompt="$1"; shift
  timeout "${CLI_TIMEOUT:-120}" claude -p "$prompt" \
    --output-format json \
    --allowedTools "Read,Bash(echo *),Bash(ls *)" \
    "$@" 2>>"$TEST_STDERR"
}

run_gemini() {
  local prompt="$1"; shift
  timeout "${CLI_TIMEOUT:-120}" gemini \
    --yolo --sandbox=false \
    --output-format json \
    "$prompt" \
    "$@" 2>>"$TEST_STDERR"
}

run_opencode() {
  local prompt="$1"; shift
  timeout "${CLI_TIMEOUT:-120}" opencode -p "$prompt" \
    -q -f json \
    "$@" 2>>"$TEST_STDERR"
}

run_copilot() {
  local prompt="$1"; shift
  timeout "${CLI_TIMEOUT:-120}" copilot -p "$prompt" \
    --yes --allow-all-tools \
    "$@" 2>>"$TEST_STDERR"
}

run_codex() {
  local prompt="$1"; shift
  timeout "${CLI_TIMEOUT:-120}" codex exec -q --full-auto \
    "$prompt" \
    "$@" 2>>"$TEST_STDERR"
}
```

### Headless Invocation Summary

| CLI | Headless Command | JSON Output | Auto-Approve | Confidence |
|-----|-----------------|-------------|--------------|------------|
| Claude Code | `claude -p "prompt"` | `--output-format json` | `--allowedTools "..."` | HIGH |
| Gemini CLI | `gemini "prompt"` | `--output-format json` | `--yolo --sandbox=false` | HIGH |
| OpenCode | `opencode -p "prompt"` | `-f json -q` | Auto in non-interactive | MEDIUM |
| Copilot CLI | `copilot -p "prompt"` | N/A (text only) | `--yes --allow-all-tools` | HIGH |
| Codex CLI | `codex exec "prompt"` | N/A (text only) | `-q --full-auto` | HIGH |

### Hook Configuration Per CLI

| CLI | Hook Mechanism | Config Location | Agent Tag |
|-----|---------------|-----------------|-----------|
| Claude Code | CCH hooks with pipe handler | `.claude/hooks/` in workspace | `claude` |
| Gemini CLI | Shell hook in `.gemini/hooks/` | `.gemini/hooks/memory-capture.sh` | `gemini` |
| Copilot CLI | Shell hook via `.github/hooks/` | `.github/hooks/scripts/memory-capture.sh` | `copilot` |
| OpenCode | Plugin-based hooks | `.opencode/` in workspace | `opencode` |
| Codex CLI | **No hook support** | N/A (commands/skills only) | `codex` |

## Patterns to Follow

### Pattern 1: common.bash as Single Source of Truth

**What:** All shared logic in one file sourced by every .bats file.
**When:** Always. Never duplicate workspace, daemon, or CLI wrapper code.
**Why:** Single point of maintenance. When a CLI changes flags, update one function.

```bash
# Every .bats file starts with:
load '../test_helper/bats-support/load'
load '../test_helper/bats-assert/load'
load '../test_helper/common'
```

### Pattern 2: setup_file / teardown_file for Expensive Operations

**What:** Use bats file-level hooks (not per-test) for workspace and daemon lifecycle.
**When:** Daemon startup, workspace creation, fixture copying.
**Why:** Starting a daemon per-test is slow. Per-file gives one daemon for all tests in the file.

```bash
setup_file() {
  export PROJECT_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd)"
  require_cli "claude"
  require_daemon_binary

  create_workspace
  copy_adapter_files "claude"
  seed_test_files
  start_daemon
}

teardown_file() {
  stop_daemon
  cleanup_workspace
}
```

### Pattern 3: Daemon Port Discovery via Port 0

**What:** Start daemon on port 0 (OS assigns), extract actual port from log.
**When:** Every test workspace that needs a daemon.
**Why:** Avoids port conflicts in parallel test execution.

```bash
start_daemon() {
  "$PROJECT_ROOT/target/release/memory-daemon" \
    --db-path "$TEST_WORKSPACE/db" \
    --port 0 \
    > "$TEST_WORKSPACE/daemon.log" 2>&1 &
  export DAEMON_PID=$!

  for i in $(seq 1 50); do
    DAEMON_PORT=$(grep -o 'listening on.*:[0-9]*' "$TEST_WORKSPACE/daemon.log" 2>/dev/null \
      | grep -o '[0-9]*$' | head -1)
    [ -n "$DAEMON_PORT" ] && break
    sleep 0.1
  done
  export DAEMON_PORT
}
```

### Pattern 4: require_cli Skip Pattern

**What:** Skip entire test file when a CLI is not installed.
**When:** Top of setup_file in every CLI-specific .bats file.
**Why:** CI shows "skipped", not "failed".

```bash
require_cli() {
  if ! command -v "$1" >/dev/null 2>&1; then
    skip "CLI '$1' not installed"
  fi
}
```

### Pattern 5: Assert via gRPC, Not CLI Output

**What:** Verify outcomes by querying the daemon's gRPC API.
**When:** Pipeline tests that verify event ingestion.
**Why:** The daemon is the source of truth. CLI output is non-deterministic.

```bash
@test "hook ingest produces events in daemon" {
  run_claude "List the files in this directory"
  sleep 2  # allow async hook processing

  local count
  count=$(grpcurl -plaintext "localhost:$DAEMON_PORT" \
    memory.MemoryService/GetStats 2>/dev/null | jq -r '.event_count')

  assert [ "$count" -ge 1 ]
}
```

### Pattern 6: Hook Script Testing via Stdin Pipe

**What:** Test hook scripts directly by piping JSON payloads.
**When:** hooks.bats files for each CLI (except Codex).
**Why:** Fast, deterministic, no API key needed.

```bash
@test "gemini hook: SessionStart produces valid output" {
  run bash "$TEST_PROJECT/.gemini/hooks/memory-capture.sh" \
    < "$BATS_TEST_DIRNAME/../fixtures/hook-payloads/gemini-session-start.json"

  assert_success
  assert_output "{}"
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Shared Daemon Across Test Files
**Why bad:** Ordering dependencies, shared state corruption, blocks parallel execution.
**Instead:** Each .bats file starts its own daemon in setup_file.

### Anti-Pattern 2: Hardcoded Ports
**Why bad:** Parallel execution causes conflicts.
**Instead:** Port 0 with discovery from daemon log.

### Anti-Pattern 3: Asserting on LLM Output Content
**Why bad:** Non-deterministic. Tests will be flaky.
**Instead:** Assert structural properties: exit code, JSON validity, field presence, event counts.

### Anti-Pattern 4: Inline Flag Strings in Tests
**Why bad:** Flag changes require updating every test.
**Instead:** Per-CLI wrapper functions in common.bash.

### Anti-Pattern 5: Testing Without Timeout
**Why bad:** Hung CLI blocks CI indefinitely.
**Instead:** Every CLI invocation wrapped in `timeout`.

### Anti-Pattern 6: Custom Test Runner Instead of bats-core
**Why bad:** Reinvents TAP output, parallel execution, JUnit reporting, assertion helpers.
**Instead:** Use bats-core. It provides all of this out of the box.

## CI Integration Architecture

### Separate CI Job with Matrix

```yaml
  cli-e2e:
    name: CLI E2E (${{ matrix.cli }})
    runs-on: ubuntu-latest
    needs: [build]
    strategy:
      fail-fast: false
      matrix:
        cli: [claude, gemini, opencode, copilot, codex]
    steps:
      - uses: actions/checkout@v4
      - name: Install bats-core
        run: |
          git clone --depth 1 --branch v1.12.0 \
            https://github.com/bats-core/bats-core.git /tmp/bats
          sudo /tmp/bats/install.sh /usr/local
          cd tests/e2e-cli/test_helper
          git clone --depth 1 https://github.com/bats-core/bats-support.git
          git clone --depth 1 https://github.com/bats-core/bats-assert.git
          git clone --depth 1 https://github.com/bats-core/bats-file.git
      - name: Build binaries
        run: cargo build --release -p memory-daemon -p memory-ingest
      - name: Run ${{ matrix.cli }} E2E
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          bats tests/e2e-cli/${{ matrix.cli }}/ \
            --report-formatter junit \
            --output ./test-results/${{ matrix.cli }}/
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: e2e-${{ matrix.cli }}
          path: test-results/
      - uses: test-summary/action@v2
        if: always()
        with:
          paths: test-results/${{ matrix.cli }}/**/*.xml
```

### Progression Strategy

```
Phase 1: continue-on-error: true (informational)
Phase 2: Required for Claude Code only (most stable)
Phase 3: Required for all CLIs with available binaries
```

## Taskfile Integration

```yaml
  cli-e2e:
    desc: "Run CLI E2E tests (all available CLIs)"
    cmds:
      - cargo build --release -p memory-daemon -p memory-ingest
      - export PATH="$PWD/target/release:$PATH" && bats tests/e2e-cli/*/

  cli-e2e-claude:
    desc: "Run CLI E2E tests (Claude Code only)"
    cmds:
      - cargo build --release -p memory-daemon -p memory-ingest
      - export PATH="$PWD/target/release:$PATH" && bats tests/e2e-cli/claude/

  setup-bats:
    desc: "Install bats-core and helpers locally"
    cmds:
      - tests/e2e-cli/setup-bats.sh
```

## New Files Summary

| File/Dir | Type | Purpose |
|----------|------|---------|
| `tests/e2e-cli/test_helper/common.bash` | New | Core shared library |
| `tests/e2e-cli/setup-bats.sh` | New | Install bats + helpers |
| `tests/e2e-cli/fixtures/` | New | Test data and project templates |
| `tests/e2e-cli/{claude,gemini,opencode,copilot,codex}/*.bats` | New | Per-CLI test files |

### Modified Files

| File | Change |
|------|--------|
| `.gitignore` | Add `tests/e2e-cli/test_helper/bats-*`, `test-results/`, `test-artifacts/` |
| `Taskfile.yml` | Add `cli-e2e`, `cli-e2e-claude`, `setup-bats` tasks |
| `.github/workflows/ci.yml` | Add `cli-e2e` matrix job (initially optional) |

## Sources

- [bats-core docs](https://bats-core.readthedocs.io/en/latest/) -- HIGH confidence
- [Claude Code headless docs](https://code.claude.com/docs/en/headless) -- HIGH confidence
- [Gemini CLI headless docs](https://google-gemini.github.io/gemini-cli/docs/cli/headless.html) -- HIGH confidence
- [Codex CLI non-interactive docs](https://developers.openai.com/codex/noninteractive) -- HIGH confidence
- [Copilot CLI docs](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/use-copilot-cli) -- HIGH confidence
- [OpenCode CLI docs](https://opencode.ai/docs/cli/) -- MEDIUM confidence
- [test-summary/action](https://github.com/test-summary/action) -- HIGH confidence
