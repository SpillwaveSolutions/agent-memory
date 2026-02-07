# Phase 16: Memory Ranking Enhancements

## Action Required

**The only immediate action is to store this plan:**

1. This plan is stored at: `docs/plans/phase-16-memory-ranking-plan.md`
2. The RFC is already stored at: `docs/plans/memory-ranking-enhancements-rfc.md`

No implementation work until the plan is approved.

---

## Overview

Incremental enhancements to agent-memory's retrieval and storage policies, extending the existing time-decay pattern from Topics to all memory types.

**RFC:** [docs/plans/memory-ranking-enhancements-rfc.md](docs/plans/memory-ranking-enhancements-rfc.md)

**Feature Branch:** `feature/phase-16-memory-ranking`

---

## Context

The current v2.0.0 stack provides excellent navigation and search:
- TOC hierarchy (always works backbone)
- BM25 + vector + topics (accelerated search)
- Time-decayed importance scoring (Topics only)

**Gap:** All non-topic memories are treated equally. No mechanism to surface frequently-accessed memories, prevent redundant storage, or differentiate important memories from observations.

**Goal:** Add retrieval policy improvements respecting append-only constraints.

---

## Architectural Constraints

### Append-Only Storage Model

TOC nodes and Grips are **immutable** (per TOC-06 and Phase 1 decisions):
- Nodes are versioned, not mutated
- Per-read mutation would spam new versions or break immutability

**Implication:** Usage counters CANNOT live on TocNode/Grip. Need separate storage.

### Embedding Stack Independence

Event ingestion currently does NOT depend on:
- Candle embedding model
- Vector index availability

**Implication:** Novelty check must be best-effort with explicit opt-in.

---

## Retention Matrix (Authoritative)

Per PRDs, the canonical retention rules are:

### Vector Index (FR-08)

| Level | Retention Days | Notes |
|-------|----------------|-------|
| Segment | 30 | High churn, rolled up quickly |
| Grip | 30 | Same as segment |
| Day | 365 | Mid-term recall |
| Week | 1825 | 5 years |
| Month | 36500 | Effectively forever |

### BM25 Index (FR-09)

| Level | Retention Days | Notes |
|-------|----------------|-------|
| Segment | 30 | High churn |
| Day | 180 | Mid-term recall while rollups mature |
| Week | 1825 | 5 years |
| Month/Year | Never pruned | Stable anchors |

**Protection Rule:** Month and Year nodes are NEVER pruned from either index.

---

## Scope: Tier 1 + Lifecycle Automation

| Feature | Complexity | Risk | Notes |
|---------|------------|------|-------|
| Salience Scoring | Low | Low | Write-time only |
| Usage Counters | Medium | Medium | Requires new CF |
| Novelty Threshold | Medium | Medium | Best-effort, not blocking |
| Vector Pruning Automation | Low | Low | Wire existing API |
| BM25 Lifecycle (FR-09) | Medium | Medium | Align with PRD |

**Estimated Effort:** ~2.5 weeks

---

## Implementation Plan

### Plan 16-01: Salience Scoring (Write-Time Only)

**Goal:** Score memories by importance at write time

**Design:**
- Salience is computed ONCE at node creation (not on read)
- Stored as immutable field on TocNode/Grip
- No mutation required - respects append-only model

**Changes:**
1. Add `memory-types/src/salience.rs`:
   ```rust
   pub enum MemoryKind {
       Observation,    // Default (no boost)
       Preference,     // "prefer", "like", "avoid"
       Procedure,      // "step", "first", "then"
       Constraint,     // "must", "should", "need to"
       Definition,     // "is defined as", "means"
   }

   pub struct SalienceScorer { config: SalienceConfig }

   impl SalienceScorer {
       /// Calculate salience at node creation time (immutable)
       pub fn calculate(&self, text: &str, kind: MemoryKind, is_pinned: bool) -> f32;
   }
   ```

2. Add fields to `TocNode` and `Grip` (schema migration):
   - `salience_score: f32` (default 0.5 for existing data)
   - `memory_kind: MemoryKind` (default Observation)
   - `is_pinned: bool` (default false)

3. Update TOC builder to calculate salience on node creation

4. Add configuration under existing teleport namespace:
   ```toml
   [teleport.ranking.salience]
   enabled = true
   length_density_weight = 0.45
   kind_boost = 0.20
   pinned_boost = 0.20
   ```

**Migration:**
- Existing nodes get default values (salience=0.5, kind=Observation)
- No backfill required - new nodes only
- Wire protocol: add fields with default values (backward compatible)

**Tests:**
- Unit tests for salience calculation
- Integration test: verify salience persisted with new nodes
- Backward compat test: existing nodes readable without salience fields

### Plan 16-02: Usage Counters (Separate CF)

**Goal:** Track access patterns WITHOUT mutating immutable nodes

**Design:**
- New column family `CF_USAGE_COUNTERS` stores usage separately
- Key: node_id or grip_id
- Value: `{ access_count: u32, last_accessed: DateTime<Utc> }`
- Batch writes to avoid write amplification
- **Cache-first reads** to avoid hot-path CF lookups

**Read Path Strategy (Critical for Performance):**

```
Search Request
     │
     ▼
┌────────────────┐
│ Get doc_ids    │ (from BM25/Vector/TOC)
└────────┬───────┘
         │
         ▼
┌────────────────────────────────────────────┐
│ UsageCache.get_batch(doc_ids)              │
│  - Check in-memory LRU cache first         │
│  - Return cached entries immediately       │
│  - Log cache hit rate metric               │
└────────┬───────────────────────────────────┘
         │ cache miss for some IDs?
         │
         ▼ (async, non-blocking)
┌────────────────────────────────────────────┐
│ Spawn background task to prefetch:         │
│  - Load missing IDs from CF_USAGE_COUNTERS │
│  - Populate cache for future requests      │
│  - Does NOT block current search           │
└────────────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────┐
│ Rank results with available usage data     │
│  - Cache hits: use actual counts           │
│  - Cache misses: use default (count=0)     │
│  - NO per-read CF lookups on hot path      │
└────────────────────────────────────────────┘
```

**Changes:**
1. Add `CF_USAGE_COUNTERS` column family to storage

