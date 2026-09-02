# Agent-Memory vs. the memory-layer field

**Status:** current as of 2026-08-30. Competitor facts below are cited to
public sources and were checked on that date; re-verify before republishing
this as a post — this space moves monthly.

**Rule for this document:** every claim about *our* system is either verifiable
in this repository today or is labelled as not-yet-true. That rule is why the
benchmark section says what it says.

---

## The one-line difference

Mem0, Zep, MemMachine, and Letta are memory layers you *call*. Agent-Memory is
a memory layer that *watches* — it sits beside the CLI, captures what actually
happened through hooks, and never asks the model to spend tokens deciding what
to remember.

Everything else in this document follows from that.

---

## Where we are structurally different

Three dimensions where the difference is architectural rather than a matter of
tuning. These are the only three we lead with.

### 1. Passive, zero-token capture

Every hosted memory layer has the same capture contract: the agent decides
what is worth remembering and calls `add()` / `store()` / a memory tool. That
decision costs tokens on every turn, and it is skipped exactly when the context
window is under pressure — which is exactly when memory matters most.

Agent-Memory's capture path is CLI hooks (`SessionStart`, `UserPromptSubmit`,
`PostToolUse`, `Stop`) piping events into `memory-ingest`. The agent is not in
the loop and pays nothing. This is also why our event log is
*ground-truth-preserving* by construction: we store what happened, and derive
summaries later, out of band.

