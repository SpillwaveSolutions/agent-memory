//! Query RPCs for TOC navigation and event retrieval.
//!
//! Per QRY-01: GetTocRoot returns top-level time nodes
//! Per QRY-02: GetNode returns node with children and summary
//! Per QRY-03: BrowseToc supports pagination of children
//! Per QRY-04: GetEvents retrieves raw events by time range
//! Per QRY-05: ExpandGrip retrieves context around grip excerpt

use std::sync::Arc;
use tracing::debug;

use memory_storage::Storage;
use memory_toc::{ExpandConfig, GripExpander};
use memory_types::{
    Event as DomainEvent,
    Grip as DomainGrip,
    TocLevel as DomainTocLevel,
    TocNode as DomainTocNode,
};

use crate::pb::{
    BrowseTocRequest, BrowseTocResponse,
    ExpandGripRequest, ExpandGripResponse,
    GetEventsRequest, GetEventsResponse,
    GetNodeRequest, GetNodeResponse,
    GetTocRootRequest, GetTocRootResponse,
    Event as PbEvent, EventRole as PbEventRole, EventType as PbEventType,
    Grip as PbGrip,
    TocBullet as PbTocBullet, TocLevel as PbTocLevel, TocNode as PbTocNode,
};

/// Convert domain TocLevel to proto TocLevel
fn to_pb_level(level: DomainTocLevel) -> i32 {
    match level {
        DomainTocLevel::Year => PbTocLevel::Year as i32,
        DomainTocLevel::Month => PbTocLevel::Month as i32,
        DomainTocLevel::Week => PbTocLevel::Week as i32,
        DomainTocLevel::Day => PbTocLevel::Day as i32,
        DomainTocLevel::Segment => PbTocLevel::Segment as i32,
    }
}

/// Convert domain TocNode to proto TocNode
fn to_pb_node(node: &DomainTocNode) -> PbTocNode {
    PbTocNode {
        node_id: node.node_id.clone(),
        level: to_pb_level(node.level),
        title: node.title.clone(),
        bullets: node.bullets.iter().map(|b| PbTocBullet {
            text: b.text.clone(),
            grip_ids: b.grip_ids.clone(),
        }).collect(),
        keywords: node.keywords.clone(),
        child_node_ids: node.child_node_ids.clone(),
        start_time_ms: node.start_time.timestamp_millis(),
        end_time_ms: node.end_time.timestamp_millis(),
    }
}

/// Convert domain EventRole to proto EventRole
fn to_pb_role(role: memory_types::EventRole) -> i32 {
    match role {
        memory_types::EventRole::User => PbEventRole::User as i32,
        memory_types::EventRole::Assistant => PbEventRole::Assistant as i32,
        memory_types::EventRole::System => PbEventRole::System as i32,
        memory_types::EventRole::Tool => PbEventRole::Tool as i32,
    }
}

/// Convert domain EventType to proto EventType
fn to_pb_event_type(event_type: memory_types::EventType) -> i32 {
    match event_type {
        memory_types::EventType::SessionStart => PbEventType::SessionStart as i32,
        memory_types::EventType::UserMessage => PbEventType::UserMessage as i32,
        memory_types::EventType::AssistantMessage => PbEventType::AssistantMessage as i32,
        memory_types::EventType::ToolResult => PbEventType::ToolResult as i32,
        memory_types::EventType::AssistantStop => PbEventType::AssistantStop as i32,
        memory_types::EventType::SubagentStart => PbEventType::SubagentStart as i32,
        memory_types::EventType::SubagentStop => PbEventType::SubagentStop as i32,
        memory_types::EventType::SessionEnd => PbEventType::SessionEnd as i32,
    }
}

/// Convert domain Event to proto Event
fn to_pb_event(event: &DomainEvent) -> PbEvent {
    PbEvent {
        event_id: event.event_id.clone(),
        session_id: event.session_id.clone(),
        timestamp_ms: event.timestamp.timestamp_millis(),
        event_type: to_pb_event_type(event.event_type.clone()),
        role: to_pb_role(event.role.clone()),
        text: event.text.clone(),
        metadata: event.metadata.clone(),
    }
}

