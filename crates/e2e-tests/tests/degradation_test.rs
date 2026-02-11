//! Graceful degradation E2E tests for agent-memory.
//!
//! E2E-06: Verify the retrieval pipeline degrades gracefully when indexes
//! are unavailable. The system must never panic, must detect the correct
//! capability tier, must attempt appropriate fallback layers, and must
//! report useful warnings.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::{build_toc_segment, create_test_events, ingest_events, TestHarness};
use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer, TeleportSearcher};
use memory_service::pb::{
    CapabilityTier as ProtoTier, GetRetrievalCapabilitiesRequest, RouteQueryRequest,
};
use memory_service::RetrievalHandler;

/// E2E-06: Worst case -- all indexes missing, system falls back to Agentic-only tier.
///
/// Verifies the system works in Agentic-only mode when no search indexes are configured.
/// Data exists in storage (TOC segment built), but no BM25/Vector/Topic indexes are present.
#[tokio::test]
async fn test_degradation_all_indexes_missing() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create and ingest 6 events
    let events = create_test_events(
        "e2e-degrade-all-session",
        6,
        "Discussing graceful degradation patterns and error handling in distributed systems",
    );
    ingest_events(&harness.storage, &events);

    // 3. Build TOC segment (data exists in storage, but no search indexes)
    let _toc_node = build_toc_segment(harness.storage.clone(), events).await;

    // 4. Create RetrievalHandler with NO indexes
    let handler =
        RetrievalHandler::with_services(harness.storage.clone(), None, None, None);

    // 5. Call get_retrieval_capabilities
    let response = handler
        .get_retrieval_capabilities(Request::new(GetRetrievalCapabilitiesRequest {}))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 6. Verify tier and layer statuses
    assert_eq!(
        resp.tier,
        ProtoTier::Agentic as i32,
        "Tier should be Agentic when all indexes are missing"
    );

    let bm25_status = resp.bm25_status.expect("bm25_status should be present");
    assert!(!bm25_status.enabled, "BM25 should not be enabled");

    let vector_status = resp.vector_status.expect("vector_status should be present");
    assert!(!vector_status.enabled, "Vector should not be enabled");

    let topics_status = resp.topics_status.expect("topics_status should be present");
    assert!(!topics_status.enabled, "Topics should not be enabled");

    let agentic_status = resp.agentic_status.expect("agentic_status should be present");
    assert!(agentic_status.healthy, "Agentic should always be healthy");

    assert!(
        !resp.warnings.is_empty(),
        "Warnings should be non-empty when indexes are missing"
    );

    // 7. Call route_query -- must not panic or error
    let route_response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "what were we discussing?".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: None,
        }))
        .await
        .unwrap();

    let route_resp = route_response.into_inner();

    // 8. Verify route_query response
    let explanation = route_resp
        .explanation
        .expect("Explanation should be present");
    assert_eq!(
        explanation.tier,
        ProtoTier::Agentic as i32,
        "Explanation tier should be Agentic"
    );

    assert!(
        !route_resp.layers_attempted.is_empty(),
        "layers_attempted should be non-empty (at least Agentic)"
    );

    // has_results may be false (Agentic layer currently returns empty),
    // but the call must not fail -- we already verified that above by unwrap().
}

/// E2E-06: BM25 missing -- system detects degradation and still responds.
///
/// Verifies that when BM25 is not configured, the system detects the Agentic
/// tier and route_query does not fail.
#[tokio::test]
async fn test_degradation_no_bm25_index() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create and ingest events, build TOC segment
    let events = create_test_events(
        "e2e-degrade-bm25-session",
        6,
        "Authentication error handling with JWT token validation and refresh logic",
    );
    ingest_events(&harness.storage, &events);
    let _toc_node = build_toc_segment(harness.storage.clone(), events).await;

    // 3. Create RetrievalHandler with NO indexes (BM25 not configured)
    let handler =
        RetrievalHandler::with_services(harness.storage.clone(), None, None, None);

    // 4. Call get_retrieval_capabilities
    let response = handler
        .get_retrieval_capabilities(Request::new(GetRetrievalCapabilitiesRequest {}))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 5. Verify BM25 is not enabled
    let bm25_status = resp.bm25_status.expect("bm25_status should be present");
    assert!(!bm25_status.enabled, "BM25 should not be enabled");

    // 6. Verify tier is Agentic (since nothing else is configured either)
    assert_eq!(
        resp.tier,
        ProtoTier::Agentic as i32,
        "Tier should be Agentic when no indexes are configured"
    );

    // 7. Call route_query -- must succeed
    let route_response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "find the error message about auth".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: None,
        }))
        .await
        .unwrap();

    let route_resp = route_response.into_inner();

    // 8. Verify explanation tier reflects the degraded tier
    let explanation = route_resp
        .explanation
        .expect("Explanation should be present");
    assert_eq!(
        explanation.tier,
        ProtoTier::Agentic as i32,
        "Explanation tier should reflect degraded Agentic tier"
    );

    // 9. Verify the system attempted layers (candidates_considered in explanation)
    assert!(
        !explanation.candidates_considered.is_empty(),
        "candidates_considered should show layers the system tried"
    );
}

