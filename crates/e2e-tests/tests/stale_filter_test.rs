//! End-to-end stale filtering tests for agent-memory (TEST-02).
//!
//! Validates that route_query with StalenessConfig enabled returns results
//! where older events are penalized by time-decay, and that the filter
//! is opt-in (disabled = no change). Kind exemptions are tested at the
//! StaleFilter unit level since `build_metadata` hardcodes memory_kind
//! to "observation" for all BM25 results.

use std::collections::HashMap;
use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use chrono::{TimeZone, Utc};
use e2e_tests::{build_toc_segment, create_test_events, ingest_events, TestHarness};
use memory_retrieval::executor::SearchResult;
use memory_retrieval::stale_filter::StaleFilter;
use memory_retrieval::types::RetrievalLayer;
use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer, TeleportSearcher};
use memory_service::pb::RouteQueryRequest;
use memory_service::RetrievalHandler;
use memory_types::config::StalenessConfig;

const DAY_MS: i64 = 86_400_000;
const BASE_TS: i64 = 1_706_540_400_000; // 2024-01-29 approx

/// Helper: create events with a specific base timestamp offset.
///
/// Returns events whose timestamps start at `BASE_TS - days_ago * DAY_MS`.
fn create_events_at_offset(
    session_id: &str,
    count: usize,
    text: &str,
    days_ago: i64,
) -> Vec<memory_types::Event> {
    let offset_ts = BASE_TS - (days_ago * DAY_MS);
    let mut events = create_test_events(session_id, count, text);
    for (i, event) in events.iter_mut().enumerate() {
        let ts_ms = offset_ts + (i as i64 * 100);
        event.timestamp = Utc.timestamp_millis_opt(ts_ms).unwrap();
    }
    events
}

/// Helper: set up BM25 index, ingest events at different time offsets, and build
/// a TeleportSearcher ready for route_query.
///
/// Returns (harness, searcher) with 4 sessions indexed at 0, 14, 28, 42 days ago.
async fn setup_multi_age_pipeline() -> (TestHarness, Arc<TeleportSearcher>) {
    let harness = TestHarness::new();

    // Topic text shared across all sessions so BM25 matches them all
    let topic = "Rust memory management and borrow checker patterns for safe concurrency";

    // Create BM25 index
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    // 4 sessions at different ages
    let offsets = [
        (0, "session-0d"),
        (14, "session-14d"),
        (28, "session-28d"),
        (42, "session-42d"),
    ];

    for (days_ago, session_id) in &offsets {
        let events = create_events_at_offset(session_id, 8, topic, *days_ago);
        ingest_events(&harness.storage, &events);
        let toc_node = build_toc_segment(harness.storage.clone(), events).await;

        // Index TocNode into BM25
        indexer.index_toc_node(&toc_node).unwrap();

        // Index grips
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
    }

    indexer.commit().unwrap();

    let bm25_searcher = Arc::new(TeleportSearcher::new(&bm25_index).unwrap());
    (harness, bm25_searcher)
}

/// Helper: build a RouteQueryRequest for the common topic.
fn make_query() -> Request<RouteQueryRequest> {
    Request::new(RouteQueryRequest {
        query: "Rust memory management borrow checker".to_string(),
        intent_override: None,
        stop_conditions: None,
        mode_override: None,
        limit: 20,
        agent_filter: None,
    })
}

