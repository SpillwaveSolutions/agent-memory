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

use crate::column_families::{build_cf_descriptors, ALL_CF_NAMES, CF_EVENTS, CF_OUTBOX, CF_CHECKPOINTS};
use crate::error::StorageError;
use crate::keys::{EventKey, OutboxKey, CheckpointKey};

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
}
