//! Index rebuild functionality for reconstructing search indexes from storage.
//!
//! Provides utilities for rebuilding BM25 and vector indexes from scratch
//! by iterating through all TOC nodes and grips in storage.

use std::sync::Arc;

use tracing::{debug, info, warn};

use memory_storage::Storage;
use memory_types::{Grip, TocLevel, TocNode};

use crate::bm25_updater::Bm25IndexUpdater;
use crate::checkpoint::{IndexCheckpoint, IndexType};
use crate::error::IndexingError;
use crate::updater::IndexUpdater;
use crate::vector_updater::VectorIndexUpdater;

/// Configuration for index rebuild operations.
#[derive(Debug, Clone)]
pub struct RebuildConfig {
    /// Number of documents to process before reporting progress.
    pub batch_size: usize,
    /// Which index types to rebuild.
    pub index_types: Vec<IndexType>,
    /// Whether to clear existing indexes before rebuilding.
    pub clear_first: bool,
    /// Whether to continue on individual document errors.
    pub continue_on_error: bool,
}

impl Default for RebuildConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            index_types: vec![IndexType::Bm25, IndexType::Vector],
            clear_first: true,
            continue_on_error: true,
        }
    }
}

impl RebuildConfig {
    /// Create a config for BM25 only.
    pub fn bm25_only() -> Self {
        Self {
            index_types: vec![IndexType::Bm25],
            ..Default::default()
        }
    }

    /// Create a config for vector only.
    pub fn vector_only() -> Self {
        Self {
            index_types: vec![IndexType::Vector],
            ..Default::default()
        }
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set whether to clear indexes first.
    pub fn with_clear_first(mut self, clear: bool) -> Self {
        self.clear_first = clear;
        self
    }

    /// Set whether to continue on errors.
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }
}

/// Progress tracking for rebuild operations.
#[derive(Debug, Clone, Default)]
pub struct RebuildProgress {
    /// Total documents processed.
    pub total_processed: u64,
    /// Number of TOC nodes indexed.
    pub toc_nodes_indexed: u64,
    /// Number of grips indexed.
    pub grips_indexed: u64,
    /// Number of errors encountered.
    pub errors: u64,
    /// Number of documents skipped (already indexed or empty).
    pub skipped: u64,
    /// Whether the rebuild completed successfully.
    pub completed: bool,
}

impl RebuildProgress {
    /// Create a new progress tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful TOC node index.
    pub fn record_toc_node(&mut self) {
        self.toc_nodes_indexed += 1;
        self.total_processed += 1;
    }

    /// Record a successful grip index.
    pub fn record_grip(&mut self) {
        self.grips_indexed += 1;
        self.total_processed += 1;
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        self.errors += 1;
        self.total_processed += 1;
    }

    /// Record a skipped document.
    pub fn record_skip(&mut self) {
        self.skipped += 1;
        self.total_processed += 1;
    }

    /// Mark as completed.
    pub fn mark_completed(&mut self) {
        self.completed = true;
    }
}

/// Result of a rebuild operation.
#[derive(Debug)]
pub struct RebuildResult {
    /// Progress statistics.
    pub progress: RebuildProgress,
    /// Time taken in milliseconds.
    pub elapsed_ms: u64,
    /// Per-index results.
    pub index_results: Vec<(IndexType, IndexCheckpoint)>,
}

/// Trait for receiving rebuild progress updates.
pub trait ProgressCallback: Send {
    /// Called after each batch of documents is processed.
    fn on_progress(&self, progress: &RebuildProgress);
}

/// A no-op progress callback for when progress reporting isn't needed.
pub struct NoOpProgressCallback;

impl ProgressCallback for NoOpProgressCallback {
    fn on_progress(&self, _progress: &RebuildProgress) {}
}

/// A callback that logs progress at info level.
pub struct LoggingProgressCallback {
    batch_size: usize,
}

