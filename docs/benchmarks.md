# Performance Benchmarks

The `perf_bench` harness measures **ingest/index setup** separately from
**query** latency. Schema version 2 (2026-08-30) exists because the v3.0
numbers mixed those two and then published the mix as "TOC navigation."

## What each step actually measures

| Step | Kind | What is timed |
|------|------|----------------|
| `vector_model_load` | setup | One-shot Candle embedder load (process lifetime) |
| `{single,multi}.ingest` | setup | RocksDB `put_event` of the corpus |
| `{single,multi}.toc_build` | setup | `build_toc_segment`: MockSummarizer rollup, grip extract, parent TOC writes |
| `{single,multi}.bm25_index` | setup | Tantivy add + commit |
| `{single,multi}.vector_index` | setup | Embed TOC bullets + HNSW add |
| `{single,multi}.topics_index` | setup | Write topic records |
| `{single,multi}.toc` | **query** | Three `get_toc_node` lookups (year, day, segment) |
| `{single,multi}.bm25` | **query** | Tantivy BM25 search |
| `{single,multi}.vector` | **query** | HNSW search (model already loaded) |
| `{single,multi}.topics` | **query** | Top-topics + topic search |
| `{single,multi}.route_query` | **query** | `MemoryOrchestrator` / `RouteQuery` RPC |

## The 64.6 second TOC number (retired)

`crates/e2e-tests/benchmarks/latest.json` from 2026-02-12 (`tier=medium`,
`mode=warm`, `iterations=3`) recorded `single.toc` p50 = **64,576 ms** and
`single.vector` p50 = **7,171 ms**. That file timed:

```text
toc_start
  build_toc_segment(240 events)   # rollup — the 65s
  navigate_toc()                  # two RocksDB gets
toc_ms  →  labeled "toc"
```

It was a harness defect, not a query-path cost. p90/p99 were interpolated
from **3 samples** and are not percentiles. Schema v2 refuses to emit p90
unless `n ≥ 10` and p99 unless `n ≥ 30`.

If someone quotes the 65s figure: it is ingest-time mock rollup of a 240-event
segment, not "answer last week without scanning everything."

## Modes (structurally different)

- **Warm:** one store setup, one discarded warmup query, then N query samples
  against a stable index. This is the number that should be compared to the
  "always-works foundation" claim.
- **Cold:** a new store per iteration (setup + first query). Setup costs
  appear as `*_build` / `*_index` / `vector_model_load`.

## Run

Default `--iterations` is 30 (query samples).

Small, warm (the usual local check):

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode warm --iterations 30 --out-dir crates/e2e-tests/benchmarks
```

Medium, warm (the number that replaces the 65s claim):

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier medium --mode warm --iterations 30 --out-dir crates/e2e-tests/benchmarks
```

Cold (new store per iteration — expensive because it repeats `toc_build`):

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 30 --out-dir crates/e2e-tests/benchmarks
```

Optional JSONL trace of `memory_types::Event` payloads:

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode warm --iterations 30 --trace path/to/events.jsonl --out-dir crates/e2e-tests/benchmarks
```

## Output

- `latest.txt` — table with kind, samples, min/p50/p90/p99/max. `-` means the
  percentile was withheld for lack of samples.
- `latest.json` — structured metrics plus `corpus`, `hardware`, and `caveats`.

## Baseline comparisons

`crates/e2e-tests/benchmarks/baseline.json` is schema version 2. Comparison
runs only against **query** steps of a matching tier/mode. Version-1 files
(the mixed setup+query numbers) are ignored, not compared.

