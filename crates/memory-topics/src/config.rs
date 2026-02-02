//! Topic configuration.

use serde::{Deserialize, Serialize};

/// Master configuration for topic functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicsConfig {
    /// Master switch - topics disabled by default
    #[serde(default)]
    pub enabled: bool,

    /// Extraction settings
    #[serde(default)]
    pub extraction: ExtractionConfig,

    /// Labeling settings
    #[serde(default)]
    pub labeling: LabelingConfig,

    /// Importance scoring settings
    #[serde(default)]
    pub importance: ImportanceConfig,

    /// Relationship detection settings
    #[serde(default)]
    pub relationships: RelationshipsConfig,

    /// Lifecycle management settings
    #[serde(default)]
    pub lifecycle: LifecycleConfig,
}

#[allow(clippy::derivable_impls)]
impl Default for TopicsConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default per TOPIC-07
            extraction: ExtractionConfig::default(),
            labeling: LabelingConfig::default(),
            importance: ImportanceConfig::default(),
            relationships: RelationshipsConfig::default(),
            lifecycle: LifecycleConfig::default(),
        }
    }
}

/// Topic extraction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Minimum cluster size for HDBSCAN
    #[serde(default = "default_min_cluster_size")]
    pub min_cluster_size: usize,

    /// Minimum similarity threshold for cluster membership
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,

    /// Cron schedule for extraction job
    #[serde(default = "default_extraction_schedule")]
    pub schedule: String,

    /// Maximum nodes to process per batch
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            min_cluster_size: default_min_cluster_size(),
            similarity_threshold: default_similarity_threshold(),
            schedule: default_extraction_schedule(),
            batch_size: default_batch_size(),
        }
    }
}

fn default_min_cluster_size() -> usize {
    3
}
fn default_similarity_threshold() -> f32 {
    0.75
}
fn default_extraction_schedule() -> String {
    "0 4 * * *".to_string() // 4 AM daily
}
fn default_batch_size() -> usize {
    500
}

/// Topic labeling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelingConfig {
    /// Whether to use LLM for labeling
    #[serde(default = "default_true")]
    pub use_llm: bool,

    /// Fall back to keyword extraction if LLM fails
    #[serde(default = "default_true")]
    pub fallback_to_keywords: bool,

    /// Maximum label length
    #[serde(default = "default_max_label_length")]
    pub max_label_length: usize,

    /// Number of top keywords to extract
    #[serde(default = "default_top_keywords")]
    pub top_keywords: usize,
}

impl Default for LabelingConfig {
    fn default() -> Self {
        Self {
            use_llm: default_true(),
            fallback_to_keywords: default_true(),
            max_label_length: default_max_label_length(),
            top_keywords: default_top_keywords(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_max_label_length() -> usize {
    50
}
fn default_top_keywords() -> usize {
    5
}

/// Importance scoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceConfig {
    /// Half-life in days for decay
    #[serde(default = "default_half_life_days")]
    pub half_life_days: u32,

    /// Boost multiplier for mentions within 7 days
    #[serde(default = "default_recency_boost")]
    pub recency_boost: f64,
}

impl Default for ImportanceConfig {
    fn default() -> Self {
        Self {
            half_life_days: default_half_life_days(),
            recency_boost: default_recency_boost(),
        }
    }
}

fn default_half_life_days() -> u32 {
    30
}
fn default_recency_boost() -> f64 {
    2.0
}

/// Relationship detection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipsConfig {
    /// Minimum similarity for "similar" relationship
    #[serde(default = "default_similar_threshold")]
    pub similar_threshold: f32,

    /// Maximum hierarchy depth
    #[serde(default = "default_max_hierarchy_depth")]
    pub max_hierarchy_depth: usize,

    /// Enable parent/child detection
    #[serde(default = "default_true")]
    pub enable_hierarchy: bool,
}

impl Default for RelationshipsConfig {
    fn default() -> Self {
        Self {
            similar_threshold: default_similar_threshold(),
            max_hierarchy_depth: default_max_hierarchy_depth(),
            enable_hierarchy: default_true(),
        }
    }
}

fn default_similar_threshold() -> f32 {
    0.8
}
fn default_max_hierarchy_depth() -> usize {
    3
}

/// Lifecycle management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Days of inactivity before pruning
    #[serde(default = "default_prune_after_days")]
    pub prune_after_days: u32,

    /// Cron schedule for pruning job
    #[serde(default = "default_prune_schedule")]
    pub prune_schedule: String,

    /// Enable automatic resurrection on re-mention
    #[serde(default = "default_true")]
    pub auto_resurrect: bool,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            prune_after_days: default_prune_after_days(),
            prune_schedule: default_prune_schedule(),
            auto_resurrect: default_true(),
        }
    }
}

fn default_prune_after_days() -> u32 {
    90
}
fn default_prune_schedule() -> String {
    "0 5 * * 0".to_string() // 5 AM Sunday
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_disabled() {
        let config = TopicsConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_extraction_defaults() {
        let config = ExtractionConfig::default();
        assert_eq!(config.min_cluster_size, 3);
        assert!((config.similarity_threshold - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_labeling_defaults() {
        let config = LabelingConfig::default();
        assert!(config.use_llm);
        assert!(config.fallback_to_keywords);
        assert_eq!(config.max_label_length, 50);
        assert_eq!(config.top_keywords, 5);
    }

    #[test]
    fn test_importance_defaults() {
        let config = ImportanceConfig::default();
        assert_eq!(config.half_life_days, 30);
        assert!((config.recency_boost - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_relationships_defaults() {
        let config = RelationshipsConfig::default();
        assert!((config.similar_threshold - 0.8).abs() < f32::EPSILON);
        assert_eq!(config.max_hierarchy_depth, 3);
        assert!(config.enable_hierarchy);
    }

    #[test]
    fn test_lifecycle_defaults() {
        let config = LifecycleConfig::default();
        assert_eq!(config.prune_after_days, 90);
        assert_eq!(config.prune_schedule, "0 5 * * 0");
        assert!(config.auto_resurrect);
    }

    #[test]
    fn test_config_serialization() {
        let config = TopicsConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: TopicsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.enabled, parsed.enabled);
        assert_eq!(
            config.extraction.min_cluster_size,
            parsed.extraction.min_cluster_size
        );
    }
}
