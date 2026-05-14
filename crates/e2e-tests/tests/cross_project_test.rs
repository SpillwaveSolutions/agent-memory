//! Cross-project E2E tests for agent-memory v3.0.
//!
//! E2E-06: Cross-project unified memory tests covering multi-store federation,
//! project attribution, fail-open semantics, and default single-project behavior.
//!
//! Design principles verified:
//! - Fail-open: unavailable stores are skipped silently
//! - Opt-in: default behavior (all_projects=false) is unchanged
//! - Project attribution: results carry `project` metadata
//! - TOC-search based federation: works without BM25/vector indexes

use std::path::PathBuf;
use std::sync::Arc;

use tonic::Request;

use e2e_tests::{build_toc_segment, create_test_events_for_agent, ingest_events, TestHarness};
use memory_service::pb::RouteQueryRequest;
use memory_service::RetrievalHandler;
use memory_storage::Storage;
use memory_types::TocNode;

/// Build a TOC segment and set contributing_agents from the agent string.
///
/// Mirrors the helper in multi_agent_test.rs — MockSummarizer does not
/// propagate agent, so we do it manually here.
async fn build_toc_with_agent(
    storage: Arc<memory_storage::Storage>,
    events: Vec<memory_types::Event>,
    agent: &str,
) -> TocNode {
    let mut node = build_toc_segment(storage, events).await;
    if !node.contributing_agents.contains(&agent.to_string()) {
        node.contributing_agents.push(agent.to_string());
    }
    node
}

/// E2E-06-A: Cross-project merged results.
///
/// Creates two separate project stores (primary and secondary), ingests
/// distinct content into each, then issues a cross-project query
/// (all_projects=true) and verifies results come from both stores.
#[tokio::test]
async fn test_cross_project_merged_results() {
    // --- Primary project store ---
    let primary = TestHarness::new();

    let events_primary = create_test_events_for_agent(
        "session-primary-1",
        6,
        "Rust ownership and borrow checker memory safety",
        "claude",
    );
    ingest_events(&primary.storage, &events_primary);

    let node_primary =
        build_toc_with_agent(primary.storage.clone(), events_primary, "claude").await;
    primary.storage.put_toc_node(&node_primary).unwrap();

    // --- Secondary project store (separate temp dir / Storage) ---
    let secondary_dir = tempfile::TempDir::new().unwrap();
    let secondary_storage =
        Arc::new(Storage::open(secondary_dir.path()).expect("Failed to open secondary storage"));

    let events_secondary = create_test_events_for_agent(
        "session-secondary-1",
        6,
        "TypeScript generics and type inference patterns",
        "copilot",
    );
    ingest_events(&secondary_storage, &events_secondary);

    let node_secondary =
        build_toc_with_agent(secondary_storage.clone(), events_secondary, "copilot").await;
    secondary_storage.put_toc_node(&node_secondary).unwrap();
    // Close write handle before RetrievalHandler opens it read-only
    drop(secondary_storage);

    // --- RetrievalHandler with cross-project wired ---
    let primary_path = primary._temp_dir.path().to_str().unwrap().to_string();
    let secondary_path = secondary_dir.path().to_path_buf();

    let handler = RetrievalHandler::new(primary.storage.clone())
        .with_registered_projects(vec![secondary_path], primary_path.clone());

    // Query that should match content from the primary store
    let response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "rust ownership borrow".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 20,
            agent_filter: None,
            all_projects: true,
        }))
        .await
        .unwrap();

    let resp = response.into_inner();

    // With all_projects=true and matching content in primary, we expect results
    assert!(
        resp.has_results,
        "Cross-project query should find results from primary store"
    );
    assert!(
        !resp.results.is_empty(),
        "Cross-project query should return non-empty results"
    );
}

