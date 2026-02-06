# BM25 Teleport - Product Requirements Document

**Version:** 1.0
**Date:** 2026-02-01
**Phase:** 11

---

## 1. Executive Summary

### What is BM25 Teleport?

BM25 Teleport is a **keyword-based search accelerator** that enables agents to "teleport" directly to relevant TOC nodes or grips without traversing the entire time hierarchy. Unlike traditional search systems where indexing is the primary navigation mechanism, BM25 Teleport is a **disposable accelerator** - the TOC hierarchy remains the source of truth, and the index can be rebuilt at any time from storage.

### Core Philosophy

> "Indexes are accelerators, not dependencies."

This aligns with the agent-memory Progressive Disclosure Architecture (PDA):
- **Primary navigation**: TOC hierarchy (Year → Month → Week → Day → Segment)
- **Acceleration layer**: BM25 index for keyword-based teleportation
- **Fallback**: If index is unavailable, TOC navigation still works (via Phase 10.5)

### Why BM25?

BM25 (Best Matching 25) provides:
1. **Lexical grounding** - Finds exact keyword matches when agents know what terms to search
2. **Fast retrieval** - Sub-100ms queries against thousands of documents
3. **Complementary to semantic search** - Works alongside future vector search (Phase 12)
4. **Transparent scoring** - Agents understand why results ranked as they did

---

## 2. Optional and Configurable

### Core Principle: Indexes are Optional

BM25 Teleport is **entirely optional**. Users can disable it without losing any functionality - the system falls back to TOC-based navigation (Phase 10.5) which works without any index dependencies.

This is critical because:
1. **Resource conservation** - Some users may not want index overhead
2. **Simplicity preference** - TOC navigation may be sufficient for smaller memory stores
3. **Privacy concerns** - Some users may prefer not to maintain searchable indexes
4. **Rebuild scenarios** - Index can be deleted and system still works

### Configuration

```toml
# ~/.config/agent-memory/config.toml

[teleport]
# Master switch for all teleport indexes
enabled = true           # default: true

[teleport.bm25]
# BM25 keyword search (Phase 11)
enabled = true           # default: true
index_path = "~/.local/share/agent-memory/bm25-index/"
memory_budget_mb = 50    # IndexWriter memory budget
commit_interval_secs = 60

[teleport.vector]
# Vector semantic search (Phase 12, future)
enabled = false          # default: false (not yet implemented)
```

### Agent Skill Behavior

Agent skills MUST handle teleport being disabled gracefully:

#### Checking Teleport Availability

```
# Agent should first check if teleport is available
1. Call GetTeleportStatus() RPC
2. Response includes:
   - bm25_enabled: bool
   - bm25_healthy: bool (index exists and readable)
   - vector_enabled: bool
   - vector_healthy: bool
```

#### When Teleport is DISABLED

Agent skills should:
1. **Not offer teleport commands** - Don't suggest `/memory-search` if BM25 is disabled
2. **Use TOC navigation** - Fall back to SearchChildren RPC (Phase 10.5)
3. **Inform user if asked** - "Teleport search is disabled. Using TOC navigation instead."

Example agent flow when disabled:
```
User: "Find discussions about JWT tokens"

Agent (teleport disabled):
1. GetTeleportStatus() → bm25_enabled: false
2. Fall back to TOC search:
   - SearchChildren(query="JWT tokens", level=Month)
   - Drill into matching months
   - SearchChildren(query="JWT tokens", level=Week)
   - Continue progressive disclosure
3. Return results via TOC path
```

#### When Teleport is ENABLED

Agent skills should:
1. **Prefer teleport for keyword queries** - Faster, more direct
2. **Fall back to TOC if teleport fails** - Index may be rebuilding
3. **Combine approaches** - Use teleport to find starting points, then TOC to explore

Example agent flow when enabled:
```
User: "Find discussions about JWT tokens"

Agent (teleport enabled):
1. GetTeleportStatus() → bm25_enabled: true, bm25_healthy: true
2. TeleportSearch(query="JWT tokens", limit=5)
   → [(grip:abc, 0.92), (node:segment-xyz, 0.85)]
3. Return top results with evidence
4. Offer: "Expand grip:abc for full context?"
```

