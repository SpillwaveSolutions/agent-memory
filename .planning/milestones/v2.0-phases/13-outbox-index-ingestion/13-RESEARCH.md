# Phase 13: Outbox Index Ingestion - Research

**Researched:** 2026-02-01
**Domain:** Event-driven index ingestion, outbox pattern, checkpoint-based crash recovery, async indexing
**Confidence:** HIGH

## Summary

Phase 13 implements the Index Lifecycle layer of the Cognitive Architecture, connecting the outbox pattern (already established in memory-storage) to both BM25/Tantivy (Phase 11) and Vector/HNSW (Phase 12) indexes. The outbox column family (CF_OUTBOX) already stores entries atomically with events via `put_event()`. This phase builds the consumer side that reads outbox entries, dispatches them to appropriate indexers, and tracks progress via checkpoints for crash recovery.

The research confirms the existing codebase has all necessary infrastructure: outbox entries are written with monotonic sequence keys, checkpoint storage is implemented in CF_CHECKPOINTS, and the scheduler (Phase 10) provides the execution framework for background jobs. The implementation follows the transactional outbox pattern with at-least-once delivery semantics, requiring idempotent index operations.

**Primary recommendation:** Create a unified `IndexingPipeline` in a new `memory-indexing` crate that consumes outbox entries, dispatches to BM25/vector indexers based on entry type, tracks checkpoint per index type, and integrates with the scheduler for periodic processing. Support full rebuild via admin command that iterates all TOC nodes and grips from RocksDB.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| memory-storage | workspace | Outbox reading, checkpoint persistence | Already has outbox/checkpoint APIs |
| memory-scheduler | workspace | Background job execution | Already has cron scheduling, overlap policy |
| memory-search (Phase 11) | workspace | BM25/Tantivy indexing | Provides SearchIndexer for text indexing |
| memory-vector (Phase 12) | workspace | HNSW vector indexing | Provides VectorIndexPipeline for embeddings |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio | 1.43 | Async runtime, spawn_blocking | Already in workspace |
| tracing | 0.1 | Logging | Already in workspace |
| serde | 1.0 | Checkpoint serialization | Already in workspace |
| chrono | 0.4 | Timestamp handling | Already in workspace |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Poll-based consumer | Push-based (channel) | Poll simpler for batch processing, push adds complexity |
| Single checkpoint | Per-index checkpoints | Per-index allows independent recovery |
| Unified pipeline | Separate pipelines | Unified simpler, shares outbox iteration |

**Installation:**
```toml
# New crate in workspace
[package]
name = "memory-indexing"
version = "0.1.0"
edition = "2021"

[dependencies]
memory-storage = { workspace = true }
memory-types = { workspace = true }
memory-search = { workspace = true }
memory-vector = { workspace = true }
tokio = { workspace = true, features = ["rt-multi-thread", "sync"] }
tracing = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
chrono = { workspace = true, features = ["serde"] }
thiserror = { workspace = true }
```

## Architecture Patterns

### Recommended Project Structure
```
crates/
  memory-indexing/           # NEW crate for index lifecycle
    src/
      lib.rs                 # Public API exports
      pipeline.rs            # IndexingPipeline: unified outbox consumer
      consumer.rs            # OutboxConsumer: reads and dispatches entries
      checkpoint.rs          # IndexCheckpoint: per-index progress tracking
      rebuild.rs             # Full rebuild from storage
      error.rs               # Error types
    Cargo.toml
```

