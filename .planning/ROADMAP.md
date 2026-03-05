# Roadmap: Agent Memory

## Milestones

- ✅ **v1.0 MVP** — Phases 1-9 (shipped 2026-01-30)
- ✅ **v2.0 Scheduler+Teleport** — Phases 10-17 (shipped 2026-02-07)
- ✅ **v2.1 Multi-Agent Ecosystem** — Phases 18-23 (shipped 2026-02-10)
- ✅ **v2.2 Production Hardening** — Phases 24-27 (shipped 2026-02-11)
- ✅ **v2.3 Install & Setup Experience** — Phases 28-29 (shipped 2026-02-12)
- 🚧 **v2.4 Headless CLI Testing** — Phases 30-34 (in progress)

## Phases

<details>
<summary>v1.0 MVP (Phases 1-9) -- SHIPPED 2026-01-30</summary>

- [x] Phase 1: Foundation (5/5 plans) -- completed 2026-01-29
- [x] Phase 2: TOC Building (3/3 plans) -- completed 2026-01-29
- [x] Phase 3: Grips & Provenance (3/3 plans) -- completed 2026-01-29
- [x] Phase 5: Integration (3/3 plans) -- completed 2026-01-30
- [x] Phase 6: End-to-End (2/2 plans) -- completed 2026-01-30
- [x] Phase 7: CCH Integration (1/1 plan) -- completed 2026-01-30
- [x] Phase 8: CCH Hook Integration (1/1 plan) -- completed 2026-01-30
- [x] Phase 9: Setup Installer Plugin (4/4 plans) -- completed 2026-01-30

See: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>v2.0 Scheduler+Teleport (Phases 10-17) -- SHIPPED 2026-02-07</summary>

- [x] Phase 10: Background Scheduler (4/4 plans) -- completed 2026-02-01
- [x] Phase 10.5: Agentic TOC Search (3/3 plans) -- completed 2026-02-01
- [x] Phase 11: BM25 Teleport Tantivy (4/4 plans) -- completed 2026-02-03
- [x] Phase 12: Vector Teleport HNSW (5/5 plans) -- completed 2026-02-03
- [x] Phase 13: Outbox Index Ingestion (4/4 plans) -- completed 2026-02-03
- [x] Phase 14: Topic Graph Memory (6/6 plans) -- completed 2026-02-05
- [x] Phase 15: Configuration Wizard Skills (5/5 plans) -- completed 2026-02-05
- [x] Phase 16: Memory Ranking Enhancements (5/5 plans) -- completed 2026-02-06
- [x] Phase 17: Agent Retrieval Policy (6/6 plans) -- completed 2026-02-07

See: `.planning/milestones/v2.0-ROADMAP.md`

</details>

<details>
<summary>v2.1 Multi-Agent Ecosystem (Phases 18-23) -- SHIPPED 2026-02-10</summary>

- [x] Phase 18: Agent Tagging Infrastructure (4/4 plans) -- completed 2026-02-08
- [x] Phase 19: OpenCode Commands and Skills (5/5 plans) -- completed 2026-02-09
- [x] Phase 20: OpenCode Event Capture + Unified Queries (3/3 plans) -- completed 2026-02-09
- [x] Phase 21: Gemini CLI Adapter (4/4 plans) -- completed 2026-02-10
- [x] Phase 22: Copilot CLI Adapter (3/3 plans) -- completed 2026-02-10
- [x] Phase 23: Cross-Agent Discovery + Documentation (3/3 plans) -- completed 2026-02-10

See: `.planning/milestones/v2.1-ROADMAP.md`

</details>

<details>
<summary>v2.2 Production Hardening (Phases 24-27) -- SHIPPED 2026-02-11</summary>

- [x] Phase 24: Proto & Service Debt Cleanup (3/3 plans) -- completed 2026-02-11
- [x] Phase 25: E2E Core Pipeline Tests (3/3 plans) -- completed 2026-02-11
- [x] Phase 26: E2E Advanced Scenario Tests (3/3 plans) -- completed 2026-02-11
- [x] Phase 27: CI/CD E2E Integration (1/1 plan) -- completed 2026-02-11

