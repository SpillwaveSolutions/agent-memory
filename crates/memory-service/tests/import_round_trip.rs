//! Round-trip integration tests for the import handler pipeline.
//!
//! Tests verify that importing JSONL chunks via `import::import_chunks` correctly
//! stores records in RocksDB, handles idempotency, and respects dry_run mode.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use tempfile::TempDir;

use memory_service::import;
use memory_service::pb::{BackupChunkType, ImportChunk};
use memory_storage::Storage;
use memory_types::{Event, EventRole, EventType};

fn make_storage(dir: &TempDir) -> Arc<Storage> {
    Arc::new(Storage::open(dir.path()).expect("open storage"))
}

/// Create a test event with a valid ULID-format event_id.
fn make_test_event(ulid_suffix: u8, text: &str) -> Event {
    // Build a deterministic ULID: 10 bytes timestamp + 16 bytes randomness encoded as Crockford base32.
    // We use a fixed timestamp and vary the last byte for uniqueness.
    let ulid = ulid::Ulid::from_parts(1_700_000_000_000, u128::from(ulid_suffix));
    Event::new(
        ulid.to_string(),
        "test-session".to_string(),
        DateTime::from_timestamp_millis(1_700_000_000_000)
            .unwrap_or_else(Utc::now),
        EventType::UserMessage,
        EventRole::User,
        text.to_string(),
    )
}

#[test]
fn test_import_events_round_trip() {
    let dir = TempDir::new().unwrap();
    let storage = make_storage(&dir);

    let event = make_test_event(1, "hello world");
    let event_id = event.event_id.clone();
    let jsonl = serde_json::to_string(&event).unwrap();

    let chunk = ImportChunk {
        chunk_type: BackupChunkType::Events as i32,
        jsonl_data: jsonl,
        record_count: 1,
        dry_run: false,
        events_only: false,
    };

    let result = import::import_chunks(&storage, &[chunk]);

    assert_eq!(result.events_imported, 1);
    assert_eq!(result.events_skipped, 0);
    assert_eq!(result.errors, 0);

    // Verify event is retrievable from storage
    let retrieved_bytes = storage.get_event(&event_id).expect("get_event failed");
    assert!(
        retrieved_bytes.is_some(),
        "Event should exist in storage after import"
    );

    let retrieved_event = Event::from_bytes(&retrieved_bytes.unwrap()).unwrap();
    assert_eq!(retrieved_event.event_id, event_id);
    assert_eq!(retrieved_event.text, "hello world");
}

#[test]
fn test_import_events_idempotent() {
    let dir = TempDir::new().unwrap();
    let storage = make_storage(&dir);

    let event = make_test_event(2, "deduplicated");
    let jsonl = serde_json::to_string(&event).unwrap();

    // First import
    let chunk1 = ImportChunk {
        chunk_type: BackupChunkType::Events as i32,
        jsonl_data: jsonl.clone(),
        record_count: 1,
        dry_run: false,
        events_only: false,
    };
    let result1 = import::import_chunks(&storage, &[chunk1]);
    assert_eq!(result1.events_imported, 1);
    assert_eq!(result1.events_skipped, 0);

    // Second import -- same event should be skipped
    let chunk2 = ImportChunk {
        chunk_type: BackupChunkType::Events as i32,
        jsonl_data: jsonl,
        record_count: 1,
        dry_run: false,
        events_only: false,
    };
    let result2 = import::import_chunks(&storage, &[chunk2]);
    assert_eq!(
        result2.events_skipped, 1,
        "Duplicate event should be skipped"
    );
    assert_eq!(result2.events_imported, 0);
}

#[test]
fn test_import_dry_run_no_writes() {
    let dir = TempDir::new().unwrap();
    let storage = make_storage(&dir);

    let event = make_test_event(3, "should not be stored");
    let event_id = event.event_id.clone();
    let jsonl = serde_json::to_string(&event).unwrap();

    let chunk = ImportChunk {
        chunk_type: BackupChunkType::Events as i32,
        jsonl_data: jsonl,
        record_count: 1,
        dry_run: true,
        events_only: false,
    };
    let result = import::import_chunks(&storage, &[chunk]);

    assert!(result.dry_run, "Result should report dry_run=true");
    assert_eq!(
        result.events_imported, 1,
        "dry_run still counts the record"
    );

    // Event must NOT be in storage
    let retrieved = storage
        .get_event(&event_id)
        .expect("get_event failed");
    assert!(
        retrieved.is_none(),
        "dry_run must not write to storage"
    );
}