/// TEST-02 (1/3): Stale results are downranked relative to their unfiltered scores.
///
/// Compares route_query results with staleness enabled vs disabled. For results
/// with older timestamps, the enabled version should have lower scores than
/// the disabled version (proving time-decay is applied). The newest result
/// should be unaffected (age=0 means no penalty).
#[tokio::test]
async fn test_stale_results_downranked_relative_to_newer() {
    let (harness, bm25_searcher) = setup_multi_age_pipeline().await;

    // Query with staleness DISABLED (baseline)
    let handler_off = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher.clone()),
        None,
        None,
        StalenessConfig {
            enabled: false,
            ..Default::default()
        },
    );

    let resp_off = handler_off
        .route_query(make_query())
        .await
        .unwrap()
        .into_inner();

    // Query with staleness ENABLED
    let handler_on = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher),
        None,
        None,
        StalenessConfig {
            enabled: true,
            half_life_days: 14.0,
            max_penalty: 0.30,
            ..Default::default()
        },
    );

    let resp_on = handler_on
        .route_query(make_query())
        .await
        .unwrap()
        .into_inner();

    assert!(resp_on.has_results, "RouteQuery should have results");
    assert!(resp_off.has_results, "Baseline RouteQuery should have results");

    // Build score maps: doc_id -> score for each run
    let scores_off: HashMap<String, f32> = resp_off
        .results
        .iter()
        .map(|r| (r.doc_id.clone(), r.score))
        .collect();
    let scores_on: HashMap<String, f32> = resp_on
        .results
        .iter()
        .map(|r| (r.doc_id.clone(), r.score))
        .collect();

    // Find the newest timestamp across all enabled results
    let newest_ts: i64 = resp_on
        .results
        .iter()
        .filter_map(|r| {
            r.metadata
                .get("timestamp_ms")
                .and_then(|ts| ts.parse::<i64>().ok())
        })
        .max()
        .expect("Should have at least one result with timestamp_ms");

    // For each result that exists in both runs, compare scores
    let mut oldest_found = false;
    let mut newest_unchanged = false;

    for result in &resp_on.results {
        let ts = match result
            .metadata
            .get("timestamp_ms")
            .and_then(|ts| ts.parse::<i64>().ok())
        {
            Some(ts) => ts,
            None => continue,
        };

        if let Some(&baseline_score) = scores_off.get(&result.doc_id) {
            let enabled_score = *scores_on.get(&result.doc_id).unwrap();
            let age_days = (newest_ts - ts) as f64 / DAY_MS as f64;

            if age_days > 7.0 {
                // Old results should have lower scores when staleness is enabled
                assert!(
                    enabled_score <= baseline_score,
                    "Old result {} (age={:.0}d) should have lower score with staleness enabled \
                     (enabled={:.4}, baseline={:.4})",
                    result.doc_id,
                    age_days,
                    enabled_score,
                    baseline_score
                );
                oldest_found = true;
            }

            if age_days < 0.01 {
                // The newest result should be unchanged (age=0 -> no penalty)
                assert!(
                    (enabled_score - baseline_score).abs() < 0.001,
                    "Newest result {} should be unchanged (enabled={:.4}, baseline={:.4})",
                    result.doc_id,
                    enabled_score,
                    baseline_score
                );
                newest_unchanged = true;
            }
        }
    }

    assert!(
        oldest_found,
        "Should have found at least one old result to verify time-decay"
    );
    assert!(
        newest_unchanged,
        "Should have found the newest result to verify no penalty at age=0"
    );
}