See: `.planning/milestones/v2.2-ROADMAP.md`

</details>

<details>
<summary>v2.3 Install & Setup Experience (Phases 28-29) -- SHIPPED 2026-02-12</summary>

- [x] Phase 28: Install & Configuration Skills + User Guides (1/1 plan) -- completed 2026-02-12
- [x] Phase 29: Performance Benchmarks (1/1 plan) -- completed 2026-02-12

See: `.planning/milestones/v2.3-ROADMAP.md`

</details>

### v2.4 Headless CLI Testing (In Progress)

**Milestone Goal:** Build a shell-based E2E test harness that spawns real CLI processes in headless mode, validating integration behavior across 5 AI coding CLIs with isolated workspaces and matrix reporting.

- [x] **Phase 30: Claude Code CLI Harness** - Build bats-core framework + all Claude Code headless tests
- [x] **Phase 31: Gemini CLI Tests** - Apply harness to Gemini CLI with JSON stdin hooks
- [x] **Phase 32: OpenCode CLI Tests** - Apply harness to OpenCode CLI with headless quirk handling
- [ ] **Phase 33: Copilot CLI Tests** - Apply harness to Copilot CLI with session ID synthesis
- [ ] **Phase 34: Codex CLI Adapter + Tests + Matrix Report** - New adapter, hook-excluded tests, cross-CLI matrix

## Phase Details

### Phase 30: Claude Code CLI Harness
**Goal**: Developers can run isolated shell-based E2E tests for Claude Code that validate the full hook-to-query pipeline, with reusable framework infrastructure for all subsequent CLI phases
**Depends on**: Phase 29 (v2.3 complete)
**Requirements**: HARN-01, HARN-02, HARN-03, HARN-04, HARN-05, HARN-06, HARN-07, CLDE-01, CLDE-02, CLDE-03, CLDE-04
**Success Criteria** (what must be TRUE):
  1. Running `bats tests/cli/claude-code/` executes all Claude Code tests in isolated temp workspaces, each with its own daemon on an OS-assigned port
  2. Tests that require `claude` binary skip gracefully with informative message when binary is not installed
  3. Claude Code hook fires produce events visible via gRPC query in the same test workspace
  4. JUnit XML report is generated and CI matrix job uploads failure artifacts (logs, workspace tarballs)
  5. A `tests/cli/lib/common.bash` library exists that other CLI test phases can source (via `load ../lib/common`) for workspace setup, daemon lifecycle, and CLI wrappers
**Plans:** 6 plans
Plans:
- [x] 30-01-PLAN.md — Common helper library (common.bash + cli_wrappers.bash) + workspace/daemon lifecycle
- [x] 30-02-PLAN.md — Fixture JSON payloads + e2e-cli.yml CI workflow with 5-CLI matrix
- [x] 30-03-PLAN.md — Smoke tests + hook capture tests (all event types via stdin pipe)
- [x] 30-04-PLAN.md — E2E pipeline tests + negative tests (daemon down, malformed, timeout)
- [x] 30-05-PLAN.md — Fix memory-ingest MEMORY_DAEMON_ADDR env var support
- [x] 30-06-PLAN.md — Fix hooks.bats Layer 2 assertions + ROADMAP path correction

### Phase 31: Gemini CLI Tests
**Goal**: Developers can run isolated shell-based E2E tests for Gemini CLI that validate hook capture and the full ingest-to-query pipeline
**Depends on**: Phase 30 (framework)
**Requirements**: GEMI-01, GEMI-02, GEMI-03, GEMI-04
**Success Criteria** (what must be TRUE):
  1. Running `bats tests/cli/gemini/` executes all Gemini tests in isolated workspaces, reusing Phase 30 common helpers
  2. Gemini CLI binary detection and graceful skip works when `gemini` is not installed
  3. Gemini hook handler correctly captures events with agent field set to "gemini" and events are queryable via gRPC
  4. Negative tests verify daemon-down and malformed-input handling without test failures leaking
