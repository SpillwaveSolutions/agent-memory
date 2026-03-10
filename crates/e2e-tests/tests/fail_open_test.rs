//! End-to-end fail-open tests for agent-memory (TEST-03).
//!
//! Validates graceful degradation when dedup gate or stale filter
//! encounters errors:
//! - Embedder disabled (None) -> all events ingest normally
//! - Embedder errors -> events pass through unchanged
//! - StaleFilter without timestamps -> results returned unmodified

use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::{build_toc_segment, create_test_events, ingest_events, TestHarness};
use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer, TeleportSearcher};
use memory_service::novelty::EmbedderTrait;
use memory_service::pb::memory_service_server::MemoryService;
use memory_service::pb::{
    Event as ProtoEvent, EventRole as ProtoEventRole, EventType as ProtoEventType,
    IngestEventRequest, RouteQueryRequest,
};
use memory_service::{MemoryServiceImpl, NoveltyChecker, RetrievalHandler};
use memory_types::config::{DedupConfig, StalenessConfig};

/// A failing embedder that always returns an error.
struct FailingEmbedder;

#[async_trait::async_trait]
impl EmbedderTrait for FailingEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
        Err("model load failed".to_string())
    }
}

/// Create a proto Event suitable for IngestEventRequest with a valid ULID event_id.
fn make_proto_event(index: usize, session_id: &str, text: &str) -> ProtoEvent {
    let ts_ms: u64 = 1_706_540_400_000 + (index as u64 * 100);
    let ulid = ulid::Ulid::from_parts(ts_ms, rand::random());
    ProtoEvent {
        event_id: ulid.to_string(),
        session_id: session_id.to_string(),
        timestamp_ms: ts_ms as i64,
        event_type: ProtoEventType::UserMessage as i32,
        role: ProtoEventRole::User as i32,
        text: text.to_string(),
        metadata: Default::default(),
        agent: Some("test-agent".to_string()),
    }
}

/// TEST-03 (1/3): Embedder disabled (None) -- all events ingest normally with outbox entries.
///
/// When NoveltyChecker has no embedder (natural fail path when CandleEmbedder
/// fails to load), the dedup gate fails open: all events pass through, are
/// stored in RocksDB, and have outbox entries.
#[tokio::test]
async fn test_fail_open_embedder_disabled_events_still_stored() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create NoveltyChecker with embedder=None (fail-open path)
    let checker = NoveltyChecker::new(
        None,
        None,
        DedupConfig {
            enabled: true,
            ..Default::default()
        },
    );

    // 3. Create MemoryServiceImpl and set the novelty checker
    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(Arc::new(checker));

    // 4. Ingest 5 events with distinct event_ids and texts
    let mut responses = Vec::new();
    for i in 0..5 {
        let event = make_proto_event(
            i,
            "fail-open-session",
            &format!(
                "This is a sufficiently long test message about fail-open behavior number {i} for dedup gate testing"
            ),
        );
        let resp = service
            .ingest_event(Request::new(IngestEventRequest {
                event: Some(event),
            }))
            .await
            .unwrap();
        responses.push(resp.into_inner());
    }

    // 5. Assert ALL ingest_event calls returned Ok (verified by unwrap above)
    // Assert deduplicated=false for all (fail-open means events pass through)
    for (i, resp) in responses.iter().enumerate() {
        assert!(
            !resp.deduplicated,
            "Event {i} should NOT be marked deduplicated when embedder is None"
        );
        assert!(
            resp.created,
            "Event {i} should be created successfully"
        );
    }

    // 6. Assert all 5 events stored in RocksDB
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(
        stats.event_count, 5,
        "All 5 events should be stored in RocksDB"
    );

    // 7. Assert all 5 have outbox entries (proving normal ingest path)
    let outbox = harness.storage.get_outbox_entries(0, 100).unwrap();
    assert_eq!(
        outbox.len(),
        5,
        "All 5 events should have outbox entries"
    );
}

