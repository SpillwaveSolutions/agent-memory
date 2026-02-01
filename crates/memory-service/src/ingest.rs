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
use memory_storage::Storage;
use memory_types::{Event, EventRole, EventType, OutboxEntry};

use crate::pb::{
    memory_service_server::MemoryService,
    BrowseTocRequest, BrowseTocResponse,
    Event as ProtoEvent,
    EventRole as ProtoEventRole,
    EventType as ProtoEventType,
    ExpandGripRequest, ExpandGripResponse,
    GetEventsRequest, GetEventsResponse,
    GetNodeRequest, GetNodeResponse,
    GetSchedulerStatusRequest, GetSchedulerStatusResponse,
    GetTocRootRequest, GetTocRootResponse,
    IngestEventRequest,
    IngestEventResponse,
    PauseJobRequest, PauseJobResponse,
    ResumeJobRequest, ResumeJobResponse,
};
use crate::query;
use crate::scheduler_service::SchedulerGrpcService;

/// Implementation of the MemoryService gRPC service.
pub struct MemoryServiceImpl {
    storage: Arc<Storage>,
    scheduler_service: Option<SchedulerGrpcService>,
}

impl MemoryServiceImpl {
    /// Create a new MemoryServiceImpl with the given storage.
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            scheduler_service: None,
        }
    }

    /// Create a new MemoryServiceImpl with storage and scheduler.
    ///
    /// When scheduler is provided, the scheduler-related RPCs
    /// (GetSchedulerStatus, PauseJob, ResumeJob) will be functional.
    pub fn with_scheduler(storage: Arc<Storage>, scheduler: Arc<SchedulerService>) -> Self {
        Self {
            storage,
            scheduler_service: Some(SchedulerGrpcService::new(scheduler)),
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
    fn convert_event(proto: ProtoEvent) -> Result<Event, Status> {
        let timestamp = Utc
            .timestamp_millis_opt(proto.timestamp_ms)
            .single()
            .ok_or_else(|| Status::invalid_argument("Invalid timestamp"))?;

        let role = Self::convert_role(
            ProtoEventRole::try_from(proto.role).unwrap_or(ProtoEventRole::Unspecified)
        );
        let event_type = Self::convert_event_type(
            ProtoEventType::try_from(proto.event_type).unwrap_or(ProtoEventType::Unspecified)
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

        let proto_event = req.event.ok_or_else(|| {
            Status::invalid_argument("Event is required")
        })?;

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
        let (_, created) = self.storage.put_event(&event_id, &event_bytes, &outbox_bytes)
            .map_err(|e| {
                error!("Failed to store event: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?;

        if created {
            info!("Stored new event: {}", event_id);
        } else {
            debug!("Event already exists (idempotent): {}", event_id);
        }

        Ok(Response::new(IngestEventResponse {
            event_id,
            created,
        }))
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
        };

        // First ingestion
        let response1 = service.ingest_event(Request::new(IngestEventRequest {
            event: Some(event.clone()),
        })).await.unwrap();

        // Second ingestion (same event_id)
        let response2 = service.ingest_event(Request::new(IngestEventRequest {
            event: Some(event),
        })).await.unwrap();

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
            }),
        });

        let response = service.ingest_event(request).await.unwrap();
        let resp = response.into_inner();

        assert!(resp.created);
    }
}
