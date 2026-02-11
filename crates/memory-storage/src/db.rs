//! RocksDB wrapper for agent-memory storage.
//!
//! Provides:
//! - Database open/close with column family setup
//! - Atomic write batches (event + outbox per ING-05)
//! - Single-key and range reads
//! - Idempotent writes (ING-03)

use rocksdb::{Direction, IteratorMode, Options, WriteBatch, DB};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};

use crate::column_families::{
    build_cf_descriptors, ALL_CF_NAMES, CF_CHECKPOINTS, CF_EVENTS, CF_GRIPS, CF_OUTBOX,
    CF_TOC_LATEST, CF_TOC_NODES,
};
use crate::error::StorageError;
use crate::keys::{CheckpointKey, EventKey, OutboxKey};
use memory_types::OutboxEntry;

// Re-export TocLevel for use in this crate
pub use memory_types::TocLevel;

/// Main storage interface for agent-memory
pub struct Storage {
    db: DB,
    /// Outbox sequence counter for monotonic ordering
    outbox_sequence: AtomicU64,
}

impl Storage {
    /// Open storage at the given path, creating if necessary
    ///
    /// Per STOR-04: Each project gets its own RocksDB instance.
    /// Per STOR-05: Uses Universal compaction for append-only workload.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        info!("Opening storage at {:?}", path);

        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        // Universal compaction for append-only (STOR-05)
        db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);
        // Limit memory usage during compaction
        db_opts.set_max_background_jobs(4);

        let cf_descriptors = build_cf_descriptors();
        let db = DB::open_cf_descriptors(&db_opts, path, cf_descriptors)?;

        // Initialize outbox sequence from highest existing key
        let outbox_sequence = Self::load_outbox_sequence(&db)?;

        Ok(Self {
            db,
            outbox_sequence: AtomicU64::new(outbox_sequence),
        })
    }

    /// Load the highest outbox sequence number from storage
    fn load_outbox_sequence(db: &DB) -> Result<u64, StorageError> {
        let cf = db
            .cf_handle(CF_OUTBOX)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_OUTBOX.to_string()))?;

        // Iterate in reverse to find highest key
        let mut iter = db.iterator_cf(&cf, IteratorMode::End);
        if let Some(result) = iter.next() {
            let (key, _) = result?;
            let outbox_key = OutboxKey::from_bytes(&key)?;
            return Ok(outbox_key.sequence + 1);
        }
        Ok(0)
    }

    /// Get next outbox sequence number
    fn next_outbox_sequence(&self) -> u64 {
        self.outbox_sequence.fetch_add(1, Ordering::SeqCst)
    }

    /// Store an event with atomic outbox entry (ING-05)
    ///
    /// Returns (event_key, created) where created=false if event already existed (ING-03 idempotent)
    pub fn put_event(
        &self,
        event_id: &str,
        event_bytes: &[u8],
        outbox_bytes: &[u8],
    ) -> Result<(EventKey, bool), StorageError> {
        let events_cf = self
            .db
            .cf_handle(CF_EVENTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EVENTS.to_string()))?;
        let outbox_cf = self
            .db
            .cf_handle(CF_OUTBOX)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_OUTBOX.to_string()))?;

        // Parse event_id to get key (ING-03: idempotent using event_id)
        let event_key = EventKey::from_event_id(event_id)?;

        // Check if already exists (idempotent)
        if self.db.get_cf(&events_cf, event_key.to_bytes())?.is_some() {
            debug!("Event {} already exists, skipping", event_id);
            return Ok((event_key, false));
        }

        // Atomic write: event + outbox entry
        let outbox_key = OutboxKey::new(self.next_outbox_sequence());

        let mut batch = WriteBatch::default();
        batch.put_cf(&events_cf, event_key.to_bytes(), event_bytes);
        batch.put_cf(&outbox_cf, outbox_key.to_bytes(), outbox_bytes);

        self.db.write(batch)?;
        debug!(
            "Stored event {} with outbox seq {}",
            event_id, outbox_key.sequence
        );

        Ok((event_key, true))
    }

    /// Get an event by its event_id
    pub fn get_event(&self, event_id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let events_cf = self
            .db
            .cf_handle(CF_EVENTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EVENTS.to_string()))?;

        let event_key = EventKey::from_event_id(event_id)?;
        let result = self.db.get_cf(&events_cf, event_key.to_bytes())?;
        Ok(result)
    }

    /// Get events in a time range [start_ms, end_ms)
    ///
    /// Returns Vec<(EventKey, bytes)> ordered by time.
    pub fn get_events_in_range(
        &self,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<(EventKey, Vec<u8>)>, StorageError> {
        let events_cf = self
            .db
            .cf_handle(CF_EVENTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EVENTS.to_string()))?;

        let start_prefix = EventKey::prefix_start(start_ms);
        let end_prefix = EventKey::prefix_end(end_ms);

        let mut results = Vec::new();
        let iter = self.db.iterator_cf(
            &events_cf,
            IteratorMode::From(&start_prefix, Direction::Forward),
        );

        for item in iter {
            let (key, value) = item?;
            // Stop if we've passed the end prefix
            if key.as_ref() >= end_prefix.as_slice() {
                break;
            }
            let event_key = EventKey::from_bytes(&key)?;
            results.push((event_key, value.to_vec()));
        }

        Ok(results)
    }

    /// Store a checkpoint for crash recovery (STOR-03)
    pub fn put_checkpoint(
        &self,
        job_name: &str,
        checkpoint_bytes: &[u8],
    ) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_CHECKPOINTS.to_string()))?;

        let key = CheckpointKey::new(job_name);
        self.db.put_cf(&cf, key.to_bytes(), checkpoint_bytes)?;
        Ok(())
    }

    /// Get a checkpoint for crash recovery (STOR-03)
    pub fn get_checkpoint(&self, job_name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_CHECKPOINTS.to_string()))?;

        let key = CheckpointKey::new(job_name);
        let result = self.db.get_cf(&cf, key.to_bytes())?;
        Ok(result)
    }

    // ==================== Outbox Methods ====================

    /// Get outbox entries starting from a sequence number.
    ///
    /// Returns Vec of (sequence, entry) tuples in sequence order.
    /// Used by indexing pipelines to consume outbox entries.
    pub fn get_outbox_entries(
        &self,
        start_sequence: u64,
        limit: usize,
    ) -> Result<Vec<(u64, OutboxEntry)>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_OUTBOX)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_OUTBOX.to_string()))?;

        let start_key = OutboxKey::new(start_sequence);
        let iter = self.db.iterator_cf(
            &cf,
            IteratorMode::From(&start_key.to_bytes(), Direction::Forward),
        );

        let mut results = Vec::new();
        for item in iter.take(limit) {
            let (key, value) = item?;
            let outbox_key = OutboxKey::from_bytes(&key)?;
            let entry = OutboxEntry::from_bytes(&value)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            results.push((outbox_key.sequence, entry));
        }

        Ok(results)
    }

    /// Delete outbox entries up to and including a sequence number.
    ///
    /// Used to clean up processed outbox entries after all indexes
    /// have been updated. Returns count of deleted entries.
    pub fn delete_outbox_entries(&self, up_to_sequence: u64) -> Result<usize, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_OUTBOX)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_OUTBOX.to_string()))?;

        // Collect keys to delete
        let iter = self.db.iterator_cf(&cf, IteratorMode::Start);
        let mut batch = WriteBatch::default();
        let mut count = 0;

        for item in iter {
            let (key, _) = item?;
            let outbox_key = OutboxKey::from_bytes(&key)?;

            if outbox_key.sequence > up_to_sequence {
                break;
            }

            batch.delete_cf(&cf, &key);
            count += 1;
        }

        if count > 0 {
            self.db.write(batch)?;
            debug!(
                "Deleted {} outbox entries up to sequence {}",
                count, up_to_sequence
            );
        }

        Ok(count)
    }

    /// Flush all column families to disk
    pub fn flush(&self) -> Result<(), StorageError> {
        for cf_name in ALL_CF_NAMES {
            if let Some(cf) = self.db.cf_handle(cf_name) {
                self.db.flush_cf(&cf)?;
            }
        }
        Ok(())
    }

    // ==================== TOC Node Methods ====================

    /// Store a TOC node with versioning (TOC-06).
    ///
    /// Appends a new version rather than mutating.
    /// Updates toc_latest to point to new version.
    pub fn put_toc_node(&self, node: &memory_types::TocNode) -> Result<(), StorageError> {
        let nodes_cf = self
            .db
            .cf_handle(CF_TOC_NODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_NODES.to_string()))?;
        let latest_cf = self
            .db
            .cf_handle(CF_TOC_LATEST)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_LATEST.to_string()))?;

        // Get current version
        let latest_key = format!("latest:{}", node.node_id);
        let current_version = self
            .db
            .get_cf(&latest_cf, &latest_key)?
            .map(|b| {
                if b.len() >= 4 {
                    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
                } else {
                    0
                }
            })
            .unwrap_or(0);

        let new_version = current_version + 1;
        let versioned_key = format!("toc:{}:v{:06}", node.node_id, new_version);

        // Update node version
        let mut versioned_node = node.clone();
        versioned_node.version = new_version;

        let node_bytes = versioned_node
            .to_bytes()
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Atomic write: node + latest pointer
        let mut batch = WriteBatch::default();
        batch.put_cf(&nodes_cf, versioned_key.as_bytes(), &node_bytes);
        batch.put_cf(&latest_cf, latest_key.as_bytes(), new_version.to_be_bytes());

        self.db.write(batch)?;

        debug!(node_id = %node.node_id, version = new_version, "Stored TOC node");
        Ok(())
    }

    /// Get the latest version of a TOC node.
    pub fn get_toc_node(
        &self,
        node_id: &str,
    ) -> Result<Option<memory_types::TocNode>, StorageError> {
        let nodes_cf = self
            .db
            .cf_handle(CF_TOC_NODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_NODES.to_string()))?;
        let latest_cf = self
            .db
            .cf_handle(CF_TOC_LATEST)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_LATEST.to_string()))?;

        // Get latest version number
        let latest_key = format!("latest:{}", node_id);
        let version = match self.db.get_cf(&latest_cf, &latest_key)? {
            Some(b) if b.len() >= 4 => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            _ => return Ok(None),
        };

        // Get versioned node
        let versioned_key = format!("toc:{}:v{:06}", node_id, version);
        match self.db.get_cf(&nodes_cf, versioned_key.as_bytes())? {
            Some(bytes) => {
                let node = memory_types::TocNode::from_bytes(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    /// Get TOC nodes by level, optionally filtered by time range.
    pub fn get_toc_nodes_by_level(
        &self,
        level: memory_types::TocLevel,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<memory_types::TocNode>, StorageError> {
        let nodes_cf = self
            .db
            .cf_handle(CF_TOC_NODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_NODES.to_string()))?;
        let latest_cf = self
            .db
            .cf_handle(CF_TOC_LATEST)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_LATEST.to_string()))?;

        let level_prefix = format!("latest:toc:{}:", level);
        let mut nodes = Vec::new();

        // Iterate through latest pointers to find all nodes of this level
        let iter = self.db.iterator_cf(
            &latest_cf,
            IteratorMode::From(level_prefix.as_bytes(), Direction::Forward),
        );

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Stop if we've passed this level's prefix
            if !key_str.starts_with(&level_prefix) {
                break;
            }

            // Get the node_id from key
            let node_id = key_str.trim_start_matches("latest:");
            if value.len() >= 4 {
                let version = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
                let versioned_key = format!("toc:{}:v{:06}", node_id, version);

                if let Some(bytes) = self.db.get_cf(&nodes_cf, versioned_key.as_bytes())? {
                    let node = memory_types::TocNode::from_bytes(&bytes)
                        .map_err(|e| StorageError::Serialization(e.to_string()))?;

                    // Filter by time range if specified
                    let include = match (start_time, end_time) {
                        (Some(start), Some(end)) => {
                            node.end_time >= start && node.start_time <= end
                        }
                        (Some(start), None) => node.end_time >= start,
                        (None, Some(end)) => node.start_time <= end,
                        (None, None) => true,
                    };

                    if include {
                        nodes.push(node);
                    }
                }
            }
        }

        // Sort by start_time
        nodes.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        Ok(nodes)
    }

    /// Get child nodes of a parent node.
    pub fn get_child_nodes(
        &self,
        parent_node_id: &str,
    ) -> Result<Vec<memory_types::TocNode>, StorageError> {
        let parent = self.get_toc_node(parent_node_id)?;
        match parent {
            Some(node) => {
                let mut children = Vec::new();
                for child_id in &node.child_node_ids {
                    if let Some(child) = self.get_toc_node(child_id)? {
                        children.push(child);
                    }
                }
                children.sort_by(|a, b| a.start_time.cmp(&b.start_time));
                Ok(children)
            }
            None => Ok(Vec::new()),
        }
    }

    // ==================== Grip Methods ====================

    /// Store a grip.
    pub fn put_grip(&self, grip: &memory_types::Grip) -> Result<(), StorageError> {
        let grips_cf = self
            .db
            .cf_handle(CF_GRIPS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_GRIPS.to_string()))?;

        let grip_bytes = grip
            .to_bytes()
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.db
            .put_cf(&grips_cf, grip.grip_id.as_bytes(), &grip_bytes)?;

        // If linked to a TOC node, create index entry
        if let Some(ref node_id) = grip.toc_node_id {
            let index_key = format!("node:{}:{}", node_id, grip.grip_id);
            self.db.put_cf(&grips_cf, index_key.as_bytes(), [])?;
        }

        debug!(grip_id = %grip.grip_id, "Stored grip");
        Ok(())
    }

    /// Get a grip by ID.
    pub fn get_grip(&self, grip_id: &str) -> Result<Option<memory_types::Grip>, StorageError> {
        let grips_cf = self
            .db
            .cf_handle(CF_GRIPS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_GRIPS.to_string()))?;

        match self.db.get_cf(&grips_cf, grip_id.as_bytes())? {
            Some(bytes) => {
                let grip = memory_types::Grip::from_bytes(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(grip))
            }
            None => Ok(None),
        }
    }

    /// Get all grips linked to a TOC node.
    pub fn get_grips_for_node(
        &self,
        node_id: &str,
    ) -> Result<Vec<memory_types::Grip>, StorageError> {
        let grips_cf = self
            .db
            .cf_handle(CF_GRIPS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_GRIPS.to_string()))?;

        let prefix = format!("node:{}:", node_id);
        let mut grips = Vec::new();

        let iter = self.db.iterator_cf(
            &grips_cf,
            IteratorMode::From(prefix.as_bytes(), Direction::Forward),
        );

        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Stop if we've passed this node's prefix
            if !key_str.starts_with(&prefix) {
                break;
            }

            // Extract grip_id from key
            let grip_id = key_str.trim_start_matches(&prefix);
            if let Some(grip) = self.get_grip(grip_id)? {
                grips.push(grip);
            }
        }

        Ok(grips)
    }

    /// Delete a grip and its index entry.
    pub fn delete_grip(&self, grip_id: &str) -> Result<(), StorageError> {
        let grips_cf = self
            .db
            .cf_handle(CF_GRIPS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_GRIPS.to_string()))?;

        // Get grip first to find index entry
        if let Some(grip) = self.get_grip(grip_id)? {
            // Delete index entry if exists
            if let Some(ref node_id) = grip.toc_node_id {
                let index_key = format!("node:{}:{}", node_id, grip_id);
                self.db.delete_cf(&grips_cf, index_key.as_bytes())?;
            }
        }

        // Delete grip itself
        self.db.delete_cf(&grips_cf, grip_id.as_bytes())?;

        debug!(grip_id = %grip_id, "Deleted grip");
        Ok(())
    }

    // ===== Generic Column Family Operations =====

    /// Put a value into a specific column family.
    ///
    /// This is a low-level method for use by other crates that manage their own
    /// column families (e.g., memory-topics).
    pub fn put(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    /// Get a value from a specific column family.
    ///
    /// This is a low-level method for use by other crates that manage their own
    /// column families (e.g., memory-topics).
    pub fn get(&self, cf_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))?;
        let result = self.db.get_cf(&cf, key)?;
        Ok(result)
    }

    /// Delete a value from a specific column family.
    ///
    /// This is a low-level method for use by other crates that manage their own
    /// column families (e.g., memory-topics).
    pub fn delete(&self, cf_name: &str, key: &[u8]) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))?;
        self.db.delete_cf(&cf, key)?;
        Ok(())
    }

    /// Iterate over entries with a given prefix in a column family.
    ///
    /// Returns an iterator of (key, value) pairs.
    /// This is a low-level method for use by other crates that manage their own
    /// column families (e.g., memory-topics).
    #[allow(clippy::type_complexity)]
    pub fn prefix_iterator(
        &self,
        cf_name: &str,
        prefix: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, StorageError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))?;

        let mut results = Vec::new();
        let iter = self
            .db
            .iterator_cf(&cf, IteratorMode::From(prefix, Direction::Forward));

        for item in iter {
            let (key, value) = item?;
            // Stop if we've passed the prefix
            if !key.starts_with(prefix) {
                break;
            }
            results.push((key.to_vec(), value.to_vec()));
        }

        Ok(results)
    }

    // ===== Admin Operations =====

    /// Trigger manual compaction on all column families.
    ///
    /// Per CLI-03: Admin commands include compact.
    pub fn compact(&self) -> Result<(), StorageError> {
        info!("Starting full compaction...");
        self.db.compact_range::<&[u8], &[u8]>(None, None);

        for cf_name in &[
            CF_EVENTS,
            CF_TOC_NODES,
            CF_TOC_LATEST,
            CF_GRIPS,
            CF_OUTBOX,
            CF_CHECKPOINTS,
        ] {
            if let Some(cf) = self.db.cf_handle(cf_name) {
                self.db.compact_range_cf::<&[u8], &[u8]>(&cf, None, None);
            }
        }
        info!("Compaction complete");
        Ok(())
    }

    /// Trigger compaction on a specific column family.
    pub fn compact_cf(&self, cf_name: &str) -> Result<(), StorageError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))?;
        info!(cf = %cf_name, "Starting compaction...");
        self.db.compact_range_cf::<&[u8], &[u8]>(&cf, None, None);
        info!(cf = %cf_name, "Compaction complete");
        Ok(())
    }

    /// Get database statistics.
    ///
    /// Per CLI-03: Admin commands include status.
    pub fn get_stats(&self) -> Result<StorageStats, StorageError> {
        let mut stats = StorageStats::default();

        // Count events
        if let Some(cf) = self.db.cf_handle(CF_EVENTS) {
            stats.event_count = self.count_cf_entries(cf)?;
        }

        // Count TOC nodes
        if let Some(cf) = self.db.cf_handle(CF_TOC_NODES) {
            stats.toc_node_count = self.count_cf_entries(cf)?;
        }

        // Count grips
        if let Some(cf) = self.db.cf_handle(CF_GRIPS) {
            stats.grip_count = self.count_cf_entries(cf)?;
        }

        // Count outbox entries
        if let Some(cf) = self.db.cf_handle(CF_OUTBOX) {
            stats.outbox_count = self.count_cf_entries(cf)?;
        }

        // Get disk usage
        stats.disk_usage_bytes = self.get_disk_usage()?;

        Ok(stats)
    }

    fn count_cf_entries(&self, cf: &rocksdb::ColumnFamily) -> Result<u64, StorageError> {
        let mut count = 0u64;
        let iter = self.db.iterator_cf(cf, IteratorMode::Start);
        for item in iter {
            item?;
            count += 1;
        }
        Ok(count)
    }

    fn get_disk_usage(&self) -> Result<u64, StorageError> {
        let path = self.db.path();
        let mut total_size = 0u64;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }
}

