# Research Summary: Agent Memory System

**Researched:** 2026-01-29
**Confidence:** HIGH

## Executive Summary

This research validates the user's architectural decisions and identifies key implementation considerations for building a conversational memory system with TOC-based agentic navigation.

**Key validation:** The TOC-first, append-only, time-primary architecture is **novel and differentiated**. No existing memory system (Letta, Mem0, Graphiti, LangGraph) uses table-of-contents hierarchy as the primary navigation axis.

---

## Stack Decisions (STACK.md)

### Recommended Core Stack

| Component | Crate | Version | Confidence |
|-----------|-------|---------|------------|
| Async Runtime | tokio | 1.49.0 | HIGH |
| Storage | rocksdb | 0.24.0 | HIGH |
| gRPC | tonic + prost | 0.14.3 | HIGH |
| BM25 Search | tantivy | 0.25.0 | HIGH |
| Vector Index | hnsw_rs | 0.3.3 | HIGH |
| IDs | ulid | 1.2.1 | HIGH |
| Serialization | serde + serde_json | 1.0 | HIGH |

### Key Findings

1. **Tokio 1.49.0 LTS** (until Sep 2026) provides stability guarantees
2. **Tonic 0.14.3** is becoming official Rust gRPC (partnership with gRPC team)
3. **RocksDB** is correct for append-only; alternatives (sled, redb, Fjall) are either unstable or wrong data structure
4. **Pure Rust** for search/vector (Tantivy, hnsw_rs) avoids C++ binding complexity
5. **MSRV is Rust 1.82** (prost 0.14.3 requirement)

### Watch Out For

- Windows RocksDB builds need explicit CI testing
- `protoc` required system-wide for tonic-build

---

## Feature Landscape (FEATURES.md)

### Table Stakes (Must Have)

- Persistent storage across sessions
- Conversation history append
- Basic retrieval by time
- Full-text search
- User/agent scoping
- Read/query API
- Write/ingest API

### Core Differentiators (Unique to This System)

| Feature | Value | Competitor Comparison |
|---------|-------|----------------------|
| **TOC hierarchy navigation** | Deterministic drill-down without LLM inference | Unique - no existing system uses this |
| **Grips (excerpt + pointer)** | Provenance with verifiable citations | Unique - PROV-AGENT paper validates need |
| **Teleports (index jumps)** | O(1) access to specific points | Unique - others use ANN or graph traversal |
| **Hook-based passive capture** | Zero token overhead | Unique - Letta/Mem0 consume tokens for memory ops |
| **Time as primary axis** | Optimized for "last week" queries | TSM paper: 22.56% improvement vs dialogue-time |
| **Append-only immutability** | Full audit trail, no data loss | Simpler/safer than Letta's updates or Mem0's merges |

### Anti-Features (Explicitly Avoid)

- Vector search as primary retrieval (fails temporal queries)
- Automatic fact extraction (token cost + hallucination risk)
- Self-modifying memory (security vulnerability per ZombieAgent research)
- Always-on context injection (token waste)
- Complex graph relationships (unnecessary for this use case)
- LLM-in-the-loop for storage (latency, cost)

---

## Architecture Patterns (ARCHITECTURE.md)

### Component Boundaries

```
Hook Handler (external) --gRPC--> Memory Daemon
                                      |
                                 Service Layer (tonic)
                                      |
                                 Domain Layer (TOC, Events, Grips)
                                      |
                                 Storage Layer (RocksDB)
                                      |
                         +------------+------------+
                         |            |            |
                    Outbox       Tantivy       HNSW
                    Relay        (BM25)      (Vector)
```

### Data Flows

1. **Ingestion**: Hook → gRPC → Validate → Write events CF + outbox CF
2. **TOC Building**: Timer/threshold → Read events → Segment → Summarize → Write TOC
3. **Query**: Agent → GetTocRoot → GetNode (drill) → GetEvents (last resort)
4. **Teleport**: Query indexes → Return node_ids/grip_ids → Agent navigates from entry point

### Key Patterns to Follow