### Pattern 1: Unified Outbox Consumer with Dispatch
**What:** Single consumer reads outbox, dispatches to appropriate indexer based on action type
**When to use:** When multiple indexes need updates from same source
**Example:**
```rust
// Source: Derived from existing memory-toc/rollup.rs patterns
pub struct IndexingPipeline {
    storage: Arc<Storage>,
    bm25_indexer: Option<Arc<SearchIndexer>>,
    vector_indexer: Option<Arc<VectorIndexPipeline>>,
    batch_size: usize,
}

impl IndexingPipeline {
    pub async fn process_batch(&self) -> Result<IndexingStats, IndexingError> {
        // Load checkpoint to get last processed sequence
        let checkpoint = self.load_checkpoint()?;
        let start_seq = checkpoint.map(|c| c.last_sequence + 1).unwrap_or(0);

        // Read batch of outbox entries
        let entries = self.storage.get_outbox_entries(start_seq, self.batch_size)?;

        let mut stats = IndexingStats::default();
        for entry in entries {
            match entry.action {
                OutboxAction::IndexEvent => {
                    // Index for both BM25 and vector if available
                    if let Some(ref bm25) = self.bm25_indexer {
                        self.index_for_bm25(bm25, &entry).await?;
                        stats.bm25_indexed += 1;
                    }
                    if let Some(ref vector) = self.vector_indexer {
                        self.index_for_vector(vector, &entry).await?;
                        stats.vector_indexed += 1;
                    }
                }
                OutboxAction::UpdateToc => {
                    // TOC updates trigger index of new TOC node
                    self.index_toc_update(&entry).await?;
                    stats.toc_updates += 1;
                }
            }

            // Update checkpoint after each entry (crash recovery)
            self.save_checkpoint(entry.sequence)?;
        }

        Ok(stats)
    }
}
```

### Pattern 2: Checkpoint Per Index Type
**What:** Track separate checkpoints for BM25 and vector indexes
**When to use:** When indexes may have different processing speeds or failures
**Example:**
```rust
// Source: Derived from memory-toc/rollup.rs RollupCheckpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCheckpoint {
    pub index_type: IndexType,  // BM25 or Vector
    pub last_sequence: u64,     // Last outbox sequence processed
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub last_processed_time: DateTime<Utc>,
    pub processed_count: u64,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexType {
    Bm25,
    Vector,
    Combined, // For unified processing
}

impl IndexCheckpoint {
    pub fn checkpoint_key(index_type: &IndexType) -> String {
        match index_type {
            IndexType::Bm25 => "index_bm25".to_string(),
            IndexType::Vector => "index_vector".to_string(),
            IndexType::Combined => "index_combined".to_string(),
        }
    }
}
```

### Pattern 3: Idempotent Index Updates
**What:** Use delete-then-add pattern for updates; skip if already indexed
**When to use:** Always - at-least-once delivery requires idempotency
**Example:**
```rust
// Source: Phase 11 research - Tantivy delete/add pattern
impl IndexingPipeline {
    async fn index_toc_node(&self, node: &TocNode) -> Result<(), IndexingError> {
        if let Some(ref bm25) = self.bm25_indexer {
            // Delete existing document first (idempotent)
            bm25.delete_document(&node.node_id)?;
            // Add new version
            bm25.index_toc_node(node)?;
        }

        if let Some(ref vector) = self.vector_indexer {
            // Vector index also uses delete-then-add
            vector.update_toc_node(node).await?;
        }

        Ok(())
    }
}
```

