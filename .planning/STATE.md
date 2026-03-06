# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-05)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.5 Semantic Dedup & Retrieval Quality

## Current Position

Milestone: v2.5 Semantic Dedup & Retrieval Quality
Phase: 35 of 38 (DedupGate Foundation)
Plan: 0 of 2 in current phase
Status: Ready to plan
Last activity: 2026-03-05 — Roadmap created for v2.5 milestone

Progress: [░░░░░░░░░░] 0% (0/9 plans)

## Decisions

- Store-and-skip-outbox for dedup duplicates (preserve append-only invariant)
- InFlightBuffer as primary dedup source (HNSW contains TOC nodes, not raw events)
- Default similarity threshold 0.85 (conservative for all-MiniLM-L6-v2)
- Structural events bypass dedup entirely
- Max stale penalty bounded at 30% to prevent score collapse
- High-salience kinds (Constraint, Definition, Procedure) exempt from staleness
- DedupConfig replaces NoveltyConfig; [novelty] kept as serde(alias) for backward compat

## Blockers

- None

## Reference Projects

- `/Users/richardhightower/clients/spillwave/src/rulez_plugin` — hook implementation reference

## Performance Metrics

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)
- v2.1 Multi-Agent Ecosystem: Shipped 2026-02-10 (6 phases, 22 plans)
- v2.2 Production Hardening: Shipped 2026-02-11 (4 phases, 10 plans)
- v2.3 Install & Setup Experience: Shipped 2026-02-12 (2 phases, 2 plans)
- v2.4 Headless CLI Testing: Shipped 2026-03-05 (5 phases, 15 plans)

## Cumulative Stats

- 44,917 LOC Rust across 14 crates
- 5 adapter plugins (Claude Code, OpenCode, Gemini CLI, Copilot CLI, Codex CLI)
- 29 E2E tests + 144 bats CLI tests across 5 CLIs
- 34 phases, 111 plans across 6 milestones

## Session Continuity

**Last Session:** 2026-03-05
**Stopped At:** v2.5 roadmap created, Phase 35 ready to plan
**Resume File:** None
