# Phase 30: Claude Code CLI Harness - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the bats-core E2E framework infrastructure (workspace isolation, daemon lifecycle, CLI wrappers, reporting, CI) plus all Claude Code headless tests (smoke, hook capture, E2E pipeline, negative). This is the foundation phase — everything built here gets reused by phases 31-34.

</domain>

<decisions>
## Implementation Decisions

### Test directory layout
- Tests live at `tests/cli/` (short path, clear purpose)
- Organized by CLI, then by category: `tests/cli/claude-code/smoke.bats`, `tests/cli/claude-code/hooks.bats`, etc.
- Shared helpers in `tests/cli/lib/` (common.bash, cli_wrappers.bash)
- Fixtures in `tests/cli/fixtures/` (JSON payloads, expected outputs)
- Per-run workspaces in `tests/cli/.runs/<run-id>/` (gitignored)

### Daemon lifecycle strategy
- Claude's Discretion: per-.bats-file vs per-CLI-directory daemon scope (pick based on isolation vs startup cost)
- Daemon binary auto-built in setup — harness runs `cargo build -p memory-daemon` if binary is stale
- Daemon failure to start = hard failure for that test file (not skip — daemon issues must be visible)
- Claude's Discretion: health check timeout (reasonable default with configurable override)

### Hook testing approach
- Two-layer proof: marker file on disk for quick checks + gRPC query for full pipeline verification
- Both unit + integration: pipe synthetic JSON stdin to hook scripts (fast, no API key) AND spawn real CLI (when available)
- Test all 7 event types: SessionStart, UserPromptSubmit, PostToolUse, Stop, SubagentStart, SubagentStop, SessionEnd
- Claude's Discretion: dry-run capture mechanism (file vs stdout — pick most testable approach)

### CI matrix & reporting
- New dedicated workflow: `e2e-cli.yml` (separate from cargo tests in ci.yml)
- Full 5-CLI matrix skeleton from Phase 30 — only Claude Code tests exist initially, others skip gracefully
- Missing CLI binary in CI = skip with annotation (shows "skipped" not "failed" in matrix)
- Use GitHub environment `e2e-cli` for API key secrets (ANTHROPIC_API_KEY, GOOGLE_API_KEY, OPENAI_API_KEY, GH_TOKEN_COPILOT, CODEX_API_KEY)
- Claude's Discretion: failure artifact bundle (balance debug info vs artifact size)

### Claude's Discretion
- Daemon scope (per-file vs per-directory)
- Health check timeout value
- Dry-run capture mechanism
- Failure artifact contents (workspace tarball vs logs only)

</decisions>

<specifics>
## Specific Ideas

- Existing `MEMORY_INGEST_DRY_RUN=1` env var in hook scripts enables testing without running daemon — use this for unit-level hook tests
- Reference project `/Users/richardhightower/clients/spillwave/src/rulez_plugin` has hook implementation patterns to study
- Research recommends `timeout`/`gtimeout` wrapping every CLI invocation to prevent CI deadlocks
- Claude Code headless flags: `-p --output-format json`
- bats-core 1.12 with `--report-formatter junit` for CI-parseable output

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 30-claude-code-cli-harness*
*Context gathered: 2026-02-22*
