# Phase 56: Honest Benchmarks - Context

**Gathered:** 2026-08-30
**Status:** In execution
**Source:** docs/plans/v3.1-make-it-true-plan.md

## Phase Boundary

A benchmark story that survives ten minutes of hostile review — or no
benchmark story at all. No new retrieval capabilities.

## What was wrong (v3.0)

- `recall_at_5` == `accuracy` (one bool per test / test count)
- `compression_ratio` summed **path-string** lengths, not file contents
- `memory add`/`search` failures swallowed (`let _ =`); dead daemon → accuracy 0.0
- Shared store across tests and LOCOMO conversations
- 4 tests / ~60 lines
- LOCOMO adapter invented `conversation_id`/`turns`/`questions`; download URL 404s
- Substring scoring labeled as if it were LOCOMO

## Decisions

- Real recall@k against `relevant` labels; omit the metric when empty
- Compression reads setup file contents (`ceil(chars/4)`)
- Fail loud on CLI errors
- Mock backend: isolated store per test / per LOCOMO conversation (tested)
- CLI backend: fail loud; isolation is "fresh daemon" (operator); documented
- ≥25 fixtures across temporal / multi / compress with distractor sessions
- Real locomo10.json schema; numeric answers; category 1–5 map
- Substring metric named `context_hit_rate`; never `--compare`
- LLM-as-judge is the only `locomo_llm_judge` path (temp 0, model recorded)
- Decision gate: HOLD comparison marketing until a real llm-judge artifact exists
- CI smoke runs the 1-conversation fixture with mock judge (not `--help`)