/// Statistics about the storage.
#[derive(Debug, Default)]
pub struct StorageStats {
    /// Number of events stored
    pub event_count: u64,
    /// Number of TOC nodes
    pub toc_node_count: u64,
    /// Number of grips
    pub grip_count: u64,
    /// Number of pending outbox entries
    pub outbox_count: u64,
    /// Total disk usage in bytes
    pub disk_usage_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_open_creates_column_families() {
        let (storage, _temp) = create_test_storage();
        // Verify all CFs exist by trying to get handles
        for cf_name in ALL_CF_NAMES {
            assert!(
                storage.db.cf_handle(cf_name).is_some(),
                "CF {} should exist",
                cf_name
            );
        }
    }

    #[test]
    fn test_put_and_get_event() {
        let (storage, _temp) = create_test_storage();

        let event_id = ulid::Ulid::new().to_string();
        let event_bytes = b"test event data";
        let outbox_bytes = b"outbox entry";

        let (key, created) = storage
            .put_event(&event_id, event_bytes, outbox_bytes)
            .unwrap();
        assert!(created);
        assert_eq!(key.event_id(), event_id);

        let retrieved = storage.get_event(&event_id).unwrap();
        assert_eq!(retrieved, Some(event_bytes.to_vec()));
    }

    #[test]
    fn test_idempotent_put() {
        let (storage, _temp) = create_test_storage();

        let event_id = ulid::Ulid::new().to_string();
        let event_bytes = b"test event data";
        let outbox_bytes = b"outbox entry";

        let (_, created1) = storage
            .put_event(&event_id, event_bytes, outbox_bytes)
            .unwrap();
        let (_, created2) = storage
            .put_event(&event_id, event_bytes, outbox_bytes)
            .unwrap();

        assert!(created1);
        assert!(!created2); // Second write should be idempotent
    }

