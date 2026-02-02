# Hierarchical Vector Indexing System - Product Requirements Document

**Version:** 2.0
**Date:** 2026-02-01
**Phase:** 12 (Vector Teleport) + 13 (Outbox Index Ingestion)
**Status:** Draft for Architecture Review

---

## 1. Executive Summary

### Current System Capabilities

The agent-memory system already provides:

| Capability | Phase | Description |
|-----------|-------|-------------|
| **Hierarchical TOC** | Phase 2 | Year > Month > Week > Day > Segment organization |
| **Provenance via Grips** | Phase 3 | Excerpts with event pointers for citation |
| **Agentic Search** | Phase 10.5 | Index-free term-overlap search via TOC navigation |
| **BM25 Teleport** | Phase 11 | Tantivy full-text search for keyword-based teleportation |

### What This PRD Adds

**Vector Teleport (Phase 12)**: Semantic similarity search using a local HNSW vector index on TOC summaries and grip excerpts. This enables finding conceptually related conversations even when keywords differ.

**Index Lifecycle Management**: Optional policies that prune old embeddings from the vector index to bound storage growth, while preserving the append-only primary data architecture.

### Key Design Principles

1. **Append-Only Primary Storage**: Events, TOC nodes, and grips are never deleted
2. **Rebuildable Indexes**: Vector index is an accelerator, not source of truth
3. **Local-First**: No external API dependencies for embeddings
4. **Complementary Search**: Vector search enhances, doesn't replace agentic or BM25 search
5. **Graceful Degradation**: System works without vector index (falls back to BM25/agentic)
6. **Fully Optional**: Users can disable vector search entirely via `vector_index.enabled: false`

---

## 2. Problem Statement

### Current State

The agent-memory system has powerful search capabilities through Phase 10.5 (agentic TOC search) and Phase 11 (BM25 teleport). However, both rely on keyword matching:

