//! E2E dedup tests proving duplicate events are stored but not indexed (TEST-01).
//!
//! Validates the full dedup pipeline end-to-end:
//! - Duplicate events stored in RocksDB but absent from outbox
//! - Structural events (SessionStart) bypass dedup gate entirely
//! - IngestEventResponse.deduplicated field is correct

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::{
    create_proto_event, create_proto_event_structural, uniform_normalized, MockEmbedder,
    TestHarness,
};
use memory_service::novelty::EmbedderTrait;
use memory_service::pb::memory_service_server::MemoryService;
use memory_service::pb::IngestEventRequest;
use memory_service::{MemoryServiceImpl, NoveltyChecker};
use memory_types::config::DedupConfig;
use memory_types::dedup::InFlightBuffer;

/// Generate a ULID string from a timestamp and a seed for the random part.
fn make_ulid(ts_ms: u64, seed: u128) -> String {
    ulid::Ulid::from_parts(ts_ms, seed).to_string()
}

/// Sequential embedder that returns a different embedding per call.
///
/// Pops from a VecDeque on each `embed()` call, allowing tests to control
/// the exact similarity between consecutive events.
struct SequentialEmbedder {
    embeddings: Mutex<VecDeque<Vec<f32>>>,
}

impl SequentialEmbedder {
    fn new(embeddings: Vec<Vec<f32>>) -> Self {
        Self {
            embeddings: Mutex::new(VecDeque::from(embeddings)),
        }
    }
}

#[async_trait::async_trait]
impl EmbedderTrait for SequentialEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
        let mut queue = self.embeddings.lock().map_err(|e| e.to_string())?;
        queue
            .pop_front()
            .ok_or_else(|| "SequentialEmbedder: no more embeddings".to_string())
    }
}

/// TEST-01: Duplicate events are stored in RocksDB but absent from outbox.
///
/// Proves the store-and-skip-outbox pattern: both events exist in storage,
/// but only the novel event has an outbox entry.
#[tokio::test]
async fn test_dedup_duplicate_stored_but_not_indexed() {
    let harness = TestHarness::new();
    let dim = 384;
    let embedding = uniform_normalized(dim);

    let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
    let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
        embedding: embedding.clone(),
    });
    let checker = Arc::new(NoveltyChecker::with_in_flight_buffer(
        Some(embedder),
        buffer.clone(),
        DedupConfig {
            enabled: true,
            threshold: 0.85,
            min_text_length: 10,
            ..Default::default()
        },
    ));

    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(checker);

    let ts1 = 1_706_540_400_000u64;
    let ts2 = ts1 + 100;
    let event1_id = make_ulid(ts1, 1001);
    let event2_id = make_ulid(ts2, 1002);

    // Ingest first event (novel -- buffer is empty)
    let resp1 = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &event1_id,
                "session-dedup-1",
                ts1 as i64,
                2, // UserMessage
                "Rust memory safety and borrow checker ensures safe concurrency patterns",
            )),
        }))
        .await
        .expect("First ingest should succeed");
    let resp1 = resp1.into_inner();
    assert_eq!(resp1.created, true, "First event should be created");
    assert_eq!(
        resp1.deduplicated, false,
        "First event should NOT be deduplicated"
    );

    // Ingest second event (duplicate -- same mock embedding -> cosine ~1.0 > 0.85)
    let resp2 = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &event2_id,
                "session-dedup-1",
                ts2 as i64,
                2, // UserMessage
                "Different text but identical embedding means this is a semantic duplicate",
            )),
        }))
        .await
        .expect("Second ingest should succeed");
    let resp2 = resp2.into_inner();
    assert_eq!(
        resp2.created, true,
        "Second event should be created (stored)"
    );
    assert_eq!(
        resp2.deduplicated, true,
        "Second event should be deduplicated"
    );

    // Both events exist in storage
    let e1 = harness.storage.get_event(&event1_id).unwrap();
    assert!(e1.is_some(), "First event should exist in RocksDB");
    let e2 = harness.storage.get_event(&event2_id).unwrap();
    assert!(e2.is_some(), "Second event should exist in RocksDB");

    // Storage stats show event_count = 2
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 2, "Both events should be in storage");

    // Only 1 outbox entry (for the novel event, not the duplicate)
    let outbox_entries = harness.storage.get_outbox_entries(0, 100).unwrap();
    assert_eq!(
        outbox_entries.len(),
        1,
        "Only the novel event should have an outbox entry"
    );
}

