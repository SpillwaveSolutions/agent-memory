//! End-to-end test infrastructure for agent-memory.
//!
//! Provides a shared TestHarness and helper functions for E2E tests
//! covering the full ingest-to-query pipeline.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};

use memory_storage::Storage;
use memory_toc::builder::TocBuilder;
use memory_toc::segmenter::segment_events;
use memory_toc::summarizer::MockSummarizer;
use memory_toc::SegmentationConfig;
use memory_types::{Event, EventRole, EventType, TocNode};

/// Shared test harness for E2E tests.
///
/// Provides storage, index paths, and helper methods for setting up
/// end-to-end test scenarios.
pub struct TestHarness {
    /// Keeps temp dir alive for the lifetime of the harness
    pub _temp_dir: tempfile::TempDir,
    /// Shared storage instance
    pub storage: Arc<Storage>,
    /// Path for BM25 index files
    pub bm25_index_path: PathBuf,
    /// Path for vector index files
    pub vector_index_path: PathBuf,
}

impl TestHarness {
    /// Create a new test harness with temp directory and storage.
    pub fn new() -> Self {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let storage =
            Arc::new(Storage::open(temp_dir.path()).expect("Failed to open test storage"));

        let bm25_index_path = temp_dir.path().join("bm25-index");
        let vector_index_path = temp_dir.path().join("vector-index");

        std::fs::create_dir_all(&bm25_index_path).expect("Failed to create bm25 index dir");
        std::fs::create_dir_all(&vector_index_path).expect("Failed to create vector index dir");

        Self {
            _temp_dir: temp_dir,
            storage,
            bm25_index_path,
            vector_index_path,
        }
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Ingest events into storage with outbox entries.
///
/// Serializes each event to JSON and stores via `put_event`.
pub fn ingest_events(storage: &Storage, events: &[Event]) {
    for event in events {
        let event_bytes = serde_json::to_vec(event).expect("Failed to serialize event");
        let outbox_bytes = b"pending";
        storage
            .put_event(&event.event_id, &event_bytes, outbox_bytes)
            .expect("Failed to put event");
    }
}

/// Create N test events with sequential timestamps.
///
/// Events are created with ULID-based IDs, 100ms apart, using the
/// given base text as a template (appending index).
pub fn create_test_events(session_id: &str, count: usize, base_text: &str) -> Vec<Event> {
    let base_ts: i64 = 1_706_540_400_000; // 2024-01-29 approx
    let mut events = Vec::with_capacity(count);

    for i in 0..count {
        let ts_ms = base_ts + (i as i64 * 100);
        let ulid = ulid::Ulid::from_parts(ts_ms as u64, rand::random());
        let timestamp: DateTime<Utc> = Utc.timestamp_millis_opt(ts_ms).unwrap();

        let (event_type, role) = if i % 2 == 0 {
            (EventType::UserMessage, EventRole::User)
        } else {
            (EventType::AssistantMessage, EventRole::Assistant)
        };

        let text = format!("{} (message {})", base_text, i);

        let event = Event::new(
            ulid.to_string(),
            session_id.to_string(),
            timestamp,
            event_type,
            role,
            text,
        )
        .with_agent("claude");

        events.push(event);
    }

    events
}

/// Create N test events for a specific agent with sequential timestamps.
///
/// Like `create_test_events` but allows specifying the agent name.
/// Uses realistic agent names (e.g., "claude", "copilot", "gemini").
pub fn create_test_events_for_agent(
    session_id: &str,
    count: usize,
    base_text: &str,
    agent: &str,
) -> Vec<Event> {
    let base_ts: i64 = 1_706_540_400_000; // 2024-01-29 approx
    let mut events = Vec::with_capacity(count);

    for i in 0..count {
        let ts_ms = base_ts + (i as i64 * 100);
        let ulid = ulid::Ulid::from_parts(ts_ms as u64, rand::random());
        let timestamp: DateTime<Utc> = Utc.timestamp_millis_opt(ts_ms).unwrap();

        let (event_type, role) = if i % 2 == 0 {
            (EventType::UserMessage, EventRole::User)
        } else {
            (EventType::AssistantMessage, EventRole::Assistant)
        };

        let text = format!("{} (message {})", base_text, i);

        let event = Event::new(
            ulid.to_string(),
            session_id.to_string(),
            timestamp,
            event_type,
            role,
            text,
        )
        .with_agent(agent);

        events.push(event);
    }

    events
}

/// Build a TOC segment from events using MockSummarizer.
///
/// Segments the events, then processes the first segment through the
/// TocBuilder to create a TocNode with grips.
pub async fn build_toc_segment(storage: Arc<Storage>, events: Vec<Event>) -> TocNode {
    let config = SegmentationConfig {
        // Use high thresholds so all events go into one segment
        time_threshold_ms: 999_999_999,
        token_threshold: 999_999,
        overlap_time_ms: 0,
        overlap_tokens: 0,
        max_tool_result_chars: 1000,
    };

    let segments = segment_events(events, config);
    assert!(
        !segments.is_empty(),
        "Expected at least one segment from events"
    );

    let summarizer = Arc::new(MockSummarizer::new());
    let builder = TocBuilder::new(storage, summarizer);

    builder
        .process_segment(&segments[0])
        .await
        .expect("Failed to process segment into TocNode")
}
