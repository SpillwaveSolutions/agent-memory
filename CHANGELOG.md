# Changelog

Notable changes per release. Dates are the release date, not the merge date of
the last commit.

The guiding rule for this file, after the v3.0 retrospective: **a change is
listed only if it is true of the shipped code.** Claims that turned out to be
aspirational are recorded as retractions, not quietly dropped.

## v3.1.0 — Make It True (2026-08-31)

No new capabilities. This release closes the gap between what the project
claimed and what it did.

### Added

- Root `README.md` and `LICENSE` (MIT) — the repository had neither
- `docs/positioning/agent-memory-vs-competition.md` — head-to-head against
  Mem0, Zep, MemMachine and Letta, with a claims ledger and the platform-risk
  argument stated up front
- `docs/verification/57-quickstart-transcript.md` — the quickstart executed on
  a clean machine, including the three defects the first run exposed
- `docs/benchmarks.md` — what the performance harness measures and what it does
  not
- Supported-surface tiering: Tier 1 (Claude Code, Codex CLI) gates every PR;
  Tier 2 (Gemini, Copilot) runs on a weekly schedule
- Release archives now ship all four binaries (`memory-daemon`,
  `memory-ingest`, `memory`, `memory-installer`); previously the CLI the
  quickstart depends on was not in the release at all

### Fixed

- **First-run daemon indexed nothing.** A fresh store had no `db/search` or
  `db/vector`, so the outbox indexing job never registered and every query
  returned an empty result set with no error. `start_daemon` now creates them
- **`memory-orchestrator` was unreachable** from any shipped binary. `RouteQuery`
  now calls it, and `memory search` is a client of that RPC
- **Hybrid search was not hybrid** — it now fetches BM25 and vector results and
  fuses them
- **BM25 outbox drain was a no-op**; events are indexed, and misses warn and
  increment a counter instead of passing silently
- **BM25 stored no text for events**, so previews were empty and the LLM
  reranker was judging blank bodies. New event documents are `TEXT | STORED`
- **A successful LLM rerank was undone** by a salience re-sort afterwards
- **Explainability misreported what ran**: a failed LLM rerank said
  `rerank=llm`, `layers_attempted` listed layers that returned nothing, and
  client `stop_conditions` / `mode_override` were echoed back while being
  ignored. All now report and behave truthfully
- Per-event grip full-scan on the BM25 drain (an O(n²) drain) removed
- Retrieval fan-out is concurrent unless explicitly sequential
- Query, prune and dedup share one HNSW handle; embedding dimension comes from
  the embedder rather than a hardcoded 384
- Lock poisoning is recovered and counted rather than panicking
- CI pinned to Rust 1.97 via `rust-toolchain.toml`, after a floating-stable
  Clippy lint reddened `main`

### Changed — now fails loudly instead of silently

- `memory-daemon start --background` exits non-zero with guidance; background
  daemonization is not implemented
- `memory-daemon admin rebuild-toc` exits non-zero; offline TOC rebuild is not
  implemented and no longer prints a TODO and exits 0
- `CrossEncoderReranker` returns an explicit `NotImplemented` error rather than
  degrading quietly
- An unknown `--rerank` value is rejected rather than falling back to heuristic
- `admin rebuild-bm25` is relabelled: it prunes documents below `--min-level`
  and re-indexes nothing, which is what it always did

### Removed (breaking)

- **OpenCode is no longer a supported runtime.** Every method of its converter
  returned empty, so `memory-installer --agent opencode` exited 0 and wrote no
  files. The converter, the `Runtime::OpenCode` variant, its tool mappings, its
  bats suite and the archived plugin directory are gone. `--agent opencode` now
  exits 2. The runtime-agnostic `memory-ingest --agent opencode` path is
  unaffected. See [UPGRADING](docs/UPGRADING.md)

### Benchmarks

- The "64.6 second TOC navigation" figure is **retracted**. It timed
  ingest-time summarization rollup and labelled it navigation. Warm query
  `single.toc` p50 is 0.13 ms over 30 samples
- Percentiles are withheld below 10 samples (p90) and 30 samples (p99) instead
  of being interpolated from 3
- Committed results in `benchmarks/results/` are **mock-backend and mock-judge**
  and are labelled as such. No comparative accuracy claim ships anywhere in this
  repository until a real-backend, real-judge run is committed beside it

### Known gaps

- No backfill for events indexed before v3.1 — their `text_preview` stays empty
- Vector search requires the embedding model download on first daemon start;
  with no network the daemon warns and runs BM25-only
- Ingest to searchable is a ~1 minute scheduled outbox drain, not synchronous

## Earlier releases

Milestones v1.0 through v3.0 predate this file. Their scope is recorded in
`.planning/MILESTONES.md` and `.planning/ROADMAP.md`, and upgrade notes for
v2.0.0 through v2.2.0 are in [docs/UPGRADING.md](docs/UPGRADING.md).
