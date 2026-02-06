//! Table of Contents (TOC) node types.
//!
//! The TOC is a time-based hierarchy:
//! Year -> Month -> Week -> Day -> Segment
//!
//! Each node contains a summary with title, bullets, and keywords.
//!
//! ## Phase 16 Enhancements
//!
//! TocNode now includes salience fields for memory ranking:
//! - `salience_score`: Importance score calculated at write time
//! - `memory_kind`: Classification (observation, preference, procedure, etc.)
//! - `is_pinned`: Whether the node is pinned for boosted importance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::salience::{default_salience, MemoryKind};

/// Level in the TOC hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TocLevel {
    Year,
    Month,
    Week,
    Day,
    Segment,
}

impl TocLevel {
    /// Get the parent level, if any
    pub fn parent(&self) -> Option<TocLevel> {
        match self {
            TocLevel::Year => None,
            TocLevel::Month => Some(TocLevel::Year),
            TocLevel::Week => Some(TocLevel::Month),
            TocLevel::Day => Some(TocLevel::Week),
            TocLevel::Segment => Some(TocLevel::Day),
        }
    }

    /// Get the child level, if any
    pub fn child(&self) -> Option<TocLevel> {
        match self {
            TocLevel::Year => Some(TocLevel::Month),
            TocLevel::Month => Some(TocLevel::Week),
            TocLevel::Week => Some(TocLevel::Day),
            TocLevel::Day => Some(TocLevel::Segment),
            TocLevel::Segment => None,
        }
    }
}

impl std::fmt::Display for TocLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TocLevel::Year => write!(f, "year"),
            TocLevel::Month => write!(f, "month"),
            TocLevel::Week => write!(f, "week"),
            TocLevel::Day => write!(f, "day"),
            TocLevel::Segment => write!(f, "segment"),
        }
    }
}

/// A bullet point in a TOC node summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocBullet {
    /// The bullet text
    pub text: String,

    /// Optional grip IDs that support this bullet (provenance)
    #[serde(default)]
    pub grip_ids: Vec<String>,
}

impl TocBullet {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            grip_ids: Vec::new(),
        }
    }

    pub fn with_grips(mut self, grip_ids: Vec<String>) -> Self {
        self.grip_ids = grip_ids;
        self
    }
}

/// A node in the Table of Contents hierarchy.
///
/// TOC nodes summarize time periods and link to children for drill-down.
/// Per TOC-02: Stores title, bullets, keywords, child_node_ids.
/// Per TOC-06: Nodes are versioned (append new version, don't mutate).
///
/// ## Phase 16 Salience Fields
///
/// New fields for memory ranking (calculated once at write time):
/// - `salience_score`: Importance score (0.0-1.0+)
/// - `memory_kind`: Classification of the memory type
/// - `is_pinned`: Whether this node is pinned for boosted importance
///
/// These fields have serde defaults for backward compatibility with v2.0.0 data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocNode {
    /// Unique identifier for this node
    pub node_id: String,

    /// Level in the hierarchy
    pub level: TocLevel,

    /// Human-readable title (e.g., "January 2024", "Week of Jan 15")
    pub title: String,

    /// Start of the time period this node covers
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub start_time: DateTime<Utc>,

    /// End of the time period this node covers
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub end_time: DateTime<Utc>,

    /// Summary bullet points
    pub bullets: Vec<TocBullet>,

    /// Keywords for search/filtering
    #[serde(default)]
    pub keywords: Vec<String>,

    /// IDs of child nodes (for drill-down)
    #[serde(default)]
    pub child_node_ids: Vec<String>,

    /// Version number (for TOC-06 versioning)
    pub version: u32,

    /// When this version was created
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,

    // === Phase 16: Salience Fields (backward compatible with serde defaults) ===
    /// Salience score (0.0-1.0+) computed at creation time.
    /// Higher scores indicate more important memories.
    /// Default: 0.5 (neutral) for existing v2.0.0 data.
    #[serde(default = "default_salience")]
    pub salience_score: f32,

    /// Classification of memory type (observation, preference, procedure, constraint, definition).
    /// Used for kind-based boosting in rankings.
    /// Default: Observation for existing v2.0.0 data.
    #[serde(default)]
    pub memory_kind: MemoryKind,

    /// Whether this node is pinned (boosted importance).
    /// Default: false for existing v2.0.0 data.
    #[serde(default)]
    pub is_pinned: bool,
}