/// Convert domain Grip to proto Grip
fn to_pb_grip(grip: &DomainGrip) -> PbGrip {
    PbGrip {
        grip_id: grip.grip_id.clone(),
        excerpt: grip.excerpt.clone(),
        event_id_start: grip.event_id_start.clone(),
        event_id_end: grip.event_id_end.clone(),
        timestamp_ms: grip.timestamp.timestamp_millis(),
        source: grip.source.clone(),
        toc_node_id: grip.toc_node_id.clone(),
    }
}

/// Handle GetTocRoot RPC.
///
/// Returns year-level nodes, optionally filtered by year.
pub fn get_toc_root(
    storage: &Storage,
    request: GetTocRootRequest,
) -> Result<GetTocRootResponse, tonic::Status> {
    debug!("GetTocRoot: year={:?}", request.year);

    let mut nodes = Vec::new();

    // If specific year requested, try to get that node
    if let Some(year) = request.year {
        let node_id = format!("toc:year:{}", year);
        if let Ok(Some(node)) = storage.get_toc_node(&node_id) {
            nodes.push(to_pb_node(&node));
        }
    } else {
        // Get all year nodes
        if let Ok(year_nodes) = storage.get_all_year_nodes() {
            for node in year_nodes {
                nodes.push(to_pb_node(&node));
            }
        }
    }

    // Sort by start time descending (most recent first)
    nodes.sort_by(|a, b| b.start_time_ms.cmp(&a.start_time_ms));

    Ok(GetTocRootResponse { nodes })
}

/// Handle GetNode RPC.
///
/// Returns a specific node by ID with its children and summary.
pub fn get_node(
    storage: &Storage,
    request: GetNodeRequest,
) -> Result<GetNodeResponse, tonic::Status> {
    debug!("GetNode: node_id={}", request.node_id);

    match storage.get_toc_node(&request.node_id) {
        Ok(Some(node)) => Ok(GetNodeResponse {
            node: Some(to_pb_node(&node)),
            found: true,
        }),
        Ok(None) => Ok(GetNodeResponse {
            node: None,
            found: false,
        }),
        Err(e) => Err(tonic::Status::internal(format!("Storage error: {}", e))),
    }
}

/// Handle BrowseToc RPC.
///
/// Returns paginated children of a node.
pub fn browse_toc(
    storage: &Storage,
    request: BrowseTocRequest,
) -> Result<BrowseTocResponse, tonic::Status> {
    let page_size = if request.page_size <= 0 || request.page_size > 100 {
        10
    } else {
        request.page_size as usize
    };

    debug!(
        "BrowseToc: parent={}, page_size={}, token={:?}",
        request.parent_node_id, page_size, request.continuation_token
    );

    // Get all children
    let all_children = storage.get_child_nodes(&request.parent_node_id)
        .map_err(|e| tonic::Status::internal(format!("Storage error: {}", e)))?;

    let total_count = all_children.len();

    // Parse continuation token as offset
    let offset: usize = if request.continuation_token.is_empty() {
        0
    } else {
        request.continuation_token.parse().unwrap_or(0)
    };

    // Get page of children
    let page_children: Vec<PbTocNode> = all_children
        .iter()
        .skip(offset)
        .take(page_size)
        .map(to_pb_node)
        .collect();

    let next_offset = offset + page_children.len();
    let has_more = next_offset < total_count;
    let next_token = if has_more {
        next_offset.to_string()
    } else {
        String::new()
    };

    Ok(BrowseTocResponse {
        children: page_children,
        next_continuation_token: next_token,
        has_more,
        total_count: total_count as i32,
    })
}

