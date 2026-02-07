//! BM25 index lifecycle management per FR-09.
//!
//! Retention rules from PRD:
//! - Segment: 30 days (high churn)
//! - Grip: 30 days (same as segment)
//! - Day: 180 days (mid-term recall while rollups mature)
//! - Week: 1825 days (5 years)
//! - Month: NEVER pruned (stable anchor)
//! - Year: NEVER pruned (stable anchor)
//!
//! IMPORTANT: DISABLED by default per PRD "append-only, no eviction" philosophy.
//! Must be explicitly enabled via configuration.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for BM25 lifecycle per FR-09.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25LifecycleConfig {
    /// MUST be explicitly enabled (PRD default: append-only, no eviction).
    #[serde(default)]
    pub enabled: bool,

    /// Retention days for segment-level docs.
    #[serde(default = "default_segment_retention")]
    pub segment_retention_days: u32,

    /// Retention days for grip-level docs.
    #[serde(default = "default_grip_retention")]
    pub grip_retention_days: u32,

    /// Retention days for day-level docs.
    #[serde(default = "default_day_retention")]
    pub day_retention_days: u32,

    /// Retention days for week-level docs.
    #[serde(default = "default_week_retention")]
    pub week_retention_days: u32,
    // NOTE: month and year are NEVER pruned (protected)
}

fn default_segment_retention() -> u32 {
    30
}

fn default_grip_retention() -> u32 {
    30
}

fn default_day_retention() -> u32 {
    180 // Different from vector (180 vs 365)
}

fn default_week_retention() -> u32 {
    1825 // 5 years
}

impl Default for Bm25LifecycleConfig {
    fn default() -> Self {
        Self {
            enabled: false, // DISABLED by default per PRD
            segment_retention_days: default_segment_retention(),
            grip_retention_days: default_grip_retention(),
            day_retention_days: default_day_retention(),
            week_retention_days: default_week_retention(),
        }
    }
}

impl Bm25LifecycleConfig {
    /// Create an enabled lifecycle config with default retentions.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }
}

/// Statistics from a BM25 prune operation.
#[derive(Debug, Clone, Default)]
pub struct Bm25PruneStats {
    pub segments_pruned: u32,
    pub grips_pruned: u32,
    pub days_pruned: u32,
    pub weeks_pruned: u32,
    pub optimized: bool,
    pub errors: Vec<String>,
}

impl Bm25PruneStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, level: &str, count: u32) {
        match level {
            "segment" => self.segments_pruned += count,
            "grip" => self.grips_pruned += count,
            "day" => self.days_pruned += count,
            "week" => self.weeks_pruned += count,
            _ => {}
        }
    }

    pub fn total(&self) -> u32 {
        self.segments_pruned + self.grips_pruned + self.days_pruned + self.weeks_pruned
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0 && self.errors.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Protected levels that are NEVER pruned.
pub const PROTECTED_LEVELS: &[&str] = &["month", "year"];

/// Check if a level is protected from pruning.
pub fn is_protected_level(level: &str) -> bool {
    PROTECTED_LEVELS.contains(&level)
}

/// Get retention config as a map of level -> retention_days.
pub fn retention_map(config: &Bm25LifecycleConfig) -> HashMap<&'static str, u32> {
    let mut map = HashMap::new();
    map.insert("segment", config.segment_retention_days);
    map.insert("grip", config.grip_retention_days);
    map.insert("day", config.day_retention_days);
    map.insert("week", config.week_retention_days);
    map
}

/// Calculate cutoff date for a given retention period.
pub fn cutoff_date(retention_days: u32) -> DateTime<Utc> {
    Utc::now() - Duration::days(retention_days as i64)
}

/// BM25 maintenance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25MaintenanceConfig {
    /// Cron schedule for prune job (default: daily 3 AM).
    #[serde(default = "default_prune_schedule")]
    pub prune_schedule: String,

    /// Run index optimization after pruning (per FR-09).
    #[serde(default = "default_true")]
    pub optimize_after_prune: bool,
}

fn default_prune_schedule() -> String {
    "0 3 * * *".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Bm25MaintenanceConfig {
    fn default() -> Self {
        Self {
            prune_schedule: default_prune_schedule(),
            optimize_after_prune: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_by_default() {
        let config = Bm25LifecycleConfig::default();
        assert!(!config.enabled); // MUST be false by default
    }

    #[test]
    fn test_enabled_constructor() {
        let config = Bm25LifecycleConfig::enabled();
        assert!(config.enabled);
    }

    #[test]
    fn test_default_retention() {
        let config = Bm25LifecycleConfig::default();
        assert_eq!(config.segment_retention_days, 30);
        assert_eq!(config.grip_retention_days, 30);
        assert_eq!(config.day_retention_days, 180); // Different from vector
        assert_eq!(config.week_retention_days, 1825);
    }

    #[test]
    fn test_protected_levels() {
        assert!(is_protected_level("month"));
        assert!(is_protected_level("year"));
        assert!(!is_protected_level("segment"));
        assert!(!is_protected_level("grip"));
        assert!(!is_protected_level("day"));
        assert!(!is_protected_level("week"));
    }

    #[test]
    fn test_prune_stats() {
        let mut stats = Bm25PruneStats::new();
        assert!(stats.is_empty());

        stats.add("segment", 10);
        stats.add("day", 5);
        assert_eq!(stats.total(), 15);
        assert_eq!(stats.segments_pruned, 10);
        assert_eq!(stats.days_pruned, 5);
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_prune_stats_with_optimize() {
        let mut stats = Bm25PruneStats::new();
        stats.add("segment", 10);
        stats.optimized = true;
        assert!(stats.optimized);
    }

    #[test]
    fn test_prune_stats_errors() {
        let mut stats = Bm25PruneStats::new();
        stats.errors.push("Test error".to_string());
        assert!(stats.has_errors());
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_retention_map() {
        let config = Bm25LifecycleConfig::default();
        let map = retention_map(&config);
        assert_eq!(map.get("segment"), Some(&30));
        assert_eq!(map.get("grip"), Some(&30));
        assert_eq!(map.get("day"), Some(&180));
        assert_eq!(map.get("week"), Some(&1825));
        assert_eq!(map.get("month"), None); // Protected, not in map
        assert_eq!(map.get("year"), None); // Protected, not in map
    }

    #[test]
    fn test_cutoff_date() {
        let now = Utc::now();
        let cutoff = cutoff_date(30);
        let expected = now - Duration::days(30);
        // Allow 1 second tolerance for test timing
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn test_maintenance_config_defaults() {
        let config = Bm25MaintenanceConfig::default();
        assert_eq!(config.prune_schedule, "0 3 * * *");
        assert!(config.optimize_after_prune);
    }

    #[test]
    fn test_config_serialization() {
        let config = Bm25LifecycleConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: Bm25LifecycleConfig = serde_json::from_str(&json).unwrap();
        assert!(!decoded.enabled);
        assert_eq!(decoded.day_retention_days, 180);
    }
}
