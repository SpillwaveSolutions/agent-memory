# v3.0 Competitive Parity & Benchmarks — Design Spec

**Date:** 2026-03-21
**Status:** Approved
**Milestone:** v3.0 (follows v2.7 Multi-Runtime Portability)

---

## Milestone Goal

Close the three gaps that keep Agent-Memory from being the category leader:
retrieval pipeline orchestration, a dead-simple CLI API, and a benchmark suite
that produces a publishable LOCOMO score.

**Positioning statement:**
> Agent-Memory is the only cognitive memory architecture for AI agents —
> hierarchical, temporal, evolving — now with best-in-class retrieval
> orchestration and measurable proof.

---

## Competitive Context

| System | Tier | Strength | Weakness |
|--------|------|----------|----------|
| MemMachine | Tier 2 | Retrieval pipeline, LOCOMO benchmarks | No memory evolution, flat storage |
| Mem0 | Tier 2 | Simple API, memory lifecycle | Still fundamentally RAG-based |
| MemX | Tier 3 | Hybrid retrieval, confidence gating | No hierarchical structure |
| **Agent-Memory** | Tier 4 | Hierarchical compaction, multi-index, episodic memory, memory evolution | Retrieval not orchestrated as pipeline, no benchmark scores, complex API |

**Gaps to close in v3.0:**
1. Retrieval pipeline is not productized (MemMachine gap)
2. No dead-simple developer API (Mem0 gap)
3. No publishable benchmark score (narrative gap)

---

## Three Phases (Sequential)

| Phase | Name | Depends On |
|-------|------|------------|
| A | Retrieval Orchestrator | v2.7 complete |
| B | Simple CLI API | Phase A |
| C | Benchmark Suite | Phase B |

**Side quest (not a GSD phase):** Positioning writeup produced alongside Phase C.

---

## Phase A: Retrieval Orchestrator

### Goal
A new `memory-orchestrator` crate that coordinates all existing indexes into
a single, ranked, confidence-scored result set. Existing crates unchanged.

### Interface to `memory-retrieval`

The existing `memory-retrieval` crate contains `RetrievalExecutor`,
`FallbackChain`, `LayerExecutor`, `apply_combined_ranking`, and `StaleFilter`.
The orchestrator **wraps** `RetrievalExecutor` — it calls it as-is for each
index layer, then applies RRF fusion across the per-layer result sets returned.
`LayerExecutor` impls remain in `memory-retrieval`; the orchestrator consumes
their output via the existing `RetrievalResult` types. No code inside
`memory-retrieval` is modified. The orchestrator adds query expansion before
fan-out and RRF + reranking after fan-out — these are purely additive steps
around the existing executor interface.

### Pipeline

```
Query Input
    ↓
1. Query Expansion
   - Heuristic variants (stemming, synonyms) — default
   - LLM-assisted expansion via --expand=llm flag (optional)
    ↓
2. Multi-Index Fan-out (parallel, via RetrievalExecutor per layer)
   - BM25/Tantivy        → RetrievalResult (scored hits)
   - HNSW/Vector         → RetrievalResult (scored hits)
   - Topic Graph         → RetrievalResult (scored hits)
   - TOC hierarchy       → RetrievalResult (temporal anchor hits)
    ↓
3. Rank Fusion (heuristic default)
   - Reciprocal Rank Fusion (RRF) merges all four RetrievalResult sets
   - Salience weight applied (Phase 40 scores, already in RetrievalResult)
   - Recency decay applied (lifecycle scores, already in RetrievalResult)
   - Confidence score computed per result (normalized RRF score)
    ↓
4. Reranking (opt-in, layered — see scope note below)
   - Default:            heuristic (RRF output as-is)
   - --rerank=llm        configured LLM judges top-k candidates
    ↓
5. Context Builder
   - Structured output: Summary / Relevant Events / Key Entities / Open Questions
   - JSON and markdown output modes
```

### Reranking Scope (Phase A)

