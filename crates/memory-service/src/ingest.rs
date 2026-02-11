//! IngestEvent RPC implementation.
//!
//! Handles event ingestion by:
//! 1. Converting proto Event to domain Event
//! 2. Storing in RocksDB with atomic outbox entry (ING-05)
//! 3. Returning idempotent result (ING-03)

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};

use memory_scheduler::SchedulerService;
use memory_search::TeleportSearcher;
use memory_storage::Storage;
use memory_types::{Event, EventRole, EventType, NoveltyConfig, OutboxEntry, SalienceConfig};

use crate::hybrid::HybridSearchHandler;
use crate::agents::AgentDiscoveryHandler;
use crate::pb::{
    memory_service_server::MemoryService, BrowseTocRequest, BrowseTocResponse,
    ClassifyQueryIntentRequest, ClassifyQueryIntentResponse, Event as ProtoEvent,
    EventRole as ProtoEventRole, EventType as ProtoEventType, ExpandGripRequest,
    ExpandGripResponse, GetAgentActivityRequest, GetAgentActivityResponse, GetEventsRequest,
    GetEventsResponse, GetNodeRequest, GetNodeResponse, GetRankingStatusRequest,
    GetRankingStatusResponse, GetRelatedTopicsRequest, GetRelatedTopicsResponse,
    GetRetrievalCapabilitiesRequest, GetRetrievalCapabilitiesResponse, GetSchedulerStatusRequest,
    GetSchedulerStatusResponse, GetTocRootRequest, GetTocRootResponse, GetTopTopicsRequest,
    GetTopTopicsResponse, GetTopicGraphStatusRequest, GetTopicGraphStatusResponse,
    GetTopicsByQueryRequest, GetTopicsByQueryResponse, GetVectorIndexStatusRequest,
    HybridSearchRequest, HybridSearchResponse, IngestEventRequest, IngestEventResponse,
    ListAgentsRequest, ListAgentsResponse, PauseJobRequest, PauseJobResponse,
    PruneBm25IndexRequest, PruneBm25IndexResponse, PruneVectorIndexRequest,
    PruneVectorIndexResponse, ResumeJobRequest, ResumeJobResponse, RouteQueryRequest,
    RouteQueryResponse, SearchChildrenRequest, SearchChildrenResponse, SearchNodeRequest,
    SearchNodeResponse, TeleportSearchRequest, TeleportSearchResponse, VectorIndexStatus,
    VectorTeleportRequest, VectorTeleportResponse,
};
use crate::query;
use crate::retrieval::RetrievalHandler;
use crate::scheduler_service::SchedulerGrpcService;
use crate::search_service;
use crate::teleport_service;
use crate::topics::TopicGraphHandler;
use crate::vector::VectorTeleportHandler;

/// Implementation of the MemoryService gRPC service.
pub struct MemoryServiceImpl {
    storage: Arc<Storage>,
    scheduler_service: Option<SchedulerGrpcService>,
    teleport_searcher: Option<Arc<TeleportSearcher>>,
    vector_service: Option<Arc<VectorTeleportHandler>>,
    hybrid_service: Option<Arc<HybridSearchHandler>>,
    topic_service: Option<Arc<TopicGraphHandler>>,
    retrieval_service: Option<Arc<RetrievalHandler>>,
    agent_service: Arc<AgentDiscoveryHandler>,
}

