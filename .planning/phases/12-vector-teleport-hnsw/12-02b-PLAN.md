---
phase: 12-vector-teleport-hnsw
plan: 02b
type: execute
wave: 3
depends_on: [12-01, 12-02]
files_modified:
  - crates/memory-vector/src/pipeline.rs
  - crates/memory-vector/src/lib.rs
  - crates/memory-storage/src/checkpoint.rs
  - crates/memory-scheduler/src/jobs/vector_index.rs
  - crates/memory-daemon/src/commands/admin.rs
autonomous: true

must_haves:
  truths:
    - "New TOC nodes and grips are automatically indexed via outbox consumer"
    - "Vector indexing checkpoints survive crash and restart"
    - "Admin rebuild-vectors command rebuilds entire index from scratch"
    - "Admin prune-vectors command removes old vectors without data loss"
  artifacts:
    - path: "crates/memory-vector/src/pipeline.rs"
      provides: "Outbox-driven vector indexing pipeline"
      exports: ["VectorIndexPipeline", "IndexingJob"]
    - path: "crates/memory-scheduler/src/jobs/vector_index.rs"
      provides: "Scheduled vector indexing job"
      exports: ["VectorIndexJob"]
    - path: "crates/memory-daemon/src/commands/admin.rs"
      provides: "Admin commands for vector lifecycle"
      contains: "prune_vectors"
  key_links:
    - from: "crates/memory-vector/src/pipeline.rs"
      to: "memory-embeddings::CandleEmbedder"
      via: "generates embeddings for text"
      pattern: "embedder.embed(text)"
    - from: "crates/memory-vector/src/pipeline.rs"
      to: "memory-storage::Outbox"
      via: "consumes outbox entries"
      pattern: "outbox.consume_pending()"
    - from: "crates/memory-vector/src/pipeline.rs"
      to: "memory-storage::Checkpoint"
      via: "tracks indexing progress"
      pattern: "checkpoint.set(VECTOR_INDEX_CHECKPOINT)"
---

<objective>
Implement outbox-driven vector indexing pipeline with checkpoint-based crash recovery.

Purpose: Enable automatic vector indexing of new TOC nodes and grips as they are created. The pipeline consumes outbox entries, generates embeddings, adds to HNSW index, and updates checkpoint. This ensures vectors are indexed incrementally and crash recovery works correctly.

Output: VectorIndexPipeline that reads outbox entries, embeds text, indexes vectors, and maintains checkpoint. Admin commands for rebuild and prune operations.
</objective>

<execution_context>
@/Users/richardhightower/.claude/get-shit-done/workflows/execute-plan.md
@/Users/richardhightower/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/12-vector-teleport-hnsw/12-RESEARCH.md
@.planning/phases/12-vector-teleport-hnsw/12-01-SUMMARY.md
@.planning/phases/12-vector-teleport-hnsw/12-02-SUMMARY.md
@crates/memory-storage/src/outbox.rs
@crates/memory-storage/src/checkpoint.rs
@crates/memory-embeddings/src/model.rs
@crates/memory-vector/src/hnsw.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create vector indexing pipeline</name>
  <files>
    crates/memory-vector/src/pipeline.rs
    crates/memory-vector/src/lib.rs
  </files>
  <action>
Create the outbox-driven vector indexing pipeline:

1. Create `crates/memory-vector/src/pipeline.rs`:
   ```rust
   //! Outbox-driven vector indexing pipeline.
   //!
   //! Consumes TOC_NODE_CREATED and GRIP_CREATED outbox entries,
   //! generates embeddings, and adds to HNSW index with checkpoint tracking.
   //!
   //! Requirements: FR-09 (Outbox-driven indexing), FR-10 (Checkpoint-based recovery)

   use std::sync::Arc;

   use tracing::{debug, info, warn, error};

   use memory_embeddings::{EmbeddingModel, EmbeddingError};
   use memory_storage::{Storage, OutboxEntry, OutboxEntryType};
   use memory_types::TocNode;

   use crate::error::VectorError;
   use crate::hnsw::HnswIndex;
   use crate::metadata::{VectorMetadata, VectorEntry, DocType};
   use crate::index::VectorIndex;

   /// Checkpoint key for vector indexing
   pub const VECTOR_INDEX_CHECKPOINT: &str = "vector_index_last_processed";

   /// Statistics from indexing run
   #[derive(Debug, Default, Clone)]
   pub struct IndexingStats {
       pub entries_processed: usize,
       pub vectors_added: usize,
       pub vectors_skipped: usize,
       pub errors: usize,
   }

   /// Vector indexing pipeline configuration
   #[derive(Debug, Clone)]
   pub struct PipelineConfig {
       /// Batch size for processing outbox entries
       pub batch_size: usize,
       /// Maximum entries to process per run (0 = unlimited)
       pub max_entries_per_run: usize,
       /// Whether to continue on individual entry errors
       pub continue_on_error: bool,
   }

   impl Default for PipelineConfig {
       fn default() -> Self {
           Self {
               batch_size: 32,
               max_entries_per_run: 1000,
               continue_on_error: true,
           }
       }
   }

   /// Vector indexing pipeline.
   ///
   /// Reads outbox entries, generates embeddings, and indexes vectors.
   pub struct VectorIndexPipeline<E: EmbeddingModel> {
       storage: Arc<Storage>,
       embedder: Arc<E>,
       index: Arc<parking_lot::RwLock<HnswIndex>>,
       metadata: Arc<VectorMetadata>,
       config: PipelineConfig,
   }

   impl<E: EmbeddingModel> VectorIndexPipeline<E> {
       /// Create a new pipeline.
       pub fn new(
           storage: Arc<Storage>,
           embedder: Arc<E>,
           index: Arc<parking_lot::RwLock<HnswIndex>>,
           metadata: Arc<VectorMetadata>,
           config: PipelineConfig,
       ) -> Self {
           Self {
               storage,
               embedder,
               index,
               metadata,
               config,
           }
       }

       /// Run the indexing pipeline.
       ///
       /// Processes pending outbox entries from the last checkpoint.
       pub async fn run(&self) -> Result<IndexingStats, VectorError> {
           let mut stats = IndexingStats::default();

           // Get last checkpoint
           let last_processed = self.storage
               .get_checkpoint(VECTOR_INDEX_CHECKPOINT)?
               .unwrap_or_default();

           info!(checkpoint = %last_processed, "Starting vector indexing from checkpoint");

           // Get pending entries after checkpoint
           let entries = self.storage.get_outbox_entries_after(
               &last_processed,
               self.config.max_entries_per_run,
           )?;

           if entries.is_empty() {
               debug!("No pending entries to index");
               return Ok(stats);
           }

           info!(count = entries.len(), "Processing outbox entries");

           // Process in batches
           for batch in entries.chunks(self.config.batch_size) {
               match self.process_batch(batch).await {
                   Ok(batch_stats) => {
                       stats.entries_processed += batch_stats.entries_processed;
                       stats.vectors_added += batch_stats.vectors_added;
                       stats.vectors_skipped += batch_stats.vectors_skipped;
                       stats.errors += batch_stats.errors;
                   }
                   Err(e) => {
                       error!(error = %e, "Batch processing failed");
                       if !self.config.continue_on_error {
                           return Err(e);
                       }
                       stats.errors += batch.len();
                   }
               }

               // Update checkpoint after each batch
               if let Some(last) = batch.last() {
                   self.storage.set_checkpoint(
                       VECTOR_INDEX_CHECKPOINT,
                       &last.entry_id,
                   )?;
               }
           }

           // Save index
           self.index.write().save()?;

           info!(
               processed = stats.entries_processed,
               added = stats.vectors_added,
               skipped = stats.vectors_skipped,
               errors = stats.errors,
               "Vector indexing complete"
           );

           Ok(stats)
       }

       /// Process a batch of outbox entries.
       async fn process_batch(&self, entries: &[OutboxEntry]) -> Result<IndexingStats, VectorError> {
           let mut stats = IndexingStats::default();

           for entry in entries {
               stats.entries_processed += 1;

               match self.process_entry(entry).await {
                   Ok(true) => stats.vectors_added += 1,
                   Ok(false) => stats.vectors_skipped += 1,
                   Err(e) => {
                       warn!(entry_id = %entry.entry_id, error = %e, "Failed to process entry");
                       if self.config.continue_on_error {
                           stats.errors += 1;
                       } else {
                           return Err(e);
                       }
                   }
               }
           }

           Ok(stats)
       }

       /// Process a single outbox entry.
       ///
       /// Returns true if vector was added, false if skipped.
       async fn process_entry(&self, entry: &OutboxEntry) -> Result<bool, VectorError> {
           // Only process TOC and grip entries
           let (doc_type, doc_id, text) = match &entry.entry_type {
               OutboxEntryType::TocNodeCreated { node_id } => {
                   let node = self.storage.get_toc_node(node_id)?
                       .ok_or_else(|| VectorError::NotFound(0))?;
                   let text = self.extract_node_text(&node);
                   (DocType::TocNode, node_id.clone(), text)
               }
               OutboxEntryType::GripCreated { grip_id } => {
                   let grip = self.storage.get_grip(grip_id)?
                       .ok_or_else(|| VectorError::NotFound(0))?;
                   (DocType::Grip, grip_id.clone(), grip.excerpt.clone())
               }
               _ => return Ok(false), // Skip other entry types
           };

           // Skip if already indexed
           if self.metadata.find_by_doc_id(&doc_id)?.is_some() {
               debug!(doc_id = %doc_id, "Already indexed, skipping");
               return Ok(false);
           }

           // Skip empty text
           if text.trim().is_empty() {
               debug!(doc_id = %doc_id, "Empty text, skipping");
               return Ok(false);
           }

           // Generate embedding
           let embedding = self.embedder.embed(&text)?;

           // Get next vector ID
           let vector_id = self.metadata.next_vector_id()?;

           // Add to index
           self.index.write().add(vector_id, &embedding)?;

           // Store metadata
           let meta_entry = VectorEntry::new(
               vector_id,
               doc_type,
               doc_id.clone(),
               entry.created_at,
               &text,
           );
           self.metadata.put(&meta_entry)?;

           debug!(vector_id = vector_id, doc_id = %doc_id, "Indexed vector");
           Ok(true)
       }

       /// Extract searchable text from a TOC node.
       fn extract_node_text(&self, node: &TocNode) -> String {
           let mut parts = Vec::new();

           // Include title
           if !node.title.is_empty() {
               parts.push(node.title.clone());
           }

           // Include bullets
           for bullet in &node.bullets {
               parts.push(bullet.text.clone());
           }

           // Include keywords
           if !node.keywords.is_empty() {
               parts.push(node.keywords.join(" "));
           }

           parts.join(". ")
       }

       /// Rebuild entire vector index from scratch.
       ///
       /// Clears existing index and re-indexes all TOC nodes and grips.
       pub async fn rebuild(&self) -> Result<IndexingStats, VectorError> {
           info!("Starting full vector index rebuild");

           // Clear index and metadata
           self.index.write().clear()?;
           // Note: VectorMetadata doesn't have clear(), would need to add

           // Reset checkpoint
           self.storage.set_checkpoint(VECTOR_INDEX_CHECKPOINT, "")?;

           // Re-run from beginning
           self.run().await
       }

       /// Prune old vectors based on age.
       ///
       /// Removes vectors older than age_days from the HNSW index.
       /// Does NOT delete primary data (TOC nodes, grips remain in RocksDB).
       pub async fn prune(&self, age_days: u64) -> Result<usize, VectorError> {
           let cutoff_ms = chrono::Utc::now().timestamp_millis()
               - (age_days as i64 * 24 * 60 * 60 * 1000);

           info!(age_days = age_days, cutoff_ms = cutoff_ms, "Pruning old vectors");

           let all_entries = self.metadata.get_all()?;
           let mut pruned = 0;

           for entry in all_entries {
               if entry.created_at < cutoff_ms {
                   // Remove from HNSW index
                   self.index.write().remove(entry.vector_id)?;
                   // Remove metadata
                   self.metadata.delete(entry.vector_id)?;
                   pruned += 1;
               }
           }

           if pruned > 0 {
               self.index.write().save()?;
           }

           info!(pruned = pruned, "Prune complete");
           Ok(pruned)
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       // Integration tests would require full storage + embedder setup
       // See tests/integration/vector_pipeline_test.rs
   }
   ```

