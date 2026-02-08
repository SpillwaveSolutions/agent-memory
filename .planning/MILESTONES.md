# Project Milestones: Agent Memory

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

---