impl MemoryServiceImpl {
    /// Create a new MemoryServiceImpl with the given storage.
    pub fn new(storage: Arc<Storage>) -> Self {
        let retrieval = Arc::new(RetrievalHandler::new(storage.clone()));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: None,
            teleport_searcher: None,
            vector_service: None,
            hybrid_service: None,
            topic_service: None,
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with storage and scheduler.
    ///
    /// When scheduler is provided, the scheduler-related RPCs
    /// (GetSchedulerStatus, PauseJob, ResumeJob) will be functional.
    pub fn with_scheduler(storage: Arc<Storage>, scheduler: Arc<SchedulerService>) -> Self {
        let retrieval = Arc::new(RetrievalHandler::new(storage.clone()));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: Some(SchedulerGrpcService::new(scheduler)),
            teleport_searcher: None,
            vector_service: None,
            hybrid_service: None,
            topic_service: None,
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with storage, scheduler, and teleport searcher.
    ///
    /// When teleport searcher is provided, the TeleportSearch RPC will be functional.
    pub fn with_scheduler_and_search(
        storage: Arc<Storage>,
        scheduler: Arc<SchedulerService>,
        searcher: Arc<TeleportSearcher>,
    ) -> Self {
        let retrieval = Arc::new(RetrievalHandler::with_services(
            storage.clone(),
            Some(searcher.clone()),
            None,
            None,
        ));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: Some(SchedulerGrpcService::new(scheduler)),
            teleport_searcher: Some(searcher),
            vector_service: None,
            hybrid_service: None,
            topic_service: None,
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with storage and teleport searcher (no scheduler).
    pub fn with_search(storage: Arc<Storage>, searcher: Arc<TeleportSearcher>) -> Self {
        let retrieval = Arc::new(RetrievalHandler::with_services(
            storage.clone(),
            Some(searcher.clone()),
            None,
            None,
        ));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: None,
            teleport_searcher: Some(searcher),
            vector_service: None,
            hybrid_service: None,
            topic_service: None,
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with storage and vector search.
    ///
    /// When vector service is provided, VectorTeleport and HybridSearch RPCs will be functional.
    pub fn with_vector(storage: Arc<Storage>, vector_handler: Arc<VectorTeleportHandler>) -> Self {
        let hybrid_handler = Arc::new(HybridSearchHandler::new(vector_handler.clone()));
        let retrieval = Arc::new(RetrievalHandler::with_services(
            storage.clone(),
            None,
            Some(vector_handler.clone()),
            None,
        ));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: None,
            teleport_searcher: None,
            vector_service: Some(vector_handler),
            hybrid_service: Some(hybrid_handler),
            topic_service: None,
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with storage and topic graph.
    ///
    /// When topic service is provided, the topic graph RPCs will be functional.
    pub fn with_topics(storage: Arc<Storage>, topic_handler: Arc<TopicGraphHandler>) -> Self {
        let retrieval = Arc::new(RetrievalHandler::with_services(
            storage.clone(),
            None,
            None,
            Some(topic_handler.clone()),
        ));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: None,
            teleport_searcher: None,
            vector_service: None,
            hybrid_service: None,
            topic_service: Some(topic_handler),
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with all services.
    pub fn with_all_services(
        storage: Arc<Storage>,
        scheduler: Arc<SchedulerService>,
        searcher: Arc<TeleportSearcher>,
        vector_handler: Arc<VectorTeleportHandler>,
    ) -> Self {
        let hybrid_handler = Arc::new(HybridSearchHandler::new(vector_handler.clone()));
        let retrieval = Arc::new(RetrievalHandler::with_services(
            storage.clone(),
            Some(searcher.clone()),
            Some(vector_handler.clone()),
            None,
        ));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: Some(SchedulerGrpcService::new(scheduler)),
            teleport_searcher: Some(searcher),
            vector_service: Some(vector_handler),
            hybrid_service: Some(hybrid_handler),
            topic_service: None,
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Create a new MemoryServiceImpl with all services including topics.
    pub fn with_all_services_and_topics(
        storage: Arc<Storage>,
        scheduler: Arc<SchedulerService>,
        searcher: Arc<TeleportSearcher>,
        vector_handler: Arc<VectorTeleportHandler>,
        topic_handler: Arc<TopicGraphHandler>,
    ) -> Self {
        let hybrid_handler = Arc::new(HybridSearchHandler::new(vector_handler.clone()));
        let retrieval = Arc::new(RetrievalHandler::with_services(
            storage.clone(),
            Some(searcher.clone()),
            Some(vector_handler.clone()),
            Some(topic_handler.clone()),
        ));
        let agent_svc = Arc::new(AgentDiscoveryHandler::new(storage.clone()));
        Self {
            storage,
            scheduler_service: Some(SchedulerGrpcService::new(scheduler)),
            teleport_searcher: Some(searcher),
            vector_service: Some(vector_handler),
            hybrid_service: Some(hybrid_handler),
            topic_service: Some(topic_handler),
            retrieval_service: Some(retrieval),
            agent_service: agent_svc,
        }
    }

    /// Convert proto EventRole to domain EventRole
    fn convert_role(proto_role: ProtoEventRole) -> EventRole {
        match proto_role {
            ProtoEventRole::User => EventRole::User,
            ProtoEventRole::Assistant => EventRole::Assistant,
            ProtoEventRole::System => EventRole::System,
            ProtoEventRole::Tool => EventRole::Tool,
            ProtoEventRole::Unspecified => EventRole::User, // Default
        }
    }

    /// Convert proto EventType to domain EventType
    fn convert_event_type(proto_type: ProtoEventType) -> EventType {
        match proto_type {
            ProtoEventType::SessionStart => EventType::SessionStart,
            ProtoEventType::UserMessage => EventType::UserMessage,
            ProtoEventType::AssistantMessage => EventType::AssistantMessage,
            ProtoEventType::ToolResult => EventType::ToolResult,
            ProtoEventType::AssistantStop => EventType::AssistantStop,
            ProtoEventType::SubagentStart => EventType::SubagentStart,
            ProtoEventType::SubagentStop => EventType::SubagentStop,
            ProtoEventType::SessionEnd => EventType::SessionEnd,
            ProtoEventType::Unspecified => EventType::UserMessage, // Default
        }
    }

    /// Convert proto Event to domain Event
    #[allow(clippy::result_large_err)]
    fn convert_event(proto: ProtoEvent) -> Result<Event, Status> {
        let timestamp = Utc
            .timestamp_millis_opt(proto.timestamp_ms)
            .single()
            .ok_or_else(|| Status::invalid_argument("Invalid timestamp"))?;

        let role = Self::convert_role(
            ProtoEventRole::try_from(proto.role).unwrap_or(ProtoEventRole::Unspecified),
        );
        let event_type = Self::convert_event_type(
            ProtoEventType::try_from(proto.event_type).unwrap_or(ProtoEventType::Unspecified),
        );

        let mut event = Event::new(
            proto.event_id,
            proto.session_id,
            timestamp,
            event_type,
            role,
            proto.text,
        );

        if !proto.metadata.is_empty() {
            event = event.with_metadata(proto.metadata);
        }

        // Phase 18: Extract agent, normalize to lowercase, treat empty as None
        if let Some(agent) = proto.agent.filter(|s| !s.is_empty()) {
            event = event.with_agent(agent.to_lowercase());
        }

        Ok(event)
    }
}

#[tonic::async_trait]
impl MemoryService for MemoryServiceImpl {
    /// Ingest a conversation event.
    ///
    /// Per ING-01: Accepts Event message via gRPC.
    /// Per ING-03: Idempotent using event_id as key.
    /// Per ING-05: Outbox entry written atomically with event.
    async fn ingest_event(
        &self,
        request: Request<IngestEventRequest>,
    ) -> Result<Response<IngestEventResponse>, Status> {
        let req = request.into_inner();

        let proto_event = req
            .event
            .ok_or_else(|| Status::invalid_argument("Event is required"))?;

        // Validate event_id
        if proto_event.event_id.is_empty() {
            return Err(Status::invalid_argument("event_id is required"));
        }

        // Validate session_id
        if proto_event.session_id.is_empty() {
            return Err(Status::invalid_argument("session_id is required"));
        }

        debug!("Ingesting event: {}", proto_event.event_id);

        // Convert proto to domain type
        let event = Self::convert_event(proto_event)?;
        let event_id = event.event_id.clone();
        let timestamp_ms = event.timestamp_ms();

        // Serialize event for storage
        let event_bytes = event.to_bytes().map_err(|e| {
            error!("Failed to serialize event: {}", e);
            Status::internal("Failed to serialize event")
        })?;

        // Create outbox entry for async index updates (ING-05)
        let outbox_entry = OutboxEntry::for_toc(event_id.clone(), timestamp_ms);
        let outbox_bytes = outbox_entry.to_bytes().map_err(|e| {
            error!("Failed to serialize outbox entry: {}", e);
            Status::internal("Failed to serialize outbox entry")
        })?;

        // Store event with atomic outbox write
        let (_, created) = self
            .storage
            .put_event(&event_id, &event_bytes, &outbox_bytes)
            .map_err(|e| {
                error!("Failed to store event: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?;

        if created {
            info!("Stored new event: {}", event_id);
        } else {
            debug!("Event already exists (idempotent): {}", event_id);
        }

        Ok(Response::new(IngestEventResponse { event_id, created }))
    }

    /// Get root TOC nodes (year level).
    async fn get_toc_root(
        &self,
        request: Request<GetTocRootRequest>,
    ) -> Result<Response<GetTocRootResponse>, Status> {
        query::get_toc_root(self.storage.clone(), request).await
    }

    /// Get a specific TOC node by ID.
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        query::get_node(self.storage.clone(), request).await
    }

    /// Browse children of a TOC node with pagination.
    async fn browse_toc(
        &self,
        request: Request<BrowseTocRequest>,
    ) -> Result<Response<BrowseTocResponse>, Status> {
        query::browse_toc(self.storage.clone(), request).await
    }

    /// Get events in a time range.
    async fn get_events(
        &self,
        request: Request<GetEventsRequest>,
    ) -> Result<Response<GetEventsResponse>, Status> {
        query::get_events(self.storage.clone(), request).await
    }

    /// Expand a grip to show context events.
    async fn expand_grip(
        &self,
        request: Request<ExpandGripRequest>,
    ) -> Result<Response<ExpandGripResponse>, Status> {
        query::expand_grip(self.storage.clone(), request).await
    }

    /// Get scheduler and job status.
    ///
    /// Per SCHED-05: Job status observable via gRPC.
    async fn get_scheduler_status(
        &self,
        request: Request<GetSchedulerStatusRequest>,
    ) -> Result<Response<GetSchedulerStatusResponse>, Status> {
        match &self.scheduler_service {
            Some(svc) => svc.get_scheduler_status(request).await,
            None => Ok(Response::new(GetSchedulerStatusResponse {
                scheduler_running: false,
                jobs: vec![],
            })),
        }
    }

    /// Pause a scheduled job.
    async fn pause_job(
        &self,
        request: Request<PauseJobRequest>,
    ) -> Result<Response<PauseJobResponse>, Status> {
        match &self.scheduler_service {
            Some(svc) => svc.pause_job(request).await,
            None => Ok(Response::new(PauseJobResponse {
                success: false,
                error: Some("Scheduler not configured".to_string()),
            })),
        }
    }

    /// Resume a paused job.
    async fn resume_job(
        &self,
        request: Request<ResumeJobRequest>,
    ) -> Result<Response<ResumeJobResponse>, Status> {
        match &self.scheduler_service {
            Some(svc) => svc.resume_job(request).await,
            None => Ok(Response::new(ResumeJobResponse {
                success: false,
                error: Some("Scheduler not configured".to_string()),
            })),
        }
    }

    /// Search within a single TOC node.
    ///
    /// Per SEARCH-01: SearchNode searches node's fields for query terms.
    async fn search_node(
        &self,
        request: Request<SearchNodeRequest>,
    ) -> Result<Response<SearchNodeResponse>, Status> {
        search_service::search_node(Arc::clone(&self.storage), request).await
    }

    /// Search across children of a parent node.
    ///
    /// Per SEARCH-02: SearchChildren searches all children of parent.
    async fn search_children(
        &self,
        request: Request<SearchChildrenRequest>,
    ) -> Result<Response<SearchChildrenResponse>, Status> {
        search_service::search_children(Arc::clone(&self.storage), request).await
    }

    /// Teleport search for TOC nodes or grips using BM25 ranking.
    ///
    /// Per TEL-01 through TEL-04: BM25 search with relevance scores.
    async fn teleport_search(
        &self,
        request: Request<TeleportSearchRequest>,
    ) -> Result<Response<TeleportSearchResponse>, Status> {
        match &self.teleport_searcher {
            Some(searcher) => {
                teleport_service::handle_teleport_search(searcher.clone(), request).await
            }
            None => Err(Status::unavailable("Search index not configured")),
        }
    }

    /// Vector semantic search using HNSW index.
    ///
    /// Per VEC-01: Semantic similarity search over TOC nodes and grips.
    async fn vector_teleport(
        &self,
        request: Request<VectorTeleportRequest>,
    ) -> Result<Response<VectorTeleportResponse>, Status> {
        match &self.vector_service {
            Some(svc) => svc.vector_teleport(request).await,
            None => Err(Status::unavailable("Vector index not enabled")),
        }
    }

    /// Hybrid BM25 + vector search using RRF fusion.
    ///
    /// Per VEC-02: Combines BM25 and vector scores using RRF.
    async fn hybrid_search(
        &self,
        request: Request<HybridSearchRequest>,
    ) -> Result<Response<HybridSearchResponse>, Status> {
        match &self.hybrid_service {
            Some(svc) => svc.hybrid_search(request).await,
            None => Err(Status::unavailable("Vector index not enabled")),
        }
    }

    /// Get vector index status and statistics.
    ///
    /// Per VEC-03: Returns index availability and stats.
    async fn get_vector_index_status(
        &self,
        request: Request<GetVectorIndexStatusRequest>,
    ) -> Result<Response<VectorIndexStatus>, Status> {
        match &self.vector_service {
            Some(svc) => svc.get_vector_index_status(request).await,
            None => Ok(Response::new(VectorIndexStatus {
                available: false,
                vector_count: 0,
                dimension: 0,
                last_indexed: String::new(),
                index_path: String::new(),
                size_bytes: 0,
            })),
        }
    }

    /// Get topic graph status and statistics.
    ///
    /// Per TOPIC-08: Returns topic graph availability and stats.
    async fn get_topic_graph_status(
        &self,
        request: Request<GetTopicGraphStatusRequest>,
    ) -> Result<Response<GetTopicGraphStatusResponse>, Status> {
        match &self.topic_service {
            Some(svc) => svc.get_topic_graph_status(request).await,
            None => Ok(Response::new(GetTopicGraphStatusResponse {
                topic_count: 0,
                relationship_count: 0,
                last_updated: String::new(),
                available: false,
            })),
        }
    }

    /// Get topics matching a query.
    ///
    /// Per TOPIC-08: Search topics by keywords.
    async fn get_topics_by_query(
        &self,
        request: Request<GetTopicsByQueryRequest>,
    ) -> Result<Response<GetTopicsByQueryResponse>, Status> {
        match &self.topic_service {
            Some(svc) => svc.get_topics_by_query(request).await,
            None => Err(Status::unavailable("Topic graph not enabled")),
        }
    }

    /// Get topics related to a specific topic.
    ///
    /// Per TOPIC-08: Navigate topic relationships.
    async fn get_related_topics(
        &self,
        request: Request<GetRelatedTopicsRequest>,
    ) -> Result<Response<GetRelatedTopicsResponse>, Status> {
        match &self.topic_service {
            Some(svc) => svc.get_related_topics(request).await,
            None => Err(Status::unavailable("Topic graph not enabled")),
        }
    }

    /// Get top topics by importance score.
    ///
    /// Per TOPIC-08: Get most important topics.
    async fn get_top_topics(
        &self,
        request: Request<GetTopTopicsRequest>,
    ) -> Result<Response<GetTopTopicsResponse>, Status> {
        match &self.topic_service {
            Some(svc) => svc.get_top_topics(request).await,
            None => Err(Status::unavailable("Topic graph not enabled")),
        }
    }

    /// Get retrieval capabilities.
    ///
    /// Per RETR-01: Combined status check pattern.
    async fn get_retrieval_capabilities(
        &self,
        request: Request<GetRetrievalCapabilitiesRequest>,
    ) -> Result<Response<GetRetrievalCapabilitiesResponse>, Status> {
        match &self.retrieval_service {
            Some(svc) => svc.get_retrieval_capabilities(request).await,
            None => Err(Status::unavailable("Retrieval service not configured")),
        }
    }

    /// Classify query intent.
    ///
    /// Per RETR-04: Intent classification with keyword heuristics.
    async fn classify_query_intent(
        &self,
        request: Request<ClassifyQueryIntentRequest>,
    ) -> Result<Response<ClassifyQueryIntentResponse>, Status> {
        match &self.retrieval_service {
            Some(svc) => svc.classify_query_intent(request).await,
            None => Err(Status::unavailable("Retrieval service not configured")),
        }
    }

    /// Route a query through optimal layers.
    ///
    /// Per RETR-05: Fallback chains with explainability.
    async fn route_query(
        &self,
        request: Request<RouteQueryRequest>,
    ) -> Result<Response<RouteQueryResponse>, Status> {
        match &self.retrieval_service {
            Some(svc) => svc.route_query(request).await,
            None => Err(Status::unavailable("Retrieval service not configured")),
        }
    }

    /// Prune old vectors per lifecycle policy (FR-08).
    async fn prune_vector_index(
        &self,
        _request: Request<PruneVectorIndexRequest>,
    ) -> Result<Response<PruneVectorIndexResponse>, Status> {
        // TODO: Implement vector lifecycle pruning
        Ok(Response::new(PruneVectorIndexResponse {
            success: true,
            segments_pruned: 0,
            grips_pruned: 0,
            days_pruned: 0,
            weeks_pruned: 0,
            message: "Vector pruning not yet implemented".to_string(),
        }))
    }

    /// Prune old BM25 documents per lifecycle policy (FR-09).
    async fn prune_bm25_index(
        &self,
        _request: Request<PruneBm25IndexRequest>,
    ) -> Result<Response<PruneBm25IndexResponse>, Status> {
        // TODO: Implement BM25 lifecycle pruning
        Ok(Response::new(PruneBm25IndexResponse {
            success: true,
            segments_pruned: 0,
            grips_pruned: 0,
            days_pruned: 0,
            weeks_pruned: 0,
            optimized: false,
            message: "BM25 pruning not yet implemented".to_string(),
        }))
    }

    /// Get ranking and novelty status.
    ///
    /// Returns actual configuration values from SalienceConfig and NoveltyConfig defaults.
    /// Usage decay is always enabled (Phase 16 design).
    /// Vector/BM25 lifecycle status reflects whether the respective services are configured.
    async fn get_ranking_status(
        &self,
        _request: Request<GetRankingStatusRequest>,
    ) -> Result<Response<GetRankingStatusResponse>, Status> {
        let salience_config = SalienceConfig::default();
        let novelty_config = NoveltyConfig::default();

        Ok(Response::new(GetRankingStatusResponse {
            salience_enabled: salience_config.enabled,
            usage_decay_enabled: true, // Always active per Phase 16 design
            novelty_enabled: novelty_config.enabled,
            // In-memory only counters; return 0 for a fresh/stateless query
            novelty_checked_total: 0,
            novelty_rejected_total: 0,
            novelty_skipped_total: 0,
            // Vector lifecycle: enabled if vector service is configured
            vector_lifecycle_enabled: self.vector_service.is_some(),
            vector_last_prune_timestamp: 0, // No persistent prune history yet
            vector_last_prune_count: 0,
            // BM25 lifecycle: disabled by default per Bm25LifecycleConfig
            bm25_lifecycle_enabled: false,
            bm25_last_prune_timestamp: 0,
            bm25_last_prune_count: 0,
        }))
    }

    /// List all contributing agents with summary statistics.
    ///
    /// Per R4.3.1: Cross-agent discovery.
    async fn list_agents(
        &self,
        request: Request<ListAgentsRequest>,
    ) -> Result<Response<ListAgentsResponse>, Status> {
        self.agent_service.list_agents(request).await
    }

    /// Get agent activity bucketed by time period.
    ///
    /// Per R4.3.2: Agent activity timeline.
    async fn get_agent_activity(
        &self,
        request: Request<GetAgentActivityRequest>,
    ) -> Result<Response<GetAgentActivityResponse>, Status> {
        self.agent_service.get_agent_activity(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_service() -> (MemoryServiceImpl, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path()).unwrap();
        let service = MemoryServiceImpl::new(Arc::new(storage));
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_ingest_event_success() {
        let (service, _temp) = create_test_service();

        let request = Request::new(IngestEventRequest {
            event: Some(ProtoEvent {
                event_id: ulid::Ulid::new().to_string(),
                session_id: "session-123".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                event_type: ProtoEventType::UserMessage as i32,
                role: ProtoEventRole::User as i32,
                text: "Hello, world!".to_string(),
                metadata: HashMap::new(),
                agent: None,
            }),
        });

        let response = service.ingest_event(request).await.unwrap();
        let resp = response.into_inner();

        assert!(resp.created);
        assert!(!resp.event_id.is_empty());
    }

    #[tokio::test]
    async fn test_ingest_event_idempotent() {
        let (service, _temp) = create_test_service();

        let event_id = ulid::Ulid::new().to_string();
        let event = ProtoEvent {
            event_id: event_id.clone(),
            session_id: "session-123".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            event_type: ProtoEventType::UserMessage as i32,
            role: ProtoEventRole::User as i32,
            text: "Hello, world!".to_string(),
            metadata: HashMap::new(),
            agent: None,
        };

        // First ingestion
        let response1 = service
            .ingest_event(Request::new(IngestEventRequest {
                event: Some(event.clone()),
            }))
            .await
            .unwrap();

        // Second ingestion (same event_id)
        let response2 = service
            .ingest_event(Request::new(IngestEventRequest { event: Some(event) }))
            .await
            .unwrap();

        assert!(response1.into_inner().created);
        assert!(!response2.into_inner().created); // Idempotent
    }

    #[tokio::test]
    async fn test_ingest_event_missing_event() {
        let (service, _temp) = create_test_service();

        let request = Request::new(IngestEventRequest { event: None });
        let result = service.ingest_event(request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_ingest_event_missing_event_id() {
        let (service, _temp) = create_test_service();

        let request = Request::new(IngestEventRequest {
            event: Some(ProtoEvent {
                event_id: "".to_string(),
                session_id: "session-123".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                event_type: ProtoEventType::UserMessage as i32,
                role: ProtoEventRole::User as i32,
                text: "Hello, world!".to_string(),
                metadata: HashMap::new(),
                agent: None,
            }),
        });

        let result = service.ingest_event(request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_ingest_event_missing_session_id() {
        let (service, _temp) = create_test_service();

        let request = Request::new(IngestEventRequest {
            event: Some(ProtoEvent {
                event_id: ulid::Ulid::new().to_string(),
                session_id: "".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                event_type: ProtoEventType::UserMessage as i32,
                role: ProtoEventRole::User as i32,
                text: "Hello, world!".to_string(),
                metadata: HashMap::new(),
                agent: None,
            }),
        });

        let result = service.ingest_event(request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_ingest_event_with_metadata() {
        let (service, _temp) = create_test_service();

        let mut metadata = HashMap::new();
        metadata.insert("tool_name".to_string(), "Read".to_string());
        metadata.insert("file_path".to_string(), "/tmp/test.rs".to_string());

        let request = Request::new(IngestEventRequest {
            event: Some(ProtoEvent {
                event_id: ulid::Ulid::new().to_string(),
                session_id: "session-123".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                event_type: ProtoEventType::ToolResult as i32,
                role: ProtoEventRole::Tool as i32,
                text: "File contents here".to_string(),
                metadata,
                agent: None,
            }),
        });

        let response = service.ingest_event(request).await.unwrap();
        let resp = response.into_inner();

        assert!(resp.created);
    }

    #[tokio::test]
    async fn test_get_ranking_status_returns_defaults() {
        let (service, _temp) = create_test_service();

        let response = service
            .get_ranking_status(Request::new(GetRankingStatusRequest {}))
            .await
            .unwrap();

        let resp = response.into_inner();

        // Salience is enabled by default (SalienceConfig::default().enabled == true)
        assert!(resp.salience_enabled);

        // Usage decay is always active per Phase 16 design
        assert!(resp.usage_decay_enabled);

        // Novelty is disabled by default (NoveltyConfig::default().enabled == false)
        assert!(!resp.novelty_enabled);

        // Novelty counters are in-memory only, should be 0
        assert_eq!(resp.novelty_checked_total, 0);
        assert_eq!(resp.novelty_rejected_total, 0);
        assert_eq!(resp.novelty_skipped_total, 0);

        // Vector/BM25 lifecycle: no services configured in basic test service
        assert!(!resp.vector_lifecycle_enabled);
        assert!(!resp.bm25_lifecycle_enabled);
        assert_eq!(resp.vector_last_prune_timestamp, 0);
        assert_eq!(resp.vector_last_prune_count, 0);
        assert_eq!(resp.bm25_last_prune_timestamp, 0);
        assert_eq!(resp.bm25_last_prune_count, 0);
    }

    #[test]
    fn test_convert_event_with_agent() {
        // Test with agent present
        let proto = ProtoEvent {
            event_id: "test-123".to_string(),
            session_id: "session-1".to_string(),
            timestamp_ms: 1704067200000,
            event_type: ProtoEventType::UserMessage as i32,
            role: ProtoEventRole::User as i32,
            text: "Hello".to_string(),
            metadata: HashMap::new(),
            agent: Some("Claude".to_string()),
        };

        let event = MemoryServiceImpl::convert_event(proto).unwrap();
        assert_eq!(event.agent, Some("claude".to_string())); // Normalized to lowercase
    }

    #[test]
    fn test_convert_event_without_agent() {
        // Test without agent (None)
        let proto = ProtoEvent {
            event_id: "test-456".to_string(),
            session_id: "session-1".to_string(),
            timestamp_ms: 1704067200000,
            event_type: ProtoEventType::UserMessage as i32,
            role: ProtoEventRole::User as i32,
            text: "Hello".to_string(),
            metadata: HashMap::new(),
            agent: None,
        };

        let event = MemoryServiceImpl::convert_event(proto).unwrap();
        assert!(event.agent.is_none());
    }

    #[test]
    fn test_convert_event_with_empty_agent() {
        // Test with empty agent string (treated as None)
        let proto = ProtoEvent {
            event_id: "test-789".to_string(),
            session_id: "session-1".to_string(),
            timestamp_ms: 1704067200000,
            event_type: ProtoEventType::UserMessage as i32,
            role: ProtoEventRole::User as i32,
            text: "Hello".to_string(),
            metadata: HashMap::new(),
            agent: Some("".to_string()),
        };

        let event = MemoryServiceImpl::convert_event(proto).unwrap();
        assert!(event.agent.is_none()); // Empty string treated as None
    }
}
