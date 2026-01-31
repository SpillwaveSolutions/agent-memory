//! Integration tests for the agent-memory system.
//!
//! These tests validate the complete workflow from event ingestion
//! through query resolution.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::time::sleep;

use memory_client::{map_hook_event, HookEvent, HookEventType, MemoryClient};
use memory_service::run_server_with_shutdown;
use memory_storage::Storage;
use memory_types::{Event, EventRole, EventType};

/// Test harness that manages daemon lifecycle.
struct TestHarness {
    _temp_dir: TempDir,
    storage: Arc<Storage>,
    endpoint: String,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    _server_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
}

impl TestHarness {
    /// Create a new test harness with a running server.
    async fn new(port: u16) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage = Arc::new(Storage::open(temp_dir.path()).expect("Failed to open storage"));

        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let service_storage = storage.clone();
        let server_handle = tokio::spawn(async move {
            run_server_with_shutdown(
                addr,
                service_storage,
                async {
                    shutdown_rx.await.ok();
                },
            )
            .await
        });

        // Wait for server to start
        sleep(Duration::from_millis(200)).await;

        let endpoint = format!("http://127.0.0.1:{}", port);

        Self {
            _temp_dir: temp_dir,
            storage,
            endpoint,
            shutdown_tx: Some(shutdown_tx),
            _server_handle: server_handle,
        }
    }

    /// Create a client connected to this harness.
    async fn client(&self) -> MemoryClient {
        // Retry connection a few times
        for _ in 0..5 {
            match MemoryClient::connect(&self.endpoint).await {
                Ok(client) => return client,
                Err(_) => sleep(Duration::from_millis(100)).await,
            }
        }
        panic!("Failed to connect to server at {}", self.endpoint);
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

// ==================== Event Ingestion Tests ====================

#[tokio::test]
async fn test_event_ingestion_lifecycle() {
    let harness = TestHarness::new(50100).await;
    let mut client = harness.client().await;

    // Create conversation events using hook mapping
    let session_id = "test-session-123";
    let events = vec![
        HookEvent::new(session_id, HookEventType::SessionStart, "Session started"),
        HookEvent::new(
            session_id,
            HookEventType::UserPromptSubmit,
            "What is Rust?",
        ),
        HookEvent::new(
            session_id,
            HookEventType::AssistantResponse,
            "Rust is a systems programming language.",
        ),
    ];

    // Ingest events
    for hook_event in events {
        let event = map_hook_event(hook_event);
        let (event_id, created) = client.ingest(event).await.unwrap();
        assert!(created);
        assert!(!event_id.is_empty());
    }

    // Verify events are stored
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 3);
}

#[tokio::test]
async fn test_event_idempotent_ingestion() {
    let harness = TestHarness::new(50101).await;
    let mut client = harness.client().await;

    // Create a single event
    let event = Event::new(
        ulid::Ulid::new().to_string(),
        "session-456".to_string(),
        chrono::Utc::now(),
        EventType::UserMessage,
        EventRole::User,
        "Hello, world!".to_string(),
    );
    let event_id = event.event_id.clone();

    // First ingestion
    let (_, created1) = client.ingest(event.clone()).await.unwrap();
    assert!(created1);

    // Second ingestion (same event)
    let (_, created2) = client.ingest(event).await.unwrap();
    assert!(!created2); // Should be idempotent

    // Still only one event
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 1);

    // Event ID should match
    assert!(!event_id.is_empty());
}

#[tokio::test]
async fn test_event_with_metadata() {
    let harness = TestHarness::new(50102).await;
    let mut client = harness.client().await;

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("tool_name".to_string(), "Read".to_string());
    metadata.insert("file_path".to_string(), "/tmp/test.rs".to_string());

    let hook_event =
        HookEvent::new("session-789", HookEventType::ToolResult, "File contents here")
            .with_tool_name("Read")
            .with_metadata(metadata);

    let event = map_hook_event(hook_event);
    let (event_id, created) = client.ingest(event).await.unwrap();

    assert!(created);
    assert!(!event_id.is_empty());
}

// ==================== TOC Query Tests ====================

#[tokio::test]
async fn test_get_toc_root_empty() {
    let harness = TestHarness::new(50103).await;
    let mut client = harness.client().await;

    // No events, no TOC nodes
    let root_nodes = client.get_toc_root().await.unwrap();
    assert!(root_nodes.is_empty());
}

#[tokio::test]
async fn test_get_node_not_found() {
    let harness = TestHarness::new(50104).await;
    let mut client = harness.client().await;

    let node = client.get_node("toc:year:2099").await.unwrap();
    assert!(node.is_none());
}

#[tokio::test]
async fn test_browse_toc_empty() {
    let harness = TestHarness::new(50105).await;
    let mut client = harness.client().await;

    let result = client
        .browse_toc("toc:year:2026", 10, None)
        .await
        .unwrap();
    assert!(result.children.is_empty());
    assert!(!result.has_more);
}