### Skill Documentation Requirements

Agent skills (SKILL.md files) MUST document:

1. **Teleport dependency** - Whether the command uses teleport
2. **Fallback behavior** - What happens when teleport is disabled
3. **Configuration guidance** - How users can enable/disable

Example skill documentation:
```markdown
## /memory-search

Searches past conversations for keywords.

**Teleport:** Uses BM25 teleport when enabled (faster)
**Fallback:** Uses TOC navigation when teleport disabled (still works)

### Configuration

To disable teleport search:
```toml
[teleport.bm25]
enabled = false
```

When disabled, this command uses progressive TOC search instead.
```

### RPC: GetTeleportStatus

New RPC to check teleport configuration and health:

```protobuf
message GetTeleportStatusRequest {}

message TeleportStatus {
  bool bm25_enabled = 1;
  bool bm25_healthy = 2;
  int64 bm25_doc_count = 3;

  bool vector_enabled = 4;
  bool vector_healthy = 5;
  int64 vector_doc_count = 6;

  string message = 7;  // Human-readable status
}

rpc GetTeleportStatus(GetTeleportStatusRequest) returns (TeleportStatus);
```

---

## 3. Architecture Alignment

### How BM25 Fits the Existing Design

```
┌─────────────────────────────────────────────────────────────┐
│                      Agent Memory                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │  TOC Tree    │    │   RocksDB    │    │   Tantivy    │  │
│  │ (Navigation) │    │  (Storage)   │    │   (Index)    │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │           │
│         │     Source of     │    Accelerator    │           │
│         │       Truth       │    (Disposable)   │           │
│         │                   │                   │           │
│  ┌──────▼───────────────────▼───────────────────▼──────┐   │
│  │                   gRPC Service                       │   │
│  │  GetTocRoot | GetNode | TeleportSearch | ExpandGrip │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Key Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| Single Tantivy index | Simpler than per-level indexes; use `doc_type` field to filter |
| Index TOC nodes + Grips | Not raw events (token explosion, redundant with summaries) |
| Primary data append-only | RocksDB remains immutable source of truth |
| Index lifecycle | Prune fine-grain docs over time; keep coarse rollups resident |
| MmapDirectory | Persistent, memory-efficient, crash-safe |

---

## 4. Terminology Mapping

The following table maps conceptual PRD terms to actual agent-memory implementation:

| Conceptual Term | Agent-Memory Implementation | Notes |
|-----------------|----------------------------|-------|
| Hot layer | Segment (30 min / 4K tokens) | Most recent, finest granularity |
| Warm layer | Day / Week nodes | Medium granularity rollups |
| Cold layer | Month / Year nodes | Coarse rollups |
| Archive layer | Year nodes | Oldest, most compressed |
| Lexical compaction | LLM rollup summarization | Summaries compress detail; lifecycle pruning limits fine-grain docs |
| Document | `TocNode` or `Grip` | Two document types in single index |
| TTL / Eviction | Not for primary data; index uses level-based retention |
| Layer indexes | Single index with `doc_type` field | Query filters by type if needed |

---

## 5. What Gets Indexed

### Document Types

The BM25 index contains two document types, distinguished by a `doc_type` field:

#### TOC Nodes (`doc_type: "toc_node"`)

| Field | Source | Indexed | Stored |
|-------|--------|---------|--------|
| doc_type | Literal "toc_node" | STRING | Yes |
| doc_id | `node.node_id` | STRING | Yes |
| level | `node.level` (year/month/week/day/segment) | STRING | Yes |
| text | `title + " " + bullets.map(b => b.text).join(" ")` | TEXT | No |
| keywords | `node.keywords.join(" ")` | TEXT | Yes |
| timestamp_ms | `node.start_time.timestamp_millis()` | STRING | Yes |

#### Grips (`doc_type: "grip"`)

| Field | Source | Indexed | Stored |
|-------|--------|---------|--------|
| doc_type | Literal "grip" | STRING | Yes |
| doc_id | `grip.grip_id` | STRING | Yes |
| text | `grip.excerpt` | TEXT | No |
| timestamp_ms | `grip.timestamp.timestamp_millis()` | STRING | Yes |

### Why Not Index Raw Events?

1. **Token explosion**: Raw events contain full conversation text; TOC summaries compress this
2. **Redundancy**: Summaries already capture key concepts from events
3. **Index bloat**: 100K events → 100K documents vs. ~5K TOC nodes + grips
4. **Grips provide provenance**: When detail is needed, grips link back to source events

---

## 6. Four-Level Agentic Search Model

Search is performed as a four-stage agentic process using Progressive Disclosure:

### Level 1: TOC Root (Orientation)

**Purpose:** Establish temporal scope
**RPC:** `GetTocRoot()`
**Returns:** Top-level time period nodes (Years)
**Agent action:** Identify which time periods are relevant to query

### Level 2: Hierarchy Navigation (Abstraction)

**Purpose:** Drill into relevant time periods using summaries
**RPCs:** `GetNode(node_id)`, `BrowseToc(parent_id)`
**Returns:** TOC nodes with title, bullets, keywords, child_ids
**Agent action:** Read summaries, decide which children to explore

### Level 3: BM25 Teleport (Grounding)

**Purpose:** Jump directly to relevant nodes or grips via keyword search
**RPC:** `TeleportSearch(query, doc_type?, limit)`
**Returns:** Ranked list of node_ids or grip_ids with BM25 scores
**Agent action:** Use teleport when keywords are known; bypass hierarchy traversal

### Level 4: Raw Evidence (Verification)

**Purpose:** Retrieve source evidence when summaries are insufficient
**RPCs:** `ExpandGrip(grip_id)`, `GetEvents(time_range)`
**Returns:** Context events around grip excerpt, or raw events
**Agent action:** Verify summary claims, cite specific evidence

### Navigation Flow

```
Query: "What did we discuss about JWT token expiration?"