### Pattern 4: Full Rebuild from Storage
**What:** Admin command iterates all TOC nodes and grips, re-indexes completely
**When to use:** After corruption, model upgrade, or initial setup
**Example:**
```rust
// Source: Derived from docs/prds/hierarchical-vector-indexing-prd.md
impl IndexingPipeline {
    pub async fn rebuild_all(&self) -> Result<RebuildStats, IndexingError> {
        info!("Starting full index rebuild");

        // Clear existing indexes
        if let Some(ref bm25) = self.bm25_indexer {
            bm25.clear()?;
        }
        if let Some(ref vector) = self.vector_indexer {
            vector.clear()?;
        }

        let mut stats = RebuildStats::default();

        // Iterate all TOC nodes by level
        for level in [TocLevel::Year, TocLevel::Month, TocLevel::Week,
                      TocLevel::Day, TocLevel::Segment] {
            let nodes = self.storage.get_toc_nodes_by_level(level, None, None)?;
            for node in nodes {
                self.index_toc_node(&node).await?;
                stats.nodes_indexed += 1;

                // Progress reporting
                if stats.nodes_indexed % 100 == 0 {
                    info!(count = stats.nodes_indexed, "Rebuild progress");
                }
            }
        }

        // Index all grips
        let grips = self.storage.get_all_grips()?;
        for grip in grips {
            self.index_grip(&grip).await?;
            stats.grips_indexed += 1;
        }

        // Commit indexes
        if let Some(ref bm25) = self.bm25_indexer {
            bm25.commit()?;
        }
        if let Some(ref vector) = self.vector_indexer {
            vector.commit()?;
        }

        // Reset checkpoint to current position
        self.reset_checkpoint()?;

        info!(stats = ?stats, "Full rebuild complete");
        Ok(stats)
    }
}
```

### Pattern 5: Scheduler Integration
**What:** Register indexing job with scheduler using existing patterns
**When to use:** For periodic background processing
**Example:**
```rust
// Source: Derived from memory-scheduler/jobs/rollup.rs
pub async fn create_indexing_job(
    scheduler: &SchedulerService,
    pipeline: Arc<IndexingPipeline>,
    config: IndexingJobConfig,
) -> Result<(), SchedulerError> {
    scheduler.register_job(
        "index_outbox_consumer",
        &config.cron,           // e.g., "*/30 * * * * *" (every 30 seconds)
        Some(&config.timezone),
        OverlapPolicy::Skip,    // Don't overlap - one consumer at a time
        JitterConfig::new(config.jitter_secs),
        move || {
            let pipeline = pipeline.clone();
            async move {
                match pipeline.process_batch().await {
                    Ok(stats) => {
                        if stats.total() > 0 {
                            info!(stats = ?stats, "Indexing batch complete");
                        }
                        Ok(())
                    }
                    Err(e) => {
                        warn!(error = %e, "Indexing batch failed");
                        Err(e.to_string())
                    }
                }
            }
        },
    ).await
}
```

