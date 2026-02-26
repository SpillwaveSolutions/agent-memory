# Research Summary: Headless CLI E2E Testing Harness

**Domain:** Shell-based E2E integration testing for 5 AI coding CLI tools
**Researched:** 2026-02-22
**Overall confidence:** HIGH

## Executive Summary

The v2.4 milestone adds a shell-first E2E test harness that spawns real CLI processes (Claude Code, Gemini CLI, OpenCode, Copilot CLI, Codex CLI) in headless mode. Research confirms all 5 CLIs have workable non-interactive modes, though with different flag patterns and maturity levels. The stack recommendation is bats-core 1.12 as the test framework, producing native JUnit XML for CI integration. No new Rust crates are needed -- this is a shell-only layer that sits above the existing cargo E2E tests.

All 5 CLIs support non-interactive execution: Claude Code uses `-p` with `--output-format json`, Gemini CLI uses positional args with `--output-format json`, OpenCode uses `-p -q -f json`, Copilot CLI uses `-p --yes --allow-all-tools`, and Codex CLI uses `codex exec -q --full-auto`. The critical finding is that Codex CLI has NO hook/extension system, so hook-dependent tests must be skipped for it. OpenCode's headless mode is the least mature (MEDIUM confidence) and will likely need the most workaround effort.

The testing strategy centers on isolated temp-directory workspaces per test file, each with its own memory-daemon instance on an OS-assigned port. bats-core's `setup_file`/`teardown_file` lifecycle hooks manage workspace creation, daemon startup, and cleanup. Failed test workspaces are preserved as tar.gz artifacts for CI debugging.

CI integration uses a GitHub Actions matrix (5 CLIs x test categories) with `fail-fast: false` so all CLIs report even when one fails. bats-core's native JUnit formatter produces XML reports consumed by test-summary/action for PR check rendering.

## Key Findings

**Stack:** bats-core 1.12 + bats-assert/support/file helpers, jq for JSON validation, timeout/gtimeout for process guards. No Python, no Bun, no new Rust deps.

**Architecture:** Shell test layer above existing cargo E2E tests. Each .bats file gets isolated workspace with its own daemon. Common helpers in test_helper/common.bash.

**Critical pitfall:** CLI processes hanging in "non-interactive" mode due to TTY detection bugs, permission prompts, or auth failures. Every CLI invocation must use `timeout` as a kill guard.

## Implications for Roadmap

Based on research, suggested phase structure:

1. **Phase: Codex Adapter** - Build the new Codex CLI adapter (commands + skills only, no hooks)
   - Addresses: New adapter requirement
   - Avoids: Blocking on harness for adapter work
   - Rationale: Small, independent deliverable. Can be validated with existing cargo E2E patterns.

2. **Phase: Claude Code Harness (Framework Phase)** - Build the bats-core infrastructure using Claude Code as the first CLI
   - Addresses: Workspace isolation, daemon lifecycle, common helpers, CI integration, reporting
   - Avoids: Over-engineering by building framework against a well-understood CLI
   - Rationale: Claude Code has the most mature headless mode (HIGH confidence) and existing CCH hooks. All framework patterns are proven here before applying to other CLIs.

3. **Phase: Gemini CLI Tests** - Apply framework to Gemini CLI
   - Addresses: Gemini-specific hook testing (JSON stdin, `{}` stdout), `--yolo --sandbox=false` flags
   - Avoids: Gemini sandbox complexity (disable sandbox for local tests)

4. **Phase: OpenCode CLI Tests** - Apply framework to OpenCode CLI
   - Addresses: OpenCode headless quirks (newer feature, less stable)
   - Avoids: Blocking on OpenCode maturity; skip/warn patterns for rough edges
   - Rationale: OpenCode is MEDIUM confidence; schedule later to benefit from any upstream fixes.

5. **Phase: Copilot CLI Tests** - Apply framework to Copilot CLI
   - Addresses: Copilot session ID synthesis, `--yes --allow-all-tools` for non-interactive
   - Avoids: MCP/permission hang issues (force `--allow-all-tools`)

6. **Phase: Codex CLI Tests + Matrix Report** - Final CLI tests (no hooks) + cross-CLI matrix reporting
   - Addresses: Codex commands-only testing, aggregate CLI x scenario matrix
   - Avoids: Testing hooks that do not exist

**Phase ordering rationale:**
- Codex adapter first because it is a Rust deliverable independent of shell harness
- Claude Code second because it builds the framework; all subsequent phases reuse it
- Remaining CLIs ordered by confidence level (HIGH first, MEDIUM last)
- Matrix reporting last because it aggregates results from all prior phases

**Research flags for phases:**
- Phase 2 (Claude Code Harness): Standard patterns, unlikely to need deeper research
- Phase 4 (OpenCode): Likely needs deeper research -- headless mode is newer and less documented
- Phase 5 (Copilot): May need research on session ID synthesis in parallel test execution

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack (bats-core) | HIGH | Well-established, version verified, JUnit output confirmed |
| CLI Headless Modes | HIGH (4/5) | Claude, Gemini, Copilot, Codex verified via official docs. OpenCode is MEDIUM. |
| Features | HIGH | Derived from existing adapter code and project requirements |
| Architecture | HIGH | Standard shell test patterns with bats-core lifecycle hooks |
| Pitfalls | HIGH | CLI hang issues documented in GitHub issues; timeout mitigation is proven |

## Gaps to Address

- OpenCode headless mode maturity -- test early in development, file upstream issues if needed
- Copilot CLI parallel test isolation -- session ID synthesis via temp files may need per-test CWD hash
- API key requirements per CLI -- which CLIs need valid API keys for plugin listing vs. actual LLM calls?
- Gemini CLI ANSI output in headless mode -- does `--output-format json` suppress all ANSI codes?
- Copilot CLI JSON output -- no `--output-format json` found; may need to parse text output

---
*Research complete. Ready for roadmap creation.*