/// Handle GetEvents RPC.
///
/// Retrieves events in a time range.
pub fn get_events(
    storage: &Storage,
    request: GetEventsRequest,
) -> Result<GetEventsResponse, tonic::Status> {
    let limit = if request.limit <= 0 || request.limit > 1000 {
        100
    } else {
        request.limit as usize
    };

    debug!(
        "GetEvents: start={}, end={}, limit={}",
        request.start_time_ms, request.end_time_ms, limit
    );

    let events_data = storage.get_events_in_range(
        request.start_time_ms,
        request.end_time_ms,
    ).map_err(|e| tonic::Status::internal(format!("Storage error: {}", e)))?;

    let mut events = Vec::new();
    let mut has_more = false;

    for (_key, bytes) in events_data {
        if events.len() >= limit {
            has_more = true;
            break;
        }

        let event: DomainEvent = serde_json::from_slice(&bytes)
            .map_err(|e| tonic::Status::internal(format!("Deserialization error: {}", e)))?;
        events.push(to_pb_event(&event));
    }

    Ok(GetEventsResponse { events, has_more })
}

/// Handle ExpandGrip RPC.
///
/// Expands a grip with surrounding context events.
pub fn expand_grip(
    storage: Arc<Storage>,
    request: ExpandGripRequest,
) -> Result<ExpandGripResponse, tonic::Status> {
    let events_before = if request.events_before <= 0 { 3 } else { request.events_before as usize };
    let events_after = if request.events_after <= 0 { 3 } else { request.events_after as usize };

    debug!(
        "ExpandGrip: grip_id={}, before={}, after={}",
        request.grip_id, events_before, events_after
    );

    let config = ExpandConfig {
        events_before,
        events_after,
        ..Default::default()
    };

    let expander = GripExpander::with_config(storage.clone(), config);

    match expander.expand(&request.grip_id) {
        Ok(expanded) => Ok(ExpandGripResponse {
            grip: Some(to_pb_grip(&expanded.grip)),
            events_before: expanded.events_before.iter().map(to_pb_event).collect(),
            excerpt_events: expanded.excerpt_events.iter().map(to_pb_event).collect(),
            events_after: expanded.events_after.iter().map(to_pb_event).collect(),
            found: true,
        }),
        Err(memory_toc::ExpandError::GripNotFound(_)) => Ok(ExpandGripResponse {
            grip: None,
            events_before: Vec::new(),
            excerpt_events: Vec::new(),
            events_after: Vec::new(),
            found: false,
        }),
        Err(e) => Err(tonic::Status::internal(format!("Expand error: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use memory_types::{EventRole, EventType, TocNode as DomainTocNode};
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        (storage, temp_dir)
    }

    fn create_test_node(node_id: &str, level: DomainTocLevel, title: &str) -> DomainTocNode {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 29, 12, 0, 0).unwrap();
        let mut node = DomainTocNode::new(
            node_id.to_string(),
            level,
            title.to_string(),
            timestamp,
            timestamp,
        );
        node.keywords = vec!["test".to_string()];
        node
    }

    fn create_test_event(text: &str, timestamp_ms: i64) -> DomainEvent {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        DomainEvent::new(
            ulid.to_string(),
            "session-123".to_string(),
            chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[test]
    fn test_get_node_found() {
        let (storage, _temp) = create_test_storage();

        let node = create_test_node("toc:year:2024", DomainTocLevel::Year, "2024");
        storage.put_toc_node(&node).unwrap();

        let request = GetNodeRequest {
            node_id: "toc:year:2024".to_string(),
        };

        let response = get_node(&storage, request).unwrap();

        assert!(response.found);
        assert!(response.node.is_some());
        assert_eq!(response.node.unwrap().title, "2024");
    }

    #[test]
    fn test_get_node_not_found() {
        let (storage, _temp) = create_test_storage();

        let request = GetNodeRequest {
            node_id: "toc:year:1999".to_string(),
        };

        let response = get_node(&storage, request).unwrap();

        assert!(!response.found);
        assert!(response.node.is_none());
    }

    #[test]
    fn test_browse_toc_pagination() {
        let (storage, _temp) = create_test_storage();

        // Create parent with children
        let mut parent = create_test_node("toc:year:2024", DomainTocLevel::Year, "2024");
        for month in 1..=12 {
            let child_id = format!("toc:month:2024-{:02}", month);
            parent.child_node_ids.push(child_id.clone());

            let child = create_test_node(&child_id, DomainTocLevel::Month, &format!("Month {}", month));
            storage.put_toc_node(&child).unwrap();
        }
        storage.put_toc_node(&parent).unwrap();

        // First page
        let request = BrowseTocRequest {
            parent_node_id: "toc:year:2024".to_string(),
            page_size: 5,
            continuation_token: String::new(),
        };

        let response = browse_toc(&storage, request).unwrap();

        assert_eq!(response.children.len(), 5);
        assert!(response.has_more);
        assert_eq!(response.total_count, 12);
        assert_eq!(response.next_continuation_token, "5");

        // Second page
        let request = BrowseTocRequest {
            parent_node_id: "toc:year:2024".to_string(),
            page_size: 5,
            continuation_token: "5".to_string(),
        };

        let response = browse_toc(&storage, request).unwrap();

        assert_eq!(response.children.len(), 5);
        assert!(response.has_more);

        // Last page
        let request = BrowseTocRequest {
            parent_node_id: "toc:year:2024".to_string(),
            page_size: 5,
            continuation_token: "10".to_string(),
        };

        let response = browse_toc(&storage, request).unwrap();

        assert_eq!(response.children.len(), 2);
        assert!(!response.has_more);
    }

    #[test]
    fn test_get_events_basic() {
        let (storage, _temp) = create_test_storage();

        // Store some events
        for i in 0..5 {
            let ts = 1706540400000 + i * 60000; // 1 minute apart
            let event = create_test_event("test", ts);
            let event_bytes = serde_json::to_vec(&event).unwrap();
            storage.put_event(&event.event_id, &event_bytes, b"outbox").unwrap();
        }

        let request = GetEventsRequest {
            start_time_ms: 1706540400000,
            end_time_ms: 1706540700000,
            limit: 10,
        };

        let response = get_events(&storage, request).unwrap();

        assert_eq!(response.events.len(), 5);
        assert!(!response.has_more);
    }

    #[test]
    fn test_get_events_with_limit() {
        let (storage, _temp) = create_test_storage();

        // Store 10 events
        for i in 0..10 {
            let ts = 1706540400000 + i * 60000;
            let event = create_test_event("test", ts);
            let event_bytes = serde_json::to_vec(&event).unwrap();
            storage.put_event(&event.event_id, &event_bytes, b"outbox").unwrap();
        }

        let request = GetEventsRequest {
            start_time_ms: 1706540400000,
            end_time_ms: 1706541000000,
            limit: 5,
        };

        let response = get_events(&storage, request).unwrap();

        assert_eq!(response.events.len(), 5);
        assert!(response.has_more);
    }

    #[test]
    fn test_get_toc_root() {
        let (storage, _temp) = create_test_storage();

        // Create year nodes
        for year in [2022, 2023, 2024] {
            let node = create_test_node(&format!("toc:year:{}", year), DomainTocLevel::Year, &format!("{}", year));
            storage.put_toc_node(&node).unwrap();
        }

        let request = GetTocRootRequest { year: None };
        let response = get_toc_root(&storage, request).unwrap();

        assert_eq!(response.nodes.len(), 3);
    }

    #[test]
    fn test_get_toc_root_filtered() {
        let (storage, _temp) = create_test_storage();

        // Create year nodes
        for year in [2022, 2023, 2024] {
            let node = create_test_node(&format!("toc:year:{}", year), DomainTocLevel::Year, &format!("{}", year));
            storage.put_toc_node(&node).unwrap();
        }

        let request = GetTocRootRequest { year: Some(2024) };
        let response = get_toc_root(&storage, request).unwrap();

        assert_eq!(response.nodes.len(), 1);
        assert_eq!(response.nodes[0].title, "2024");
    }
}
