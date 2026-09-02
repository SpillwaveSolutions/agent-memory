//! Indexing pipeline for processing outbox entries.
//!
//! Coordinates multiple index updaters, manages checkpoints,
//! and handles batch processing with crash recovery.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{debug, info, warn};

use memory_storage::{EventKey, Storage};

use memory_types::OutboxEntry;

use crate::checkpoint::{IndexCheckpoint, IndexType};
use crate::error::IndexingError;
use crate::updater::{IndexUpdater, UpdateResult};

/// Result of processing a batch of outbox entries.
#[derive(Debug, Default)]
pub struct ProcessResult {
    /// Results per index type
    pub by_index: HashMap<IndexType, UpdateResult>,
    /// Total entries processed across all indexes
    pub total_processed: usize,
    /// The last sequence number processed
    pub last_sequence: Option<u64>,
    /// Whether all indexes were successfully committed
    pub committed: bool,
}

impl ProcessResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a result for an index type.
    pub fn add_result(&mut self, index_type: IndexType, result: UpdateResult) {
        self.total_processed += result.processed;
        // Sequence 0 is a real outbox id. Using `last_sequence > 0` dropped the
        // first event, so a one-entry drain never wrote a checkpoint.
        if result.total() > 0 {
            self.last_sequence = Some(match self.last_sequence {
                Some(last_seq) => last_seq.max(result.last_sequence),
                None => result.last_sequence,
            });
        }
        self.by_index.insert(index_type, result);
    }

    /// Check if any entries were processed.
    pub fn has_updates(&self) -> bool {
        self.total_processed > 0
    }
}

/// Where a backfill read its work items from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackfillSource {
    /// Remaining outbox entries from the start sequence.
    #[default]
    Outbox,
    /// Event log (outbox empty, or `--from-sequence 0` force re-index).
    EventLog,
}

impl std::fmt::Display for BackfillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackfillSource::Outbox => write!(f, "outbox"),
            BackfillSource::EventLog => write!(f, "event-log"),
        }
    }
}

/// Options for [`IndexingPipeline::backfill`].
#[derive(Debug, Clone)]
pub struct BackfillConfig {
    /// Outbox/synthetic sequence to start from. `None` resumes from the
    /// checkpoint (`last_sequence + 1` when `processed_count > 0`, else 0).
    /// `Some(0)` re-indexes every event from the event log (needed to fill
    /// empty pre-v3.1 `text_preview`s after the outbox has been drained).
    pub from_sequence: Option<u64>,
    /// Entries per commit.
    pub batch_size: usize,
    /// Count work and print; do not index or write checkpoints.
    pub dry_run: bool,
    /// Stop after this many batches. `None` runs to completion. Tests use
    /// `Some(1)` as the equivalent of SIGINT after the first commit.
    pub max_batches: Option<usize>,
}

impl Default for BackfillConfig {
    fn default() -> Self {
        Self {
            from_sequence: None,
            batch_size: 500,
            dry_run: false,
            max_batches: None,
        }
    }
}

impl BackfillConfig {
    /// Resume from checkpoint (or 0 if none).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the start sequence. Pass `Some(0)` to force a full re-index.
    pub fn with_from_sequence(mut self, from: Option<u64>) -> Self {
        self.from_sequence = from;
        self
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Count only; write nothing.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Cap the number of batches (interrupt simulation).
    pub fn with_max_batches(mut self, max_batches: Option<usize>) -> Self {
        self.max_batches = max_batches;
        self
    }
}

/// Result of a backfill run.
#[derive(Debug, Clone, Default)]
pub struct BackfillReport {
    /// Events offered to updaters this run. Zero on a second catch-up run.
    pub documents: usize,
    /// Entries visited (equals `documents` unless dry-run, in which case
    /// this is the count that would have been indexed).
    pub would_index: usize,
    /// Updater-level skips (unused by the mock; reserved for callers).
    pub skipped: usize,
    /// Entries that failed to index (continue-on-error).
    pub errors: usize,
    /// Highest sequence written to the checkpoint. `None` if nothing ran.
    pub last_sequence: Option<u64>,
    /// Outbox vs event-log fallback.
    pub source: BackfillSource,
    /// Whether this run wrote nothing.
    pub dry_run: bool,
}

impl BackfillReport {
    /// True when this run indexed at least one event.
    pub fn has_updates(&self) -> bool {
        self.documents > 0
    }
}

/// Configuration for the indexing pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum entries to process per batch
    pub batch_size: usize,
    /// Whether to continue processing on individual entry errors
    pub continue_on_error: bool,
    /// Whether to commit after each batch
    pub commit_after_batch: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            continue_on_error: true,
            commit_after_batch: true,
        }
    }
}

