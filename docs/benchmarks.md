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
