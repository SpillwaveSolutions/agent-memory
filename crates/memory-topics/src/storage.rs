//! Topic storage operations.
//!
//! Manages topics, links, and relationships in RocksDB column families.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use memory_storage::Storage;
use tracing::{debug, info, instrument};

use crate::error::TopicsError;
use crate::importance::ImportanceScorer;
use crate::types::{
    RelationshipType, Topic, TopicLink, TopicRelationship, TopicStats, TopicStatus,
};

/// Column family names (must match memory-storage)
pub const CF_TOPICS: &str = "topics";
pub const CF_TOPIC_LINKS: &str = "topic_links";
pub const CF_TOPIC_RELS: &str = "topic_rels";
/// Column family for topic relationships (alias for CF_TOPIC_RELS)
pub const CF_TOPIC_RELATIONSHIPS: &str = "topic_rels";

/// Key format for topics: topic:{topic_id}
pub fn topic_key(topic_id: &str) -> String {
    format!("topic:{}", topic_id)
}

/// Key format for topic links: link:{topic_id}:{node_id}
pub fn topic_link_key(topic_id: &str, node_id: &str) -> String {
    format!("link:{}:{}", topic_id, node_id)
}

/// Secondary index: node:{node_id}:{topic_id}
pub fn node_topic_key(node_id: &str, topic_id: &str) -> String {
    format!("node:{}:{}", node_id, topic_id)
}

/// Key format for relationships: rel:{from}:{type}:{to}
pub fn relationship_key(from_id: &str, rel_type: &str, to_id: &str) -> String {
    format!("rel:{}:{}:{}", from_id, rel_type, to_id)
}

/// Topic storage interface.
pub struct TopicStorage {
    storage: Arc<Storage>,
}

impl TopicStorage {
    /// Create a new topic storage wrapper.
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Get underlying storage.
    pub fn storage(&self) -> &Arc<Storage> {
        &self.storage
    }

    // --- Topic CRUD ---

    /// Save a topic.
    #[instrument(skip(self, topic), fields(topic_id = %topic.topic_id))]
    pub fn save_topic(&self, topic: &Topic) -> Result<(), TopicsError> {
        let key = topic_key(&topic.topic_id);
        let value = serde_json::to_vec(topic)?;
        self.storage.put(CF_TOPICS, key.as_bytes(), &value)?;
        debug!("Saved topic");
        Ok(())
    }

    /// Get a topic by ID.
    #[instrument(skip(self))]
    pub fn get_topic(&self, topic_id: &str) -> Result<Option<Topic>, TopicsError> {
        let key = topic_key(topic_id);
        match self.storage.get(CF_TOPICS, key.as_bytes())? {
            Some(bytes) => {
                let topic: Topic = serde_json::from_slice(&bytes)?;
                Ok(Some(topic))
            }
            None => Ok(None),
        }
    }

    /// Delete a topic and its links.
    #[instrument(skip(self))]
    pub fn delete_topic(&self, topic_id: &str) -> Result<(), TopicsError> {
        // Delete topic
        let key = topic_key(topic_id);
        self.storage.delete(CF_TOPICS, key.as_bytes())?;

        // Delete links (would need iteration in production)
        // For now, links are cleaned up separately
        debug!("Deleted topic");
        Ok(())
    }

    /// List all active topics.
    pub fn list_topics(&self) -> Result<Vec<Topic>, TopicsError> {
        let prefix = b"topic:";
        let mut topics = Vec::new();

        for (_, value) in self.storage.prefix_iterator(CF_TOPICS, prefix)? {
            let topic: Topic = serde_json::from_slice(&value)?;
            if topic.status == TopicStatus::Active {
                topics.push(topic);
            }
        }

        Ok(topics)
    }

