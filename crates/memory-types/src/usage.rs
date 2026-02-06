//! Usage tracking types for access pattern analysis.
//!
//! Per Phase 16 Plan 02: Track access patterns WITHOUT mutating immutable nodes.
//! Usage data stored separately in CF_USAGE_COUNTERS column family.
//!
//! ## Design Principles
//!
//! - Cache-first reads return cached data immediately without blocking on CF read
//! - Pending writes are batched and flushed periodically (default: 60s)
//! - Cache misses return default (count=0) and queue prefetch
//! - LRU cache bounded to configurable size (default: 10K entries)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Usage statistics for a document (TOC node, grip, topic).
///
/// This data is stored separately in CF_USAGE_COUNTERS to preserve
/// the immutability of TocNode and Grip records.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UsageStats {
    /// Number of times this document was accessed
    pub access_count: u32,

    /// Last access timestamp (None if never accessed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<DateTime<Utc>>,
}

impl UsageStats {
    /// Create new usage stats with zero access.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create usage stats with initial values.
    pub fn with_count(access_count: u32) -> Self {
        Self {
            access_count,
            last_accessed: if access_count > 0 {
                Some(Utc::now())
            } else {
                None
            },
        }
    }

    /// Increment access count and update timestamp.
    pub fn record_access(&mut self) {
        self.access_count = self.access_count.saturating_add(1);
        self.last_accessed = Some(Utc::now());
    }

