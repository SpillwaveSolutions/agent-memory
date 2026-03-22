# Project Milestones: Agent Memory

## v2.7 Multi-Runtime Portability (Shipped: 2026-03-22)

**Delivered:** Rust-based multi-runtime installer that converts canonical Claude plugin source into runtime-specific installations for 6 targets, replacing 5 manually-maintained adapter directories with a single conversion pipeline.

**Phases completed:** 45-50 (6 phases, 11 plans)

**Key accomplishments:**

- `memory-installer` crate with `RuntimeConverter` trait and 6 runtime converters (Claude, Gemini, Codex, Copilot, Skills, OpenCode stub)
- Plugin parser with 2-level discovery (installer-sources.json → marketplace.json) and gray_matter frontmatter extraction
- Centralized tool mapping tables: 11 Claude tool names mapped across 6 runtimes with compile-time exhaustive match expressions
- format!-based YAML/TOML emitters with proper quoting, block scalars, and path rewriting helpers
- 7 E2E integration tests proving full convert-and-write pipeline for all runtimes
- 3 old adapter directories archived with README stubs (51 files, 12K+ lines removed)

**Known Gaps:**

- OC-01–06: OpenCode converter is a stub (deferred — OpenCode runtime format still evolving)

**Stats:**

- ~56,400 total LOC Rust across 15 crates
- 3,700 LOC in memory-installer crate
- 111 cargo tests (104 unit + 7 integration)
- Timeline: 2026-03-17 → 2026-03-22 (5 days)

---

## v2.5 Semantic Dedup & Retrieval Quality (Shipped: 2026-03-10)

**Delivered:** Ingest-time semantic dedup via vector similarity gate with configurable threshold, query-time stale filtering with time-decay and supersession detection, and 10 E2E tests proving the complete pipeline.

**Phases completed:** 35-38 (11 plans total)

**Key accomplishments:**

- InFlightBuffer (256-entry) + DedupConfig for within-session duplicate detection with configurable 0.85 similarity threshold
- Store-and-skip-outbox ingest wiring — duplicates preserved in RocksDB but excluded from BM25/HNSW indexes
- Cross-session dedup via CompositeVectorIndex querying HNSW + InFlightBuffer (structural events bypass)
- StaleFilter with exponential time-decay (14-day half-life), supersession detection, and high-salience kind exemptions
- StalenessConfig end-to-end propagation from config.toml through daemon startup to RetrievalHandler
- 10 E2E tests proving dedup, stale filtering, and fail-open behavior through complete pipeline

**Stats:**

- 48,282 total LOC Rust
- 4 phases, 11 plans, 42 commits
- 5 days from start to ship (2026-03-05 → 2026-03-10)

**Git range:** `feat(35-01)` → `docs(v2.5)`

**What's next:** Cross-project memory, admin dedup dashboard, or v2.6 enhancements

---

## v2.3 Install & Setup Experience (Shipped: 2026-02-12)

**Delivered:** Step-by-step install/config documentation with wizard-style setup skills, plus a performance benchmark harness with baseline metrics across all retrieval layers.

**Phases completed:** 28-29 (2 plans total)

**Key accomplishments:**

- Quickstart, Full Guide, and agent setup documentation for all 4 adapters (Claude Code, OpenCode, Gemini CLI, Copilot CLI)
- Four setup skills (install/configure/verify/troubleshoot) with wizard-style confirmation flows
- Performance benchmark harness (`perf_bench`) measuring ingest, TOC, BM25, vector, topic graph latency
- Baseline metrics for all tier/mode combinations (small/medium x cold/warm) with p50/p90/p99 percentiles
- Benchmark documentation with usage, outputs, and baseline update workflow

**Stats:**

- 44,912 total LOC Rust
- 2 phases, 2 plans, 9 commits
- 25 files changed, 3,210 lines added
- 1 day from start to ship (2026-02-12)

**Git range:** `docs(28-01)` → `feat(benchmarks)`

**What's next:** Cross-project memory, semantic deduplication, or v2.4 enhancements

---
## v2.2 Production Hardening (Shipped: 2026-02-11)

**Delivered:** Production-hardened system with all stub RPCs wired, 29 E2E tests across 7 files, and dedicated E2E CI job in GitHub Actions required for PR merge.

**Phases completed:** 24-27 (10 plans total)

**Key accomplishments:**

- All gRPC stub RPCs wired (GetRankingStatus, PruneVectorIndex, PruneBm25Index)
- ListAgents session_count fixed via event scanning (was returning 0)
- Agent field added to TeleportResult and VectorTeleportMatch for cross-agent attribution
- 29 E2E tests across 7 files: pipeline, BM25, vector, topic graph, multi-agent, degradation, error paths
- Dedicated E2E CI job in GitHub Actions with separate pass/fail reporting
- E2E tests required for PR merge via ci-success gate

