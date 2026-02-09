//! Memory client for connecting to the daemon.
//!
//! Per HOOK-02: Hook handlers call daemon's IngestEvent RPC.

use tonic::transport::Channel;
use tracing::{debug, info};

use memory_service::pb::{
    memory_service_client::MemoryServiceClient, BrowseTocRequest, Event as ProtoEvent,
    EventRole as ProtoEventRole, EventType as ProtoEventType, ExpandGripRequest, GetEventsRequest,
    GetNodeRequest, GetRelatedTopicsRequest, GetTocRootRequest, GetTopTopicsRequest,
    GetTopicGraphStatusRequest, GetTopicsByQueryRequest, GetVectorIndexStatusRequest,
    Grip as ProtoGrip, HybridSearchRequest, HybridSearchResponse, IngestEventRequest,
    TeleportSearchRequest, TeleportSearchResponse, TocNode as ProtoTocNode, Topic as ProtoTopic,
    VectorIndexStatus, VectorTeleportRequest, VectorTeleportResponse,
};
use memory_types::{Event, EventRole, EventType};

use crate::error::ClientError;

/// Default endpoint for the memory daemon.
pub const DEFAULT_ENDPOINT: &str = "http://[::1]:50051";

/// Client for communicating with the memory daemon.
pub struct MemoryClient {
    inner: MemoryServiceClient<Channel>,
}