- Warning / severe thresholds apply to query `p50_ms` (and ingest throughput).
- `--write-baseline` replaces the matching tier/mode run.

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier medium --mode warm --iterations 30 --write-baseline --out-dir crates/e2e-tests/benchmarks
```

## Hardware and corpus (for any committed result)

Recorded in `latest.json`:

- `corpus.event_count` — 60 (small) or 240 (medium) synthetic events unless a
  trace file is supplied
- `hardware.os` / `hardware.arch`
- `iterations` — query samples
- `caveats` — the list above, committed next to the numbers

Vector indexing downloads the default Candle model on first run. That cost is
`vector_model_load`, not `vector`.

## Committed result (2026-08-30)

Artifact: `crates/e2e-tests/benchmarks/latest.json` (schema 2).

| Field | Value |
|-------|--------|
| Corpus | 240 synthetic events (medium) |
| Mode | warm (one setup, one discarded warmup, 30 query samples) |
| Hardware | linux/x86_64 |
| Profile | `cargo run` **debug** (not `--release`) |
| Generated | 2026-08-30T17:30:20Z |

Query p50 (30 samples, p90/p99 reported):

| Step | p50 | What it is |
|------|-----|------------|
| `single.toc` | **0.13 ms** | three `get_toc_node` lookups |
| `single.bm25` | 0.57 ms | Tantivy search |
| `single.topics` | 0.97 ms | topic lookup |
| `single.vector` | 4.15 s | Candle **query embed** + HNSW (debug) |
| `single.route_query` | 4.15 s | orchestrator fan-out; dominated by the same query embed |

Setup (1 sample — min/median/max only, no fake p90/p99):

| Step | p50 | What it is |
|------|-----|------------|
| `single.toc_build` | **76.7 s** | MockSummarizer rollup of 240 events. This is the retired 64.6 s "TOC navigation" number. |
| `single.vector_index` | 12.6 s | embed TOC bullets + HNSW add |
| `vector_model_load` | 0.16 s | Candle load from local cache |
| `single.bm25_index` | 36 ms | Tantivy add+commit |
| `single.ingest` | 7.5 ms | RocksDB puts (~32k events/s) |

`single.toc` warm p50 is **0.13 ms**, well under the 500 ms acceptance bar.
The 65-second figure is ingest-time rollup (`toc_build`), not query.

Vector: model load, index build, and query are three steps. Query p50 in this
debug run is ~4 s because each search embeds the query string with Candle on
CPU; that is a real query-path cost in this profile, not embedder init. Do
not quote it as production HNSW latency — re-run `--release` before
publishing a product number.


---

# Retrieval-quality benchmarks (Phase 56)

`perf_bench` (above) measures **latency**. `memory-bench` measures **retrieval
quality**. They are different tools. This section is the methodology for the
quality harness. No number here is a published LOCOMO leaderboard score
unless its `metric` field is exactly `locomo_llm_judge` and a pinned model
id is recorded.

## Custom TOML-fixture harness

```bash
cargo run -p memory-bench -- all --backend mock --output benchmarks/results/custom-harness-mock.json
```

`--backend mock` (default) uses an isolated in-process token-overlap store
**per test**. That is a pipeline number, not production retrieval quality.
`--backend cli` shells out to a running `memory` daemon; `memory add` /
`memory search` failures abort the run (a dead daemon is not accuracy 0.0).

## LOCOMO live backend (Phase 60-01)

`--backend cli` on `memory-bench locomo` defaults to
`--isolation daemon-per-conversation`: for each conversation the harness
creates a tempdir, spawns `memory-daemon start --db-path <tmp> --port <free>
--pid-file <tmp>/daemon.pid`, waits until `memory-daemon query checkpoints`
answers, ingests, then **polls GetIndexCheckpoints** until the BM25
checkpoint covers the outbox head (timeout 5 minutes). Result JSON records
`"isolation": "per-conversation daemon"` and per-conversation
`drain_wait_ms`.

`--isolation shared` is local debugging only. It prints a bleed caveat and
must not be committed.

There is no `std::thread::sleep` standing in for drain. Poll interval is a
channel `recv_timeout`.

```bash
# CI / local live-backend smoke (1 conversation, mock judge, spawned daemon)
cargo run -p memory-bench -- locomo \
  --dataset benchmarks/fixtures/locomo-smoke.json \
  --backend cli --scorer mock \
  --isolation daemon-per-conversation
