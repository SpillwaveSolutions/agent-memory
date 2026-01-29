//! Table of Contents (TOC) node types.
//!
//! The TOC is a time-based hierarchy:
//! Year -> Month -> Week -> Day -> Segment
//!
//! Each node contains a summary with title, bullets, and keywords.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}

impl TocNode {
    /// Create a new TOC node
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
        }
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
}
