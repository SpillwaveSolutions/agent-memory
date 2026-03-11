# Technology Stack: v2.6 Episodic Memory, Salience Scoring, Lifecycle Automation

**Project:** Agent Memory — Local agentic memory system with retrieval layers
**Researched:** 2026-03-11
**Confidence:** HIGH

## Executive Summary

The v2.6 milestone adds episodic memory (task outcome tracking), salience/usage-based ranking, lifecycle automation, and BM25 hybrid wiring to a mature 14-crate Rust system (v2.5 shipped with semantic dedup + stale filtering).

**No new external dependencies required.** The existing stack (Tantivy, Candle, usearch, RocksDB) handles all new features. The key changes are:
1. **Schema extensions** in proto to episodic messages + outcome fields
2. **New crates** for episodic storage (not new packages — use existing RocksDB)
3. **Configuration** for retention, salience, value thresholds
4. **Existing APIs** (vector pruning, BM25 lifecycle) wired into scheduler

## Recommended Stack

### No New External Dependencies

| Category | Tech | Version | Why | Status |
|----------|------|---------|-----|--------|
| **Episodic Storage** | RocksDB (existing) | 0.22 | Same append-only engine + new CF_EPISODES | Already in use |
| **Hybrid Search** | Tantivy (existing) + usearch (existing) | 0.25 / 2 | RRF fusion between BM25 and vector | Implemented in v2.2 |
| **Embeddings** | Candle (existing) + all-MiniLM-L6-v2 | 0.8 | Local inference, no API calls | Validated v2.0 |
| **Async Runtime** | Tokio + tonic | 1.43 / 0.12 | gRPC service, scheduler tasks | Core infrastructure |
| **Serialization** | serde + serde_json + prost | 1.0 / 1.0 / 0.13 | Config, JSON, proto messages | Standard |
| **Time** | chrono | 0.4 | Timestamps, decay calculations | Already in use |
| **Concurrency** | dashmap + Arc + std::sync::RwLock | 6 / — / — | ConcurrentHashMap for usage stats, RwLock for InFlightBuffer | Already in use |

### Already-Integrated Libraries (No Upgrades Needed)

| Library | Current Version | Purpose | Note |
|---------|-----------------|---------|------|
| usearch | 2 | HNSW vector index + dedup similarity | Used in cross-session dedup (v2.5) |
| hdbscan | 0.12 | Semantic clustering for topic graph | Topic discovery layer (v2.0) |
| lru | 0.12 | LRU cache for usage tracking | Access count caching in storage (v2.1) |
| ulid | 1.1 | Unique ID generation | Event IDs, Episode IDs |
| tokio-cron | (via tokio-util) | Background scheduler | Job scheduling for lifecycle jobs |
| thiserror | 2.0 | Error types | Standard error handling |
| tracing | 0.1 | Observability | Logging + metrics |

## Architecture Integration Points

### 1. Episodic Memory Storage (New Crate: memory-episodes)

**Location:** `crates/memory-episodes/`
**Dependencies:** memory-types, memory-storage, memory-embeddings, tokio, serde

**Integration:**
- New column family in RocksDB: `CF_EPISODES`
- Store Episode structs (episode_id → Episode JSON in RocksDB)
- Reuse existing embedding pipeline (Candle all-MiniLM-L6-v2)
- Store episode embeddings in same vector index as TOC nodes (with metadata tag "episode")

**No new dependencies:** RocksDB is the storage engine. Episode lifecycle management reuses the existing scheduler (memory-scheduler).

### 2. Salience + Usage Ranking (memory-retrieval enhancement)

**Current state:**
- Salience fields exist in proto (TocNode.salience_score, TocNode.memory_kind) and memory-types
- Usage tracking exists (UsageStats, UsageConfig in memory-types, dashmap cache in memory-storage)
- SalienceScorer exists in memory-types but not wired into retrieval

**Changes needed:**
- Wire SalienceScorer into all retrieval result ranking (BM25, vector, topics)
- Thread usage stats from storage through retrieval pipeline
- Apply formula: `score = base_similarity * (0.55 + 0.45 * salience) * usage_penalty(access_count)`

**No new dependencies:** Uses existing UsageConfig, SalienceScorer, and dashmaps in storage.

### 3. Lifecycle Automation (memory-scheduler + memory-search enhancements)

**Current state:**
- Tokio cron scheduler exists (memory-scheduler crate)
- Vector pruning API exists: `VectorIndexPipeline::prune(age_days)`
- BM25 lifecycle config exists: `Bm25LifecycleConfig`
- RocksDB operations are append-only; soft-delete via filtered rebuild

**Changes needed:**
- Add scheduler job for vector index pruning (daily 3 AM)
- Add scheduler job for BM25 index rebuild with level filter (weekly)
- Wire config from `[lifecycle]` section in config.toml

**Configuration additions (config.toml):**
```toml
[lifecycle]
enabled = true

[lifecycle.vector]
# Existing but needs automation
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 365
prune_schedule = "0 3 * * *"

[lifecycle.bm25]
segment_retention_days = 30
grip_retention_days = 30
rebuild_schedule = "0 4 * * 0"  # Weekly Sunday 4 AM

[lifecycle.episodes]
# New: Value-based retention for episodes
value_threshold = 0.18
max_episodes = 1000
prune_schedule = "0 2 * * *"
```

**No new dependencies:** Reuses Tokio cron, existing RocksDB, existing lifecycle APIs.

### 4. BM25 Hybrid Wiring (memory-search enhancement)

**Current state:**
- HybridSearch RPC exists in proto
- BM25 search (TeleportSearch) exists
- Vector search exists
- RRF fusion algorithm designed but not fully wired into routing

