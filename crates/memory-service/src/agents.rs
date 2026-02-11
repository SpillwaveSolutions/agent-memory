//! Agent Discovery RPC handlers.
//!
//! Implements the Phase 23 Agent Discovery RPCs:
//! - ListAgents: List all contributing agents with summary statistics
//! - GetAgentActivity: Get agent activity bucketed by time period
//!
//! Per R4.3.1, R4.3.2: Cross-agent discovery and activity timeline.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Datelike, Utc};
use tonic::{Request, Response, Status};
use tracing::{debug, info};

use memory_storage::Storage;
use memory_types::{Event, TocLevel, TocNode};

use crate::pb::{
    ActivityBucket, AgentSummary, GetAgentActivityRequest, GetAgentActivityResponse,
    ListAgentsRequest, ListAgentsResponse,
};

/// Handler for agent discovery RPCs.
pub struct AgentDiscoveryHandler {
    storage: Arc<Storage>,
}

impl AgentDiscoveryHandler {
    /// Create a new agent discovery handler.
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Handle ListAgents RPC.
    ///
    /// Aggregates agents from TocNode.contributing_agents (O(k) over TOC nodes)
    /// and computes session_count from event scanning (bounded to last 365 days).
    /// Returns agent summaries sorted by last_seen_ms descending.
    pub async fn list_agents(
        &self,
        _request: Request<ListAgentsRequest>,
    ) -> Result<Response<ListAgentsResponse>, Status> {
        let all_nodes = self
            .iter_all_toc_nodes()
            .map_err(|e| Status::internal(format!("Failed to iterate TOC nodes: {}", e)))?;

        let mut agent_map: HashMap<String, AgentSummaryBuilder> = HashMap::new();

        for node in &all_nodes {
            let start_ms = node.start_time.timestamp_millis();
            let end_ms = node.end_time.timestamp_millis();

            for agent_id in &node.contributing_agents {
                let entry = agent_map
                    .entry(agent_id.clone())
                    .or_insert_with(|| AgentSummaryBuilder::new(agent_id.clone()));

                entry.node_count += 1;
                entry.first_seen_ms = entry.first_seen_ms.min(start_ms);
                entry.last_seen_ms = entry.last_seen_ms.max(end_ms);
            }
        }

        // Scan events for distinct session_ids per agent (bounded to last 365 days)
        let session_counts = self
            .count_sessions_per_agent()
            .map_err(|e| Status::internal(format!("Failed to count sessions: {}", e)))?;

        // Convert to proto summaries, sorted by last_seen descending
        let mut agents: Vec<AgentSummary> = agent_map
            .into_values()
            .map(|b| {
                let session_count = session_counts.get(&b.agent_id).copied().unwrap_or(0);
                AgentSummary {
                    agent_id: b.agent_id,
                    event_count: b.node_count, // Approximate: number of TOC nodes
                    session_count,
                    first_seen_ms: b.first_seen_ms,
                    last_seen_ms: b.last_seen_ms,
                }
            })
            .collect();

        agents.sort_by(|a, b| b.last_seen_ms.cmp(&a.last_seen_ms));

        info!(agent_count = agents.len(), "Listed agents");

        Ok(Response::new(ListAgentsResponse { agents }))
    }

