//! Error path E2E tests for agent-memory (E2E-08).
//!
//! Validates that malformed events and invalid queries are handled gracefully
//! with useful error messages containing field-level context.
//!
//! Every validation check in the service layer must produce a gRPC InvalidArgument
//! error mentioning the problematic field/value. No test should cause a panic.

use std::collections::HashMap;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::TestHarness;
use memory_service::pb::{
    memory_service_server::MemoryService, BrowseTocRequest, ClassifyQueryIntentRequest,
    Event as ProtoEvent, EventRole as ProtoEventRole, EventType as ProtoEventType,
    ExpandGripRequest, GetAgentActivityRequest, GetNodeRequest, IngestEventRequest,
    RouteQueryRequest,
};
use memory_service::{MemoryServiceImpl, RetrievalHandler};

// ===== Ingest Error Path Tests (E2E-08 ingest) =====

/// E2E-08: Ingest with missing event (None) returns InvalidArgument with "Event" context.
#[tokio::test]
async fn test_ingest_missing_event() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let request = Request::new(IngestEventRequest { event: None });
    let result = service.ingest_event(request).await;

    assert!(result.is_err(), "Expected error for missing event");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("Event"),
        "Error message should mention 'Event', got: {}",
        status.message()
    );
}

/// E2E-08: Ingest with empty event_id returns InvalidArgument with "event_id" context.
#[tokio::test]
async fn test_ingest_missing_event_id() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

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

    assert!(result.is_err(), "Expected error for empty event_id");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("event_id"),
        "Error message should mention 'event_id', got: {}",
        status.message()
    );
}

/// E2E-08: Ingest with empty session_id returns InvalidArgument with "session_id" context.
#[tokio::test]
async fn test_ingest_missing_session_id() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

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

    assert!(result.is_err(), "Expected error for empty session_id");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("session_id"),
        "Error message should mention 'session_id', got: {}",
        status.message()
    );
}

/// E2E-08: Ingest with extremely negative timestamp returns InvalidArgument with "timestamp" context.
#[tokio::test]
async fn test_ingest_invalid_timestamp() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let request = Request::new(IngestEventRequest {
        event: Some(ProtoEvent {
            event_id: ulid::Ulid::new().to_string(),
            session_id: "session-123".to_string(),
            timestamp_ms: i64::MAX,
            event_type: ProtoEventType::UserMessage as i32,
            role: ProtoEventRole::User as i32,
            text: "Hello, world!".to_string(),
            metadata: HashMap::new(),
            agent: None,
        }),
    });

    let result = service.ingest_event(request).await;

    assert!(result.is_err(), "Expected error for invalid timestamp");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().to_lowercase().contains("timestamp"),
        "Error message should mention 'timestamp', got: {}",
        status.message()
    );
}

/// E2E-08: Positive control — valid ingest succeeds (proves validation is not overly aggressive).
#[tokio::test]
async fn test_ingest_valid_event_succeeds() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let request = Request::new(IngestEventRequest {
        event: Some(ProtoEvent {
            event_id: ulid::Ulid::new().to_string(),
            session_id: "session-123".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            event_type: ProtoEventType::UserMessage as i32,
            role: ProtoEventRole::User as i32,
            text: "Hello, this is a valid event!".to_string(),
            metadata: HashMap::new(),
            agent: None,
        }),
    });

    let result = service.ingest_event(request).await;

    assert!(result.is_ok(), "Valid event should succeed");
    let response = result.unwrap().into_inner();
    assert!(response.created, "Event should be marked as created");
    assert!(!response.event_id.is_empty(), "Event ID should be set");
}

// ===== Query Error Path Tests (E2E-08 query) =====

/// E2E-08: RouteQuery with empty query returns InvalidArgument with "Query" context.
#[tokio::test]
async fn test_route_query_empty_query() {
    let harness = TestHarness::new();
    let handler = RetrievalHandler::with_services(harness.storage.clone(), None, None, None);

    let result = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: None,
        }))
        .await;

    assert!(result.is_err(), "Expected error for empty query");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("Query") || status.message().contains("query"),
        "Error message should mention 'Query' or 'query', got: {}",
        status.message()
    );
}