Option A: Hierarchical (without teleport)
  1. GetTocRoot() → [Year 2026, Year 2025]
  2. GetNode("year:2026") → [Jan, Feb, ...]
  3. GetNode("month:2026-01") → [Week 1-4]
  4. GetNode("week:2026-W04") → [Days Mon-Fri]
  5. GetNode("day:2026-01-30") → [Segment abc]
  6. Read summary → Found JWT discussion

Option B: Teleport (with BM25)
  1. TeleportSearch("JWT token expiration")
     → [(grip:xyz, 0.92), (node:segment-abc, 0.85)]
  2. ExpandGrip("grip:xyz") → Full context
  Done in 2 calls vs 6
```

---

## 7. Bounded Growth via Summarization

### Index Lifecycle (Warm → Cold) and Summarization

The BM25 index keeps coarse rollups long-term and prunes fine-grain docs after they age out. Summaries still provide compression, but the index now has an explicit lifecycle:

| Level | Default retention in index | Why |
|-------|----------------------------|-----|
| Segment | 30 days | High churn; rolled up quickly |
| Day | 180 days | Mid-term recall while weekly/monthly rollups mature |
| Week | 5 years | Good balance of specificity vs. size |
| Month/Year | Keep | Stable, low-cardinality anchors |

Retention is enforced by a scheduled prune job (FR-09) and by skipping indexing of expired fine-grain docs once their rollups exist.

### Growth Model

The agent-memory system uses **summarization-based compression plus index lifecycle pruning**:

| Layer | Creation Trigger | Content |
|-------|-----------------|---------|
| Segment | 30 min gap OR 4K tokens | Direct event summarization |
| Day | End of day (scheduled) | Rollup of day's segments |
| Week | End of week (scheduled) | Rollup of week's days |
| Month | End of month (scheduled) | Rollup of month's weeks |
| Year | End of year (scheduled) | Rollup of year's months |

### Index Growth Projections

For a typical development workflow with ~300 events/day:

| Time Period | Raw Events | TOC Nodes | Grips | Index Size |
|-------------|------------|-----------|-------|------------|
| 1 month | ~9K | ~150 | ~300 | ~3 MB |
| 6 months | ~54K | ~900 | ~1.8K | ~15 MB |
| 1 year | ~108K | ~1.8K | ~3.6K | ~30 MB |
| 5 years | ~540K | ~9K | ~18K | ~150 MB |

**Key insight:** Index grows with TOC nodes, not raw events. Rollup summarization bounds growth logarithmically.

### Why Lifecycle Instead of Blind Eviction?

1. **Append-only truth**: RocksDB stays immutable; pruning only affects the accelerator layer
2. **Signal over noise**: Old fine-grain docs drop out once rolled up, keeping recall focused
3. **Predictable size**: Level-based retention bounds index growth
4. **Rebuildable**: Full rebuilds remain supported; lifecycle is additive safety, not a dependency

---

## 8. User Stories

### Agent User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-01 | Agent | search for conversations by keyword | I can find specific discussions quickly |
| US-02 | Agent | see BM25 scores with results | I can prioritize which paths to explore |
| US-03 | Agent | filter searches by document type | I can target TOC nodes or grips specifically |
| US-04 | Agent | check if teleport is available | I can choose the best search method |

### Admin User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-05 | Admin | rebuild the BM25 index | I can recover from corruption or refresh data |
| US-06 | Admin | monitor index health and size | I can proactively manage storage |
| US-07 | Admin | disable BM25 search | I can fall back to agentic if needed |
| US-08 | Admin | configure commit intervals | I can tune performance vs freshness |

### Developer User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-09 | Developer | test BM25 search via CLI | I can debug search behavior |
| US-10 | Developer | see search latency | I can identify performance issues |
| US-11 | Developer | force index rebuild | I can test rebuild logic |

---

## 9. Goals & Objectives

### Primary Goals

| Goal | Description | Success Metric |
|------|-------------|----------------|
| **G1: Fast Keyword Search** | Sub-100ms search for exact keywords | p99 < 100ms |
| **G2: Transparent Ranking** | Agents understand why results ranked | BM25 scores exposed |
| **G3: Rebuildable Index** | Index is disposable accelerator | Rebuild < 60s for 10K docs |
| **G4: Graceful Fallback** | Works without index via TOC navigation | System never fails |
| **G5: Bounded Growth** | Index grows sub-linearly | < 200 MB after 5 years |

### Non-Goals (Out of Scope)

| Non-Goal | Reason |
|----------|--------|
| Semantic search | That's Phase 12 (Vector) |
| Delete primary data | Append-only architecture |
| Eviction/TTL | Summarization compresses instead |
| Fuzzy matching | BM25 handles typos reasonably |
| Highlighting | Not needed for agent consumption |

---

## 10. Functional Requirements

### Teleport Index Requirements

Requirements defined in REQUIREMENTS.md, Phase 11 coverage:

| Requirement | Description | Phase 11 Plan |
|-------------|-------------|---------------|
| TELE-01 | BM25 teleport index via Tantivy (embedded) | 11-01, 11-02 |
| TELE-02 | Vector teleport index via HNSW (embedded) | Phase 12 |
| TELE-03 | Outbox relay consumes outbox entries, updates indexes | Phase 13 |
| TELE-04 | TeleportQuery RPC searches BM25 and/or vector, returns node_ids/grip_ids | 11-03 |
| TELE-05 | Index rebuild command from outbox or TOC | 11-04 |
| TELE-06 | IndexStatus RPC reports index health | 11-04 |

### Functional Requirements

#### FR-01: Tantivy Index Setup

**Acceptance Criteria:**
- [ ] Tantivy 0.25 integrated into workspace
- [ ] `memory-search` crate created with schema, indexer, searcher modules
- [ ] Schema includes doc_type, doc_id, level, text, keywords, timestamp_ms fields
- [ ] Index stored in configurable path (default: `~/.local/share/agent-memory/bm25-index/`)
- [ ] Index opens existing or creates new on startup

#### FR-02: Document Indexing

**Acceptance Criteria:**
- [ ] TOC nodes indexed with title + bullets + keywords as searchable text
- [ ] Grips indexed with excerpt as searchable text
- [ ] `doc_type` field enables filtering by document type
- [ ] Update replaces existing document (delete + add pattern)
- [ ] Batch indexing supported for initial population

#### FR-03: TeleportSearch RPC

**Acceptance Criteria:**
- [ ] gRPC RPC accepts query string, optional doc_type filter, limit
- [ ] Returns ranked results with doc_id, doc_type, score
- [ ] BM25 scoring provided by Tantivy QueryParser
- [ ] Search executes in < 100ms for typical queries
- [ ] Empty query returns error, not all documents

#### FR-04: Background Commit Job

**Acceptance Criteria:**
- [ ] Index writer commits periodically (configurable, default 60s)
- [ ] Commit job registered with Phase 10 scheduler
- [ ] Reader reloads on commit (ReloadPolicy::OnCommit)
- [ ] Graceful shutdown commits pending changes

#### FR-05: CLI Teleport Command

**Acceptance Criteria:**
- [ ] `memory-daemon teleport search <query>` performs BM25 search
- [ ] `--type` flag filters by toc_node or grip
- [ ] `--limit` flag caps results (default 10)
- [ ] Output shows doc_id, doc_type, score in readable format

#### FR-06: Index Rebuild

**Acceptance Criteria:**
- [ ] `memory-daemon admin rebuild-index` rebuilds from TOC nodes and grips
- [ ] Progress reported during rebuild
- [ ] Old index backed up or replaced atomically
- [ ] Rebuild completes in < 60s for 10K documents

#### FR-07: GetTeleportStatus RPC

**Acceptance Criteria:**
- [ ] gRPC RPC returns teleport configuration and health status
- [ ] Reports whether BM25 is enabled (from config)
- [ ] Reports whether BM25 is healthy (index exists and readable)
- [ ] Reports document count for each index type
- [ ] Returns human-readable status message
- [ ] Agent skills MUST call this before using teleport

#### FR-08: Teleport Configuration

**Acceptance Criteria:**
- [ ] Master `teleport.enabled` toggle in config.toml
- [ ] Per-index toggles: `teleport.bm25.enabled`, `teleport.vector.enabled`
- [ ] Configurable index path, memory budget, commit interval
- [ ] Config changes require daemon restart (no hot reload)
- [ ] Default: BM25 enabled, Vector disabled (not yet implemented)

#### FR-09: BM25 Lifecycle Pruning

**Acceptance Criteria:**
- [ ] Configurable per-level retention days for BM25 index (segment/day/week/month)
- [ ] Scheduler job runs prune on a cron (default 03:00 daily)
- [ ] Prune only removes BM25 docs; primary RocksDB data untouched
- [ ] Post-prune optimize/compact keeps index healthy
- [ ] TeleportStatus reports last prune time and pruned doc counts
- [ ] CLI/admin command `memory-daemon admin prune-bm25 --age-days <n> --level <segment|day|week|all>`

---

## 11. Non-Functional Requirements

### NFR-01: Performance

| Metric | Target |
|--------|--------|
| Search latency (p50) | < 20ms |
| Search latency (p99) | < 100ms |
| Index commit latency | < 500ms |
| Rebuild throughput | > 1000 docs/sec |

### NFR-02: Reliability

- Index corruption detected and reported via IndexStatus RPC
- Rebuild command recovers from any corruption
- Missing index starts empty, does not block daemon startup
- Commit failures logged but don't crash daemon

### NFR-03: Scalability

- Handles 100K+ documents efficiently
- Memory footprint < 100 MB for typical workload
- Index size grows sub-linearly with TOC growth

### NFR-04: Observability

- All index operations logged with tracing spans
- IndexStatus RPC reports: document count, index size, last commit time
- Metrics exposed for search latency, commit frequency, error rates

---

## 12. Success Metrics

### Adoption

| Metric | Target | Measurement |
|--------|--------|-------------|
| Teleport usage rate | > 30% of memory queries | gRPC metrics |
| Successful teleport rate | > 90% return results | gRPC success/failure |

### Quality

| Metric | Target | Measurement |
|--------|--------|-------------|
| Query latency (p99) | < 100ms | Tracing spans |
| Rare entity recall | > 95% | Manual testing |
| False positive rate | < 10% | Manual sampling |

### Efficiency

| Metric | Target | Measurement |
|--------|--------|-------------|
| Token savings vs hierarchy traversal | > 50% | Agent token tracking |
| Index growth rate | Sub-linear | Index size over time |

---

## 13. Observability

### Prometheus Metrics

```
# BM25 index size
bm25_index_doc_count{doc_type="toc_node"} 1850
bm25_index_doc_count{doc_type="grip"} 3600
bm25_index_size_bytes 31457280

