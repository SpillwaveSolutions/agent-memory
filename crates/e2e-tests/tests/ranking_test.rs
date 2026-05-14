//! End-to-end ranking tests for agent-memory (RANK-09, RANK-10).
//!
//! Verifies that:
//! - High-salience items rank higher than low-salience items of similar similarity
//! - Usage decay penalizes frequently-accessed results
//! - Score floor prevents total suppression
//! - Ranking composes correctly with StaleFilter through route_query

use std::collections::HashMap;
use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::{build_toc_segment, create_test_events, ingest_events, TestHarness};
use memory_retrieval::{
    executor::SearchResult,
    ranking::{apply_combined_ranking, RankingConfig},
    stale_filter::StaleFilter,
    types::RetrievalLayer,
};
use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer, TeleportSearcher};
use memory_service::pb::RouteQueryRequest;
use memory_service::RetrievalHandler;
use memory_types::config::StalenessConfig;
use memory_types::salience::MemoryKind;

fn make_result(
    doc_id: &str,
    score: f32,
    salience: f32,
    access_count: u32,
    memory_kind: &str,
) -> SearchResult {
    let mut metadata = HashMap::new();
    metadata.insert("salience_score".to_string(), salience.to_string());
    metadata.insert("access_count".to_string(), access_count.to_string());
    metadata.insert("memory_kind".to_string(), memory_kind.to_string());
    SearchResult {
        doc_id: doc_id.to_string(),
        doc_type: "toc_node".to_string(),
        score,
        text_preview: format!("Preview for {doc_id}"),
        source_layer: RetrievalLayer::BM25,
        metadata,
    }
}

/// RANK-09: Pinned/high-salience items rank higher than low-salience items.
#[test]
fn test_salience_ranking_order() {
    let config = RankingConfig {
        salience_enabled: true,
        usage_decay_enabled: false,
        ..Default::default()
    };

    // All items have same base similarity score
    let results = vec![
        // Observation, short text -> low salience (~0.35-0.40)
        make_result("low_obs", 0.85, 0.38, 0, "observation"),
        // Constraint, medium text -> high salience (~0.75+)
        make_result("high_constraint", 0.85, 0.78, 0, "constraint"),
        // Pinned item -> very high salience (~1.0+)
        make_result("pinned_item", 0.85, 1.05, 0, "preference"),
    ];

    let ranked = apply_combined_ranking(results, &config);

    // Pinned item should be first (highest salience factor)
    assert_eq!(
        ranked[0].doc_id, "pinned_item",
        "Pinned item should rank first"
    );
    // Constraint should be second
    assert_eq!(
        ranked[1].doc_id, "high_constraint",
        "High-salience constraint should rank second"
    );
    // Low observation should be last
    assert_eq!(
        ranked[2].doc_id, "low_obs",
        "Low-salience observation should rank last"
    );
}

/// RANK-10: Frequently-accessed items decay in ranking.
#[test]
fn test_usage_decay_ranking_order() {
    let config = RankingConfig {
        salience_enabled: false,
        usage_decay_enabled: true,
        decay_factor: 0.15,
        ..Default::default()
    };

    // All items have same base similarity and salience
    let results = vec![
        make_result("fresh", 0.85, 0.5, 0, "observation"),
        make_result("used_5", 0.85, 0.5, 5, "observation"),
        make_result("used_20", 0.85, 0.5, 20, "observation"),
    ];

    let ranked = apply_combined_ranking(results, &config);

    // Fresh item should rank first (no decay)
    assert_eq!(ranked[0].doc_id, "fresh", "Fresh item should rank first");
    // Moderately used should be second
    assert_eq!(
        ranked[1].doc_id, "used_5",
        "Moderately used should rank second"
    );
    // Heavily used should be last
    assert_eq!(ranked[2].doc_id, "used_20", "Heavily used should rank last");

    // Verify scores are strictly decreasing
    assert!(ranked[0].score > ranked[1].score);
    assert!(ranked[1].score > ranked[2].score);
}

/// Score floor prevents complete suppression.
#[test]
fn test_score_floor_prevents_collapse() {
    let config = RankingConfig {
        salience_enabled: true,
        usage_decay_enabled: true,
        decay_factor: 0.15,
        score_floor: 0.50,
    };

    // Worst case: low salience + extremely high access count
    let results = vec![make_result(
        "heavily_used_low_sal",
        0.9,
        0.1,
        200,
        "observation",
    )];

    let ranked = apply_combined_ranking(results, &config);

    // Floor = 0.9 * 0.50 = 0.45
    let floor = 0.9 * 0.50;
    assert!(
        ranked[0].score >= floor - 0.001,
        "Score {} should be >= floor {:.3}",
        ranked[0].score,
        floor
    );
}

