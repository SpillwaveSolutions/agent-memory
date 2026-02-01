# Architecture Decision Records

This document captures the key architectural decisions made during the design and implementation of the agent-memory system. Each ADR follows a standard format documenting the context, decision, alternatives considered, and consequences.

---

## ADR-001: Append-Only Storage

**Status:** Accepted

**Date:** 2026-01-29 (Phase 1)

### Context

The agent-memory system needs to store conversation events from AI agents (Claude Code, OpenCode, Gemini CLI) for later retrieval and navigation. The fundamental question is whether to support updates and deletes on stored events, or to treat storage as an immutable log.

Key considerations:
- Conversation history should be reliable and auditable
- Agents need to trust that recalled information accurately reflects what was discussed
- Storage operations should be simple and predictable
- Crash recovery should be straightforward

### Decision

**All event storage is append-only. No updates, no deletes.**

Events are written once with a time-prefixed key (`evt:{timestamp_ms}:{ulid}`) and never modified. The only operations on the events column family are:
- PUT: Add new event
- GET: Retrieve by key
- Range SCAN: Query by time window

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| Mutable events with soft deletes | Adds complexity for tombstone tracking, compaction, and consistency guarantees |
| Event sourcing with snapshots | Snapshots add storage overhead and complexity when raw history is sufficient |
| Traditional CRUD model | Update semantics complicate crash recovery and audit trails |

### Consequences

**Positive:**
- Simpler storage layer with fewer code paths
- Natural audit trail - nothing can be silently altered
- Crash recovery is trivial - replay from last checkpoint
- Range scans are efficient with no tombstone filtering
- Events can be safely cached without invalidation concerns

**Negative:**
- Storage grows indefinitely (mitigated by RocksDB compaction and compression)
- Cannot "unsay" something - privacy-sensitive data requires full database wipe
- No in-place corrections; only new events can add context

**Validation:** Confirmed in v1.0.0 - append-only model proved reliable through all phases.

---

## ADR-002: TOC-Based Navigation

**Status:** Accepted

**Date:** 2026-01-29 (Phase 1, refined in Phase 2)

### Context

Agents need to answer queries like "what were we talking about last week?" without loading thousands of events into context. Brute-force search (scanning all events) does not scale and wastes agent context windows.

Key considerations:
- Agents have limited context windows (4K-200K tokens)
- Loading raw events for broad queries is prohibitively expensive
- Agents are capable of making navigation decisions based on summaries
- Time is a natural organizing principle for conversations

### Decision

**Use a hierarchical time-based Table of Contents (TOC) as the primary navigation mechanism.**

The TOC implements Progressive Disclosure Architecture:
1. Year nodes contain annual summaries
2. Month nodes contain monthly summaries
3. Week nodes contain weekly summaries
4. Day nodes contain daily summaries
5. Segment nodes contain conversation chunk summaries with grips (provenance links)

Agents navigate top-down, drilling into time periods that match their query context. Indexes (BM25, vector) are "teleport accelerators" - optional shortcuts that skip TOC levels when available, but the TOC always works even if indexes are unavailable.

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| Flat keyword search | No hierarchical context; hard to answer "what did we do last month?" |
| Topic-based clustering | Topic extraction is complex and error-prone; time is universal |
| Embedding-only search | Requires vector DB dependency; less interpretable than summaries |
| Chunk all events equally | Loses natural conversation boundaries; no semantic grouping |

### Consequences

**Positive:**
- Agents never need to scan everything - O(log N) navigation instead of O(N)
- Summaries are human-readable and agent-navigable
- Time-based hierarchy matches natural human memory patterns
- Index failures don't break the system - TOC is always available
- Works offline without external services

**Negative:**
- Requires summarization step (LLM API calls) to build TOC
- Summaries may miss nuances present in raw events
- Rollup jobs add background processing complexity
- Storage overhead for summary hierarchy

**Validation:** Confirmed in v1.0.0 - TOC navigation successfully supports "what were we discussing?" queries.

---

## ADR-003: gRPC Only (No HTTP)

**Status:** Accepted

**Date:** 2026-01-29 (Phase 1)

### Context

The agent-memory daemon needs an API for hook handlers to send events and for query clients to retrieve data. The question is whether to provide HTTP REST endpoints, gRPC, or both.

Key considerations:
- All clients are local (same machine) - no web browser access needed
- Strong typing reduces integration bugs
- Protocol overhead matters for high-frequency event ingestion
- Code generation simplifies client development

### Decision

**Expose only gRPC via tonic. No HTTP/REST server.**

The daemon provides:
- `IngestEvent` RPC for event ingestion
- Query RPCs (`GetTocRoot`, `GetNode`, `BrowseToc`, `GetEvents`, `ExpandGrip`)
- Health check via tonic-health
- Reflection via tonic-reflection for debugging with grpcurl

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| HTTP REST only | Weaker typing, more verbose, no streaming support |
| HTTP + gRPC dual stack | Maintenance burden for two APIs with no benefit |
| gRPC-web for browser access | No browser use case; adds transcoding complexity |
| Unix domain sockets only | Less portable; gRPC over TCP works everywhere |

