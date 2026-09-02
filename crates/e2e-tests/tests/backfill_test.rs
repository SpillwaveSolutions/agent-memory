//! OPS-01 / 61-01: admin backfill-index e2e.
//!
//! Simulates a pre-v3.1 store: events in RocksDB with real outbox JSON,
//! no indexer ever ran (empty `text_preview` gap). Backfill replays the
//! event log into Tantivy; TeleportSearch then returns non-empty previews.
//!
//! A v3.0 daemon binary fixture at 68ab122 is not practical here; the
//! schema of stored events is unchanged. What was missing is the replay.

use std::sync::Arc;

use e2e_tests::{create_test_events, ingest_events_with_outbox, TestHarness};
use memory_indexing::{
    BackfillConfig, Bm25IndexUpdater, IndexCheckpoint, IndexType, IndexingPipeline, PipelineConfig,
};
use memory_search::{
    DocType, SearchIndex, SearchIndexConfig, SearchIndexer, SearchOptions, TeleportSearcher,
};

fn bm25_pipeline(harness: &TestHarness) -> (IndexingPipeline, SearchIndex) {
    let config = SearchIndexConfig::new(&harness.bm25_index_path);
    let index = SearchIndex::open_or_create(config).expect("open search index");
    let indexer = Arc::new(SearchIndexer::new(&index).expect("indexer"));
    let updater = Bm25IndexUpdater::new(indexer, harness.storage.clone());
    let mut pipeline = IndexingPipeline::new(harness.storage.clone(), PipelineConfig::default());
    pipeline.add_updater(Box::new(updater));
    pipeline.load_checkpoints().expect("load checkpoints");
    (pipeline, index)
}

fn seed_fifty(harness: &TestHarness) {
    let events = create_test_events(
        "backfill-session",
        50,
        "backfillneedle unique token for OPS-01 preview replay zebrafizz",
    );
    ingest_events_with_outbox(&harness.storage, &events);
}

#[tokio::test]
async fn test_backfill_fifty_events_search_returns_previews() {
    let harness = TestHarness::new();
    seed_fifty(&harness);

    // No indexer ran. Outbox is real JSON waiting to be drained.
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 50);
    assert_eq!(stats.outbox_count, 50);

    let (mut pipeline, index) = bm25_pipeline(&harness);
    let report = pipeline
        .backfill(BackfillConfig::default().with_batch_size(10), |_, _| {})
        .expect("backfill");
    assert_eq!(report.documents, 50);
    assert_eq!(report.would_index, 50);
    assert!(!report.dry_run);

    let searcher = TeleportSearcher::new(&index).expect("searcher");
    let results = searcher
        .search(
            "backfillneedle",
            SearchOptions::new()
                .with_limit(50)
                .with_doc_type(DocType::Event),
        )
        .expect("search");
    assert!(
        results.len() >= 10,
        "expected event hits after backfill, got {}",
        results.len()
    );
    for hit in &results {
        assert_eq!(hit.doc_type, DocType::Event);
        assert!(
            !hit.text.is_empty(),
            "text_preview empty for {}",
            hit.doc_id
        );
        assert!(
            hit.text.contains("backfillneedle") || hit.text.contains("zebrafizz"),
            "preview missing seeded tokens: {}",
            hit.text
        );
    }

    let second = pipeline
        .backfill(BackfillConfig::default(), |_, _| {})
        .expect("second backfill");
    assert_eq!(second.documents, 0, "second run must report 0 new");
    assert_eq!(second.would_index, 0);
}

#[tokio::test]
async fn test_backfill_dry_run_writes_nothing() {
    let harness = TestHarness::new();
    seed_fifty(&harness);
    let (mut pipeline, index) = bm25_pipeline(&harness);

    let before = harness.storage.get_checkpoint("index_bm25").unwrap();
    let report = pipeline
        .backfill(BackfillConfig::default().with_dry_run(true), |_, _| {})
        .expect("dry-run");
    assert!(report.dry_run);
    assert_eq!(report.documents, 0);
    assert_eq!(report.would_index, 50);
    assert_eq!(
        harness.storage.get_checkpoint("index_bm25").unwrap(),
        before,
        "dry-run must not persist a checkpoint"
    );

    let searcher = TeleportSearcher::new(&index).expect("searcher");
    let results = searcher
        .search(
            "backfillneedle",
            SearchOptions::new()
                .with_limit(10)
                .with_doc_type(DocType::Event),
        )
        .expect("search");
    assert!(results.is_empty(), "dry-run must not write BM25 documents");
}

#[tokio::test]
async fn test_backfill_resumes_after_first_batch() {
    let harness = TestHarness::new();
    seed_fifty(&harness);

    let (mut pipeline, _index) = bm25_pipeline(&harness);
    let first = pipeline
        .backfill(
            BackfillConfig::default()
                .with_batch_size(10)
                .with_max_batches(Some(1)),
            |_, _| {},
        )
        .expect("batch 1");
    assert_eq!(first.documents, 10);
    let bytes = harness
        .storage
        .get_checkpoint("index_bm25")
        .unwrap()
        .expect("checkpoint after batch 1");
    let cp = IndexCheckpoint::from_bytes(&bytes).unwrap();
    assert_eq!(cp.index_type, IndexType::Bm25);
    assert!(cp.processed_count > 0);

    // SIGINT equivalent: drop the writer (IndexWriter lock) then resume.
    drop(pipeline);
    drop(_index);

    let (mut pipeline2, index) = bm25_pipeline(&harness);
    let rest = pipeline2
        .backfill(BackfillConfig::default().with_batch_size(10), |_, _| {})
        .expect("resume");
    assert_eq!(rest.documents, 40);

    let searcher = TeleportSearcher::new(&index).expect("searcher");
    let results = searcher
        .search(
            "zebrafizz",
            SearchOptions::new()
                .with_limit(50)
                .with_doc_type(DocType::Event),
        )
        .expect("search");
    assert!(!results.is_empty());
    assert!(results.iter().all(|h| !h.text.is_empty()));
}

#[tokio::test]
async fn test_backfill_from_sequence_zero_after_outbox_cleaned() {
    let harness = TestHarness::new();
    seed_fifty(&harness);
    // Pre-v3.1 store: outbox already drained, checkpoint caught up, events
    // still in the log with no text in the index.
    harness
        .storage
        .delete_outbox_entries(u64::MAX)
        .expect("clean outbox");
    assert_eq!(harness.storage.get_stats().unwrap().outbox_count, 0);

    let (mut pipeline, index) = bm25_pipeline(&harness);
    let report = pipeline
        .backfill(
            BackfillConfig::default().with_from_sequence(Some(0)),
            |_, _| {},
        )
        .expect("force from 0");
    assert_eq!(report.source, memory_indexing::BackfillSource::EventLog);
    assert_eq!(report.documents, 50);

    let searcher = TeleportSearcher::new(&index).expect("searcher");
    let results = searcher
        .search(
            "backfillneedle",
            SearchOptions::new()
                .with_limit(50)
                .with_doc_type(DocType::Event),
        )
        .expect("search");
    assert!(!results.is_empty());
    assert!(results.iter().all(|h| !h.text.is_empty()));
}