Phase A ships two rerank modes: **heuristic** (default, RRF output) and
**LLM** (opt-in via flag). Cross-encoder reranking (`--rerank=cross-encoder`)
is deferred: cross-encoders require a different inference path from
`all-MiniLM-L6-v2` (concatenated query+passage input, single logit output,
different tokenizer config) and no cross-encoder inference exists in
`memory-embeddings` today. Cross-encoder support is scoped to a future
sub-phase with an explicit model download flow and inference path design.
The `rerank.rs` trait is designed to accept a cross-encoder impl — the
extension point is built, the impl is not.

### Key Design Decisions

- **RRF as default merge strategy:** Parameter-free, proven in hybrid search
  literature, requires no per-corpus tuning. Salience/recency from Phase 40
  adjust the final ranking post-fusion.
- **New crate `memory-orchestrator`:** Sits between `memory-retrieval` and
  the CLI. All existing crates unchanged — orchestrator is additive.
- **Rerank mode in wizard config:** LLM rerank mode configured during the
  existing skill setup wizard. Runtime flags override per-call only.
- **Fail-open:** If any index fails during fan-out, orchestrator continues
  with available results (consistent with existing fail-open CCH policy).

### Success Criteria (Phase A, testable at PR time)

- [ ] Fusion unit tests confirm RRF produces a different ranking than any
  single input list when index scores diverge (no benchmark suite required)
- [ ] Orchestrator returns results when one of the four indexes returns empty
  (fail-open, unit tested)
- [ ] LLM rerank mode invokes the configured LLM client and re-orders results
  (integration tested with a mock LLM client)
- [ ] `cargo test -p memory-orchestrator` passes with no clippy warnings

### Deliverables
- `crates/memory-orchestrator/` — new crate with orchestrator trait + pipeline
- `crates/memory-orchestrator/src/fusion.rs` — RRF implementation
- `crates/memory-orchestrator/src/rerank.rs` — rerank trait (heuristic + LLM
  impls; cross-encoder extension point stubbed, not implemented)
- `crates/memory-orchestrator/src/context_builder.rs` — structured context assembly
- Integration with existing `memory-retrieval` crate via `RetrievalExecutor`
- Setup wizard updated to configure LLM rerank mode

---

## Phase B: Simple CLI API

### Goal
A developer-facing command layer producing structured JSON — designed to be
called from agent skills with zero context pollution.

### Binary Strategy

Phase B adds a **new `memory` binary** as a second `[[bin]]` entry in the
existing `memory-daemon` crate (or a new thin `memory-cli` crate — decided
during planning). The existing `memory-daemon` binary is **not renamed or
removed** — it continues to serve daemon management commands. The new `memory`
binary exposes the developer-facing API commands and is what agent skills call.
Existing skill hooks that call `memory-daemon` subcommands are unchanged.

### Commands

```bash
# Add an event to memory (routes through gRPC MemoryClient — daemon must be running)
memory add --content "..." --kind episodic --agent claude

# Search memory (wired to orchestrator)
memory search "query" --top=5 --format=json

# Get structured context for prompt injection
memory context "current task description" --format=json

# Get timeline for entity or topic
memory timeline --entity "auth module" --range=7d --format=json

# Get compressed summary of a time range
memory summary --range=week --format=json

# Multi-hop recall: alias for `memory search --rerank=llm --top=10`
# Intent: temporal/cross-session queries where LLM reranking improves results
memory recall "what did we decide last Tuesday?" --format=json
```

### `memory add` Daemon Dependency

All write commands (`memory add`) route through `MemoryClient` over gRPC —
the daemon must be running. This is consistent with the existing architecture
(all writes go through the daemon for locking, scheduling, and dedup). If the
daemon is not running, `memory add` exits non-zero with a clear error message:
`"memory daemon not running — start with: memory-daemon start"`.

### `memory recall` vs `memory search`