### Consequences

**Positive:**
- Single, strongly-typed API contract via protobuf
- Generated client code in Rust, Python, TypeScript reduces integration errors
- Efficient binary encoding reduces payload size
- Built-in streaming for future large result sets
- grpcurl works out of the box for debugging

**Negative:**
- Cannot use curl for quick testing (must use grpcurl)
- No web dashboard without adding a separate HTTP server
- Slightly higher learning curve for teams unfamiliar with gRPC

**Validation:** Confirmed in v1.0.0 - gRPC-only approach simplified development and testing.

---

## ADR-004: RocksDB for Storage

**Status:** Accepted

**Date:** 2026-01-29 (Phase 1)

### Context

The agent-memory system needs persistent storage that is:
- Embedded (no external database server)
- Fast for time-range scans (primary access pattern)
- Reliable with crash recovery
- Suitable for append-only workloads

### Decision

**Use RocksDB with column families for all persistent storage.**

Column family layout:
| Column Family | Purpose |
|---------------|---------|
| `events` | Raw conversation events |
| `toc_nodes` | Versioned TOC node summaries |
| `toc_latest` | Latest version pointers |
| `grips` | Excerpt provenance records |
| `outbox` | Pending background job work items |
| `checkpoints` | Background job progress tracking |

Configuration:
- Universal compaction for append-only workloads
- ZSTD compression for events
- FIFO compaction for outbox (queue-like behavior)

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| SQLite | B-tree based; less optimal for append-only LSM workloads |
| sled | Alpha quality in 2026; unstable on-disk format |
| redb | B-tree based; newer with smaller ecosystem |
| PostgreSQL | Server dependency; overkill for single-user local storage |
| File-based storage | No range scan optimization; manual index management |

### Consequences

**Positive:**
- Battle-tested at massive scale (Facebook, LinkedIn, Netflix)
- LSM-tree architecture optimized for write-heavy workloads
- Column families allow per-data-type tuning
- Excellent compression ratios with ZSTD
- Atomic batch writes for consistency
- Built-in crash recovery via WAL

**Negative:**
- Write amplification during compaction (mitigated by Universal compaction)
- Memory consumption during compaction requires careful budgeting
- RocksDB C++ dependency complicates cross-compilation
- Column family count affects open file handles

**Validation:** Confirmed in v1.0.0 - RocksDB handles event ingestion and range scans efficiently.

---

## ADR-005: Grips for Provenance

**Status:** Accepted

**Date:** 2026-01-30 (Phase 3)

### Context

When agents navigate TOC summaries, they see abstracted information (titles, bullet points). For verification and drilling down, agents need a way to trace summary claims back to source events.

Key considerations:
- Summaries compress information; original nuance may be lost
- Agents should be able to verify claims before acting on them
- Source evidence increases trust in recalled information
- The link between summary and source must be queryable

### Decision

**Introduce "grips" as the provenance mechanism linking summaries to source events.**

A grip is:
```
Grip {
    grip_id: String,        // "grip:{timestamp_ms}:{ulid}"
    excerpt: String,        // Key phrase from source events
    event_id_start: String, // First event in evidence range
    event_id_end: String,   // Last event in evidence range
    timestamp: DateTime,    // When the excerpt was captured
    toc_node_id: String,    // Parent TOC node
}
```

TOC node bullets link to grips via `grip_ids: Vec<String>`. The `ExpandGrip` RPC returns context events around the excerpt.

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| No provenance (summaries only) | Agents cannot verify claims; reduces trust |
| Store full events in TOC nodes | Bloats TOC; defeats summarization purpose |
| Link to single event IDs | Many claims span multiple events; single ID insufficient |
| Store byte offsets | Fragile if event format changes |

### Consequences

**Positive:**
- Agents can drill down from any bullet to its source evidence
- Verifiable claims increase trust in memory recall
- Grips are lightweight (excerpt + pointers, not full content)
- Grip storage is separate from TOC, enabling independent cleanup

**Negative:**
- Additional storage column family
- Summarizer complexity increases (must extract grips)
- Grip-bullet matching uses heuristics (term overlap) which may miss nuances
- Orphaned grips possible if TOC nodes are rebuilt

**Validation:** Confirmed in v1.0.0 - ExpandGrip successfully returns context around summaries.

---

## ADR-006: In-Process Scheduler

**Status:** Accepted

**Date:** 2026-01-31 (Phase 10)

### Context

The agent-memory daemon needs to run periodic background jobs:
- Day/Week/Month TOC rollups
- RocksDB compaction
- Future: index maintenance

The question is whether to use external scheduling (cron, systemd timers) or embed scheduling in the daemon.

### Decision

**Use tokio-cron-scheduler for in-process async job scheduling.**

Key features:
- Cron expression parsing via croner (DST-aware)
- Timezone support via chrono-tz
- Graceful shutdown with CancellationToken
- Custom JobRegistry for observability (last run, next run, status)
- Overlap policy (Skip) prevents job pileup

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| External cron | Requires system configuration; less portable |
| systemd timers | Linux-only; doesn't work on macOS/Windows |
| Separate scheduler process | Additional deployment complexity |
| clokwerk | Synchronous; not async-native |
| Manual tokio::spawn + sleep | Error-prone; no cron syntax |

