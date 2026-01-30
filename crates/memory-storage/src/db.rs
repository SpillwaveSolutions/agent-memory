//! RocksDB wrapper for agent-memory storage.
//!
//! Provides:
//! - Database open/close with column family setup
//! - Atomic write batches (event + outbox per ING-05)
//! - Single-key and range reads
//! - Idempotent writes (ING-03)

use rocksdb::{DB, Options, WriteBatch, IteratorMode, Direction};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};

use crate::column_families::{build_cf_descriptors, ALL_CF_NAMES, CF_EVENTS, CF_OUTBOX, CF_CHECKPOINTS, CF_TOC_NODES, CF_TOC_LATEST};
use crate::error::StorageError;
use crate::keys::{EventKey, OutboxKey, CheckpointKey};

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
        let cf = db.cf_handle(CF_OUTBOX)
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
        let events_cf = self.db.cf_handle(CF_EVENTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EVENTS.to_string()))?;
        let outbox_cf = self.db.cf_handle(CF_OUTBOX)
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
        debug!("Stored event {} with outbox seq {}", event_id, outbox_key.sequence);

        Ok((event_key, true))
    }

    /// Get an event by its event_id
    pub fn get_event(&self, event_id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let events_cf = self.db.cf_handle(CF_EVENTS)
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
        let events_cf = self.db.cf_handle(CF_EVENTS)
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
    pub fn put_checkpoint(&self, job_name: &str, checkpoint_bytes: &[u8]) -> Result<(), StorageError> {
        let cf = self.db.cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_CHECKPOINTS.to_string()))?;

        let key = CheckpointKey::new(job_name);
        self.db.put_cf(&cf, key.to_bytes(), checkpoint_bytes)?;
        Ok(())
    }

    /// Get a checkpoint for crash recovery (STOR-03)
    pub fn get_checkpoint(&self, job_name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let cf = self.db.cf_handle(CF_CHECKPOINTS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_CHECKPOINTS.to_string()))?;

        let key = CheckpointKey::new(job_name);
        let result = self.db.get_cf(&cf, key.to_bytes())?;
        Ok(result)
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
        let nodes_cf = self.db.cf_handle(CF_TOC_NODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_NODES.to_string()))?;
        let latest_cf = self.db.cf_handle(CF_TOC_LATEST)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_LATEST.to_string()))?;

        // Get current version
        let latest_key = format!("latest:{}", node.node_id);
        let current_version = self.db.get_cf(&latest_cf, &latest_key)?
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

        let node_bytes = versioned_node.to_bytes()
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Atomic write: node + latest pointer
        let mut batch = WriteBatch::default();
        batch.put_cf(&nodes_cf, versioned_key.as_bytes(), &node_bytes);
        batch.put_cf(&latest_cf, latest_key.as_bytes(), &new_version.to_be_bytes());

        self.db.write(batch)?;

        debug!(node_id = %node.node_id, version = new_version, "Stored TOC node");
        Ok(())
    }

    /// Get the latest version of a TOC node.
    pub fn get_toc_node(&self, node_id: &str) -> Result<Option<memory_types::TocNode>, StorageError> {
        let nodes_cf = self.db.cf_handle(CF_TOC_NODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_NODES.to_string()))?;
        let latest_cf = self.db.cf_handle(CF_TOC_LATEST)
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
        let nodes_cf = self.db.cf_handle(CF_TOC_NODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TOC_NODES.to_string()))?;
        let latest_cf = self.db.cf_handle(CF_TOC_LATEST)
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
                let versioned_key = format!("{}:v{:06}", node_id, version);

                if let Some(bytes) = self.db.get_cf(&nodes_cf, versioned_key.as_bytes())? {
                    let node = memory_types::TocNode::from_bytes(&bytes)
                        .map_err(|e| StorageError::Serialization(e.to_string()))?;

                    // Filter by time range if specified
                    let include = match (start_time, end_time) {
                        (Some(start), Some(end)) => node.end_time >= start && node.start_time <= end,
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
    pub fn get_child_nodes(&self, parent_node_id: &str) -> Result<Vec<memory_types::TocNode>, StorageError> {
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
            assert!(storage.db.cf_handle(cf_name).is_some(), "CF {} should exist", cf_name);
        }
    }

    #[test]
    fn test_put_and_get_event() {
        let (storage, _temp) = create_test_storage();

        let event_id = ulid::Ulid::new().to_string();
        let event_bytes = b"test event data";
        let outbox_bytes = b"outbox entry";

        let (key, created) = storage.put_event(&event_id, event_bytes, outbox_bytes).unwrap();
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

        let (_, created1) = storage.put_event(&event_id, event_bytes, outbox_bytes).unwrap();
        let (_, created2) = storage.put_event(&event_id, event_bytes, outbox_bytes).unwrap();

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

        storage.put_event(&ulid1.to_string(), b"event1", b"outbox1").unwrap();
        storage.put_event(&ulid2.to_string(), b"event2", b"outbox2").unwrap();
        storage.put_event(&ulid3.to_string(), b"event3", b"outbox3").unwrap();

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
        parent.child_node_ids.push("toc:segment:2024-01-15:abc123".to_string());
        storage.put_toc_node(&parent).unwrap();

        // Get children
        let children = storage.get_child_nodes("toc:day:2024-01-15").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].node_id, "toc:segment:2024-01-15:abc123");
    }
}