2. Add `UsageTracker` service with cache-first design:
   ```rust
   pub struct UsageTracker {
       /// LRU cache for hot doc IDs (bounded, e.g., 10K entries)
       cache: Mutex<LruCache<String, UsageStats>>,
       /// Pending writes (batched)
       pending_writes: DashMap<String, UsageUpdate>,
       /// Pending prefetch requests
       prefetch_queue: DashMap<String, ()>,
       storage: Arc<Storage>,
       config: UsageConfig,
   }

   impl UsageTracker {
       /// Record access (batched write, non-blocking)
       /// Updates cache immediately, queues CF write
       pub fn record_access(&self, doc_id: &str);

       /// Get usage for ranking - cache-first, NO blocking CF read
       /// Returns default UsageStats if not in cache
       pub fn get_usage_cached(&self, doc_id: &str) -> UsageStats;

       /// Batch get for ranking - returns available data, queues prefetch for misses
       pub fn get_batch_cached(&self, doc_ids: &[String]) -> Vec<(String, UsageStats)>;

       /// Flush pending writes (called by scheduler job, every 60s)
       pub async fn flush_writes(&self) -> Result<u32>;

       /// Process prefetch queue (called by scheduler job, every 5s)
       pub async fn process_prefetch(&self) -> Result<u32>;

       /// Warm cache on startup (load recent/frequent IDs)
       pub async fn warm_cache(&self) -> Result<u32>;
   }
   ```

3. **Write path** - Flush job runs periodically:
   - Batches pending writes every 60s (or on 1000-entry threshold)
   - Avoids write-per-read amplification
   - Single RocksDB WriteBatch

4. **Read path** - Cache-first with async prefetch:
   - `get_usage_cached()` NEVER blocks on CF read
   - Cache misses return default (count=0), queue prefetch
   - Prefetch job runs every 5s, populates cache
   - On next search, data is available from cache

5. Ranking integration (feature-flagged):
   ```rust
   fn rank_result(similarity: f32, salience: f32, usage: &UsageStats, config: &RankingConfig) -> f32 {
       if !config.usage_decay_enabled {
           return similarity * (0.55 + 0.45 * salience);
       }
       let usage_penalty = 1.0 / (1.0 + config.decay_factor * usage.access_count as f32);
       similarity * (0.55 + 0.45 * salience) * usage_penalty
   }
   ```

6. Configuration:
   ```toml
   [teleport.ranking.usage_decay]
   enabled = false  # OFF by default until validated
   decay_factor = 0.15
   flush_interval_secs = 60
   prefetch_interval_secs = 5
   cache_size = 10000  # LRU cache entries
   ```

7. Safe startup when CF absent:
   ```rust
   impl UsageTracker {
       pub fn new(storage: Arc<Storage>) -> Self {
           // Check if CF_USAGE_COUNTERS exists
           // If absent, create on first write (not on read)
           // All reads return defaults until CF is populated
       }
   }
   ```

**Metrics:**
- `usage_cache_hit_rate` - Gauge: cache hit % per minute
- `usage_cache_size` - Gauge: current cache entries
- `usage_writes_batched_total` - Counter: batched writes
- `usage_prefetch_total` - Counter: prefetch operations
- `usage_cf_read_latency_seconds` - Histogram: prefetch read latency

**Tests:**
- Unit tests for usage penalty calculation
- Unit test: cache-first returns default on miss (no CF read)
- Integration test: batch flush writes to CF
- Integration test: prefetch populates cache
- Perf test: verify no read-path latency increase (< 1ms overhead)
- Perf test: verify no write stall on search hot path

### Plan 16-03: Novelty Threshold (Best-Effort, Opt-In)

**Goal:** Prevent redundant storage - but NEVER block ingestion

**Design:**
- Novelty check is **DISABLED BY DEFAULT** (opt-in only)
- Explicit config flag required to enable
- If embedding/vector unavailable, skip check and store event
- Async check with timeout (50ms default, configurable)
- Full metrics for skip/timeout/reject rates
- **NEVER a hard gate** - always stores on any failure

**Gating Strategy:**

```
Event Arrives
     │
     ▼
┌────────────────────────────────────────┐
│ Check config: novelty.enabled?         │
│  - false (default) → SKIP, store event │
│  - true → proceed to novelty check     │
└────────┬───────────────────────────────┘
         │ enabled=true
         ▼
┌────────────────────────────────────────┐
│ Check dependencies available?          │
│  - embedder: None → SKIP, store        │
│  - vector_index: None → SKIP, store    │
│  - vector_index.ready: false → SKIP    │
└────────┬───────────────────────────────┘
         │ all available
         ▼
┌────────────────────────────────────────┐
│ Run check with timeout                 │
│  - timeout → SKIP, store               │
│  - error → SKIP, store                 │
│  - score > threshold → REJECT          │
│  - score ≤ threshold → STORE           │
└────────────────────────────────────────┘
```

**Changes:**
1. Add novelty checker with explicit opt-in:
   ```rust
   pub struct NoveltyChecker {
       embedder: Option<Arc<Embedder>>,
       vector_index: Option<Arc<VectorIndex>>,
       config: NoveltyConfig,
       metrics: NoveltyMetrics,
   }

   impl NoveltyChecker {
       /// Returns true if event should be stored (novel or check skipped)
       pub async fn should_store(&self, event: &Event) -> bool {
           // GATE 1: Feature must be explicitly enabled
           if !self.config.enabled {
               self.metrics.skipped_disabled.inc();
               return true;
           }

           // GATE 2: Embedder must be available
           let Some(embedder) = &self.embedder else {
               self.metrics.skipped_no_embedder.inc();
               tracing::debug!("Novelty check skipped: embedder unavailable");
               return true;
           };

           // GATE 3: Vector index must be available and ready
           let Some(index) = &self.vector_index else {
               self.metrics.skipped_no_index.inc();
               tracing::debug!("Novelty check skipped: vector index unavailable");
               return true;
           };

           if !index.is_ready() {
               self.metrics.skipped_index_not_ready.inc();
               tracing::debug!("Novelty check skipped: vector index not ready");
               return true;
           }

           // GATE 4: Check must complete within timeout
           let start = Instant::now();
           match tokio::time::timeout(
               Duration::from_millis(self.config.timeout_ms),
               self.check_similarity(event, embedder, index)
           ).await {
               Ok(Ok(is_novel)) => {
                   self.metrics.check_latency.observe(start.elapsed());
                   if is_novel {
                       self.metrics.stored_novel.inc();
                   } else {
                       self.metrics.rejected_duplicate.inc();
                       tracing::info!(
                           event_id = %event.id,
                           "Novelty check rejected duplicate"
                       );
                   }
                   is_novel
               }
               Ok(Err(e)) => {
                   self.metrics.skipped_error.inc();
                   tracing::warn!(?e, "Novelty check failed, storing anyway");
                   true
               }
               Err(_) => {
                   self.metrics.skipped_timeout.inc();
                   tracing::warn!(
                       timeout_ms = self.config.timeout_ms,
                       "Novelty check timed out, storing anyway"
                   );
                   true
               }
           }
       }
   }
   ```