impl PipelineConfig {
    /// Create a new config with the given batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set whether to continue on errors.
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }

    /// Set whether to commit after each batch.
    pub fn with_commit_after_batch(mut self, commit: bool) -> Self {
        self.commit_after_batch = commit;
        self
    }
}

/// Indexing pipeline that coordinates multiple index updaters.
///
/// Processes outbox entries in sequence order and updates
/// all registered indexes with checkpoint tracking.
pub struct IndexingPipeline {
    storage: Arc<Storage>,
    updaters: Vec<Box<dyn IndexUpdater>>,
    checkpoints: HashMap<IndexType, IndexCheckpoint>,
    config: PipelineConfig,
}

impl IndexingPipeline {
    /// Create a new indexing pipeline.
    pub fn new(storage: Arc<Storage>, config: PipelineConfig) -> Self {
        Self {
            storage,
            updaters: Vec::new(),
            checkpoints: HashMap::new(),
            config,
        }
    }

    /// Add an index updater to the pipeline.
    pub fn add_updater(&mut self, updater: Box<dyn IndexUpdater>) {
        let index_type = updater.index_type();
        self.updaters.push(updater);

        // Initialize checkpoint if not already loaded
        self.checkpoints
            .entry(index_type)
            .or_insert_with(|| IndexCheckpoint::new(index_type));
    }

    /// Load checkpoints from storage.
    pub fn load_checkpoints(&mut self) -> Result<(), IndexingError> {
        for updater in &self.updaters {
            let index_type = updater.index_type();
            let key = index_type.checkpoint_key();

            if let Some(bytes) = self.storage.get_checkpoint(key)? {
                let checkpoint = IndexCheckpoint::from_bytes(&bytes)?;
                info!(
                    index = %updater.name(),
                    last_sequence = checkpoint.last_sequence,
                    "Loaded checkpoint"
                );
                self.checkpoints.insert(index_type, checkpoint);
            } else {
                debug!(index = %updater.name(), "No existing checkpoint, starting from 0");
                self.checkpoints
                    .insert(index_type, IndexCheckpoint::new(index_type));
            }
        }
        Ok(())
    }

    /// Save checkpoints to storage.
    pub fn save_checkpoints(&self) -> Result<(), IndexingError> {
        for (index_type, checkpoint) in &self.checkpoints {
            let key = index_type.checkpoint_key();
            let bytes = checkpoint.to_bytes()?;
            self.storage.put_checkpoint(key, &bytes)?;
            debug!(
                index_type = %index_type,
                last_sequence = checkpoint.last_sequence,
                "Saved checkpoint"
            );
        }
        Ok(())
    }

    /// Get the minimum last_sequence across all checkpoints.
    ///
    /// This is the starting point for the next batch - we need
    /// to process from here to ensure all indexes are caught up.
    fn min_checkpoint_sequence(&self) -> u64 {
        self.checkpoints
            .values()
            .map(|c| c.last_sequence)
            .min()
            .unwrap_or(0)
    }