**Stats:**

- 43,932 total LOC Rust
- 4 phases, 10 plans, 17 commits
- 1 day from start to ship (2026-02-11)

**Git range:** `feat(24-01)` → `feat(27-01)`

**What's next:** Performance benchmarks, cross-project memory, or v2.3 enhancements

---

## v2.1 Multi-Agent Ecosystem (Shipped: 2026-02-10)

**Delivered:** Multi-agent ecosystem with 4 adapter plugins (Claude Code, OpenCode, Gemini CLI, Copilot CLI), cross-agent discovery (agent listing, activity timeline, topic-by-agent), and CLOD universal command format.

**Phases completed:** 18-23 (22 plans total)

**Key accomplishments:**

- Agent tagging infrastructure — Event.agent field, TocNode.contributing_agents, AgentAdapter trait SDK
- OpenCode plugin — 3 commands, 5 skills, navigator agent, TypeScript event capture plugin
- OpenCode event capture — agent field through ingest-to-retrieval pipeline, multi-agent query results
- Gemini CLI adapter — shell hook handler, TOML commands, skills with embedded navigator, install skill
- Copilot CLI adapter — session ID synthesis, skills, .agent.md navigator, plugin.json manifest
- Cross-agent discovery — ListAgents/GetAgentActivity RPCs, agent-filtered topics, CLOD spec + converter CLI
- Comprehensive documentation — cross-agent usage guide, adapter authoring guide, UPGRADING.md

**Stats:**

- 155 files created/modified
- 31,544 lines added (40,817 total LOC Rust)
- 6 phases, 22 plans, 76 commits
- 2 days from start to ship (2026-02-09 → 2026-02-10)

**Git range:** `feat(18-01)` → `docs(phase-23)`

**What's next:** E2E automated tests, performance benchmarks, or v2.2 enhancements

---

## v2.0.0 Scheduler+Teleport (Shipped: 2026-02-07)

**Delivered:** Full cognitive architecture with layered search (Agentic TOC → BM25 → Vector → Topics), ranking policy (salience, usage, novelty), and retrieval brainstem (intent routing, tier detection, fallback chains).

**Phases completed:** 10-17 including 10.5 (42 plans total)

**Key accomplishments:**

- Background scheduler with Tokio cron, timezone handling, overlap policy, and graceful shutdown
- Index-free agentic TOC search (Layer 2) — always works, foundation for graceful degradation
- BM25 teleport via Tantivy for keyword acceleration (Layer 3)
- Vector teleport via usearch HNSW with local all-MiniLM-L6-v2 embeddings (Layer 4)
- Topic graph memory with HDBSCAN clustering, time-decayed importance, and topic relationships (Layer 5)
- Memory ranking with salience scoring, usage tracking, novelty filtering, and index lifecycle automation (Layer 6)
- Agent retrieval policy with intent classification, tier detection, fallback chains, and skill contracts (Control Plane)

**Stats:**

- 107 files created/modified
- 27,204 lines added (229,862 total LOC Rust)
- 9 phases, 42 plans, ~150 tasks
- 7 days from start to ship (2026-01-31 → 2026-02-07)

**Git range:** `feat(10-01)` → `feat(17-06)`

**What's next:** Additional hook adapters (OpenCode, Gemini CLI), production hardening, or v2.1 enhancements

---

## v1.0.0 MVP (Shipped: 2026-01-30)

**Delivered:** Complete conversational memory system with TOC-based agentic navigation, provenance tracking via grips, Claude Code plugin with commands/agents, and automatic event capture via CCH hooks.

**Phases completed:** 1-8 (20 plans total)

**Key accomplishments:**

- RocksDB storage layer with 6 column families, time-prefixed keys, and crash recovery
- TOC hierarchy builder with automatic parent creation and rollup jobs (Year → Month → Week → Day → Segment)
- Grip provenance system linking TOC bullets to source evidence with context expansion
- gRPC service with IngestEvent, GetTocRoot, GetNode, BrowseToc, GetEvents, ExpandGrip RPCs
- Claude Code marketplace plugin with 3 commands and memory-navigator agent (99/100 skill grade)
- CCH hook integration via memory-ingest binary with fail-open behavior

**Stats:**

- 91 files created/modified
- 9,135 lines of Rust/TOML/Proto/Markdown
- 8 phases, 20 plans, ~85 tasks
- 2 days from start to ship (2026-01-29 → 2026-01-30)

**Git range:** `feat(01-00)` → `feat(08-01)`

**What's next:** Teleport indexes (BM25/vector search), additional hook adapters (OpenCode, Gemini CLI), or production hardening

## v2.4 Headless CLI Testing (Shipped: 2026-03-05)

**Phases completed:** 34 phases, 113 plans, 49 tasks

**Key accomplishments:**

- (none recorded)

---