    /// Handle GetAgentActivity RPC.
    ///
    /// Uses time-bounded event scans with chrono bucketing.
    pub async fn get_agent_activity(
        &self,
        request: Request<GetAgentActivityRequest>,
    ) -> Result<Response<GetAgentActivityResponse>, Status> {
        let req = request.into_inner();

        // Validate bucket
        let bucket = req.bucket.as_str();
        if bucket != "day" && bucket != "week" {
            return Err(Status::invalid_argument("bucket must be 'day' or 'week'"));
        }

        // Default from_ms to 30 days ago, to_ms to now
        let now_ms = Utc::now().timestamp_millis();
        let thirty_days_ms = 30 * 24 * 60 * 60 * 1000_i64;
        let from_ms = req.from_ms.unwrap_or(now_ms - thirty_days_ms);
        let to_ms = req.to_ms.unwrap_or(now_ms);

        // Get events in time range
        let raw_events = self
            .storage
            .get_events_in_range(from_ms, to_ms)
            .map_err(|e| Status::internal(format!("Failed to get events: {}", e)))?;

        // Parse events and filter by agent
        let mut buckets_map: HashMap<(String, i64), ActivityBucketBuilder> = HashMap::new();

        for (_key, bytes) in &raw_events {
            let event: Event = match serde_json::from_slice(bytes) {
                Ok(e) => e,
                Err(_) => continue, // Skip unparseable events
            };

            let agent_id = event.agent.as_deref().unwrap_or("unknown").to_string();

            // Filter by agent_id if provided
            if let Some(ref filter_agent) = req.agent_id {
                if agent_id != *filter_agent {
                    continue;
                }
            }

            let event_ms = event.timestamp.timestamp_millis();
            let (bucket_start, bucket_end) = compute_bucket(event_ms, bucket);

            let map_key = (agent_id.clone(), bucket_start);
            let entry = buckets_map
                .entry(map_key)
                .or_insert_with(|| ActivityBucketBuilder {
                    start_ms: bucket_start,
                    end_ms: bucket_end,
                    event_count: 0,
                    agent_id: agent_id.clone(),
                });
            entry.event_count += 1;
        }

        // Convert to proto buckets, sorted by start_ms ascending, then agent_id
        let mut buckets: Vec<ActivityBucket> = buckets_map
            .into_values()
            .map(|b| ActivityBucket {
                start_ms: b.start_ms,
                end_ms: b.end_ms,
                event_count: b.event_count,
                agent_id: b.agent_id,
            })
            .collect();

        buckets.sort_by(|a, b| {
            a.start_ms
                .cmp(&b.start_ms)
                .then_with(|| a.agent_id.cmp(&b.agent_id))
        });

        debug!(
            bucket_count = buckets.len(),
            bucket_type = bucket,
            "Agent activity bucketed"
        );

        Ok(Response::new(GetAgentActivityResponse { buckets }))
    }

    /// Count distinct session_ids per agent from events (bounded to last 365 days).
    ///
    /// Returns a map of agent_id -> session count.
    /// This is an O(n) scan over events, bounded to keep it performant.
    fn count_sessions_per_agent(&self) -> Result<HashMap<String, u64>, String> {
        let now_ms = Utc::now().timestamp_millis();
        let one_year_ms = 365_i64 * 24 * 60 * 60 * 1000;
        let from_ms = now_ms - one_year_ms;

        let raw_events = self
            .storage
            .get_events_in_range(from_ms, now_ms)
            .map_err(|e| e.to_string())?;

        let mut agent_sessions: HashMap<String, HashSet<String>> = HashMap::new();

        for (_key, bytes) in &raw_events {
            let event: Event = match serde_json::from_slice(bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let agent_id = event.agent.as_deref().unwrap_or("unknown").to_string();

            agent_sessions
                .entry(agent_id)
                .or_default()
                .insert(event.session_id.clone());
        }

        Ok(agent_sessions
            .into_iter()
            .map(|(agent_id, sessions)| (agent_id, sessions.len() as u64))
            .collect())
    }

    /// Iterate all TOC nodes from storage.
    ///
    /// This is O(k) where k = total TOC nodes (typically hundreds).
    fn iter_all_toc_nodes(&self) -> Result<Vec<TocNode>, String> {
        let mut all_nodes = Vec::new();
        for level in &[
            TocLevel::Year,
            TocLevel::Month,
            TocLevel::Week,
            TocLevel::Day,
            TocLevel::Segment,
        ] {
            let nodes = self
                .storage
                .get_toc_nodes_by_level(*level, None, None)
                .map_err(|e| e.to_string())?;
            all_nodes.extend(nodes);
        }
        Ok(all_nodes)
    }
}

/// Helper for building agent summaries.
struct AgentSummaryBuilder {
    agent_id: String,
    node_count: u64,
    first_seen_ms: i64,
    last_seen_ms: i64,
}

impl AgentSummaryBuilder {
    fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            node_count: 0,
            first_seen_ms: i64::MAX,
            last_seen_ms: i64::MIN,
        }
    }
}

/// Helper for building activity buckets.
struct ActivityBucketBuilder {
    start_ms: i64,
    end_ms: i64,
    event_count: u64,
    agent_id: String,
}

