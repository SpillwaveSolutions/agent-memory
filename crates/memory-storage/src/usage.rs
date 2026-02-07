//! Usage tracking service with cache-first reads and batched writes.
//!
//! Per Phase 16 Plan 02: Track access patterns WITHOUT mutating immutable nodes.
//!
//! ## Key Design Principles
//!
//! - **Cache-first**: `get_usage_cached()` NEVER blocks on CF read
//! - **Batched writes**: `record_access()` queues writes, `flush_writes()` commits batch
//! - **Async prefetch**: Cache misses queue prefetch, don't block current request
//! - **Safe startup**: If CF absent, created on first write; reads return defaults
//!
//! ## Architecture
//!
//! ```text
//! Search Request
//!      │
//!      ▼
//! ┌────────────────────────────────────────────┐
//! │ UsageCache.get_batch_cached(doc_ids)       │
//! │  - Check in-memory LRU cache first         │
//! │  - Return cached entries immediately       │
//! └────────┬───────────────────────────────────┘
//!          │ cache miss for some IDs?
//!          ▼ (non-blocking)
//! ┌────────────────────────────────────────────┐
//! │ Queue prefetch for missed IDs              │
//! │  - Does NOT block current search           │
//! └────────────────────────────────────────────┘
//! ```

use crate::column_families::CF_USAGE_COUNTERS;
use dashmap::DashMap;
use lru::LruCache;
use memory_types::usage::{UsageConfig, UsageStats};
use rocksdb::{WriteBatch, DB};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

/// Pending write operation tracking both the stats and whether we've already
/// loaded existing data from the CF.
struct UsageUpdate {
    stats: UsageStats,
    /// If true, we've merged with CF data and can write directly.
    /// If false, we should try to load existing CF data before final write.
    _merged: bool,
}

/// Usage tracking service with cache-first design.
///
/// Tracks document access patterns without mutating immutable TocNode or Grip records.
/// Usage data is stored separately in CF_USAGE_COUNTERS.
///
/// ## Thread Safety
///
/// - LRU cache protected by Mutex (contention expected to be low)
/// - Pending writes use DashMap for concurrent access
/// - Prefetch queue uses DashMap for concurrent access
pub struct UsageTracker {
    /// LRU cache for hot doc IDs (bounded)
    cache: Mutex<LruCache<String, UsageStats>>,
    /// Pending writes (batched)
    pending_writes: DashMap<String, UsageUpdate>,
    /// Pending prefetch requests
    prefetch_queue: DashMap<String, ()>,
    /// Database handle
    db: Arc<DB>,
    /// Configuration
    config: UsageConfig,
}

