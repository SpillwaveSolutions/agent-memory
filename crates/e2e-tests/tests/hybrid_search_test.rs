//! E2E hybrid search tests for agent-memory.
//!
//! Verifies that HybridSearchHandler returns combined BM25 + vector results
//! via RRF fusion, and gracefully degrades to BM25-only when vector is unavailable.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::{build_toc_segment, create_test_events, ingest_events, TestHarness};
use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer, TeleportSearcher};
use memory_service::hybrid::HybridSearchHandler;
use memory_service::pb::{HybridMode, HybridSearchRequest};
use memory_service::VectorTeleportHandler;
use memory_vector::{HnswConfig, HnswIndex, VectorMetadata};

/// Minimal VectorTeleportHandler whose index is empty so `is_available()` returns false.
fn empty_vector_handler(harness: &TestHarness) -> Arc<VectorTeleportHandler> {
    let embedder =
        memory_embeddings::CandleEmbedder::load_default().expect("Failed to load embedding model");
    let hnsw_config = HnswConfig::new(384, &harness.vector_index_path).with_capacity(10);
    let hnsw = HnswIndex::open_or_create(hnsw_config).expect("HNSW create");
    let meta_path = harness.vector_index_path.join("metadata");
    let metadata = VectorMetadata::open(&meta_path).expect("metadata");
    Arc::new(VectorTeleportHandler::new(
        Arc::new(embedder),
        Arc::new(std::sync::RwLock::new(hnsw)),
        Arc::new(metadata),
    ))
}

/// Build a BM25 searcher from indexed TOC nodes.
fn build_bm25_searcher(
    harness: &TestHarness,
    nodes: &[&memory_types::TocNode],
) -> Arc<TeleportSearcher> {
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    for node in nodes {
        indexer.index_toc_node(node).unwrap();
        for bullet in &node.bullets {
            for grip_id in &bullet.grip_ids {
                if let Some(grip) = harness.storage.get_grip(grip_id).unwrap() {
                    indexer.index_grip(&grip).unwrap();
                }
            }
        }
    }
    indexer.commit().unwrap();

    Arc::new(TeleportSearcher::new(&bm25_index).unwrap())
}

/// E2E: BM25-only fallback when vector index is empty/unavailable.
#[tokio::test]
#[ignore = "requires model download (~80MB on first run)"]
async fn test_hybrid_bm25_fallback_when_vector_unavailable() {
    let harness = TestHarness::new();

    let events_rust = create_test_events(
        "session-rust",
        6,
        "Rust ownership and borrow checker ensures memory safety without garbage collection",
    );
    let events_python = create_test_events(
        "session-python",
        6,
        "Python web frameworks like Django and Flask provide rapid development for web apps",
    );

    ingest_events(&harness.storage, &events_rust);
    ingest_events(&harness.storage, &events_python);

    let node_rust = build_toc_segment(harness.storage.clone(), events_rust).await;
    let node_python = build_toc_segment(harness.storage.clone(), events_python).await;

    let searcher = build_bm25_searcher(&harness, &[&node_rust, &node_python]);
    let vector_handler = empty_vector_handler(&harness);

    let handler = HybridSearchHandler::new(vector_handler, Some(searcher));

    assert!(handler.bm25_available(), "BM25 should be available");

    let request = Request::new(HybridSearchRequest {
        query: "rust ownership borrow".to_string(),
        top_k: 10,
        mode: HybridMode::Hybrid as i32,
        bm25_weight: 0.5,
        vector_weight: 0.5,
        time_filter: None,
        target: 0,
        agent_filter: None,
    });

    let response = handler.hybrid_search(request).await.unwrap();
    let inner = response.into_inner();

    assert_eq!(
        inner.mode_used,
        HybridMode::Bm25Only as i32,
        "Should fall back to BM25-only mode"
    );
    assert!(inner.bm25_available, "bm25_available should be true");
    assert!(
        !inner.matches.is_empty(),
        "BM25 fallback should return results"
    );

    for i in 1..inner.matches.len() {
        assert!(
            inner.matches[i - 1].score >= inner.matches[i].score,
            "Results should be in descending score order"
        );
    }
}

/// E2E: bm25_available reports correctly based on searcher presence.
#[tokio::test]
#[ignore = "requires model download (~80MB on first run)"]
async fn test_hybrid_bm25_available_reports_true() {
    let harness = TestHarness::new();

    let events = create_test_events(
        "session-test",
        4,
        "Test content for BM25 availability check",
    );
    ingest_events(&harness.storage, &events);
    let node = build_toc_segment(harness.storage.clone(), events).await;

    let searcher = build_bm25_searcher(&harness, &[&node]);
    let vector_handler = empty_vector_handler(&harness);

    let handler_with = HybridSearchHandler::new(vector_handler.clone(), Some(searcher));
    assert!(
        handler_with.bm25_available(),
        "bm25_available should be true when searcher is present"
    );

    let handler_without = HybridSearchHandler::new(vector_handler, None);
    assert!(
        !handler_without.bm25_available(),
        "bm25_available should be false when searcher is absent"
    );
}

/// E2E: BM25-only mode returns real BM25 results.
#[tokio::test]
#[ignore = "requires model download (~80MB on first run)"]
async fn test_hybrid_bm25_only_mode() {
    let harness = TestHarness::new();

    let events_rust = create_test_events(
        "session-rust",
        6,
        "Rust ownership and borrow checker ensures memory safety without garbage collection",
    );
    ingest_events(&harness.storage, &events_rust);
    let node_rust = build_toc_segment(harness.storage.clone(), events_rust).await;

    let searcher = build_bm25_searcher(&harness, &[&node_rust]);
    let vector_handler = empty_vector_handler(&harness);

    let handler = HybridSearchHandler::new(vector_handler, Some(searcher));

    let request = Request::new(HybridSearchRequest {
        query: "rust ownership borrow".to_string(),
        top_k: 10,
        mode: HybridMode::Bm25Only as i32,
        bm25_weight: 0.5,
        vector_weight: 0.5,
        time_filter: None,
        target: 0,
        agent_filter: None,
    });

    let response = handler.hybrid_search(request).await.unwrap();
    let inner = response.into_inner();

    assert!(
        !inner.matches.is_empty(),
        "BM25-only mode should return results"
    );
    assert!(inner.bm25_available, "bm25_available should be true");
}