    /// Process a batch of outbox entries from storage.
    ///
    /// Returns the processing result including per-index stats.
    pub fn process_batch(&mut self, batch_size: usize) -> Result<ProcessResult, IndexingError> {
        let start_sequence = self.min_checkpoint_sequence();
        let limit = batch_size.max(1);

        info!(
            start_sequence = start_sequence,
            limit = limit,
            "Fetching outbox entries"
        );

        let entries = self.storage.get_outbox_entries(start_sequence, limit)?;

        if entries.is_empty() {
            debug!("No outbox entries to process");
            return Ok(ProcessResult::new());
        }

        info!(count = entries.len(), "Processing outbox entries");

        let mut result = ProcessResult::new();

        // Process each updater
        for updater in &self.updaters {
            let index_type = updater.index_type();
            let checkpoint = self
                .checkpoints
                .get(&index_type)
                .cloned()
                .unwrap_or_else(|| IndexCheckpoint::new(index_type));

            // Filter entries this updater hasn't seen yet
            // When processed_count is 0, this is a fresh checkpoint and we should
            // process from the beginning (including sequence 0)
            let new_entries: Vec<_> = entries
                .iter()
                .filter(|(seq, _)| {
                    if checkpoint.processed_count == 0 {
                        true // Fresh checkpoint - process all available entries
                    } else {
                        *seq > checkpoint.last_sequence
                    }
                })
                .cloned()
                .collect();

            if new_entries.is_empty() {
                debug!(index = %updater.name(), "No new entries for this index");
                result.add_result(index_type, UpdateResult::new());
                continue;
            }

            debug!(
                index = %updater.name(),
                count = new_entries.len(),
                "Processing entries"
            );

            let mut update_result = UpdateResult::new();

            for (sequence, entry) in &new_entries {
                match updater.index_document(entry) {
                    Ok(()) => {
                        update_result.record_success();
                    }
                    Err(e) => {
                        warn!(
                            index = %updater.name(),
                            sequence = sequence,
                            event_id = %entry.event_id,
                            error = %e,
                            "Failed to index document"
                        );
                        if self.config.continue_on_error {
                            update_result.record_error();
                        } else {
                            return Err(e);
                        }
                    }
                }
                update_result.set_sequence(*sequence);
            }

            result.add_result(index_type, update_result);
        }

        // Commit if configured
        if self.config.commit_after_batch && result.has_updates() {
            self.commit()?;
            result.committed = true;

            // Update checkpoints after successful commit
            if let Some(last_seq) = result.last_sequence {
                for (index_type, checkpoint) in &mut self.checkpoints {
                    if let Some(idx_result) = result.by_index.get(index_type) {
                        // Sequence 0 is a valid last_sequence. A fresh
                        // checkpoint is last_sequence=0/processed_count=0, so
                        // `>` would skip the first outbox entry forever.
                        if checkpoint.processed_count == 0
                            || idx_result.last_sequence > checkpoint.last_sequence
                        {
                            checkpoint
                                .update(idx_result.last_sequence, idx_result.processed as u64);
                        }
                    }
                }
                self.save_checkpoints()?;

                info!(
                    last_sequence = last_seq,
                    total_processed = result.total_processed,
                    "Batch processing complete"
                );
            }
        }

        Ok(result)
    }

    /// Commit all indexes.
    pub fn commit(&self) -> Result<(), IndexingError> {
        for updater in &self.updaters {
            updater.commit()?;
            debug!(index = %updater.name(), "Committed");
        }
        Ok(())
    }

    /// Process entries until caught up or max iterations reached.
    ///
    /// Returns total processing stats across all batches.
    pub fn process_until_caught_up(
        &mut self,
        max_iterations: usize,
    ) -> Result<ProcessResult, IndexingError> {
        let mut total_result = ProcessResult::new();
        let mut iterations = 0;

        loop {
            if iterations >= max_iterations {
                info!(iterations = iterations, "Reached max iterations");
                break;
            }

            let batch_result = self.process_batch(self.config.batch_size)?;

            if !batch_result.has_updates() && batch_result.last_sequence.is_none() {
                debug!("No more entries to process");
                break;
            }

            // Merge results
            total_result.total_processed += batch_result.total_processed;
            if let Some(seq) = batch_result.last_sequence {
                total_result.last_sequence = Some(seq);
            }
            for (index_type, result) in batch_result.by_index {
                total_result
                    .by_index
                    .entry(index_type)
                    .or_default()
                    .merge(&result);
            }

            iterations += 1;
        }

        total_result.committed = true;
        Ok(total_result)
    }

    /// Clean up processed outbox entries.
    ///
    /// Deletes entries up to the minimum checkpoint sequence.
    /// Only call after confirming all indexes are caught up.
    pub fn cleanup_outbox(&self) -> Result<usize, IndexingError> {
        let min_seq = self.min_checkpoint_sequence();
        if min_seq == 0 {
            return Ok(0);
        }

        // Delete entries that all indexes have processed
        // Use min_seq - 1 to keep the last processed entry for safety
        let up_to = min_seq.saturating_sub(1);
        let deleted = self.storage.delete_outbox_entries(up_to)?;

        info!(
            min_sequence = min_seq,
            deleted = deleted,
            "Cleaned up outbox entries"
        );

        Ok(deleted)
    }

