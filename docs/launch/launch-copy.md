# Launch copy — v3.1.0

Drafts for Phase 58. Nothing here has been posted. Read the notes at the bottom
before posting anything.

---

## Show HN

**Title** (80 char limit; the guideline is "Show HN: " plus a plain description,
no adjectives, no exclamation marks):

```
Show HN: Agent-Memory – local-first memory for AI coding agents, zero-token capture
```

**Text:**

```
Every memory layer I looked at has the same capture contract: the agent decides
what is worth remembering and calls an API. That costs tokens on every turn, and
it gets skipped exactly when the context window is under pressure — which is when
memory matters most.

Agent-Memory captures passively instead. CLI hooks (SessionStart, UserPromptSubmit,
PostToolUse, Stop) pipe events into an append-only RocksDB log. The agent is not in
the loop and pays nothing. Retrieval is a time-hierarchical table of contents the
agent drills down — Year → Month → Week → Day → Segment → Grip — rather than a
similarity search over everything, so answering "what were we talking about last
week?" doesn't mean loading last week into context.

It's Rust. Everything is local: the event log, a Tantivy BM25 index, an HNSW vector
index, and Candle embeddings all run on your machine. Nothing leaves it unless you
turn on LLM summarization or reranking and supply a key.

The README has a status table saying which parts are solid, which are experimental,
and which are not implemented, because the last four phases of this project were
spent finding out that my own verification process had been grading claims instead
of behavior. Some specifics, since they're more interesting than the pitch:

- A crate I'd certified as "integrated" was unreachable from any shipped binary
- A benchmark showing 64.6s TOC navigation was a harness defect — it timed
  ingest-time summarization and labelled it navigation. Real warm p50 is 0.13ms
- The quickstart in my own README returned zero results for content just ingested,
  on a fresh install, with exit code 0 and no error

There is no benchmark comparison in the repo. The only committed results are
mock-backend runs that test the harness, not the retrieval quality, and I'm not
publishing a LoCoMo number until I've run it properly.

https://github.com/SpillwaveSolutions/agent-memory
```

---

## r/rust

**Title:**

```
Agent-Memory 3.1: local-first conversational memory for AI coding agents (Rust, RocksDB + Tantivy + HNSW)
```

**Body:**

```
Local-first memory layer for AI coding CLIs — the agent's conversation history
becomes a queryable store instead of evaporating at the end of a session.

Stack, since that's what this sub actually cares about:

- **RocksDB** for the append-only event log (immutable; summaries derived out of band)
- **Tantivy** for BM25 keyword retrieval
- **usearch** HNSW for vector retrieval, with **Candle** running the embedding model
  locally — no Python, no inference service
- **tonic**/gRPC daemon, **clap** CLI, **tokio-cron-scheduler** for the background
  rollup and indexing jobs
- Toolchain pinned via `rust-toolchain.toml` after a floating-stable Clippy lint
  reddened main overnight

The design bit I'd be interested in feedback on: retrieval is *navigational* rather
than purely similarity-based. A time hierarchy (Year → Month → Week → Day → Segment)
is built by scheduled rollup jobs, and queries drill down it, reading a summary at
each level. Three layers (BM25, vector, topic graph) feed a fusion step. The intent
is that answering "what did we decide about auth last month" reads a handful of
summary nodes rather than embedding-searching the entire corpus.

This release added no features. It was four phases of closing the gap between what
the project claimed and what it did — including a benchmark number that turned out
to be timing the wrong thing by five orders of magnitude, and a first-run bug where
the daemon accepted events and answered every query with an empty result set,
successfully, because the index directories didn't exist yet.

MIT. Builds on stable 1.97. Feedback on the retrieval architecture very welcome.

https://github.com/SpillwaveSolutions/agent-memory
```

---

## r/LocalLLaMA

**Title:**

```
Agent-Memory: fully local memory for coding agents — RocksDB + BM25 + local embeddings, nothing phones home
```

**Body:**

```
Built this because every agent memory option I found was a hosted API, and I do
client work where the conversation history is the sensitive artifact.

Everything runs on your box:

- Append-only event log (RocksDB)
- BM25 keyword index (Tantivy)
- Vector index (HNSW) with embeddings generated locally by Candle —
  all-MiniLM-L6-v2 downloaded once, then no network
- gRPC daemon + CLI

No account, no service, no telemetry. LLM summarization and LLM reranking are
optional and off unless you supply a key; without one it falls back to a heuristic
ranker and *says so* in the response payload rather than pretending.

Capture is passive — CLI hooks pipe events in, so the model never spends tokens
deciding what to remember and can't skip it when the context fills up. Works
alongside Claude Code and Codex CLI today (Gemini and Copilot are best-effort).

Honest status, since this sub can smell marketing: BM25 works well and is exact-token
(no stemming). Vector search works but the retrieval quality is not benchmarked — the
only committed benchmark artifacts are mock-backend runs that test the harness, and I
am not publishing a LoCoMo number until I've run a real one. The README has a table
listing what's solid, what's experimental, and what's not implemented.

MIT, Rust, Linux/macOS.

https://github.com/SpillwaveSolutions/agent-memory
```

---

## Before posting

1. **Tag `v3.1.0` first** so the release links resolve and Show HN visitors can
   download binaries rather than compile.
2. **Set the GitHub repo description and topics** (`ai-agents`, `memory`, `rust`,
   `claude-code`, `local-first`) and enable Discussions. A Show HN landing on a
   repo with no description reads as abandoned.
3. **Post the blog post first**, then link it from the HN thread as a comment
   rather than submitting it as the HN URL — the repo is the better submission.
4. **Sequence, don't shotgun.** HN and one subreddit on day one; the second
   subreddit later. Simultaneous posts read as a launch campaign and get less
   patience from commenters.
5. **Be around for the first three hours.** The platform-risk question ("won't
   Anthropic just build this in?") will come up; the answer is in
   `docs/positioning/agent-memory-vs-competition.md` and should be given in your
   own words, not pasted.
6. **Do not add a benchmark claim under pressure.** If someone asks how it scores
   on LoCoMo, the answer is "I haven't run it properly, so I'm not going to quote a
   number" — which is a stronger answer here than a weak score, and the whole
   reason the last milestone existed.