/// Combined formula composes properly: salience + usage + similarity all factor in.
#[test]
fn test_combined_ranking_composition() {
    let config = RankingConfig {
        salience_enabled: true,
        usage_decay_enabled: true,
        decay_factor: 0.15,
        score_floor: 0.50,
    };

    // High-salience but heavily used vs low-salience but fresh
    let results = vec![
        make_result("high_sal_used", 0.85, 1.0, 15, "constraint"),
        make_result("low_sal_fresh", 0.85, 0.3, 0, "observation"),
    ];

    let ranked = apply_combined_ranking(results, &config);

    // Both should have reasonable scores (not collapsed)
    for r in &ranked {
        assert!(
            r.score > 0.3,
            "Score for {} should be > 0.3, got {}",
            r.doc_id,
            r.score
        );
    }
}

// ============================================================================
// E2E tests: Full route_query pipeline with Storage-backed enrichment
// ============================================================================

const TOPIC: &str = "Rust ownership borrow checker lifetime annotation patterns";

fn make_route_query() -> Request<RouteQueryRequest> {
    Request::new(RouteQueryRequest {
        query: "Rust ownership borrow checker lifetime".to_string(),
        intent_override: None,
        stop_conditions: None,
        mode_override: None,
        limit: 20,
        agent_filter: None,
        all_projects: false,
    })
}

/// Set up a pipeline with multiple sessions indexed into BM25.
/// Returns (harness, searcher, toc_node_ids).
async fn setup_salience_pipeline() -> (TestHarness, Arc<TeleportSearcher>, Vec<String>) {
    let harness = TestHarness::new();

    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    let sessions = ["session-high", "session-mid", "session-low"];
    let mut node_ids = Vec::new();

    for session_id in &sessions {
        let events = create_test_events(session_id, 8, TOPIC);
        ingest_events(&harness.storage, &events);
        let toc_node = build_toc_segment(harness.storage.clone(), events).await;

        indexer.index_toc_node(&toc_node).unwrap();

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

        node_ids.push(toc_node.node_id.clone());
    }

    indexer.commit().unwrap();
    let searcher = Arc::new(TeleportSearcher::new(&bm25_index).unwrap());
    (harness, searcher, node_ids)
}

/// RANK-09 E2E: Salience enrichment flows through route_query and affects ranking.
///
/// Mutates TocNode salience scores in Storage, queries via route_query,
/// and verifies that the high-salience node outranks the low-salience one.
#[tokio::test]
async fn test_e2e_salience_enrichment_affects_ranking() {
    let (harness, searcher, node_ids) = setup_salience_pipeline().await;

    // Mutate TocNode salience in storage
    let salience_values: [(f32, MemoryKind, bool); 3] = [
        (1.0, MemoryKind::Constraint, true),
        (0.5, MemoryKind::Observation, false),
        (0.1, MemoryKind::Observation, false),
    ];

    for (i, (score, kind, pinned)) in salience_values.iter().enumerate() {
        if let Ok(Some(mut node)) = harness.storage.get_toc_node(&node_ids[i]) {
            node.salience_score = *score;
            node.memory_kind = *kind;
            node.is_pinned = *pinned;
            harness.storage.put_toc_node(&node).unwrap();
        }
    }

    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(searcher),
        None,
        None,
        StalenessConfig::default(),
    );

    let resp = handler
        .route_query(make_route_query())
        .await
        .unwrap()
        .into_inner();

    assert!(resp.has_results, "Should have search results");

    // Find scores for our mutated nodes
    let score_high = resp
        .results
        .iter()
        .find(|r| r.doc_id == node_ids[0])
        .map(|r| r.score);
    let score_low = resp
        .results
        .iter()
        .find(|r| r.doc_id == node_ids[2])
        .map(|r| r.score);

    if let (Some(high), Some(low)) = (score_high, score_low) {
        assert!(
            high > low,
            "High-salience node ({:.4}) should outrank low-salience node ({:.4})",
            high,
            low
        );
    }
}