    /// Get the current checkpoint for an index type.
    pub fn get_checkpoint(&self, index_type: IndexType) -> Option<&IndexCheckpoint> {
        self.checkpoints.get(&index_type)
    }

    /// Get all registered updater names.
    pub fn updater_names(&self) -> Vec<&str> {
        self.updaters.iter().map(|u| u.name()).collect()
    }

    /// Get the number of registered updaters.
    pub fn updater_count(&self) -> usize {
        self.updaters.len()
    }

    /// Sequence a default (no `--from-sequence`) backfill would start at.
    ///
    /// Fresh checkpoint (`processed_count == 0`) starts at 0 so sequence 0
    /// is not skipped. A caught-up checkpoint resumes at `last_sequence + 1`.
    pub fn resume_sequence(&self) -> u64 {
        self.checkpoints
            .values()
            .map(|c| {
                if c.processed_count > 0 {
                    c.last_sequence.saturating_add(1)
                } else {
                    0
                }
            })
            .min()
            .unwrap_or(0)
    }

    /// Replay events into the registered indexes.
    ///
    /// Default start resumes from the checkpoint. `--from-sequence 0` (pass
    /// `from_sequence: Some(0)`) re-indexes every event from the event log
    /// so pre-v3.1 empty `text_preview`s can be filled after the outbox has
    /// been cleaned. A second catch-up run reports 0 new documents.
    /// `--dry-run` counts and writes nothing.
    pub fn backfill<F>(
        &mut self,
        config: BackfillConfig,
        mut progress: F,
    ) -> Result<BackfillReport, IndexingError>
    where
        F: FnMut(usize, usize),
    {
        let start = config
            .from_sequence
            .unwrap_or_else(|| self.resume_sequence());
        let batch_size = config.batch_size.max(1);
        let force_event_log = config.from_sequence == Some(0);

        let outbox_probe = self.storage.get_outbox_entries(start, 1)?;
        let use_event_log = force_event_log || (start == 0 && outbox_probe.is_empty());
        let source = if use_event_log {
            BackfillSource::EventLog
        } else {
            BackfillSource::Outbox
        };

        let total = match source {
            BackfillSource::Outbox => self.storage.count_outbox_from(start)? as usize,
            BackfillSource::EventLog => {
                let n = self.storage.get_stats()?.event_count as usize;
                n.saturating_sub(start as usize)
            }
        };

        let mut report = BackfillReport {
            source,
            dry_run: config.dry_run,
            ..Default::default()
        };

        if total == 0 {
            return Ok(report);
        }

        let mut cursor_seq = start;
        let mut event_after: Option<EventKey> = None;
        let mut event_seq: u64 = 0;
        let mut batches = 0usize;

        loop {
            if config.max_batches.is_some_and(|max| batches >= max) {
                info!(batches, "Backfill stopped at max_batches");
                break;
            }

            let batch = match source {
                BackfillSource::Outbox => {
                    self.storage.get_outbox_entries(cursor_seq, batch_size)?
                }
                BackfillSource::EventLog => {
                    self.fetch_event_batch(&mut event_after, &mut event_seq, start, batch_size)?
                }
            };

            if batch.is_empty() {
                break;
            }

            let batch_last = batch.last().map(|(s, _)| *s);
            report.would_index += batch.len();

            if !config.dry_run {
                for updater in &self.updaters {
                    for (sequence, entry) in &batch {
                        match updater.index_document(entry) {
                            Ok(()) => {}
                            Err(e) => {
                                warn!(
                                    index = %updater.name(),
                                    sequence = sequence,
                                    event_id = %entry.event_id,
                                    error = %e,
                                    "Failed to index document during backfill"
                                );
                                if self.config.continue_on_error {
                                    report.errors += 1;
                                } else {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
                report.documents += batch.len();

                self.commit()?;
                if let Some(seq) = batch_last {
                    let n = batch.len() as u64;
                    for checkpoint in self.checkpoints.values_mut() {
                        checkpoint.update(seq, n);
                    }
                    self.save_checkpoints()?;
                    report.last_sequence = Some(seq);
                }
            } else if let Some(seq) = batch_last {
                report.last_sequence = Some(seq);
            }

            if let Some(seq) = batch_last {
                cursor_seq = seq.saturating_add(1);
            }

            batches += 1;
            progress(report.would_index, total);

            if batch.len() < batch_size {
                break;
            }
        }

        // Event-log complete: pin last_sequence to at least outbox_head-1 so
        // a subsequent live drain does not re-process, and default resume
        // reports 0 new.
        if !config.dry_run && source == BackfillSource::EventLog {
            if let Some(last) = report.last_sequence {
                let pin = self.storage.outbox_head().saturating_sub(1).max(last);
                for checkpoint in self.checkpoints.values_mut() {
                    if checkpoint.last_sequence < pin {
                        checkpoint.last_sequence = pin;
                    }
                }
                self.save_checkpoints()?;
                report.last_sequence = Some(pin);
            }
        }

        info!(
            documents = report.documents,
            would_index = report.would_index,
            errors = report.errors,
            source = %report.source,
            dry_run = report.dry_run,
            "Backfill complete"
        );

        Ok(report)
    }

    /// Page events and assign synthetic sequences 0..N-1.
    fn fetch_event_batch(
        &self,
        after: &mut Option<EventKey>,
        next_seq: &mut u64,
        start: u64,
        limit: usize,
    ) -> Result<Vec<(u64, OutboxEntry)>, IndexingError> {
        let mut out = Vec::with_capacity(limit.min(256));
        while out.len() < limit {
            let page = self.storage.get_events_page(after.as_ref(), 256)?;
            if page.is_empty() {
                break;
            }
            for (key, _) in page {
                *after = Some(key.clone());
                let seq = *next_seq;
                *next_seq = next_seq.saturating_add(1);
                if seq < start {
                    continue;
                }
                out.push((
                    seq,
                    OutboxEntry::for_index(key.event_id(), key.timestamp_ms),
                ));
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_types::OutboxEntry;
    use tempfile::TempDir;

    // Mock updater for testing
    struct MockUpdater {
        index_type: IndexType,
        name: &'static str,
        should_fail: bool,
    }

    impl MockUpdater {
        fn new(index_type: IndexType, name: &'static str) -> Self {
            Self {
                index_type,
                name,
                should_fail: false,
            }
        }

        #[allow(dead_code)]
        fn failing(index_type: IndexType, name: &'static str) -> Self {
            Self {
                index_type,
                name,
                should_fail: true,
            }
        }
    }

    impl IndexUpdater for MockUpdater {
        fn index_document(&self, _entry: &OutboxEntry) -> Result<(), IndexingError> {
            if self.should_fail {
                Err(IndexingError::Index("Mock failure".to_string()))
            } else {
                Ok(())
            }
        }

        fn remove_document(&self, _doc_id: &str) -> Result<(), IndexingError> {
            Ok(())
        }

        fn commit(&self) -> Result<(), IndexingError> {
            Ok(())
        }

        fn index_type(&self) -> IndexType {
            self.index_type
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path()).unwrap();
        (Arc::new(storage), temp_dir)
    }

    #[test]
    fn test_add_result_records_sequence_zero() {
        let mut result = ProcessResult::new();
        let mut update = UpdateResult::new();
        update.record_success();
        update.set_sequence(0);
        result.add_result(IndexType::Bm25, update);
        assert_eq!(result.last_sequence, Some(0));
        assert_eq!(result.total_processed, 1);
        assert!(result.has_updates());
    }

    #[test]
    fn test_pipeline_creation() {
        let (storage, _temp) = create_test_storage();
        let pipeline = IndexingPipeline::new(storage, PipelineConfig::default());

        assert_eq!(pipeline.updater_count(), 0);
    }

    #[test]
    fn test_add_updater() {
        let (storage, _temp) = create_test_storage();
        let mut pipeline = IndexingPipeline::new(storage, PipelineConfig::default());

        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Vector, "vector")));

        assert_eq!(pipeline.updater_count(), 2);
        assert_eq!(pipeline.updater_names(), vec!["bm25", "vector"]);
    }

    #[test]
    fn test_load_save_checkpoints() {
        let (storage, _temp) = create_test_storage();
        let mut pipeline = IndexingPipeline::new(storage.clone(), PipelineConfig::default());

        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();

        // Should have a checkpoint now
        let cp = pipeline.get_checkpoint(IndexType::Bm25);
        assert!(cp.is_some());
        assert_eq!(cp.unwrap().last_sequence, 0);

        // Modify and save
        pipeline
            .checkpoints
            .get_mut(&IndexType::Bm25)
            .unwrap()
            .update(42, 10);
        pipeline.save_checkpoints().unwrap();

        // Create new pipeline and load
        let mut pipeline2 = IndexingPipeline::new(storage, PipelineConfig::default());
        pipeline2.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline2.load_checkpoints().unwrap();

        let cp2 = pipeline2.get_checkpoint(IndexType::Bm25);
        assert!(cp2.is_some());
        assert_eq!(cp2.unwrap().last_sequence, 42);
    }

    #[test]
    fn test_process_batch_empty() {
        let (storage, _temp) = create_test_storage();
        let mut pipeline = IndexingPipeline::new(storage, PipelineConfig::default());

        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();

        let result = pipeline.process_batch(100).unwrap();
        assert!(!result.has_updates());
        assert!(result.last_sequence.is_none());
    }

    #[test]
    fn test_process_batch_with_entries() {
        let (storage, _temp_dir) = create_test_storage();

        // Add some outbox entries via events
        for i in 0..5 {
            let event_id = format!("event-{}", i);
            let outbox_entry = OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&ulid::Ulid::new().to_string(), b"test", &outbox_bytes)
                .unwrap();
        }

        let mut pipeline = IndexingPipeline::new(storage, PipelineConfig::default());
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();

        let result = pipeline.process_batch(100).unwrap();
        assert!(result.has_updates());
        assert_eq!(result.total_processed, 5);
        assert!(result.last_sequence.is_some());
        assert!(result.committed);
    }

    #[test]
    fn test_process_batch_sequence_zero_advances_checkpoint() {
        let (storage, _temp_dir) = create_test_storage();
        let outbox_entry = OutboxEntry::for_index("event-0".to_string(), 0);
        let outbox_bytes = outbox_entry.to_bytes().unwrap();
        storage
            .put_event(&ulid::Ulid::new().to_string(), b"test", &outbox_bytes)
            .unwrap();

        let mut pipeline = IndexingPipeline::new(storage.clone(), PipelineConfig::default());
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();

        let result = pipeline.process_batch(100).unwrap();
        assert!(result.has_updates());
        assert_eq!(result.total_processed, 1);

        let bytes = storage
            .get_checkpoint("index_bm25")
            .unwrap()
            .expect("sequence-0 batch must persist a checkpoint");
        let cp = IndexCheckpoint::from_bytes(&bytes).unwrap();
        assert_eq!(cp.last_sequence, 0);
        assert!(
            cp.processed_count > 0,
            "fresh checkpoint must record processed_count after seq 0"
        );
    }

    #[test]
    fn test_process_until_caught_up() {
        let (storage, _temp_dir) = create_test_storage();

        // Add outbox entries
        for i in 0..10 {
            let event_id = format!("event-{}", i);
            let outbox_entry = OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&ulid::Ulid::new().to_string(), b"test", &outbox_bytes)
                .unwrap();
        }

        let config = PipelineConfig::default().with_batch_size(3);
        let mut pipeline = IndexingPipeline::new(storage, config);
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();

        let result = pipeline.process_until_caught_up(100).unwrap();
        assert_eq!(result.total_processed, 10);
    }

    #[test]
    fn test_cleanup_outbox() {
        let (storage, _temp_dir) = create_test_storage();

        // Add outbox entries
        for i in 0..5 {
            let event_id = format!("event-{}", i);
            let outbox_entry = OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&ulid::Ulid::new().to_string(), b"test", &outbox_bytes)
                .unwrap();
        }

        let mut pipeline = IndexingPipeline::new(storage.clone(), PipelineConfig::default());
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();

        // Process all entries
        pipeline.process_until_caught_up(100).unwrap();

        // Cleanup should delete processed entries
        let deleted = pipeline.cleanup_outbox().unwrap();
        // Deletes up to min_seq - 1, so 4 entries (0, 1, 2, 3)
        assert!(deleted >= 3);

        // Verify remaining entries
        let remaining = storage.get_outbox_entries(0, 100).unwrap();
        assert!(remaining.len() <= 2); // At most entries 4 and maybe 5
    }

    #[test]
    fn test_min_checkpoint_sequence() {
        let (storage, _temp) = create_test_storage();
        let mut pipeline = IndexingPipeline::new(storage, PipelineConfig::default());

        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Vector, "vector")));
        pipeline.load_checkpoints().unwrap();