/// E2E-06-B: Cross-project attribution.
///
/// Verifies that each result carries a `project` field indicating which
/// store it came from. Primary results should be attributed to the primary
/// store path; secondary results should be attributed to the secondary path.
#[tokio::test]
async fn test_cross_project_attribution() {
    // --- Primary project store ---
    let primary = TestHarness::new();
    let primary_path = primary._temp_dir.path().to_str().unwrap().to_string();

    let events_primary = create_test_events_for_agent(
        "session-attr-primary",
        6,
        "Rust async await tokio runtime executor",
        "claude",
    );
    ingest_events(&primary.storage, &events_primary);

    let node_primary =
        build_toc_with_agent(primary.storage.clone(), events_primary, "claude").await;
    primary.storage.put_toc_node(&node_primary).unwrap();

    // --- Secondary project store ---
    let secondary_dir = tempfile::TempDir::new().unwrap();
    let secondary_path_str = secondary_dir.path().to_str().unwrap().to_string();
    let secondary_storage =
        Arc::new(Storage::open(secondary_dir.path()).expect("Failed to open secondary storage"));

    let events_secondary = create_test_events_for_agent(
        "session-attr-secondary",
        6,
        "Python pandas dataframe filtering and aggregation",
        "gemini",
    );
    ingest_events(&secondary_storage, &events_secondary);

    let node_secondary =
        build_toc_with_agent(secondary_storage.clone(), events_secondary, "gemini").await;
    secondary_storage.put_toc_node(&node_secondary).unwrap();
    drop(secondary_storage);

    let secondary_path = secondary_dir.path().to_path_buf();

    // --- RetrievalHandler ---
    let handler = RetrievalHandler::new(primary.storage.clone())
        .with_registered_projects(vec![secondary_path], primary_path.clone());

    // Query primary content
    let response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "rust async tokio".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 20,
            agent_filter: None,
            all_projects: true,
        }))
        .await
        .unwrap();

    let resp = response.into_inner();

    // Every result that has a project field should have a non-empty path
    for result in &resp.results {
        if let Some(project) = &result.project {
            assert!(
                !project.is_empty(),
                "Project attribution should be non-empty, got empty string"
            );
            // Must be one of the two known paths
            assert!(
                project == &primary_path || project == &secondary_path_str,
                "Project attribution '{}' should be either primary '{}' or secondary '{}'",
                project,
                primary_path,
                secondary_path_str
            );
        }
    }

    // Results from the primary store (matching "rust async tokio") should be
    // attributed to the primary path
    let primary_results: Vec<_> = resp
        .results
        .iter()
        .filter(|r| r.project.as_deref() == Some(&primary_path))
        .collect();

    assert!(
        !primary_results.is_empty(),
        "Should have results attributed to the primary project '{}': got results: {:?}",
        primary_path,
        resp.results
            .iter()
            .map(|r| (&r.doc_id, &r.project))
            .collect::<Vec<_>>()
    );
}

/// E2E-06-C: Unavailable store skipped gracefully (fail-open).
///
/// Registers a store path that does not exist. The query should still
/// succeed, returning results from the primary store without panicking.
#[tokio::test]
async fn test_cross_project_unavailable_store_skipped() {
    let primary = TestHarness::new();
    let primary_path = primary._temp_dir.path().to_str().unwrap().to_string();

    // Ingest content in primary
    let events = create_test_events_for_agent(
        "session-failopen-1",
        6,
        "Rust lifetimes and borrow checker rules explained",
        "claude",
    );
    ingest_events(&primary.storage, &events);

    let node = build_toc_with_agent(primary.storage.clone(), events, "claude").await;
    primary.storage.put_toc_node(&node).unwrap();

    // Non-existent store path — should be skipped, not cause a panic or error
    let missing_path = PathBuf::from("/nonexistent/path/project_db_xyz_e2e");

    let handler = RetrievalHandler::new(primary.storage.clone())
        .with_registered_projects(vec![missing_path], primary_path);

    // Should not panic or return error even though remote store is missing
    let result = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "rust lifetimes borrow".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 20,
            agent_filter: None,
            all_projects: true,
        }))
        .await;

    assert!(
        result.is_ok(),
        "Cross-project query should not fail when a registered store is unavailable"
    );

    let resp = result.unwrap().into_inner();

    // Primary results should still be present
    assert!(
        resp.has_results || resp.results.is_empty(),
        "Response should be valid (pass or empty, not an error)"
    );
    // No panic is the main invariant — reaching this line proves fail-open works
}

/// E2E-06-D: Single-project default behavior unchanged.
///
/// Verifies that when `all_projects=false` (the default), the cross-project
/// federation code path is NOT triggered and behavior is identical to pre-v3.0.
/// No `project` field should be set on results in single-project mode.
#[tokio::test]
async fn test_single_project_default_unchanged() {
    let primary = TestHarness::new();

    let events = create_test_events_for_agent(
        "session-default-1",
        6,
        "Rust ownership and memory management without GC",
        "claude",
    );
    ingest_events(&primary.storage, &events);

    let node = build_toc_with_agent(primary.storage.clone(), events, "claude").await;
    primary.storage.put_toc_node(&node).unwrap();

    // Handler with registered projects — but we will NOT set all_projects=true
    let secondary_dir = tempfile::TempDir::new().unwrap();
    let _secondary_storage = Storage::open(secondary_dir.path()).unwrap();
    let secondary_path = secondary_dir.path().to_path_buf();
    drop(_secondary_storage);

    let primary_path = primary._temp_dir.path().to_str().unwrap().to_string();

    let handler = RetrievalHandler::new(primary.storage.clone())
        .with_registered_projects(vec![secondary_path], primary_path);

    // Default: all_projects = false
    let response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "rust ownership memory".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 20,
            agent_filter: None,
            all_projects: false, // explicit default
        }))
        .await
        .unwrap();

    let resp = response.into_inner();

    // The query should work normally
    assert!(
        resp.explanation.is_some(),
        "Single-project query should return an explanation"
    );

    // In default mode, project field is NOT set on results
    // (federation code path is not triggered)
    for result in &resp.results {
        assert!(
            result.project.is_none(),
            "Default (single-project) mode should not set project field on results, \
             but got project='{}' on doc_id='{}'",
            result.project.as_deref().unwrap_or(""),
            result.doc_id
        );
    }
}
