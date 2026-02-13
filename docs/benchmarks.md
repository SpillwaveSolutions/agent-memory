# Performance Benchmarks

The `perf_bench` harness measures ingest throughput and retrieval latency across TOC navigation,
BM25 teleport, vector teleport, topic graph, and `route_query`.

## Run the benchmarks

Small dataset (cold):

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 3 --out-dir crates/e2e-tests/benchmarks
```

Small dataset (warm):

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode warm --iterations 3 --out-dir crates/e2e-tests/benchmarks
```

Medium dataset (cold/warm):

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier medium --mode cold --iterations 3 --out-dir crates/e2e-tests/benchmarks
cargo run -p e2e-tests --bin perf_bench -- --tier medium --mode warm --iterations 3 --out-dir crates/e2e-tests/benchmarks
```

## Optional trace input

Provide a JSONL file of `memory_types::Event` payloads to override synthetic data:

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 3 --trace path/to/events.jsonl --out-dir crates/e2e-tests/benchmarks
```

Each line must be a single JSON-encoded `Event`. The harness will re-tag session IDs and agents
to keep runs isolated while preserving the original content.

## Output conventions

The harness prints a tabular summary and JSON payload to stdout. It also writes deterministic
files in the output directory:

- `latest.txt`: human-readable table with p50/p90/p99 per step
- `latest.json`: structured metrics, including ingest throughput (events/sec)

Step keys are prefixed with `single.` or `multi.` to distinguish single-agent vs multi-agent runs.

## Baseline comparisons

Baseline metrics live in `crates/e2e-tests/benchmarks/baseline.json`. By default, each run
loads the baseline for the matching tier/mode and reports warnings or severe regressions.

- Warning: exceeds either warning relative (%) or absolute (ms/EPS) thresholds
- Severe: exceeds severe thresholds and exits with non-zero status

To update the baseline with current results:

```bash
cargo run -p e2e-tests --bin perf_bench -- --tier small --mode cold --iterations 3 --write-baseline --out-dir crates/e2e-tests/benchmarks
```

Repeat for other tier/mode combinations to keep the baseline current.

## Notes

- Vector benchmarks use the default embedding model and may download weights on first run.
- Expect vector indexing to dominate runtime for medium datasets.