/// Compute the bucket start and end timestamps for a given event timestamp.
///
/// For "day": truncates to date boundary (UTC).
/// For "week": truncates to ISO week start (Monday, UTC).
fn compute_bucket(timestamp_ms: i64, bucket: &str) -> (i64, i64) {
    let dt = DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    let date = dt.date_naive();

    match bucket {
        "week" => {
            // ISO week start = Monday
            let days_from_monday = date.weekday().num_days_from_monday();
            let monday = date - chrono::Duration::days(days_from_monday as i64);
            let sunday = monday + chrono::Duration::days(7);
            let start = monday
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis();
            let end = sunday
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis();
            (start, end)
        }
        _ => {
            // "day" bucket
            let start = date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis();
            let next_day = date + chrono::Duration::days(1);
            let end = next_day
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis();
            (start, end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};
    use tempfile::TempDir;

    fn create_test_handler() -> (AgentDiscoveryHandler, Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        let handler = AgentDiscoveryHandler::new(storage.clone());
        (handler, storage, temp_dir)
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let (handler, _, _temp) = create_test_handler();

        let response = handler
            .list_agents(Request::new(ListAgentsRequest {}))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert!(resp.agents.is_empty());
    }

    #[tokio::test]
    async fn test_list_agents_aggregates_from_toc_nodes() {
        let (handler, storage, _temp) = create_test_handler();

        // Create test TOC nodes with contributing_agents
        let node1 = TocNode::new(
            "toc:day:2026-02-08".to_string(),
            TocLevel::Day,
            "February 8, 2026".to_string(),
            Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 2, 8, 23, 59, 59).unwrap(),
        )
        .with_contributing_agents(vec!["claude".to_string(), "opencode".to_string()]);

        let node2 = TocNode::new(
            "toc:day:2026-02-09".to_string(),
            TocLevel::Day,
            "February 9, 2026".to_string(),
            Utc.with_ymd_and_hms(2026, 2, 9, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 2, 9, 23, 59, 59).unwrap(),
        )
        .with_contributing_agents(vec!["claude".to_string()]);

        // Store nodes
        storage.put_toc_node(&node1).unwrap();
        storage.put_toc_node(&node2).unwrap();

        let response = handler
            .list_agents(Request::new(ListAgentsRequest {}))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert_eq!(resp.agents.len(), 2);

        // Should be sorted by last_seen_ms descending
        // claude: first_seen=2026-02-08, last_seen=2026-02-09, count=2
        // opencode: first_seen=2026-02-08, last_seen=2026-02-08, count=1
        let claude = resp.agents.iter().find(|a| a.agent_id == "claude").unwrap();
        assert_eq!(claude.event_count, 2); // 2 TOC nodes
        assert_eq!(
            claude.first_seen_ms,
            Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0)
                .unwrap()
                .timestamp_millis()
        );
        assert_eq!(
            claude.last_seen_ms,
            Utc.with_ymd_and_hms(2026, 2, 9, 23, 59, 59)
                .unwrap()
                .timestamp_millis()
        );

        let opencode = resp
            .agents
            .iter()
            .find(|a| a.agent_id == "opencode")
            .unwrap();
        assert_eq!(opencode.event_count, 1);

        // session_count should be 0 since no events were stored (only TOC nodes)
        assert_eq!(claude.session_count, 0);
        assert_eq!(opencode.session_count, 0);
    }

    #[tokio::test]
    async fn test_list_agents_session_count_from_events() {
        let (handler, storage, _temp) = create_test_handler();

        let now_ms = Utc::now().timestamp_millis();

        // Create events with different session_ids and agents
        // claude: 3 events across 2 sessions
        // opencode: 2 events across 1 session
        let events = vec![
            create_test_event("session-A", now_ms - 100_000, Some("claude")),
            create_test_event("session-A", now_ms - 90_000, Some("claude")),
            create_test_event("session-B", now_ms - 80_000, Some("claude")),
            create_test_event("session-C", now_ms - 70_000, Some("opencode")),
            create_test_event("session-C", now_ms - 60_000, Some("opencode")),
        ];

        for event in &events {
            let bytes = event.to_bytes().unwrap();
            let outbox =
                memory_types::OutboxEntry::for_toc(event.event_id.clone(), event.timestamp_ms());
            let outbox_bytes = outbox.to_bytes().unwrap();
            storage
                .put_event(&event.event_id, &bytes, &outbox_bytes)
                .unwrap();
        }

        // Create TOC nodes so agents appear in the list
        let node = TocNode::new(
            "toc:day:test-session-count".to_string(),
            TocLevel::Day,
            "Test day".to_string(),
            Utc::now() - chrono::Duration::hours(2),
            Utc::now(),
        )
        .with_contributing_agents(vec!["claude".to_string(), "opencode".to_string()]);

        storage.put_toc_node(&node).unwrap();

        let response = handler
            .list_agents(Request::new(ListAgentsRequest {}))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert_eq!(resp.agents.len(), 2);

        let claude = resp.agents.iter().find(|a| a.agent_id == "claude").unwrap();
        assert_eq!(claude.session_count, 2); // session-A and session-B

        let opencode = resp
            .agents
            .iter()
            .find(|a| a.agent_id == "opencode")
            .unwrap();
        assert_eq!(opencode.session_count, 1); // session-C only
    }

    #[tokio::test]
    async fn test_get_agent_activity_day_buckets() {
        let (handler, storage, _temp) = create_test_handler();

        // Create events spanning 3 days
        let events = vec![
            create_test_event("sess-1", 1707350400000, Some("claude")), // 2024-02-08
            create_test_event("sess-1", 1707354000000, Some("claude")), // 2024-02-08
            create_test_event("sess-2", 1707436800000, Some("claude")), // 2024-02-09
            create_test_event("sess-2", 1707523200000, Some("opencode")), // 2024-02-10
        ];

        for event in &events {
            let bytes = event.to_bytes().unwrap();
            let outbox =
                memory_types::OutboxEntry::for_toc(event.event_id.clone(), event.timestamp_ms());
            let outbox_bytes = outbox.to_bytes().unwrap();
            storage
                .put_event(&event.event_id, &bytes, &outbox_bytes)
                .unwrap();
        }

        let response = handler
            .get_agent_activity(Request::new(GetAgentActivityRequest {
                agent_id: None,
                from_ms: Some(1707350400000),
                to_ms: Some(1707609600000),
                bucket: "day".to_string(),
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        // Should have 3 buckets: claude on 02-08 (2 events), claude on 02-09 (1), opencode on 02-10 (1)
        assert_eq!(resp.buckets.len(), 3);

        // Sorted by start_ms ascending, then agent_id
        let claude_feb8 = resp
            .buckets
            .iter()
            .find(|b| b.agent_id == "claude" && b.event_count == 2)
            .unwrap();
        assert_eq!(claude_feb8.event_count, 2);
    }

    #[tokio::test]
    async fn test_get_agent_activity_week_buckets() {
        let (handler, storage, _temp) = create_test_handler();

        // Create events spanning 2+ weeks
        // 2024-02-05 (Mon) and 2024-02-12 (Mon) are different weeks
        let events = vec![
            create_test_event("sess-1", 1707091200000, Some("claude")), // 2024-02-05 (Mon)
            create_test_event("sess-1", 1707177600000, Some("claude")), // 2024-02-06 (Tue)
            create_test_event("sess-2", 1707696000000, Some("claude")), // 2024-02-12 (Mon next week)
        ];

        for event in &events {
            let bytes = event.to_bytes().unwrap();
            let outbox =
                memory_types::OutboxEntry::for_toc(event.event_id.clone(), event.timestamp_ms());
            let outbox_bytes = outbox.to_bytes().unwrap();
            storage
                .put_event(&event.event_id, &bytes, &outbox_bytes)
                .unwrap();
        }

        let response = handler
            .get_agent_activity(Request::new(GetAgentActivityRequest {
                agent_id: None,
                from_ms: Some(1707091200000),
                to_ms: Some(1707782400000),
                bucket: "week".to_string(),
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        // Should have 2 buckets: week of 02-05 (2 events) and week of 02-12 (1 event)
        assert_eq!(resp.buckets.len(), 2);
        assert_eq!(resp.buckets[0].event_count, 2);
        assert_eq!(resp.buckets[1].event_count, 1);
    }

    #[tokio::test]
    async fn test_get_agent_activity_filtered_by_agent() {
        let (handler, storage, _temp) = create_test_handler();

        let events = vec![
            create_test_event("sess-1", 1707350400000, Some("claude")),
            create_test_event("sess-1", 1707354000000, Some("opencode")),
            create_test_event("sess-2", 1707436800000, Some("claude")),
        ];

        for event in &events {
            let bytes = event.to_bytes().unwrap();
            let outbox =
                memory_types::OutboxEntry::for_toc(event.event_id.clone(), event.timestamp_ms());
            let outbox_bytes = outbox.to_bytes().unwrap();
            storage
                .put_event(&event.event_id, &bytes, &outbox_bytes)
                .unwrap();
        }

        let response = handler
            .get_agent_activity(Request::new(GetAgentActivityRequest {
                agent_id: Some("claude".to_string()),
                from_ms: Some(1707350400000),
                to_ms: Some(1707523200000),
                bucket: "day".to_string(),
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        // Only claude's events
        for bucket in &resp.buckets {
            assert_eq!(bucket.agent_id, "claude");
        }
        let total_events: u64 = resp.buckets.iter().map(|b| b.event_count).sum();
        assert_eq!(total_events, 2); // Only claude's 2 events
    }

    #[tokio::test]
    async fn test_get_agent_activity_time_range() {
        let (handler, storage, _temp) = create_test_handler();

        let events = vec![
            create_test_event("sess-1", 1707350400000, Some("claude")), // 2024-02-08
            create_test_event("sess-1", 1707436800000, Some("claude")), // 2024-02-09
            create_test_event("sess-2", 1707523200000, Some("claude")), // 2024-02-10
        ];

        for event in &events {
            let bytes = event.to_bytes().unwrap();
            let outbox =
                memory_types::OutboxEntry::for_toc(event.event_id.clone(), event.timestamp_ms());
            let outbox_bytes = outbox.to_bytes().unwrap();
            storage
                .put_event(&event.event_id, &bytes, &outbox_bytes)
                .unwrap();
        }

        // Only request events for 2024-02-08 and 2024-02-09
        let response = handler
            .get_agent_activity(Request::new(GetAgentActivityRequest {
                agent_id: None,
                from_ms: Some(1707350400000),
                to_ms: Some(1707523200000), // Exclusive of 2024-02-10
                bucket: "day".to_string(),
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        let total_events: u64 = resp.buckets.iter().map(|b| b.event_count).sum();
        assert_eq!(total_events, 2); // Only events on 2024-02-08 and 2024-02-09
    }

    #[tokio::test]
    async fn test_get_agent_activity_invalid_bucket() {
        let (handler, _, _temp) = create_test_handler();

        let result = handler
            .get_agent_activity(Request::new(GetAgentActivityRequest {
                agent_id: None,
                from_ms: None,
                to_ms: None,
                bucket: "month".to_string(),
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn test_compute_bucket_day() {
        // 2024-02-08 12:00:00 UTC = 1707393600000
        let (start, end) = compute_bucket(1707393600000, "day");

        // Should be midnight to midnight
        let expected_start = NaiveDate::from_ymd_opt(2024, 2, 8)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let expected_end = NaiveDate::from_ymd_opt(2024, 2, 9)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();

        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_compute_bucket_week() {
        // 2024-02-08 (Thursday) 12:00 UTC = 1707393600000
        let (start, end) = compute_bucket(1707393600000, "week");

        // Week should start on Monday 2024-02-05
        let expected_start = NaiveDate::from_ymd_opt(2024, 2, 5)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        // Week should end on Monday 2024-02-12
        let expected_end = NaiveDate::from_ymd_opt(2024, 2, 12)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();

        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    /// Helper to create test events with known timestamps and agents.
    /// Uses ULID-based event IDs as required by storage layer.
    fn create_test_event(session_id: &str, timestamp_ms: i64, agent: Option<&str>) -> Event {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        let event_id = ulid.to_string();
        let timestamp = DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());

        let mut event = Event::new(
            event_id.clone(),
            session_id.to_string(),
            timestamp,
            memory_types::EventType::UserMessage,
            memory_types::EventRole::User,
            format!("Test event {}", event_id),
        );

        if let Some(agent_id) = agent {
            event = event.with_agent(agent_id);
        }

        event
    }
}