2. Update `crates/memory-vector/src/lib.rs` to add:
   ```rust
   pub mod pipeline;
   pub use pipeline::{VectorIndexPipeline, PipelineConfig, IndexingStats, VECTOR_INDEX_CHECKPOINT};
   ```

3. Add `get_all` method to `VectorMetadata` in `crates/memory-vector/src/metadata.rs`:
   ```rust
   /// Get all entries.
   pub fn get_all(&self) -> Result<Vec<VectorEntry>, VectorError> {
       let mut entries = Vec::new();
       let iter = self.db.iterator_cf(self.cf(), rocksdb::IteratorMode::Start);

       for item in iter {
           let (_, value) = item?;
           let entry: VectorEntry = serde_json::from_slice(&value)
               .map_err(|e| VectorError::Serialization(e.to_string()))?;
           entries.push(entry);
       }

       Ok(entries)
   }
   ```
  </action>
  <verify>
    cd /Users/richardhightower/clients/spillwave/src/agent-memory && cargo check -p memory-vector
  </verify>
  <done>
    VectorIndexPipeline created with run(), rebuild(), and prune() methods. Outbox consumption and checkpoint tracking implemented.
  </done>
</task>

<task type="auto">
  <name>Task 2: Create scheduled vector indexing job</name>
  <files>
    crates/memory-scheduler/src/jobs/vector_index.rs
    crates/memory-scheduler/src/jobs/mod.rs
  </files>
  <action>