- **Phase 10.5 Agentic Search**: Simple term-overlap matching works without any index, but misses variations ("JWT" won't find "token")
- **Phase 11 BM25 Search**: Handles stemming and term frequency, but still fundamentally keyword-based

### Pain Points

1. **Semantic Recall Gap**: Related conversations aren't found when keywords differ
   - "JWT authentication" won't find "token-based login"
   - "fix the bug" won't find "resolved the issue"
   - "database schema" won't find "data model design"

2. **Conceptual Similarity Missing**: Users think in concepts, not keywords
   - Want: "what did we discuss about security?"
   - Current: Must know specific terms used ("OAuth", "RBAC", "OWASP")

3. **Index Growth Concern**: Over years of continuous use, unlimited vector storage could grow unbounded

### Opportunity

Add semantic search that understands meaning, not just keywords. Position it as a complement to existing search methods, with optional lifecycle policies to manage storage growth.

---

## 3. Solution Overview

### Primary Solution: Vector Teleport

Add HNSW (Hierarchical Navigable Small World) vector index for semantic teleportation:

```
Query: "authentication system design"
                    │
                    ▼
          ┌─────────────────┐
          │  Embed Query    │ (local model, ~30ms)
          └────────┬────────┘
                   │
                   ▼
          ┌─────────────────┐
          │  HNSW Search    │ (cosine similarity, ~10ms)
          └────────┬────────┘
                   │
                   ▼
┌──────────────────────────────────────────────────┐
│  Results (by similarity):                        │
│  1. toc:day:2026-01-28 - "OAuth2 implementation" │
│  2. toc:segment:2026-01-25:abc - "JWT tokens"    │
│  3. grip:1706400000:xyz - "session management"   │
└──────────────────────────────────────────────────┘
```

### Secondary Solution: Hybrid Search

Combine BM25 and vector scores for best-of-both-worlds ranking:

```
HybridSearch(query="JWT authentication")
                    │
        ┌───────────┴───────────┐
        ▼                       ▼
   BM25 Search             Vector Search
   (exact keyword)         (semantic similarity)
        │                       │
        └───────────┬───────────┘
                    ▼
           Reciprocal Rank Fusion
                    │
                    ▼
           Combined Results
```

### Index Lifecycle (NOT Data Deletion)

Key insight: **Index pruning ≠ data deletion**.

- Primary data (events, TOC nodes, grips) is immutable, stored forever in RocksDB
- Only the vector index has lifecycle management
- Older segment/grip embeddings can be removed from HNSW
- Agent can still find old content via agentic search (Phase 10.5)

```
┌─────────────────────────────────────────────────────────────┐
│                    DATA LIFECYCLE                            │
├─────────────────────────────────────────────────────────────┤
│  RocksDB (Immutable, Forever):                              │
│  ├── CF_EVENTS      ← Never deleted                         │
│  ├── CF_TOC_NODES   ← Never deleted                         │
│  ├── CF_GRIPS       ← Never deleted                         │
│  └── CF_EMBEDDINGS  ← Persisted embeddings (never deleted)  │
├─────────────────────────────────────────────────────────────┤
│  HNSW Index (Prunable Accelerator):                         │
│  ├── Recent segments  ← Indexed (searchable via vector)     │
│  ├── Old segments     ← Pruned (still searchable via BM25)  │
│  └── Month/Week/Day   ← Always indexed (coarse anchors)     │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Terminology Mapping

| Conceptual Term | Agent-Memory Implementation | Notes |
|-----------------|----------------------------|-------|
| Vector Teleport | HNSW-based semantic search | Finds similar meaning, not exact words |
| Embedding | 384-dim float vector | Generated by local MiniLM model |
| HNSW Index | Hierarchical Navigable Small World | Approximate nearest neighbor graph |
| Pruning | Removing old vectors from HNSW | NOT deleting from storage |
| Lifecycle | Index maintenance policy | Controls HNSW size, not data retention |
| Hybrid Search | BM25 + Vector RRF combination | Best of both ranking approaches |
| CF_EMBEDDINGS | RocksDB column family | Persistent embedding storage |
| MmapDirectory | Memory-mapped index | For Tantivy (BM25), not HNSW |

---

## 5. Goals & Objectives

### Primary Goals

| Goal | Description | Success Metric |
|------|-------------|----------------|
| **G1: Semantic Search** | Find conceptually related content | >20% recall improvement over BM25 alone |
| **G2: Local Inference** | No external API dependency | Works offline, <50ms embedding latency |
| **G3: Fast Search** | Sub-second search latency | p99 < 200ms for 10K vectors |
| **G4: Bounded Growth** | Manageable index size | <500MB after 1 year with lifecycle |
| **G5: Graceful Fallback** | Works without vector index | BM25/agentic search unaffected |

### Non-Goals (Out of Scope)

| Non-Goal | Reason |
|----------|--------|
| Delete primary data | Append-only architecture is a core design principle |
| Replace agentic search | Vector search complements, doesn't replace |
| External vector DB | Pinecone, Weaviate, etc. add complexity and dependency |
| Real-time embedding | Batch processing is simpler and sufficient |
| Multi-tenant isolation | Single-user daemon per CLAUDE.md |

---

## 5.5 Optional Feature: User Opt-Out

### Vector Search is Completely Optional

Users may choose to disable vector search for any reason:

| Reason | Description |
|--------|-------------|
| **Resource constraints** | Embedding model uses ~200MB RAM, HNSW uses additional memory |
| **Storage concerns** | Even with lifecycle pruning, index adds disk usage |
| **Simplicity preference** | BM25 + agentic search may be sufficient for their needs |
| **Privacy concerns** | Some users prefer no ML inference on their data |
| **Performance focus** | Skip embedding overhead for faster overall performance |

### Configuration

```toml
# ~/.config/agent-memory/config.toml

[teleport.vector]
# Completely disables vector functionality
enabled = false
```

**Note:** See BM25 PRD Section 2 for `[teleport.bm25]` configuration.

### Behavior When Disabled

| Component | Behavior |
|-----------|----------|
| Embedding model | Not loaded (saves ~200MB RAM) |
| HNSW index | Not created (saves disk space) |
| CF_EMBEDDINGS | Not populated (no RocksDB overhead) |
| VectorTeleport RPC | Returns `UNAVAILABLE` status |
| HybridSearch RPC | Falls back to BM25-only (graceful) |
| GetVectorIndexStatus | Returns `enabled: false, ready: false` |

### Agent Skill Requirements

**CRITICAL**: Agent skills MUST handle the disabled case gracefully.

#### Before Using Vector Search

1. **Check availability first**: Call `GetVectorIndexStatus` RPC
2. **If disabled**: Use BM25 (Phase 11) or agentic search (Phase 10.5)
3. **Do NOT retry repeatedly**: If disabled, it won't become enabled during session

#### Decision Flow for Skills

```
Query arrives
    │
    ▼
GetTeleportStatus() + GetVectorIndexStatus()
    │
    ├─► vector.ready: true ─► Use HybridSearch or VectorTeleport
    │   bm25.ready: true
    │
    ├─► vector.ready: false ─► Check BM25 via GetTeleportStatus
    │                          │
    │                          ├─► bm25_healthy: true ─► TeleportSearch (BM25)
    │                          │
    │                          └─► bm25_healthy: false ─► SearchChildren (agentic)
    │
    └─► All disabled ────────► Use SearchChildren (agentic)
                               Always works, no index needed
```

**Note:** See BM25 PRD Appendix C for combined status check pattern.

#### Skill Documentation Pattern

Skills should document fallback behavior:

```markdown
## Search Capabilities

### Semantic Search (Optional)
- Requires: `vector_index.enabled: true`
- Method: HybridSearch or VectorTeleport RPC
- Best for: Finding conceptually related content

### Keyword Search (Default)
- Requires: Phase 11 (BM25 Teleport)
- Method: TeleportSearch RPC
- Best for: Exact keyword matching, always fast

### Agentic Search (Fallback)
- Requires: Nothing (always available)
- Method: SearchChildren RPC
- Best for: When indexes are unavailable or building
```

---

## 6. Success Metrics

### Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Semantic search latency p99 | < 200ms | Tracing spans |
| Query embedding latency | < 50ms | Tracing spans |
| Hybrid search latency p99 | < 300ms | Tracing spans |
| Index rebuild time (10K nodes) | < 60s | CLI timing |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Recall improvement over BM25 | > 20% | Benchmark suite |
| Precision at 10 | > 70% | Manual evaluation |
| User satisfaction | No complaints about relevance | User feedback |

### Efficiency Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Index size after 1 year | < 500MB | Disk usage |
| Memory usage (index loaded) | < 200MB | Process metrics |
| Embedding model size | < 100MB | Disk usage |

---

## 7. User Stories

### Agent User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-01 | Agent | search for semantically related conversations | I can find relevant context even with different phrasing |
| US-02 | Agent | combine keyword and semantic search | I get the best of both ranking approaches |
| US-03 | Agent | see similarity scores with results | I can prioritize which paths to explore |
| US-04 | Agent | search grips directly by meaning | I can find supporting evidence efficiently |

### Admin User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-05 | Admin | configure index lifecycle policies | I can bound storage growth |
| US-06 | Admin | rebuild the vector index | I can recover from corruption or upgrade models |
| US-07 | Admin | monitor index health and size | I can proactively manage storage |
| US-08 | Admin | disable vector search | I can fall back to BM25 if needed |

### Developer User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-09 | Developer | test vector search via CLI | I can debug search behavior |
| US-10 | Developer | see embedding/search latency | I can identify performance issues |
| US-11 | Developer | force re-embedding on model upgrade | I can update the semantic index |

---

## 8. Hierarchical Memory Model

The agent-memory system organizes conversations into a time-based hierarchy. Vector indexing adds embeddings at each level:

```
+------------------------------------------------------------------+
|  YEAR (toc:year:2026)                                            |
|  ├── Summary: "2026 focused on authentication and API design"    |
|  ├── Embedding: aggregated themes (768 dims)                     |
|  └── Indexed: optional (lowest priority, coarsest granularity)   |
+------------------------------------------------------------------+
|  MONTH (toc:month:2026-01)                                       |
|  ├── Summary: "January: OAuth2 implementation, bug fixes"        |
|  ├── Embedding: monthly themes (768 dims)                        |
|  └── Indexed: always (coarse semantic anchor)                    |
+------------------------------------------------------------------+
|  WEEK (toc:week:2026-W04)                                        |
|  ├── Summary: "Week 4: JWT debugging, token refresh"             |
|  ├── Embedding: weekly themes (768 dims)                         |
|  └── Indexed: always (medium granularity)                        |
+------------------------------------------------------------------+
|  DAY (toc:day:2026-01-30)                                        |
|  ├── Summary: "Resolved JWT expiry bug, added refresh logic"     |
|  ├── Embedding: daily themes (768 dims)                          |
|  └── Indexed: always (fine granularity)                          |
+------------------------------------------------------------------+
|  SEGMENT (toc:segment:2026-01-30:abc123) [LEAF NODE]             |
|  ├── Bullets: ["Fixed JWT expiry [grip:xyz]", ...]               |
|  ├── Embedding: segment content (768 dims)                       |
|  └── Indexed: recent only (prunable after N days)                |
+------------------------------------------------------------------+
|  GRIPS (grip:1706745600000:xyz)                                  |
|  ├── Excerpt: "The token was expiring prematurely because..."    |
|  ├── Embedding: excerpt content (768 dims)                       |
|  └── Indexed: recent only (prunable with parent segment)         |
+------------------------------------------------------------------+
```

### Navigation Flow with Vector Search

```
User: "What did we discuss about token authentication?"

1. VectorTeleport("token authentication")
   └── Returns: Day 2026-01-30 (similarity: 0.89)

2. Agent navigates to Day 2026-01-30
   └── GetNode("toc:day:2026-01-30")

3. Agent reads day summary, drills into segments
   └── Finds: Segment abc123 with JWT debugging bullets

4. Agent expands grip for full context
   └── ExpandGrip("grip:xyz") → "The token was expiring..."
```

---

## 9. Index Lifecycle Strategy

### Core Principle: Append-Only Data

The agent-memory system is built on append-only storage. This means:

- **Events are immutable**: Once ingested, never modified or deleted
- **TOC nodes are immutable**: Summaries are append-only, versions increment
- **Grips are immutable**: Excerpts with event pointers, never deleted

### What Lifecycle Management DOES NOT Do

- Delete events from CF_EVENTS
- Delete TOC nodes from CF_TOC_NODES
- Delete grips from CF_GRIPS
- Delete embeddings from CF_EMBEDDINGS

### What Lifecycle Management DOES

Prune vectors from the HNSW index (the ephemeral, rebuildable accelerator):

| Age | Segment Vectors | Grip Vectors | Day/Week/Month Vectors |
|-----|-----------------|--------------|------------------------|
| < 30 days | In HNSW | In HNSW | In HNSW |
| 30-90 days | Aggregated to Day | Removed | In HNSW |
| 90-365 days | Day vectors only | Removed | In HNSW |
| > 365 days | Week vectors only | Removed | In HNSW |

### Why This Works

When vectors are pruned from HNSW:

1. **Agentic Search (Phase 10.5)** still works - traverses TOC hierarchy, no index needed
2. **BM25 Search (Phase 11)** still works - indexes TOC text, not vectors
3. **Vector Search** finds month/week/day nodes, agent drills down from there
4. **Grips remain in RocksDB** - only vectors removed from HNSW

### Fallback Guarantee

```
Query: "authentication discussion from 2025"

Vector Index: No segment vectors (pruned after 1 year)
              ↓
           Month vectors available
              ↓
VectorTeleport → Month 2025-03 (similarity: 0.82)
              ↓
Agent uses SearchChildren (Phase 10.5) to find weeks
              ↓
Agent uses BM25 (Phase 11) to find specific days
              ↓
Agent reads TOC node, expands grips for context
```

---

## 10. Functional Requirements

### FR-01: Embedding Storage

**Description:** Persist embeddings in RocksDB for crash recovery and rebuild.

**Acceptance Criteria:**
- [ ] New column family `CF_EMBEDDINGS` stores embeddings
- [ ] Key format: `emb:{node_id}` for TOC nodes, `emb:grip:{grip_id}` for grips
- [ ] Value includes: embedding vector, model name, model version, timestamp
- [ ] Embeddings survive daemon restart
- [ ] Storage is efficient (no duplicate embeddings)

### FR-02: Local Embedding Model

**Description:** Generate embeddings locally without external API calls.

**Acceptance Criteria:**
- [ ] Uses sentence-transformers compatible model (e.g., all-MiniLM-L6-v2)
- [ ] Model weights cached in `~/.local/share/agent-memory/models/`
- [ ] Works offline after initial download
- [ ] Embedding latency < 50ms per text
- [ ] Batch embedding supported for efficiency

### FR-03: HNSW Vector Index

**Description:** Provide fast approximate nearest neighbor search.

**Acceptance Criteria:**
- [ ] Uses usearch or hnsw-rs library (local, no server)
- [ ] Index stored in `~/.local/share/agent-memory/vector-index/`
- [ ] Configurable HNSW parameters (M, ef_construction, ef_search)
- [ ] Cosine similarity metric
- [ ] Index persists across restarts
- [ ] Rebuild from CF_EMBEDDINGS if corrupted

### FR-04: VectorTeleport RPC

**Description:** Search by semantic similarity.

**Acceptance Criteria:**
- [ ] Input: query text, limit, optional level filter, include_grips flag
- [ ] Output: ranked results with node_id/grip_id, similarity score, title
- [ ] Returns embedding and search latency for observability
- [ ] Graceful error if index not available
- [ ] Minimum similarity threshold filter

### FR-05: HybridSearch RPC

**Description:** Combine BM25 and vector search results.

**Acceptance Criteria:**
- [ ] Input: query, limit, bm25_weight, vector_weight
- [ ] Output: combined results with individual and combined scores
- [ ] Uses reciprocal rank fusion (RRF) for score combination
- [ ] Configurable weights (default: 0.5/0.5)
- [ ] Falls back to BM25 only if vector index unavailable

### FR-06: RebuildVectorIndex RPC

**Description:** Admin command to rebuild the vector index.

**Acceptance Criteria:**
- [ ] Option 1: Rebuild from CF_EMBEDDINGS (fast, uses persisted)
- [ ] Option 2: Full rebuild with re-embedding (slow, regenerates all)
- [ ] Progress reporting during rebuild
- [ ] Non-blocking (runs in background)
- [ ] Status check via GetVectorIndexStatus

### FR-07: GetVectorIndexStatus RPC

**Description:** Health and metrics for vector index.

**Acceptance Criteria:**
- [ ] Reports: ready state, vector count, size bytes
- [ ] Reports: model name, dimension, last rebuild timestamp
- [ ] Prometheus metrics exposed for monitoring

### FR-08: Index Lifecycle Scheduler Job

**Description:** Periodic pruning of old vectors from HNSW.

**Acceptance Criteria:**
- [ ] Configurable retention days per level (segment, grip, day, week)
- [ ] Runs on schedule (default: daily at 3 AM)
- [ ] Batch processing to avoid blocking
- [ ] Logs pruned vector count
- [ ] Does NOT touch CF_EMBEDDINGS (only HNSW index)

### FR-09: CLI Commands

**Description:** Command-line interface for testing and administration.

**Acceptance Criteria:**
- [ ] `memory-daemon vector-search --query "text"` - Vector similarity search
- [ ] `memory-daemon hybrid-search --query "text"` - Combined search
- [ ] `memory-daemon rebuild-vector` - Rebuild index
- [ ] `memory-daemon vector-status` - Show index health
- [ ] All commands support JSON output format

---

## 11. Non-Functional Requirements

### NFR-01: Performance

| Metric | Requirement |
|--------|-------------|
| Query embedding latency | < 50ms (p99) |
| Vector search latency | < 100ms (p99) for 10K vectors |
| Hybrid search latency | < 300ms (p99) |
| Index rebuild rate | > 1000 vectors/second |
| Cold start (load index) | < 5 seconds |

### NFR-02: Scalability

| Metric | Requirement |
|--------|-------------|
| Maximum vectors in index | 1 million |
| Years of data supported | 10+ years |
| Concurrent search queries | 10 |

### NFR-03: Reliability

| Requirement | Description |
|-------------|-------------|
| Index corruption recovery | Rebuild from CF_EMBEDDINGS |
| Daemon crash recovery | Resume from last checkpoint |
| Model upgrade path | Re-embed with new model, keep old until verified |
| Graceful degradation | Falls back to BM25 if vector index unavailable |

### NFR-04: Security

| Requirement | Description |
|-------------|-------------|
| Local inference | No data sent to external APIs |
| Model verification | Checksum verification on download |
| No credential storage | Model weights are public |

### NFR-05: Observability

| Metric | Description |
|--------|-------------|
| `vector_index_size_vectors` | Gauge: vectors in index |
| `vector_search_latency_seconds` | Histogram: search duration |
| `embedding_latency_seconds` | Histogram: embedding duration |
| `index_lifecycle_pruned_total` | Counter: vectors pruned |
| `hybrid_search_latency_seconds` | Histogram: hybrid search duration |

---

## 12. System Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                       memory-daemon                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ memory-      │  │ memory-      │  │ memory-service       │  │
│  │ embeddings   │  │ vector       │  │ (gRPC handlers)      │  │
│  │              │  │              │  │                      │  │
│  │ ┌──────────┐ │  │ ┌──────────┐ │  │ ┌──────────────────┐ │  │
│  │ │ Candle   │ │  │ │ usearch  │ │  │ │ VectorTeleport   │ │  │
│  │ │ MiniLM   │ │  │ │ HNSW     │ │  │ │ HybridSearch     │ │  │
│  │ └──────────┘ │  │ └──────────┘ │  │ │ RebuildVector    │ │  │
│  └──────┬───────┘  └──────┬───────┘  │ │ GetVectorStatus  │ │  │
│         │                 │          │ └──────────────────┘ │  │
│         │                 │          └───────────┬──────────┘  │
│         │                 │                      │             │
│         └─────────┬───────┴──────────────────────┘             │
│                   │                                             │
│  ┌────────────────▼─────────────────────────────────────────┐  │
│  │                    RocksDB Storage                        │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌───────────────┐  │  │
│  │  │CF_EVENTS│ │CF_TOC   │ │CF_GRIPS │ │CF_EMBEDDINGS  │  │  │
│  │  │         │ │_NODES   │ │         │ │(NEW)          │  │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └───────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    HNSW Index File                        │  │
│  │              ~/.local/share/agent-memory/                 │  │
│  │                    vector-index/                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow: Vector Search

```
┌──────────┐     ┌──────────────┐     ┌─────────────────┐
│  Client  │────▶│ VectorTeleport│────▶│ EmbeddingModel  │
│ (Agent)  │     │     RPC       │     │   (Candle)      │
└──────────┘     └──────┬───────┘     └────────┬────────┘
                        │                      │
                        │    Query Embedding   │
                        │◀─────────────────────┘
                        │
                        ▼
               ┌─────────────────┐
               │   VectorIndex   │
               │   (HNSW)        │
               └────────┬────────┘
                        │
                        │  Top-K node_ids
                        ▼
               ┌─────────────────┐
               │    Storage      │
               │  (get titles)   │
               └────────┬────────┘
                        │
                        │  Enriched results
                        ▼
               ┌─────────────────┐
               │   Response      │
               └─────────────────┘
```

### Data Flow: Hybrid Search

```
┌──────────┐     ┌──────────────┐
│  Client  │────▶│ HybridSearch │
│ (Agent)  │     │     RPC      │
└──────────┘     └──────┬───────┘
                        │
         ┌──────────────┴──────────────┐
         │                             │
         ▼                             ▼
┌─────────────────┐           ┌─────────────────┐
│   BM25 Search   │           │  Vector Search  │
│   (Phase 11)    │           │   (Phase 12)    │
└────────┬────────┘           └────────┬────────┘
         │                             │
         │   Ranked results            │   Ranked results
         │                             │
         └──────────────┬──────────────┘
                        │
                        ▼
               ┌─────────────────┐
               │ Reciprocal Rank │
               │    Fusion       │
               └────────┬────────┘
                        │
                        │  Combined ranking
                        ▼
               ┌─────────────────┐
               │   Response      │
               └─────────────────┘
```

---

## 13. Integration with Existing Phases

### Phase Dependency Chain

```
Phase 10.5: Agentic TOC Search (FOUNDATION)
      │
      │  "Always works" index-free search
      │
      ▼
Phase 11: BM25 Teleport (KEYWORD ACCELERATION)
      │
      │  Tantivy full-text search
      │
      ▼
Phase 12: Vector Teleport (THIS PRD - SEMANTIC ACCELERATION)
      │
      │  HNSW semantic similarity
      │
      ▼
Phase 13: Outbox Index Ingestion (LIFECYCLE)
      │
      │  Incremental updates, lifecycle management
      │
      ▼
   Complete Search Stack
```

### Integration Points

| Phase | Component | Integration |
|-------|-----------|-------------|
| 10.5 | SearchNode/SearchChildren | Fallback when indexes unavailable |
| 11 | TeleportSearch RPC | HybridSearch calls BM25 search |
| 11 | Tantivy index | Same content indexed (TOC text, grips) |
| 13 | Outbox consumer | Drives embedding generation for new content |
| 13 | Checkpoint tracking | Crash recovery for embedding pipeline |
| 10 | Scheduler | Lifecycle pruning job |

### Search Method Comparison and Fallback Chain

The complete search method comparison table and fallback chain are defined in the **Agent Retrieval Policy PRD** — the single source of truth for retrieval layer selection.

**See:**
- [Agent Retrieval Policy PRD](agent-retrieval-policy-prd.md) - Fallback chains, capability tiers, skill contracts
- [Cognitive Architecture Manifesto](../COGNITIVE_ARCHITECTURE.md) - Philosophy and layer stack
- [BM25 Teleport PRD](bm25-teleport-prd.md)
- [Topic Graph Memory PRD](topic-graph-memory-prd.md)

---

## 14. Configuration Schema

```toml
# ~/.config/agent-memory/config.toml

# =============================================================================
# TELEPORT CONFIGURATION (Unified BM25 + Vector)
# =============================================================================
# See BM25 PRD Section 2 for [teleport] and [teleport.bm25] settings

[teleport.vector]
# OPTIONAL FEATURE: Set to false to completely disable vector search
# When disabled:
#   - No embedding model loaded (saves ~200MB RAM)
#   - No HNSW index created (saves disk space)
#   - VectorTeleport returns UNAVAILABLE
#   - HybridSearch falls back to BM25-only
# Default: true
enabled = true

# Embedding model configuration
[teleport.vector.embedding]
# HuggingFace model identifier
model_name = "sentence-transformers/all-MiniLM-L6-v2"
# Embedding dimension (must match model)
dimension = 384
# Local cache for model weights
cache_dir = "~/.local/share/agent-memory/models"
# Maximum text length (tokens)
max_length = 512

# HNSW index configuration
[teleport.vector.hnsw]
# Max connections per node (higher = better recall, more memory)
m = 16
# Construction-time search depth
ef_construction = 200
# Query-time search depth (higher = better recall, slower)
ef_search = 50
# Index file location
path = "~/.local/share/agent-memory/vector-index"

# Index lifecycle configuration (optional)
[teleport.vector.lifecycle]
# Enable automatic pruning
enabled = true
# Keep segment embeddings in HNSW for N days
segment_retention_days = 30
# Keep grip embeddings in HNSW for N days
grip_retention_days = 30
# Keep day embeddings for N days
day_retention_days = 365
# Keep week embeddings for N days (5 years)
week_retention_days = 1825
# Keep month embeddings for N days (forever effectively)
month_retention_days = 36500

# Scheduled maintenance
[teleport.vector.maintenance]
# Cron expression for lifecycle pruning
prune_schedule = "0 3 * * *"  # 3 AM daily
# Batch size for pruning operations
prune_batch_size = 1000
# Enable index optimization after pruning
optimize_after_prune = true

# Hybrid search defaults
[teleport.vector.hybrid]
# Default BM25 weight
default_bm25_weight = 0.5
# Default vector weight
default_vector_weight = 0.5
```

**Unified Configuration Note:** Both BM25 and Vector teleport share the parent `[teleport]` config section with a master `enabled` toggle. See BM25 PRD for complete schema.

---

## 15. API Surface

### gRPC RPCs

| Method | Path | Purpose | Availability |
|--------|------|---------|--------------|
| VectorTeleport | `MemoryService/VectorTeleport` | Semantic similarity search | Phase 12 |
| HybridSearch | `MemoryService/HybridSearch` | Combined BM25 + vector | Phase 12 |
| RebuildVectorIndex | `MemoryService/RebuildVectorIndex` | Admin: rebuild index | Phase 12 |
| GetVectorIndexStatus | `MemoryService/GetVectorIndexStatus` | Health and metrics | Phase 12 |

### CLI Commands

| Command | Description |
|---------|-------------|
| `memory-daemon vector-search --query "text"` | Vector similarity search |
| `memory-daemon hybrid-search --query "text"` | Combined search |
| `memory-daemon rebuild-vector [--from-persisted\|--full]` | Rebuild index |
| `memory-daemon vector-status` | Show index health |

### Skill Integration

Update `skills/memory-query/SKILL.md` to include:

```markdown
## Vector Search Commands (When Enabled)

**Note:** Vector search is optional. Check `GetVectorIndexStatus` before use.

### /memory-search --semantic <topic>
Search using semantic similarity. Finds conceptually related conversations
even when keywords differ.
**Requires:** `vector_index.enabled: true`
**Fallback:** Uses BM25 keyword search if disabled

### /memory-search --hybrid <topic>
Combine keyword and semantic search for best results.
**Requires:** `vector_index.enabled: true` for full functionality
**Fallback:** Uses BM25-only if vector disabled

### Checking Availability
Before using semantic features, the skill MUST:
1. Call GetVectorIndexStatus RPC
2. If ready=false, use BM25 TeleportSearch instead
3. If both unavailable, use agentic SearchChildren
```

### Agent Skill Error Codes

Skills must handle these responses when vector search is disabled:

| RPC | Status Code | Message | Skill Action |
|-----|-------------|---------|--------------|
| VectorTeleport | UNAVAILABLE | "Vector index not enabled" | Use TeleportSearch (BM25) |
| HybridSearch | OK | (works, but vector_score=0) | Results are BM25-only |
| RebuildVectorIndex | FAILED_PRECONDITION | "Vector index not enabled" | Inform user feature is disabled |
| GetVectorIndexStatus | OK | `enabled: false` | Skip vector features |

---

## 16. Observability

### Prometheus Metrics

```
# Vector index size
vector_index_size_vectors{level="segment"} 5432
vector_index_size_vectors{level="day"} 365
vector_index_size_vectors{level="week"} 52
vector_index_size_vectors{level="month"} 12

# Search latency histogram
vector_search_latency_seconds_bucket{le="0.05"} 95
vector_search_latency_seconds_bucket{le="0.1"} 98
vector_search_latency_seconds_bucket{le="0.2"} 99

# Embedding latency histogram
embedding_latency_seconds_bucket{le="0.01"} 50
embedding_latency_seconds_bucket{le="0.05"} 95
embedding_latency_seconds_bucket{le="0.1"} 99

# Hybrid search latency
hybrid_search_latency_seconds_bucket{le="0.1"} 80
hybrid_search_latency_seconds_bucket{le="0.3"} 99

# Index lifecycle
index_lifecycle_pruned_total{level="segment"} 1234
index_lifecycle_pruned_total{level="grip"} 5678
```

### Tracing

All vector operations include tracing spans:

- `vector_teleport` - Full request span
  - `embed_query` - Query embedding generation
  - `hnsw_search` - HNSW index search
  - `enrich_results` - Load titles from storage

### Health Checks

`GetVectorIndexStatus` returns:
- `ready: bool` - Index loaded and ready
- `vector_count: int64` - Total vectors
- `size_bytes: int64` - Disk size
- `model_name: string` - Embedding model
- `last_rebuild_ms: int64` - Last rebuild timestamp

---

## 17. Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Embedding model too large | High memory, slow startup | Medium | Use quantized MiniLM (~30MB) |
| HNSW index corruption | Search unavailable | Low | Rebuild from CF_EMBEDDINGS |
| Embedding model drift | Old embeddings incompatible | Medium | Store model version, re-embed on upgrade |
| Cold start latency | First query slow | Medium | Background warmup on daemon start |
| Semantic mismatch | Wrong results surfaced | Medium | Hybrid search combines with BM25 |
| Index size explosion | Disk full | Low | Lifecycle pruning, monitoring |
| Candle/usearch dependency conflicts | Build failures | Low | Pin versions, feature flags |

---

## 18. Open Questions

| Question | Status | Resolution |
|----------|--------|------------|
| Optimal embedding model? | Decided | all-MiniLM-L6-v2 for balance of size/speed/quality |
| HNSW parameters (M, ef_construction)? | Decided | M=16, ef_construction=200 (configurable) |
| Lifecycle retention periods? | Decided | 30 days for segments/grips, longer for coarser levels |
| Re-embedding on model upgrade? | Decided | Full rebuild supported via admin command |
| GPU inference needed? | Decided | No, CPU sufficient for MiniLM |

---

## 19. Out of Scope

| Item | Reason | Alternative |
|------|--------|-------------|
| External vector DB (Pinecone, Weaviate) | Adds deployment complexity | Local HNSW is sufficient |
| Real-time per-event embedding | Too slow, not needed | Batch via scheduler |
| Delete primary data | Append-only architecture | Lifecycle only prunes index |
| Multi-tenant isolation | Single-user daemon | N/A |
| GPU inference | CPU is fast enough for MiniLM | Future enhancement |
| Cross-device sync | Out of scope for v1 | Future enhancement |
| Embedding fine-tuning | Generic model works well | Future enhancement |

---

## 20. Implementation Phases

### Phase 12 Waves

| Wave | Focus | Duration | Files |
|------|-------|----------|-------|
| 12-01 | Embedding model integration | 1 plan | memory-embeddings crate |
| 12-02 | HNSW index implementation | 1 plan | memory-vector crate |
| 12-03 | gRPC service handlers | 1 plan | memory-service |
| 12-04 | CLI commands and admin | 1 plan | memory-daemon |

### Phase 13 Integration

Phase 13 (Outbox Index Ingestion) builds on Phase 12:

1. Outbox consumer triggers embedding for new TOC nodes/grips
2. Checkpoint tracking ensures crash-safe embedding pipeline
3. Lifecycle scheduler job prunes old vectors from HNSW
4. Admin rebuild command for full re-embedding

---

## 21. Appendix

### A. Proto Message Definitions

```protobuf
// ============================================
// VectorTeleport RPC
// ============================================

message VectorTeleportRequest {
  // Query text to embed and search
  string query = 1;

  // Maximum results to return (default: 10)
  int32 limit = 2;

  // Optional level filter: "segment", "day", "week", "month", "year"
  optional string level = 3;

  // Whether to include grips in results (default: true)
  bool include_grips = 4;

  // Minimum similarity threshold (default: 0.5)
  float min_similarity = 5;
}

message VectorTeleportResult {
  string doc_id = 1;
  string doc_type = 2;  // "toc_node" or "grip"
  float similarity = 3;

  // For toc_node: the level
  optional string level = 4;

  // Title/excerpt preview
  string title = 5;
}

message VectorTeleportResponse {
  repeated VectorTeleportResult results = 1;
  int64 embedding_time_ms = 2;
  int64 search_time_ms = 3;
}

// ============================================
// HybridSearch RPC
// ============================================

message HybridSearchRequest {
  string query = 1;
  int32 limit = 2;

  // BM25 weight for RRF (default: 0.5)
  float bm25_weight = 3;

  // Vector weight for RRF (default: 0.5)
  float vector_weight = 4;
}

message HybridSearchResult {
  string doc_id = 1;
  string doc_type = 2;
  float combined_score = 3;
  float bm25_score = 4;
  float vector_score = 5;
  string title = 6;
}

message HybridSearchResponse {
  repeated HybridSearchResult results = 1;
  int64 total_time_ms = 2;
}

// ============================================
// GetVectorIndexStatus RPC
// ============================================

message GetVectorIndexStatusRequest {}

message VectorIndexStatus {
  bool enabled = 1;
  bool ready = 2;
  int64 vector_count = 3;
  int64 size_bytes = 4;
  string model_name = 5;
  int32 dimension = 6;
  int64 last_rebuild_ms = 7;
  string message = 8;
}

// ============================================
// RebuildVectorIndex RPC
// ============================================

message RebuildVectorIndexRequest {
  // "from_persisted" (fast) or "full" (re-embed all)
  string mode = 1;
}

message RebuildVectorIndexResponse {
  bool success = 1;
  int64 vectors_indexed = 2;
  int64 duration_ms = 3;
  string message = 4;
}
```

### B. Embedding Model Comparison

| Model | Dimension | Size | Latency | Quality |
|-------|-----------|------|---------|---------|
| all-MiniLM-L6-v2 | 384 | 30MB | 20ms | Good |
| all-mpnet-base-v2 | 768 | 420MB | 50ms | Better |
| bge-small-en-v1.5 | 384 | 45MB | 25ms | Best (small) |
| bge-base-en-v1.5 | 768 | 440MB | 60ms | Best |

Recommendation: Start with all-MiniLM-L6-v2 for balance of size/quality/speed.

### C. HNSW Parameter Tuning

| Parameter | Description | Default | Tuning Notes |
|-----------|-------------|---------|--------------|
| M | Max connections per node | 16 | Higher = better recall, more memory |
| ef_construction | Build-time search depth | 200 | Higher = better index quality, slower build |
| ef_search | Query-time search depth | 50 | Higher = better recall, slower search |

### D. Storage Estimates

| Data Point | Count | Embedding Size | Total |
|------------|-------|----------------|-------|
| 1 year segments | ~17,520 (30min each) | 1.5KB | 26MB |
| 1 year grips | ~50,000 | 1.5KB | 75MB |
| 1 year days | 365 | 1.5KB | 0.5MB |
| 1 year weeks | 52 | 1.5KB | 0.08MB |
| 1 year months | 12 | 1.5KB | 0.02MB |
| **Total** | | | **~102MB/year** |

With lifecycle pruning (30-day segment/grip retention):
- Segments in HNSW: ~1,440 (30 days)
- Grips in HNSW: ~4,000 (30 days)
- **Active index: ~10MB** (plus coarser levels)

### E. Agent Skill Implementation Patterns

#### Pattern 1: Check-Then-Search

```markdown
## Searching with Fallback

When searching for past conversations:

1. **Check vector availability:**
   - Call `GetVectorIndexStatus`
   - Note `enabled` and `ready` fields

2. **Choose search method:**
   | Status | Method | Notes |
   |--------|--------|-------|
   | ready=true | HybridSearch | Best results |
   | ready=false, enabled=true | Wait or use BM25 | Index building |
   | enabled=false | TeleportSearch (BM25) | User disabled vector |
   | All indexes down | SearchChildren | Agentic fallback |

3. **Never assume availability:**
   - Vector search is optional
   - Users may disable it at any time
   - Skills MUST work without it
```

#### Pattern 2: Graceful Degradation in SKILL.md

```markdown
## Search Capability Tiers

This skill supports three search tiers with automatic fallback:

### Tier 1: Semantic + Keyword (Best)
**When:** Vector index enabled and ready
**Commands:** `/memory-search --hybrid`, `/memory-search --semantic`
**Capability:** Finds conceptually related content even with different wording

### Tier 2: Keyword Only (Good)
**When:** Vector disabled, BM25 available
**Commands:** `/memory-search <topic>` (auto-detects)
**Capability:** Finds exact keywords and stems

### Tier 3: Agentic Navigation (Always Works)
**When:** No indexes available
**Commands:** `/memory-navigate`
**Capability:** Traverses TOC hierarchy with term matching

The skill automatically selects the best available tier.
```

#### Pattern 3: User Communication

```markdown
## When Vector Search is Disabled

If a user requests semantic search but it's disabled:

1. **Inform clearly:** "Semantic search is not enabled. Using keyword search."
2. **Suggest enabling:** "Enable with: `vector_index.enabled: true` in config"
3. **Show results:** Provide BM25 results, note they are keyword-based
4. **Don't fail silently:** Always tell user which method was used
```

---

*PRD Created: 2026-02-01*
*Last Updated: 2026-02-01*
*Author: Agent Memory Team*
*Status: Draft for Architecture Review*
