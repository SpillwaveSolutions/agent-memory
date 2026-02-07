//! Vector index lifecycle management per FR-08.
//!
//! Retention rules from PRD:
//! - Segment: 30 days (high churn, rolled up quickly)
//! - Grip: 30 days (same as segment)
//! - Day: 365 days (mid-term recall)
//! - Week: 1825 days (5 years)
//! - Month: NEVER pruned (stable anchor)
//! - Year: NEVER pruned (stable anchor)

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for vector lifecycle per FR-08.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorLifecycleConfig {
    /// Enable automatic vector pruning.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Retention days for segment-level vectors.
    #[serde(default = "default_segment_retention")]
    pub segment_retention_days: u32,

    /// Retention days for grip-level vectors.
    #[serde(default = "default_grip_retention")]
    pub grip_retention_days: u32,

    /// Retention days for day-level vectors.
    #[serde(default = "default_day_retention")]
    pub day_retention_days: u32,

    /// Retention days for week-level vectors.
    #[serde(default = "default_week_retention")]
    pub week_retention_days: u32,
    // NOTE: month and year are NEVER pruned (protected)
}

fn default_true() -> bool {
    true
}

fn default_segment_retention() -> u32 {
    30
}

fn default_grip_retention() -> u32 {
    30
}

fn default_day_retention() -> u32 {
    365
}

fn default_week_retention() -> u32 {
    1825 // 5 years
}

impl Default for VectorLifecycleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            segment_retention_days: default_segment_retention(),
            grip_retention_days: default_grip_retention(),
            day_retention_days: default_day_retention(),
            week_retention_days: default_week_retention(),
        }
    }
}

impl VectorLifecycleConfig {
    /// Create a disabled lifecycle config.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// Statistics from a prune operation.
#[derive(Debug, Clone, Default)]
pub struct PruneStats {
    pub segments_pruned: u32,
    pub grips_pruned: u32,
    pub days_pruned: u32,
    pub weeks_pruned: u32,
    pub errors: Vec<String>,
}

impl PruneStats {
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
pub fn retention_map(config: &VectorLifecycleConfig) -> HashMap<&'static str, u32> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VectorLifecycleConfig::default();
        assert!(config.enabled);
        assert_eq!(config.segment_retention_days, 30);
        assert_eq!(config.grip_retention_days, 30);
        assert_eq!(config.day_retention_days, 365);
        assert_eq!(config.week_retention_days, 1825);
    }

    #[test]
    fn test_disabled_config() {
        let config = VectorLifecycleConfig::disabled();
        assert!(!config.enabled);
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
        let mut stats = PruneStats::new();
        assert!(stats.is_empty());

        stats.add("segment", 10);
        stats.add("day", 5);
        assert_eq!(stats.total(), 15);
        assert_eq!(stats.segments_pruned, 10);
        assert_eq!(stats.days_pruned, 5);
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_prune_stats_errors() {
        let mut stats = PruneStats::new();
        stats.errors.push("Test error".to_string());
        assert!(stats.has_errors());
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_retention_map() {
        let config = VectorLifecycleConfig::default();
        let map = retention_map(&config);
        assert_eq!(map.get("segment"), Some(&30));
        assert_eq!(map.get("grip"), Some(&30));
        assert_eq!(map.get("day"), Some(&365));
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
    fn test_config_serialization() {
        let config = VectorLifecycleConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: VectorLifecycleConfig = serde_json::from_str(&json).unwrap();
        assert!(decoded.enabled);
        assert_eq!(decoded.segment_retention_days, 30);
    }
}