/// E2E-08: ClassifyQueryIntent with empty query returns InvalidArgument.
#[tokio::test]
async fn test_classify_intent_empty_query() {
    let harness = TestHarness::new();
    let handler = RetrievalHandler::with_services(harness.storage.clone(), None, None, None);

    let result = handler
        .classify_query_intent(Request::new(ClassifyQueryIntentRequest {
            query: "".to_string(),
            timeout_ms: None,
        }))
        .await;

    assert!(result.is_err(), "Expected error for empty query");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("Query") || status.message().contains("query"),
        "Error message should mention 'Query' or 'query', got: {}",
        status.message()
    );
}

// ===== Lookup Error Path Tests (E2E-08 lookup) =====

/// E2E-08: GetNode with empty node_id returns InvalidArgument with "node_id" context.
#[tokio::test]
async fn test_get_node_empty_id() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let result = service
        .get_node(Request::new(GetNodeRequest {
            node_id: "".to_string(),
        }))
        .await;

    assert!(result.is_err(), "Expected error for empty node_id");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("node_id"),
        "Error message should mention 'node_id', got: {}",
        status.message()
    );
}

/// E2E-08: ExpandGrip with empty grip_id returns InvalidArgument with "grip_id" context.
#[tokio::test]
async fn test_expand_grip_empty_id() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let result = service
        .expand_grip(Request::new(ExpandGripRequest {
            grip_id: "".to_string(),
            events_before: None,
            events_after: None,
        }))
        .await;

    assert!(result.is_err(), "Expected error for empty grip_id");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("grip_id"),
        "Error message should mention 'grip_id', got: {}",
        status.message()
    );
}

/// E2E-08: ExpandGrip with nonexistent grip_id returns graceful empty response (no panic).
#[tokio::test]
async fn test_expand_grip_nonexistent_graceful() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let result = service
        .expand_grip(Request::new(ExpandGripRequest {
            grip_id: "nonexistent-grip-12345".to_string(),
            events_before: None,
            events_after: None,
        }))
        .await;

    assert!(
        result.is_ok(),
        "Nonexistent grip should return Ok (graceful), not error"
    );
    let response = result.unwrap().into_inner();
    assert!(
        response.grip.is_none(),
        "Grip should be None for nonexistent ID"
    );
    assert!(
        response.excerpt_events.is_empty(),
        "excerpt_events should be empty"
    );
    assert!(
        response.events_before.is_empty(),
        "events_before should be empty"
    );
    assert!(
        response.events_after.is_empty(),
        "events_after should be empty"
    );
}

// ===== Navigation Error Path Tests (E2E-08 navigation) =====

/// E2E-08: BrowseToc with empty parent_id returns InvalidArgument with "parent_id" context.
#[tokio::test]
async fn test_browse_toc_empty_parent_id() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let result = service
        .browse_toc(Request::new(BrowseTocRequest {
            parent_id: "".to_string(),
            limit: 10,
            continuation_token: None,
        }))
        .await;

    assert!(result.is_err(), "Expected error for empty parent_id");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("parent_id"),
        "Error message should mention 'parent_id', got: {}",
        status.message()
    );
}

// ===== Agent Activity Error Path Tests (E2E-08 agent) =====

/// E2E-08: GetAgentActivity with invalid bucket returns InvalidArgument with "bucket" context.
#[tokio::test]
async fn test_get_agent_activity_invalid_bucket() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let result = service
        .get_agent_activity(Request::new(GetAgentActivityRequest {
            agent_id: None,
            from_ms: None,
            to_ms: None,
            bucket: "invalid_bucket".to_string(),
        }))
        .await;

    assert!(result.is_err(), "Expected error for invalid bucket");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(
        status.message().contains("bucket"),
        "Error message should mention 'bucket', got: {}",
        status.message()
    );
}