**Plans:** 2 plans
Plans:
- [x] 31-01-PLAN.md — Gemini fixtures + smoke.bats + hooks.bats (GEMI-01, GEMI-02)
- [x] 31-02-PLAN.md — pipeline.bats + negative.bats (GEMI-03, GEMI-04)

### Phase 32: OpenCode CLI Tests
**Goal**: Developers can run isolated shell-based E2E tests for OpenCode CLI, handling its less mature headless mode with appropriate skip/warn patterns
**Depends on**: Phase 30 (framework)
**Requirements**: OPEN-01, OPEN-02, OPEN-03, OPEN-04
**Success Criteria** (what must be TRUE):
  1. Running `bats tests/cli/opencode/` executes all OpenCode tests in isolated workspaces, reusing Phase 30 common helpers
  2. OpenCode invocation uses `opencode run --format json` and timeout guards prevent hangs from headless mode quirks
  3. OpenCode hook capture produces events with agent field "opencode" queryable via gRPC pipeline test
  4. Negative tests cover daemon-down and timeout scenarios specific to OpenCode's headless behavior
**Plans:** 2 plans
Plans:
- [ ] 32-01-PLAN.md — OpenCode fixtures + run_opencode wrapper + smoke.bats + hooks.bats (OPEN-01, OPEN-02)
- [ ] 32-02-PLAN.md — pipeline.bats + negative.bats (OPEN-03, OPEN-04)

### Phase 33: Copilot CLI Tests
**Goal**: Developers can run isolated shell-based E2E tests for Copilot CLI that validate session ID synthesis and the hook-to-query pipeline
**Depends on**: Phase 30 (framework)
**Requirements**: CPLT-01, CPLT-02, CPLT-03, CPLT-04
**Success Criteria** (what must be TRUE):
  1. Running `bats tests/cli/copilot/` executes all Copilot tests in isolated workspaces, reusing Phase 30 common helpers
  2. Copilot binary detection uses correct binary name and `--yes --allow-all-tools` prevents interactive prompts
  3. Copilot session ID synthesis produces deterministic session IDs from workspace context, verified in captured events
  4. Negative tests verify daemon-down and malformed-input handling for Copilot-specific edge cases
**Plans**: TBD

### Phase 34: Codex CLI Adapter + Tests + Matrix Report
**Goal**: Codex CLI adapter exists with commands and skills (no hooks), Codex headless tests pass with hook tests skipped, and a cross-CLI matrix report aggregates results from all 5 CLIs
**Depends on**: Phase 30 (framework), Phases 31-33 (all CLI tests for matrix)
**Requirements**: CDEX-01, CDEX-02, CDEX-03, CDEX-04, CDEX-05
**Success Criteria** (what must be TRUE):
  1. A Codex CLI adapter directory exists under `adapters/codex-cli/` with commands, skills, and sandbox workaround documentation (no hook handler)
  2. Running `bats tests/cli/codex/` executes Codex tests with hook-dependent scenarios explicitly skipped and annotated
  3. Codex command invocation tests use `codex exec -q --full-auto` with timeout guards
  4. A matrix report script aggregates JUnit XML from all 5 CLIs into a CLI x scenario pass/fail/skipped summary viewable in CI
**Plans**: TBD

## Progress

| Phase | Milestone | Plans | Status | Completed |
|-------|-----------|-------|--------|-----------|
| 1-9 | v1.0 | 20/20 | Complete | 2026-01-30 |
| 10-17 | v2.0 | 42/42 | Complete | 2026-02-07 |
| 18-23 | v2.1 | 22/22 | Complete | 2026-02-10 |
| 24-27 | v2.2 | 10/10 | Complete | 2026-02-11 |
| 28-29 | v2.3 | 2/2 | Complete | 2026-02-12 |
| 30 | v2.4 | 6/6 | Complete | 2026-02-25 |
| 31 | v2.4 | 2/2 | Complete | 2026-02-25 |
| 32 | v2.4 | 2/2 | Complete | 2026-02-26 |
| 33 | v2.4 | 0/TBD | Not started | - |
| 34 | v2.4 | 0/TBD | Not started | - |

---

*Updated: 2026-02-26 after Phase 32 execution complete*
