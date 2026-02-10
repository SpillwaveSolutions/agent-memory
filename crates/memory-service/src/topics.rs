//! Topic Graph RPC implementations.
//!
//! Provides gRPC handlers for topic navigation:
//! - GetTopicGraphStatus: Check if topic graph is available
//! - GetTopicsByQuery: Search topics by keywords
//! - GetRelatedTopics: Get topics related to a given topic
//! - GetTopTopics: Get top topics by importance score

use std::sync::Arc;

use chrono::Utc;
use tonic::{Request, Response, Status};
use tracing::{debug, info};

use memory_storage::Storage;
use memory_topics::{RelationshipType, TopicStorage};

use crate::pb::{
    GetRelatedTopicsRequest, GetRelatedTopicsResponse, GetTopTopicsRequest, GetTopTopicsResponse,
    GetTopicGraphStatusRequest, GetTopicGraphStatusResponse, GetTopicsByQueryRequest,
    GetTopicsByQueryResponse, Topic as ProtoTopic, TopicRelationship as ProtoTopicRelationship,
};

/// Handler for topic graph operations.
pub struct TopicGraphHandler {
    storage: Arc<TopicStorage>,
    /// Main storage for TocNode lookups (used by agent-filtered topic queries).
    main_storage: Arc<Storage>,
}

/// Status of the topic graph.
pub struct TopicGraphStatus {
    pub available: bool,
    pub topic_count: u64,
    pub relationship_count: u64,
    pub last_updated: String,
}

/// Simplified topic search result for retrieval handler.
pub struct TopicSearchResult {
    pub id: String,
    pub label: String,
    pub importance_score: f32,
    pub keywords: Vec<String>,
}

impl TopicGraphHandler {
    /// Create a new topic graph handler.
    pub fn new(storage: Arc<TopicStorage>, main_storage: Arc<Storage>) -> Self {
        Self {
            storage,
            main_storage,
        }
    }

    /// Check if the topic graph is available.
    pub fn is_available(&self) -> bool {
        self.storage
            .get_stats()
            .map(|s| s.topic_count > 0)
            .unwrap_or(false)
    }

    /// Get the current topic graph status.
    pub async fn get_status(&self) -> TopicGraphStatus {
        let stats = self.storage.get_stats().unwrap_or_default();
        TopicGraphStatus {
            available: stats.topic_count > 0,
            topic_count: stats.topic_count,
            relationship_count: stats.relationship_count,
            last_updated: if stats.last_extraction_ms > 0 {
                chrono::DateTime::from_timestamp_millis(stats.last_extraction_ms)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            } else {
                String::new()
            },
        }
    }

