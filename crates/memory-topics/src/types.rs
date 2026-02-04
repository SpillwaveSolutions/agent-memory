//! Topic data types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A unique identifier for a topic.
pub type TopicId = String;

/// An embedding vector.
pub type Embedding = Vec<f32>;

/// A semantic topic extracted from TOC summaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// Unique identifier (ULID)
    pub topic_id: TopicId,
    /// Human-readable label (max 50 chars)
    pub label: String,
    /// Centroid embedding for similarity matching
    pub embedding: Embedding,
    /// Time-decayed importance score
    pub importance_score: f64,
    /// Number of linked TOC nodes
    pub node_count: u32,
    /// First occurrence timestamp
    pub created_at: DateTime<Utc>,
    /// Most recent mention timestamp
    pub last_mentioned_at: DateTime<Utc>,
    /// Active or pruned status
    pub status: TopicStatus,
    /// Keywords extracted from cluster
    pub keywords: Vec<String>,
}

impl Topic {
    /// Create a new topic with default values.
    pub fn new(topic_id: String, label: String, embedding: Vec<f32>) -> Self {
        let now = Utc::now();
        Self {
            topic_id,
            label,
            embedding,
            importance_score: 1.0,
            node_count: 0,
            created_at: now,
            last_mentioned_at: now,
            status: TopicStatus::Active,
            keywords: Vec::new(),
        }
    }

    /// Check if topic is active.
    pub fn is_active(&self) -> bool {
        self.status == TopicStatus::Active
    }
}

/// Topic status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TopicStatus {
    /// Topic is active and visible
    Active,
    /// Topic has been pruned due to inactivity
    Pruned,
}

/// Link between a topic and a TOC node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicLink {
    /// Topic identifier
    pub topic_id: String,
    /// TOC node identifier
    pub node_id: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
    /// When the link was created
    pub created_at: DateTime<Utc>,
}

impl TopicLink {
    /// Create a new topic-node link.
    pub fn new(topic_id: String, node_id: String, relevance: f32) -> Self {
        Self {
            topic_id,
            node_id,
            relevance,
            created_at: Utc::now(),
        }
    }
}

/// Type of relationship between topics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    /// Topics appear together in same documents
    CoOccurrence,
    /// Topics have similar embeddings
    Semantic,
    /// Parent/child hierarchical relationship
    Hierarchical,
}

impl RelationshipType {
    /// Get short code for storage key.
    pub fn code(&self) -> &'static str {
        match self {
            RelationshipType::CoOccurrence => "coo",
            RelationshipType::Semantic => "sem",
            RelationshipType::Hierarchical => "hie",
        }
    }

    /// Parse from code.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "coo" => Some(RelationshipType::CoOccurrence),
            "sem" => Some(RelationshipType::Semantic),
            "hie" => Some(RelationshipType::Hierarchical),
            _ => None,
        }
    }

    /// Get all relationship types.
    pub fn all() -> &'static [RelationshipType] {
        &[
            RelationshipType::CoOccurrence,
            RelationshipType::Semantic,
            RelationshipType::Hierarchical,
        ]
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipType::CoOccurrence => write!(f, "co-occurrence"),
            RelationshipType::Semantic => write!(f, "semantic"),
            RelationshipType::Hierarchical => write!(f, "hierarchical"),
        }
    }
}

/// Relationship between two topics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicRelationship {
    /// Source topic ID
    pub source_id: TopicId,
    /// Target topic ID
    pub target_id: TopicId,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength of relationship (0.0 - 1.0)
    pub strength: f32,
    /// Number of times relationship has been observed
    pub evidence_count: u32,
    /// When the relationship was first created
    pub created_at: DateTime<Utc>,
    /// When the relationship was last updated
    pub updated_at: DateTime<Utc>,
}