### Consequences

**Positive:**
- Self-contained daemon - no external scheduler configuration
- Portable across macOS, Linux, Windows
- Timezone-aware with DST handling
- Observable via gRPC GetSchedulerStatus RPC
- Graceful shutdown finishes in-progress jobs

**Negative:**
- Jobs run only when daemon is running
- No distributed scheduling (single instance)
- Scheduler state is in-memory (relies on checkpoints for job recovery)
- Queue overlap policy not implemented (Skip only)

**Validation:** Confirmed in v1.0.0 - rollup jobs run reliably on schedule.

---

## ADR-007: Tantivy for BM25 Search

**Status:** Accepted (Phase 11 - Planned)

**Date:** 2026-01-31

### Context

TOC navigation is the primary search mechanism, but agents sometimes want to "teleport" directly to relevant content using keywords. A full-text search index enables keyword-based jumps that bypass hierarchical navigation.

Key considerations:
- Must be embedded (no external search server)
- BM25 is the standard relevance scoring algorithm
- Index must be rebuildable from source of truth (RocksDB)
- Search should return TOC node IDs or grip pointers, not raw content

### Decision

**Use Tantivy embedded search engine for BM25-based teleport functionality.**

Index structure:
- Separate index directory (e.g., `~/.local/share/agent-memory/bm25-index/`)
- Document types: `toc_node` (title + bullets + keywords) and `grip` (excerpts)
- Fields: `doc_type`, `doc_id`, `text`, `keywords`, `timestamp_ms`
- Background commit job (every minute) via Phase 10 scheduler

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| SQLite FTS5 | Less flexible schema; weaker BM25 implementation |
| MeiliSearch | Full server process; not embeddable |
| Elasticsearch | External service; massive overkill for local use |
| Hand-rolled inverted index | Complex to implement correctly; Tantivy is battle-tested |

### Consequences

**Positive:**
- Embedded Lucene-quality search in pure Rust
- BM25 scoring out of the box
- Segment-based architecture supports concurrent reads
- Index is disposable - can rebuild from RocksDB
- Memory-mapped files for low memory footprint

**Negative:**
- Additional disk storage for index
- Commit operations are blocking (require spawn_blocking)
- Index can become stale if commit job fails
- Tantivy dependency adds compile time

**Note:** This decision is planned for Phase 11 implementation.

---

## ADR-008: Per-Project Stores

**Status:** Accepted

**Date:** 2026-01-29 (Phase 1)

### Context

Users work on multiple projects and may use agent-memory across all of them. The question is whether to:
1. Store all events in a single database with project tags
2. Create separate databases per project
3. Allow both modes

### Decision

**Default to per-project RocksDB instances, with configurable unified mode.**

Configuration:
- Default: `~/.local/share/agent-memory/{project_id}/` per project
- Alternative: Unified store with `project_id` field in events

Project identification:
- Derived from git repository root when available
- Falls back to current working directory hash
- Explicit override via MEMORY_PROJECT_ID environment variable

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|
| Unified store only | Projects pollute each other's navigation; harder mental model |
| Per-project only | Some users want cross-project search; no flexibility |
| Cloud sync across projects | Out of scope for local-first design |

### Consequences

**Positive:**
- Clean isolation between projects by default
- Simpler TOC navigation within a project
- Easy to delete a project's history (delete directory)
- Smaller database files for faster operations
- Natural sharding for potential future parallelization

**Negative:**
- Cross-project search requires explicit multi-store query
- Disk usage multiplied if same conversation spans projects
- Configuration required to specify project in ambiguous contexts

**Validation:** Confirmed in v1.0.0 - per-project stores provide clean isolation.

---

## Summary Table

| ADR | Decision | Status | Phase |
|-----|----------|--------|-------|
| ADR-001 | Append-Only Storage | Accepted | 1 |
| ADR-002 | TOC-Based Navigation | Accepted | 1, 2 |
| ADR-003 | gRPC Only (No HTTP) | Accepted | 1 |
| ADR-004 | RocksDB for Storage | Accepted | 1 |
| ADR-005 | Grips for Provenance | Accepted | 3 |
| ADR-006 | In-Process Scheduler | Accepted | 10 |
| ADR-007 | Tantivy for BM25 Search | Accepted (Planned) | 11 |
| ADR-008 | Per-Project Stores | Accepted | 1 |

---

## ADR Template

For future decisions, use this template:

```markdown
## ADR-XXX: [Title]

**Status:** Proposed | Accepted | Deprecated | Superseded

**Date:** YYYY-MM-DD

### Context

[What is the issue that we're seeing that is motivating this decision?]

### Decision

[What is the change that we're proposing and/or doing?]

### Alternatives Considered

| Alternative | Why Not Chosen |
|-------------|----------------|

### Consequences

**Positive:**
- [Benefit 1]
- [Benefit 2]

**Negative:**
- [Drawback 1]
- [Drawback 2]
```

---

*Document created: 2026-01-31*
*Last updated: 2026-01-31*