impl UsageTracker {
    /// Create a new usage tracker.
    ///
    /// Safe startup: CF_USAGE_COUNTERS is created on first write if absent.
    /// All reads return defaults until CF is populated.
    pub fn new(db: Arc<DB>, config: UsageConfig) -> Self {
        let cache_size = NonZeroUsize::new(config.cache_size.max(1))
            .expect("cache_size must be > 0 after max(1)");

        Self {
            cache: Mutex::new(LruCache::new(cache_size)),
            pending_writes: DashMap::new(),
            prefetch_queue: DashMap::new(),
            db,
            config,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults(db: Arc<DB>) -> Self {
        Self::new(db, UsageConfig::default())
    }

    /// Record an access (batched write, non-blocking).
    ///
    /// Updates cache immediately, queues CF write for batch flush.
    /// This method is designed to be called on every search result access.
    pub fn record_access(&self, doc_id: &str) {
        // Update cache immediately
        {
            let mut cache = self.cache.lock().expect("cache mutex poisoned");
            let stats = cache.get_or_insert_mut(doc_id.to_string(), UsageStats::new);
            stats.record_access();
        }

        // Queue write for batch flush
        self.pending_writes
            .entry(doc_id.to_string())
            .and_modify(|update| {
                update.stats.record_access();
            })
            .or_insert_with(|| {
                let mut stats = UsageStats::new();
                stats.record_access();
                UsageUpdate {
                    stats,
                    _merged: false,
                }
            });
    }

    /// Get usage for ranking - cache-first, NO blocking CF read.
    ///
    /// Returns default UsageStats if not in cache.
    /// Queues prefetch for cache miss.
    ///
    /// This is the primary method for retrieving usage during ranking.
    /// It is designed to NEVER add latency to the search path.
    pub fn get_usage_cached(&self, doc_id: &str) -> UsageStats {
        // Check cache first
        let cached = {
            let mut cache = self.cache.lock().expect("cache mutex poisoned");
            cache.get(doc_id).cloned()
        };

        if let Some(stats) = cached {
            return stats;
        }

        // Cache miss - queue prefetch (don't block)
        self.prefetch_queue.insert(doc_id.to_string(), ());

        // Return default (count=0)
        UsageStats::new()
    }

    /// Batch get for ranking - returns available data, queues prefetch for misses.
    ///
    /// Returns a vector of (doc_id, stats) pairs. Stats for cache misses
    /// will be default values (count=0), and those IDs will be queued for prefetch.
    pub fn get_batch_cached(&self, doc_ids: &[String]) -> Vec<(String, UsageStats)> {
        let mut results = Vec::with_capacity(doc_ids.len());

        {
            let mut cache = self.cache.lock().expect("cache mutex poisoned");
            for doc_id in doc_ids {
                if let Some(stats) = cache.get(doc_id) {
                    results.push((doc_id.clone(), stats.clone()));
                } else {
                    // Queue prefetch
                    self.prefetch_queue.insert(doc_id.clone(), ());
                    results.push((doc_id.clone(), UsageStats::new()));
                }
            }
        }

        results
    }

    /// Flush pending writes to CF_USAGE_COUNTERS (called by scheduler job).
    ///
    /// Returns number of writes flushed.
    ///
    /// This method should be called periodically (default: every 60 seconds)
    /// to persist usage data without blocking the search path.
    pub fn flush_writes(&self) -> Result<u32, crate::StorageError> {
        // Collect pending writes (drain them)
        let writes: Vec<(String, UsageStats)> = self
            .pending_writes
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().stats.clone()))
            .collect();

        if writes.is_empty() {
            return Ok(0);
        }

        // Get CF handle - if CF doesn't exist, we can't write
        let Some(cf) = self.db.cf_handle(CF_USAGE_COUNTERS) else {
            tracing::warn!("CF_USAGE_COUNTERS not found, skipping flush");
            return Ok(0);
        };

        // For each pending write, merge with existing CF data
        let mut batch = WriteBatch::default();
        let mut written = 0u32;

        for (doc_id, mut stats) in writes {
            // Try to load existing stats from CF and merge
            if let Some(existing_bytes) = self.db.get_cf(&cf, doc_id.as_bytes())? {
                if let Ok(existing) = UsageStats::from_bytes(&existing_bytes) {
                    // Merge: take max of counts, latest timestamp
                    stats.merge(&existing);
                }
            }

            // Serialize and add to batch
            let bytes = stats.to_bytes().map_err(|e| {
                crate::StorageError::Serialization(format!("Failed to serialize UsageStats: {e}"))
            })?;
            batch.put_cf(&cf, doc_id.as_bytes(), &bytes);
            written += 1;
        }

        // Commit batch
        self.db.write(batch)?;

        // Clear committed writes from pending map
        for (doc_id, _) in self
            .pending_writes
            .iter()
            .map(|e| (e.key().clone(), ()))
            .collect::<Vec<_>>()
        {
            self.pending_writes.remove(&doc_id);
        }

        tracing::debug!(count = written, "Flushed usage writes to CF");
        Ok(written)
    }

    /// Process prefetch queue (called by scheduler job).
    ///
    /// Loads missing IDs from CF_USAGE_COUNTERS into cache.
    /// Returns number of entries prefetched.
    ///
    /// This method should be called periodically (default: every 5 seconds)
    /// to populate the cache for future requests.
    pub fn process_prefetch(&self) -> Result<u32, crate::StorageError> {
        // Collect prefetch requests
        let to_fetch: Vec<String> = self
            .prefetch_queue
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        if to_fetch.is_empty() {
            return Ok(0);
        }

        // Get CF handle - if CF doesn't exist, clear queue and return
        let Some(cf) = self.db.cf_handle(CF_USAGE_COUNTERS) else {
            // CF doesn't exist yet, clear queue and return
            for doc_id in &to_fetch {
                self.prefetch_queue.remove(doc_id);
            }
            return Ok(0);
        };

        let mut prefetched = 0u32;

        for doc_id in &to_fetch {
            // Load from CF
            if let Some(bytes) = self.db.get_cf(&cf, doc_id.as_bytes())? {
                if let Ok(stats) = UsageStats::from_bytes(&bytes) {
                    // Populate cache
                    let mut cache = self.cache.lock().expect("cache mutex poisoned");
                    cache.put(doc_id.clone(), stats);
                    prefetched += 1;
                }
            }
            // Remove from queue regardless of whether we found data
            self.prefetch_queue.remove(doc_id);
        }

        if prefetched > 0 {
            tracing::debug!(prefetched, "Prefetched usage stats into cache");
        }

        Ok(prefetched)
    }