`memory recall` is a named alias for `memory search --rerank=llm --top=10`.
It uses the same code path but signals intent (cross-session, temporal,
multi-hop queries) and defaults to LLM reranking for better multi-hop accuracy.
No separate implementation — one subcommand delegates to the other.

### Output Contract (JSON Envelope)

Every command returns a consistent structure:

```json
{
  "status": "ok",
  "query": "...",
  "results": [...],
  "context": {
    "summary": "...",
    "relevant_events": [...],
    "key_entities": [...],
    "open_questions": [...]
  },
  "meta": {
    "retrieval_ms": 42,
    "tokens_estimated": 380,
    "confidence": 0.87
  }
}
```

### Design Principles

- `--format=json` is **default when stdout is not a TTY** (piped); human-readable
  when interactive. No flag needed from agent skills.
- `--rerank=llm` overrides wizard config per-call.
- All commands exit 0 on success, non-zero on hard failure — agent skills check `$?`.
- `meta.tokens_estimated` lets skills choose full context vs summary injection.

### Agent Skill Integration

Existing Claude/OpenCode/Gemini skills call `memory context "$QUERY"` in
pre-tool hooks. JSON envelope pipes directly into prompt builder. No new skill
logic beyond v2.7 hooks.

### Deliverables
- New `memory` binary (`[[bin]]` entry, location decided at planning time)
- All commands above wired to `memory-orchestrator`
- JSON envelope serialization via existing `serde_json`
- TTY detection for default format selection
- `memory-daemon` binary and existing skill hooks unchanged
- Updated canonical plugin source to reference `memory` binary in new hooks

---

## Phase C: Benchmark Suite

### Goal
A two-part benchmark system: a custom harness for internal metrics (ships first),
then a LOCOMO adapter for a publishable, comparable score.

### Sub-phase C1: Custom Harness

Three benchmark categories:

```bash
memory benchmark temporal      # temporal recall tests
memory benchmark multisession  # cross-session reasoning tests
memory benchmark compression   # token efficiency tests
memory benchmark all           # full suite + report
```

**Fixture format** (`benchmarks/fixtures/*.toml`):

```toml
[[test]]
id = "temporal-001"
description = "recall decision from prior session"
setup = ["session-a.jsonl", "session-b.jsonl"]
query = "what auth approach did we decide on?"
expected_contains = ["JWT", "stateless"]
max_tokens = 500
```

**Output metrics:**

```
accuracy:        87.3%   (+12.1% vs baseline)
recall@5:        0.91
token_usage:     avg 340 tokens per context
latency_p50:     48ms
latency_p95:     210ms
compression:     73% reduction vs raw context
```

**Baseline comparison:** `benchmarks/baselines.toml` stores manually-entered
competitor scores for side-by-side reporting. The `--compare` flag reads this
file — no scraping or external API calls.

```toml
[memmachine]
locomo_score = 0.91
token_reduction = 0.80
latency_improvement = 0.75

[mem0]
accuracy_vs_openai = 0.26
token_reduction = 0.90
```

### Sub-phase C2: LOCOMO Adapter

LOCOMO (Snap Research, public dataset) — ~300-turn multi-session conversations,
4 question types: single-hop recall, multi-hop reasoning, temporal understanding,
open-domain reasoning.

```bash
memory benchmark locomo --dataset=./locomo-data/ --output=results.json
memory benchmark locomo --compare=memmachine   # reads benchmarks/baselines.toml
```

**Dataset acquisition:** The LOCOMO dataset is downloaded separately via a
script at `benchmarks/scripts/download-locomo.sh`. The `locomo-data/` directory
is listed in `.gitignore` — it is never committed. Publishing benchmark scores
against the LOCOMO dataset is permitted under its research license (verify at
download time). The adapter feeds LOCOMO conversations through the ingestion
pipeline, runs 4 question types through the orchestrator, scores against gold
answers, and produces a comparable score in `results.json`.