/// TEST-01 variant: Two truly novel events should both get outbox entries.
///
/// Uses SequentialEmbedder to return orthogonal vectors (cosine ~0),
/// proving both events are detected as novel and indexed.
#[tokio::test]
async fn test_dedup_novel_events_all_indexed() {
    let harness = TestHarness::new();
    let dim = 384;

    // Create orthogonal vectors (cosine similarity ~0)
    let mut vec_a = vec![0.0f32; dim];
    for v in vec_a.iter_mut().take(dim / 2) {
        *v = 1.0 / ((dim / 2) as f32).sqrt();
    }
    let mut vec_b = vec![0.0f32; dim];
    for v in vec_b.iter_mut().skip(dim / 2) {
        *v = 1.0 / ((dim / 2) as f32).sqrt();
    }

    let embedder: Arc<dyn EmbedderTrait> = Arc::new(SequentialEmbedder::new(vec![vec_a, vec_b]));
    let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
    let checker = Arc::new(NoveltyChecker::with_in_flight_buffer(
        Some(embedder),
        buffer.clone(),
        DedupConfig {
            enabled: true,
            threshold: 0.85,
            min_text_length: 10,
            ..Default::default()
        },
    ));

    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(checker);

    let ts1 = 1_706_540_400_000u64;
    let ts2 = ts1 + 100;

    // Ingest first event
    let resp1 = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &make_ulid(ts1, 2001),
                "session-novel-1",
                ts1 as i64,
                2,
                "First novel event about Rust ownership model and move semantics",
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp1.deduplicated, false, "First event should be novel");

    // Ingest second event (orthogonal embedding -> novel)
    let resp2 = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &make_ulid(ts2, 2002),
                "session-novel-1",
                ts2 as i64,
                2,
                "Second novel event about async runtime and tokio task scheduling",
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        resp2.deduplicated, false,
        "Second event should also be novel"
    );

    // Both events should have outbox entries
    let outbox_entries = harness.storage.get_outbox_entries(0, 100).unwrap();
    assert_eq!(
        outbox_entries.len(),
        2,
        "Both novel events should have outbox entries"
    );
}

/// DEDUP-04: Structural events bypass the dedup gate entirely.
///
/// Even with a MockEmbedder that returns identical embeddings (would match
/// as duplicate for normal events), SessionStart events bypass dedup and
/// are indexed normally.
#[tokio::test]
async fn test_dedup_structural_events_bypass_gate() {
    let harness = TestHarness::new();
    let dim = 384;
    let embedding = uniform_normalized(dim);

    let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
    let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
        embedding: embedding.clone(),
    });
    let checker = Arc::new(NoveltyChecker::with_in_flight_buffer(
        Some(embedder),
        buffer.clone(),
        DedupConfig {
            enabled: true,
            threshold: 0.85,
            min_text_length: 10,
            ..Default::default()
        },
    ));

    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(checker);

    let ts1 = 1_706_540_400_000u64;
    let ts2 = ts1 + 100;

    // Ingest a normal UserMessage event (novel -- buffer empty)
    let resp1 = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &make_ulid(ts1, 3001),
                "session-struct-1",
                ts1 as i64,
                2, // UserMessage
                "Normal user message that will populate the dedup buffer",
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp1.deduplicated, false, "Normal event should be novel");

    // Ingest a SessionStart structural event -- should bypass dedup entirely
    let resp2 = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event_structural(
                &make_ulid(ts2, 3002),
                "session-struct-1",
                ts2 as i64,
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        resp2.deduplicated, false,
        "Structural event should bypass dedup (deduplicated=false)"
    );
    assert_eq!(resp2.created, true, "Structural event should be created");

    // Both events should have outbox entries (structural is indexed normally)
    let outbox_entries = harness.storage.get_outbox_entries(0, 100).unwrap();
    assert_eq!(
        outbox_entries.len(),
        2,
        "Both normal and structural events should have outbox entries"
    );
}

/// TEST-01: IngestEventResponse fields are correct for all event types.
///
/// Verifies: novel -> created=true, deduplicated=false
///           duplicate -> created=true, deduplicated=true
///           structural -> created=true, deduplicated=false
#[tokio::test]
async fn test_dedup_response_fields() {
    let harness = TestHarness::new();
    let dim = 384;
    let embedding = uniform_normalized(dim);

    let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
    let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
        embedding: embedding.clone(),
    });
    let checker = Arc::new(NoveltyChecker::with_in_flight_buffer(
        Some(embedder),
        buffer.clone(),
        DedupConfig {
            enabled: true,
            threshold: 0.85,
            min_text_length: 10,
            ..Default::default()
        },
    ));

    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_novelty_checker(checker);

    let ts1 = 1_706_540_400_000u64;
    let ts2 = ts1 + 100;
    let ts3 = ts1 + 200;

    // Novel event
    let resp_novel = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &make_ulid(ts1, 4001),
                "session-resp-1",
                ts1 as i64,
                2,
                "Novel event for response field verification test case",
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp_novel.created, true, "Novel: created should be true");
    assert_eq!(
        resp_novel.deduplicated, false,
        "Novel: deduplicated should be false"
    );

    // Duplicate event (same embedding -> cosine ~1.0)
    let resp_dup = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event(
                &make_ulid(ts2, 4002),
                "session-resp-1",
                ts2 as i64,
                2,
                "Duplicate event with same embedding for response field test",
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp_dup.created, true, "Duplicate: created should be true");
    assert_eq!(
        resp_dup.deduplicated, true,
        "Duplicate: deduplicated should be true"
    );

    // Structural event (bypasses dedup)
    let resp_struct = service
        .ingest_event(Request::new(IngestEventRequest {
            event: Some(create_proto_event_structural(
                &make_ulid(ts3, 4003),
                "session-resp-1",
                ts3 as i64,
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        resp_struct.created, true,
        "Structural: created should be true"
    );
    assert_eq!(
        resp_struct.deduplicated, false,
        "Structural: deduplicated should be false"
    );
}
