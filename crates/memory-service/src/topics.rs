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

use memory_topics::{RelationshipType, TopicStorage};

use crate::pb::{
    GetRelatedTopicsRequest, GetRelatedTopicsResponse, GetTopTopicsRequest, GetTopTopicsResponse,
    GetTopicGraphStatusRequest, GetTopicGraphStatusResponse, GetTopicsByQueryRequest,
    GetTopicsByQueryResponse, Topic as ProtoTopic, TopicRelationship as ProtoTopicRelationship,
};

/// Handler for topic graph operations.
pub struct TopicGraphHandler {
    storage: Arc<TopicStorage>,
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
    pub fn new(storage: Arc<TopicStorage>) -> Self {
        Self { storage }
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

        debug!(limit = limit, days = _days, "GetTopTopics request");

        // Get top topics sorted by importance
        let topics = self.storage.get_top_topics(limit).map_err(|e| {
            tracing::error!("Failed to get top topics: {}", e);
            Status::internal(format!("Failed to get top topics: {}", e))
        })?;

        // Note: The days parameter could be used to filter topics by last_mentioned_at
        // within the lookback window. For now, we rely on the importance scorer's
        // time-decay which already factors in recency. Future enhancement could
        // filter out topics not mentioned within the days window.
        let now = Utc::now();
        let cutoff = now - chrono::Duration::days(_days as i64);

        let filtered_topics: Vec<_> = topics
            .into_iter()
            .filter(|t| t.last_mentioned_at >= cutoff)
            .collect();

        let proto_topics: Vec<ProtoTopic> =
            filtered_topics.into_iter().map(topic_to_proto).collect();

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
}