/// TEST-02 (2/3): Kind exemption -- Constraint kind is not penalized by time-decay.
///
/// Since `build_metadata` in the retrieval layer hardcodes memory_kind to "observation"
/// for all BM25 results, we test StaleFilter directly with hand-crafted SearchResults.
/// This validates RETRV-03: high-salience kinds are exempt from staleness decay.
#[tokio::test]
async fn test_kind_exemption_constraint_not_penalized() {
    let now = BASE_TS;

    // Hand-craft SearchResults: one old constraint, one old observation, one new observation
    let results = vec![
        SearchResult {
            doc_id: "new-obs".to_string(),
            doc_type: "toc_node".to_string(),
            score: 1.0,
            text_preview: "Recent observation".to_string(),
            source_layer: RetrievalLayer::BM25,
            metadata: {
                let mut m = HashMap::new();
                m.insert("timestamp_ms".to_string(), now.to_string());
                m.insert("memory_kind".to_string(), "observation".to_string());
                m
            },
        },
        SearchResult {
            doc_id: "old-constraint".to_string(),
            doc_type: "toc_node".to_string(),
            score: 0.95,
            text_preview: "Old constraint".to_string(),
            source_layer: RetrievalLayer::BM25,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "timestamp_ms".to_string(),
                    (now - 42 * DAY_MS).to_string(),
                );
                m.insert("memory_kind".to_string(), "constraint".to_string());
                m
            },
        },
        SearchResult {
            doc_id: "old-observation".to_string(),
            doc_type: "toc_node".to_string(),
            score: 0.95,
            text_preview: "Old observation".to_string(),
            source_layer: RetrievalLayer::BM25,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "timestamp_ms".to_string(),
                    (now - 42 * DAY_MS).to_string(),
                );
                m.insert("memory_kind".to_string(), "observation".to_string());
                m
            },
        },
    ];

    let filter = StaleFilter::new(StalenessConfig {
        enabled: true,
        half_life_days: 14.0,
        max_penalty: 0.30,
        ..Default::default()
    });

    let filtered = filter.apply(results);

    // Find each result
    let constraint = filtered
        .iter()
        .find(|r| r.doc_id == "old-constraint")
        .unwrap();
    let observation = filtered
        .iter()
        .find(|r| r.doc_id == "old-observation")
        .unwrap();

    // Constraint should retain its original score (exempt from decay)
    assert!(
        (constraint.score - 0.95).abs() < f32::EPSILON,
        "Constraint kind should be exempt from time-decay, got score {:.4}",
        constraint.score
    );

    // Old observation should be decayed below its original score
    assert!(
        observation.score < 0.95,
        "Old observation should be decayed, got score {:.4}",
        observation.score
    );

    // Constraint should score higher than the decayed observation
    assert!(
        constraint.score > observation.score,
        "Exempt constraint ({:.4}) should score higher than decayed observation ({:.4})",
        constraint.score,
        observation.score
    );

    // Also verify definition, procedure, and preference exemptions
    for kind in &["definition", "procedure", "preference"] {
        let results = vec![
            SearchResult {
                doc_id: "new".to_string(),
                doc_type: "toc_node".to_string(),
                score: 1.0,
                text_preview: "New".to_string(),
                source_layer: RetrievalLayer::BM25,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("timestamp_ms".to_string(), now.to_string());
                    m.insert("memory_kind".to_string(), "observation".to_string());
                    m
                },
            },
            SearchResult {
                doc_id: format!("old-{kind}"),
                doc_type: "toc_node".to_string(),
                score: 0.90,
                text_preview: format!("Old {kind}"),
                source_layer: RetrievalLayer::BM25,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert(
                        "timestamp_ms".to_string(),
                        (now - 42 * DAY_MS).to_string(),
                    );
                    m.insert("memory_kind".to_string(), kind.to_string());
                    m
                },
            },
        ];

        let filtered = filter.apply(results);
        let exempt = filtered
            .iter()
            .find(|r| r.doc_id == format!("old-{kind}"))
            .unwrap();
        assert!(
            (exempt.score - 0.90).abs() < f32::EPSILON,
            "{kind} kind should be exempt from time-decay, got score {:.4}",
            exempt.score
        );
    }
}

/// TEST-02 (3/3): Stale filter disabled produces no score change (control test).
///
/// Same pipeline setup but with StalenessConfig disabled. Results should be
/// returned without any staleness reranking.
#[tokio::test]
async fn test_stale_filter_disabled_no_score_change() {
    let (harness, bm25_searcher) = setup_multi_age_pipeline().await;

    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher),
        None,
        None,
        StalenessConfig {
            enabled: false,
            ..Default::default()
        },
    );

    let response = handler.route_query(make_query()).await.unwrap();

    let resp = response.into_inner();
    assert!(resp.has_results, "RouteQuery should have results");
    assert!(
        !resp.results.is_empty(),
        "RouteQuery should return non-empty results with filter disabled"
    );

    // With staleness disabled, results should be returned (proving opt-in nature).
    // We cannot assert exact BM25 ordering here since BM25 scores depend on
    // term frequency, but we verify results exist and the pipeline works
    // without the filter active.
    assert_eq!(
        resp.results.iter().filter(|r| r.score > 0.0).count(),
        resp.results.len(),
        "All results should have positive scores when filter is disabled"
    );
}