    #[test]
    fn test_get_events_in_range() {
        let (storage, _temp) = create_test_storage();

        // Create events at different timestamps
        let ts1 = 1000i64;
        let ts2 = 2000i64;
        let ts3 = 3000i64;

        let ulid1 = ulid::Ulid::from_parts(ts1 as u64, rand::random());
        let ulid2 = ulid::Ulid::from_parts(ts2 as u64, rand::random());
        let ulid3 = ulid::Ulid::from_parts(ts3 as u64, rand::random());

        storage
            .put_event(&ulid1.to_string(), b"event1", b"outbox1")
            .unwrap();
        storage
            .put_event(&ulid2.to_string(), b"event2", b"outbox2")
            .unwrap();
        storage
            .put_event(&ulid3.to_string(), b"event3", b"outbox3")
            .unwrap();

        // Query range [1500, 2500) should only get event2
        let results = storage.get_events_in_range(1500, 2500).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, b"event2");
    }

    #[test]
    fn test_checkpoint_roundtrip() {
        let (storage, _temp) = create_test_storage();

        let job_name = "test_job";
        let checkpoint_data = b"checkpoint state";

        storage.put_checkpoint(job_name, checkpoint_data).unwrap();
        let retrieved = storage.get_checkpoint(job_name).unwrap();

        assert_eq!(retrieved, Some(checkpoint_data.to_vec()));
    }

    #[test]
    fn test_toc_node_roundtrip() {
        let (storage, _temp) = create_test_storage();

        let node = memory_types::TocNode::new(
            "toc:day:2024-01-15".to_string(),
            memory_types::TocLevel::Day,
            "Monday, January 15, 2024".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        storage.put_toc_node(&node).unwrap();
        let retrieved = storage.get_toc_node("toc:day:2024-01-15").unwrap();

        assert!(retrieved.is_some());
        let retrieved_node = retrieved.unwrap();
        assert_eq!(retrieved_node.node_id, node.node_id);
        assert_eq!(retrieved_node.title, node.title);
        assert_eq!(retrieved_node.version, 1);
    }

    #[test]
    fn test_toc_node_versioning() {
        let (storage, _temp) = create_test_storage();

        let mut node = memory_types::TocNode::new(
            "toc:day:2024-01-16".to_string(),
            memory_types::TocLevel::Day,
            "Tuesday".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        // First version
        storage.put_toc_node(&node).unwrap();

        // Update and store again
        node.title = "Tuesday (updated)".to_string();
        storage.put_toc_node(&node).unwrap();

        // Should get latest version
        let retrieved = storage.get_toc_node("toc:day:2024-01-16").unwrap().unwrap();
        assert_eq!(retrieved.title, "Tuesday (updated)");
        assert_eq!(retrieved.version, 2);
    }

    #[test]
    fn test_toc_node_not_found() {
        let (storage, _temp) = create_test_storage();

        let result = storage.get_toc_node("toc:nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_child_nodes_empty() {
        let (storage, _temp) = create_test_storage();

        let result = storage.get_child_nodes("toc:nonexistent").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_child_nodes() {
        let (storage, _temp) = create_test_storage();

        // Create child segment node
        let child = memory_types::TocNode::new(
            "toc:segment:2024-01-15:abc123".to_string(),
            memory_types::TocLevel::Segment,
            "Conversation about testing".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        storage.put_toc_node(&child).unwrap();

        // Create parent day node with child reference
        let mut parent = memory_types::TocNode::new(
            "toc:day:2024-01-15".to_string(),
            memory_types::TocLevel::Day,
            "January 15".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        parent
            .child_node_ids
            .push("toc:segment:2024-01-15:abc123".to_string());
        storage.put_toc_node(&parent).unwrap();

        // Get children
        let children = storage.get_child_nodes("toc:day:2024-01-15").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].node_id, "toc:segment:2024-01-15:abc123");
    }

    #[test]
    fn test_grip_roundtrip() {
        let (storage, _temp) = create_test_storage();

        let grip = memory_types::Grip::new(
            "grip:1706540400000:test123".to_string(),
            "User asked about authentication".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            chrono::Utc::now(),
            "segment_summarizer".to_string(),
        );

        storage.put_grip(&grip).unwrap();
        let retrieved = storage.get_grip("grip:1706540400000:test123").unwrap();

        assert!(retrieved.is_some());
        let retrieved_grip = retrieved.unwrap();
        assert_eq!(retrieved_grip.excerpt, grip.excerpt);
    }

    #[test]
    fn test_grip_with_node_index() {
        let (storage, _temp) = create_test_storage();

        let grip = memory_types::Grip::new(
            "grip:1706540400000:test456".to_string(),
            "Discussed JWT tokens".to_string(),
            "event-010".to_string(),
            "event-015".to_string(),
            chrono::Utc::now(),
            "segment_summarizer".to_string(),
        )
        .with_toc_node("toc:day:2024-01-29".to_string());

        storage.put_grip(&grip).unwrap();

        let grips = storage.get_grips_for_node("toc:day:2024-01-29").unwrap();
        assert_eq!(grips.len(), 1);
        assert_eq!(grips[0].grip_id, "grip:1706540400000:test456");
    }

    #[test]
    fn test_grip_not_found() {
        let (storage, _temp) = create_test_storage();

        let result = storage.get_grip("grip:nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_grip() {
        let (storage, _temp) = create_test_storage();

        let grip = memory_types::Grip::new(
            "grip:1706540400000:del123".to_string(),
            "Test excerpt".to_string(),
            "event-001".to_string(),
            "event-002".to_string(),
            chrono::Utc::now(),
            "test".to_string(),
        )
        .with_toc_node("toc:day:2024-01-30".to_string());

        storage.put_grip(&grip).unwrap();
        assert!(storage
            .get_grip("grip:1706540400000:del123")
            .unwrap()
            .is_some());

        storage.delete_grip("grip:1706540400000:del123").unwrap();
        assert!(storage
            .get_grip("grip:1706540400000:del123")
            .unwrap()
            .is_none());

        // Index should also be deleted
        let grips = storage.get_grips_for_node("toc:day:2024-01-30").unwrap();
        assert!(grips.is_empty());
    }

    // ==================== Outbox Tests ====================

    #[test]
    fn test_get_outbox_entries_empty() {
        let (storage, _temp) = create_test_storage();

        let entries = storage.get_outbox_entries(0, 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_get_outbox_entries_after_event() {
        let (storage, _temp) = create_test_storage();

        // Create an event which also creates an outbox entry
        let event_id = ulid::Ulid::new().to_string();
        let outbox_entry = memory_types::OutboxEntry::for_index(event_id.clone(), 1000);
        let outbox_bytes = outbox_entry.to_bytes().unwrap();

        storage
            .put_event(&event_id, b"test event", &outbox_bytes)
            .unwrap();

        // Read outbox entries
        let entries = storage.get_outbox_entries(0, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, 0); // First sequence is 0
        assert_eq!(entries[0].1.event_id, event_id);
    }

    #[test]
    fn test_get_outbox_entries_with_limit() {
        let (storage, _temp) = create_test_storage();

        // Create multiple events
        for i in 0..5 {
            let event_id = ulid::Ulid::new().to_string();
            let outbox_entry = memory_types::OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&event_id, b"test", &outbox_bytes)
                .unwrap();
        }

        // Read with limit
        let entries = storage.get_outbox_entries(0, 3).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, 0);
        assert_eq!(entries[1].0, 1);
        assert_eq!(entries[2].0, 2);
    }

    #[test]
    fn test_get_outbox_entries_from_offset() {
        let (storage, _temp) = create_test_storage();

        // Create multiple events
        let mut event_ids = Vec::new();
        for i in 0..5 {
            let event_id = ulid::Ulid::new().to_string();
            event_ids.push(event_id.clone());
            let outbox_entry = memory_types::OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&event_id, b"test", &outbox_bytes)
                .unwrap();
        }

        // Read starting from sequence 2
        let entries = storage.get_outbox_entries(2, 10).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, 2);
        assert_eq!(entries[0].1.event_id, event_ids[2]);
        assert_eq!(entries[1].0, 3);
        assert_eq!(entries[2].0, 4);
    }

    #[test]
    fn test_delete_outbox_entries() {
        let (storage, _temp) = create_test_storage();

        // Create multiple events
        for i in 0..5 {
            let event_id = ulid::Ulid::new().to_string();
            let outbox_entry = memory_types::OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&event_id, b"test", &outbox_bytes)
                .unwrap();
        }

        // Delete entries up to sequence 2 (inclusive)
        let deleted = storage.delete_outbox_entries(2).unwrap();
        assert_eq!(deleted, 3); // Sequences 0, 1, 2

        // Verify remaining entries
        let entries = storage.get_outbox_entries(0, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, 3);
        assert_eq!(entries[1].0, 4);
    }

    #[test]
    fn test_delete_outbox_entries_none() {
        let (storage, _temp) = create_test_storage();

        // Delete from empty outbox
        let deleted = storage.delete_outbox_entries(10).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_delete_outbox_entries_all() {
        let (storage, _temp) = create_test_storage();

        // Create multiple events
        for i in 0..3 {
            let event_id = ulid::Ulid::new().to_string();
            let outbox_entry = memory_types::OutboxEntry::for_index(event_id.clone(), i * 1000);
            let outbox_bytes = outbox_entry.to_bytes().unwrap();
            storage
                .put_event(&event_id, b"test", &outbox_bytes)
                .unwrap();
        }

        // Delete all entries
        let deleted = storage.delete_outbox_entries(100).unwrap();
        assert_eq!(deleted, 3);

        // Verify all gone
        let entries = storage.get_outbox_entries(0, 10).unwrap();
        assert!(entries.is_empty());
    }
}
