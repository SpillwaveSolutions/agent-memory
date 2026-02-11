//! BM25 teleport search E2E tests for agent-memory.
//!
//! E2E-02: BM25 ingest -> index -> search with relevance ranking
//! Verifies BM25 keyword search returns results ranked by relevance score.

use pretty_assertions::assert_eq;

use e2e_tests::{build_toc_segment, create_test_events, ingest_events, TestHarness};
use memory_search::{
    DocType, SearchIndex, SearchIndexConfig, SearchIndexer, SearchOptions, TeleportSearcher,
};
use memory_types::{TocBullet, TocLevel, TocNode};

/// E2E-02: BM25 search pipeline with relevance ranking.
///
/// Ingests 3 topically distinct event segments, builds TOC nodes,
/// indexes into BM25, and verifies search returns correct ranking.
#[tokio::test]
async fn test_bm25_ingest_index_search_ranked() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create 3 distinct conversation segments about different topics
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
    let events_sql = create_test_events(
        "session-sql",
        6,
        "Database query optimization using SQL indexing and execution plans for performance",
    );

    // 3. Ingest all events
    ingest_events(&harness.storage, &events_rust);
    ingest_events(&harness.storage, &events_python);
    ingest_events(&harness.storage, &events_sql);

    // 4. Build TOC segments for each group
    let node_rust = build_toc_segment(harness.storage.clone(), events_rust).await;
    let node_python = build_toc_segment(harness.storage.clone(), events_python).await;
    let node_sql = build_toc_segment(harness.storage.clone(), events_sql).await;

    // 5. Create BM25 index
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    // 6. Index all 3 TocNodes
    indexer.index_toc_node(&node_rust).unwrap();
    indexer.index_toc_node(&node_python).unwrap();
    indexer.index_toc_node(&node_sql).unwrap();

    // Also index any grips from each node, tracking per-segment grip IDs
    let mut rust_doc_ids: Vec<String> = vec![node_rust.node_id.clone()];
    let mut python_doc_ids: Vec<String> = vec![node_python.node_id.clone()];
    let mut sql_doc_ids: Vec<String> = vec![node_sql.node_id.clone()];

    for (node, doc_ids) in [
        (&node_rust, &mut rust_doc_ids),
        (&node_python, &mut python_doc_ids),
        (&node_sql, &mut sql_doc_ids),
    ] {
        let grip_ids: Vec<String> = node
            .bullets
            .iter()
            .flat_map(|b| b.grip_ids.iter().cloned())
            .collect();
        for grip_id in &grip_ids {
            if let Some(grip) = harness.storage.get_grip(grip_id).unwrap() {
                indexer.index_grip(&grip).unwrap();
                doc_ids.push(grip_id.clone());
            }
        }
    }

    // 7. Commit the index
    indexer.commit().unwrap();

    // 8. Create TeleportSearcher
    let searcher = TeleportSearcher::new(&bm25_index).unwrap();

    // 9. Search for "rust ownership borrow"
    let results_rust = searcher
        .search(
            "rust ownership borrow",
            SearchOptions::new().with_limit(10),
        )
        .unwrap();

    // 10. Verify results
    assert!(
        !results_rust.is_empty(),
        "Search for 'rust ownership borrow' should return results"
    );

    // First result should be from the Rust segment (node or grip)
    assert!(
        rust_doc_ids.contains(&results_rust[0].doc_id),
        "Top result for Rust query should be from Rust segment, got: {}",
        results_rust[0].doc_id
    );

    // Results should be in descending score order
    for i in 1..results_rust.len() {
        assert!(
            results_rust[i - 1].score >= results_rust[i].score,
            "Results should be in descending score order: {} >= {} (positions {} and {})",
            results_rust[i - 1].score,
            results_rust[i].score,
            i - 1,
            i
        );
    }

    // No Python-segment result should rank higher than the top Rust-segment result
    let top_rust_score = results_rust[0].score;
    for result in &results_rust {
        if python_doc_ids.contains(&result.doc_id) {
            assert!(
                result.score <= top_rust_score,
                "Python result should not outrank the top Rust result"
            );
        }
    }

    // 11. Search for "python flask django" and verify Python segment ranks first
    let results_python = searcher
        .search(
            "python flask django",
            SearchOptions::new().with_limit(10),
        )
        .unwrap();

    assert!(
        !results_python.is_empty(),
        "Search for 'python flask django' should return results"
    );

    assert!(
        python_doc_ids.contains(&results_python[0].doc_id),
        "Top result for Python query should be from Python segment, got: {}",
        results_python[0].doc_id
    );

    // 12. Search for gibberish and verify 0 results
    let results_gibberish = searcher
        .search(
            "nonexistent_gibberish_term_xyz",
            SearchOptions::new().with_limit(10),
        )
        .unwrap();

    assert_eq!(
        results_gibberish.len(),
        0,
        "Search for nonexistent term should return 0 results"
    );
}

