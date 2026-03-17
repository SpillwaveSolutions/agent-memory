---
gsd_state_version: 1.0
milestone: v2.7
milestone_name: Multi-Runtime Portability
status: defining_requirements
stopped_at: null
last_updated: "2026-03-16T00:00:00.000Z"
last_activity: 2026-03-16 — Milestone v2.7 started
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 13
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-16)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.7 Multi-Runtime Portability

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-03-16 — Milestone v2.7 started

Progress: [░░░░░░░░░░] 0% (0/13 plans)

## Decisions

- Installer written in Rust (new workspace crate `memory-installer`)
- Canonical source format is Claude plugin format
- Merge query+setup plugins into single `plugins/memory-plugin/` tree
- Converter trait pattern — one impl per runtime
- Tool name mapping tables modeled after GSD's approach
- Runtime-neutral storage at `~/.config/agent-memory/`
- Old manual adapters archived and replaced by installer output

## Blockers

- None

## Accumulated Context

(Carried from v2.6)

- ActionResult uses tagged enum (status+detail) for JSON clarity
- Storage.db made pub(crate) for cross-module CF access
- Value scoring uses midpoint-distance formula
- EpisodicConfig disabled by default (explicit opt-in)
- Salience enrichment via enrich_with_salience() bridges Storage→ranking metadata
- usearch pinned <2.24 (upstream MSVC + aarch64 bugs)
- Release workflow: protoc v25.1 for aarch64 cross-compile, /MD CRT for Windows

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)
- v2.1 Multi-Agent Ecosystem: Shipped 2026-02-10 (6 phases, 22 plans)
- v2.2 Production Hardening: Shipped 2026-02-11 (4 phases, 10 plans)
- v2.3 Install & Setup Experience: Shipped 2026-02-12 (2 phases, 2 plans)
- v2.4 Headless CLI Testing: Shipped 2026-03-05 (5 phases, 15 plans)
- v2.5 Semantic Dedup & Retrieval Quality: Shipped 2026-03-10 (4 phases, 11 plans)
- v2.6 Cognitive Retrieval: Shipped 2026-03-16 (6 phases, 13 plans)

## Cumulative Stats

- ~50,000+ LOC Rust across 14 crates
- 5 adapter plugins (Claude Code, OpenCode, Gemini CLI, Copilot CLI, Codex CLI)
- 45+ E2E tests + 144 bats CLI tests across 5 CLIs
- 44 phases, 135 plans across 8 milestones

## Session Continuity

**Last Session:** 2026-03-16
**Stopped At:** Milestone v2.7 initialized — defining requirements
**Resume File:** N/A