/// TEST-03 (2/3): Embedder errors -- events pass through unchanged.
///
/// When the embedder returns errors (e.g., model failed to load), the dedup
/// gate fails open: events are not marked as duplicates and are stored normally.
#[tokio::test]
async fn test_fail_open_embedder_error_events_pass_through() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create NoveltyChecker with a FailingEmbedder
    let failing_embedder: Arc<dyn EmbedderTrait> = Arc::new(FailingEmbedder);
    let checker = NoveltyChecker::new(
        Some(failing_embedder),
        None,
        DedupConfig {
            enabled: true,
            ..Default::default()
        },
    );

    // 3. Create MemoryServiceImpl with this checker
    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(Arc::new(checker));

    // 4. Ingest 3 events
    let mut responses = Vec::new();
    for i in 0..3 {
        let event = make_proto_event(
            i,
            "fail-embed-session",
            &format!(
                "This is a sufficiently long test message about embedding failures number {i} for fail-open testing"
            ),
        );
        let resp = service
            .ingest_event(Request::new(IngestEventRequest {
                event: Some(event),
            }))
            .await
            .unwrap();
        responses.push(resp.into_inner());
    }

    // 5. Assert all return Ok with deduplicated=false (fail-open)
    for (i, resp) in responses.iter().enumerate() {
        assert!(
            !resp.deduplicated,
            "Event {i} should NOT be deduplicated when embedder errors"
        );
        assert!(
            resp.created,
            "Event {i} should be created despite embedder error"
        );
    }

    // 6. Assert all 3 events in storage with outbox entries
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(
        stats.event_count, 3,
        "All 3 events should be stored in RocksDB"
    );

    let outbox = harness.storage.get_outbox_entries(0, 100).unwrap();
    assert_eq!(
        outbox.len(),
        3,
        "All 3 events should have outbox entries"
    );
}

/// TEST-03 (3/3): StaleFilter fail-open -- results returned even without timestamp metadata.
///
/// When StaleFilter receives results that lack timestamp_ms in their metadata,
/// it returns them unmodified rather than erroring out.
#[tokio::test]
async fn test_fail_open_staleness_no_timestamp_returns_results() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Ingest events via the helper that bypasses dedup
    let events = create_test_events(
        "e2e-staleness-failopen-session",
        12,
        "Rust memory safety and borrow checker ensures safe concurrency",
    );
    ingest_events(&harness.storage, &events);

    // 3. Build TOC and index into BM25 (same pattern as pipeline_test.rs)
    let toc_node = build_toc_segment(harness.storage.clone(), events).await;

    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    indexer.index_toc_node(&toc_node).unwrap();

    // Index grips too
    let grip_ids: Vec<String> = toc_node
        .bullets
        .iter()
        .flat_map(|b| b.grip_ids.iter().cloned())
        .collect();
    for grip_id in &grip_ids {
        if let Some(grip) = harness.storage.get_grip(grip_id).unwrap() {
            indexer.index_grip(&grip).unwrap();
        }
    }
    indexer.commit().unwrap();

    let bm25_searcher = Arc::new(TeleportSearcher::new(&bm25_index).unwrap());

    // 4. Create RetrievalHandler with StalenessConfig enabled
    let staleness_config = StalenessConfig {
        enabled: true,
        half_life_days: 14.0,
        max_penalty: 0.30,
        ..Default::default()
    };
    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher),
        None,
        None,
        staleness_config,
    );

    // 5. Call route_query with a matching query
    let response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "memory safety borrow checker".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: None,
        }))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 6. Assert has_results=true -- StaleFilter gracefully handles results
    // that may lack timestamp_ms metadata rather than erroring out
    assert!(
        resp.has_results,
        "RouteQuery should have results even when StaleFilter is enabled"
    );
    assert!(
        !resp.results.is_empty(),
        "Results should be non-empty (filter passed them through)"
    );
}