Wire vector indexing to the scheduler:

1. Create `crates/memory-scheduler/src/jobs/vector_index.rs`:
   ```rust
   //! Scheduled vector indexing job.
   //!
   //! Runs periodically to index new TOC nodes and grips.
   //! Uses checkpoint to resume from last position after restart.

   use std::sync::Arc;

   use parking_lot::RwLock;
   use tracing::{info, error};

   use memory_embeddings::CandleEmbedder;
   use memory_storage::Storage;
   use memory_vector::{
       HnswIndex, VectorMetadata, VectorIndexPipeline, PipelineConfig,
   };

   use crate::job::{Job, JobResult};

   /// Vector indexing job configuration
   #[derive(Debug, Clone)]
   pub struct VectorIndexJobConfig {
       /// Cron schedule (default: every 5 minutes)
       pub schedule: String,
       /// Pipeline configuration
       pub pipeline: PipelineConfig,
   }

   impl Default for VectorIndexJobConfig {
       fn default() -> Self {
           Self {
               schedule: "0 */5 * * * *".to_string(), // Every 5 minutes
               pipeline: PipelineConfig::default(),
           }
       }
   }

   /// Vector indexing scheduled job.
   pub struct VectorIndexJob {
       pipeline: VectorIndexPipeline<CandleEmbedder>,
   }

   impl VectorIndexJob {
       /// Create a new vector indexing job.
       pub fn new(
           storage: Arc<Storage>,
           embedder: Arc<CandleEmbedder>,
           index: Arc<RwLock<HnswIndex>>,
           metadata: Arc<VectorMetadata>,
           config: PipelineConfig,
       ) -> Self {
           let pipeline = VectorIndexPipeline::new(
               storage,
               embedder,
               index,
               metadata,
               config,
           );
           Self { pipeline }
       }
   }

   #[async_trait::async_trait]
   impl Job for VectorIndexJob {
       fn name(&self) -> &str {
           "vector_indexing"
       }

       async fn run(&self) -> JobResult {
           info!("Running vector indexing job");

           match self.pipeline.run().await {
               Ok(stats) => {
                   info!(
                       added = stats.vectors_added,
                       skipped = stats.vectors_skipped,
                       errors = stats.errors,
                       "Vector indexing job complete"
                   );
                   if stats.errors > 0 {
                       JobResult::PartialSuccess {
                           message: format!(
                               "Indexed {} vectors with {} errors",
                               stats.vectors_added, stats.errors
                           ),
                       }
                   } else {
                       JobResult::Success
                   }
               }
               Err(e) => {
                   error!(error = %e, "Vector indexing job failed");
                   JobResult::Failed {
                       message: e.to_string(),
                       retryable: true,
                   }
               }
           }
       }
   }
   ```

2. Update `crates/memory-scheduler/src/jobs/mod.rs` to add:
   ```rust
   pub mod vector_index;
   pub use vector_index::{VectorIndexJob, VectorIndexJobConfig};
   ```
  </action>
  <verify>
    cd /Users/richardhightower/clients/spillwave/src/agent-memory && cargo check -p memory-scheduler
  </verify>
  <done>
    VectorIndexJob created and wired to scheduler. Job runs pipeline.run() on schedule.
  </done>
</task>

<task type="auto">
  <name>Task 3: Add admin prune and rebuild commands</name>
  <files>
    crates/memory-daemon/src/commands/admin.rs
  </files>
  <action>
Add admin commands for vector lifecycle management:

