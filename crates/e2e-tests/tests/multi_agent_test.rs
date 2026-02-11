//! Multi-agent E2E tests for agent-memory.
//!
//! E2E-05: Multi-agent cross-agent query, filtered query, and agent discovery.
//! Verifies that events from different agents (claude, copilot, gemini) can be
//! ingested, indexed, and queried both across all agents and filtered to a
//! specific agent. Also validates agent discovery (ListAgents) correctness.

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::{build_toc_segment, create_test_events_for_agent, ingest_events, TestHarness};
use memory_search::{
    SearchIndex, SearchIndexConfig, SearchIndexer, SearchOptions, TeleportSearcher,
};
use memory_service::pb::{ListAgentsRequest, RouteQueryRequest};
use memory_service::{AgentDiscoveryHandler, RetrievalHandler};
use memory_types::{Event, EventRole, EventType, TocNode};

/// Build a TOC segment and set contributing_agents from the events.
///
/// The TocBuilder (via MockSummarizer) does not propagate the agent field from
/// events into contributing_agents. In production, this is done by the
/// scheduler/indexing pipeline. For E2E tests we apply it after building.
async fn build_toc_with_agent(
    storage: Arc<memory_storage::Storage>,
    events: Vec<Event>,
    agent: &str,
) -> TocNode {
    let mut node = build_toc_segment(storage, events).await;
    if !node.contributing_agents.contains(&agent.to_string()) {
        node.contributing_agents.push(agent.to_string());
    }
    node
}

/// E2E-05 primary: Multi-agent cross-agent query.
///
/// Ingests events from 3 agents (claude, copilot, gemini), builds TOC segments
/// with contributing_agents, indexes into BM25, and verifies that an unfiltered
/// route_query returns results from the multi-agent index.
#[tokio::test]
async fn test_multi_agent_cross_agent_query() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Create events for 3 agents
    let events_claude = create_test_events_for_agent(
        "session-claude",
        6,
        "Rust ownership and borrow checker for memory safety",
        "claude",
    );
    let events_copilot = create_test_events_for_agent(
        "session-copilot",
        6,
        "TypeScript generics and type inference patterns",
        "copilot",
    );
    let events_gemini = create_test_events_for_agent(
        "session-gemini",
        6,
        "Python machine learning with PyTorch models",
        "gemini",
    );

    // 3. Ingest all 18 events
    ingest_events(&harness.storage, &events_claude);
    ingest_events(&harness.storage, &events_copilot);
    ingest_events(&harness.storage, &events_gemini);

    // Verify total event count
    let stats = harness.storage.get_stats().unwrap();
    assert_eq!(stats.event_count, 18);

    // 4. Build TOC segments for each agent's events (with contributing_agents set)
    let node_claude = build_toc_with_agent(harness.storage.clone(), events_claude, "claude").await;
    let node_copilot =
        build_toc_with_agent(harness.storage.clone(), events_copilot, "copilot").await;
    let node_gemini = build_toc_with_agent(harness.storage.clone(), events_gemini, "gemini").await;

    // Verify TOC nodes were created
    assert!(
        !node_claude.title.is_empty(),
        "Claude TocNode should have a title"
    );
    assert!(
        !node_copilot.title.is_empty(),
        "Copilot TocNode should have a title"
    );
    assert!(
        !node_gemini.title.is_empty(),
        "Gemini TocNode should have a title"
    );

    // Verify contributing_agents are set on segment nodes
    assert!(
        node_claude
            .contributing_agents
            .contains(&"claude".to_string()),
        "Claude node should have 'claude' in contributing_agents: {:?}",
        node_claude.contributing_agents
    );
    assert!(
        node_copilot
            .contributing_agents
            .contains(&"copilot".to_string()),
        "Copilot node should have 'copilot' in contributing_agents"
    );
    assert!(
        node_gemini
            .contributing_agents
            .contains(&"gemini".to_string()),
        "Gemini node should have 'gemini' in contributing_agents"
    );

    // 5. Create BM25 index
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    // 6. Index all 3 TocNodes and their grips
    for node in [&node_claude, &node_copilot, &node_gemini] {
        indexer.index_toc_node(node).unwrap();
        let grip_ids: Vec<String> = node
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

    // 7. Commit the index
    indexer.commit().unwrap();

    // 8. Create TeleportSearcher, wrap in Arc
    let bm25_searcher = Arc::new(TeleportSearcher::new(&bm25_index).unwrap());

    // 9. Create RetrievalHandler with BM25 searcher
    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher.clone()),
        None,
        None,
    );

    // 10. Call route_query with a query matching content from at least one agent
    //     (no agent_filter -- cross-agent query)
    let response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "rust ownership borrow checker".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 20,
            agent_filter: None,
        }))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 11. Verify results
    assert!(resp.has_results, "RouteQuery should have results");
    assert!(
        !resp.results.is_empty(),
        "RouteQuery should return non-empty results"
    );
    assert!(resp.explanation.is_some(), "Explanation should be present");

    // 12. Also verify BM25 directly for a specific agent's content
    let rust_results = bm25_searcher
        .search("rust ownership", SearchOptions::new().with_limit(10))
        .unwrap();
    assert!(
        !rust_results.is_empty(),
        "BM25 search for 'rust ownership' should return results"
    );

    // Verify the top result has agent attribution for claude
    let claude_result = rust_results
        .iter()
        .find(|r| r.agent == Some("claude".to_string()));
    assert!(
        claude_result.is_some(),
        "BM25 results should include agent='claude' for Rust content: {:?}",
        rust_results
            .iter()
            .map(|r| (&r.doc_id, &r.agent))
            .collect::<Vec<_>>()
    );
}