```

Metrics:

| Metric | What it is |
|--------|------------|
| `accuracy` | fraction of tests whose retrieved text contains at least one `expected_contains` string |
| `recall_at_k` | fraction of **labeled `relevant` items** found in the top-k ranked texts. Not equal to accuracy. Omitted when no test supplies `relevant`. |
| `compression_ratio` | `1 - context_tokens / raw_tokens` where `raw_tokens` is `ceil(chars/4)` of setup **file contents** (not path-string lengths) |
| `latency_p50_ms` / `p95` | search latency |

`--compare` prints competitor rows with a **Metric** column. MemMachine's
0.91 is `LOCOMO LLM-judge (their paper)`. Mem0's +26% is `relative delta vs
OpenAI memory`. Those rows are not commensurable with fixture `accuracy`.

## LOCOMO adapter v2

Real schema (`data/locomo10.json` from [snap-research/locomo](https://github.com/snap-research/locomo)):

- top-level JSON array
- each sample: `sample_id`, `conversation` (`speaker_a`/`speaker_b`, `session_N` + `session_N_date_time`), `qa[]` with `question`, `answer` (string **or** number), integer `category`, `evidence[]`

Category map (from `task_eval/evaluation.py` + inspection of `category: 2` "When did…" items): `1=multi_hop`, `2=temporal`, `3=open_domain`, `4=single_hop`, `5=adversarial`.

Download (prints `LICENSE.txt`, CC BY-NC 4.0, before the data file):

```bash
./benchmarks/scripts/download-locomo.sh locomo-data
```

`locomo-data/` is gitignored. Do not commit the dataset.

One **isolated store per `sample_id`**. Sessions are ingested with their
parsed timestamps (`1:56 pm on 8 May, 2023`) and speakers.

### Scorers

| `--scorer` | `metric` in results.json | When to use |
|------------|--------------------------|-------------|
| `mock` (default) | `context_hit_rate` | CI smoke. Gold-answer substring in retrieved context. **Not a LOCOMO score.** `--compare` is refused. |
| `llm-judge` | `locomo_llm_judge` | Retrieve → generate answer → judge at temperature 0. Requires `OPENAI_API_KEY` or `ANTHROPIC_API_KEY`. Model id is recorded. |

```bash
# CI / local smoke (committed fixture, 1 conversation, mock judge)
cargo run -p memory-bench -- smoke --output benchmarks/results/locomo-smoke.json

# Full dataset, still not a LOCOMO score
cargo run -p memory-bench -- locomo --dataset locomo-data --scorer mock

# Live daemon, isolated per conversation (the 60-02 path)
cargo run -p memory-bench -- locomo --dataset locomo-data --backend cli --scorer llm-judge \
  --isolation daemon-per-conversation --output benchmarks/results/locomo-$(date -u +%F).json

# Cost-capped dry run
cargo run -p memory-bench -- locomo --dataset locomo-data --backend cli --scorer llm-judge \
  --limit-questions 200 --output benchmarks/results/locomo-$(date -u +%F)-partial.json
```

`memory add --timestamp RFC3339 --session-id ID --role user|assistant` exists
so a live-daemon LOCOMO run can preserve session time.

## Decision gate (2026-08-30)

**HOLD comparison marketing.** This phase commits:

1. `benchmarks/results/locomo-smoke.json` — 1-conversation fixture, `metric=context_hit_rate`, mock retrieval. Pipeline evidence, not a leaderboard number.
2. `benchmarks/results/custom-harness-mock.json` — ≥25 fixtures, `backend=mock`. Pipeline evidence.

No `locomo_llm_judge` artifact is committed because no API key was used for a
full `locomo10.json` run. Until that artifact exists, README/docs must not
claim a LOCOMO score, and `--compare` must not imply Agent-Memory "beats"
Mem0/MemMachine.

## Smoke fixture

`benchmarks/fixtures/locomo-smoke.json` is a 1-conversation file in the real
schema (including a numeric `answer` in the v2 tests). It is not a subset of
the official dataset; it exists so CI can execute parse → ingest → retrieve →
score without downloading CC-BY-NC data.
