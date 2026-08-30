# Phase 57 quickstart execution transcript

**Executed:** 2026-08-30
**Machine:** Linux x86_64 container, fresh clone, **no Rust toolchain, no
`protoc`, no `libclang`, and no `~/.local/share/agent-memory` store**
**Method:** the root `README.md` "Quickstart (5 minutes)" section, run verbatim,
in order, with nothing added that the README does not tell you to run.

This file is the execution evidence for the Phase 57 exit criterion. It is a
record of what happened, including the three things that did not work the first
time and what was changed as a result.

---

## What the first run found

The README was written first and then executed. Three defects surfaced, all of
them in the "documented happy path silently does nothing" family this milestone
exists to eliminate:

### 1. `memory search` returned zero results for content just added

```console
$ memory add --content "We chose RS256 over HS256 for the auth service JWTs" --agent claude
{"status":"ok","query":"add","results":{"created":true,"event_id":"01M1ABYF8RR4F8JCGGE2Z0V864"},...}

$ memory search "which JWT signing algorithm did we pick" --top 5
{"status":"ok","query":"...","results":[],"meta":{"retrieval_ms":0,"tokens_estimated":0,"confidence":0.0}}
```

No error, no warning, `confidence 0.0`. The daemon had said why at startup, in
an INFO line nobody reads:

```text
INFO memory_daemon::commands: No BM25 index at ".../db/search"; RouteQuery will skip BM25
WARN memory_daemon::commands: Indexing job not registered: Search index directory not found
INFO memory_daemon::commands: Run 'rebuild-indexes' to initialize the search index
```

A fresh store has no `search/` or `vector/` directory, and both the outbox
indexing job and the prune jobs only register when their directory *already*
exists. So a first-run daemon accepted events forever and answered every query
with nothing.

**Fix:** `start_daemon` now creates `db/search` and `db/vector` before job
registration (`crates/memory-daemon/src/commands.rs`).

### 2. The remedy the daemon printed did not work

```console
$ memory-daemon admin rebuild-indexes
Error: RocksDB error: IO error: While lock file: .../db/LOCK: Resource temporarily unavailable
   [12 frames of anyhow backtrace]
```

`rebuild-indexes` needs the RocksDB lock, which the running daemon holds. And
after stopping the daemon it reported `No documents found in storage to index.`
— because it indexes TOC nodes and grips, of which a fresh store has zero,
not raw events.

**Fix:** made moot by fix 1 — the documented path no longer routes through
`rebuild-indexes` at all.

### 3. `admin rebuild-toc` printed a TODO and exited 0

```console
$ memory-daemon admin rebuild-toc
Found 2 events to process

TOC rebuild not yet fully implemented.
This would require re-running segmentation and summarization.
Events are intact and can be manually processed.
$ echo $?
0
```

`--dry-run` even said "To actually rebuild, run without --dry-run", which was
false. Same class of defect as the `--background` flag Phase 54 fixed.

**Fix:** it now fails loudly with guidance.

```console
$ memory-daemon admin rebuild-toc
Found 2 events to process
Error: offline TOC rebuild is not implemented; TOC nodes are produced by the daemon's
scheduled rollup jobs (toc_rollup_day / _week / _month) -- run `memory-daemon start
--foreground` and check `memory-daemon scheduler status`. Your events are intact in
the event log.
$ echo $?
1
```

---

## The verifying run (store wiped, README followed verbatim)

`rm -rf ~/.local/share/agent-memory ~/.cache/agent-memory` first, so this is a
true first run.

### Step 1 — Build

```console
$ cargo build --release -p memory-daemon -p memory-ingest -p memory-cli
    Finished `release` profile [optimized] target(s) in 3m 11s
```

The prerequisites line in the README is load-bearing: without
`protobuf-compiler` the build fails at `prost-build` with `Could not find
protoc`, and `rocksdb` needs `libclang-dev`. Both were installed by following
the README's prerequisite block, on a machine that had neither.

### Step 2 — Start the daemon

```console
$ memory-daemon start --foreground &
$ memory-daemon status
Memory daemon is running (PID 13963)
PID file: "/root/.cache/agent-memory/daemon.pid"
```

Daemon log, showing the fix from defect 1 taking effect on a fresh store:

```text
INFO memory_daemon::commands: Created index directory "/root/.local/share/agent-memory/db/search"
INFO memory_daemon::commands: Created index directory "/root/.local/share/agent-memory/db/vector"
INFO memory_daemon::commands: BM25 searcher attached docs=0
WARN memory_daemon::commands: Failed to load embedder for vector search error=Failed to
     download model: ... Connection Failed: tls connection init failed
INFO memory_scheduler::jobs::indexing: Registered outbox indexing job
```