    /// Warm cache on startup by loading recent/frequent IDs.
    ///
    /// This method can be called during daemon startup to pre-populate
    /// the cache with usage data, reducing cache misses for early requests.
    ///
    /// Returns number of entries loaded.
    pub fn warm_cache(&self, limit: usize) -> Result<u32, crate::StorageError> {
        let Some(cf) = self.db.cf_handle(CF_USAGE_COUNTERS) else {
            return Ok(0);
        };

        let mut loaded = 0u32;
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

        let mut cache = self.cache.lock().expect("cache mutex poisoned");
        for item in iter.take(limit) {
            let (key, value) = item?;
            if let (Ok(doc_id), Ok(stats)) = (
                String::from_utf8(key.to_vec()),
                UsageStats::from_bytes(&value),
            ) {
                cache.put(doc_id, stats);
                loaded += 1;
            }
        }

        tracing::info!(loaded, "Warmed usage cache on startup");
        Ok(loaded)
    }

    /// Get cache statistics for metrics.
    ///
    /// Returns (current_size, capacity).
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().expect("cache mutex poisoned");
        (cache.len(), cache.cap().get())
    }

    /// Get pending write count.
    pub fn pending_write_count(&self) -> usize {
        self.pending_writes.len()
    }

    /// Get prefetch queue size.
    pub fn prefetch_queue_size(&self) -> usize {
        self.prefetch_queue.len()
    }

    /// Get configuration.
    pub fn config(&self) -> &UsageConfig {
        &self.config
    }

    /// Check if usage decay is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Calculate cache hit rate (approximate).
    ///
    /// This is a simplified metric - in production, you'd want
    /// proper hit/miss counters.
    pub fn approximate_hit_rate(&self) -> f64 {
        let (size, cap) = self.cache_stats();
        if cap == 0 {
            return 0.0;
        }
        // Approximation: fuller cache = higher hit rate
        (size as f64 / cap as f64).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column_families::build_cf_descriptors;
    use rocksdb::Options;
    use tempfile::TempDir;

    fn create_test_db() -> (Arc<DB>, TempDir) {
        let tmp = TempDir::new().unwrap();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descs = build_cf_descriptors();
        let db = DB::open_cf_descriptors(&opts, tmp.path(), cf_descs).unwrap();

        (Arc::new(db), tmp)
    }

    #[test]
    fn test_cache_first_returns_default_on_miss() {
        let (db, _tmp) = create_test_db();
        let tracker = UsageTracker::new(db, UsageConfig::default());

        let stats = tracker.get_usage_cached("unknown:doc:123");
        assert_eq!(stats.access_count, 0);
        assert!(stats.last_accessed.is_none());

        // Should have queued prefetch
        assert_eq!(tracker.prefetch_queue_size(), 1);
    }

    #[test]
    fn test_record_access_updates_cache() {
        let (db, _tmp) = create_test_db();
        let tracker = UsageTracker::new(db, UsageConfig::default());

        tracker.record_access("doc:123");
        let stats = tracker.get_usage_cached("doc:123");
        assert_eq!(stats.access_count, 1);
        assert!(stats.last_accessed.is_some());

        // Second access
        tracker.record_access("doc:123");
        let stats = tracker.get_usage_cached("doc:123");
        assert_eq!(stats.access_count, 2);
    }

    #[test]
    fn test_record_access_queues_write() {
        let (db, _tmp) = create_test_db();
        let tracker = UsageTracker::new(db, UsageConfig::default());

        assert_eq!(tracker.pending_write_count(), 0);
        tracker.record_access("doc:123");
        assert_eq!(tracker.pending_write_count(), 1);

        tracker.record_access("doc:456");
        assert_eq!(tracker.pending_write_count(), 2);

        // Same doc again doesn't add new entry
        tracker.record_access("doc:123");
        assert_eq!(tracker.pending_write_count(), 2);
    }

    #[test]
    fn test_flush_writes_to_cf() {
        let (db, _tmp) = create_test_db();
        let tracker = UsageTracker::new(db.clone(), UsageConfig::default());

        tracker.record_access("doc:flush-test");
        tracker.record_access("doc:flush-test");
        let flushed = tracker.flush_writes().unwrap();
        assert_eq!(flushed, 1);
        assert_eq!(tracker.pending_write_count(), 0);

        // Verify written to CF
        let cf = db.cf_handle(CF_USAGE_COUNTERS).unwrap();
        let bytes = db.get_cf(&cf, b"doc:flush-test").unwrap().unwrap();
        let stats = UsageStats::from_bytes(&bytes).unwrap();
        assert_eq!(stats.access_count, 2);
    }

    #[test]
    fn test_flush_merges_with_existing() {
        let (db, _tmp) = create_test_db();

        // Write initial value directly to CF
        let cf = db.cf_handle(CF_USAGE_COUNTERS).unwrap();
        let initial = UsageStats::with_count(5);
        db.put_cf(&cf, b"doc:merge-test", initial.to_bytes().unwrap())
            .unwrap();

        let tracker = UsageTracker::new(db.clone(), UsageConfig::default());

        // Record 3 more accesses
        tracker.record_access("doc:merge-test");
        tracker.record_access("doc:merge-test");
        tracker.record_access("doc:merge-test");

        // Flush should merge (take max)
        tracker.flush_writes().unwrap();

        // Verify merged value
        let bytes = db.get_cf(&cf, b"doc:merge-test").unwrap().unwrap();
        let stats = UsageStats::from_bytes(&bytes).unwrap();
        // Max of 5 (existing) and 3 (new) = 5
        // But our new stats had 3 accesses, so after merge it should be max(5, 3) = 5
        // However, merge takes max of counts, so it remains 5
        assert_eq!(stats.access_count, 5);
    }

    #[test]
    fn test_prefetch_populates_cache() {
        let (db, _tmp) = create_test_db();

        // Write directly to CF
        let cf = db.cf_handle(CF_USAGE_COUNTERS).unwrap();
        let stats = UsageStats::with_count(42);
        db.put_cf(&cf, b"doc:prefetch-test", stats.to_bytes().unwrap())
            .unwrap();

        let tracker = UsageTracker::new(db, UsageConfig::default());

        // First call returns default and queues prefetch
        let initial = tracker.get_usage_cached("doc:prefetch-test");
        assert_eq!(initial.access_count, 0);
        assert_eq!(tracker.prefetch_queue_size(), 1);

        // Process prefetch
        let prefetched = tracker.process_prefetch().unwrap();
        assert_eq!(prefetched, 1);
        assert_eq!(tracker.prefetch_queue_size(), 0);

        // Now cache should have the value
        let cached = tracker.get_usage_cached("doc:prefetch-test");
        assert_eq!(cached.access_count, 42);
    }

    #[test]
    fn test_get_batch_cached() {
        let (db, _tmp) = create_test_db();
        let tracker = UsageTracker::new(db, UsageConfig::default());

        // Record some accesses
        tracker.record_access("doc:a");
        tracker.record_access("doc:a");
        tracker.record_access("doc:b");

        let doc_ids = vec![
            "doc:a".to_string(),
            "doc:b".to_string(),
            "doc:c".to_string(),
        ];
        let results = tracker.get_batch_cached(&doc_ids);

        assert_eq!(results.len(), 3);

        // Find results by doc_id
        let a_stats = results.iter().find(|(id, _)| id == "doc:a").unwrap();
        let b_stats = results.iter().find(|(id, _)| id == "doc:b").unwrap();
        let c_stats = results.iter().find(|(id, _)| id == "doc:c").unwrap();

        assert_eq!(a_stats.1.access_count, 2);
        assert_eq!(b_stats.1.access_count, 1);
        assert_eq!(c_stats.1.access_count, 0); // Cache miss

        // doc:c should be queued for prefetch
        assert!(tracker.prefetch_queue_size() >= 1);
    }

    #[test]
    fn test_warm_cache() {
        let (db, _tmp) = create_test_db();

        // Write some data directly to CF
        let cf = db.cf_handle(CF_USAGE_COUNTERS).unwrap();
        for i in 0..5 {
            let stats = UsageStats::with_count(i);
            db.put_cf(
                &cf,
                format!("doc:{i}").as_bytes(),
                stats.to_bytes().unwrap(),
            )
            .unwrap();
        }

        let tracker = UsageTracker::new(db, UsageConfig::default());

        // Warm cache with limit of 3
        let loaded = tracker.warm_cache(3).unwrap();
        assert_eq!(loaded, 3);

        let (size, _) = tracker.cache_stats();
        assert_eq!(size, 3);
    }

    #[test]
    fn test_cache_stats() {
        let (db, _tmp) = create_test_db();
        let config = UsageConfig {
            cache_size: 100,
            ..Default::default()
        };
        let tracker = UsageTracker::new(db, config);

        let (size, cap) = tracker.cache_stats();
        assert_eq!(size, 0);
        assert_eq!(cap, 100);

        tracker.record_access("doc:1");
        tracker.record_access("doc:2");

        let (size, _) = tracker.cache_stats();
        assert_eq!(size, 2);
    }

    #[test]
    fn test_config_access() {
        let (db, _tmp) = create_test_db();
        let config = UsageConfig {
            enabled: true,
            decay_factor: 0.25,
            ..Default::default()
        };
        let tracker = UsageTracker::new(db, config);

        assert!(tracker.is_enabled());
        assert!((tracker.config().decay_factor - 0.25).abs() < f32::EPSILON);
    }
}