### Deliverables
- `benchmarks/` directory with fixture format and runner
- `benchmarks/baselines.toml` with manually-entered competitor scores
- `benchmarks/scripts/download-locomo.sh` — dataset download script
- `locomo-data/` added to `.gitignore`
- `memory benchmark` subcommand group
- LOCOMO adapter with dataset loader and scorer
- JSON + markdown report output
- CI integration (optional run, not blocking — requires `--dataset` flag to activate)

---

## Side Quest: Positioning Writeup

**Not a GSD phase** — produced alongside Phase C by the developer.

**Output:** `docs/positioning/agent-memory-vs-competition.md`

Content:
- Head-to-head table: Agent-Memory vs Mem0 vs MemMachine across 6 dimensions
  (memory model, evolution, retrieval, structure, cognitive fidelity, API ergonomics)
- LOCOMO score comparison (populated after C2)
- "Beyond RAG: Cognitive Memory Architecture" narrative framing
- Publishable as blog post with minor editing

---

## Architecture Overview

```
                ┌─────────────────────────┐
                │      Agent / Skill      │
                └──────────┬──────────────┘
                           ↓  (shell: memory search/context/recall)
                ┌─────────────────────────┐
                │   `memory` binary       │  ← Phase B (new binary)
                │  search/context/recall  │
                └──────────┬──────────────┘
                           ↓
                ┌─────────────────────────┐
                │  memory-orchestrator    │  ← Phase A (new crate)
                │  query expand → fan-out │
                │  RRF → rerank → context │
                └──────────┬──────────────┘
                           ↓  (wraps RetrievalExecutor per layer)
          ┌────────────────┼────────────────┐
          ↓                ↓                ↓
      BM25/Tantivy    HNSW/Vector    Topic Graph + TOC
     (memory-search) (memory-vector) (memory-topics + memory-toc)
          └────────────────┼────────────────┘
                           ↓
                ┌─────────────────────────┐
                │   Memory Storage        │
                │  RocksDB append-only    │
                │  Hierarchical TOC       │
                │  Salience + Lifecycle   │
                └─────────────────────────┘

`memory-daemon` binary unchanged — daemon management + gRPC server
`memory add` → gRPC MemoryClient → memory-daemon (daemon must be running)
```

---

## What This Milestone Does NOT Include

- REST/HTTP endpoint (future milestone)
- Python SDK (future milestone, wraps CLI)
- Memory views UI (future milestone)
- Multi-agent shared memory changes (shipped in v2.1)
- Cross-encoder reranking (deferred — requires new inference path in memory-embeddings)

---

## Success Criteria

**Phase A (testable at PR time):**
- [ ] RRF unit tests confirm different ranking than any single input when scores diverge
- [ ] Fail-open: orchestrator returns results when one index returns empty
- [ ] LLM rerank mode works with mock LLM client (integration test)
- [ ] `cargo test -p memory-orchestrator` passes, zero clippy warnings

**Phase B:**
- [ ] `memory search "query" --format=json` returns JSON envelope in <100ms p50
- [ ] `memory recall` delegates to search with `--rerank=llm --top=10`
- [ ] `memory add` with daemon not running exits non-zero with clear error message
- [ ] TTY detection: JSON when piped, human-readable when interactive
- [ ] `memory-daemon` binary and existing skill hooks unchanged

**Phase C:**
- [ ] Custom benchmark suite runs end-to-end with fixture files
- [ ] LOCOMO adapter ingests dataset and produces `results.json` with aggregate score
- [ ] `--compare=memmachine` reads `benchmarks/baselines.toml` and prints side-by-side
- [ ] `locomo-data/` confirmed in `.gitignore`
- [ ] CI runs benchmark suite (non-blocking, skips LOCOMO without `--dataset` flag)

**All phases:**
- [ ] All new code passes `task pr-precheck` (clippy + fmt + test + doc)
- [ ] Positioning writeup published at `docs/positioning/agent-memory-vs-competition.md`
