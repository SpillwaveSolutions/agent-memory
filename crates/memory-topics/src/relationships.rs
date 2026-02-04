//! Topic relationship tracking and graph building.
//!
//! This module provides tools for building and managing relationships between topics,
//! enabling the construction of a topic graph that captures semantic, co-occurrence,
//! and hierarchical connections.
//!
//! ## Relationship Types
//!
//! - **Co-occurrence**: Topics that appear together in the same documents
//! - **Semantic**: Topics with similar embedding vectors
//! - **Hierarchical**: Parent/child relationships between topics
//!
//! ## Usage
//!
//! ```rust,ignore
//! use memory_topics::relationships::{RelationshipBuilder, TopicGraphBuilder};
//! use memory_topics::types::RelationshipType;
//!
//! // Build a single relationship
//! let rel = RelationshipBuilder::new("topic-a", "topic-b")
//!     .relationship_type(RelationshipType::Semantic)
//!     .strength(0.85)
//!     .build()
//!     .unwrap();
//!
//! // Build a topic graph
//! let mut builder = TopicGraphBuilder::new();
//! builder.add_co_occurrence("topic-1", "topic-2", "doc-123");
//! builder.add_co_occurrence("topic-1", "topic-2", "doc-456"); // Strengthens existing
//! let relationships = builder.build();
//! ```

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tracing::{debug, instrument};

use crate::error::TopicsError;
use crate::similarity::cosine_similarity;
use crate::types::{Embedding, RelationshipType, TopicId, TopicRelationship};

/// Default strength increase per co-occurrence evidence.
const CO_OCCURRENCE_STRENGTH_DELTA: f32 = 0.1;

/// Default threshold for semantic similarity relationships.
const DEFAULT_SEMANTIC_THRESHOLD: f32 = 0.75;

/// Builder for constructing `TopicRelationship` instances with validation.
#[derive(Debug, Clone)]
pub struct RelationshipBuilder {
    source_id: TopicId,
    target_id: TopicId,
    relationship_type: Option<RelationshipType>,
    strength: Option<f32>,
    evidence_count: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl RelationshipBuilder {
    /// Create a new relationship builder with source and target topics.
    pub fn new(source_id: impl Into<TopicId>, target_id: impl Into<TopicId>) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            relationship_type: None,
            strength: None,
            evidence_count: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set the relationship type.
    pub fn relationship_type(mut self, rel_type: RelationshipType) -> Self {
        self.relationship_type = Some(rel_type);
        self
    }

    /// Set the relationship strength (0.0 - 1.0).
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = Some(strength);
        self
    }

    /// Set the evidence count.
    pub fn evidence_count(mut self, count: u32) -> Self {
        self.evidence_count = Some(count);
        self
    }

    /// Set the created timestamp.
    pub fn created_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.created_at = Some(timestamp);
        self
    }

    /// Set the updated timestamp.
    pub fn updated_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.updated_at = Some(timestamp);
        self
    }

    /// Build the relationship, validating all fields.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Source and target are the same topic
    /// - Relationship type is not specified
    pub fn build(self) -> Result<TopicRelationship, TopicsError> {
        // Validate source != target
        if self.source_id == self.target_id {
            return Err(TopicsError::InvalidInput(
                "Source and target topics must be different".to_string(),
            ));
        }

        // Validate relationship type is set
        let rel_type = self.relationship_type.ok_or_else(|| {
            TopicsError::InvalidInput("Relationship type must be specified".to_string())
        })?;

        let strength = self.strength.unwrap_or(0.5);
        let evidence_count = self.evidence_count.unwrap_or(1);
        let now = Utc::now();
        let created_at = self.created_at.unwrap_or(now);
        let updated_at = self.updated_at.unwrap_or(now);

        Ok(TopicRelationship::with_timestamps(
            self.source_id,
            self.target_id,
            rel_type,
            strength,
            evidence_count,
            created_at,
            updated_at,
        ))
    }
}

/// Key for tracking relationships in the graph builder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RelationshipKey {
    source_id: TopicId,
    target_id: TopicId,
    rel_type: RelationshipType,
}

impl RelationshipKey {
    fn new(source_id: TopicId, target_id: TopicId, rel_type: RelationshipType) -> Self {
        Self {
            source_id,
            target_id,
            rel_type,
        }
    }