2. Configuration with explicit enable flag:
   ```toml
   [teleport.ranking.novelty]
   # MUST be explicitly set to true to enable (default: false)
   enabled = false

   # Similarity threshold - events above this are considered duplicates
   # Range: 0.0-1.0, higher = stricter (more duplicates detected)
   threshold = 0.82

   # Maximum time to wait for novelty check (ms)
   # If exceeded, event is stored anyway
   timeout_ms = 50

   # Minimum event text length to check (skip very short events)
   min_text_length = 50
   ```

3. Metrics struct for observability:
   ```rust
   pub struct NoveltyMetrics {
       pub skipped_disabled: Counter,
       pub skipped_no_embedder: Counter,
       pub skipped_no_index: Counter,
       pub skipped_index_not_ready: Counter,
       pub skipped_error: Counter,
       pub skipped_timeout: Counter,
       pub stored_novel: Counter,
       pub rejected_duplicate: Counter,
       pub check_latency: Histogram,
   }
   ```

4. Prometheus metrics exposed:
   ```
   novelty_skipped_total{reason="disabled"}
   novelty_skipped_total{reason="no_embedder"}
   novelty_skipped_total{reason="no_index"}
   novelty_skipped_total{reason="index_not_ready"}
   novelty_skipped_total{reason="error"}
   novelty_skipped_total{reason="timeout"}
   novelty_stored_total
   novelty_rejected_total
   novelty_check_latency_seconds{quantile="0.5|0.9|0.99"}
   ```

5. Status RPC includes novelty state:
   ```protobuf
message TeleportStatus {
    // ... existing fields ...
    // NOTE: pick field numbers AFTER current highest in proto to avoid conflicts (e.g., start at 50+)
    bool novelty_enabled = 50;
    int64 novelty_checked_total = 51;
    int64 novelty_rejected_total = 52;
    int64 novelty_skipped_total = 53;
}
   ```

**Timeout Budget:**
- Default: 50ms (configurable)
- At 100 events/second ingest rate: 5 seconds/second budget
- Single embedding: ~30ms (local MiniLM)
- Single HNSW search: ~10ms
- Total per-event: ~40ms (within budget)
- If load increases, timeouts auto-shed load

**Tests:**
- Unit test: disabled by default (check config.enabled)
- Unit test: threshold comparison logic
- Integration test: fallback when embedder unavailable
- Integration test: fallback when index not ready
- Integration test: timeout behavior (inject slow embedder)
- Integration test: metrics increment correctly
- Perf test: verify timeout budget under load (100 events/s)

### Plan 16-04: Vector Pruning Automation (FR-08)

**Goal:** Implement FR-08 from Vector PRD via admin RPC, not scheduler-owned pipeline

**PRD Traceability:** Vector PRD FR-08 "Index Lifecycle Scheduler Job"