impl MemoryClient {
    /// Connect to the memory daemon.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The gRPC endpoint (e.g., `http://localhost:50051`)
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Connection` if connection fails.
    pub async fn connect(endpoint: &str) -> Result<Self, ClientError> {
        info!("Connecting to memory daemon at {}", endpoint);
        let inner = MemoryServiceClient::connect(endpoint.to_string())
            .await
            .map_err(ClientError::Connection)?;
        Ok(Self { inner })
    }

    /// Connect to the default endpoint.
    pub async fn connect_default() -> Result<Self, ClientError> {
        Self::connect(DEFAULT_ENDPOINT).await
    }

    /// Ingest an event into the memory system.
    ///
    /// Per HOOK-02: Hook handlers call daemon's IngestEvent RPC.
    ///
    /// # Returns
    ///
    /// Returns the event_id and whether the event was newly created.
    pub async fn ingest(&mut self, event: Event) -> Result<(String, bool), ClientError> {
        debug!("Ingesting event: {}", event.event_id);

        let proto_event = event_to_proto(event);
        let request = tonic::Request::new(IngestEventRequest {
            event: Some(proto_event),
        });

        let response = self.inner.ingest_event(request).await?;
        let resp = response.into_inner();

        if resp.created {
            info!("Event ingested: {}", resp.event_id);
        } else {
            debug!("Event already existed (idempotent): {}", resp.event_id);
        }

        Ok((resp.event_id, resp.created))
    }

    /// Ingest multiple events in sequence.
    ///
    /// # Returns
    ///
    /// Returns the number of events newly created.
    pub async fn ingest_batch(&mut self, events: Vec<Event>) -> Result<usize, ClientError> {
        let mut created_count = 0;
        for event in events {
            let (_, created) = self.ingest(event).await?;
            if created {
                created_count += 1;
            }
        }
        Ok(created_count)
    }

    // ===== Query Methods =====

    /// Get root TOC nodes (year level).
    ///
    /// Per QRY-01: Returns top-level time nodes sorted by time descending.
    pub async fn get_toc_root(&mut self) -> Result<Vec<ProtoTocNode>, ClientError> {
        debug!("GetTocRoot request");
        let request = tonic::Request::new(GetTocRootRequest {});
        let response = self.inner.get_toc_root(request).await?;
        Ok(response.into_inner().nodes)
    }

    /// Get a specific TOC node by ID.
    ///
    /// Per QRY-02: Returns node with children and summary.
    pub async fn get_node(&mut self, node_id: &str) -> Result<Option<ProtoTocNode>, ClientError> {
        debug!("GetNode request: {}", node_id);
        let request = tonic::Request::new(GetNodeRequest {
            node_id: node_id.to_string(),
        });
        let response = self.inner.get_node(request).await?;
        Ok(response.into_inner().node)
    }

    /// Browse children of a TOC node with pagination.
    ///
    /// Per QRY-03: Supports pagination of children.
    pub async fn browse_toc(
        &mut self,
        parent_id: &str,
        limit: u32,
        continuation_token: Option<String>,
    ) -> Result<BrowseTocResult, ClientError> {
        debug!("BrowseToc request: parent={}, limit={}", parent_id, limit);
        let request = tonic::Request::new(BrowseTocRequest {
            parent_id: parent_id.to_string(),
            limit: limit as i32,
            continuation_token,
        });
        let response = self.inner.browse_toc(request).await?;
        let resp = response.into_inner();
        Ok(BrowseTocResult {
            children: resp.children,
            continuation_token: resp.continuation_token,
            has_more: resp.has_more,
        })
    }

    /// Get events in a time range.
    ///
    /// Per QRY-04: Retrieves raw events by time range.
    pub async fn get_events(
        &mut self,
        from_timestamp_ms: i64,
        to_timestamp_ms: i64,
        limit: u32,
    ) -> Result<GetEventsResult, ClientError> {
        debug!(
            "GetEvents request: from={} to={} limit={}",
            from_timestamp_ms, to_timestamp_ms, limit
        );
        let request = tonic::Request::new(GetEventsRequest {
            from_timestamp_ms,
            to_timestamp_ms,
            limit: limit as i32,
        });
        let response = self.inner.get_events(request).await?;
        let resp = response.into_inner();
        Ok(GetEventsResult {
            events: resp.events,
            has_more: resp.has_more,
        })
    }

    /// Expand a grip to show context events.
    ///
    /// Per QRY-05: Retrieves context around grip excerpt.
    pub async fn expand_grip(
        &mut self,
        grip_id: &str,
        events_before: Option<u32>,
        events_after: Option<u32>,
    ) -> Result<ExpandGripResult, ClientError> {
        debug!("ExpandGrip request: {}", grip_id);
        let request = tonic::Request::new(ExpandGripRequest {
            grip_id: grip_id.to_string(),
            events_before: events_before.map(|v| v as i32),
            events_after: events_after.map(|v| v as i32),
        });
        let response = self.inner.expand_grip(request).await?;
        let resp = response.into_inner();
        Ok(ExpandGripResult {
            grip: resp.grip,
            events_before: resp.events_before,
            excerpt_events: resp.excerpt_events,
            events_after: resp.events_after,
        })
    }

    // ===== Teleport Search Methods =====

    /// Search for TOC nodes or grips using BM25 keyword search.
    ///
    /// Per TEL-02: BM25 search returns ranked results.
    pub async fn teleport_search(
        &mut self,
        query: &str,
        doc_type: i32,
        limit: i32,
    ) -> Result<TeleportSearchResponse, ClientError> {
        debug!("TeleportSearch request: query={}", query);
        let request = tonic::Request::new(TeleportSearchRequest {
            query: query.to_string(),
            doc_type,
            limit,
            agent_filter: None,
        });
        let response = self.inner.teleport_search(request).await?;
        Ok(response.into_inner())
    }

    // ===== Vector Search Methods =====

    /// Search for TOC nodes or grips using vector semantic search.
    ///
    /// Per VEC-01: Vector similarity search using HNSW index.
    ///
    /// # Arguments
    ///
    /// * `query` - Query text to embed and search
    /// * `top_k` - Number of results to return
    /// * `min_score` - Minimum similarity score (0.0-1.0)
    /// * `target` - Target type filter (0=unspecified, 1=toc, 2=grip, 3=all)
    pub async fn vector_teleport(
        &mut self,
        query: &str,
        top_k: i32,
        min_score: f32,
        target: i32,
    ) -> Result<VectorTeleportResponse, ClientError> {
        debug!("VectorTeleport request: query={}", query);
        let request = tonic::Request::new(VectorTeleportRequest {
            query: query.to_string(),
            top_k,
            min_score,
            time_filter: None,
            target,
            agent_filter: None,
        });
        let response = self.inner.vector_teleport(request).await?;
        Ok(response.into_inner())
    }

    /// Hybrid BM25 + vector search using RRF fusion.
    ///
    /// Per VEC-02: Combines keyword and semantic matching.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query
    /// * `top_k` - Number of results to return
    /// * `mode` - Search mode (0=unspecified, 1=vector-only, 2=bm25-only, 3=hybrid)
    /// * `bm25_weight` - Weight for BM25 in fusion (0.0-1.0)
    /// * `vector_weight` - Weight for vector in fusion (0.0-1.0)
    /// * `target` - Target type filter (0=unspecified, 1=toc, 2=grip, 3=all)
    pub async fn hybrid_search(
        &mut self,
        query: &str,
        top_k: i32,
        mode: i32,
        bm25_weight: f32,
        vector_weight: f32,
        target: i32,
    ) -> Result<HybridSearchResponse, ClientError> {
        debug!("HybridSearch request: query={}, mode={}", query, mode);
        let request = tonic::Request::new(HybridSearchRequest {
            query: query.to_string(),
            top_k,
            mode,
            bm25_weight,
            vector_weight,
            time_filter: None,
            target,
            agent_filter: None,
        });
        let response = self.inner.hybrid_search(request).await?;
        Ok(response.into_inner())
    }

    /// Get vector index status and statistics.
    ///
    /// Per VEC-03: Observable index health and stats.
    pub async fn get_vector_index_status(&mut self) -> Result<VectorIndexStatus, ClientError> {
        debug!("GetVectorIndexStatus request");
        let request = tonic::Request::new(GetVectorIndexStatusRequest {});
        let response = self.inner.get_vector_index_status(request).await?;
        Ok(response.into_inner())
    }

    // ===== Topic Graph Methods (Phase 14) =====

    /// Get topic graph status and statistics.
    ///
    /// Per TOPIC-08: Topic graph discovery.
    pub async fn get_topic_graph_status(&mut self) -> Result<TopicGraphStatus, ClientError> {
        debug!("GetTopicGraphStatus request");
        let request = tonic::Request::new(GetTopicGraphStatusRequest {});
        let response = self.inner.get_topic_graph_status(request).await?;
        let resp = response.into_inner();
        Ok(TopicGraphStatus {
            topic_count: resp.topic_count,
            relationship_count: resp.relationship_count,
            last_updated: resp.last_updated,
            available: resp.available,
        })
    }

    /// Get topics matching a query.
    ///
    /// Searches topic labels and keywords for matches.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query (keywords to match)
    /// * `limit` - Maximum results to return
    pub async fn get_topics_by_query(
        &mut self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ProtoTopic>, ClientError> {
        debug!("GetTopicsByQuery request: query={}", query);
        let request = tonic::Request::new(GetTopicsByQueryRequest {
            query: query.to_string(),
            limit,
        });
        let response = self.inner.get_topics_by_query(request).await?;
        Ok(response.into_inner().topics)
    }

    /// Get topics related to a specific topic.
    ///
    /// # Arguments
    ///
    /// * `topic_id` - Topic to find related topics for
    /// * `rel_type` - Optional relationship type filter ("co-occurrence", "semantic", "hierarchical")
    /// * `limit` - Maximum results to return
    pub async fn get_related_topics(
        &mut self,
        topic_id: &str,
        rel_type: Option<&str>,
        limit: u32,
    ) -> Result<RelatedTopicsResult, ClientError> {
        debug!("GetRelatedTopics request: topic_id={}", topic_id);
        let request = tonic::Request::new(GetRelatedTopicsRequest {
            topic_id: topic_id.to_string(),
            relationship_type: rel_type.unwrap_or("").to_string(),
            limit,
        });
        let response = self.inner.get_related_topics(request).await?;
        let resp = response.into_inner();
        Ok(RelatedTopicsResult {
            related_topics: resp.related_topics,
            relationships: resp.relationships,
        })
    }

    /// Get top topics by importance score.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum results to return
    /// * `days` - Look back window in days for importance calculation
    pub async fn get_top_topics(
        &mut self,
        limit: u32,
        days: u32,
    ) -> Result<Vec<ProtoTopic>, ClientError> {
        debug!("GetTopTopics request: limit={}, days={}", limit, days);
        let request = tonic::Request::new(GetTopTopicsRequest { limit, days });
        let response = self.inner.get_top_topics(request).await?;
        Ok(response.into_inner().topics)
    }
}

/// Topic graph status.
#[derive(Debug)]
pub struct TopicGraphStatus {
    pub topic_count: u64,
    pub relationship_count: u64,
    pub last_updated: String,
    pub available: bool,
}

/// Result of get_related_topics operation.
#[derive(Debug)]
pub struct RelatedTopicsResult {
    pub related_topics: Vec<ProtoTopic>,
    pub relationships: Vec<memory_service::pb::TopicRelationship>,
}

/// Result of browse_toc operation.
#[derive(Debug)]
pub struct BrowseTocResult {
    pub children: Vec<ProtoTocNode>,
    pub continuation_token: Option<String>,
    pub has_more: bool,
}

/// Result of get_events operation.
#[derive(Debug)]
pub struct GetEventsResult {
    pub events: Vec<ProtoEvent>,
    pub has_more: bool,
}

/// Result of expand_grip operation.
#[derive(Debug)]
pub struct ExpandGripResult {
    pub grip: Option<ProtoGrip>,
    pub events_before: Vec<ProtoEvent>,
    pub excerpt_events: Vec<ProtoEvent>,
    pub events_after: Vec<ProtoEvent>,
}

/// Convert domain Event to proto Event.
fn event_to_proto(event: Event) -> ProtoEvent {
    let event_type = match event.event_type {
        EventType::SessionStart => ProtoEventType::SessionStart,
        EventType::UserMessage => ProtoEventType::UserMessage,
        EventType::AssistantMessage => ProtoEventType::AssistantMessage,
        EventType::ToolResult => ProtoEventType::ToolResult,
        EventType::AssistantStop => ProtoEventType::AssistantStop,
        EventType::SubagentStart => ProtoEventType::SubagentStart,
        EventType::SubagentStop => ProtoEventType::SubagentStop,
        EventType::SessionEnd => ProtoEventType::SessionEnd,
    };

    let role = match event.role {
        EventRole::User => ProtoEventRole::User,
        EventRole::Assistant => ProtoEventRole::Assistant,
        EventRole::System => ProtoEventRole::System,
        EventRole::Tool => ProtoEventRole::Tool,
    };

    ProtoEvent {
        event_id: event.event_id,
        session_id: event.session_id,
        timestamp_ms: event.timestamp.timestamp_millis(),
        event_type: event_type as i32,
        role: role as i32,
        text: event.text,
        metadata: event.metadata,
        agent: event.agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_event_to_proto_user_message() {
        let event = Event::new(
            "evt-1".to_string(),
            "session-1".to_string(),
            Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            "Hello!".to_string(),
        );

        let proto = event_to_proto(event);

        assert_eq!(proto.event_id, "evt-1");
        assert_eq!(proto.session_id, "session-1");
        assert_eq!(proto.event_type, ProtoEventType::UserMessage as i32);
        assert_eq!(proto.role, ProtoEventRole::User as i32);
        assert_eq!(proto.text, "Hello!");
    }

    #[test]
    fn test_event_to_proto_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());

        let event = Event::new(
            "evt-1".to_string(),
            "session-1".to_string(),
            Utc::now(),
            EventType::ToolResult,
            EventRole::Tool,
            "Result".to_string(),
        )
        .with_metadata(metadata);

        let proto = event_to_proto(event);

        assert_eq!(proto.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_event_to_proto_all_types() {
        // Test all event type mappings
        let types = vec![
            (EventType::SessionStart, ProtoEventType::SessionStart),
            (EventType::UserMessage, ProtoEventType::UserMessage),
            (
                EventType::AssistantMessage,
                ProtoEventType::AssistantMessage,
            ),
            (EventType::ToolResult, ProtoEventType::ToolResult),
            (EventType::AssistantStop, ProtoEventType::AssistantStop),
            (EventType::SubagentStart, ProtoEventType::SubagentStart),
            (EventType::SubagentStop, ProtoEventType::SubagentStop),
            (EventType::SessionEnd, ProtoEventType::SessionEnd),
        ];

        for (domain_type, proto_type) in types {
            let event = Event::new(
                "evt-1".to_string(),
                "session-1".to_string(),
                Utc::now(),
                domain_type,
                EventRole::System,
                "Test".to_string(),
            );
            let proto = event_to_proto(event);
            assert_eq!(proto.event_type, proto_type as i32);
        }
    }

    #[test]
    fn test_event_to_proto_all_roles() {
        let roles = vec![
            (EventRole::User, ProtoEventRole::User),
            (EventRole::Assistant, ProtoEventRole::Assistant),
            (EventRole::System, ProtoEventRole::System),
            (EventRole::Tool, ProtoEventRole::Tool),
        ];

        for (domain_role, proto_role) in roles {
            let event = Event::new(
                "evt-1".to_string(),
                "session-1".to_string(),
                Utc::now(),
                EventType::UserMessage,
                domain_role,
                "Test".to_string(),
            );
            let proto = event_to_proto(event);
            assert_eq!(proto.role, proto_role as i32);
        }
    }
}
