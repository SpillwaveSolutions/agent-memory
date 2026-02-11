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
    memory_service_server::MemoryService, Event as ProtoEvent, EventRole as ProtoEventRole,
    EventType as ProtoEventType, IngestEventRequest,
};
use memory_service::MemoryServiceImpl;

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
