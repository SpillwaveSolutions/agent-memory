//! End-to-end topic graph clustering tests for agent-memory.
//!
//! E2E-04: Topic creation -> storage -> retrieval via get_top_topics
//! E2E-04b: Topic search by keyword query
//! E2E-04c: Topic graph status reporting

use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::TestHarness;
use memory_service::pb::{GetTopTopicsRequest, GetTopicGraphStatusRequest};
use memory_service::TopicGraphHandler;
use memory_topics::{Topic, TopicStatus, TopicStorage};

/// Helper: create a test topic with the given attributes.
fn create_test_topic(
    id: &str,
    label: &str,
    keywords: &[&str],
    importance_score: f64,
) -> Topic {
    let mut topic = Topic::new(id.to_string(), label.to_string(), vec![0.0_f32; 384]);
    topic.importance_score = importance_score;
    topic.keywords = keywords.iter().map(|k| k.to_string()).collect();
    topic.status = TopicStatus::Active;
    topic
}

/// E2E-04: Verify get_top_topics returns topics ordered by importance score.
///
/// Creates 5 topics with known importance scores via TopicStorage::save_topic,
/// then verifies TopicGraphHandler::get_top_topics returns them in the correct
/// order with proper limiting.
#[tokio::test]
async fn test_topic_ingest_cluster_get_top_topics() {
    // 1. Create a TestHarness
    let harness = TestHarness::new();

    // 2. Create TopicStorage
    let topic_storage = TopicStorage::new(harness.storage.clone());

    // 3. Create 5 topics with distinct importance scores
    let topics = [
        create_test_topic(
            "topic-1",
            "Rust Memory Safety",
            &["rust", "ownership", "borrow"],
            0.9,
        ),
        create_test_topic(
            "topic-2",
            "Database Optimization",
            &["sql", "index", "query"],
            0.7,
        ),
        create_test_topic(
            "topic-3",
            "Authentication Design",
            &["auth", "jwt", "token"],
            0.5,
        ),
        create_test_topic(
            "topic-4",
            "Testing Strategies",
            &["test", "mock", "assert"],
            0.3,
        ),
        create_test_topic(
            "topic-5",
            "CI/CD Pipeline",
            &["ci", "deploy", "github"],
            0.1,
        ),
    ];

    // Save all topics
    for topic in &topics {
        topic_storage
            .save_topic(topic)
            .expect("Failed to save topic");
    }

    // 4. Create TopicGraphHandler
    let handler = TopicGraphHandler::new(
        Arc::new(topic_storage),
        harness.storage.clone(),
    );

    // 5. Call get_top_topics with limit: 3
    let response = handler
        .get_top_topics(Request::new(GetTopTopicsRequest {
            limit: 3,
            days: 30,
            agent_filter: None,
        }))
        .await
        .expect("get_top_topics failed");

    let result_topics = response.into_inner().topics;

    // 6. Verify: Response has 3 topics
    assert_eq!(
        result_topics.len(),
        3,
        "Should return exactly 3 topics with limit=3"
    );

    // 7. Verify: Topics are ordered by importance (highest first)
    assert_eq!(
        result_topics[0].label, "Rust Memory Safety",
        "First topic should be 'Rust Memory Safety' (highest importance)"
    );
    assert_eq!(
        result_topics[1].label, "Database Optimization",
        "Second topic should be 'Database Optimization'"
    );
    assert_eq!(
        result_topics[2].label, "Authentication Design",
        "Third topic should be 'Authentication Design'"
    );

    // 8. Verify: Each topic has non-empty label and topic_id
    for topic in &result_topics {
        assert!(!topic.id.is_empty(), "Topic id should not be empty");
        assert!(!topic.label.is_empty(), "Topic label should not be empty");
    }

    // 9. Verify: First topic importance >= second topic importance
    assert!(
        result_topics[0].importance_score >= result_topics[1].importance_score,
        "Topics should be sorted by importance descending: {} >= {}",
        result_topics[0].importance_score,
        result_topics[1].importance_score
    );
    assert!(
        result_topics[1].importance_score >= result_topics[2].importance_score,
        "Topics should be sorted by importance descending: {} >= {}",
        result_topics[1].importance_score,
        result_topics[2].importance_score
    );

    // 10. Call with limit: 1 and verify only 1 topic returned (the most important)
    let response_one = handler
        .get_top_topics(Request::new(GetTopTopicsRequest {
            limit: 1,
            days: 30,
            agent_filter: None,
        }))
        .await
        .expect("get_top_topics with limit=1 failed");

    let one_topic = response_one.into_inner().topics;
    assert_eq!(one_topic.len(), 1, "Should return exactly 1 topic with limit=1");
    assert_eq!(
        one_topic[0].label, "Rust Memory Safety",
        "The single returned topic should be the most important one"
    );
}