/// E2E-02b: BM25 search with document type filtering.
///
/// Verifies that doc_type filter correctly isolates TocNode vs Grip results.
#[tokio::test]
async fn test_bm25_search_filters_by_doc_type() {
    // 1. Create harness, ingest events, build TOC segment
    let harness = TestHarness::new();

    let events = create_test_events(
        "session-filter",
        8,
        "Rust memory allocation and heap management for systems programming",
    );
    ingest_events(&harness.storage, &events);
    let toc_node = build_toc_segment(harness.storage.clone(), events).await;

    // 2. Index both nodes and grips into BM25
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    indexer.index_toc_node(&toc_node).unwrap();

    let grip_ids: Vec<String> = toc_node
        .bullets
        .iter()
        .flat_map(|b| b.grip_ids.iter().cloned())
        .collect();

    let mut grip_count = 0;
    for grip_id in &grip_ids {
        if let Some(grip) = harness.storage.get_grip(grip_id).unwrap() {
            indexer.index_grip(&grip).unwrap();
            grip_count += 1;
        }
    }

    indexer.commit().unwrap();

    let searcher = TeleportSearcher::new(&bm25_index).unwrap();

    // 3. Search with TocNode filter
    let toc_results = searcher
        .search(
            "memory allocation",
            SearchOptions::new()
                .with_doc_type(DocType::TocNode)
                .with_limit(10),
        )
        .unwrap();

    for result in &toc_results {
        assert_eq!(
            result.doc_type,
            DocType::TocNode,
            "TocNode-filtered results should only contain TocNode docs"
        );
    }

    // 4. Search with Grip filter (only if grips exist)
    if grip_count > 0 {
        let grip_results = searcher
            .search(
                "memory allocation",
                SearchOptions::new()
                    .with_doc_type(DocType::Grip)
                    .with_limit(10),
            )
            .unwrap();

        for result in &grip_results {
            assert_eq!(
                result.doc_type,
                DocType::Grip,
                "Grip-filtered results should only contain Grip docs"
            );
        }
    }

    // 5. Search with no filter — verify TocNode results are present
    let all_results = searcher
        .search("memory allocation", SearchOptions::new().with_limit(20))
        .unwrap();

    let has_toc = all_results.iter().any(|r| r.doc_type == DocType::TocNode);
    assert!(has_toc, "Unfiltered search should include TocNode results");

    // If grips were indexed, unfiltered search should also include Grip results
    if grip_count > 0 {
        let has_grip = all_results.iter().any(|r| r.doc_type == DocType::Grip);
        assert!(has_grip, "Unfiltered search should include Grip results when grips are indexed");
    }
}

/// E2E-02c: BM25 search with agent attribution.
///
/// Verifies agent field propagation through BM25 indexing and search results.
#[tokio::test]
async fn test_bm25_search_with_agent_attribution() {
    let harness = TestHarness::new();

    // 1. Create BM25 index
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    // 2. Create a TocNode WITH contributing_agents = ["claude"]
    let node_with_agent = TocNode::new(
        "toc:segment:agent-test-1".to_string(),
        TocLevel::Segment,
        "Claude discussion about neural networks and transformers".to_string(),
        chrono::Utc::now(),
        chrono::Utc::now(),
    )
    .with_contributing_agent("claude");

    // Add searchable content via bullets and keywords
    let mut node_with_agent = node_with_agent;
    node_with_agent.bullets = vec![TocBullet::new(
        "Deep learning with neural networks and transformer architectures",
    )];
    node_with_agent.keywords = vec![
        "neural".to_string(),
        "transformers".to_string(),
        "claude".to_string(),
    ];

    // 3. Create a TocNode WITHOUT contributing_agents
    let mut node_without_agent = TocNode::new(
        "toc:segment:agent-test-2".to_string(),
        TocLevel::Segment,
        "General discussion about compilers and parsing".to_string(),
        chrono::Utc::now(),
        chrono::Utc::now(),
    );
    node_without_agent.bullets = vec![TocBullet::new(
        "Compiler design including lexer and parser implementation",
    )];
    node_without_agent.keywords = vec!["compilers".to_string(), "parsing".to_string()];

    // 4. Index both nodes
    indexer.index_toc_node(&node_with_agent).unwrap();
    indexer.index_toc_node(&node_without_agent).unwrap();
    indexer.commit().unwrap();

    // 5. Search and verify agent field on agent-attributed node
    let searcher = TeleportSearcher::new(&bm25_index).unwrap();

    let results_neural = searcher
        .search(
            "neural networks transformers",
            SearchOptions::new().with_limit(10),
        )
        .unwrap();

    assert!(
        !results_neural.is_empty(),
        "Search for 'neural networks' should return results"
    );

    // Find the result with our agent node
    let agent_result = results_neural
        .iter()
        .find(|r| r.doc_id == "toc:segment:agent-test-1");
    assert!(
        agent_result.is_some(),
        "Should find the agent-attributed node in results"
    );
    assert_eq!(
        agent_result.unwrap().agent,
        Some("claude".to_string()),
        "Agent field should be Some('claude') for agent-attributed node"
    );

    // 6. Search for non-agent node and verify agent is None
    let results_compiler = searcher
        .search(
            "compilers parsing lexer",
            SearchOptions::new().with_limit(10),
        )
        .unwrap();

    assert!(
        !results_compiler.is_empty(),
        "Search for 'compilers' should return results"
    );

    let no_agent_result = results_compiler
        .iter()
        .find(|r| r.doc_id == "toc:segment:agent-test-2");
    assert!(
        no_agent_result.is_some(),
        "Should find the non-agent node in results"
    );
    assert_eq!(
        no_agent_result.unwrap().agent, None,
        "Agent field should be None for node without contributing_agents"
    );
}