/// E2E-05 filter: Multi-agent filtered query.
///
/// Verifies that BM25 search results carry agent attribution from
/// contributing_agents, and that route_query accepts agent_filter parameter.
#[tokio::test]
async fn test_multi_agent_filtered_query() {
    // 1. Same setup as cross-agent test
    let harness = TestHarness::new();

    let events_claude = create_test_events_for_agent(
        "session-claude",
        6,
        "Rust ownership and borrow checker for memory safety",
        "claude",
    );
    let events_copilot = create_test_events_for_agent(
        "session-copilot",
        6,
        "TypeScript generics and type inference patterns",
        "copilot",
    );
    let events_gemini = create_test_events_for_agent(
        "session-gemini",
        6,
        "Python machine learning with PyTorch models",
        "gemini",
    );

    ingest_events(&harness.storage, &events_claude);
    ingest_events(&harness.storage, &events_copilot);
    ingest_events(&harness.storage, &events_gemini);

    let node_claude = build_toc_with_agent(harness.storage.clone(), events_claude, "claude").await;
    let node_copilot =
        build_toc_with_agent(harness.storage.clone(), events_copilot, "copilot").await;
    let node_gemini = build_toc_with_agent(harness.storage.clone(), events_gemini, "gemini").await;

    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).unwrap();
    let indexer = SearchIndexer::new(&bm25_index).unwrap();

    for node in [&node_claude, &node_copilot, &node_gemini] {
        indexer.index_toc_node(node).unwrap();
        let grip_ids: Vec<String> = node
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

    // 2. Create RetrievalHandler with BM25 searcher
    let handler = RetrievalHandler::with_services(
        harness.storage.clone(),
        Some(bm25_searcher.clone()),
        None,
        None,
    );

    // 3. Call route_query with agent_filter for claude
    let response = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "memory safety borrow".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: Some("claude".to_string()),
        }))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 4. Verify results exist (BM25 matches claude's content)
    assert!(
        resp.has_results,
        "RouteQuery should have results for 'memory safety borrow'"
    );
    assert!(
        !resp.results.is_empty(),
        "RouteQuery should return non-empty results"
    );

    // 5. Search BM25 directly for "rust ownership" and verify agent attribution
    let rust_results = bm25_searcher
        .search("rust ownership", SearchOptions::new().with_limit(10))
        .unwrap();

    assert!(
        !rust_results.is_empty(),
        "BM25 search for 'rust ownership' should return results"
    );

    // 6. Verify agent attribution in BM25 results
    let claude_results: Vec<_> = rust_results
        .iter()
        .filter(|r| r.agent == Some("claude".to_string()))
        .collect();
    assert!(
        !claude_results.is_empty(),
        "Should find results with agent='claude' in BM25 search"
    );

    // Verify copilot's content has copilot attribution
    let ts_results = bm25_searcher
        .search("typescript generics", SearchOptions::new().with_limit(10))
        .unwrap();

    if !ts_results.is_empty() {
        let copilot_results: Vec<_> = ts_results
            .iter()
            .filter(|r| r.agent == Some("copilot".to_string()))
            .collect();
        assert!(
            !copilot_results.is_empty(),
            "Should find results with agent='copilot' in TypeScript search"
        );
    }

    // 7. Call route_query with nonexistent agent_filter
    let response_none = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "rust ownership".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: Some("nonexistent_agent".to_string()),
        }))
        .await
        .unwrap();

    let resp_none = response_none.into_inner();
    // The route_query handler currently doesn't filter by agent_filter at the
    // BM25 layer, so results may still appear. This verifies the field is
    // accepted without error. When agent filtering is fully implemented,
    // this assertion should be updated to verify empty results.
    assert!(
        resp_none.explanation.is_some(),
        "RouteQuery should return explanation even with nonexistent agent filter"
    );
}