**Design:**
- Scheduler triggers prune via admin RPC (doesn't own embedder)
- Uses existing `VectorIndexPipeline::prune(age_days)` API
- Per-level retention enforced by checking doc_type in prune logic
- CLI command for manual prune with level/age options
- Status metrics exposed via GetVectorIndexStatus

**Retention Rules (from PRD, enforced in prune logic):**

| Level | Retention Days | Enforcement |
|-------|----------------|-------------|
| Segment | 30 | Prune vectors where `doc_type="segment"` AND `created_at < now - 30d` |
| Grip | 30 | Prune vectors where `doc_type="grip"` AND `created_at < now - 30d` |
| Day | 365 | Prune vectors where `doc_type="day"` AND `created_at < now - 365d` |
| Week | 1825 | Prune vectors where `doc_type="week"` AND `created_at < now - 1825d` |
| Month | NEVER | **PROTECTED** - Month vectors are never pruned |
| Year | NEVER | **PROTECTED** - Year vectors are never pruned |

**Changes:**
1. Extend prune API to support per-level retention:
   ```rust
   impl VectorIndexPipeline {
       /// Prune vectors per level using configured retention
       pub async fn prune_by_lifecycle(
           &self,
           config: &VectorLifecycleConfig,
           dry_run: bool
       ) -> Result<PruneStats> {
           let mut stats = PruneStats::default();
           let now = Utc::now();

           // PROTECTED: Never prune month/year
           for (level, retention_days) in [
               ("segment", config.segment_retention_days),
               ("grip", config.grip_retention_days),
               ("day", config.day_retention_days),
               ("week", config.week_retention_days),
           ] {
               let cutoff = now - Duration::days(retention_days as i64);
               let pruned = self.prune_level(level, cutoff, dry_run).await?;
               stats.add(level, pruned);
           }

           // Explicitly skip month and year
           tracing::info!("Skipping month/year vectors (protected)");

           stats
       }
   }
   ```

2. Add admin RPC with per-level support:
   ```protobuf
   message PruneVectorIndexRequest {
       // Optional: prune specific level only. Empty = all levels per config.
       string level = 1;  // "segment", "grip", "day", "week", or "" for all
       // Override retention days (0 = use config)
       uint32 age_days_override = 2;
       bool dry_run = 3;
   }

   message PruneVectorIndexResponse {
       bool success = 1;
       uint32 segments_pruned = 2;
       uint32 grips_pruned = 3;
       uint32 days_pruned = 4;
       uint32 weeks_pruned = 5;
       string message = 6;
   }

   rpc PruneVectorIndex(PruneVectorIndexRequest) returns (PruneVectorIndexResponse);
   ```

3. Scheduler job calls admin RPC:
   ```rust
   pub struct VectorPruneJob {
       admin_client: AdminClient,  // Calls RPC, doesn't own pipeline
       config: VectorLifecycleConfig,
   }

   impl VectorPruneJob {
       pub async fn run(&self) -> Result<()> {
           let response = self.admin_client
               .prune_vector_index(PruneVectorIndexRequest {
                   level: String::new(),  // All levels
                   age_days_override: 0,  // Use config
                   dry_run: false,
               })
               .await?;

           tracing::info!(
               segments = response.segments_pruned,
               grips = response.grips_pruned,
               days = response.days_pruned,
               weeks = response.weeks_pruned,
               "Vector prune job completed"
           );
           Ok(())
       }
   }
   ```

4. Add CLI command:
   ```bash
   # Prune all levels per config
   memory-daemon admin prune-vectors --dry-run
   memory-daemon admin prune-vectors

   # Prune specific level with override
   memory-daemon admin prune-vectors --level segment --age-days 14
   ```

5. Update GetVectorIndexStatus with lifecycle metrics:
   ```protobuf
message VectorIndexStatus {
       // ... existing fields ...
       // Use field numbers AFTER current max (e.g., 50+) to avoid collisions
       int64 last_prune_timestamp = 50;
       uint32 last_prune_segments_removed = 51;
       uint32 last_prune_grips_removed = 52;
       uint32 last_prune_days_removed = 53;
       uint32 last_prune_weeks_removed = 54;
       // Protected level counts (never pruned)
       uint32 month_vectors_count = 55;
       uint32 year_vectors_count = 56;
   }
   ```

6. Configuration (use existing namespace from PRD):
   ```toml
   [teleport.vector.lifecycle]
   enabled = true
   segment_retention_days = 30
   grip_retention_days = 30
   day_retention_days = 365
   week_retention_days = 1825
   # month/year: not configurable, always protected

   [teleport.vector.maintenance]
   prune_schedule = "0 3 * * *"  # Daily at 3 AM
   prune_batch_size = 1000
   optimize_after_prune = true
   ```

**Prometheus Metrics:**
```
vector_prune_total{level="segment|grip|day|week"}
vector_prune_latency_seconds
vector_prune_last_run_timestamp
vector_protected_count{level="month|year"}
```

**Tests:**
- Integration test: prune removes old segment/grip vectors
- Integration test: prune respects per-level retention
- Integration test: month/year vectors are NEVER pruned (protected)
- Integration test: dry-run reports without removing
- Integration test: status RPC shows prune metrics
- Test: CLI command with --level and --age-days flags

### Plan 16-05: BM25 Lifecycle (FR-09 Alignment)

**Goal:** Implement FR-09 per-level retention with scheduled prune and telemetry

**PRD Traceability:** BM25 PRD FR-09 "BM25 Lifecycle Pruning"

**PRD FR-09 Acceptance Criteria:**
- [x] Configurable per-level retention days for BM25 index (segment/day/week/month)
- [x] Scheduler job runs prune on a cron (default 03:00 daily)
- [x] Prune only removes BM25 docs; primary RocksDB data untouched
- [x] Post-prune optimize/compact keeps index healthy
- [x] TeleportStatus reports last prune time and pruned doc counts
- [x] CLI/admin command `memory-daemon admin prune-bm25 --age-days <n> --level <segment|day|week|all>`

**Design:**
Per PRD FR-09 requirements:
1. Per-level retention configuration (from PRD Section 7)
2. Scheduled prune command via admin RPC
3. Post-prune optimize/compact
4. Status/metrics via GetTeleportStatus
5. CLI command with --level filter

**Retention Rules (from PRD Section 7, enforced in prune logic):**

| Level | Retention Days | Enforcement |
|-------|----------------|-------------|
| Segment | 30 | Delete docs where `doc_type="segment"` AND `created_at < now - 30d` |
| Grip | 30 | Delete docs where `doc_type="grip"` AND `created_at < now - 30d` |
| Day | 180 | Delete docs where `doc_type="day"` AND `created_at < now - 180d` |
| Week | 1825 | Delete docs where `doc_type="week"` AND `created_at < now - 1825d` |
| Month | NEVER | **PROTECTED** - Month docs are never pruned |
| Year | NEVER | **PROTECTED** - Year docs are never pruned |

**Changes:**
1. Add prune-by-level to BM25 indexer:
   ```rust
   impl Bm25Indexer {
       /// Prune documents per level using configured retention
       pub async fn prune_by_lifecycle(
           &self,
           config: &Bm25LifecycleConfig,
           dry_run: bool
       ) -> Result<Bm25PruneStats> {
           let mut stats = Bm25PruneStats::default();
           let now = Utc::now();

           // PROTECTED: Never prune month/year
           for (level, retention_days) in [
               ("segment", config.segment_retention_days),
               ("grip", config.grip_retention_days),
               ("day", config.day_retention_days),
               ("week", config.week_retention_days),
           ] {
               let cutoff = now - Duration::days(retention_days as i64);

               // Use Tantivy delete_term on doc_type + timestamp range
               let deleted = self.delete_docs_before(level, cutoff, dry_run).await?;
               stats.add(level, deleted);
           }

           // Explicitly skip month and year
           tracing::info!("Skipping month/year docs (protected)");

           // Post-prune optimize (per FR-09)
           if !dry_run && stats.total() > 0 {
               self.writer.commit()?;
               self.optimize_index().await?;
           }

           stats
       }

       async fn delete_docs_before(
           &self,
           doc_type: &str,
           cutoff: DateTime<Utc>,
           dry_run: bool
       ) -> Result<u32> {
           // Query: doc_type=X AND created_at < cutoff
           let query = BooleanQuery::new(vec![
               (Occur::Must, TermQuery::new(doc_type_term(doc_type))),
               (Occur::Must, RangeQuery::new_date_max(cutoff.timestamp_millis())),
           ]);

           if dry_run {
               let count = self.searcher.search(&query, &Count)?;
               return Ok(count as u32);
           }

           let doc_ids = self.searcher.search(&query, &DocSetCollector)?;
           for doc_id in &doc_ids {
               self.writer.delete_document(*doc_id)?;
           }
           Ok(doc_ids.len() as u32)
       }

       async fn optimize_index(&self) -> Result<()> {
           // Merge segments after delete (per FR-09 "Post-prune optimize/compact")
           self.writer.merge(&MergePolicy::default()).await?;
           tracing::info!("BM25 index optimized after prune");
           Ok(())
       }
   }
   ```

2. Add admin RPC with per-level support (per FR-09):
   ```protobuf
   message PruneBm25IndexRequest {
       // Optional: prune specific level only. Empty = all levels per config.
       // Valid values: "segment", "grip", "day", "week", "all", ""
       string level = 1;
       // Override retention days (0 = use config)
       uint32 age_days_override = 2;
       bool dry_run = 3;
   }

   message PruneBm25IndexResponse {
       bool success = 1;
       uint32 segments_pruned = 2;
       uint32 grips_pruned = 3;
       uint32 days_pruned = 4;
       uint32 weeks_pruned = 5;
       string message = 6;
   }

   rpc PruneBm25Index(PruneBm25IndexRequest) returns (PruneBm25IndexResponse);
   ```

3. Scheduler job calls prune RPC:
   ```rust
   pub struct Bm25PruneJob {
       admin_client: AdminClient,
       config: Bm25LifecycleConfig,
   }

   impl Bm25PruneJob {
       pub async fn run(&self) -> Result<()> {
           if !self.config.enabled {
               tracing::debug!("BM25 lifecycle disabled, skipping prune");
               return Ok(());
           }

           let response = self.admin_client
               .prune_bm25_index(PruneBm25IndexRequest {
                   level: String::new(),  // All levels
                   age_days_override: 0,  // Use config
                   dry_run: false,
               })
               .await?;

           tracing::info!(
               segments = response.segments_pruned,
               grips = response.grips_pruned,
               days = response.days_pruned,
               weeks = response.weeks_pruned,
               "BM25 prune job completed"
           );
           Ok(())
       }
   }
   ```

4. Update GetTeleportStatus with lifecycle metrics (per FR-09):
   ```protobuf
message TeleportStatus {
       // ... existing fields ...
       // Use field numbers AFTER current max (e.g., 60+) to avoid collisions
       int64 bm25_last_prune_timestamp = 60;
       uint32 bm25_last_prune_segments = 61;
       uint32 bm25_last_prune_grips = 62;
       uint32 bm25_last_prune_days = 63;
       uint32 bm25_last_prune_weeks = 64;
       // Protected level counts
       uint32 bm25_month_docs_count = 65;
       uint32 bm25_year_docs_count = 66;
   }
   ```

5. CLI command (per FR-09):
   ```bash
   # Prune all levels per config
   memory-daemon admin prune-bm25 --dry-run
   memory-daemon admin prune-bm25

   # Prune specific level with override (per FR-09)
   memory-daemon admin prune-bm25 --level segment --age-days 14
   memory-daemon admin prune-bm25 --level all --age-days 30
   ```

6. Configuration (opt-in per PRD "append-only by default"):
   ```toml
   [teleport.bm25.lifecycle]
   # MUST be explicitly enabled (PRD default: append-only, no eviction)
   enabled = false

   # Per-level retention (from PRD Section 7)
   segment_retention_days = 30
   grip_retention_days = 30
   day_retention_days = 180
   week_retention_days = 1825
   # month/year: not configurable, always protected

   [teleport.bm25.maintenance]
   prune_schedule = "0 3 * * *"  # Daily at 3 AM (per FR-09)
   optimize_after_prune = true   # Per FR-09 "Post-prune optimize/compact"
   ```

**Prometheus Metrics:**
```
bm25_prune_total{level="segment|grip|day|week"}
bm25_prune_latency_seconds
bm25_prune_last_run_timestamp
bm25_optimize_latency_seconds
bm25_protected_count{level="month|year"}
```

**Tests:**
- Integration test: prune removes old segment/grip/day/week docs
- Integration test: prune respects per-level retention
- Integration test: month/year docs are NEVER pruned (protected)
- Integration test: dry-run reports without removing
- Integration test: optimize runs after prune
- Integration test: status RPC shows prune metrics
- Test: CLI command with --level and --age-days flags
- Test: disabled by default (config.enabled check)

---

## Ranking Fusion Strategy

### Problem

New salience/usage factors are introduced, but there's no recalibration with existing topic time-decay and hybrid score fusion. Without staged rollout, ranking quality could regress.

### Solution: Per-Signal Feature Flags with Staged Rollout

**Current Ranking (v2.0.0):**
```rust
// Hybrid search ranking
fn current_ranking(bm25_score: f32, vector_score: f32, config: &HybridConfig) -> f32 {
    // Reciprocal Rank Fusion
    let rrf_score = (1.0 / (60.0 + bm25_rank)) + (1.0 / (60.0 + vector_rank));
    rrf_score * config.bm25_weight * config.vector_weight
}

// Topic ranking (existing time-decay)
fn topic_importance(topic: &Topic) -> f32 {
    // 30-day half-life exponential decay
    let age_days = (now - topic.last_seen).num_days() as f32;
    let decay = 0.5_f32.powf(age_days / 30.0);
    topic.mention_count as f32 * decay
}
```

**New Ranking (Phase 16) - Additive with Flags:**
```rust
fn phase16_ranking(
    similarity: f32,       // From BM25/Vector/RRF
    salience: f32,         // NEW: Write-time score (0.0-1.0)
    usage: &UsageStats,    // NEW: From CF_USAGE_COUNTERS
    config: &RankingConfig,
) -> f32 {
    let mut score = similarity;

    // Signal 1: Salience factor (feature-flagged)
    if config.salience_enabled {
        let salience_factor = 0.55 + 0.45 * salience;
        score *= salience_factor;
    }

    // Signal 2: Usage decay (feature-flagged)
    if config.usage_decay_enabled {
        let usage_penalty = 1.0 / (1.0 + config.decay_factor * usage.access_count as f32);
        score *= usage_penalty;
    }

    // Signal 3: Topic time-decay (already exists, unchanged)
    // Handled separately in TopicSearch, not modified here

    score
}
```

### Staged Rollout Plan

| Stage | Salience | Usage Decay | Novelty | Duration | Exit Criteria |
|-------|----------|-------------|---------|----------|---------------|
| 0 (v2.0.0) | OFF | OFF | OFF | Baseline | N/A |
| 1 | ON | OFF | OFF | 1 week | No ranking regressions in tests |
| 2 | ON | ON | OFF | 2 weeks | Cache hit rate > 80%, no latency increase |
| 3 | ON | ON | ON (opt-in) | Ongoing | Metrics show value |

### Rollback Plan

If ranking quality degrades:
1. Disable individual signal via config (no code deploy)
2. Restart daemon
3. Ranking reverts to previous behavior immediately

```toml
# Emergency rollback: disable all new signals
[teleport.ranking]
enabled = false  # Master switch disables all ranking enhancements
```

### Integration with Existing Topic Time-Decay

Topic importance scoring is **unchanged** - Phase 16 signals are additive:

```
Query Results
     │
     ▼
┌────────────────────────────────────────┐
│ Search Layer (BM25/Vector/Topics)      │
│  - BM25: TF-IDF keyword matching       │
│  - Vector: Cosine similarity           │
│  - Topics: Time-decayed importance     │  ← Existing, unchanged
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ RRF Fusion (existing)                  │
│  - Combines BM25 + Vector ranks        │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ Phase 16 Post-Processing (NEW)         │
│  - Salience factor (if enabled)        │
│  - Usage decay (if enabled)            │
│  - Applied AFTER existing ranking      │
└────────────────────────────────────────┘
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/memory-types/src/lib.rs` | Export salience module |
| `crates/memory-types/src/salience.rs` | New file |
| `crates/memory-types/src/toc.rs` | Add salience fields (immutable) |
| `crates/memory-types/src/grip.rs` | Add salience fields (immutable) |
| `crates/memory-storage/src/column_families.rs` | Add CF_USAGE_COUNTERS |
| `crates/memory-storage/src/usage.rs` | New UsageTracker |
| `crates/memory-toc/src/builder.rs` | Calculate salience on creation |
| `crates/memory-service/src/hybrid.rs` | Integrate salience/usage into ranking |
| `crates/memory-service/src/ingest.rs` | Add best-effort novelty check |
| `crates/memory-service/src/admin.rs` | Add prune RPCs |
| `crates/memory-scheduler/src/jobs/` | Add prune jobs (call RPCs) |
| `crates/memory-search/src/indexer.rs` | Add prune() method |
| `crates/memory-daemon/src/admin.rs` | Add prune CLI commands |
| `proto/memory.proto` | Add salience fields, prune RPCs, status metrics |

---

## Configuration

### Schema Documentation

All Phase 16 config lives under existing `[teleport]` namespace for consistency with PRDs.

**Config file:** `~/.config/agent-memory/config.toml`

```toml
# =============================================================================
# PHASE 16: MEMORY RANKING ENHANCEMENTS
# =============================================================================

# -----------------------------------------------------------------------------
# RANKING POLICY (NEW)
# Controls ranking signals applied to search results
# -----------------------------------------------------------------------------

[teleport.ranking]
# Master switch for all ranking enhancements
# Set to false for emergency rollback to v2.0.0 behavior
enabled = true

[teleport.ranking.salience]
# Salience scoring: boost important memories at write time
# Applied to new TocNodes and Grips only (existing data uses defaults)
enabled = true
# Weight for text length density (longer = more salient), range: 0.0-1.0
length_density_weight = 0.45
# Boost for special memory kinds (preference/procedure/constraint/definition)
kind_boost = 0.20
# Boost for pinned memories
pinned_boost = 0.20

[teleport.ranking.usage_decay]
# Usage-based decay: penalize frequently-accessed memories
# DISABLED by default - enable after validating cache performance
enabled = false
# Decay factor: higher = more aggressive penalty for high-access items
# Formula: 1 / (1 + decay_factor * access_count)
decay_factor = 0.15
# How often to flush pending writes to CF_USAGE_COUNTERS (seconds)
flush_interval_secs = 60
# How often to process prefetch queue for cache population (seconds)
prefetch_interval_secs = 5
# LRU cache size for hot doc_ids
cache_size = 10000

[teleport.ranking.novelty]
# Novelty threshold: prevent storing near-duplicate events
# DISABLED by default - explicitly opt-in required
enabled = false
# Similarity threshold: events above this are considered duplicates (0.0-1.0)
# Higher = stricter, more duplicates detected
threshold = 0.82
# Maximum time for novelty check (ms). If exceeded, event is stored anyway.
timeout_ms = 50
# Skip novelty check for events shorter than this (characters)
min_text_length = 50

# -----------------------------------------------------------------------------
# VECTOR INDEX LIFECYCLE (FR-08)
# Controls automatic pruning of old vectors from HNSW index
# -----------------------------------------------------------------------------

[teleport.vector.lifecycle]
# Enable automatic vector pruning (recommended)
enabled = true
# Retention days per level (per PRD Section 13)
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 365
week_retention_days = 1825
# NOTE: month and year vectors are NEVER pruned (protected)

[teleport.vector.maintenance]
# Cron schedule for prune job (default: daily 3 AM)
prune_schedule = "0 3 * * *"
# Batch size for prune operations to avoid blocking
prune_batch_size = 1000
# Run index optimization after pruning
optimize_after_prune = true

# -----------------------------------------------------------------------------
# BM25 INDEX LIFECYCLE (FR-09)
# Controls automatic pruning of old docs from Tantivy index
# DISABLED by default per PRD "append-only, no eviction" philosophy
# -----------------------------------------------------------------------------

[teleport.bm25.lifecycle]
# MUST be explicitly enabled (PRD default: append-only)
enabled = false
# Retention days per level (per PRD Section 7)
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 180
week_retention_days = 1825
# NOTE: month and year docs are NEVER pruned (protected)

[teleport.bm25.maintenance]
# Cron schedule for prune job (default: daily 3 AM)
prune_schedule = "0 3 * * *"
# Run index optimization after pruning (per FR-09)
optimize_after_prune = true
```

### Config Validation

On daemon startup, validate Phase 16 config:

```rust
impl RankingConfig {
    pub fn validate(&self) -> Result<()> {
        // Each weight must be in range; they are applied multiplicatively, not expected to sum.
        for w in [
            self.salience.length_density_weight,
            self.salience.kind_boost,
            self.salience.pinned_boost,
        ] {
            if !(0.0..=1.0).contains(&w) {
                return Err(ConfigError::InvalidSalienceWeight(w));
            }
        }

        if !(0.0..=1.0).contains(&self.novelty.threshold) {
            return Err(ConfigError::InvalidNoveltyThreshold(self.novelty.threshold));
        }

        if self.usage_decay.decay_factor <= 0.0 {
            return Err(ConfigError::InvalidDecayFactor(self.usage_decay.decay_factor));
        }

        Ok(())
    }
}
```

### Environment Variable Overrides

For operational flexibility (e.g., emergency disable):

```bash
# Disable all ranking enhancements
AGENT_MEMORY_TELEPORT_RANKING_ENABLED=false

# Disable specific signals
AGENT_MEMORY_TELEPORT_RANKING_SALIENCE_ENABLED=false
AGENT_MEMORY_TELEPORT_RANKING_USAGE_DECAY_ENABLED=false
AGENT_MEMORY_TELEPORT_RANKING_NOVELTY_ENABLED=false
```

---

## Backward Compatibility

### Breaking Changes: NONE

Phase 16 is fully backward compatible with v2.0.0 data.

| Change | Impact | Mitigation |
|--------|--------|------------|
| New salience fields on TocNode/Grip | Existing nodes lack fields | Default values on read |
| New CF_USAGE_COUNTERS | CF doesn't exist | Created on first write, reads return defaults |
| New proto fields | Old clients don't send | Proto3 defaults (0, false, empty) |
| New config keys | Old configs don't have them | Compile-time defaults |

### Schema Changes

**TocNode and Grip (memory-types):**

```rust
// v2.0.0 (existing)
pub struct TocNode {
    pub node_id: String,
    pub level: TocLevel,
    pub title: String,
    pub bullets: Vec<Bullet>,
    pub keywords: Vec<String>,
    pub created_at: DateTime<Utc>,
    // ...
}

// v2.1.0 (Phase 16) - ADDITIVE ONLY
pub struct TocNode {
    pub node_id: String,
    pub level: TocLevel,
    pub title: String,
    pub bullets: Vec<Bullet>,
    pub keywords: Vec<String>,
    pub created_at: DateTime<Utc>,
    // NEW fields with defaults for backward compatibility
    #[serde(default = "default_salience")]
    pub salience_score: f32,  // Default: 0.5
    #[serde(default)]
    pub memory_kind: MemoryKind,  // Default: Observation
    #[serde(default)]
    pub is_pinned: bool,  // Default: false
}

fn default_salience() -> f32 { 0.5 }
```

**Proto changes (memory.proto):**

```protobuf
// v2.0.0 (existing)
message TocNode {
    string node_id = 1;
    TocLevel level = 2;
    // ... existing fields
}

// v2.1.0 (Phase 16) - ADDITIVE ONLY
message TocNode {
    string node_id = 1;
    TocLevel level = 2;
    // ... existing fields

    // NEW fields (field numbers > 100 to avoid conflicts)
    float salience_score = 101;  // Default: 0.0 (treated as 0.5)
    MemoryKind memory_kind = 102;  // Default: OBSERVATION
    bool is_pinned = 103;  // Default: false
}

// Special handling for salience_score default
// Proto3 default is 0.0, but we want 0.5 for neutral
// Service layer translates: 0.0 → 0.5 on read
```

### Migration Strategy

**Phase 1: Deploy (No Migration Required)**

1. Deploy new code with Phase 16 features
2. All features disabled by default
3. Existing data reads normally with default values
4. New data written with salience scores

**Phase 2: Enable Features Incrementally**

1. Enable salience scoring → new nodes get scored
2. Wait 1 week, verify ranking quality
3. Enable usage tracking → CF created on first access
4. Wait 2 weeks, verify cache hit rate
5. Enable novelty (opt-in per user request)

**Phase 3: Optional Backfill (NOT Required)**

If desired, can backfill salience for existing nodes:

```bash
# Optional: recompute salience for existing nodes
memory-daemon admin backfill-salience --dry-run
memory-daemon admin backfill-salience --since 2026-01-01
```

This is NOT required - defaults work fine.

### Compatibility Tests

```rust
#[test]
fn test_deserialize_v200_toc_node() {
    // v2.0.0 serialized node (no salience fields)
    let json = r#"{"node_id":"toc:day:2026-01-01","level":"day",...}"#;
    let node: TocNode = serde_json::from_str(json).unwrap();

    // Should use defaults
    assert_eq!(node.salience_score, 0.5);
    assert_eq!(node.memory_kind, MemoryKind::Observation);
    assert_eq!(node.is_pinned, false);
}

#[test]
fn test_usage_cf_absent() {
    // CF_USAGE_COUNTERS doesn't exist yet
    let tracker = UsageTracker::new(storage);

    // Should return default stats, not error
    let stats = tracker.get_usage_cached("toc:day:2026-01-01");
    assert_eq!(stats.access_count, 0);
    assert!(stats.last_accessed.is_none());
}

#[test]
fn test_old_proto_client() {
    // Client sends TocNode without salience fields
    let request = GetNodeRequest { node_id: "..." };

    // Response should include salience with defaults
    let response = service.get_node(request).await?;
    assert_eq!(response.node.salience_score, 0.5);
}
```

### Version Gating

No explicit version gating needed because:
1. All new fields have serde defaults
2. Proto3 has implicit defaults (0, false, empty)
3. Features are disabled by default in config
4. CF created lazily on first write

### Rollback Procedure

If issues discovered after enabling Phase 16:

1. Disable features in config (no code change needed)
2. Restart daemon
3. Ranking reverts to v2.0.0 behavior
4. CF_USAGE_COUNTERS data retained but ignored
5. Salience fields retained but unused (factor = 1.0)

---

## Success Criteria

### Tier 1: Ranking Policy

| # | Criterion | PRD Trace | Verification |
|---|-----------|-----------|--------------|
| 1 | Salience scoring applied to new TOC nodes and Grips (write-time only) | RFC | Unit test: new node has salience_score |
| 2 | Salience defaults (0.5/Observation/false) work for existing data | RFC | Compat test: deserialize v2.0.0 node |
| 3 | Usage counters stored in separate CF (CF_USAGE_COUNTERS) | RFC | Integration test: verify CF writes |
| 4 | Usage read path uses cache-first (no CF read on hot path) | RFC | Perf test: search latency < 1ms overhead |
| 5 | Usage flush job batches writes (no per-read write amplification) | RFC | Perf test: no write stalls under load |
| 6 | Ranking incorporates salience (feature-flagged) | RFC | Unit test: salience=1.0 when disabled |
| 7 | Ranking incorporates usage decay (feature-flagged) | RFC | Unit test: decay=1.0 when disabled |
| 8 | Novelty check is opt-in (disabled by default) | RFC | Config test: default = false |
| 9 | Novelty check has fallback on embedder/index unavailable | RFC | Integration test: store on fallback |
| 10 | Novelty metrics track skip/timeout/reject rates | RFC | Metrics test: counters increment |

### Tier 1.5: Lifecycle Automation

| # | Criterion | PRD Trace | Verification |
|---|-----------|-----------|--------------|
| 11 | Vector pruning via admin RPC | Vector FR-08 | Integration test: prune removes vectors |
| 12 | Vector prune respects per-level retention | Vector FR-08 | Test: segment@30d, day@365d, week@1825d |
| 13 | Vector prune protects month/year (never pruned) | Vector FR-08 | Test: month/year count unchanged |
| 14 | Vector scheduler job calls admin RPC | Vector FR-08 | Integration test: job runs successfully |
| 15 | BM25 pruning via admin RPC | BM25 FR-09 | Integration test: prune removes docs |
| 16 | BM25 prune respects per-level retention | BM25 FR-09 | Test: segment@30d, day@180d, week@1825d |
| 17 | BM25 prune protects month/year (never pruned) | BM25 FR-09 | Test: month/year count unchanged |
| 18 | BM25 post-prune optimize runs | BM25 FR-09 | Test: optimize called after prune |
| 19 | GetVectorIndexStatus includes prune metrics | Vector FR-08 | RPC test: last_prune_* fields |
| 20 | GetTeleportStatus includes prune metrics | BM25 FR-09 | RPC test: bm25_last_prune_* fields |
| 21 | CLI `prune-vectors` with --level, --age-days, --dry-run | Vector FR-09 | CLI test: all flags work |
| 22 | CLI `prune-bm25` with --level, --age-days, --dry-run | BM25 FR-09 | CLI test: all flags work |

### Cross-Cutting

| # | Criterion | PRD Trace | Verification |
|---|-----------|-----------|--------------|
| 23 | All features behind config flags | All | Config test: verify defaults |
| 24 | Master ranking switch disables all signals | RFC | Config test: enabled=false → no effect |
| 25 | Environment variable overrides work | RFC | Env test: override via AGENT_MEMORY_* |
| 26 | Backward compatible with v2.0.0 data | All | Compat test: read old data |
| 27 | No breaking proto changes | All | Proto test: old client compatibility |
| 28 | All tests pass (unit, integration, perf, compat) | All | CI pipeline green |
| 29 | PRDs updated to reflect implementation | Vector FR-08, BM25 FR-09 | Doc review |

### Performance Criteria

| Metric | Target | Verification |
|--------|--------|--------------|
| Search latency overhead (with usage tracking) | < 1ms | Perf test |
| Usage cache hit rate | > 80% after warmup | Metrics |
| Novelty check latency | < 50ms (default timeout) | Perf test |
| No write stalls on search path | 0 | Perf test under load |
| Prune job duration (10K vectors) | < 60s | Perf test |

---

## Verification

### Build and Test

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace --all-features

# Run specific test categories
cargo test --workspace unit           # Unit tests
cargo test --workspace integration    # Integration tests
cargo test --workspace compat         # Backward compatibility tests
cargo test --workspace perf -- --ignored  # Performance tests (slow)
```

### Salience Scoring

```bash
# Create test event with known content
memory-daemon ingest --text "I prefer to use async/await patterns for Rust code"

# Query node and verify salience
memory-daemon query node --node-id "toc:day:2026-02-04" --format json | jq '.salience_score'
# Expected: > 0.5 (preference detected)

# Verify existing data uses defaults
memory-daemon query node --node-id "toc:day:2026-01-01" --format json | jq '.salience_score'
# Expected: 0.5 (default for existing data)
```

### Usage Tracking

```bash
# Check CF exists after first search
memory-daemon admin storage-stats
# Should show CF_USAGE_COUNTERS in column families

# Check usage stats
memory-daemon admin usage-stats
# Shows: cache_size, cache_hit_rate, pending_writes

# Verify cache-first behavior (no latency increase)
hyperfine --warmup 3 'memory-daemon teleport hybrid-search -q "test"'
# Compare to baseline v2.0.0 latency
```

### Novelty Check

```bash
# Verify disabled by default
memory-daemon config get teleport.ranking.novelty.enabled
# Expected: false

# Enable novelty (for testing)
memory-daemon config set teleport.ranking.novelty.enabled true
memory-daemon restart

# Ingest duplicate event
memory-daemon ingest --text "This is a test message"
memory-daemon ingest --text "This is a test message"  # Near-duplicate

# Check logs for rejection
grep "Novelty check rejected duplicate" /var/log/agent-memory/daemon.log

# Check metrics
curl http://localhost:50051/metrics | grep novelty
# Expected: novelty_rejected_total > 0

# Verify fallback: stop vector index, ingest should still work
memory-daemon config set teleport.vector.enabled false
memory-daemon restart
memory-daemon ingest --text "New event without vector index"
# Should succeed with log: "Novelty check skipped: vector index unavailable"
```

### Vector Pruning (FR-08)

```bash
# Dry run - see what would be pruned
memory-daemon admin prune-vectors --dry-run
# Output: Would prune: segments=X, grips=Y, days=Z, weeks=W

# Prune specific level
memory-daemon admin prune-vectors --level segment --age-days 14 --dry-run

# Execute prune
memory-daemon admin prune-vectors
# Output: Pruned: segments=X, grips=Y, days=Z, weeks=W

# Verify month/year protected
memory-daemon teleport vector-stats --format json | jq '.month_vectors_count, .year_vectors_count'
# Should be unchanged after prune

# Check status metrics
memory-daemon teleport status --format json | jq '.last_prune_timestamp, .last_prune_segments_removed'
```

### BM25 Pruning (FR-09)

```bash
# Verify disabled by default
memory-daemon config get teleport.bm25.lifecycle.enabled
# Expected: false

# Enable for testing
memory-daemon config set teleport.bm25.lifecycle.enabled true

# Dry run
memory-daemon admin prune-bm25 --dry-run
# Output: Would prune: segments=X, grips=Y, days=Z, weeks=W

# Execute prune
memory-daemon admin prune-bm25

# Verify month/year protected
memory-daemon teleport stats --format json | jq '.bm25_month_docs_count, .bm25_year_docs_count'
# Should be unchanged after prune

# Check status metrics
memory-daemon teleport status --format json | jq '.bm25_last_prune_timestamp'
```

### Scheduler Jobs

```bash
# Check registered jobs
memory-daemon scheduler status
# Expected: vector_prune (daily 3 AM), bm25_prune (daily 3 AM if enabled)

# Force job execution (for testing)
memory-daemon scheduler run-now vector_prune
memory-daemon scheduler run-now bm25_prune

# Check job history
memory-daemon scheduler history --job vector_prune --limit 5
```

### Metrics Verification

```bash
# Prometheus metrics endpoint
curl http://localhost:50051/metrics | grep -E "(usage_|novelty_|vector_prune|bm25_prune)"

# Expected metrics:
# usage_cache_hit_rate
# usage_cache_size
# usage_writes_batched_total
# novelty_skipped_total{reason="disabled|no_embedder|..."}
# novelty_rejected_total
# vector_prune_total{level="segment|grip|day|week"}
# bm25_prune_total{level="segment|grip|day|week"}
```

### Rollback Test

```bash
# Disable all Phase 16 features
memory-daemon config set teleport.ranking.enabled false
memory-daemon restart

# Verify ranking reverts to v2.0.0 behavior
memory-daemon teleport hybrid-search -q "test" --debug
# Debug output should show: salience_factor=1.0, usage_penalty=1.0
```

---

## PRD Updates Required

After implementation, update these PRDs to reflect actual behavior:

1. **docs/prds/hierarchical-vector-indexing-prd.md**
   - Document that FR-08 (Index Lifecycle Scheduler Job) is now implemented
   - Add CLI command reference for `prune-vectors`
   - Add scheduler job details

2. **docs/prds/bm25-teleport-prd.md**
   - Clarify that "append-only, no eviction" applies to primary storage
   - Add section on how "eventually only month-level indexed" is achieved via filtered rebuilds
   - Add lifecycle configuration section

---

## Future Work (Deferred)

If Tier 1 + Lifecycle proves valuable, consider:

- **Tier 2: Episodic Memory** - Task outcome tracking, lessons learned
- **Tier 3: Consolidation** - Extract preferences/constraints/procedures

See RFC for full details: [docs/plans/memory-ranking-enhancements-rfc.md](docs/plans/memory-ranking-enhancements-rfc.md)
