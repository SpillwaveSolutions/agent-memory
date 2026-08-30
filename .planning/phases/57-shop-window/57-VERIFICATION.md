---
phase: 57-shop-window
verified: 2026-08-30
status: passed
---

# Phase 57: Shop Window & Positioning Verification

**Phase Goal:** a stranger landing on the repo understands what it is, trusts
it, and can run it — and the project's public claims match Phases 54–56 reality.

## Execution evidence

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 1 | Root `README.md` exists and renders the landing page | FILE | `README.md`, 200+ lines: pitch, ASCII architecture, quickstart, status table, tiers, docs index |
| 2 | `LICENSE` present and matches `Cargo.toml` | FILE | `LICENSE` (MIT); `workspace.package.license = "MIT"` |
| 3 | `repository` points at the real remote | RUN | `Cargo.toml:30` = `SpillwaveSolutions/agent-memory`; `git remote -v` agrees |
| 4 | Quickstart executed verbatim on a machine with no toolchain and no store | **RUN** | `docs/verification/57-quickstart-transcript.md` — full transcript, store wiped first |
| 5 | Search returns the ingested event after the documented wait | **RUN** | `memory search "which JWT signing algorithm did we pick"` → 1 hit, `source_layer: bm25`, `text_preview` populated |
| 6 | Positioning doc exists, leads with the three structural differences | FILE | `docs/positioning/agent-memory-vs-competition.md` — head-to-head table, "where they are ahead of us", platform risk, claims ledger with sources + dates |
| 7 | No comparative benchmark claim ships (Phase 56 gate honored) | DOCS | Positioning doc states the only committed results are mock-backend / mock-judge and declines the comparison; README Benchmarks section says the same |
| 8 | OpenCode stub removed, not archived | **RUN** | `memory-installer install --agent opencode --project` → `invalid value 'opencode'`, exit 2. Previously exited 0 and wrote nothing |
| 9 | No stub converter can be reintroduced silently | TEST | `every_offered_runtime_converts_something` in `crates/memory-installer/tests/e2e_converters.rs` iterates `Runtime::value_variants()` |
| 10 | Tier 1 gates PRs; Tier 2 runs on a schedule | CI | `e2e-cli.yml` matrix = `[claude-code, codex]` on push/PR; new `e2e-cli-tier2.yml` matrix = `[gemini, copilot]` on `schedule` + `workflow_dispatch` |

## Defects found by executing the quickstart (and fixed in this phase)

The README was written first, then run. Three defects surfaced — all in the
"documented happy path silently does nothing" family:

| # | Defect | Fix |
|---|--------|-----|
| A | A fresh store has no `db/search` or `db/vector`, so the outbox indexing job never registered and **every query returned `results: []` with `confidence 0.0` and no error** | `start_daemon` creates both index directories before job registration (`crates/memory-daemon/src/commands.rs`) |
| B | The remedy the daemon itself printed (`admin rebuild-indexes`) failed on the RocksDB lock while the daemon ran, and reported "No documents found" when stopped — it indexes TOC nodes and grips, not raw events | Moot after A: the documented path no longer routes through it |
| C | `admin rebuild-toc` printed "TOC rebuild not yet fully implemented" and **exited 0**; `--dry-run` claimed "To actually rebuild, run without --dry-run" | Now `anyhow::bail!`s with guidance and exits 1 — same treatment Phase 54 gave `--background` |

Two truths about retrieval were also found and written into the README rather
than left for a user to discover:

- Indexing is a ~1-minute scheduled outbox drain, not synchronous with ingest.
  The README now has an explicit wait step.
- BM25 does not stem: `jwt` does not match `JWTs`. The README's second example
  was changed to a query BM25 can answer, and the status table says so.

## Human verification (blockers)

- [x] Quickstart executed start-to-finish from the README verbatim, on a fresh
      store, with the transcript committed
- [x] `memory search` returns the ingested event (not an empty result set)
- [x] `--agent opencode` is rejected rather than silently succeeding
- [x] No comparative accuracy claim anywhere public

## Not done (stated, not waved through)

| Item | Why | Owner |
|---|---|---|
| GitHub repo description, topics, Discussions | Repository settings — cannot be changed from a PR | Maintainer, before launch |
| asciinema / GIF demo of drill-down navigation | Not recorded. The executed transcript is committed instead | Phase 58 (Launch) |
| Vector search exercised in the quickstart | This container's proxy blocks the Hugging Face model download; the daemon warned and ran BM25-only, as documented | Covered by the workspace test suite |
| macOS quickstart | Not run here. macOS prerequisites in the README come from the CI workflow, which does run `macos-latest` | Phase 58 (Launch) |