    /// List topics sorted by importance score (descending).
    pub fn list_topics_by_importance(&self, limit: usize) -> Result<Vec<Topic>, TopicsError> {
        let mut topics = self.list_topics()?;
        topics.sort_by(|a, b| {
            b.importance_score
                .partial_cmp(&a.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        topics.truncate(limit);
        Ok(topics)
    }

    // --- Topic Links ---

    /// Save a topic-node link.
    #[instrument(skip(self, link), fields(topic_id = %link.topic_id, node_id = %link.node_id))]
    pub fn save_link(&self, link: &TopicLink) -> Result<(), TopicsError> {
        let value = serde_json::to_vec(link)?;

        // Primary key: topic -> nodes
        let primary_key = topic_link_key(&link.topic_id, &link.node_id);
        self.storage
            .put(CF_TOPIC_LINKS, primary_key.as_bytes(), &value)?;

        // Secondary key: node -> topics
        let secondary_key = node_topic_key(&link.node_id, &link.topic_id);
        self.storage
            .put(CF_TOPIC_LINKS, secondary_key.as_bytes(), &value)?;

        debug!("Saved topic link");
        Ok(())
    }

    /// Get links for a topic.
    pub fn get_links_for_topic(&self, topic_id: &str) -> Result<Vec<TopicLink>, TopicsError> {
        let prefix = format!("link:{}:", topic_id);
        let mut links = Vec::new();

        for (_, value) in self
            .storage
            .prefix_iterator(CF_TOPIC_LINKS, prefix.as_bytes())?
        {
            let link: TopicLink = serde_json::from_slice(&value)?;
            links.push(link);
        }

        // Sort by relevance descending
        links.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(links)
    }

    /// Get topics for a node.
    pub fn get_topics_for_node(&self, node_id: &str) -> Result<Vec<TopicLink>, TopicsError> {
        let prefix = format!("node:{}:", node_id);
        let mut links = Vec::new();

        for (_, value) in self
            .storage
            .prefix_iterator(CF_TOPIC_LINKS, prefix.as_bytes())?
        {
            let link: TopicLink = serde_json::from_slice(&value)?;
            links.push(link);
        }

        Ok(links)
    }

    // --- Relationships ---

    /// Save a topic relationship.
    #[instrument(skip(self, rel), fields(source = %rel.source_id, target = %rel.target_id))]
    pub fn save_relationship(&self, rel: &TopicRelationship) -> Result<(), TopicsError> {
        let key = relationship_key(&rel.source_id, rel.relationship_type.code(), &rel.target_id);
        let value = serde_json::to_vec(rel)?;
        self.storage.put(CF_TOPIC_RELS, key.as_bytes(), &value)?;
        debug!("Saved relationship");
        Ok(())
    }

    /// Store a topic relationship (alias for save_relationship).
    #[instrument(skip(self, rel), fields(source = %rel.source_id, target = %rel.target_id))]
    pub fn store_relationship(&self, rel: &TopicRelationship) -> Result<(), TopicsError> {
        self.save_relationship(rel)
    }

    /// Get relationships for a topic.
    pub fn get_relationships(&self, topic_id: &str) -> Result<Vec<TopicRelationship>, TopicsError> {
        self.get_relationships_filtered(topic_id, None)
    }

    /// Get relationships for a topic, optionally filtered by type.
    pub fn get_relationships_filtered(
        &self,
        topic_id: &str,
        rel_type: Option<RelationshipType>,
    ) -> Result<Vec<TopicRelationship>, TopicsError> {
        let prefix = match rel_type {
            Some(rt) => format!("rel:{}:{}:", topic_id, rt.code()),
            None => format!("rel:{}:", topic_id),
        };

        let mut rels = Vec::new();

        for (_, value) in self
            .storage
            .prefix_iterator(CF_TOPIC_RELS, prefix.as_bytes())?
        {
            let rel: TopicRelationship = serde_json::from_slice(&value)?;
            rels.push(rel);
        }

        // Sort by strength descending
        rels.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(rels)
    }

    /// Get related topics with their relationship strength.
    ///
    /// Returns topics related to the given topic, optionally filtered by relationship type,
    /// sorted by strength descending and limited to `limit` results.
    pub fn get_related_topics(
        &self,
        topic_id: &str,
        rel_type: Option<RelationshipType>,
        limit: usize,
    ) -> Result<Vec<(String, f32)>, TopicsError> {
        let rels = self.get_relationships_filtered(topic_id, rel_type)?;
        let related: Vec<(String, f32)> = rels
            .into_iter()
            .take(limit)
            .map(|rel| (rel.target_id, rel.strength))
            .collect();
        Ok(related)
    }

    /// Update the strength of an existing relationship.
    ///
    /// Returns an error if the relationship doesn't exist.
    #[instrument(skip(self))]
    pub fn update_relationship_strength(
        &self,
        source_id: &str,
        target_id: &str,
        rel_type: RelationshipType,
        new_strength: f32,
    ) -> Result<(), TopicsError> {
        let key = relationship_key(source_id, rel_type.code(), target_id);

        // Load existing relationship
        let bytes = self
            .storage
            .get(CF_TOPIC_RELS, key.as_bytes())?
            .ok_or_else(|| {
                TopicsError::NotFound(format!(
                    "Relationship {}->{}:{}",
                    source_id,
                    target_id,
                    rel_type.code()
                ))
            })?;

        let mut rel: TopicRelationship = serde_json::from_slice(&bytes)?;
        rel.set_strength(new_strength);

        // Save updated relationship
        let value = serde_json::to_vec(&rel)?;
        self.storage.put(CF_TOPIC_RELS, key.as_bytes(), &value)?;

        debug!(new_strength = rel.strength, "Updated relationship strength");
        Ok(())
    }

    /// Get a specific relationship between two topics.
    pub fn get_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        rel_type: RelationshipType,
    ) -> Result<Option<TopicRelationship>, TopicsError> {
        let key = relationship_key(source_id, rel_type.code(), target_id);
        match self.storage.get(CF_TOPIC_RELS, key.as_bytes())? {
            Some(bytes) => {
                let rel: TopicRelationship = serde_json::from_slice(&bytes)?;
                Ok(Some(rel))
            }
            None => Ok(None),
        }
    }

    /// Delete a relationship between two topics.
    #[instrument(skip(self))]
    pub fn delete_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        rel_type: RelationshipType,
    ) -> Result<(), TopicsError> {
        let key = relationship_key(source_id, rel_type.code(), target_id);
        self.storage.delete(CF_TOPIC_RELS, key.as_bytes())?;
        debug!("Deleted relationship");
        Ok(())
    }

    /// Increment evidence count for a relationship and optionally strengthen it.
    ///
    /// If the relationship doesn't exist, creates a new one.
    #[instrument(skip(self))]
    pub fn record_relationship_evidence(
        &self,
        source_id: &str,
        target_id: &str,
        rel_type: RelationshipType,
        strength_delta: f32,
    ) -> Result<(), TopicsError> {
        let key = relationship_key(source_id, rel_type.code(), target_id);

        let rel = match self.storage.get(CF_TOPIC_RELS, key.as_bytes())? {
            Some(bytes) => {
                let mut existing: TopicRelationship = serde_json::from_slice(&bytes)?;
                existing.add_evidence();
                existing.strengthen(strength_delta);
                existing
            }
            None => {
                // Create new relationship
                TopicRelationship::new(
                    source_id.to_string(),
                    target_id.to_string(),
                    rel_type,
                    strength_delta.clamp(0.0, 1.0),
                )
            }
        };

        let value = serde_json::to_vec(&rel)?;
        self.storage.put(CF_TOPIC_RELS, key.as_bytes(), &value)?;

        debug!(
            evidence_count = rel.evidence_count,
            strength = rel.strength,
            "Recorded relationship evidence"
        );
        Ok(())
    }

    // --- Statistics ---

    /// Get topic graph statistics.
    pub fn get_stats(&self) -> Result<TopicStats, TopicsError> {
        let topics = self.list_topics()?;
        let topic_count = topics.len() as u64;

        // Count links (approximate via prefix scan)
        let link_count = self
            .storage
            .prefix_iterator(CF_TOPIC_LINKS, b"link:")?
            .len() as u64;

        // Count relationships
        let relationship_count = self.storage.prefix_iterator(CF_TOPIC_RELS, b"rel:")?.len() as u64;

        Ok(TopicStats {
            topic_count,
            link_count,
            relationship_count,
            last_extraction_ms: 0, // Set by extraction job
            half_life_days: 30,    // From config
            similarity_threshold: 0.75,
        })
    }

    // --- Importance Scoring ---

    /// Touch a topic to update its last_mentioned_at timestamp and recalculate importance.
    ///
    /// This updates the topic's timestamp to `now` and recalculates the importance
    /// score using the provided scorer. The node count is NOT incremented.
    ///
    /// # Arguments
    /// * `topic_id` - ID of the topic to touch
    /// * `scorer` - Importance scorer for recalculation
    /// * `now` - Current timestamp
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(TopicsError::NotFound)` if topic doesn't exist
    #[instrument(skip(self, scorer))]
    pub fn touch_topic(
        &self,
        topic_id: &str,
        scorer: &ImportanceScorer,
        now: DateTime<Utc>,
    ) -> Result<(), TopicsError> {
        let mut topic = self
            .get_topic(topic_id)?
            .ok_or_else(|| TopicsError::NotFound(topic_id.to_string()))?;

        scorer.touch_topic(&mut topic, now);
        self.save_topic(&topic)?;

        debug!(new_score = topic.importance_score, "Touched topic");
        Ok(())
    }

    /// Record a topic mention, incrementing node count and updating importance.
    ///
    /// This is used when a new node is linked to the topic. It:
    /// - Increments the node count
    /// - Updates last_mentioned_at to now
    /// - Recalculates the importance score
    ///
    /// # Arguments
    /// * `topic_id` - ID of the topic that was mentioned
    /// * `scorer` - Importance scorer for recalculation
    /// * `now` - Current timestamp
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(TopicsError::NotFound)` if topic doesn't exist
    #[instrument(skip(self, scorer))]
    pub fn record_topic_mention(
        &self,
        topic_id: &str,
        scorer: &ImportanceScorer,
        now: DateTime<Utc>,
    ) -> Result<(), TopicsError> {
        let mut topic = self
            .get_topic(topic_id)?
            .ok_or_else(|| TopicsError::NotFound(topic_id.to_string()))?;

        scorer.on_topic_mentioned(&mut topic, now);
        self.save_topic(&topic)?;

        debug!(
            node_count = topic.node_count,
            new_score = topic.importance_score,
            "Recorded topic mention"
        );
        Ok(())
    }

    /// Get top topics sorted by importance score.
    ///
    /// Alias for `list_topics_by_importance` with clearer naming.
    pub fn get_top_topics(&self, limit: usize) -> Result<Vec<Topic>, TopicsError> {
        self.list_topics_by_importance(limit)
    }

    /// Get topics for a specific agent, sorted by combined importance and relevance.
    ///
    /// Uses the indirect path: Topic -> TopicLink -> TocNode -> contributing_agents
    /// to find which topics a given agent has contributed to.
    ///
    /// # Arguments
    /// * `main_storage` - Main storage for TocNode lookups
    /// * `agent_id` - Agent identifier to filter by (case-insensitive)
    /// * `limit` - Maximum number of topics to return
    ///
    /// # Returns
    /// Vec of (Topic, agent_relevance) tuples sorted by importance * relevance descending.
    pub fn get_topics_for_agent(
        &self,
        main_storage: &Storage,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<(Topic, f32)>, TopicsError> {
        let agent_lower = agent_id.to_lowercase();
        let topics = self.list_topics()?;
        let mut scored_topics: Vec<(Topic, f32)> = Vec::new();

        for topic in topics {
            let links = self.get_links_for_topic(&topic.topic_id)?;
            let mut max_relevance: f32 = 0.0;
            let mut agent_found = false;

            for link in &links {
                // Look up TocNode via main storage to check contributing_agents
                if let Ok(Some(node)) = main_storage.get_toc_node(&link.node_id) {
                    if node.contributing_agents.contains(&agent_lower) {
                        agent_found = true;
                        if link.relevance > max_relevance {
                            max_relevance = link.relevance;
                        }
                    }
                }
            }

            if agent_found {
                scored_topics.push((topic, max_relevance));
            }
        }

        // Sort by importance_score * agent_relevance descending
        scored_topics.sort_by(|a, b| {
            let score_a = a.0.importance_score as f32 * a.1;
            let score_b = b.0.importance_score as f32 * b.1;
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored_topics.truncate(limit);
        Ok(scored_topics)
    }

    /// Refresh importance scores for all topics.
    ///
    /// This is intended to be run as a periodic background job to ensure
    /// topic scores reflect current decay. Only topics whose scores have
    /// changed are persisted.
    ///
    /// # Arguments
    /// * `scorer` - Importance scorer for recalculation
    ///
    /// # Returns
    /// Number of topics that were updated
    #[instrument(skip(self, scorer))]
    pub fn refresh_importance_scores(&self, scorer: &ImportanceScorer) -> Result<u32, TopicsError> {
        let now = Utc::now();
        let mut topics = self.list_topics()?;
        let updated = scorer.recalculate_all(&mut topics, now);

        // Persist updated topics
        for topic in &topics {
            self.save_topic(topic)?;
        }

        info!(
            updated_count = updated,
            total = topics.len(),
            "Refreshed importance scores"
        );
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_key() {
        assert_eq!(topic_key("abc123"), "topic:abc123");
    }

    #[test]
    fn test_topic_link_key() {
        assert_eq!(topic_link_key("t1", "n1"), "link:t1:n1");
    }

    #[test]
    fn test_node_topic_key() {
        assert_eq!(node_topic_key("n1", "t1"), "node:n1:t1");
    }

    #[test]
    fn test_relationship_key() {
        assert_eq!(relationship_key("t1", "sem", "t2"), "rel:t1:sem:t2");
    }

    #[test]
    fn test_relationship_key_with_type() {
        assert_eq!(
            relationship_key("topic-a", RelationshipType::Hierarchical.code(), "topic-b"),
            "rel:topic-a:hie:topic-b"
        );
    }

    #[test]
    fn test_relationship_key_co_occurrence() {
        assert_eq!(
            relationship_key("topic-x", RelationshipType::CoOccurrence.code(), "topic-y"),
            "rel:topic-x:coo:topic-y"
        );
    }

    #[test]
    fn test_cf_topic_relationships_alias() {
        assert_eq!(CF_TOPIC_RELATIONSHIPS, CF_TOPIC_RELS);
    }
}