1. **Column Family Isolation** - Separate CFs for events, toc_nodes, outbox, grips, checkpoints
2. **Append-Only with Versioned TOC** - Events immutable; TOC nodes versioned
3. **Transactional Outbox** - Atomic write of data + outbox entry; async relay to indexes
4. **Checkpoint-Based Crash Recovery** - Save progress markers; resume from checkpoint
5. **Segment Overlap** - 5 min or 500 tokens overlap for context continuity

### Workspace Structure

```
agent-memory/
├── proto/memory.proto
├── crates/
│   ├── memory-types/      # Event, TocNode, Grip
│   ├── memory-storage/    # RocksDB wrapper
│   ├── memory-domain/     # TOC builder, segmenter, summarizer
│   ├── memory-index/      # Tantivy, HNSW, outbox relay
│   ├── memory-service/    # gRPC handlers
│   └── memory-daemon/     # Binary entry point
└── hook-handler/          # Separate binary for hooks
```

---

## Critical Pitfalls (PITFALLS.md)

### Must Address in Phase 1 (Storage)

| Pitfall | Severity | Prevention |
|---------|----------|------------|
| RocksDB write amplification | CRITICAL | Use FIFO or Universal compaction, not Level |
| Key design preventing time scans | HIGH | Time-prefix keys: `evt:{ts}:{ulid}` |
| Out-of-order events | HIGH | Idempotent writes, source timestamps |
| Memory consumption during compaction | MEDIUM | 50-60% memory budget, limit concurrency |

### Must Address in Phase 2 (TOC)

| Pitfall | Severity | Prevention |
|---------|----------|------------|
| Summarization information loss | CRITICAL | Fact extraction layer before summarization |
| TOC as ground truth | CRITICAL | Navigation-only API; always verify against events |
| Over-engineering TOC levels | LOW | Start with session+day only |

### Must Address in Phase 3+ (Indexes)

| Pitfall | Severity | Prevention |
|---------|----------|------------|
| Embedding model version drift | CRITICAL | Version metadata, atomic re-indexing |
| BM25/vector preprocessing mismatch | LOW | Shared preprocessing module |
| Recency bias burying old facts | HIGH | Fact type classification, importance anchoring |

### Your Non-Goals as Protection

| Non-Goal | Pitfalls Prevented |
|----------|-------------------|
| No graph database | Over-engineering, graph complexity |
| No multi-tenant | Permission bugs, key collisions |
| No deletes | Consistency bugs, tombstone accumulation |
| No premature optimization | Wasted effort on unvalidated features |

---

## Roadmap Implications

Based on research, suggested phase structure:

### Phase 0: Foundation (MVP)
- Storage Layer (RocksDB with correct compaction)
- Domain Types (Event, TocNode, Grip)
- Service Layer (gRPC scaffolding)
- IngestEvent RPC
- Hook Handler Client
- Basic TOC Building (segments)
- Query RPCs (GetTocRoot, GetNode, GetEvents)

### Phase 1: Quality & Trust
- Grips + provenance
- Summary-to-Grip linking
- Better segmentation (token-aware)

### Phase 2: Teleports
- Outbox infrastructure
- BM25 index (Tantivy)
- Vector index (HNSW)
- TeleportQuery RPC with fusion

### Phase 3: Resilience
- Parallel scan infrastructure
- Range-limited scan by TOC bounds
- Fallback path integration

---

## Open Questions

1. **Segment boundary algorithm**: Optimal combination of time (30 min) and tokens (4K) thresholds?
2. **Summarizer trait API**: Exact interface for pluggable LLM summarizers?
3. **Multi-project discovery**: How does daemon find/select per-project stores?
4. **Hook format normalization**: How different are Claude Code vs OpenCode vs Gemini CLI hook payloads?
5. **Embedding generation**: Local (ONNX/llama-cpp-rs) vs API for vector teleports?

---

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Stack recommendations | HIGH | Verified via crates.io, docs.rs |
| Feature differentiation | HIGH | Validated no existing system uses TOC approach |
| Architecture patterns | HIGH | Established patterns (outbox, column families) |
| Pitfall identification | HIGH | RocksDB, summarization issues well-documented |
| Phase structure | MEDIUM | Logical but may need adjustment |

---

*Research complete. Ready for requirements definition and roadmap creation.*