# Search latency histogram
bm25_search_latency_seconds_bucket{le="0.02"} 95
bm25_search_latency_seconds_bucket{le="0.05"} 98
bm25_search_latency_seconds_bucket{le="0.1"} 99

# Commit metrics
bm25_commit_latency_seconds_bucket{le="0.5"} 99
bm25_commit_total 1234
bm25_commit_errors_total 0

# Index operations
bm25_index_operations_total{op="add"} 5450
bm25_index_operations_total{op="delete"} 0
bm25_rebuild_duration_seconds 45.2
```

### Tracing Spans

All BM25 operations include tracing spans:

- `teleport_search` - Full request span
  - `parse_query` - Query parsing
  - `execute_search` - Tantivy search execution
  - `collect_results` - Result collection and enrichment

### Health Checks

`GetTeleportStatus` returns:
- `bm25_enabled: bool` - Configuration state
- `bm25_healthy: bool` - Index accessible
- `bm25_doc_count: int64` - Total indexed documents
- `message: string` - Human-readable status

---

## 14. Integration with Phase 11 Plans

Phase 11 is divided into 4 implementation plans:

| Plan | Focus | Key Deliverables |
|------|-------|------------------|
| 11-01 | Tantivy Integration | memory-search crate, schema, index setup |
| 11-02 | Indexing Pipeline | IndexWriter, TOC/grip document mapping |
| 11-03 | Search API | TeleportSearch RPC, BM25 query handling |
| 11-04 | CLI & Jobs | teleport command, commit job, rebuild command |

### Dependencies

| Dependency | Reason | Status |
|------------|--------|--------|
| Phase 10 (Scheduler) | Periodic commit job | Complete |
| Phase 10.5 (TOC Search) | Fallback if index unavailable | Planned |
| Phase 2-3 (TOC, Grips) | Content to index | Complete |

### Future Phases

- **Phase 12 (Vector Teleport)**: Adds semantic similarity via HNSW embeddings
- **Phase 13 (Outbox Ingestion)**: Event-driven index updates for crash recovery

---

## 15. Out of Scope

| Item | Reason | Future Phase |
|------|--------|--------------|
| Vector/semantic search | Separate concern | Phase 12 |
| Outbox-driven indexing | Adds complexity | Phase 13 |
| Fuzzy matching | BM25 handles typos reasonably | Future |
| Query syntax (AND/OR/NOT) | Tantivy QueryParser handles this | Use default |
| Highlighting | Not needed for agent consumption | Future |
| Eviction/TTL | Append-only architecture | Never |

---

## 16. Open Questions

| Question | Status | Resolution |
|----------|--------|------------|
| Optimal memory budget for IndexWriter? | Decided | Start with 50MB, make configurable |
| Should search span all doc_types by default? | Decided | Yes, filter with optional param |
| Commit frequency tradeoff? | Decided | 60s default, configurable |

---

## 17. Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Tantivy index corruption | Search unavailable | Low | Rebuild from TOC nodes + grips |
| Cold start latency | First query slow | Medium | Background index loading on daemon start |
| Index size growth | Disk full | Low | Monitoring + optional compaction |
| Blocking async runtime | gRPC timeouts | Medium | Use `spawn_blocking` for Tantivy ops |
| Commit job failure | Stale search results | Low | Retry logic + alerting |
| Lockfile conflicts | Index unavailable | Low | Cleanup on startup |

---

## 18. Implementation Waves

| Wave | Plan | Focus | Key Deliverables |
|------|------|-------|------------------|
| 1 | 11-01 | Tantivy Integration | memory-search crate, schema, MmapDirectory setup |
| 2 | 11-02 | Indexing Pipeline | IndexWriter, TOC/grip document mapping, batch indexing |
| 3 | 11-03 | Search API | TeleportSearch RPC, GetTeleportStatus RPC, BM25 scoring |
| 3 | 11-04 | CLI & Jobs | teleport command, commit job, rebuild command, status |

---

## Appendix A: Tantivy Schema Reference

```rust
// From 11-RESEARCH.md
pub fn build_teleport_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Document type: "toc_node" or "grip"
    schema_builder.add_text_field("doc_type", STRING | STORED);

    // Primary key - node_id or grip_id
    schema_builder.add_text_field("doc_id", STRING | STORED);

    // TOC level (for toc_node only)
    schema_builder.add_text_field("level", STRING);

    // Searchable text content
    schema_builder.add_text_field("text", TEXT);

    // Keywords (indexed and stored)
    schema_builder.add_text_field("keywords", TEXT | STORED);

    // Timestamp for potential recency boosting
    schema_builder.add_text_field("timestamp_ms", STRING | STORED);

    schema_builder.build()
}
```

---

## Appendix B: Storage Estimates

| Data Point | Count (1 year) | Index Size | Notes |
|------------|----------------|------------|-------|
| TOC Segments | ~17,520 (30min each) | ~15 MB | title + bullets + keywords |
| TOC Days | 365 | ~2 MB | daily summaries |
| TOC Weeks | 52 | ~0.5 MB | weekly summaries |
| TOC Months | 12 | ~0.1 MB | monthly summaries |
| Grips | ~50,000 | ~10 MB | excerpt text only |
| **Total** | | **~30 MB/year** | |

After 5 years: ~150 MB (highly compressible, Tantivy handles this efficiently)

---

## Appendix C: Proto Definitions

```protobuf
// ============================================
// TeleportSearch RPC
// ============================================