/// E2E-04b: Verify topic search by keyword query.
///
/// Uses the direct search_topics method to find topics matching keywords.
#[tokio::test]
async fn test_topic_search_by_query() {
    // 1. Create harness and topics (same setup)
    let harness = TestHarness::new();
    let topic_storage = TopicStorage::new(harness.storage.clone());

    let topics = [
        create_test_topic(
            "topic-1",
            "Rust Memory Safety",
            &["rust", "ownership", "borrow"],
            0.9,
        ),
        create_test_topic(
            "topic-2",
            "Database Optimization",
            &["sql", "index", "query"],
            0.7,
        ),
        create_test_topic(
            "topic-3",
            "Authentication Design",
            &["auth", "jwt", "token"],
            0.5,
        ),
    ];

    for topic in &topics {
        topic_storage.save_topic(topic).expect("Failed to save topic");
    }

    let handler = TopicGraphHandler::new(
        Arc::new(topic_storage),
        harness.storage.clone(),
    );

    // 2. Search for "rust ownership"
    let rust_results = handler
        .search_topics("rust ownership", 10)
        .await
        .expect("search_topics for 'rust ownership' failed");

    assert!(
        !rust_results.is_empty(),
        "Search for 'rust ownership' should return results"
    );
    assert_eq!(
        rust_results[0].label, "Rust Memory Safety",
        "First result for 'rust ownership' should be 'Rust Memory Safety'"
    );

    // 3. Search for "authentication jwt"
    let auth_results = handler
        .search_topics("authentication jwt", 10)
        .await
        .expect("search_topics for 'authentication jwt' failed");

    assert!(
        !auth_results.is_empty(),
        "Search for 'authentication jwt' should return results"
    );
    assert_eq!(
        auth_results[0].label, "Authentication Design",
        "First result for 'authentication jwt' should be 'Authentication Design'"
    );

    // 4. Search for nonexistent term
    let empty_results = handler
        .search_topics("nonexistent_xyz", 10)
        .await
        .expect("search_topics for nonexistent term failed");

    assert!(
        empty_results.is_empty(),
        "Search for 'nonexistent_xyz' should return empty results"
    );
}

/// E2E-04c: Verify topic graph status reporting.
///
/// Checks that get_status reports correct availability and topic count.
#[tokio::test]
async fn test_topic_graph_status() {
    // 1. Create harness and topics (same setup)
    let harness = TestHarness::new();
    let topic_storage = TopicStorage::new(harness.storage.clone());

    let topics = [
        create_test_topic(
            "topic-1",
            "Rust Memory Safety",
            &["rust", "ownership", "borrow"],
            0.9,
        ),
        create_test_topic(
            "topic-2",
            "Database Optimization",
            &["sql", "index", "query"],
            0.7,
        ),
        create_test_topic(
            "topic-3",
            "Authentication Design",
            &["auth", "jwt", "token"],
            0.5,
        ),
        create_test_topic(
            "topic-4",
            "Testing Strategies",
            &["test", "mock", "assert"],
            0.3,
        ),
        create_test_topic(
            "topic-5",
            "CI/CD Pipeline",
            &["ci", "deploy", "github"],
            0.1,
        ),
    ];

    for topic in &topics {
        topic_storage.save_topic(topic).expect("Failed to save topic");
    }

    let handler = TopicGraphHandler::new(
        Arc::new(topic_storage),
        harness.storage.clone(),
    );

    // 2. Call get_status
    let status = handler.get_status().await;

    // 3. Verify
    assert!(status.available, "Topic graph should be available when topics exist");
    assert_eq!(
        status.topic_count, 5,
        "Should report 5 topics"
    );

    // 4. Also verify via the RPC method
    let rpc_response = handler
        .get_topic_graph_status(Request::new(GetTopicGraphStatusRequest {}))
        .await
        .expect("get_topic_graph_status RPC failed");

    let rpc_status = rpc_response.into_inner();
    assert!(rpc_status.available, "RPC status should report available=true");
    assert_eq!(
        rpc_status.topic_count, 5,
        "RPC should report 5 topics"
    );
}