### Anti-Patterns to Avoid
- **Processing outbox synchronously during ingestion:** Never block event ingestion on indexing. Always async via background job.
- **Single checkpoint for multiple indexes:** If one index fails, the other shouldn't re-process. Use per-index checkpoints.
- **Committing after every document:** Batch commits for efficiency. Commit after batch or periodically.
- **No idempotency:** Must handle duplicate processing due to at-least-once semantics.
- **Blocking the async runtime:** Use `spawn_blocking` for Tantivy operations (they're synchronous).

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Outbox reading | Custom iterator | Existing storage.get_outbox_entries() | Already implemented with FIFO semantics |
| Checkpoint persistence | File-based | storage.put_checkpoint()/get_checkpoint() | Already in CF_CHECKPOINTS |
| Cron scheduling | Custom timer | memory-scheduler | Has overlap policy, jitter, timezone |
| BM25 indexing | Custom full-text | memory-search/SearchIndexer | Phase 11 provides complete solution |
| Vector indexing | Custom embedding | memory-vector/VectorIndexPipeline | Phase 12 handles embedding + HNSW |
| Batch iteration | Manual loop | Iterator with batch_size | Rust iterators compose well |

**Key insight:** The existing codebase already has 80% of the infrastructure. Phase 13 is primarily orchestration code that connects existing components.

## Common Pitfalls

### Pitfall 1: Outbox Entries Not Being Deleted
**What goes wrong:** Outbox grows unboundedly
**Why it happens:** Forgetting to delete processed entries or relying on FIFO compaction alone
**How to avoid:** Explicitly delete entries after successful indexing, or mark as processed
**Warning signs:** CF_OUTBOX entry count in stats grows continuously

### Pitfall 2: Checkpoint Saved Before Index Commit
**What goes wrong:** Crash loses indexed documents
**Why it happens:** Checkpoint saved before Tantivy/HNSW commit completes
**How to avoid:** Save checkpoint AFTER index commit succeeds
**Warning signs:** After restart, search returns fewer results than expected

### Pitfall 3: Blocking Async Runtime with Tantivy
**What goes wrong:** gRPC requests timeout during indexing
**Why it happens:** Tantivy operations are synchronous, block tokio threads
**How to avoid:** Use `tokio::task::spawn_blocking` for all Tantivy calls
**Warning signs:** High latency on unrelated gRPC calls during indexing bursts

### Pitfall 4: No Progress on Persistent Failure
**What goes wrong:** Same entry fails repeatedly, pipeline stuck
**Why it happens:** Entry causes error, checkpoint not advanced, retry forever
**How to avoid:** Implement dead-letter handling or skip-after-N-retries
**Warning signs:** Same error message in logs repeatedly

### Pitfall 5: Index Drift from Storage
**What goes wrong:** Index contains stale or missing data
**Why it happens:** Outbox entries lost, index not rebuilt after issue
**How to avoid:** Periodic consistency check, admin rebuild command
**Warning signs:** Search returns results for deleted items or misses recent items

## Code Examples

Verified patterns from official sources and existing codebase:

### Outbox Entry Reading (Add to Storage)
```rust
// Source: Derived from existing memory-storage/db.rs patterns
impl Storage {
    /// Get outbox entries starting from a sequence number.
    /// Returns up to `limit` entries, ordered by sequence.
    pub fn get_outbox_entries(
        &self,
        start_sequence: u64,
        limit: usize,
    ) -> Result<Vec<(u64, OutboxEntry)>, StorageError> {
        let outbox_cf = self.db.cf_handle(CF_OUTBOX)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_OUTBOX.to_string()))?;

        let start_key = OutboxKey::new(start_sequence);
        let mut results = Vec::with_capacity(limit);

        let iter = self.db.iterator_cf(
            &outbox_cf,
            IteratorMode::From(&start_key.to_bytes(), Direction::Forward),
        );

        for item in iter.take(limit) {
            let (key, value) = item?;
            let outbox_key = OutboxKey::from_bytes(&key)?;
            let entry = OutboxEntry::from_bytes(&value)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            results.push((outbox_key.sequence, entry));
        }

        Ok(results)
    }

    /// Delete outbox entries up to and including the given sequence.
    pub fn delete_outbox_entries(&self, up_to_sequence: u64) -> Result<usize, StorageError> {
        let outbox_cf = self.db.cf_handle(CF_OUTBOX)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_OUTBOX.to_string()))?;

        let mut count = 0;
        let mut batch = WriteBatch::default();

        let iter = self.db.iterator_cf(&outbox_cf, IteratorMode::Start);
        for item in iter {
            let (key, _) = item?;
            let outbox_key = OutboxKey::from_bytes(&key)?;
            if outbox_key.sequence > up_to_sequence {
                break;
            }
            batch.delete_cf(&outbox_cf, &key);
            count += 1;
        }

        self.db.write(batch)?;
        Ok(count)
    }
}
```

### Extended OutboxEntry for Index Types
```rust
// Source: Derived from memory-types/outbox.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxAction {
    /// Index this event for BM25/vector search (existing)
    IndexEvent,
    /// Update TOC node with new event (existing)
    UpdateToc,
    /// Index a newly created TOC node
    IndexTocNode { node_id: String },
    /// Index a newly created grip
    IndexGrip { grip_id: String },
}

impl OutboxEntry {
    /// Create entry for TOC node indexing
    pub fn for_toc_node(event_id: String, timestamp_ms: i64, node_id: String) -> Self {
        Self {
            event_id,
            timestamp_ms,
            action: OutboxAction::IndexTocNode { node_id },
        }
    }

    /// Create entry for grip indexing
    pub fn for_grip(event_id: String, timestamp_ms: i64, grip_id: String) -> Self {
        Self {
            event_id,
            timestamp_ms,
            action: OutboxAction::IndexGrip { grip_id },
        }
    }
}
```

### IndexingJobConfig
```rust
// Source: Derived from memory-scheduler/jobs/rollup.rs RollupJobConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingJobConfig {
    /// Cron expression (default: "*/30 * * * * *" = every 30 seconds)
    pub cron: String,

    /// Timezone for scheduling (default: "UTC")
    pub timezone: String,

    /// Max jitter in seconds (default: 5)
    pub jitter_secs: u64,

    /// Batch size per run (default: 100)
    pub batch_size: usize,

    /// Whether to enable BM25 indexing
    pub enable_bm25: bool,

    /// Whether to enable vector indexing
    pub enable_vector: bool,
}

impl Default for IndexingJobConfig {
    fn default() -> Self {
        Self {
            cron: "*/30 * * * * *".to_string(),
            timezone: "UTC".to_string(),
            jitter_secs: 5,
            batch_size: 100,
            enable_bm25: true,
            enable_vector: true,
        }
    }
}
```

### Admin Rebuild Command Pattern
```rust
// Source: Derived from memory-daemon/src/commands.rs admin patterns
#[derive(Parser)]
pub struct RebuildIndexesCmd {
    /// Only rebuild BM25 index
    #[arg(long)]
    bm25_only: bool,

    /// Only rebuild vector index
    #[arg(long)]
    vector_only: bool,

    /// Show what would be rebuilt without rebuilding
    #[arg(long)]
    dry_run: bool,
}

pub async fn rebuild_indexes(
    storage: Arc<Storage>,
    bm25: Option<Arc<SearchIndexer>>,
    vector: Option<Arc<VectorIndexPipeline>>,
    cmd: RebuildIndexesCmd,
) -> Result<(), MemoryError> {
    let stats = storage.get_stats()?;

    if cmd.dry_run {
        println!("Would rebuild indexes:");
        println!("  TOC nodes: {}", stats.toc_node_count);
        println!("  Grips: {}", stats.grip_count);
        if !cmd.vector_only {
            println!("  BM25: {}", if bm25.is_some() { "enabled" } else { "disabled" });
        }
        if !cmd.bm25_only {
            println!("  Vector: {}", if vector.is_some() { "enabled" } else { "disabled" });
        }
        return Ok(());
    }

    let pipeline = IndexingPipeline::new(
        storage,
        if cmd.vector_only { None } else { bm25 },
        if cmd.bm25_only { None } else { vector },
    );

    println!("Starting index rebuild...");
    let rebuild_stats = pipeline.rebuild_all().await?;

    println!("Rebuild complete:");
    println!("  TOC nodes indexed: {}", rebuild_stats.nodes_indexed);
    println!("  Grips indexed: {}", rebuild_stats.grips_indexed);
    println!("  Duration: {:?}", rebuild_stats.duration);

    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Synchronous indexing | Async via outbox | Phase 11+ design | Non-blocking ingestion |
| Full rebuild only | Incremental + rebuild | This phase | Efficient updates |
| Single index | Multi-index dispatch | Phase 11+12 | BM25 + Vector combined |

**Deprecated/outdated:**
- Direct indexing during event ingestion: Use outbox pattern instead
- Shared checkpoint for all indexes: Use per-index checkpoints

## Open Questions

Things that couldn't be fully resolved:

1. **Outbox entry cleanup timing**
   - What we know: Entries should be deleted after successful indexing
   - What's unclear: Should delete happen immediately or in batch? What about FIFO compaction interaction?
   - Recommendation: Delete in batch after each consumer run; FIFO compaction handles any stragglers

2. **Dead-letter handling for persistent failures**
   - What we know: Some entries may fail repeatedly (e.g., invalid data)
   - What's unclear: Should there be a dead-letter queue or just skip after N retries?
   - Recommendation: Skip after 3 retries, log error, advance checkpoint. User can rebuild if needed.

3. **Consistency verification**
   - What we know: Index can drift from storage due to bugs or partial failures
   - What's unclear: How often to check? What metrics to track?
   - Recommendation: Expose IndexStatus RPC with document counts; let admin compare with storage stats

## Sources

### Primary (HIGH confidence)
- [Existing memory-storage/db.rs](crates/memory-storage/src/db.rs) - Outbox and checkpoint APIs
- [Existing memory-types/outbox.rs](crates/memory-types/src/outbox.rs) - OutboxEntry type
- [Existing memory-scheduler](crates/memory-scheduler/src/) - Job scheduling patterns
- [Phase 11 Research](../11-bm25-teleport-tantivy/11-RESEARCH.md) - Tantivy patterns
- [Phase 12 Research](../12-vector-teleport-hnsw/12-RESEARCH.md) - Vector indexing patterns
- [Tantivy docs.rs](https://docs.rs/tantivy/latest/tantivy/) - IndexWriter, commit, delete_term
- [USearch Rust README](https://github.com/unum-cloud/usearch/blob/main/rust/README.md) - add, save, load

### Secondary (MEDIUM confidence)
- [Transactional Outbox Pattern](https://microservices.io/patterns/data/transactional-outbox.html) - Pattern fundamentals
- [Outbox Patterns Explained](https://event-driven.io/en/outbox_inbox_patterns_and_delivery_guarantees_explained/) - At-least-once semantics
- [tokio-cron-scheduler](https://github.com/mvniekerk/tokio-cron-scheduler) - Cron job patterns

### Tertiary (LOW confidence)
- Web search results on idempotent processing - General patterns, not Rust-specific

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All components exist in workspace
- Architecture: HIGH - Derived from existing patterns in codebase
- Pitfalls: MEDIUM - Based on general distributed systems knowledge
- Code examples: HIGH - Derived from existing codebase patterns

**Research date:** 2026-02-01
**Valid until:** 2026-03-01 (30 days - stable domain, existing infrastructure)

---

## Recommended Plan Breakdown

Based on this research, Phase 13 should be split into 4 plans:

### Plan 13-01: Outbox Consumer Infrastructure
**Focus:** Create `memory-indexing` crate, outbox reading, checkpoint tracking
**Tasks:**
- Add memory-indexing crate to workspace
- Implement Storage::get_outbox_entries() and delete_outbox_entries()
- Create IndexCheckpoint type with per-index tracking
- Implement checkpoint load/save using existing CF_CHECKPOINTS
- Unit tests for outbox reading and checkpoint persistence

### Plan 13-02: Incremental Index Updates
**Focus:** IndexingPipeline for dispatching to BM25 and vector indexers
**Tasks:**
- Implement IndexingPipeline with bm25/vector indexer injection
- Add index_toc_node() and index_grip() with idempotent update pattern
- Implement process_batch() for outbox consumption
- Add spawn_blocking wrapper for Tantivy operations
- Integration tests with mock indexers

### Plan 13-03: Full Rebuild Command
**Focus:** Admin command for complete index rebuild from storage
**Tasks:**
- Implement rebuild_all() method in IndexingPipeline
- Add RebuildIndexesCmd to CLI (rebuild-indexes subcommand)
- Support --bm25-only, --vector-only, --dry-run flags
- Progress reporting during rebuild
- Integration test for rebuild functionality

### Plan 13-04: Scheduler Integration
**Focus:** Background job for periodic outbox processing
**Tasks:**
- Create IndexingJobConfig for job configuration
- Implement create_indexing_job() following rollup job pattern
- Wire indexing job into daemon startup
- Add GetIndexingStatus RPC for observability
- End-to-end tests verifying async indexing flow
- Update documentation