message TeleportSearchRequest {
  // Query text for BM25 search
  string query = 1;

  // Optional filter: "toc_node" or "grip"
  optional string doc_type = 2;

  // Maximum results to return (default: 10)
  int32 limit = 3;

  // Optional TOC level filter: "segment", "day", "week", "month", "year"
  optional string level = 4;
}

message TeleportSearchResult {
  string doc_id = 1;
  string doc_type = 2;
  float score = 3;

  // For toc_node: the level
  optional string level = 4;

  // For toc_node: keywords from the node
  repeated string keywords = 5;
}

message TeleportSearchResponse {
  repeated TeleportSearchResult results = 1;
  int64 search_time_ms = 2;
  int32 total_hits = 3;
}

// ============================================
// GetTeleportStatus RPC
// ============================================

message GetTeleportStatusRequest {}

message TeleportStatus {
  bool bm25_enabled = 1;
  bool bm25_healthy = 2;
  int64 bm25_doc_count = 3;

  bool vector_enabled = 4;
  bool vector_healthy = 5;
  int64 vector_doc_count = 6;

  string message = 7;
}

// ============================================
// RebuildIndex RPC
// ============================================

message RebuildIndexRequest {
  // Index type: "bm25" or "vector"
  string index_type = 1;
}

message RebuildIndexResponse {
  bool success = 1;
  int64 documents_indexed = 2;
  int64 duration_ms = 3;
  string message = 4;
}
```

---

## Appendix D: Example Search Session

```
# Search for JWT-related content
$ memory-daemon teleport search "JWT token expiration"