impl LoggingProgressCallback {
    /// Create a new logging progress callback.
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

impl ProgressCallback for LoggingProgressCallback {
    fn on_progress(&self, progress: &RebuildProgress) {
        if progress
            .total_processed
            .is_multiple_of(self.batch_size as u64)
        {
            info!(
                total = progress.total_processed,
                toc_nodes = progress.toc_nodes_indexed,
                grips = progress.grips_indexed,
                errors = progress.errors,
                "Rebuild progress"
            );
        }
    }
}

/// Iterate through all TOC nodes in storage.
///
/// Returns nodes from all levels, ordered by level (Year -> Month -> Week -> Day -> Segment).
pub fn iter_all_toc_nodes(storage: &Storage) -> Result<Vec<TocNode>, IndexingError> {
    let mut all_nodes = Vec::new();

    // Iterate through all TOC levels
    for level in &[
        TocLevel::Year,
        TocLevel::Month,
        TocLevel::Week,
        TocLevel::Day,
        TocLevel::Segment,
    ] {
        let nodes = storage
            .get_toc_nodes_by_level(*level, None, None)
            .map_err(IndexingError::Storage)?;
        all_nodes.extend(nodes);
    }

    debug!(count = all_nodes.len(), "Found TOC nodes in storage");
    Ok(all_nodes)
}

/// Iterate through all grips in storage.
///
/// Uses prefix iteration on the grips column family.
pub fn iter_all_grips(storage: &Storage) -> Result<Vec<Grip>, IndexingError> {
    let mut grips = Vec::new();

    // Use prefix_iterator with empty prefix to get all grips
    // Grips are stored with their grip_id as key
    // We need to filter out index entries (which start with "node:")
    let entries = storage
        .prefix_iterator("grips", b"grip:")
        .map_err(IndexingError::Storage)?;

    for (key, value) in entries {
        let key_str = String::from_utf8_lossy(&key);
        // Only process grip entries, not index entries
        if key_str.starts_with("grip:") {
            match Grip::from_bytes(&value) {
                Ok(grip) => grips.push(grip),
                Err(e) => {
                    warn!(key = %key_str, error = %e, "Failed to deserialize grip");
                }
            }
        }
    }

    debug!(count = grips.len(), "Found grips in storage");
    Ok(grips)
}

/// Rebuild BM25 index from storage.
pub fn rebuild_bm25_index<P: ProgressCallback>(
    storage: Arc<Storage>,
    updater: &Bm25IndexUpdater,
    config: &RebuildConfig,
    progress_callback: &P,
) -> Result<RebuildProgress, IndexingError> {
    let mut progress = RebuildProgress::new();

    info!("Starting BM25 index rebuild...");

    // Iterate through all TOC nodes
    let nodes = iter_all_toc_nodes(&storage)?;
    info!(count = nodes.len(), "Found TOC nodes to index");

    for node in nodes {
        match updater.index_node(&node) {
            Ok(()) => {
                progress.record_toc_node();
            }
            Err(e) => {
                if config.continue_on_error {
                    warn!(node_id = %node.node_id, error = %e, "Failed to index TOC node");
                    progress.record_error();
                } else {
                    return Err(e);
                }
            }
        }

        if progress
            .total_processed
            .is_multiple_of(config.batch_size as u64)
        {
            progress_callback.on_progress(&progress);
        }
    }

    // Iterate through all grips
    let grips = iter_all_grips(&storage)?;
    info!(count = grips.len(), "Found grips to index");

    for grip in grips {
        match updater.index_grip_direct(&grip) {
            Ok(()) => {
                progress.record_grip();
            }
            Err(e) => {
                if config.continue_on_error {
                    warn!(grip_id = %grip.grip_id, error = %e, "Failed to index grip");
                    progress.record_error();
                } else {
                    return Err(e);
                }
            }
        }

        if progress
            .total_processed
            .is_multiple_of(config.batch_size as u64)
        {
            progress_callback.on_progress(&progress);
        }
    }

    // Commit the index
    updater.commit()?;
    progress.mark_completed();
    progress_callback.on_progress(&progress);

    info!(
        toc_nodes = progress.toc_nodes_indexed,
        grips = progress.grips_indexed,
        errors = progress.errors,
        "BM25 index rebuild complete"
    );

    Ok(progress)
}

/// Rebuild vector index from storage.
pub fn rebuild_vector_index<P: ProgressCallback, E: memory_embeddings::EmbeddingModel>(
    storage: Arc<Storage>,
    updater: &VectorIndexUpdater<E>,
    config: &RebuildConfig,
    progress_callback: &P,
) -> Result<RebuildProgress, IndexingError> {
    let mut progress = RebuildProgress::new();

    info!("Starting vector index rebuild...");

    // Iterate through all TOC nodes
    let nodes = iter_all_toc_nodes(&storage)?;
    info!(count = nodes.len(), "Found TOC nodes to index");

    for node in nodes {
        match updater.index_node(&node) {
            Ok(true) => {
                progress.record_toc_node();
            }
            Ok(false) => {
                progress.record_skip();
            }
            Err(e) => {
                if config.continue_on_error {
                    warn!(node_id = %node.node_id, error = %e, "Failed to index TOC node");
                    progress.record_error();
                } else {
                    return Err(e);
                }
            }
        }

        if progress
            .total_processed
            .is_multiple_of(config.batch_size as u64)
        {
            progress_callback.on_progress(&progress);
        }
    }

    // Iterate through all grips
    let grips = iter_all_grips(&storage)?;
    info!(count = grips.len(), "Found grips to index");

    for grip in grips {
        match updater.index_grip_direct(&grip) {
            Ok(true) => {
                progress.record_grip();
            }
            Ok(false) => {
                progress.record_skip();
            }
            Err(e) => {
                if config.continue_on_error {
                    warn!(grip_id = %grip.grip_id, error = %e, "Failed to index grip");
                    progress.record_error();
                } else {
                    return Err(e);
                }
            }
        }

        if progress
            .total_processed
            .is_multiple_of(config.batch_size as u64)
        {
            progress_callback.on_progress(&progress);
        }
    }

    // Commit the index
    updater.commit()?;
    progress.mark_completed();
    progress_callback.on_progress(&progress);

    info!(
        toc_nodes = progress.toc_nodes_indexed,
        grips = progress.grips_indexed,
        skipped = progress.skipped,
        errors = progress.errors,
        "Vector index rebuild complete"
    );

    Ok(progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebuild_config_default() {
        let config = RebuildConfig::default();
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.index_types.len(), 2);
        assert!(config.clear_first);
        assert!(config.continue_on_error);
    }

    #[test]
    fn test_rebuild_config_bm25_only() {
        let config = RebuildConfig::bm25_only();
        assert_eq!(config.index_types.len(), 1);
        assert_eq!(config.index_types[0], IndexType::Bm25);
    }

    #[test]
    fn test_rebuild_config_vector_only() {
        let config = RebuildConfig::vector_only();
        assert_eq!(config.index_types.len(), 1);
        assert_eq!(config.index_types[0], IndexType::Vector);
    }

    #[test]
    fn test_rebuild_config_builder() {
        let config = RebuildConfig::default()
            .with_batch_size(50)
            .with_clear_first(false)
            .with_continue_on_error(false);

        assert_eq!(config.batch_size, 50);
        assert!(!config.clear_first);
        assert!(!config.continue_on_error);
    }

    #[test]
    fn test_rebuild_progress() {
        let mut progress = RebuildProgress::new();
        assert_eq!(progress.total_processed, 0);

        progress.record_toc_node();
        assert_eq!(progress.toc_nodes_indexed, 1);
        assert_eq!(progress.total_processed, 1);

        progress.record_grip();
        assert_eq!(progress.grips_indexed, 1);
        assert_eq!(progress.total_processed, 2);

        progress.record_error();
        assert_eq!(progress.errors, 1);
        assert_eq!(progress.total_processed, 3);

        progress.record_skip();
        assert_eq!(progress.skipped, 1);
        assert_eq!(progress.total_processed, 4);

        assert!(!progress.completed);
        progress.mark_completed();
        assert!(progress.completed);
    }

    #[test]
    fn test_no_op_progress_callback() {
        let callback = NoOpProgressCallback;
        let progress = RebuildProgress::new();
        callback.on_progress(&progress); // Should not panic
    }
}