MemMachine independently arrived at the ground-truth-preserving half of this —
it stores raw episodes and minimises routine LLM extraction
([MemMachine paper](https://arxiv.org/html/2604.04853v1)). It still captures
through an API the agent calls. The zero-token half is ours.

### 2. Local-first by default, not as a deployment option

The event log (RocksDB), the BM25 index (Tantivy), the vector index (HNSW),
and the embedding model (Candle) all run on the developer's machine. Nothing
leaves it unless you opt into an LLM summarizer or LLM reranking and supply a
key.

This is not "we also have a self-hosted tier". There is no service to phone
home to. For consultants under client NDAs, regulated teams, and anyone whose
conversation history is the sensitive artifact, that is a categorical
difference rather than a pricing one.

### 3. Cross-CLI, because memory lives beside the CLI

A single store is reachable from any runtime that can run a hook or pipe an
event. That is a direct answer to the portability gap in vendor-native memory:
Claude Code's Auto Memory is scoped to one project and does not travel to other
tools or repositories
([overview](https://www.mindstudio.ai/blog/claude-code-memory-levels-explained-6-layers-claude-md-cross-tool-shared-memory)).

Our supported surface is deliberately narrower than it was — see
[README, "Supported surface"](../../README.md#supported-surface). Two Tier 1
runtimes we actually gate on beats six runtimes we cannot maintain.

---

## Head-to-head

| Dimension | Agent-Memory | Mem0 | Zep (Graphiti) | MemMachine | Letta |
|---|---|---|---|---|---|
| **Memory model** | Append-only event log + time-hierarchical TOC (Year→…→Segment) + grips | Three-tier hierarchy (user/session/agent): vector + graph + key-value | Temporal knowledge graph with explicit fact-validity intervals | Working + episodic + profile memory over preserved raw episodes | OS-style virtual memory; context paged in and out |
| **Capture cost** | **Zero tokens** — CLI hooks, agent not involved | Agent calls the API | Agent calls the API | Agent calls the API / MCP | Agent synthesizes memory during the conversation |
| **What is stored** | Raw events, immutable; summaries derived out of band | Extracted facts | Extracted facts + relations, temporally scoped | Raw episodes, minimal routine extraction | Synthesized state |
| **Locality / privacy** | Local-first; no service; keys only for optional LLM steps | Hosted or self-host | Hosted or self-host | Self-hostable server | Self-hostable server |
| **Reach** | Cross-CLI, one store per machine | SDK-reachable from anything | SDK-reachable from anything | Python/TS SDK, REST, MCP | Agent framework |
| **Provenance** | Grips: every excerpt links to the source events | Source attribution | Graph edges carry provenance and validity | Ground-truth episodes retained | Varies |
| **Evolution over time** | Time hierarchy is the primary axis; navigate by *when* | Consolidation over facts | Fact validity intervals — the strongest temporal story in the field | Contextual expansion around matches | Paging, not history |

Sources for the competitor columns:
[framework survey](https://www.graphlit.com/blog/survey-of-ai-agent-memory-frameworks),
[five-system comparison](https://medium.com/@wasowski.jarek/i-compared-5-ai-agent-memory-systems-across-6-dimensions-none-wins-6a658335ed0a),
[MemMachine](https://memmachine.ai/),
[Zep/Graphiti](https://www.graphlit.com/blog/survey-of-ai-agent-memory-frameworks).

### Where they are ahead of us

Stated plainly, because a comparison table that only flatters us is marketing:

- **Zep's temporal knowledge graph** models fact *validity* — "this was true
  between March and June". We model *when it was said*, which is a weaker
  claim. If your question is "what is currently true about this customer",
  Zep's model is the better fit.
- **Mem0 and MemMachine publish benchmark numbers we do not have.** See below.
- **Letta's paging model** solves a problem we do not attempt: keeping a
  long-running agent coherent inside one very long task.
- **Everyone else has an SDK story.** We have a gRPC daemon, a CLI, and hooks.
  If you are building a product rather than working in a terminal, they are
  easier to adopt today.

---

## Benchmarks: what we can and cannot say

**We are not making a comparative accuracy claim.** Here is the whole basis
for that decision.

MemMachine reports **0.9169 on LoCoMo** with `gpt-4.1-mini`, above published
Mem0, Zep, Memobase, LangMem, and OpenAI baselines
([paper](https://arxiv.org/pdf/2604.04853)).

What this repository has committed, in `benchmarks/results/`:

| Artifact | What it is | Why it is not a competitive number |
|---|---|---|
| `locomo-smoke.json` | 1 conversation, 4 questions, `metric = context_hit_rate`, `judge = mock`, score 0.5 | A mock judge on a 4-question fixture. It measures whether the harness works, not whether the memory is good. It is not LoCoMo and is not labelled LoCoMo |
| `custom-harness-mock.json` | 25 fixture tests, `backend = mock`, 22 passing | The backend is in-process token-overlap retrieval. Its own `caveats` field says it is not a production quality number |

### Claims Ledger (quality artifacts, not competitor scores)

These rows exist so every README "Solid"/"Works" quality claim has a committed
file next to it. They are **not** LOCOMO numbers and they are **not**
commensurable with MemMachine / Mem0.

| Claim | Artifact | What the number actually is |
|---|---|---|
| Vector quality on paraphrases (QUAL-01) | [`semantic-hybrid.json`](../../benchmarks/results/semantic-hybrid.json) vs [`semantic-bm25.json`](../../benchmarks/results/semantic-bm25.json) | Mock hybrid recall@5 = **1.00** vs BM25 **0.00** on 16 tests whose relevant sessions share meaning but not tokens with the query. Mock vector is a committed paraphrase lexicon + TF-IDF cosine, not Candle/HNSW. `--layers` is mock-only; live `memory search` is always RouteQuery hybrid |
| Topic clustering (QUAL-02) | [`topics-quality.json`](../../benchmarks/results/topics-quality.json) | Purity and adjusted rand index from `TopicExtractor::cluster` on a synthetic 80-doc / 8-cluster TF-IDF corpus. Not Candle embeddings, not live TOC summaries |

A run against a real backend with a real LLM judge has not been performed.
Until one is committed next to the claim, this document, the README, and the
repository make **no accuracy comparison to any of the systems above**. If that
run lands and the score is not competitive, the plan is to publish the
methodology and the number without comparison marketing — the local-first,
zero-token, cross-CLI argument does not depend on winning LoCoMo.

We do have a real performance story with committed artifacts, and one retracted
claim: the "65 second TOC navigation" figure that circulated internally was a
harness defect — it timed ingest-time summarization rollup and labelled it
navigation. [docs/benchmarks.md](../benchmarks.md) has the full account.

---

## The platform-risk question, head-on

> "Anthropic and OpenAI are shipping native memory. Why would this survive?"

It is the right question and it deserves the answer before someone posts it in
a comment thread.

**What is true:** Claude Code has shipped Auto Memory on by default since
v2.1.59 (February 2026), and chat memory that carries summaries across
sessions reached all tiers in March 2026. Vendor-native memory is real, it is
free, and it is good enough for a large fraction of users. Anyone selling a
memory layer that competes on "the vendor has no memory" has already lost.

**What is also true:** vendor memory is vendor-shaped by construction.

1. **It is single-vendor.** What Claude Code learns in your repo is not
   available to Codex, Gemini CLI, or Copilot. Every CLI you add starts from
   zero. Developers running more than one agent — an increasingly normal
   setup — get N disconnected memories.
2. **It is non-portable.** There is no export that another tool can consume.
   Switching runtimes means abandoning history, which is a real switching cost
   that benefits the vendor, not you.
3. **It is scoped to the vendor's unit of work.** Auto Memory is per-project;
   insights captured in repo A stay in repo A. Cross-project questions — "have
   I solved this auth problem before, anywhere?" — are outside its model.
4. **It is not yours.** It lives in the vendor's product boundary, on the
   vendor's retention policy, on the vendor's roadmap.

**So the framing is not "instead of".** The native layer deepens one tool; the
portable layer keeps every tool on the same page. Agent-Memory's bet is that
the cross-CLI, local, exportable layer is the part vendors are structurally
unlikely to build, because building it well means making their users easier to
leave.

**The honest risk:** if a developer only ever uses one CLI and does not care
where their history lives, native memory is sufficient and we are not needed.
That is a real segment, and it is not our segment.

---

## Who this is for

- Developers running **more than one agent CLI** who are tired of re-explaining
  the same decisions to each of them
- Anyone whose conversation history is **sensitive by default** — client work
  under NDA, regulated environments, security research
- People who want memory that costs **nothing per turn**, because they have
  watched an agent skip its own memory tool when the context filled up

## Who this is not for

- Single-CLI users happy with vendor-native memory
- Teams that need a hosted, multi-tenant, SLA-backed service today
- Product builders who need an SDK now — the surface here is a daemon, a CLI,
  and hooks

---

## Claims ledger

Anything in this document that could go stale, with where to re-check it.

| Claim | Source of truth | Last checked |
|---|---|---|
| MemMachine LoCoMo 0.9169 (`gpt-4.1-mini`) | [arXiv 2604.04853](https://arxiv.org/pdf/2604.04853) | 2026-08-30 |
| Zep models fact-validity intervals | [Graphlit survey](https://www.graphlit.com/blog/survey-of-ai-agent-memory-frameworks) | 2026-08-30 |
| Mem0 three-tier vector + graph + KV | [Graphlit survey](https://www.graphlit.com/blog/survey-of-ai-agent-memory-frameworks) | 2026-08-30 |
| Letta OS-style virtual memory paging | [five-system comparison](https://medium.com/@wasowski.jarek/i-compared-5-ai-agent-memory-systems-across-6-dimensions-none-wins-6a658335ed0a) | 2026-08-30 |
| Claude Code Auto Memory default-on, per-project | [Claude Code memory levels](https://www.mindstudio.ai/blog/claude-code-memory-levels-explained-6-layers-claude-md-cross-tool-shared-memory) | 2026-08-30 |
| Our own committed benchmark artifacts | `benchmarks/results/` in this repo | 2026-08-30 |
| Our own capability status | [README status table](../../README.md#status-what-is-solid-what-is-not) | 2026-08-30 |