impl TopicRelationship {
    /// Create a new relationship with current timestamp.
    pub fn new(
        source_id: TopicId,
        target_id: TopicId,
        relationship_type: RelationshipType,
        strength: f32,
    ) -> Self {
        let now = Utc::now();
        Self {
            source_id,
            target_id,
            relationship_type,
            strength: strength.clamp(0.0, 1.0),
            evidence_count: 1,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new relationship with explicit timestamps.
    pub fn with_timestamps(
        source_id: TopicId,
        target_id: TopicId,
        relationship_type: RelationshipType,
        strength: f32,
        evidence_count: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            source_id,
            target_id,
            relationship_type,
            strength: strength.clamp(0.0, 1.0),
            evidence_count,
            created_at,
            updated_at,
        }
    }

    /// Check if this relationship is between the given topics (in either direction).
    pub fn connects(&self, topic_a: &TopicId, topic_b: &TopicId) -> bool {
        (self.source_id == *topic_a && self.target_id == *topic_b)
            || (self.source_id == *topic_b && self.target_id == *topic_a)
    }

    /// Update the strength, clamping to valid range.
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength.clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    /// Increment the evidence count and update timestamp.
    pub fn add_evidence(&mut self) {
        self.evidence_count = self.evidence_count.saturating_add(1);
        self.updated_at = Utc::now();
    }

    /// Strengthen the relationship by a delta amount.
    pub fn strengthen(&mut self, delta: f32) {
        self.set_strength(self.strength + delta);
    }

    /// Weaken the relationship by a delta amount.
    pub fn weaken(&mut self, delta: f32) {
        self.set_strength(self.strength - delta);
    }

    // Legacy compatibility aliases
    /// Get the source topic ID (legacy alias for from_topic_id).
    #[deprecated(note = "Use source_id instead")]
    pub fn from_topic_id(&self) -> &TopicId {
        &self.source_id
    }

    /// Get the target topic ID (legacy alias for to_topic_id).
    #[deprecated(note = "Use target_id instead")]
    pub fn to_topic_id(&self) -> &TopicId {
        &self.target_id
    }

    /// Get the strength (legacy alias for score).
    #[deprecated(note = "Use strength instead")]
    pub fn score(&self) -> f32 {
        self.strength
    }
}

/// Statistics about the topic graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicStats {
    /// Total number of topics
    pub topic_count: u64,
    /// Number of topic-node links
    pub link_count: u64,
    /// Number of topic relationships
    pub relationship_count: u64,
    /// Timestamp of last extraction (ms since epoch)
    pub last_extraction_ms: i64,
    /// Configured half-life in days
    pub half_life_days: u32,
    /// Configured similarity threshold
    pub similarity_threshold: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_new() {
        let topic = Topic::new(
            "01HRQ7D5KQ".to_string(),
            "Test Topic".to_string(),
            vec![0.1, 0.2, 0.3],
        );
        assert!(topic.is_active());
        assert_eq!(topic.node_count, 0);
        assert!((topic.importance_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_relationship_type_code() {
        assert_eq!(RelationshipType::CoOccurrence.code(), "coo");
        assert_eq!(RelationshipType::Semantic.code(), "sem");
        assert_eq!(RelationshipType::Hierarchical.code(), "hie");
        assert_eq!(
            RelationshipType::from_code("coo"),
            Some(RelationshipType::CoOccurrence)
        );
        assert_eq!(
            RelationshipType::from_code("sem"),
            Some(RelationshipType::Semantic)
        );
        assert_eq!(
            RelationshipType::from_code("hie"),
            Some(RelationshipType::Hierarchical)
        );
    }

    #[test]
    fn test_relationship_type_all() {
        let all = RelationshipType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&RelationshipType::CoOccurrence));
        assert!(all.contains(&RelationshipType::Semantic));
        assert!(all.contains(&RelationshipType::Hierarchical));
    }

    #[test]
    fn test_relationship_type_display() {
        assert_eq!(
            format!("{}", RelationshipType::CoOccurrence),
            "co-occurrence"
        );
        assert_eq!(format!("{}", RelationshipType::Semantic), "semantic");
        assert_eq!(
            format!("{}", RelationshipType::Hierarchical),
            "hierarchical"
        );
    }

    #[test]
    fn test_topic_link_new() {
        let link = TopicLink::new("topic-123".to_string(), "node-456".to_string(), 0.85);
        assert_eq!(link.topic_id, "topic-123");
        assert_eq!(link.node_id, "node-456");
        assert!((link.relevance - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_topic_relationship_new() {
        let rel = TopicRelationship::new(
            "topic-a".to_string(),
            "topic-b".to_string(),
            RelationshipType::Semantic,
            0.92,
        );
        assert_eq!(rel.source_id, "topic-a");
        assert_eq!(rel.target_id, "topic-b");
        assert_eq!(rel.relationship_type, RelationshipType::Semantic);
        assert!((rel.strength - 0.92).abs() < f32::EPSILON);
        assert_eq!(rel.evidence_count, 1);
    }

    #[test]
    fn test_topic_relationship_strength_clamping() {
        // Test clamping on creation
        let rel_high = TopicRelationship::new(
            "a".to_string(),
            "b".to_string(),
            RelationshipType::CoOccurrence,
            1.5,
        );
        assert!((rel_high.strength - 1.0).abs() < f32::EPSILON);

        let rel_low = TopicRelationship::new(
            "a".to_string(),
            "b".to_string(),
            RelationshipType::CoOccurrence,
            -0.5,
        );
        assert!(rel_low.strength.abs() < f32::EPSILON);

        // Test clamping on set_strength
        let mut rel = TopicRelationship::new(
            "a".to_string(),
            "b".to_string(),
            RelationshipType::Semantic,
            0.5,
        );
        rel.set_strength(2.0);
        assert!((rel.strength - 1.0).abs() < f32::EPSILON);
        rel.set_strength(-1.0);
        assert!(rel.strength.abs() < f32::EPSILON);
    }

    #[test]
    fn test_topic_relationship_strengthen_weaken() {
        let mut rel = TopicRelationship::new(
            "a".to_string(),
            "b".to_string(),
            RelationshipType::Semantic,
            0.5,
        );

        rel.strengthen(0.2);
        assert!((rel.strength - 0.7).abs() < f32::EPSILON);

        rel.weaken(0.3);
        assert!((rel.strength - 0.4).abs() < f32::EPSILON);

        // Test clamping
        rel.strengthen(1.0);
        assert!((rel.strength - 1.0).abs() < f32::EPSILON);

        rel.weaken(2.0);
        assert!(rel.strength.abs() < f32::EPSILON);
    }

    #[test]
    fn test_topic_relationship_add_evidence() {
        let mut rel = TopicRelationship::new(
            "a".to_string(),
            "b".to_string(),
            RelationshipType::CoOccurrence,
            0.5,
        );
        assert_eq!(rel.evidence_count, 1);

        rel.add_evidence();
        assert_eq!(rel.evidence_count, 2);

        rel.add_evidence();
        assert_eq!(rel.evidence_count, 3);
    }

    #[test]
    fn test_topic_relationship_connects() {
        let rel = TopicRelationship::new(
            "topic-a".to_string(),
            "topic-b".to_string(),
            RelationshipType::Semantic,
            0.8,
        );

        assert!(rel.connects(&"topic-a".to_string(), &"topic-b".to_string()));
        assert!(rel.connects(&"topic-b".to_string(), &"topic-a".to_string()));
        assert!(!rel.connects(&"topic-a".to_string(), &"topic-c".to_string()));
        assert!(!rel.connects(&"topic-c".to_string(), &"topic-d".to_string()));
    }

    #[test]
    fn test_topic_relationship_with_timestamps() {
        let created = Utc::now() - chrono::Duration::days(10);
        let updated = Utc::now() - chrono::Duration::days(1);

        let rel = TopicRelationship::with_timestamps(
            "a".to_string(),
            "b".to_string(),
            RelationshipType::Hierarchical,
            0.9,
            5,
            created,
            updated,
        );

        assert_eq!(rel.evidence_count, 5);
        assert_eq!(rel.created_at, created);
        assert_eq!(rel.updated_at, updated);
    }

    #[test]
    fn test_relationship_type_from_code_invalid() {
        assert_eq!(RelationshipType::from_code("invalid"), None);
        assert_eq!(RelationshipType::from_code(""), None);
        assert_eq!(RelationshipType::from_code("sim"), None); // Old code no longer valid
    }

    #[test]
    fn test_topic_status_equality() {
        assert_eq!(TopicStatus::Active, TopicStatus::Active);
        assert_ne!(TopicStatus::Active, TopicStatus::Pruned);
    }
}