    /// Merge with another UsageStats, taking the maximum values.
    pub fn merge(&mut self, other: &UsageStats) {
        self.access_count = self.access_count.max(other.access_count);
        self.last_accessed = match (self.last_accessed, other.last_accessed) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Configuration for usage tracking and decay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageConfig {
    /// Whether usage decay is enabled in ranking.
    /// OFF by default until validated.
    #[serde(default)]
    pub enabled: bool,

    /// Decay factor for usage penalty (higher = more aggressive).
    /// Formula: 1 / (1 + decay_factor * access_count)
    #[serde(default = "default_decay_factor")]
    pub decay_factor: f32,

    /// How often to flush pending writes to CF (seconds).
    #[serde(default = "default_flush_interval")]
    pub flush_interval_secs: u64,

    /// How often to process prefetch queue (seconds).
    #[serde(default = "default_prefetch_interval")]
    pub prefetch_interval_secs: u64,

    /// LRU cache size (number of entries).
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
}

fn default_decay_factor() -> f32 {
    0.15
}

fn default_flush_interval() -> u64 {
    60
}

fn default_prefetch_interval() -> u64 {
    5
}

fn default_cache_size() -> usize {
    10_000
}

impl Default for UsageConfig {
    fn default() -> Self {
        Self {
            enabled: false, // OFF by default until validated
            decay_factor: default_decay_factor(),
            flush_interval_secs: default_flush_interval(),
            prefetch_interval_secs: default_prefetch_interval(),
            cache_size: default_cache_size(),
        }
    }
}

impl UsageConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), String> {
        if self.decay_factor <= 0.0 {
            return Err(format!(
                "decay_factor must be positive, got {}",
                self.decay_factor
            ));
        }
        if self.cache_size == 0 {
            return Err("cache_size must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Calculate usage penalty for ranking.
///
/// Returns value between 0.0 and 1.0:
/// - 1.0 = no penalty (access_count = 0)
/// - Approaches 0.0 as access_count increases
///
/// Formula: 1 / (1 + decay_factor * access_count)
pub fn usage_penalty(access_count: u32, decay_factor: f32) -> f32 {
    1.0 / (1.0 + decay_factor * access_count as f32)
}

/// Apply usage penalty to a score.
///
/// Returns: score * usage_penalty(access_count, decay_factor)
pub fn apply_usage_penalty(score: f32, access_count: u32, decay_factor: f32) -> f32 {
    score * usage_penalty(access_count, decay_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_stats_default() {
        let stats = UsageStats::new();
        assert_eq!(stats.access_count, 0);
        assert!(stats.last_accessed.is_none());
    }

    #[test]
    fn test_usage_stats_with_count() {
        let stats = UsageStats::with_count(5);
        assert_eq!(stats.access_count, 5);
        assert!(stats.last_accessed.is_some());

        let empty = UsageStats::with_count(0);
        assert_eq!(empty.access_count, 0);
        assert!(empty.last_accessed.is_none());
    }

    #[test]
    fn test_usage_stats_record_access() {
        let mut stats = UsageStats::new();
        stats.record_access();
        assert_eq!(stats.access_count, 1);
        assert!(stats.last_accessed.is_some());

        stats.record_access();
        assert_eq!(stats.access_count, 2);
    }

    #[test]
    fn test_usage_stats_saturating_add() {
        let mut stats = UsageStats {
            access_count: u32::MAX,
            last_accessed: None,
        };
        stats.record_access();
        assert_eq!(stats.access_count, u32::MAX); // Saturates, doesn't overflow
    }

    #[test]
    fn test_usage_stats_merge() {
        let mut a = UsageStats::with_count(5);
        let b = UsageStats::with_count(10);
        a.merge(&b);
        assert_eq!(a.access_count, 10);

        let mut c = UsageStats::with_count(15);
        let d = UsageStats::with_count(3);
        c.merge(&d);
        assert_eq!(c.access_count, 15);
    }

    #[test]
    fn test_usage_stats_serialization() {
        let mut stats = UsageStats::new();
        stats.record_access();

        let bytes = stats.to_bytes().unwrap();
        let decoded = UsageStats::from_bytes(&bytes).unwrap();

        assert_eq!(stats.access_count, decoded.access_count);
        assert!(decoded.last_accessed.is_some());
    }

    #[test]
    fn test_usage_stats_serialization_roundtrip() {
        let stats = UsageStats {
            access_count: 42,
            last_accessed: Some(Utc::now()),
        };

        let bytes = stats.to_bytes().unwrap();
        let decoded = UsageStats::from_bytes(&bytes).unwrap();

        assert_eq!(stats.access_count, decoded.access_count);
    }

    #[test]
    fn test_usage_config_default() {
        let config = UsageConfig::default();
        assert!(!config.enabled);
        assert!((config.decay_factor - 0.15).abs() < f32::EPSILON);
        assert_eq!(config.flush_interval_secs, 60);
        assert_eq!(config.prefetch_interval_secs, 5);
        assert_eq!(config.cache_size, 10_000);
    }

    #[test]
    fn test_usage_config_validate() {
        let valid = UsageConfig::default();
        assert!(valid.validate().is_ok());

        let invalid_decay = UsageConfig {
            decay_factor: 0.0,
            ..Default::default()
        };
        assert!(invalid_decay.validate().is_err());

        let invalid_cache = UsageConfig {
            cache_size: 0,
            ..Default::default()
        };
        assert!(invalid_cache.validate().is_err());
    }

    #[test]
    fn test_usage_penalty_zero_access() {
        let penalty = usage_penalty(0, 0.15);
        assert!((penalty - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_usage_penalty_decreases_with_access() {
        let p0 = usage_penalty(0, 0.15);
        let p1 = usage_penalty(1, 0.15);
        let p10 = usage_penalty(10, 0.15);
        let p100 = usage_penalty(100, 0.15);

        assert!(p1 < p0);
        assert!(p10 < p1);
        assert!(p100 < p10);
    }

    #[test]
    fn test_usage_penalty_calculation() {
        // 1 / (1 + 0.15 * 10) = 1 / 2.5 = 0.4
        let penalty = usage_penalty(10, 0.15);
        assert!((penalty - 0.4).abs() < f32::EPSILON);

        // 1 / (1 + 0.15 * 100) = 1 / 16 = 0.0625
        let penalty = usage_penalty(100, 0.15);
        assert!((penalty - 0.0625).abs() < 0.0001);
    }

    #[test]
    fn test_apply_usage_penalty() {
        let score = apply_usage_penalty(1.0, 0, 0.15);
        assert!((score - 1.0).abs() < f32::EPSILON);

        let score = apply_usage_penalty(0.8, 10, 0.15);
        // 0.8 * 0.4 = 0.32
        assert!((score - 0.32).abs() < f32::EPSILON);
    }
}