Results (3 found):
  1. grip:abc123 (score: 0.92)
     Type: grip

  2. toc_node:segment-2026-01-30-001 (score: 0.85)
     Type: toc_node
     Level: segment
     Keywords: JWT, authentication, token

  3. toc_node:day-2026-01-30 (score: 0.71)
     Type: toc_node
     Level: day

# Get details on top result
$ memory-daemon query expand-grip abc123

Excerpt: "The JWT token expiration was set to 1 hour, but we
         discussed extending it to 24 hours for mobile clients."

Events: 3 (timestamps 14:32:01 - 14:35:22)
```

---

## Appendix E: Agent Skill Integration Guide

### Pattern 1: Check-Then-Search

Skills MUST check availability before using teleport:

```rust
// Example: Check before using BM25 teleport
async fn search_with_fallback(query: &str) -> Result<SearchResults> {
    // First, check if BM25 teleport is available
    let status = client.get_teleport_status().await;

    match status {
        Ok(status) if status.bm25_enabled && status.bm25_healthy => {
            // BM25 available - use teleport
            client.teleport_search(TeleportSearchRequest {
                query: query.to_string(),
                ..Default::default()
            }).await
        }
        _ => {
            // BM25 unavailable - fall back to agentic TOC search
            client.search_children(SearchChildrenRequest {
                query: query.to_string(),
                ..Default::default()
            }).await
        }
    }
}
```

### Pattern 2: Search Capability Tiers

Skills should document their fallback behavior:

```markdown
## Search Capability Tiers