/// RANK-10 E2E: Access count enrichment flows through route_query.
///
/// Verifies that access_count metadata is enriched from Storage and
/// all results have valid positive scores through the pipeline.
/// Note: usage_decay is off by default in RankingConfig, so this test
/// validates the enrichment path rather than decay ordering (which is
/// covered by the unit-level test_usage_decay_ranking_order above).
#[tokio::test]
async fn test_e2e_access_count_enrichment() {
    let (harness, searcher, node_ids) = setup_salience_pipeline().await;

    // Set different access counts; keep salience neutral
    let access_counts: [u32; 3] = [0, 10, 50];
    for (i, &count) in access_counts.iter().enumerate() {
        if let Ok(Some(mut node)) = harness.storage.get_toc_node(&node_ids[i]) {
            node.salience_score = 0.5;
            node.access_count = count;
            harness.storage.put_toc_node(&node).unwrap();
        }
    }

    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(searcher),
        None,
        None,
        StalenessConfig::default(),
    );

    let resp = handler
        .route_query(make_route_query())
        .await
        .unwrap()
        .into_inner();

    assert!(resp.has_results, "Should have search results");

    // All returned results should have positive scores
    for result in &resp.results {
        assert!(
            result.score > 0.0,
            "Result {} should have positive score, got {}",
            result.doc_id,
            result.score
        );
    }

    // Verify the pipeline returns results for our nodes (enrichment didn't break anything)
    let found_count = resp
        .results
        .iter()
        .filter(|r| node_ids.contains(&r.doc_id))
        .count();
    assert!(
        found_count > 0,
        "Should find at least one of our TocNodes in results"
    );
}

/// Composition: ranking composes with StaleFilter — old high-salience constraint
/// is exempt from staleness and still ranks well.
#[test]
fn test_ranking_composes_with_stale_filter() {
    let now_ms = 1_706_540_400_000i64;
    let day_ms = 86_400_000i64;

    let mut meta_old = HashMap::new();
    meta_old.insert(
        "timestamp_ms".to_string(),
        (now_ms - 30 * day_ms).to_string(),
    );
    meta_old.insert("memory_kind".to_string(), "constraint".to_string());
    meta_old.insert("salience_score".to_string(), "1.0".to_string());
    meta_old.insert("access_count".to_string(), "0".to_string());

    let mut meta_new = HashMap::new();
    meta_new.insert("timestamp_ms".to_string(), now_ms.to_string());
    meta_new.insert("memory_kind".to_string(), "observation".to_string());
    meta_new.insert("salience_score".to_string(), "0.2".to_string());
    meta_new.insert("access_count".to_string(), "0".to_string());

    let results = vec![
        SearchResult {
            doc_id: "old-constraint".to_string(),
            doc_type: "toc_node".to_string(),
            score: 0.85,
            text_preview: "Old but important constraint".to_string(),
            source_layer: RetrievalLayer::BM25,
            metadata: meta_old,
        },
        SearchResult {
            doc_id: "new-observation".to_string(),
            doc_type: "toc_node".to_string(),
            score: 0.85,
            text_preview: "Recent low-salience observation".to_string(),
            source_layer: RetrievalLayer::BM25,
            metadata: meta_new,
        },
    ];

    // Apply stale filter first (like route_query does)
    let stale_filter = StaleFilter::new(StalenessConfig {
        enabled: true,
        half_life_days: 14.0,
        max_penalty: 0.30,
        ..Default::default()
    });
    let after_stale = stale_filter.apply(results);

    // Constraint should be exempt from staleness decay
    let constraint = after_stale
        .iter()
        .find(|r| r.doc_id == "old-constraint")
        .unwrap();
    assert!(
        (constraint.score - 0.85).abs() < f32::EPSILON,
        "Constraint should be exempt from stale decay, got {:.4}",
        constraint.score
    );

    // Apply combined ranking
    let ranking_config = RankingConfig {
        salience_enabled: true,
        usage_decay_enabled: false,
        ..Default::default()
    };
    let ranked = apply_combined_ranking(after_stale, &ranking_config);

    let constraint_final = ranked
        .iter()
        .find(|r| r.doc_id == "old-constraint")
        .unwrap();
    let observation_final = ranked
        .iter()
        .find(|r| r.doc_id == "new-observation")
        .unwrap();

    assert!(
        constraint_final.score > observation_final.score,
        "High-salience constraint ({:.4}) should outrank low-salience observation ({:.4})",
        constraint_final.score,
        observation_final.score
    );
}