    /// Direct search method for retrieval handler.
    ///
    /// Returns simplified results for use by the retrieval executor.
    pub async fn search_topics(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<TopicSearchResult>, String> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let all_topics = self
            .storage
            .list_topics()
            .map_err(|e| format!("Failed to list topics: {}", e))?;

        // Filter topics by query matching label or keywords
        let mut matching_topics: Vec<_> = all_topics
            .into_iter()
            .filter(|topic| {
                let label_lower = topic.label.to_lowercase();
                let keywords_lower: Vec<String> =
                    topic.keywords.iter().map(|k| k.to_lowercase()).collect();

                query_terms.iter().any(|term| {
                    label_lower.contains(term) || keywords_lower.iter().any(|k| k.contains(term))
                })
            })
            .collect();

        // Sort by importance score descending
        matching_topics.sort_by(|a, b| {
            b.importance_score
                .partial_cmp(&a.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results and convert to search results
        let results: Vec<TopicSearchResult> = matching_topics
            .into_iter()
            .take(limit as usize)
            .map(|t| TopicSearchResult {
                id: t.topic_id,
                label: t.label,
                importance_score: t.importance_score as f32,
                keywords: t.keywords,
            })
            .collect();

        Ok(results)
    }

    /// Handle GetTopicGraphStatus RPC request.
    pub async fn get_topic_graph_status(
        &self,
        _request: Request<GetTopicGraphStatusRequest>,
    ) -> Result<Response<GetTopicGraphStatusResponse>, Status> {
        debug!("GetTopicGraphStatus request");

        let stats = self.storage.get_stats().map_err(|e| {
            tracing::error!("Failed to get topic stats: {}", e);
            Status::internal(format!("Failed to get topic stats: {}", e))
        })?;

        let last_updated = if stats.last_extraction_ms > 0 {
            chrono::DateTime::from_timestamp_millis(stats.last_extraction_ms)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok(Response::new(GetTopicGraphStatusResponse {
            topic_count: stats.topic_count,
            relationship_count: stats.relationship_count,
            last_updated,
            available: stats.topic_count > 0,
        }))
    }

    /// Handle GetTopicsByQuery RPC request.
    pub async fn get_topics_by_query(
        &self,
        request: Request<GetTopicsByQueryRequest>,
    ) -> Result<Response<GetTopicsByQueryResponse>, Status> {
        let req = request.into_inner();
        let query = req.query.to_lowercase();
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            10
        };

        debug!(query = %query, limit = limit, "GetTopicsByQuery request");

        let all_topics = self.storage.list_topics().map_err(|e| {
            tracing::error!("Failed to list topics: {}", e);
            Status::internal(format!("Failed to list topics: {}", e))
        })?;

        // Filter topics by query matching label or keywords
        let query_terms: Vec<&str> = query.split_whitespace().collect();
        let mut matching_topics: Vec<_> = all_topics
            .into_iter()
            .filter(|topic| {
                let label_lower = topic.label.to_lowercase();
                let keywords_lower: Vec<String> =
                    topic.keywords.iter().map(|k| k.to_lowercase()).collect();

                query_terms.iter().any(|term| {
                    label_lower.contains(term) || keywords_lower.iter().any(|k| k.contains(term))
                })
            })
            .collect();

        // Sort by importance score descending
        matching_topics.sort_by(|a, b| {
            b.importance_score
                .partial_cmp(&a.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        matching_topics.truncate(limit);

        let proto_topics: Vec<ProtoTopic> =
            matching_topics.into_iter().map(topic_to_proto).collect();

        info!(query = %query, results = proto_topics.len(), "GetTopicsByQuery complete");

        Ok(Response::new(GetTopicsByQueryResponse {
            topics: proto_topics,
        }))
    }

    /// Handle GetRelatedTopics RPC request.
    pub async fn get_related_topics(
        &self,
        request: Request<GetRelatedTopicsRequest>,
    ) -> Result<Response<GetRelatedTopicsResponse>, Status> {
        let req = request.into_inner();
        let topic_id = &req.topic_id;
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            10
        };

        // Parse optional relationship type filter
        let rel_type_filter = if req.relationship_type.is_empty() {
            None
        } else {
            parse_relationship_type(&req.relationship_type)
        };

        debug!(
            topic_id = %topic_id,
            relationship_type = ?rel_type_filter,
            limit = limit,
            "GetRelatedTopics request"
        );

        // Verify source topic exists
        let _source_topic = self
            .storage
            .get_topic(topic_id)
            .map_err(|e| {
                tracing::error!("Failed to get topic: {}", e);
                Status::internal(format!("Failed to get topic: {}", e))
            })?
            .ok_or_else(|| Status::not_found(format!("Topic not found: {}", topic_id)))?;

        // Get relationships for the topic
        let relationships = self
            .storage
            .get_relationships_filtered(topic_id, rel_type_filter)
            .map_err(|e| {
                tracing::error!("Failed to get relationships: {}", e);
                Status::internal(format!("Failed to get relationships: {}", e))
            })?;

        // Limit relationships
        let limited_rels: Vec<_> = relationships.into_iter().take(limit).collect();

        // Fetch related topics
        let mut related_topics = Vec::new();
        let mut proto_relationships = Vec::new();

        for rel in &limited_rels {
            if let Ok(Some(topic)) = self.storage.get_topic(&rel.target_id) {
                related_topics.push(topic_to_proto(topic));
                proto_relationships.push(relationship_to_proto(rel));
            }
        }

        info!(
            topic_id = %topic_id,
            results = related_topics.len(),
            "GetRelatedTopics complete"
        );

        Ok(Response::new(GetRelatedTopicsResponse {
            related_topics,
            relationships: proto_relationships,
        }))
    }

    /// Handle GetTopTopics RPC request.
    ///
    /// When `agent_filter` is set, returns only topics that the specified agent
    /// has contributed to (via TopicLink -> TocNode -> contributing_agents).
    pub async fn get_top_topics(
        &self,
        request: Request<GetTopTopicsRequest>,
    ) -> Result<Response<GetTopTopicsResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            10
        };
        let _days = if req.days > 0 { req.days } else { 30 };
        let agent_filter = req.agent_filter.filter(|s| !s.is_empty());

        debug!(
            limit = limit,
            days = _days,
            agent_filter = ?agent_filter,
            "GetTopTopics request"
        );

        let now = Utc::now();
        let cutoff = now - chrono::Duration::days(_days as i64);

        let proto_topics: Vec<ProtoTopic> = if let Some(agent_id) = agent_filter {
            // Phase 23: Agent-filtered topic query
            let agent_topics = self
                .storage
                .get_topics_for_agent(&self.main_storage, &agent_id, limit)
                .map_err(|e| {
                    tracing::error!("Failed to get topics for agent: {}", e);
                    Status::internal(format!("Failed to get topics for agent: {}", e))
                })?;

            agent_topics
                .into_iter()
                .filter(|(t, _)| t.last_mentioned_at >= cutoff)
                .map(|(t, _relevance)| topic_to_proto(t))
                .collect()
        } else {
            // Existing behavior: return all top topics by importance
            let topics = self.storage.get_top_topics(limit).map_err(|e| {
                tracing::error!("Failed to get top topics: {}", e);
                Status::internal(format!("Failed to get top topics: {}", e))
            })?;

            topics
                .into_iter()
                .filter(|t| t.last_mentioned_at >= cutoff)
                .map(topic_to_proto)
                .collect()
        };

        info!(
            limit = limit,
            results = proto_topics.len(),
            "GetTopTopics complete"
        );

        Ok(Response::new(GetTopTopicsResponse {
            topics: proto_topics,
        }))
    }
}

/// Convert a domain Topic to a proto Topic.
fn topic_to_proto(topic: memory_topics::Topic) -> ProtoTopic {
    ProtoTopic {
        id: topic.topic_id,
        label: topic.label,
        importance_score: topic.importance_score as f32,
        keywords: topic.keywords,
        created_at: topic.created_at.to_rfc3339(),
        last_mention: topic.last_mentioned_at.to_rfc3339(),
    }
}

/// Convert a domain TopicRelationship to a proto TopicRelationship.
fn relationship_to_proto(rel: &memory_topics::TopicRelationship) -> ProtoTopicRelationship {
    ProtoTopicRelationship {
        source_id: rel.source_id.clone(),
        target_id: rel.target_id.clone(),
        relationship_type: rel.relationship_type.to_string(),
        strength: rel.strength,
    }
}

/// Parse a relationship type string to RelationshipType.
fn parse_relationship_type(s: &str) -> Option<RelationshipType> {
    match s.to_lowercase().as_str() {
        "co-occurrence" | "cooccurrence" | "coo" => Some(RelationshipType::CoOccurrence),
        "semantic" | "sem" => Some(RelationshipType::Semantic),
        "hierarchical" | "hie" => Some(RelationshipType::Hierarchical),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use memory_topics::{Topic, TopicRelationship};

    #[test]
    fn test_topic_to_proto() {
        let now = Utc::now();
        let topic = Topic {
            topic_id: "topic-123".to_string(),
            label: "Machine Learning".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            importance_score: 0.85,
            node_count: 10,
            created_at: now,
            last_mentioned_at: now,
            status: memory_topics::TopicStatus::Active,
            keywords: vec!["ml".to_string(), "ai".to_string()],
        };

        let proto = topic_to_proto(topic);

        assert_eq!(proto.id, "topic-123");
        assert_eq!(proto.label, "Machine Learning");
        assert!((proto.importance_score - 0.85).abs() < f32::EPSILON);
        assert_eq!(proto.keywords, vec!["ml", "ai"]);
    }

    #[test]
    fn test_relationship_to_proto() {
        let rel = TopicRelationship::new(
            "topic-a".to_string(),
            "topic-b".to_string(),
            RelationshipType::Semantic,
            0.75,
        );

        let proto = relationship_to_proto(&rel);

        assert_eq!(proto.source_id, "topic-a");
        assert_eq!(proto.target_id, "topic-b");
        assert_eq!(proto.relationship_type, "semantic");
        assert!((proto.strength - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_relationship_type() {
        assert_eq!(
            parse_relationship_type("co-occurrence"),
            Some(RelationshipType::CoOccurrence)
        );
        assert_eq!(
            parse_relationship_type("semantic"),
            Some(RelationshipType::Semantic)
        );
        assert_eq!(
            parse_relationship_type("hierarchical"),
            Some(RelationshipType::Hierarchical)
        );
        assert_eq!(
            parse_relationship_type("coo"),
            Some(RelationshipType::CoOccurrence)
        );
        assert_eq!(
            parse_relationship_type("sem"),
            Some(RelationshipType::Semantic)
        );
        assert_eq!(
            parse_relationship_type("hie"),
            Some(RelationshipType::Hierarchical)
        );
        assert_eq!(parse_relationship_type("unknown"), None);
        assert_eq!(parse_relationship_type(""), None);
    }

    // === Phase 23: Agent-filtered GetTopTopics integration tests ===

    use memory_topics::{TopicLink, TopicStorage};
    use memory_types::{TocLevel, TocNode};
    use tempfile::TempDir;

    /// Helper: create storage and TopicGraphHandler for integration tests.
    fn create_test_handler() -> (TempDir, Arc<TopicGraphHandler>) {
        let dir = TempDir::new().unwrap();
        let storage = Arc::new(memory_storage::Storage::open(dir.path()).unwrap());
        let topic_storage = Arc::new(TopicStorage::new(storage.clone()));
        let handler = Arc::new(TopicGraphHandler::new(topic_storage, storage));
        (dir, handler)
    }

    /// Helper: create a test topic with known importance score.
    fn make_topic(id: &str, label: &str, importance: f64) -> Topic {
        let now = Utc::now();
        Topic {
            topic_id: id.to_string(),
            label: label.to_string(),
            embedding: vec![0.1, 0.2],
            importance_score: importance,
            node_count: 5,
            created_at: now,
            last_mentioned_at: now,
            status: memory_topics::TopicStatus::Active,
            keywords: vec!["test".to_string()],
        }
    }

    /// Helper: store a TocNode with contributing agents.
    fn store_node(storage: &memory_storage::Storage, node_id: &str, agents: &[&str]) {
        let now = Utc::now();
        let mut node = TocNode::new(
            node_id.to_string(),
            TocLevel::Day,
            format!("Node {}", node_id),
            now,
            now,
        );
        for agent in agents {
            node = node.with_contributing_agent(*agent);
        }
        storage.put_toc_node(&node).unwrap();
    }

    #[tokio::test]
    async fn test_get_top_topics_without_agent_filter() {
        let (_dir, handler) = create_test_handler();

        // Add topics directly to storage
        let t1 = make_topic("t1", "Topic One", 0.9);
        let t2 = make_topic("t2", "Topic Two", 0.5);
        handler.storage.save_topic(&t1).unwrap();
        handler.storage.save_topic(&t2).unwrap();

        // Call GetTopTopics without agent_filter
        let request = tonic::Request::new(GetTopTopicsRequest {
            limit: 10,
            days: 30,
            agent_filter: None,
        });

        let response = handler.get_top_topics(request).await.unwrap();
        let topics = response.into_inner().topics;

        assert_eq!(topics.len(), 2, "Should return all topics");
        // Should be sorted by importance descending
        assert_eq!(topics[0].id, "t1");
        assert_eq!(topics[1].id, "t2");
    }

    #[tokio::test]
    async fn test_get_top_topics_with_agent_filter() {
        let (_dir, handler) = create_test_handler();

        // Create topics
        let t1 = make_topic("t1", "Claude Topic", 0.9);
        let t2 = make_topic("t2", "OpenCode Topic", 0.8);
        let t3 = make_topic("t3", "Shared Topic", 0.7);
        handler.storage.save_topic(&t1).unwrap();
        handler.storage.save_topic(&t2).unwrap();
        handler.storage.save_topic(&t3).unwrap();

        // Store TocNodes with agents
        store_node(&handler.main_storage, "node-1", &["claude"]);
        store_node(&handler.main_storage, "node-2", &["opencode"]);
        store_node(&handler.main_storage, "node-3", &["claude", "opencode"]);

        // Create topic links
        handler
            .storage
            .save_link(&TopicLink::new(
                "t1".to_string(),
                "node-1".to_string(),
                0.9,
            ))
            .unwrap();
        handler
            .storage
            .save_link(&TopicLink::new(
                "t2".to_string(),
                "node-2".to_string(),
                0.8,
            ))
            .unwrap();
        handler
            .storage
            .save_link(&TopicLink::new(
                "t3".to_string(),
                "node-3".to_string(),
                0.7,
            ))
            .unwrap();

        // Filter by "claude" - should get t1 (node-1 has claude) and t3 (node-3 has claude)
        let request = tonic::Request::new(GetTopTopicsRequest {
            limit: 10,
            days: 30,
            agent_filter: Some("claude".to_string()),
        });

        let response = handler.get_top_topics(request).await.unwrap();
        let topics = response.into_inner().topics;

        assert_eq!(topics.len(), 2, "Should return 2 topics for claude");
        let ids: Vec<&str> = topics.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"t1"), "Should include t1 (claude)");
        assert!(ids.contains(&"t3"), "Should include t3 (shared)");
        assert!(!ids.contains(&"t2"), "Should NOT include t2 (opencode only)");
    }

    #[tokio::test]
    async fn test_get_top_topics_agent_filter_empty_string_ignored() {
        let (_dir, handler) = create_test_handler();

        let t1 = make_topic("t1", "Any Topic", 0.9);
        handler.storage.save_topic(&t1).unwrap();

        // Empty string agent_filter should behave like no filter
        let request = tonic::Request::new(GetTopTopicsRequest {
            limit: 10,
            days: 30,
            agent_filter: Some(String::new()),
        });

        let response = handler.get_top_topics(request).await.unwrap();
        let topics = response.into_inner().topics;

        assert_eq!(topics.len(), 1, "Empty filter should return all topics");
    }
}