/// E2E-06: BM25 present, vector missing -- system uses Keyword tier.
///
/// Verifies that when only BM25 is configured, the system correctly detects
/// Keyword tier and returns BM25 results.
#[tokio::test]
async fn test_degradation_bm25_present_vector_missing() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create and ingest events, build TOC segment
    let events = create_test_events(
        "e2e-degrade-vector-session",
        6,
        "Rust ownership and borrow checker ensures memory safety without garbage collection",
    );
    ingest_events(&harness.storage, &events);
    let toc_node = build_toc_segment(harness.storage.clone(), events).await;

    // 3. Build BM25 index and index the TOC node (same pattern as pipeline_test.rs)
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    indexer.index_toc_node(&toc_node).unwrap();

    // Also index any grips
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

    // 4. Create RetrievalHandler with BM25 present, vector and topics absent
    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher),
        None,
        None,
    );

    // 5. Call get_retrieval_capabilities
    let response = handler
        .get_retrieval_capabilities(Request::new(GetRetrievalCapabilitiesRequest {}))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 6. Verify tier and statuses
    assert_eq!(
        resp.tier,
        ProtoTier::Keyword as i32,
        "Tier should be Keyword when only BM25 is present"
    );

    let bm25_status = resp.bm25_status.expect("bm25_status should be present");
    assert!(bm25_status.enabled, "BM25 should be enabled");
    assert!(bm25_status.healthy, "BM25 should be healthy (has docs)");

    let vector_status = resp.vector_status.expect("vector_status should be present");
    assert!(!vector_status.enabled, "Vector should not be enabled");

    let topics_status = resp.topics_status.expect("topics_status should be present");
    assert!(!topics_status.enabled, "Topics should not be enabled");

    // 7. Call route_query with terms matching the ingested content
    let route_response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "ownership borrow checker memory safety".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: None,
        }))
        .await
        .unwrap();

    let route_resp = route_response.into_inner();

    // 8. Verify results
    assert!(
        route_resp.has_results,
        "BM25 should find results for matching terms"
    );
    assert!(
        !route_resp.results.is_empty(),
        "Results should be non-empty"
    );

    let explanation = route_resp
        .explanation
        .expect("Explanation should be present");
    assert_eq!(
        explanation.tier,
        ProtoTier::Keyword as i32,
        "Explanation tier should be Keyword"
    );

    // Verify results have valid doc_ids
    for result in &route_resp.results {
        assert!(
            !result.doc_id.is_empty(),
            "Result doc_id should not be empty"
        );
    }

    // The system did NOT panic despite missing vector/topics -- verified by reaching here.
}

/// E2E-06: Capability warnings contain useful context about what is missing.
///
/// Verifies that the warnings returned by get_retrieval_capabilities contain
/// specific information about which indexes are missing.
#[tokio::test]
async fn test_degradation_capabilities_warnings_contain_context() {
    // 1. Create harness with storage only
    let harness = TestHarness::new();

    // 2. Create RetrievalHandler with NO indexes
    let handler =
        RetrievalHandler::with_services(harness.storage.clone(), None, None, None);

    // 3. Call get_retrieval_capabilities
    let response = handler
        .get_retrieval_capabilities(Request::new(GetRetrievalCapabilitiesRequest {}))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 4. Verify warnings list
    assert!(
        !resp.warnings.is_empty(),
        "Warnings should be non-empty when indexes are missing"
    );

    let warnings_joined = resp.warnings.join(" ").to_lowercase();

    // At least one warning mentions BM25
    assert!(
        warnings_joined.contains("bm25"),
        "Warnings should mention BM25, got: {:?}",
        resp.warnings
    );

    // At least one warning mentions Vector
    assert!(
        warnings_joined.contains("vector"),
        "Warnings should mention Vector, got: {:?}",
        resp.warnings
    );

    // At least one warning mentions Topic
    assert!(
        warnings_joined.contains("topic"),
        "Warnings should mention Topic, got: {:?}",
        resp.warnings
    );
}
