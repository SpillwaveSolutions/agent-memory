//! Grip type for provenance anchoring.
//!
//! Grips link TOC summaries to source events, providing evidence
//! for claims made in bullet points.
//!
//! ## Phase 16 Enhancements
//!
//! Grip now includes salience fields for memory ranking:
//! - `salience_score`: Importance score calculated at write time
//! - `memory_kind`: Classification (observation, preference, procedure, etc.)
//! - `is_pinned`: Whether the grip is pinned for boosted importance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::salience::{default_salience, MemoryKind};

/// A grip anchors a summary excerpt to source events.
///
/// Per GRIP-01: Contains excerpt, event_id_start, event_id_end, timestamp, source.
/// Per GRIP-02: TOC node bullets link to supporting grips.
///
/// ## Phase 16 Salience Fields
///
/// New fields for memory ranking (calculated once at write time):
/// - `salience_score`: Importance score (0.0-1.0+)
/// - `memory_kind`: Classification of the memory type
/// - `is_pinned`: Whether this grip is pinned for boosted importance
///
/// These fields have serde defaults for backward compatibility with v2.0.0 data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grip {
    /// Unique identifier for this grip
    pub grip_id: String,

    /// The excerpt text that this grip anchors
    pub excerpt: String,

    /// First event in the range that supports this excerpt
    pub event_id_start: String,

    /// Last event in the range that supports this excerpt
    pub event_id_end: String,

    /// Timestamp of the excerpt (typically the start event's timestamp)
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,

    /// Source context (e.g., which summarization produced this)
    pub source: String,

    /// Optional: The TOC node ID that uses this grip
    #[serde(default)]
    pub toc_node_id: Option<String>,

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

    /// Whether this grip is pinned (boosted importance).
    /// Default: false for existing v2.0.0 data.
    #[serde(default)]
    pub is_pinned: bool,
}

impl Grip {
    /// Create a new grip with default salience values.
    pub fn new(
        grip_id: String,
        excerpt: String,
        event_id_start: String,
        event_id_end: String,
        timestamp: DateTime<Utc>,
        source: String,
    ) -> Self {
        Self {
            grip_id,
            excerpt,
            event_id_start,
            event_id_end,
            timestamp,
            source,
            toc_node_id: None,
            // Phase 16: Default salience values
            salience_score: default_salience(),
            memory_kind: MemoryKind::default(),
            is_pinned: false,
        }
    }

    /// Link this grip to a TOC node
    pub fn with_toc_node(mut self, toc_node_id: String) -> Self {
        self.toc_node_id = Some(toc_node_id);
        self
    }

    /// Set salience fields on this grip.
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
    fn test_grip_serialization() {
        let grip = Grip::new(
            "grip-123".to_string(),
            "User asked about Rust memory safety".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "segment_summarizer".to_string(),
        )
        .with_toc_node("toc-day-20240115".to_string());

        let bytes = grip.to_bytes().unwrap();
        let decoded = Grip::from_bytes(&bytes).unwrap();

        assert_eq!(grip.grip_id, decoded.grip_id);
        assert_eq!(grip.excerpt, decoded.excerpt);
        assert_eq!(grip.toc_node_id, decoded.toc_node_id);
    }

    // === Phase 16: Salience Tests ===

    #[test]
    fn test_grip_default_salience() {
        let grip = Grip::new(
            "grip-123".to_string(),
            "Test excerpt".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "test".to_string(),
        );

        assert!((grip.salience_score - 0.5).abs() < f32::EPSILON);
        assert_eq!(grip.memory_kind, MemoryKind::Observation);
        assert!(!grip.is_pinned);
    }

    #[test]
    fn test_grip_with_salience() {
        let grip = Grip::new(
            "grip-123".to_string(),
            "Test excerpt".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "test".to_string(),
        )
        .with_salience(0.85, MemoryKind::Preference, true);

        assert!((grip.salience_score - 0.85).abs() < f32::EPSILON);
        assert_eq!(grip.memory_kind, MemoryKind::Preference);
        assert!(grip.is_pinned);
    }

    #[test]
    fn test_grip_salience_builder_methods() {
        let grip = Grip::new(
            "grip-123".to_string(),
            "Test excerpt".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "test".to_string(),
        )
        .with_salience_score(0.75)
        .with_memory_kind(MemoryKind::Procedure)
        .with_pinned(true);

        assert!((grip.salience_score - 0.75).abs() < f32::EPSILON);
        assert_eq!(grip.memory_kind, MemoryKind::Procedure);
        assert!(grip.is_pinned);
    }

    #[test]
    fn test_grip_serialization_with_salience() {
        let grip = Grip::new(
            "grip-123".to_string(),
            "Test excerpt".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "test".to_string(),
        )
        .with_salience(0.9, MemoryKind::Constraint, true);

        let bytes = grip.to_bytes().unwrap();
        let decoded = Grip::from_bytes(&bytes).unwrap();

        assert!((decoded.salience_score - 0.9).abs() < f32::EPSILON);
        assert_eq!(decoded.memory_kind, MemoryKind::Constraint);
        assert!(decoded.is_pinned);
    }

    #[test]
    fn test_grip_backward_compat_v200() {
        // Simulate v2.0.0 serialized grip (no salience fields)
        let v200_json = r#"{
            "grip_id": "grip-001",
            "excerpt": "User discussed Rust patterns",
            "event_id_start": "event-001",
            "event_id_end": "event-003",
            "timestamp": 1735689600000,
            "source": "segment_summarizer"
        }"#;

        let grip: Grip = serde_json::from_str(v200_json).unwrap();

        // Verify default salience values are applied
        assert!((grip.salience_score - 0.5).abs() < f32::EPSILON);
        assert_eq!(grip.memory_kind, MemoryKind::Observation);
        assert!(!grip.is_pinned);

        // Verify other fields loaded correctly
        assert_eq!(grip.grip_id, "grip-001");
        assert_eq!(grip.excerpt, "User discussed Rust patterns");
    }
}
