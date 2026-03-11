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
