# Phase 34: Codex CLI Adapter + Tests + Matrix Report - Research

**Researched:** 2026-03-05
**Domain:** OpenAI Codex CLI integration, Bats E2E testing, JUnit XML aggregation
**Confidence:** HIGH

## Summary

OpenAI Codex CLI (v0.111.0 as of 2026-03-05) is a terminal-based coding agent that supports headless execution via `codex exec`, custom skills via `SKILL.md` files, and sandbox-controlled execution. Critically, **Codex CLI does NOT have a hooks/event capture system** -- a GitHub discussion (#2150) confirms hooks are requested but not yet implemented. This means the Codex adapter will have commands and skills only, with no hook handler. Hook-dependent tests must be explicitly skipped.

The existing project has a well-established pattern across 4 CLIs (Claude Code, Gemini, OpenCode, Copilot) with consistent test structure: `smoke.bats`, `hooks.bats`, `pipeline.bats`, `negative.bats`. The Codex adapter follows a simpler variant since there are no hooks to test. The cross-CLI matrix report aggregates JUnit XML artifacts already produced by the CI workflow (`e2e-cli.yml`) which already includes `codex` in its matrix.

**Primary recommendation:** Create the Codex adapter at `adapters/codex-cli/` with skills in `.codex/skills/` format, write Codex bats tests with hook scenarios explicitly skipped, add a `run_codex()` wrapper to `cli_wrappers.bash`, create Codex fixtures, and build a matrix report script that downloads/parses JUnit XML from all 5 CLI test runs.

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| Codex CLI | 0.111.0 | Target CLI for adapter | Latest stable release |
| Bats-core | 1.x | Test framework | Already used for all 4 CLI test suites |
| JUnit XML | N/A | Test report format | Bats `--report-formatter junit` already configured in CI |
| jq | 1.6+ | JSON processing in scripts | Already a project dependency |
| xmlstarlet or xsltproc | System | XML parsing for matrix report | Standard Unix tools for JUnit XML aggregation |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `timeout`/`gtimeout` | Codex exec timeout guards | Every headless codex invocation |
| GitHub Actions artifacts | JUnit XML collection | Matrix report aggregation in CI |
| bash/awk/sed | Matrix report formatting | Parsing JUnit XML into summary table |

## Architecture Patterns

### Codex Adapter Directory Structure
```
adapters/codex-cli/
├── .codex/
│   └── skills/
│       ├── memory-query/
│       │   ├── SKILL.md              # Core query skill (name + description frontmatter)
│       │   └── references/
│       │       └── command-reference.md
│       ├── retrieval-policy/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── topic-graph/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── bm25-search/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       └── vector-search/
│           ├── SKILL.md
│           └── references/
│               └── command-reference.md
├── SANDBOX-WORKAROUND.md             # Documents sandbox/network config for daemon access
├── README.md
└── .gitignore
```

### Codex Skill SKILL.md Format (Verified from Official Docs)
```yaml
---
name: memory-query
description: >
  Search and retrieve conversation memories from agent-memory daemon.
  Activate when user asks to recall, search, find previous sessions,
  or asks "what did we discuss". Do NOT activate for general coding tasks.
---

## Instructions

[Skill body with memory-daemon CLI commands]
```

Skills are discovered from:
- `$CWD/.agents/skills` (repo-level, new convention)
- `$CWD/.codex/skills` (legacy repo-level)
- `$HOME/.agents/skills` (user-level)
- `$HOME/.codex/skills` (user-level legacy)

**Note:** The Codex skill discovery paths changed recently. The current canonical path is `.agents/skills/` but `.codex/skills/` still works. Use `.codex/skills/` to match the adapter directory name convention.

### Codex Test Directory Structure
```
tests/cli/codex/
├── smoke.bats          # Binary detection, codex exec basics
├── pipeline.bats       # Ingest -> query cycle (same as other CLIs)
├── negative.bats       # Fail-open, malformed input (hooks skipped)
└── hooks.bats          # ALL tests skipped with annotation (no hook support)
```

### Codex Fixtures
```
tests/cli/fixtures/codex/
├── session-start.json   # {"hook_event_name":"SessionStart","agent":"codex",...}
├── user-prompt.json
├── pre-tool-use.json
├── post-tool-use.json
├── session-end.json
└── malformed.json
```

### Matrix Report Script
```
scripts/
└── cli-matrix-report.sh   # Aggregates JUnit XML from all 5 CLIs
```

### Pattern: Codex Headless Invocation

Codex CLI uses `codex exec` for headless mode (NOT `codex exec -q --full-auto` -- see pitfalls below):

```bash
# Correct invocation (verified from official docs)
codex exec --full-auto "echo hello"

# With JSON output
codex exec --full-auto --json "echo hello"

# With sandbox and timeout
timeout 120s codex exec --full-auto -s workspace-write "echo hello"
```

**Key flags:**
- `--full-auto` applies `workspace-write` sandbox + `on-request` approvals (automation preset)
- `--json` / `--experimental-json` outputs newline-delimited JSON events
- `-s workspace-write` / `-s danger-full-access` controls sandbox level
- `-o <file>` writes final assistant message to file (useful for assertions)
- No `-q` / `--quiet` flag exists in official docs

### Pattern: Hook Tests Skipped with Annotation

```bash
@test "hooks: SessionStart event capture (SKIPPED - Codex has no hook system)" {
  skip "Codex CLI does not support hooks (see GitHub Discussion #2150)"
}
```

### Pattern: Sandbox Workaround for Daemon Access

Codex runs commands in a sandbox by default. For memory-daemon connectivity:

```toml
# .codex/config.toml (project-level) or ~/.codex/config.toml (user-level)
[sandbox_workspace_write]
network_access = true    # Required for gRPC to memory-daemon
```

**macOS caveat:** On macOS, `network_access = true` in config.toml may be silently ignored by the Seatbelt sandbox. The workaround is to use `--sandbox danger-full-access` or run with `--dangerously-bypass-approvals-and-sandbox`. This MUST be documented in `SANDBOX-WORKAROUND.md`.

### Pattern: CLI Wrapper for Codex

Add to `cli_wrappers.bash`:

```bash
run_codex() {
    # Usage: run_codex <prompt> [extra args...]
    # Wraps codex exec in headless mode with timeout and JSON output.
    local test_stderr="${TEST_WORKSPACE:-/tmp}/codex_stderr.log"
    export TEST_STDERR="${test_stderr}"

    local cmd=("codex" "exec" "--full-auto" "--json" "$@")

    if [[ -n "${TIMEOUT_CMD}" ]]; then
        "${TIMEOUT_CMD}" "${CLI_TIMEOUT}s" "${cmd[@]}" 2>"${test_stderr}"
    else
        "${cmd[@]}" 2>"${test_stderr}"
    fi
}
```

### Pattern: JUnit XML Aggregation

Bats produces JUnit XML via `--report-formatter junit --output <dir>`. The CI already uploads artifacts as `junit-<cli>-<os>`. The matrix report script:

1. Downloads all `junit-*` artifacts (or reads local files)
2. Parses each XML for test counts (pass/fail/skip)
3. Extracts per-test-case results
4. Outputs a CLI x scenario matrix (markdown table)

JUnit XML structure from bats:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="smoke.bats" tests="8" failures="0" skipped="2" time="5.123">
    <testcase name="memory-daemon binary exists and is executable" time="0.01"/>
    <testcase name="codex binary detection (skip if not installed)">
      <skipped message="Skipping: Codex CLI not installed"/>
    </testcase>
  </testsuite>
</testsuites>
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JUnit XML parsing | Custom XML parser | `xmlstarlet` or `python3 -c "import xml.etree..."` | XML parsing edge cases, encoding |
| Timeout wrapping | Custom process management | `timeout`/`gtimeout` (already in cli_wrappers) | Signal handling, cleanup |
| Test skip annotations | Custom skip logic | Bats `skip "reason"` built-in | Consistent with existing tests |
| Codex binary detection | Custom PATH search | `require_cli codex "Codex CLI"` (existing helper) | Already standardized |
| JUnit XML generation | Custom reporter | `bats --report-formatter junit` | Already configured in CI |

## Common Pitfalls

### Pitfall 1: `-q` Flag Does Not Exist
**What goes wrong:** Phase requirements mention `codex exec -q --full-auto` but `-q`/`--quiet` is NOT a documented Codex CLI flag.
**Why it happens:** Confusion with other CLIs or outdated information.
**How to avoid:** Use `codex exec --full-auto` without `-q`. For quiet output, redirect stderr. The `--json` flag controls output format.
**Warning signs:** `codex: error: unrecognized arguments: -q`
**Confidence:** HIGH (verified against official CLI reference at developers.openai.com/codex/cli/reference/)

### Pitfall 2: macOS Seatbelt Ignores network_access Config
**What goes wrong:** Setting `network_access = true` in config.toml works on Linux (Landlock) but is silently ignored on macOS (Seatbelt sandbox).
**Why it happens:** macOS Seatbelt sandbox implementation doesn't read the TOML config for network policy.
**How to avoid:** Document the workaround in `SANDBOX-WORKAROUND.md`. For testing, use `--sandbox danger-full-access` on macOS.
**Warning signs:** gRPC connection refused on macOS but works on Linux.
**Confidence:** MEDIUM (from GitHub issue #5041 and SmartScope blog, not yet verified against latest release)

### Pitfall 3: Codex Skills Path Convention Change
**What goes wrong:** Skills placed in `.codex/skills/` may not be discovered if Codex expects `.agents/skills/`.
**Why it happens:** Codex is transitioning from `.codex/` to `.agents/` as the canonical config directory.
**How to avoid:** Document both paths. Test with the path that Codex actually discovers. The official docs list `.agents/skills` as the primary repo-level path.
**Warning signs:** Skills not auto-activating when they should.
**Confidence:** MEDIUM (official docs show `.agents/skills` as primary, but `.codex/` still documented)

### Pitfall 4: No Hook System Means No Event Capture
**What goes wrong:** Attempting to create a hook handler for Codex -- no such system exists.
**Why it happens:** Every other adapter (Claude Code, Gemini, OpenCode, Copilot) has hooks.
**How to avoid:** The adapter explicitly documents this limitation. Hook-dependent tests are skipped with clear annotations referencing GitHub Discussion #2150.
**Warning signs:** N/A -- this is a known constraint, not a bug.
**Confidence:** HIGH (confirmed via official docs, GitHub discussion, and changelog)

### Pitfall 5: Matrix Report Artifact Timing in CI
**What goes wrong:** Matrix report job runs before all CLI test jobs complete, producing incomplete report.
**Why it happens:** GitHub Actions `needs` dependency not correctly configured.
**How to avoid:** The matrix report must run in a separate job with `needs: [e2e-cli]` to wait for all matrix entries.
**Warning signs:** Report shows 0 tests for some CLIs.
**Confidence:** HIGH (standard GitHub Actions pattern)

### Pitfall 6: Missing Test Directory Triggers Skip, Not Failure
**What goes wrong:** If `tests/cli/codex/` doesn't exist, CI should skip gracefully.
**Why it happens:** The existing CI workflow already handles this with `if [ -d "tests/cli/${{ matrix.cli }}" ]`.
**How to avoid:** This is already handled correctly in `e2e-cli.yml` lines 79-85. No change needed.
**Confidence:** HIGH (verified in existing CI workflow)

## Code Examples

### Codex Smoke Test Pattern (smoke.bats)
```bash
#!/usr/bin/env bats
# Codex CLI smoke tests -- binary detection, basic ingest, daemon connectivity
#
# Tests 1-6: Always run (require only cargo-built binaries + daemon)
# Tests 7-8: Require codex CLI binary (skip gracefully if not installed)

load '../lib/common'
load '../lib/cli_wrappers'

setup_file() {
  build_daemon_if_needed
  setup_workspace
  start_daemon
}

teardown_file() {
  stop_daemon
  teardown_workspace
}

@test "memory-daemon binary exists and is executable" {
  [ -f "$MEMORY_DAEMON_BIN" ]
  [ -x "$MEMORY_DAEMON_BIN" ]
}

@test "codex binary detection works (skip if not installed)" {
  require_cli codex "Codex CLI"
  run codex --version
  [ "$status" -eq 0 ]
}

@test "codex headless mode produces output (skip if not installed)" {
  require_cli codex "Codex CLI"
  run run_codex "echo hello"
  if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    skip "Codex headless mode timed out"
  fi
  [ "$status" -eq 0 ]
  [[ -n "$output" ]]
}
```

### Codex Hooks Test Pattern (all skipped)
```bash
#!/usr/bin/env bats
# Codex CLI hook tests -- ALL SKIPPED
# Codex CLI does not support a hooks/event capture system.
# See: https://github.com/openai/codex/discussions/2150

load '../lib/common'
load '../lib/cli_wrappers'

@test "hooks: SessionStart event capture (SKIPPED - no hook system)" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}

@test "hooks: UserPromptSubmit event capture (SKIPPED - no hook system)" {
  skip "Codex CLI does not support hooks (GitHub Discussion #2150)"
}
# ... etc for all hook event types
```

### Matrix Report Script Pattern
```bash
#!/usr/bin/env bash
# cli-matrix-report.sh -- Aggregate JUnit XML from all 5 CLIs into a summary table
# Usage: ./scripts/cli-matrix-report.sh [junit-dir]
# Expects: junit-dir/junit-<cli>-<os>/report.xml

set -euo pipefail

JUNIT_DIR="${1:-.}"
CLIS=("claude-code" "gemini" "opencode" "copilot" "codex")

echo "| Scenario | claude-code | gemini | opencode | copilot | codex |"
echo "|----------|-------------|--------|----------|---------|-------|"

# Parse each CLI's JUnit XML and build matrix rows
# Use python3 xml.etree.ElementTree for portable XML parsing
python3 - "$JUNIT_DIR" <<'PYEOF'
import sys, os, xml.etree.ElementTree as ET
from collections import defaultdict

junit_dir = sys.argv[1]
clis = ["claude-code", "gemini", "opencode", "copilot", "codex"]

# Collect all test cases per CLI
results = {}  # cli -> {test_name -> status}
for cli in clis:
    results[cli] = {}
    for pattern in [f"junit-{cli}-*"]:
        import glob
        for xml_dir in glob.glob(os.path.join(junit_dir, pattern)):
            xml_path = os.path.join(xml_dir, "report.xml")
            if not os.path.exists(xml_path):
                continue
            tree = ET.parse(xml_path)
            for tc in tree.iter("testcase"):
                name = tc.get("name", "unknown")
                if tc.find("failure") is not None:
                    results[cli][name] = "FAIL"
                elif tc.find("skipped") is not None:
                    results[cli][name] = "SKIP"
                else:
                    results[cli][name] = "PASS"

# Collect all unique test names
all_tests = sorted(set(n for cli_r in results.values() for n in cli_r))

for test in all_tests:
    row = [test]
    for cli in clis:
        status = results[cli].get(test, "-")
        row.append(status)
    print("| " + " | ".join(row) + " |")
PYEOF
```

### Codex SKILL.md Example (memory-query)
```yaml
---
name: memory-query
description: >
  Search and retrieve conversation memories stored by agent-memory daemon.
  Use when the user asks to recall, search, find previous sessions, look up
  what was discussed, or retrieve conversation history. Do NOT use for
  general coding, file editing, or non-memory tasks.
---

## Memory Query

You have access to the `memory-daemon` CLI for searching conversation history.

### Commands

**Search conversations:**
```bash
memory-daemon retrieval route "<query>" [--agent codex]
```

**Recent events:**
```bash
memory-daemon query events --from <unix_ms> --to <unix_ms> --limit 10
```

**Browse topic tree:**
```bash
memory-daemon query root
```

### Tips
- Default searches span ALL agents (Claude, OpenCode, Gemini, Copilot, Codex)
- Add `--agent codex` to filter to Codex sessions only
- Use `retrieval status` to check available search tiers
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `.codex/skills/` path | `.agents/skills/` path | Codex 0.110+ | Skills discovery path changing |
| No plugin system | Plugin system (0.110.0) | 2026-02 | Skills can be installed via plugin marketplace |
| No hooks | Still no hooks (Discussion #2150) | N/A | Event capture not possible for Codex |
| Basic notify config | Notify on agent-turn-complete | Codex 0.100+ | Limited event awareness, not hooks |

**Deprecated/outdated:**
- The `-q` / `--quiet` flag mentioned in requirements does NOT exist in Codex CLI. Use `--json` for structured output or redirect stderr for quieter execution.

## Open Questions

1. **Skills path: `.codex/skills/` vs `.agents/skills/`?**
   - What we know: Official docs list `.agents/skills` as primary REPO scope path. `.codex/skills` still works at USER scope.
   - What's unclear: Whether `.codex/skills/` is deprecated at repo level or still supported.
   - Recommendation: Use `.codex/skills/` for the adapter directory (matches existing project convention of `.claude/`, `.gemini/`, etc.) but test with `.agents/skills/` as well. Document both paths.

2. **macOS sandbox network_access reliability?**
   - What we know: GitHub issues report Seatbelt ignoring config.toml network_access on macOS.
   - What's unclear: Whether this was fixed in v0.111.0.
   - Recommendation: Document the workaround (`--sandbox danger-full-access` for macOS). Test on both platforms in CI.

3. **Matrix report: local script vs CI-only?**
   - What we know: JUnit XML artifacts are uploaded per-CLI per-OS in CI.
   - What's unclear: Whether the report should work locally (reading local bats output) or only in CI (downloading artifacts).
   - Recommendation: Build the script to accept a directory of JUnit XMLs. It works locally (point at `.runs/`) and in CI (after artifact download). Add a CI job that runs after all test matrix entries complete.

## Sources

### Primary (HIGH confidence)
- [Codex CLI Command Line Reference](https://developers.openai.com/codex/cli/reference/) -- verified no `-q` flag, confirmed `codex exec --full-auto --json` pattern
- [Codex CLI Features](https://developers.openai.com/codex/cli/features/) -- sandbox modes, MCP support, skills
- [Codex CLI Skills Documentation](https://developers.openai.com/codex/skills/) -- SKILL.md format, discovery paths, frontmatter
- [Codex Configuration Reference](https://developers.openai.com/codex/config-reference/) -- config.toml structure, sandbox settings
- [Codex Security/Sandbox](https://developers.openai.com/codex/security/) -- Seatbelt (macOS) vs Landlock (Linux), network access control
- [Codex Changelog](https://developers.openai.com/codex/changelog/) -- v0.111.0 current, v0.110.0 added plugin system

### Secondary (MEDIUM confidence)
- [GitHub Discussion #2150: Hooks Feature Request](https://github.com/openai/codex/discussions/2150) -- confirms hooks not yet implemented, basic notify only
- [SmartScope: Fix Codex CLI Network Restrictions](https://smartscope.blog/en/generative-ai/chatgpt/codex-network-restrictions-solution/) -- macOS sandbox workaround details
- [Bats-core Documentation: JUnit Formatter](https://bats-core.readthedocs.io/en/stable/usage.html) -- `--report-formatter junit` output format

### Tertiary (LOW confidence)
- [Codex Headless Mode DeepWiki](https://deepwiki.com/openai/codex/4.2-headless-execution-mode-(codex-exec)) -- third-party documentation, cross-verified with official reference

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - verified against official Codex docs and existing project patterns
- Architecture: HIGH - adapter structure follows established patterns from 4 existing adapters; Codex skill format verified from official docs
- Pitfalls: HIGH for no-hooks and no-`-q` flag (verified); MEDIUM for macOS sandbox issue (GitHub issues, not yet personally verified on latest)
- Matrix report: MEDIUM - JUnit XML format verified, aggregation script is standard but untested

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (Codex CLI releases weekly; check changelog for hooks addition)