    /// Create a canonical key where source < target to handle bidirectional relationships.
    fn canonical(topic_a: &TopicId, topic_b: &TopicId, rel_type: RelationshipType) -> Self {
        if topic_a <= topic_b {
            Self::new(topic_a.clone(), topic_b.clone(), rel_type)
        } else {
            Self::new(topic_b.clone(), topic_a.clone(), rel_type)
        }
    }
}

/// Tracks relationship evidence during graph building.
#[derive(Debug, Clone)]
struct RelationshipEvidence {
    source_id: TopicId,
    target_id: TopicId,
    rel_type: RelationshipType,
    strength: f32,
    evidence_count: u32,
    document_ids: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RelationshipEvidence {
    fn new(
        source_id: TopicId,
        target_id: TopicId,
        rel_type: RelationshipType,
        initial_strength: f32,
    ) -> Self {
        let now = Utc::now();
        Self {
            source_id,
            target_id,
            rel_type,
            strength: initial_strength.clamp(0.0, 1.0),
            evidence_count: 1,
            document_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn add_evidence(&mut self, doc_id: Option<&str>, strength_delta: f32) {
        self.evidence_count = self.evidence_count.saturating_add(1);
        self.strength = (self.strength + strength_delta).clamp(0.0, 1.0);
        self.updated_at = Utc::now();

        if let Some(id) = doc_id {
            if !self.document_ids.contains(&id.to_string()) {
                self.document_ids.push(id.to_string());
            }
        }
    }

    fn into_relationship(self) -> TopicRelationship {
        TopicRelationship::with_timestamps(
            self.source_id,
            self.target_id,
            self.rel_type,
            self.strength,
            self.evidence_count,
            self.created_at,
            self.updated_at,
        )
    }
}

/// Builder for constructing a topic relationship graph.
///
/// The `TopicGraphBuilder` accumulates relationship evidence and produces
/// a set of `TopicRelationship` instances that can be stored.
///
/// ## Features
///
/// - Tracks co-occurrence of topics in documents
/// - Computes semantic relationships from embeddings
/// - Supports explicit hierarchical relationships
/// - Handles bidirectional relationships (A->B and B->A are tracked once)
/// - Strengthens relationships as more evidence is added
#[derive(Debug, Default)]
pub struct TopicGraphBuilder {
    relationships: HashMap<RelationshipKey, RelationshipEvidence>,
    co_occurrence_strength_delta: f32,
    semantic_threshold: f32,
}

impl TopicGraphBuilder {
    /// Create a new topic graph builder with default settings.
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
            co_occurrence_strength_delta: CO_OCCURRENCE_STRENGTH_DELTA,
            semantic_threshold: DEFAULT_SEMANTIC_THRESHOLD,
        }
    }

    /// Create a builder with custom settings.
    pub fn with_settings(co_occurrence_strength_delta: f32, semantic_threshold: f32) -> Self {
        Self {
            relationships: HashMap::new(),
            co_occurrence_strength_delta,
            semantic_threshold,
        }
    }

    /// Set the strength delta added per co-occurrence evidence.
    pub fn set_co_occurrence_strength_delta(&mut self, delta: f32) {
        self.co_occurrence_strength_delta = delta;
    }

    /// Set the threshold for semantic similarity relationships.
    pub fn set_semantic_threshold(&mut self, threshold: f32) {
        self.semantic_threshold = threshold;
    }

    /// Track co-occurring topics from the same document.
    ///
    /// When topics appear together in a document, they are considered related.
    /// Repeated co-occurrences strengthen the relationship.
    ///
    /// # Arguments
    ///
    /// * `topic_a` - First topic ID
    /// * `topic_b` - Second topic ID
    /// * `doc_id` - Document where they co-occur
    #[instrument(skip(self))]
    pub fn add_co_occurrence(&mut self, topic_a: &TopicId, topic_b: &TopicId, doc_id: &str) {
        if topic_a == topic_b {
            debug!("Ignoring self-relationship");
            return;
        }

        let key = RelationshipKey::canonical(topic_a, topic_b, RelationshipType::CoOccurrence);

        self.relationships
            .entry(key)
            .and_modify(|e| {
                e.add_evidence(Some(doc_id), self.co_occurrence_strength_delta);
            })
            .or_insert_with(|| {
                let mut evidence = RelationshipEvidence::new(
                    topic_a.clone(),
                    topic_b.clone(),
                    RelationshipType::CoOccurrence,
                    self.co_occurrence_strength_delta,
                );
                evidence.document_ids.push(doc_id.to_string());
                evidence
            });

        debug!(
            topic_a = %topic_a,
            topic_b = %topic_b,
            doc_id = %doc_id,
            "Added co-occurrence relationship"
        );
    }

    /// Compute semantic relationships from topic embeddings.
    ///
    /// Topics with embedding similarity above the threshold are related.
    ///
    /// # Arguments
    ///
    /// * `embeddings` - Slice of (topic_id, embedding) pairs
    /// * `threshold` - Minimum similarity for relationship (overrides builder default if provided)
    #[instrument(skip(self, embeddings))]
    pub fn compute_semantic_relationships(
        &mut self,
        embeddings: &[(TopicId, Embedding)],
        threshold: Option<f32>,
    ) {
        let threshold = threshold.unwrap_or(self.semantic_threshold);
        let n = embeddings.len();

        debug!(
            count = n,
            threshold = threshold,
            "Computing semantic relationships"
        );

        for i in 0..n {
            for j in (i + 1)..n {
                let (topic_a, emb_a) = &embeddings[i];
                let (topic_b, emb_b) = &embeddings[j];

                if topic_a == topic_b {
                    continue;
                }

                let similarity = cosine_similarity(emb_a, emb_b);

                if similarity >= threshold {
                    let key =
                        RelationshipKey::canonical(topic_a, topic_b, RelationshipType::Semantic);

                    self.relationships
                        .entry(key)
                        .and_modify(|e| {
                            // Update strength to max of existing and new similarity
                            if similarity > e.strength {
                                e.strength = similarity;
                                e.updated_at = Utc::now();
                            }
                            e.evidence_count = e.evidence_count.saturating_add(1);
                        })
                        .or_insert_with(|| {
                            RelationshipEvidence::new(
                                topic_a.clone(),
                                topic_b.clone(),
                                RelationshipType::Semantic,
                                similarity,
                            )
                        });

                    debug!(
                        topic_a = %topic_a,
                        topic_b = %topic_b,
                        similarity = similarity,
                        "Added semantic relationship"
                    );
                }
            }
        }
    }

    /// Set an explicit hierarchical relationship (parent/child).
    ///
    /// # Arguments
    ///
    /// * `parent` - Parent topic ID
    /// * `child` - Child topic ID
    #[instrument(skip(self))]
    pub fn set_hierarchy(&mut self, parent: &TopicId, child: &TopicId) {
        if parent == child {
            debug!("Ignoring self-hierarchy");
            return;
        }

        // For hierarchy, direction matters: parent -> child
        let key = RelationshipKey::new(
            parent.clone(),
            child.clone(),
            RelationshipType::Hierarchical,
        );

        self.relationships
            .entry(key)
            .and_modify(|e| {
                e.add_evidence(None, 0.0);
            })
            .or_insert_with(|| {
                RelationshipEvidence::new(
                    parent.clone(),
                    child.clone(),
                    RelationshipType::Hierarchical,
                    1.0, // Hierarchical relationships have full strength
                )
            });

        debug!(
            parent = %parent,
            child = %child,
            "Set hierarchical relationship"
        );
    }

    /// Get topics related to the given topic.
    ///
    /// # Arguments
    ///
    /// * `topic_id` - Topic to find relationships for
    /// * `limit` - Maximum number of related topics to return
    ///
    /// # Returns
    ///
    /// Vector of (related_topic_id, strength) pairs, sorted by strength descending.
    pub fn get_related_topics(&self, topic_id: &TopicId, limit: usize) -> Vec<(TopicId, f32)> {
        let mut related: Vec<(TopicId, f32)> = self
            .relationships
            .values()
            .filter_map(|evidence| {
                if evidence.source_id == *topic_id {
                    Some((evidence.target_id.clone(), evidence.strength))
                } else if evidence.target_id == *topic_id {
                    Some((evidence.source_id.clone(), evidence.strength))
                } else {
                    None
                }
            })
            .collect();

        // Sort by strength descending
        related.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        related.truncate(limit);
        related
    }

    /// Strengthen an existing relationship by a delta amount.
    ///
    /// # Arguments
    ///
    /// * `source` - Source topic ID
    /// * `target` - Target topic ID
    /// * `delta` - Amount to strengthen (can be negative to weaken)
    ///
    /// # Returns
    ///
    /// `true` if the relationship was found and updated, `false` otherwise.
    pub fn strengthen_relationship(
        &mut self,
        source: &TopicId,
        target: &TopicId,
        delta: f32,
    ) -> bool {
        // Try all relationship types
        for rel_type in RelationshipType::all() {
            // Try canonical key first
            let key = RelationshipKey::canonical(source, target, *rel_type);
            if let Some(evidence) = self.relationships.get_mut(&key) {
                evidence.strength = (evidence.strength + delta).clamp(0.0, 1.0);
                evidence.updated_at = Utc::now();
                return true;
            }

            // For hierarchical, also try direct key
            if *rel_type == RelationshipType::Hierarchical {
                let direct_key = RelationshipKey::new(source.clone(), target.clone(), *rel_type);
                if let Some(evidence) = self.relationships.get_mut(&direct_key) {
                    evidence.strength = (evidence.strength + delta).clamp(0.0, 1.0);
                    evidence.updated_at = Utc::now();
                    return true;
                }
            }
        }
        false
    }

    /// Get the number of relationships tracked.
    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    /// Check if a relationship exists between two topics.
    pub fn has_relationship(&self, topic_a: &TopicId, topic_b: &TopicId) -> bool {
        for rel_type in RelationshipType::all() {
            let key = RelationshipKey::canonical(topic_a, topic_b, *rel_type);
            if self.relationships.contains_key(&key) {
                return true;
            }
            // For hierarchical, check both directions
            if *rel_type == RelationshipType::Hierarchical {
                let direct_key_ab =
                    RelationshipKey::new(topic_a.clone(), topic_b.clone(), *rel_type);
                let direct_key_ba =
                    RelationshipKey::new(topic_b.clone(), topic_a.clone(), *rel_type);
                if self.relationships.contains_key(&direct_key_ab)
                    || self.relationships.contains_key(&direct_key_ba)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Build the final list of relationships.
    ///
    /// Consumes the builder and returns all accumulated relationships.
    pub fn build(self) -> Vec<TopicRelationship> {
        self.relationships
            .into_values()
            .map(|e| e.into_relationship())
            .collect()
    }

    /// Get relationships without consuming the builder.
    pub fn get_relationships(&self) -> Vec<TopicRelationship> {
        self.relationships
            .values()
            .map(|e| {
                TopicRelationship::with_timestamps(
                    e.source_id.clone(),
                    e.target_id.clone(),
                    e.rel_type,
                    e.strength,
                    e.evidence_count,
                    e.created_at,
                    e.updated_at,
                )
            })
            .collect()
    }

    /// Clear all tracked relationships.
    pub fn clear(&mut self) {
        self.relationships.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== RelationshipBuilder Tests ====================

    #[test]
    fn test_relationship_builder_basic() {
        let rel = RelationshipBuilder::new("topic-a", "topic-b")
            .relationship_type(RelationshipType::Semantic)
            .strength(0.85)
            .build()
            .unwrap();

        assert_eq!(rel.source_id, "topic-a");
        assert_eq!(rel.target_id, "topic-b");
        assert_eq!(rel.relationship_type, RelationshipType::Semantic);
        assert!((rel.strength - 0.85).abs() < f32::EPSILON);
        assert_eq!(rel.evidence_count, 1);
    }

    #[test]
    fn test_relationship_builder_all_fields() {
        let created = Utc::now() - chrono::Duration::days(10);
        let updated = Utc::now() - chrono::Duration::days(1);

        let rel = RelationshipBuilder::new("source", "target")
            .relationship_type(RelationshipType::CoOccurrence)
            .strength(0.7)
            .evidence_count(5)
            .created_at(created)
            .updated_at(updated)
            .build()
            .unwrap();

        assert_eq!(rel.evidence_count, 5);
        assert_eq!(rel.created_at, created);
        assert_eq!(rel.updated_at, updated);
    }

    #[test]
    fn test_relationship_builder_defaults() {
        let rel = RelationshipBuilder::new("a", "b")
            .relationship_type(RelationshipType::Hierarchical)
            .build()
            .unwrap();

        // Default strength is 0.5
        assert!((rel.strength - 0.5).abs() < f32::EPSILON);
        // Default evidence count is 1
        assert_eq!(rel.evidence_count, 1);
    }

    #[test]
    fn test_relationship_builder_strength_clamping() {
        let rel_high = RelationshipBuilder::new("a", "b")
            .relationship_type(RelationshipType::Semantic)
            .strength(1.5)
            .build()
            .unwrap();
        assert!((rel_high.strength - 1.0).abs() < f32::EPSILON);

        let rel_low = RelationshipBuilder::new("a", "b")
            .relationship_type(RelationshipType::Semantic)
            .strength(-0.5)
            .build()
            .unwrap();
        assert!(rel_low.strength.abs() < f32::EPSILON);
    }

    #[test]
    fn test_relationship_builder_error_same_topic() {
        let result = RelationshipBuilder::new("topic-x", "topic-x")
            .relationship_type(RelationshipType::Semantic)
            .build();

        assert!(result.is_err());
        match result {
            Err(TopicsError::InvalidInput(msg)) => {
                assert!(msg.contains("different"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_relationship_builder_error_no_type() {
        let result = RelationshipBuilder::new("a", "b").strength(0.5).build();

        assert!(result.is_err());
        match result {
            Err(TopicsError::InvalidInput(msg)) => {
                assert!(msg.contains("type"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // ==================== TopicGraphBuilder Tests ====================

    #[test]
    fn test_graph_builder_new() {
        let builder = TopicGraphBuilder::new();
        assert_eq!(builder.relationship_count(), 0);
    }

    #[test]
    fn test_graph_builder_with_settings() {
        let builder = TopicGraphBuilder::with_settings(0.2, 0.9);
        assert!((builder.co_occurrence_strength_delta - 0.2).abs() < f32::EPSILON);
        assert!((builder.semantic_threshold - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_add_co_occurrence_single() {
        let mut builder = TopicGraphBuilder::new();
        builder.add_co_occurrence(&"topic-1".to_string(), &"topic-2".to_string(), "doc-1");

        assert_eq!(builder.relationship_count(), 1);

        let rels = builder.build();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].relationship_type, RelationshipType::CoOccurrence);
        assert_eq!(rels[0].evidence_count, 1);
    }

    #[test]
    fn test_add_co_occurrence_strengthens() {
        let mut builder = TopicGraphBuilder::new();
        let topic_a = "topic-1".to_string();
        let topic_b = "topic-2".to_string();

        builder.add_co_occurrence(&topic_a, &topic_b, "doc-1");
        builder.add_co_occurrence(&topic_a, &topic_b, "doc-2");
        builder.add_co_occurrence(&topic_a, &topic_b, "doc-3");

        assert_eq!(builder.relationship_count(), 1);

        let rels = builder.build();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].evidence_count, 3);
        // Strength should be 3 * 0.1 = 0.3
        assert!((rels[0].strength - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_add_co_occurrence_bidirectional() {
        let mut builder = TopicGraphBuilder::new();
        let topic_a = "topic-1".to_string();
        let topic_b = "topic-2".to_string();

        // Adding in either order should create one relationship
        builder.add_co_occurrence(&topic_a, &topic_b, "doc-1");
        builder.add_co_occurrence(&topic_b, &topic_a, "doc-2");

        assert_eq!(builder.relationship_count(), 1);

        let rels = builder.build();
        assert_eq!(rels[0].evidence_count, 2);
    }

    #[test]
    fn test_add_co_occurrence_ignores_self() {
        let mut builder = TopicGraphBuilder::new();
        builder.add_co_occurrence(&"topic-1".to_string(), &"topic-1".to_string(), "doc-1");

        assert_eq!(builder.relationship_count(), 0);
    }

    #[test]
    fn test_compute_semantic_relationships() {
        let mut builder = TopicGraphBuilder::new();

        // Similar vectors (high similarity)
        let embeddings = vec![
            ("topic-1".to_string(), vec![1.0, 0.0, 0.0]),
            ("topic-2".to_string(), vec![0.9, 0.1, 0.0]),
            ("topic-3".to_string(), vec![0.0, 0.0, 1.0]), // Orthogonal, won't match
        ];

        builder.compute_semantic_relationships(&embeddings, Some(0.8));

        // Should only create relationship between topic-1 and topic-2
        assert_eq!(builder.relationship_count(), 1);

        let rels = builder.build();
        assert_eq!(rels[0].relationship_type, RelationshipType::Semantic);
        assert!(rels[0].strength >= 0.8);
    }

    #[test]
    fn test_compute_semantic_relationships_uses_default_threshold() {
        let mut builder = TopicGraphBuilder::with_settings(0.1, 0.5);

        let embeddings = vec![
            ("topic-1".to_string(), vec![1.0, 0.0]),
            ("topic-2".to_string(), vec![0.6, 0.8]), // Similarity ~0.6
        ];

        builder.compute_semantic_relationships(&embeddings, None);

        // Should create relationship since 0.6 > 0.5 (default threshold)
        assert_eq!(builder.relationship_count(), 1);
    }

    #[test]
    fn test_set_hierarchy() {
        let mut builder = TopicGraphBuilder::new();
        builder.set_hierarchy(&"parent-topic".to_string(), &"child-topic".to_string());

        assert_eq!(builder.relationship_count(), 1);

        let rels = builder.build();
        assert_eq!(rels[0].relationship_type, RelationshipType::Hierarchical);
        assert_eq!(rels[0].source_id, "parent-topic");
        assert_eq!(rels[0].target_id, "child-topic");
        assert!((rels[0].strength - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_hierarchy_ignores_self() {
        let mut builder = TopicGraphBuilder::new();
        builder.set_hierarchy(&"topic".to_string(), &"topic".to_string());

        assert_eq!(builder.relationship_count(), 0);
    }

    #[test]
    fn test_get_related_topics() {
        let mut builder = TopicGraphBuilder::new();
        let main = "main-topic".to_string();
        let related_1 = "related-1".to_string();
        let related_2 = "related-2".to_string();
        let unrelated = "unrelated".to_string();

        // Create relationships with different strengths
        builder.add_co_occurrence(&main, &related_1, "doc-1");
        builder.add_co_occurrence(&main, &related_1, "doc-2"); // strength 0.2
        builder.add_co_occurrence(&main, &related_2, "doc-3"); // strength 0.1
        builder.add_co_occurrence(&related_1, &unrelated, "doc-4"); // not related to main

        let related = builder.get_related_topics(&main, 10);

        assert_eq!(related.len(), 2);
        // Should be sorted by strength descending
        assert_eq!(related[0].0, related_1);
        assert_eq!(related[1].0, related_2);
    }

    #[test]
    fn test_get_related_topics_limit() {
        let mut builder = TopicGraphBuilder::new();
        let main = "main".to_string();

        for i in 0..10 {
            builder.add_co_occurrence(&main, &format!("topic-{}", i), "doc");
        }

        let related = builder.get_related_topics(&main, 5);
        assert_eq!(related.len(), 5);
    }

    #[test]
    fn test_strengthen_relationship() {
        let mut builder = TopicGraphBuilder::new();
        let topic_a = "topic-a".to_string();
        let topic_b = "topic-b".to_string();

        builder.add_co_occurrence(&topic_a, &topic_b, "doc-1");

        // Initial strength is 0.1
        let initial_rels = builder.get_relationships();
        let initial_strength = initial_rels[0].strength;

        // Strengthen by 0.25
        let updated = builder.strengthen_relationship(&topic_a, &topic_b, 0.25);
        assert!(updated);

        let rels = builder.get_relationships();
        assert!((rels[0].strength - (initial_strength + 0.25)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_strengthen_relationship_clamping() {
        let mut builder = TopicGraphBuilder::new();
        let topic_a = "topic-a".to_string();
        let topic_b = "topic-b".to_string();

        builder.add_co_occurrence(&topic_a, &topic_b, "doc-1");

        // Try to strengthen beyond 1.0
        builder.strengthen_relationship(&topic_a, &topic_b, 10.0);
        let rels = builder.get_relationships();
        assert!((rels[0].strength - 1.0).abs() < f32::EPSILON);

        // Try to weaken below 0.0
        builder.strengthen_relationship(&topic_a, &topic_b, -10.0);
        let rels = builder.get_relationships();
        assert!(rels[0].strength.abs() < f32::EPSILON);
    }

    #[test]
    fn test_strengthen_relationship_not_found() {
        let mut builder = TopicGraphBuilder::new();
        let result = builder.strengthen_relationship(
            &"nonexistent-a".to_string(),
            &"nonexistent-b".to_string(),
            0.1,
        );
        assert!(!result);
    }

    #[test]
    fn test_has_relationship() {
        let mut builder = TopicGraphBuilder::new();
        let topic_a = "topic-a".to_string();
        let topic_b = "topic-b".to_string();
        let topic_c = "topic-c".to_string();

        builder.add_co_occurrence(&topic_a, &topic_b, "doc-1");

        assert!(builder.has_relationship(&topic_a, &topic_b));
        assert!(builder.has_relationship(&topic_b, &topic_a)); // Bidirectional
        assert!(!builder.has_relationship(&topic_a, &topic_c));
    }

    #[test]
    fn test_clear() {
        let mut builder = TopicGraphBuilder::new();
        builder.add_co_occurrence(&"a".to_string(), &"b".to_string(), "doc");
        builder.add_co_occurrence(&"c".to_string(), &"d".to_string(), "doc");

        assert_eq!(builder.relationship_count(), 2);

        builder.clear();

        assert_eq!(builder.relationship_count(), 0);
    }

    #[test]
    fn test_build_consumes_builder() {
        let mut builder = TopicGraphBuilder::new();
        builder.add_co_occurrence(&"a".to_string(), &"b".to_string(), "doc");

        let rels = builder.build();
        assert_eq!(rels.len(), 1);
        // builder is now consumed
    }

    #[test]
    fn test_get_relationships_preserves_builder() {
        let mut builder = TopicGraphBuilder::new();
        builder.add_co_occurrence(&"a".to_string(), &"b".to_string(), "doc");

        let rels1 = builder.get_relationships();
        let rels2 = builder.get_relationships();

        assert_eq!(rels1.len(), rels2.len());
    }

    #[test]
    fn test_multiple_relationship_types_same_topics() {
        let mut builder = TopicGraphBuilder::new();
        let topic_a = "topic-a".to_string();
        let topic_b = "topic-b".to_string();

        // Same topics can have different relationship types
        builder.add_co_occurrence(&topic_a, &topic_b, "doc-1");
        builder.set_hierarchy(&topic_a, &topic_b);

        let embeddings = vec![
            (topic_a.clone(), vec![1.0, 0.0]),
            (topic_b.clone(), vec![0.9, 0.1]),
        ];
        builder.compute_semantic_relationships(&embeddings, Some(0.5));

        // Should have 3 different relationship types
        assert_eq!(builder.relationship_count(), 3);
    }

    #[test]
    fn test_integration_complex_graph() {
        let mut builder = TopicGraphBuilder::with_settings(0.15, 0.7);

        let topics = vec![
            ("rust".to_string(), vec![1.0, 0.0, 0.0]),
            ("programming".to_string(), vec![0.9, 0.1, 0.0]),
            ("memory".to_string(), vec![0.8, 0.2, 0.0]),
            ("databases".to_string(), vec![0.0, 1.0, 0.0]),
            ("sql".to_string(), vec![0.1, 0.9, 0.0]),
        ];

        // Add co-occurrences
        builder.add_co_occurrence(&topics[0].0, &topics[1].0, "doc-rust-programming");
        builder.add_co_occurrence(&topics[0].0, &topics[2].0, "doc-rust-memory");
        builder.add_co_occurrence(&topics[3].0, &topics[4].0, "doc-db-sql");

        // Add hierarchy
        builder.set_hierarchy(&topics[1].0, &topics[0].0); // programming -> rust

        // Compute semantic relationships
        builder.compute_semantic_relationships(&topics, None);

        let rels = builder.build();

        // Verify we have multiple relationship types
        let co_occur_count = rels
            .iter()
            .filter(|r| r.relationship_type == RelationshipType::CoOccurrence)
            .count();
        let semantic_count = rels
            .iter()
            .filter(|r| r.relationship_type == RelationshipType::Semantic)
            .count();
        let hier_count = rels
            .iter()
            .filter(|r| r.relationship_type == RelationshipType::Hierarchical)
            .count();

        assert!(
            co_occur_count > 0,
            "Should have co-occurrence relationships"
        );
        assert!(semantic_count > 0, "Should have semantic relationships");
        assert_eq!(hier_count, 1, "Should have one hierarchical relationship");
    }
}