1. Update CLI to add admin subcommands in `crates/memory-daemon/src/cli.rs`:
   ```rust
   #[derive(Subcommand, Debug)]
   pub enum AdminCommand {
       /// Rebuild TOC hierarchy from events
       RebuildToc { /* existing */ },

       /// Compact RocksDB storage
       Compact { /* existing */ },

       /// Show daemon status
       Status { /* existing */ },

       /// Prune old vectors from HNSW index (data remains in RocksDB)
       PruneVectors {
           /// Age threshold in days
           #[arg(long, default_value = "365")]
           age_days: u64,
           /// gRPC server address
           #[arg(long, default_value = "http://127.0.0.1:50051")]
           addr: String,
       },

       /// Rebuild entire vector index from scratch
       RebuildVectors {
           /// gRPC server address
           #[arg(long, default_value = "http://127.0.0.1:50051")]
           addr: String,
       },
   }
   ```

2. Add to `crates/memory-daemon/src/commands/admin.rs`:
   ```rust
   pub async fn prune_vectors(addr: &str, age_days: u64) -> Result<()> {
       println!("Pruning vectors older than {} days...", age_days);
       println!("NOTE: This removes vectors from HNSW index only.");
       println!("      Primary data (TOC nodes, grips) remains in RocksDB.");

       let mut client = MemoryClient::connect(addr).await?;
       let response = client.prune_vectors(age_days).await?;

       println!("\nPrune complete:");
       println!("  Vectors removed: {}", response.pruned_count);
       println!("  Index size after: {} bytes", response.index_size_bytes);

       Ok(())
   }

   pub async fn rebuild_vectors(addr: &str) -> Result<()> {
       println!("Rebuilding vector index...");
       println!("This will clear the index and re-embed all TOC nodes and grips.");

       let mut client = MemoryClient::connect(addr).await?;
       let response = client.rebuild_vectors().await?;

       println!("\nRebuild complete:");
       println!("  Vectors indexed: {}", response.vectors_indexed);
       println!("  Errors: {}", response.errors);
       println!("  Duration: {:?}", response.duration_ms);

       Ok(())
   }
   ```

3. Add gRPC RPCs to proto/memory.proto:
   ```protobuf
   // Vector lifecycle RPCs
   rpc PruneVectors(PruneVectorsRequest) returns (PruneVectorsResponse);
   rpc RebuildVectors(RebuildVectorsRequest) returns (RebuildVectorsResponse);

   message PruneVectorsRequest {
     uint64 age_days = 1;
   }

   message PruneVectorsResponse {
     uint64 pruned_count = 1;
     uint64 index_size_bytes = 2;
   }

   message RebuildVectorsRequest {}

   message RebuildVectorsResponse {
     uint64 vectors_indexed = 1;
     uint64 errors = 2;
     uint64 duration_ms = 3;
   }
   ```

4. Implement gRPC handlers in memory-service.
  </action>
  <verify>
    cd /Users/richardhightower/clients/spillwave/src/agent-memory && cargo build -p memory-daemon && ./target/debug/memory-daemon admin --help
  </verify>
  <done>
    Admin prune-vectors and rebuild-vectors commands added. gRPC RPCs defined and implemented.
  </done>
</task>

</tasks>

<verification>
```bash
cd /Users/richardhightower/clients/spillwave/src/agent-memory

# Pipeline compiles
cargo check -p memory-vector

# Scheduler job compiles
cargo check -p memory-scheduler

# Daemon builds with admin commands
cargo build -p memory-daemon

# Admin help shows vector commands
./target/debug/memory-daemon admin --help

# Clippy clean
cargo clippy -p memory-vector -p memory-scheduler -p memory-daemon -- -D warnings
```
</verification>

<success_criteria>
- [ ] VectorIndexPipeline consumes outbox entries
- [ ] Pipeline generates embeddings and adds to HNSW index
- [ ] Checkpoint tracking survives restart (VECTOR_INDEX_CHECKPOINT)
- [ ] VectorIndexJob runs on scheduler (every 5 minutes default)
- [ ] admin prune-vectors removes old vectors (not primary data)
- [ ] admin rebuild-vectors clears and rebuilds index
- [ ] gRPC PruneVectors and RebuildVectors RPCs implemented
- [ ] All tests pass
- [ ] No clippy warnings
</success_criteria>

<output>
After completion, create `.planning/phases/12-vector-teleport-hnsw/12-02b-SUMMARY.md`
</output>
