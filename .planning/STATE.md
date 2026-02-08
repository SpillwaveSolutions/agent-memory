# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-08)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** v2.1 Multi-Agent Ecosystem — OpenCode plugin, Gemini/Copilot adapters, cross-agent sharing

## Current Position

Milestone: v2.1 Multi-Agent Ecosystem
Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-02-08 — Milestone v2.1 started

Progress v2.1: [░░░░░░░░░░░░░░░░░░░░] 0% (0/? plans)

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)

## Accumulated Context

### Key Decisions (from v2.0)

Full decision log in PROJECT.md Key Decisions table.

- Indexes are accelerators, not dependencies (graceful degradation)
- Skills are the control plane (executive function)
- Local embeddings via Candle (no API dependency)
- Tier detection enables fallback chains
- Index lifecycle automation via scheduler

### v2.1 Context

- OpenCode uses plugins (not hooks) — different integration pattern than Claude Code
- Gemini CLI and Copilot CLI have hooks similar to Claude Code
- Cross-agent sharing via agent-tagged events in unified store
- Full Claude parity is the target for all adapters

### Technical Debt (Accepted)

- 3 stub RPCs: GetRankingStatus, PruneVectorIndex, PruneBm25Index (admin features)
- Missing SUMMARY.md files for some phases

## Next Steps

1. Research agent ecosystems (OpenCode plugin API, Gemini/Copilot hook formats)
2. Define requirements
3. Create roadmap

---
*Updated: 2026-02-08 after v2.1 milestone initialization*
