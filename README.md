# Agent Memory

**Local-first conversational memory for AI coding agents.** Your agent answers
"what were we talking about last week?" by *navigating* a time-hierarchical
index — not by replaying your whole history into its context window.

[![CI](https://github.com/SpillwaveSolutions/agent-memory/actions/workflows/ci.yml/badge.svg)](https://github.com/SpillwaveSolutions/agent-memory/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Everything runs on your machine: an append-only RocksDB event log, a local
BM25 index (Tantivy), a local vector index (HNSW + Candle embeddings), and a
gRPC daemon your agent talks to. Capture is passive — CLI hooks pipe events in,
so the agent spends **zero tokens** recording what it did.

---

## Why this instead of a memory API

Three things are structurally different, not just tuned differently:

- **Passive, zero-token capture.** Events arrive from CLI hooks. The agent is
  not asked to "decide what to remember", so remembering costs no tokens and
  cannot be skipped when the context is full.
- **Local-first.** The event log, the indexes, and the embeddings live in
  `~/.local/share/agent-memory`. Nothing leaves the machine unless you turn on
  an LLM summarizer or LLM reranking and give it an API key.
- **Cross-CLI.** Memory is a layer beside the CLI, not inside one, so the same
  store is reachable from more than one agent runtime.

The long-form comparison against Mem0, Zep, MemMachine, and Letta — including
the "won't the vendors just build this in?" question — is in
[docs/positioning/agent-memory-vs-competition.md](docs/positioning/agent-memory-vs-competition.md).

---

## How retrieval works

Instead of scanning everything, the agent drills down a Table of Contents built
over time, reading a summary at each level and deciding whether to go deeper:

```
Year ──▶ Month ──▶ Week ──▶ Day ──▶ Segment ──▶ Grip ──▶ raw events
                                                  │
                          summary + keywords ─────┘  excerpt + provenance
```

Three independent retrieval layers feed a single fused ranking:

```
                       ┌──────────────────────────────┐
   hooks ──▶ ingest ──▶│  RocksDB append-only log     │
  (passive)            └───────────────┬──────────────┘
                                       │ outbox
                 ┌─────────────────────┼─────────────────────┐
                 ▼                     ▼                     ▼
          BM25 (Tantivy)        HNSW vectors           TOC + topics
                 └─────────────────────┼─────────────────────┘
                                       ▼
                             MemoryOrchestrator
                        (fuse ─▶ rerank ─▶ explain)
                                       ▼
                            gRPC  ──▶  memory search
```

`MemoryOrchestrator` is reachable from the shipped binaries: the daemon's
`RouteQuery` RPC calls it, and `memory search` is a client of that RPC.

---

## Quickstart (5 minutes)

Prerequisites: **macOS or Linux**, `protoc`, and Rust (the repo pins the
toolchain in `rust-toolchain.toml`, so `rustup` picks the right one).

```bash
# Ubuntu/Debian: sudo apt-get install -y protobuf-compiler libclang-dev
# macOS:         brew install protobuf llvm
```

### 1. Build

```bash
git clone https://github.com/SpillwaveSolutions/agent-memory.git
cd agent-memory
cargo build --release -p memory-daemon -p memory-ingest -p memory-cli
export PATH="$PWD/target/release:$PATH"
```

Prefer not to compile? Each release ships one archive per platform containing
all four binaries — see
[docs/setup/quickstart.md](docs/setup/quickstart.md#option-b-prebuilt-binaries).

### 2. Start the daemon

The daemon runs in the foreground. There is no built-in background mode — use
`systemd`, `launchd`, or your terminal multiplexer.

```bash
memory-daemon start --foreground &
memory-daemon status
```

First start creates the store and its index directories under
`~/.local/share/agent-memory/` and downloads the embedding model for vector
search. With no network it logs a warning and runs BM25-only — the daemon still
starts.

### 3. Record something

In real use, CLI hooks do this for you (see step 6). To prove the path works:

```bash
memory add --content "We chose RS256 over HS256 for the auth service JWTs" --agent claude
memory add --content "Rate limiting lives in the gateway, not the auth service" --agent claude
```

### 4. Wait for the indexer

Indexing is **not synchronous with ingest**. Events land in the event log
immediately and an outbox drain indexes them on a one-minute schedule, so a
query issued straight after `memory add` legitimately returns nothing.

```bash
sleep 70
```

### 5. Ask for it back

```bash
memory search "which JWT signing algorithm did we pick" --top 5
memory search "rate limiting gateway" --format json | jq '.results[0].text_preview'
```

BM25 matches tokens as written — it does not stem, so `jwt` will not find
`JWTs`. Paraphrase matching is the vector layer's job, and that needs the
embedding model from step 2.

`memory recall` is the same search with LLM reranking; it needs an API key and
falls back to the heuristic ranker (and *says so* in its explainability
payload) when the model is unavailable.

### 6. Wire it to your agent

```bash
cargo build --release -p memory-installer
target/release/memory-installer install --agent claude --project
```

Installs hooks, commands, and skills into `./.claude/`. Use `--global` for
`~/.claude/`, and `--dry-run` to see the file list first. Run
`memory-installer install --help` for the runtimes on offer.

---

## Status: what is solid, what is not

This table is the point of the v3.1 milestone. If a row says experimental, it
is experimental.

| Area | Status | Notes |
|---|---|---|
| Append-only event log (RocksDB) | **Solid** | Immutable, durable, the source of truth |
| Passive hook capture → `memory-ingest` | **Solid** | Covered by the bats CLI suites on Linux + macOS |
| TOC build and drill-down navigation | **Solid** | Year → Month → Week → Day → Segment → Grip |
| Grips / provenance | **Solid** | Excerpts link back to the events they came from |
| BM25 keyword search (Tantivy) | **Solid** | Exact tokens, no stemming (`jwt` does not match `JWTs`). Events indexed before v3.1 have empty `text_preview` and there is no backfill command — see [UPGRADING](docs/UPGRADING.md) and [#41](https://github.com/SpillwaveSolutions/agent-memory/issues/41) |
| Vector search (HNSW + Candle) | **Solid** | Mechanism is wired; retrieval *quality* is not yet measured ([#40](https://github.com/SpillwaveSolutions/agent-memory/issues/40)). First daemon start downloads the embedding model; with no network the daemon warns and runs BM25-only |
| Topic graph | **Works** | Clustering quality is not benchmarked ([#47](https://github.com/SpillwaveSolutions/agent-memory/issues/47)) |
| Hybrid fusion + `RouteQuery` orchestration | **Works** | Wired end-to-end in Phase 54; explainability reports what actually ran |
| LLM summarization / LLM rerank | **Experimental** | Needs an API key; fails open to the heuristic ranker and reports `rerank=heuristic` when it does |
| Cross-project federated query | **Experimental** | Implemented; not performance-characterised |
| Cross-encoder rerank | **Not implemented** | The extension point exists and returns an explicit error — it is not silently degraded. Build only if [#39](https://github.com/SpillwaveSolutions/agent-memory/issues/39) says retrieval is the bottleneck ([#44](https://github.com/SpillwaveSolutions/agent-memory/issues/44)) |
| Ingest → searchable latency | **~1 minute** | The outbox drains on a schedule; ingest is deliberately not blocked on indexing |
| Offline TOC rebuild (`admin rebuild-toc`) | **Not implemented** | Exits non-zero with guidance. TOC nodes come from the daemon's scheduled rollup jobs ([#43](https://github.com/SpillwaveSolutions/agent-memory/issues/43)) |
| Background daemonization | **Not implemented** | `--background` exits non-zero with guidance rather than pretending. v3.2 will ship `install-service` unit files ([#42](https://github.com/SpillwaveSolutions/agent-memory/issues/42)) |

### Benchmarks

`docs/benchmarks.md` explains what the harness measures and, specifically, why
the old "65 second TOC" number was a harness defect (it timed ingest-time
rollup and labelled it navigation).

Committed results live in `benchmarks/results/`. Today both are **mock-backend**
runs — a mock retrieval backend and a mock judge — so they demonstrate the
harness, not competitive quality. **There is deliberately no comparison
marketing in this repo**, and there will not be until a real-backend,
real-judge run is committed next to the claim
([#39](https://github.com/SpillwaveSolutions/agent-memory/issues/39)).

---

## Supported surface

Maintaining six runtime converters and five CLI test suites as first-class
promises is not sustainable for this project's size, so the promise is tiered
rather than uniform.

| Tier | Runtimes | What it means |
|---|---|---|
| **Tier 1 — supported** | Claude Code, Codex CLI | Converters are exercised on every PR (bats suites on Linux + macOS). Bugs here are release blockers |
| **Tier 2 — best effort** | Gemini CLI, Copilot CLI | Converters are implemented and tested; their bats suites run on a weekly schedule, not the PR gate. Fixes are welcome, response is not guaranteed |
| **Not supported** | OpenCode | The converter was an empty stub that reported success and wrote nothing. It was removed in Phase 57 rather than shipped. `--agent opencode` is now rejected |

Any runtime can still feed the store directly by piping events to
`memory-ingest` with `--agent <name>` — that path is runtime-agnostic and is
unaffected by tiering.

---

## Documentation

| Doc | What's in it |
|---|---|
| [docs/README.md](docs/README.md) | Concepts: progressive disclosure, TOC navigation, grips |
| [docs/setup/quickstart.md](docs/setup/quickstart.md) | Longer install path, including prebuilt binaries |
| [docs/setup/agent-setup.md](docs/setup/agent-setup.md) | Per-runtime hook wiring |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crate layout and data flow |
| [docs/API.md](docs/API.md) | gRPC surface |
| [docs/benchmarks.md](docs/benchmarks.md) | What the perf harness measures, and what it does not |
| [docs/verification/57-quickstart-transcript.md](docs/verification/57-quickstart-transcript.md) | The transcript of this quickstart being run on a clean machine, defects and all |
| [docs/positioning/agent-memory-vs-competition.md](docs/positioning/agent-memory-vs-competition.md) | Head-to-head vs Mem0 / Zep / MemMachine / Letta |
| [docs/UPGRADING.md](docs/UPGRADING.md) | Version-to-version migration notes |
| [docs/RELEASING.md](docs/RELEASING.md) | How to cut a tag so the pipeline cannot repeat the v3.1.0 stale-ref incident |
| [CHANGELOG.md](CHANGELOG.md) | What changed per release, including retractions |

## Contributing

`task pr-precheck` before every PR — it runs the same format, clippy, test, and
doc gates CI does. See [CLAUDE.md](CLAUDE.md) for repository conventions.

## License

MIT — see [LICENSE](LICENSE).