// ==================== Event Retrieval Tests ====================

#[tokio::test]
async fn test_get_events_in_range() {
    let harness = TestHarness::new(50106).await;
    let mut client = harness.client().await;

    // Ingest some events
    let now = chrono::Utc::now();
    let now_ms = now.timestamp_millis();

    for i in 0..5 {
        let event = Event::new(
            ulid::Ulid::new().to_string(),
            "session-range".to_string(),
            now + chrono::Duration::milliseconds(i * 100),
            EventType::UserMessage,
            EventRole::User,
            format!("Message {}", i),
        );
        client.ingest(event).await.unwrap();
    }

    // Query all events
    let result = client
        .get_events(now_ms - 1000, now_ms + 1000, 10)
        .await
        .unwrap();

    assert_eq!(result.events.len(), 5);
    assert!(!result.has_more);
}

#[tokio::test]
async fn test_get_events_with_limit() {
    let harness = TestHarness::new(50107).await;
    let mut client = harness.client().await;

    let now = chrono::Utc::now();
    let now_ms = now.timestamp_millis();

    // Ingest 10 events
    for i in 0..10 {
        let event = Event::new(
            ulid::Ulid::new().to_string(),
            "session-limit".to_string(),
            now + chrono::Duration::milliseconds(i * 100),
            EventType::UserMessage,
            EventRole::User,
            format!("Message {}", i),
        );
        client.ingest(event).await.unwrap();
    }

    // Query with limit
    let result = client
        .get_events(now_ms - 1000, now_ms + 2000, 5)
        .await
        .unwrap();

    assert_eq!(result.events.len(), 5);
    assert!(result.has_more);
}

// ==================== Grip Expansion Tests ====================

#[tokio::test]
async fn test_expand_grip_not_found() {
    let harness = TestHarness::new(50108).await;
    let mut client = harness.client().await;

    let result = client
        .expand_grip("grip:nonexistent", Some(2), Some(2))
        .await
        .unwrap();

    assert!(result.grip.is_none());
}

// ==================== Storage Tests ====================

#[tokio::test]
async fn test_storage_stats() {
    let harness = TestHarness::new(50109).await;
    let mut client = harness.client().await;

    // Ingest some events
    for i in 0..3 {
        let event = Event::new(
            ulid::Ulid::new().to_string(),
            "session-stats".to_string(),
            chrono::Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            format!("Message {}", i),
        );
        client.ingest(event).await.unwrap();
    }

    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 3);
    assert!(stats.disk_usage_bytes > 0);
}

// ==================== Crash Recovery Tests ====================

#[tokio::test]
async fn test_crash_recovery_events_persist() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();

    // First session: create storage and ingest events
    {
        let storage = Storage::open(db_path).unwrap();
        let event = Event::new(
            ulid::Ulid::new().to_string(),
            "session-crash".to_string(),
            chrono::Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            "Test message".to_string(),
        );
        let bytes = event.to_bytes().unwrap();
        storage.put_event(&event.event_id, &bytes, &[]).unwrap();
    } // Storage closed - simulates crash

    // Second session: reopen storage - should recover
    {
        let storage = Storage::open(db_path).unwrap();
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.event_count, 1);
    }
}

#[tokio::test]
async fn test_crash_recovery_checkpoint_persists() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();

    let checkpoint_data = b"checkpoint state v1";

    // First session: write checkpoint
    {
        let storage = Storage::open(db_path).unwrap();
        storage.put_checkpoint("test-job", checkpoint_data).unwrap();
    }

    // Second session: checkpoint should be recoverable
    {
        let storage = Storage::open(db_path).unwrap();
        let checkpoint = storage.get_checkpoint("test-job").unwrap();
        assert!(checkpoint.is_some());
        assert_eq!(checkpoint.unwrap(), checkpoint_data.to_vec());
    }
}

// ==================== Hook Mapping Tests ====================

#[tokio::test]
async fn test_hook_event_mapping_all_types() {
    let harness = TestHarness::new(50110).await;
    let mut client = harness.client().await;

    let session_id = "session-hook-types";

    let hook_types = vec![
        HookEventType::SessionStart,
        HookEventType::UserPromptSubmit,
        HookEventType::AssistantResponse,
        HookEventType::ToolUse,
        HookEventType::ToolResult,
        HookEventType::SubagentStart,
        HookEventType::SubagentStop,
        HookEventType::Stop,
    ];

    for hook_type in hook_types {
        let hook_event = HookEvent::new(session_id, hook_type, "Test content");
        let event = map_hook_event(hook_event);
        let (_, created) = client.ingest(event).await.unwrap();
        assert!(created);
    }

    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 8);
}