**Environment caveat:** this container's egress proxy blocks the Hugging Face
model download, so the embedder never loaded and **vector search was not
exercised**. The daemon warned and continued BM25-only, which is the documented
behaviour. Vector retrieval is verified by the workspace test suite, not by
this transcript.

### Step 3 — Record something

```console
$ memory add --content "We chose RS256 over HS256 for the auth service JWTs" --agent claude
{"status":"ok","query":"add","results":{"created":true,"event_id":"01M1AC7CSHMS9XN65TPACNBHDF"},"meta":{"retrieval_ms":0,"tokens_estimated":88,"confidence":1.0}}

$ memory add --content "Rate limiting lives in the gateway, not the auth service" --agent claude
{"status":"ok","query":"add","results":{"created":true,"event_id":"01M1AC7CSQQ2Y2FCT5QVNDR378"},"meta":{"retrieval_ms":0,"tokens_estimated":92,"confidence":1.0}}
```

### Step 4 — Wait for the indexer

```console
$ sleep 70
```

Indexing is a scheduled outbox drain (`0 * * * * *`, up to 10s jitter), not a
synchronous write. Observed drain:

```text
INFO memory_scheduler::scheduler: Job started   job=outbox_indexing
INFO memory_scheduler::scheduler: Job completed job=outbox_indexing duration_ms=8911
```

The README now states this instead of implying `add` then `search` is instant.

### Step 5 — Ask for it back

```console
$ memory search "which JWT signing algorithm did we pick" --top 5
{"status":"ok","query":"which JWT signing algorithm did we pick","results":[{"agent":"claude",
"doc_id":"01M1AC7CSHMS9XN65TPACNBHDF","doc_type":"event","metadata":{"agent":"claude",
"memory_kind":"observation","timestamp_ms":"1788128506673"},"score":0.01270491722971201,
"source_layer":"bm25","text_preview":"We chose RS256 over HS256 for the auth service JWTs"}],
"meta":{"retrieval_ms":1,"tokens_estimated":88,"confidence":0.01270491722971201}}

$ memory search "rate limiting gateway" --format json | jq '.results[0].text_preview'
"Rate limiting lives in the gateway, not the auth service"
```

`text_preview` being populated on an `doc_type: event` result is the Phase 54.5
`TEXT | STORED` fix visible end-to-end: before it, event hits came back with
empty previews.

**Retrieval caveat found and documented:** BM25 does not stem.

```console
$ memory search "jwt" --format json
{"status":"ok","query":"jwt","results":[],...}

$ memory search "JWTs" --format json | jq -r '.results[0].text_preview'
We chose RS256 over HS256 for the auth service JWTs
```

The README's second example was changed from `jwt` to a query that BM25 can
actually answer, and the status table now says exact-token, no stemming.

### Step 6 — Wire it to your agent

```console
$ memory-installer install --agent claude --project --dry-run
[DRY-RUN] CREATE .../.claude/plugins/memory-plugin/commands/memory-search.md   1631 bytes
[DRY-RUN] CREATE .../.claude/plugins/memory-plugin/commands/memory-recent.md   1626 bytes
[DRY-RUN] CREATE .../.claude/plugins/memory-plugin/commands/memory-context.md  2082 bytes
[DRY-RUN] CREATE .../.claude/plugins/memory-plugin/commands/memory-setup.md   15679 bytes
[DRY-RUN] CREATE .../.claude/plugins/memory-plugin/commands/memory-status.md  10940 bytes
[DRY-RUN] CREATE .../.claude/plugins/memory-plugin/commands/memory-config.md  12702 bytes
```

And the Phase 57 scope trim, verified at the CLI:

```console
$ memory-installer install --agent opencode --project
error: invalid value 'opencode' for '--agent <AGENT>'
  [possible values: claude, gemini, codex, copilot, skills]
$ echo $?
2
```

Previously this exited 0 and wrote no files.

---

## What this transcript does *not* establish

Stated explicitly, so nobody reads more into it than it supports:

- **Vector search was not exercised** — the model download is blocked in this
  environment. BM25 only.
- **No hooks were installed into a live agent.** Step 6 was run as `--dry-run`;
  the hook-driven capture loop is covered by the bats suites in CI, not here.
- **This is not a performance measurement.** `retrieval_ms: 1` on a two-event
  store is not a benchmark. See `docs/benchmarks.md`.
- **macOS was not tested.** The build prerequisites for macOS in the README are
  taken from the CI workflow, which does run on `macos-latest`.