This skill supports two search tiers with automatic fallback:

### Tier 1: BM25 Keyword Search (Preferred)
**When:** BM25 teleport enabled and healthy
**Method:** TeleportSearch RPC
**Capability:** Fast keyword matching with BM25 ranking

### Tier 2: Agentic TOC Search (Fallback)
**When:** BM25 disabled or unavailable
**Method:** SearchChildren RPC (Phase 10.5)
**Capability:** Index-free term matching, always works

The skill automatically selects the best available tier.
```

### Pattern 3: User Communication

Skills MUST inform users which method was used:

```markdown
## When BM25 is Disabled

If teleport search is disabled:

1. **Inform clearly:** "BM25 search is not enabled. Using TOC navigation."
2. **Suggest enabling:** "Enable with: `teleport.bm25.enabled: true` in config"
3. **Show results:** Provide agentic search results
4. **Don't fail silently:** Always tell user which method was used
```

### Combined Status Check (BM25 + Vector)

When Phase 12 is available, check both indexes:

```rust
async fn search_best_available(query: &str) -> Result<SearchResults> {
    let bm25_status = client.get_teleport_status().await.ok();
    let vector_status = client.get_vector_index_status().await.ok();

    let bm25_ready = bm25_status.map(|s| s.bm25_healthy).unwrap_or(false);
    let vector_ready = vector_status.map(|s| s.ready).unwrap_or(false);

    match (vector_ready, bm25_ready) {
        (true, true) => client.hybrid_search(query).await,   // Best: both
        (true, false) => client.vector_teleport(query).await, // Semantic only
        (false, true) => client.teleport_search(query).await, // BM25 only
        (false, false) => client.search_children(query).await, // Agentic fallback
    }
}
```

---

## Appendix F: Agent Skill Error Codes

Skills must handle these responses when BM25 is disabled:

| RPC | Status Code | Message | Skill Action |
|-----|-------------|---------|--------------|
| TeleportSearch | UNAVAILABLE | "BM25 index not enabled" | Use SearchChildren (agentic) |
| GetTeleportStatus | OK | `bm25_enabled: false` | Skip BM25 features |
| GetTeleportStatus | OK | `bm25_healthy: false` | Index exists but unhealthy, use agentic |

### Response Handling Example

```rust
match client.teleport_search(request).await {
    Ok(response) => {
        // Success - use results
        display_results(response.results);
    }
    Err(status) if status.code() == Code::Unavailable => {
        // BM25 disabled - inform user and fall back
        println!("BM25 search is disabled. Using TOC navigation.");
        let fallback = client.search_children(fallback_request).await?;
        display_results(fallback.results);
    }
    Err(status) => {
        // Other error - propagate
        return Err(status.into());
    }
}
```

---

## Appendix G: Search Method Comparison and Fallback Chain

The complete search method comparison table and fallback chain are defined in the **Agent Retrieval Policy PRD** — the single source of truth for retrieval layer selection.

**See:** [Agent Retrieval Policy PRD](agent-retrieval-policy-prd.md) for:
- Search method comparison (all phases)
- Intent-aware fallback chains
- Capability tier definitions
- Skill integration patterns

**See also:**
- [Cognitive Architecture Manifesto](../COGNITIVE_ARCHITECTURE.md)
- Vector PRD: `hierarchical-vector-indexing-prd.md`
- Topic Graph PRD: `topic-graph-memory-prd.md`

---

*PRD Created: 2026-02-01*
*Last Updated: 2026-02-01*
*Author: Agent Memory Team*