**Changes needed:**
- Wire BM25 results through hybrid search handler (not hardcoded `false`)
- Apply RRF normalization: `score = 60 / (60 + rank_bm25) + 60 / (60 + rank_vector)`
- Weight fusion by mode (HYBRID_MODE_HYBRID uses 0.5/0.5 by default)
- Ensure agent filtering applied to both tiers

**No new dependencies:** Uses existing Tantivy and usearch.

## Integration Path (No Blockers)

```
v2.5 (Shipped) → v2.6 (New)
├─ Existing Schema ✓
│  ├─ TocNode.salience_score (proto field 101)
│  ├─ TocNode.memory_kind (proto field 102)
│  └─ Grip.salience_score (proto field 11)
│
├─ New Schema (Proto additions, field numbers > 200)
│  ├─ Episode message (new column family CF_EPISODES)
│  ├─ StartEpisodeRequest/Response
│  ├─ RecordActionRequest/Response
│  ├─ CompleteEpisodeRequest/Response
│  └─ GetSimilarEpisodesRequest/Response
│
├─ Storage (RocksDB only)
│  ├─ CF_EPISODES (append-only episode journal)
│  └─ Existing usage stats cache (dashmap)
│
├─ Computation (Existing ML stack)
│  ├─ Episode embeddings (Candle all-MiniLM-L6-v2)
│  ├─ Similarity search (usearch HNSW)
│  └─ Salience scoring (existing formula)
│
├─ Lifecycle (Tokio scheduler only)
│  ├─ Vector prune job (existing API, new scheduler wiring)
│  ├─ BM25 rebuild job (existing API, new scheduler wiring)
│  └─ Episode prune job (new, reuses same job framework)
│
└─ Retrieval (memory-retrieval + handlers)
   ├─ Hybrid search wiring (existing RPC, new routing)
   ├─ Salience integration (existing scorer, new ranking layer)
   ├─ Usage decay application (existing stats, new formula)
   └─ Episode similarity search (new handler, existing embeddings)
```

## What NOT to Add

| Anti-Pattern | Reason | What to Do Instead |
|--------------|--------|-------------------|
| New async runtime | Tokio is standard for Rust systems | Keep tokio 1.43 |
| Separate vector DB (Weaviate, Qdrant, etc.) | Single-process system; RocksDB is correct | Store vectors in HNSW index + metadata |
| SQL database (SQLx, Tokio-postgres) | Append-only RocksDB is the model | Add new column families, not tables |
| New LLM API for embeddings | Local Candle ensures zero API dependency | Use all-MiniLM-L6-v2 exclusively |
| Feature flag framework (feature-gates) | Not needed; code is simple enough | Use config.toml bools for toggles |
| Streaming/real-time updates (tonic streaming for episodes) | Unidirectional request/response is correct | Keep gRPC request/response pattern |
| Consolidation/NLP extraction (spaCy, NLTK) | Out of scope for v2.6; episodic memory only | Defer to v2.7 if pursued |

## Verification Checklist

- [x] Episodic storage: RocksDB column family sufficient (no new DB)
- [x] Embeddings: Candle handles episodes same as TOC nodes
- [x] Hybrid search: Existing BM25/vector APIs, just needs routing wiring
- [x] Lifecycle jobs: Tokio scheduler covers vector/BM25/episode pruning
- [x] Salience: Proto fields and SalienceScorer already defined; integrate into ranking
- [x] Usage tracking: dashmap + LRU cache already in place
- [x] No runtime changes: Tokio 1.43 sufficient for all async operations
- [x] Proto safety: Field numbers > 200 reserved for phase 23+ (safe to add episodes)
- [x] Backward compatibility: All new fields optional in proto; serde(default) handles JSON parsing

## Confidence Assessment

| Component | Level | Notes |
|-----------|-------|-------|
| **RocksDB schema** | HIGH | CF_EPISODES is straightforward append-only; validated pattern |
| **Embeddings** | HIGH | all-MiniLM-L6-v2 + Candle proven in production (v2.0+) |
| **Vector search** | HIGH | usearch HNSW + dedup similarity search working (v2.5) |
| **Scheduler** | HIGH | Tokio cron job framework operational since v2.0 |
| **Hybrid fusion** | MEDIUM | RRF algorithm designed, existing handlers need wiring only |
| **Salience integration** | HIGH | SalienceScorer exists, needs threading through retrieval |
| **Configuration** | HIGH | config.toml pattern established; new sections are additive |
| **Episode retention** | MEDIUM | Value-based pruning algorithm is novel but low-complexity (threshold check) |

## Sources

- **Code:** `/Users/richardhightower/clients/spillwave/src/agent-memory/`
  - Workspace Cargo.toml (dependencies verified 2026-03-11)
  - proto/memory.proto (schema v2.5 shipped, v2.6 additions safe in field > 200)
  - crates/memory-types/src/ (SalienceScorer, UsageStats, UsageConfig, DedupConfig, StalenessConfig)
  - crates/memory-storage/src/ (dashmap 6.0, lru 0.12, RocksDB 0.22)
  - crates/memory-search/src/lifecycle.rs (Bm25LifecycleConfig, retention_map)
  - crates/memory-scheduler/ (Tokio cron job framework)
  - crates/memory-vector/ (VectorIndexPipeline::prune API)

- **Design:** `.planning/PROJECT.md` (v2.6 requirements, validated decisions)
- **RFC:** `docs/plans/memory-ranking-enhancements-rfc.md` (episodic memory Tier 2 spec, lifecycle Tier 1.5)

---
*Research completed 2026-03-11. No external dependencies added. All features implemented via existing crates + RocksDB column families + proto schema extensions.*
