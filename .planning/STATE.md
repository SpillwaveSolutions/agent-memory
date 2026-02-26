# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-22)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.4 Headless CLI Testing — Phase 32 (OpenCode CLI Tests)

## Current Position

Milestone: v2.4 Headless CLI Testing
Phase: 32 of 34 (OpenCode CLI Tests) — IN PROGRESS
**Current Plan:** 2
**Total Plans in Phase:** 2
**Status:** Ready to execute
**Last Activity:** 2026-02-26

**Progress:** [███████░░░] 69%

## Decisions

- Shell-first harness using bats-core 1.12 (no Python/Bun unless validation)
- Real CLI processes in headless mode, not simulated
- Phase 30 builds all framework infra + Claude Code tests; phases 31-34 reuse it
- Codex CLI gets new adapter with commands/skills only (no hooks)
- Hook-dependent tests skipped for Codex
- Existing 29 cargo E2E tests remain as separate test layer
- Codex adapter includes sandbox workaround documentation
- Fixtures match CchEvent struct fields from memory-ingest for compatibility
- Bats helpers installed via git clone in CI (cross-platform reliable)
- Missing CLI test dir triggers skip annotation, not failure
- [Phase 30-01]: Random port selection instead of --port 0 (daemon logs requested addr not bound addr)
- [Phase 30-03]: IPv4 (127.0.0.1) for daemon connectivity: daemon binds 0.0.0.0, not [::1]
- [Phase 30-03]: TCP nc check preferred over grpcurl for daemon health (no grpc.health service)
- [Phase 30-03]: Build-resilient setup: fallback to existing binary when cargo build fails
- [Phase 30-04]: DEFAULT_ENDPOINT changed from [::1] to 127.0.0.1 to match daemon 0.0.0.0 bind address
- [Phase 30-04]: Removed short flag from global --log-level to fix clap conflict with --limit
- [Phase 30-05]: No unit tests for env var read -- validated by E2E bats tests
- [Phase 30]: bash -n not valid for bats files; use bats --count for syntax validation
- [Phase 31-01]: Fixed jq -n to jq -nc in memory-capture.sh (multi-line JSON broke memory-ingest read_line)
- [Phase 31-01]: sleep 2 between hook invocation and gRPC query for background ingest timing
- [Phase 31-02]: Pipeline tests use direct CchEvent format for deterministic storage layer testing
- [Phase 31-02]: Negative tests cover both memory-ingest and memory-capture.sh fail-open paths separately
- [Phase 32-01]: Direct CchEvent ingest pattern for OpenCode (TypeScript plugin not testable from shell)
- [Phase 32-01]: Agent field test verifies ingest acceptance + gRPC storage (query display doesn't show agent metadata)
- [Phase 32]: Negative tests cover memory-ingest fail-open only for OpenCode (TypeScript plugin not shell-testable)

## Blockers

- None

## Reference Projects

- `/Users/richardhightower/clients/spillwave/src/rulez_plugin` — hook implementation reference

## Performance Metrics

| Phase | Duration | Tasks | Files |
|-------|----------|-------|-------|
| 30-02 | 1min | 2 | 11 |
| Phase 30-01 P01 | 3min | 2 tasks | 3 files |
| Phase 30-03 P03 | 11min | 2 tasks | 4 files |
| Phase 30-04 P04 | 17min | 2 tasks | 4 files |
| Phase 30-05 P05 | 5min | 2 tasks | 2 files |
| Phase 30 P06 | 2min | 2 tasks | 2 files |
| Phase 31-01 | 6min | 2 tasks | 10 files |
| Phase 31-02 P02 | 3min | 2 tasks | 2 files |
| Phase 32-01 | 4min | 2 tasks | 9 files |
| Phase 32-02 PP02 | 3min | 2 tasks | 2 files |

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)
- v2.1 Multi-Agent Ecosystem: Shipped 2026-02-10 (6 phases, 22 plans)
- v2.2 Production Hardening: Shipped 2026-02-11 (4 phases, 10 plans)
- v2.3 Install & Setup Experience: Shipped 2026-02-12 (2 phases, 2 plans)

## Cumulative Stats

- 44,912 LOC Rust across 14 crates
- 4 adapter plugins (Claude Code, OpenCode, Gemini CLI, Copilot CLI)
- 4 setup skills (install, configure, verify, troubleshoot)
- 29 E2E tests, dedicated CI job
- Performance benchmark harness with baselines
- 31 phases, 98 plans across 5 milestones

## Session Continuity

**Last Session:** 2026-02-26T07:03:15.553Z
**Stopped At:** Completed 32-02-PLAN.md -- 25/25 OpenCode tests passing (8 smoke + 7 hooks + 5 pipeline + 5 negative)
**Resume File:** None