        // Both start at 0
        assert_eq!(pipeline.min_checkpoint_sequence(), 0);

        // Update one
        pipeline
            .checkpoints
            .get_mut(&IndexType::Bm25)
            .unwrap()
            .update(10, 5);

        // Min should still be 0 (vector hasn't caught up)
        assert_eq!(pipeline.min_checkpoint_sequence(), 0);

        // Update both
        pipeline
            .checkpoints
            .get_mut(&IndexType::Vector)
            .unwrap()
            .update(8, 4);

        // Min should now be 8
        assert_eq!(pipeline.min_checkpoint_sequence(), 8);
    }

    #[test]
    fn test_pipeline_config() {
        let config = PipelineConfig::default()
            .with_batch_size(50)
            .with_continue_on_error(false)
            .with_commit_after_batch(false);

        assert_eq!(config.batch_size, 50);
        assert!(!config.continue_on_error);
        assert!(!config.commit_after_batch);
    }

    #[test]
    fn test_process_result() {
        let mut result = ProcessResult::new();
        assert!(!result.has_updates());

        let update1 = UpdateResult {
            processed: 5,
            skipped: 2,
            errors: 1,
            last_sequence: 10,
        };

        result.add_result(IndexType::Bm25, update1);
        assert!(result.has_updates());
        assert_eq!(result.total_processed, 5);
        assert_eq!(result.last_sequence, Some(10));

        let update2 = UpdateResult {
            processed: 3,
            skipped: 0,
            errors: 0,
            last_sequence: 15,
        };

        result.add_result(IndexType::Vector, update2);
        assert_eq!(result.total_processed, 8);
        assert_eq!(result.last_sequence, Some(15));
    }

    fn seed_outbox_events(storage: &Storage, n: usize) {
        for i in 0..n {
            let event_id = ulid::Ulid::new().to_string();
            let outbox_entry = OutboxEntry::for_index(event_id.clone(), i as i64 * 1000);
            storage
                .put_event(
                    &event_id,
                    format!("event-{i}").as_bytes(),
                    &outbox_entry.to_bytes().unwrap(),
                )
                .unwrap();
        }
    }

    fn pipeline_with_mock(storage: Arc<Storage>) -> IndexingPipeline {
        let mut pipeline = IndexingPipeline::new(storage, PipelineConfig::default());
        pipeline.add_updater(Box::new(MockUpdater::new(IndexType::Bm25, "bm25")));
        pipeline.load_checkpoints().unwrap();
        pipeline
    }

    #[test]
    fn test_backfill_empty_store() {
        let (storage, _temp) = create_test_storage();
        let mut pipeline = pipeline_with_mock(storage);
        let report = pipeline
            .backfill(BackfillConfig::default(), |_, _| {})
            .unwrap();
        assert_eq!(report.documents, 0);
        assert_eq!(report.would_index, 0);
        assert!(!report.has_updates());
    }

    #[test]
    fn test_backfill_outbox_then_second_run_zero() {
        let (storage, _temp) = create_test_storage();
        seed_outbox_events(&storage, 10);
        let mut pipeline = pipeline_with_mock(storage.clone());

        let report = pipeline
            .backfill(BackfillConfig::default().with_batch_size(4), |_, _| {})
            .unwrap();
        assert_eq!(report.documents, 10);
        assert_eq!(report.would_index, 10);
        assert_eq!(report.source, BackfillSource::Outbox);
        assert!(report.last_sequence.is_some());

        let cp = pipeline.get_checkpoint(IndexType::Bm25).unwrap();
        assert_eq!(cp.last_sequence, report.last_sequence.unwrap());
        assert!(cp.processed_count >= 10);

        let second = pipeline
            .backfill(BackfillConfig::default(), |_, _| {})
            .unwrap();
        assert_eq!(second.documents, 0);
        assert_eq!(second.would_index, 0);
    }

    #[test]
    fn test_backfill_dry_run_writes_nothing() {
        let (storage, _temp) = create_test_storage();
        seed_outbox_events(&storage, 7);
        let mut pipeline = pipeline_with_mock(storage.clone());

        let before = storage.get_checkpoint("index_bm25").unwrap();
        let report = pipeline
            .backfill(BackfillConfig::default().with_dry_run(true), |_, _| {})
            .unwrap();
        assert!(report.dry_run);
        assert_eq!(report.documents, 0);
        assert_eq!(report.would_index, 7);
        assert_eq!(storage.get_checkpoint("index_bm25").unwrap(), before);

        let cp = pipeline.get_checkpoint(IndexType::Bm25).unwrap();
        assert_eq!(cp.processed_count, 0);
        assert_eq!(cp.last_sequence, 0);
    }

    #[test]
    fn test_backfill_resumes_after_max_batches() {
        let (storage, _temp) = create_test_storage();
        seed_outbox_events(&storage, 10);
        let mut pipeline = pipeline_with_mock(storage.clone());

        let first = pipeline
            .backfill(
                BackfillConfig::default()
                    .with_batch_size(3)
                    .with_max_batches(Some(1)),
                |_, _| {},
            )
            .unwrap();
        assert_eq!(first.documents, 3);
        assert_eq!(first.last_sequence, Some(2));

        let mut pipeline2 = pipeline_with_mock(storage);
        let rest = pipeline2
            .backfill(BackfillConfig::default().with_batch_size(3), |_, _| {})
            .unwrap();
        assert_eq!(rest.documents, 7);
        assert_eq!(rest.last_sequence, Some(9));
    }

    #[test]
    fn test_backfill_event_log_when_outbox_empty() {
        let (storage, _temp) = create_test_storage();
        seed_outbox_events(&storage, 5);
        // Simulate a drained/cleaned outbox: events remain, outbox gone.
        storage.delete_outbox_entries(u64::MAX).unwrap();
        assert!(storage.get_outbox_entries(0, 10).unwrap().is_empty());

        let mut pipeline = pipeline_with_mock(storage.clone());
        let report = pipeline
            .backfill(BackfillConfig::default(), |_, _| {})
            .unwrap();
        assert_eq!(report.source, BackfillSource::EventLog);
        assert_eq!(report.documents, 5);

        let second = pipeline
            .backfill(BackfillConfig::default(), |_, _| {})
            .unwrap();
        assert_eq!(second.documents, 0);
        assert_eq!(second.would_index, 0);
    }

    #[test]
    fn test_backfill_from_sequence_zero_reindexes() {
        let (storage, _temp) = create_test_storage();
        seed_outbox_events(&storage, 4);
        let mut pipeline = pipeline_with_mock(storage);

        let first = pipeline
            .backfill(BackfillConfig::default(), |_, _| {})
            .unwrap();
        assert_eq!(first.documents, 4);

        // Caught up. Force re-index of empty previews via --from-sequence 0.
        let again = pipeline
            .backfill(
                BackfillConfig::default().with_from_sequence(Some(0)),
                |_, _| {},
            )
            .unwrap();
        assert_eq!(again.source, BackfillSource::EventLog);
        assert_eq!(again.documents, 4);
        assert_eq!(again.would_index, 4);
    }

    #[test]
    fn test_backfill_progress_callback() {
        let (storage, _temp) = create_test_storage();
        seed_outbox_events(&storage, 5);
        let mut pipeline = pipeline_with_mock(storage);
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let ticks_clone = ticks.clone();
        pipeline
            .backfill(
                BackfillConfig::default().with_batch_size(2),
                move |n, total| {
                    ticks_clone.lock().unwrap().push((n, total));
                },
            )
            .unwrap();
        let ticks = ticks.lock().unwrap().clone();
        assert!(!ticks.is_empty());
        assert_eq!(ticks.last().copied(), Some((5, 5)));
    }
}