impl TocNode {
    /// Create a new TOC node with default salience values.
    pub fn new(
        node_id: String,
        level: TocLevel,
        title: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            node_id,
            level,
            title,
            start_time,
            end_time,
            bullets: Vec::new(),
            keywords: Vec::new(),
            child_node_ids: Vec::new(),
            version: 1,
            created_at: Utc::now(),
            // Phase 16: Default salience values
            salience_score: default_salience(),
            memory_kind: MemoryKind::default(),
            is_pinned: false,
        }
    }

    /// Set salience fields on this node.
    ///
    /// Use this builder method to set write-time salience values.
    pub fn with_salience(mut self, score: f32, kind: MemoryKind, pinned: bool) -> Self {
        self.salience_score = score;
        self.memory_kind = kind;
        self.is_pinned = pinned;
        self
    }

    /// Set only the salience score.
    pub fn with_salience_score(mut self, score: f32) -> Self {
        self.salience_score = score;
        self
    }

    /// Set the memory kind.
    pub fn with_memory_kind(mut self, kind: MemoryKind) -> Self {
        self.memory_kind = kind;
        self
    }

    /// Set the pinned status.
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.is_pinned = pinned;
        self
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toc_level_hierarchy() {
        assert_eq!(TocLevel::Segment.parent(), Some(TocLevel::Day));
        assert_eq!(TocLevel::Day.parent(), Some(TocLevel::Week));
        assert_eq!(TocLevel::Year.parent(), None);
        assert_eq!(TocLevel::Year.child(), Some(TocLevel::Month));
        assert_eq!(TocLevel::Segment.child(), None);
    }

    #[test]
    fn test_toc_node_serialization() {
        let node = TocNode::new(
            "node-123".to_string(),
            TocLevel::Day,
            "Monday, January 15, 2024".to_string(),
            Utc::now(),
            Utc::now(),
        );

        let bytes = node.to_bytes().unwrap();
        let decoded = TocNode::from_bytes(&bytes).unwrap();

        assert_eq!(node.node_id, decoded.node_id);
        assert_eq!(node.level, decoded.level);
        assert_eq!(node.title, decoded.title);
    }

    // === Phase 16: Salience Tests ===

    #[test]
    fn test_toc_node_default_salience() {
        let node = TocNode::new(
            "node-123".to_string(),
            TocLevel::Day,
            "Test Node".to_string(),
            Utc::now(),
            Utc::now(),
        );

        assert!((node.salience_score - 0.5).abs() < f32::EPSILON);
        assert_eq!(node.memory_kind, MemoryKind::Observation);
        assert!(!node.is_pinned);
    }

    #[test]
    fn test_toc_node_with_salience() {
        let node = TocNode::new(
            "node-123".to_string(),
            TocLevel::Day,
            "Test Node".to_string(),
            Utc::now(),
            Utc::now(),
        )
        .with_salience(0.85, MemoryKind::Preference, true);

        assert!((node.salience_score - 0.85).abs() < f32::EPSILON);
        assert_eq!(node.memory_kind, MemoryKind::Preference);
        assert!(node.is_pinned);
    }

    #[test]
    fn test_toc_node_salience_builder_methods() {
        let node = TocNode::new(
            "node-123".to_string(),
            TocLevel::Day,
            "Test Node".to_string(),
            Utc::now(),
            Utc::now(),
        )
        .with_salience_score(0.75)
        .with_memory_kind(MemoryKind::Procedure)
        .with_pinned(true);

        assert!((node.salience_score - 0.75).abs() < f32::EPSILON);
        assert_eq!(node.memory_kind, MemoryKind::Procedure);
        assert!(node.is_pinned);
    }

    #[test]
    fn test_toc_node_serialization_with_salience() {
        let node = TocNode::new(
            "node-123".to_string(),
            TocLevel::Day,
            "Test Node".to_string(),
            Utc::now(),
            Utc::now(),
        )
        .with_salience(0.9, MemoryKind::Constraint, true);

        let bytes = node.to_bytes().unwrap();
        let decoded = TocNode::from_bytes(&bytes).unwrap();

        assert!((decoded.salience_score - 0.9).abs() < f32::EPSILON);
        assert_eq!(decoded.memory_kind, MemoryKind::Constraint);
        assert!(decoded.is_pinned);
    }

    #[test]
    fn test_toc_node_backward_compat_v200() {
        // Simulate v2.0.0 serialized node (no salience fields)
        // This JSON represents what old data looks like
        let v200_json = r#"{
            "node_id": "toc:day:2026-01-01",
            "level": "day",
            "title": "January 1, 2026",
            "start_time": 1735689600000,
            "end_time": 1735776000000,
            "bullets": [],
            "keywords": [],
            "child_node_ids": [],
            "version": 1,
            "created_at": 1735689600000
        }"#;

        let node: TocNode = serde_json::from_str(v200_json).unwrap();

        // Verify default salience values are applied
        assert!((node.salience_score - 0.5).abs() < f32::EPSILON);
        assert_eq!(node.memory_kind, MemoryKind::Observation);
        assert!(!node.is_pinned);

        // Verify other fields loaded correctly
        assert_eq!(node.node_id, "toc:day:2026-01-01");
        assert_eq!(node.level, TocLevel::Day);
    }
}