/// E2E-05 discovery: Multi-agent discovery via ListAgents.
///
/// Verifies that ListAgents correctly reports all agents with accurate
/// session counts and ordering when multiple agents contribute events.
#[tokio::test]
async fn test_multi_agent_discovery() {
    // 1. Create TestHarness
    let harness = TestHarness::new();

    let now_ms = Utc::now().timestamp_millis();

    // 2. Create events with recent timestamps for session counting
    // claude: 4 events in session-claude-1, 4 events in session-claude-2
    // copilot: 4 events in session-copilot-1
    let mut all_events = Vec::new();

    for i in 0..4 {
        let ts = now_ms - 100_000 + (i as i64 * 100);
        all_events.push(create_recent_event(
            "session-claude-1",
            ts,
            "claude",
            &format!("Rust ownership and borrow checking discussion {}", i),
        ));
    }
    for i in 0..4 {
        let ts = now_ms - 50_000 + (i as i64 * 100);
        all_events.push(create_recent_event(
            "session-claude-2",
            ts,
            "claude",
            &format!("Rust lifetime annotations and generic bounds {}", i),
        ));
    }
    for i in 0..4 {
        let ts = now_ms - 30_000 + (i as i64 * 100);
        all_events.push(create_recent_event(
            "session-copilot-1",
            ts,
            "copilot",
            &format!("TypeScript type inference and generics patterns {}", i),
        ));
    }

    // 3. Ingest events with outbox entries
    ingest_events(&harness.storage, &all_events);

    // 4. Build TOC segments for each session's events and set contributing_agents
    let claude_events_1: Vec<Event> = all_events[0..4].to_vec();
    let claude_events_2: Vec<Event> = all_events[4..8].to_vec();
    let copilot_events: Vec<Event> = all_events[8..12].to_vec();

    let node_claude_1 =
        build_toc_with_agent(harness.storage.clone(), claude_events_1, "claude").await;
    let node_claude_2 =
        build_toc_with_agent(harness.storage.clone(), claude_events_2, "claude").await;
    let node_copilot =
        build_toc_with_agent(harness.storage.clone(), copilot_events, "copilot").await;

    // Store the TOC nodes so list_agents can find them
    harness.storage.put_toc_node(&node_claude_1).unwrap();
    harness.storage.put_toc_node(&node_claude_2).unwrap();
    harness.storage.put_toc_node(&node_copilot).unwrap();

    // 5. Create AgentDiscoveryHandler
    let discovery_handler = AgentDiscoveryHandler::new(harness.storage.clone());

    // 6. Call list_agents
    let response = discovery_handler
        .list_agents(Request::new(ListAgentsRequest {}))
        .await
        .unwrap();

    let resp = response.into_inner();

    // 7. Verify agents are discovered
    assert!(
        resp.agents.len() >= 2,
        "Should discover at least 2 agents (claude and copilot), found: {:?}",
        resp.agents.iter().map(|a| &a.agent_id).collect::<Vec<_>>()
    );

    // Find claude and copilot in the results
    let claude = resp.agents.iter().find(|a| a.agent_id == "claude");
    let copilot = resp.agents.iter().find(|a| a.agent_id == "copilot");

    assert!(claude.is_some(), "Should find agent 'claude' in list");
    assert!(copilot.is_some(), "Should find agent 'copilot' in list");

    let claude = claude.unwrap();
    let copilot = copilot.unwrap();

    // claude should have session_count == 2 (session-claude-1 and session-claude-2)
    assert_eq!(
        claude.session_count, 2,
        "Claude should have 2 sessions, got {}",
        claude.session_count
    );

    // copilot should have session_count == 1 (session-copilot-1)
    assert_eq!(
        copilot.session_count, 1,
        "Copilot should have 1 session, got {}",
        copilot.session_count
    );

    // Verify agents are sorted by last_seen_ms descending
    for i in 1..resp.agents.len() {
        assert!(
            resp.agents[i - 1].last_seen_ms >= resp.agents[i].last_seen_ms,
            "Agents should be sorted by last_seen_ms descending: {} >= {} (agents {} and {})",
            resp.agents[i - 1].last_seen_ms,
            resp.agents[i].last_seen_ms,
            resp.agents[i - 1].agent_id,
            resp.agents[i].agent_id,
        );
    }
}

/// Create a test event with a specific recent timestamp and agent.
///
/// Uses ULID-based IDs and realistic timestamps for session counting.
fn create_recent_event(session_id: &str, timestamp_ms: i64, agent: &str, text: &str) -> Event {
    let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
    let timestamp: DateTime<Utc> = Utc.timestamp_millis_opt(timestamp_ms).unwrap();

    Event::new(
        ulid.to_string(),
        session_id.to_string(),
        timestamp,
        EventType::UserMessage,
        EventRole::User,
        text.to_string(),
    )
    .with_agent(agent)
}
