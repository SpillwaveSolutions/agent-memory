---
gsd_state_version: 1.0
milestone: v2.5
milestone_name: Semantic Dedup & Retrieval Quality
status: completed
stopped_at: Completed 38-02 stale filter E2E tests (TEST-02)
last_updated: "2026-03-10T03:46:51.065Z"
last_activity: 2026-03-10 — Completed 38-02 Stale Filter E2E Tests (TEST-02 closed)
progress:
  total_phases: 4
  completed_phases: 4
  total_plans: 11
  completed_plans: 11
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-10)

**Core value:** Agent can answer "what were we talking about last week?" without scanning everything
**Current focus:** Planning next milestone

## Current Position

Milestone: v2.5 Semantic Dedup & Retrieval Quality — SHIPPED
Status: Milestone archived, ready for next milestone
Last activity: 2026-03-10 — Archived v2.5 milestone

Progress: [██████████] 100% (11/11 plans) — SHIPPED

## Decisions

- Store-and-skip-outbox for dedup duplicates (preserve append-only invariant)
- InFlightBuffer as primary dedup source (HNSW contains TOC nodes, not raw events)
- Default similarity threshold 0.85 (conservative for all-MiniLM-L6-v2)
- Structural events bypass dedup entirely
- Max stale penalty bounded at 30% to prevent score collapse
- High-salience kinds (Constraint, Definition, Procedure) exempt from staleness
- DedupConfig replaces NoveltyConfig; [novelty] kept as serde(alias) for backward compat
- Cosine similarity as dot product (vectors pre-normalized by CandleEmbedder)
- NoveltyConfig kept as type alias for backward compat (not deprecated)
- InFlightBufferIndex uses threshold 0.0 in find_similar; caller does threshold comparison
- push_to_buffer is explicit (not auto-push in should_store) to avoid pushing for failed stores
- std::sync::RwLock for InFlightBuffer (not tokio) since operations are sub-microsecond
- CandleEmbedderAdapter uses spawn_blocking for CPU-bound embed calls
- DedupResult carries embedding alongside should_store for post-store buffer push
- deduplicated field in IngestEventResponse deferred to proto update (36-02)
- events_skipped in GetDedupStatus = total_stored minus stored_novel (all fail-open cases)
- buffer_size hardcoded to 0 in GetDedupStatus (buffer len exposure deferred)
- CompositeVectorIndex searches all backends, returns highest-scoring result
- HnswIndexAdapter is_ready returns false when HNSW empty (no false positives)
- Daemon falls back to buffer-only when HNSW directory absent
- All Observations get uniform decay regardless of salience score
- memory_kind defaults to "observation" for all retrieval layers
- Dot product used as cosine similarity for supersession (vectors pre-normalized)
- Supersession iterates newest-first, breaks on first match (no transitivity)
- StalenessConfig propagated via with_services parameter (not global state)
- All MemoryServiceImpl with_* constructors accept StalenessConfig (no defaults in production)
- ULID-based event_ids required for proto events in E2E tests (storage validates format)
- E2E staleness test compares enabled-vs-disabled scores (BM25 TF-IDF varies across docs)

## Blockers

- None

## Reference Projects

- `/Users/richardhightower/clients/spillwave/src/rulez_plugin` — hook implementation reference

## Performance Metrics

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 35-01 | 1 | 3min | 3min |
| 35-02 | 1 | 3min | 3min |
| 36-01 | 1 | 4min | 4min |
| 36-02 | 1 | 6min | 6min |
| 36-03 | 1 | 4min | 4min |
| 37-01 | 1 | 5min | 5min |
| 37-02 | 1 | 8min | 8min |
| 37-03 | 1 | 4min | 4min |
| 38-01 | 1 | 3min | 3min |
| 38-02 | 1 | 3min | 3min |
| 38-03 | 1 | 2min | 2min |

## Milestone History

See: .planning/MILESTONES.md for complete history

- v1.0.0 MVP: Shipped 2026-01-30 (8 phases, 20 plans)
- v2.0.0 Scheduler+Teleport: Shipped 2026-02-07 (9 phases, 42 plans)
- v2.1 Multi-Agent Ecosystem: Shipped 2026-02-10 (6 phases, 22 plans)
- v2.2 Production Hardening: Shipped 2026-02-11 (4 phases, 10 plans)
- v2.3 Install & Setup Experience: Shipped 2026-02-12 (2 phases, 2 plans)
- v2.4 Headless CLI Testing: Shipped 2026-03-05 (5 phases, 15 plans)
- v2.5 Semantic Dedup & Retrieval Quality: Shipped 2026-03-10 (4 phases, 11 plans)

## Cumulative Stats

- 48,282 LOC Rust across 14 crates
- 5 adapter plugins (Claude Code, OpenCode, Gemini CLI, Copilot CLI, Codex CLI)
- 39 E2E tests + 144 bats CLI tests across 5 CLIs
- 38 phases, 122 plans across 7 milestones

## Session Continuity

**Last Session:** 2026-03-10
**Stopped At:** v2.5 milestone archived
**Resume File:** N/A — start next milestone with /gsd:new-milestone
