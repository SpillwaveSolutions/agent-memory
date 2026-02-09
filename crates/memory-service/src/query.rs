//! Query RPC implementations.
//!
//! Per QRY-01 through QRY-05: TOC navigation and event retrieval.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, warn};

use memory_storage::Storage;
use memory_types::{
    Event, EventRole, EventType, TocLevel as DomainTocLevel, TocNode as DomainTocNode,
};

use crate::pb::{
    BrowseTocRequest, BrowseTocResponse, Event as ProtoEvent, EventRole as ProtoEventRole,
    EventType as ProtoEventType, ExpandGripRequest, ExpandGripResponse, GetEventsRequest,
    GetEventsResponse, GetNodeRequest, GetNodeResponse, GetTocRootRequest, GetTocRootResponse,
    Grip as ProtoGrip, MemoryKind as ProtoMemoryKind, TocBullet as ProtoTocBullet,
    TocLevel as ProtoTocLevel, TocNode as ProtoTocNode,
};

/// Get root TOC nodes (year level).
///
/// Per QRY-01: GetTocRoot returns top-level time nodes.
pub async fn get_toc_root(
    storage: Arc<Storage>,
    _request: Request<GetTocRootRequest>,
) -> Result<Response<GetTocRootResponse>, Status> {
    debug!("GetTocRoot request");

    let year_nodes = storage
        .get_toc_nodes_by_level(DomainTocLevel::Year, None, None)
        .map_err(|e| Status::internal(format!("Storage error: {}", e)))?;

    // Sort by time descending (most recent first)
    let mut nodes: Vec<ProtoTocNode> = year_nodes.into_iter().map(domain_to_proto_node).collect();
    nodes.reverse();

    Ok(Response::new(GetTocRootResponse { nodes }))
}

/// Get a specific TOC node by ID.
///
/// Per QRY-02: GetNode returns node with children and summary.
pub async fn get_node(
    storage: Arc<Storage>,
    request: Request<GetNodeRequest>,
) -> Result<Response<GetNodeResponse>, Status> {
    let req = request.into_inner();
    debug!("GetNode request: {}", req.node_id);

    if req.node_id.is_empty() {
        return Err(Status::invalid_argument("node_id is required"));
    }

    let node = storage
        .get_toc_node(&req.node_id)
        .map_err(|e| Status::internal(format!("Storage error: {}", e)))?;

    let proto_node = node.map(domain_to_proto_node);

    Ok(Response::new(GetNodeResponse { node: proto_node }))
}

/// Browse children of a TOC node with pagination.
///
/// Per QRY-03: BrowseToc supports pagination of children.
pub async fn browse_toc(
    storage: Arc<Storage>,
    request: Request<BrowseTocRequest>,
) -> Result<Response<BrowseTocResponse>, Status> {
    let req = request.into_inner();
    debug!(
        "BrowseToc request: parent={}, limit={}",
        req.parent_id, req.limit
    );

    if req.parent_id.is_empty() {
        return Err(Status::invalid_argument("parent_id is required"));
    }

    let limit = if req.limit <= 0 {
        20
    } else {
        req.limit as usize
    };
    let offset: usize = req
        .continuation_token
        .as_ref()
        .and_then(|t| t.parse().ok())
        .unwrap_or(0);

    // Get all child nodes
    let all_children = storage
        .get_child_nodes(&req.parent_id)
        .map_err(|e| Status::internal(format!("Storage error: {}", e)))?;

    // Apply pagination
    let total = all_children.len();
    let children: Vec<ProtoTocNode> = all_children
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(domain_to_proto_node)
        .collect();

    let next_offset = offset + children.len();
    let has_more = next_offset < total;
    let continuation_token = if has_more {
        Some(next_offset.to_string())
    } else {
        None
    };

    Ok(Response::new(BrowseTocResponse {
        children,
        continuation_token,
        has_more,
    }))
}

/// Get events in a time range.
///
/// Per QRY-04: GetEvents retrieves raw events by time range.
pub async fn get_events(
    storage: Arc<Storage>,
    request: Request<GetEventsRequest>,
) -> Result<Response<GetEventsResponse>, Status> {
    let req = request.into_inner();
    debug!(
        "GetEvents request: from={} to={} limit={}",
        req.from_timestamp_ms, req.to_timestamp_ms, req.limit
    );

    let limit = if req.limit <= 0 {
        50
    } else {
        req.limit as usize
    };

    let raw_events = storage
        .get_events_in_range(req.from_timestamp_ms, req.to_timestamp_ms)
        .map_err(|e| Status::internal(format!("Storage error: {}", e)))?;

    let has_more = raw_events.len() > limit;

    let mut events = Vec::new();
    for (_key, bytes) in raw_events.into_iter().take(limit) {
        match Event::from_bytes(&bytes) {
            Ok(event) => events.push(domain_to_proto_event(event)),
            Err(e) => {
                warn!("Failed to deserialize event: {}", e);
                continue;
            }
        }
    }

    Ok(Response::new(GetEventsResponse { events, has_more }))
}

/// Expand a grip to show context events.
///
/// Per QRY-05: ExpandGrip retrieves context around grip excerpt.
pub async fn expand_grip(
    storage: Arc<Storage>,
    request: Request<ExpandGripRequest>,
) -> Result<Response<ExpandGripResponse>, Status> {
    let req = request.into_inner();
    debug!("ExpandGrip request: {}", req.grip_id);

    if req.grip_id.is_empty() {
        return Err(Status::invalid_argument("grip_id is required"));
    }

    // Get the grip
    let grip = match storage.get_grip(&req.grip_id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            warn!("Grip not found: {}", req.grip_id);
            return Ok(Response::new(ExpandGripResponse {
                grip: None,
                events_before: vec![],
                excerpt_events: vec![],
                events_after: vec![],
            }));
        }
        Err(e) => return Err(Status::internal(format!("Storage error: {}", e))),
    };

    let events_before_count = req.events_before.unwrap_or(3) as usize;
    let events_after_count = req.events_after.unwrap_or(3) as usize;

    // Get events around the grip's time range
    // The grip has timestamp_ms which we use to find surrounding events
    let grip_time = grip.timestamp.timestamp_millis();

    // Query a wider time range to get context
    let time_window_ms: i64 = 3600000; // 1 hour window
    let start_time = grip_time.saturating_sub(time_window_ms);
    let end_time = grip_time.saturating_add(time_window_ms);

    let raw_events = storage
        .get_events_in_range(start_time, end_time)
        .map_err(|e| Status::internal(format!("Storage error: {}", e)))?;

    // Deserialize events
    let all_events: Vec<Event> = raw_events
        .into_iter()
        .filter_map(|(_key, bytes)| Event::from_bytes(&bytes).ok())
        .collect();

    // Find the grip's events and partition
    let mut events_before = Vec::new();
    let mut excerpt_events = Vec::new();
    let mut events_after = Vec::new();

    let mut found_start = false;
    let mut found_end = false;

    for event in all_events {
        if !found_start {
            if event.event_id == grip.event_id_start {
                found_start = true;
                excerpt_events.push(event);
            } else {
                events_before.push(event);
            }
        } else if !found_end {
            excerpt_events.push(event.clone());
            if event.event_id == grip.event_id_end {
                found_end = true;
            }
        } else {
            events_after.push(event);
        }
    }

    // Limit the before/after events
    let events_before: Vec<ProtoEvent> = events_before
        .into_iter()
        .rev()
        .take(events_before_count)
        .rev()
        .map(domain_to_proto_event)
        .collect();

    let excerpt_events: Vec<ProtoEvent> = excerpt_events
        .into_iter()
        .map(domain_to_proto_event)
        .collect();

    let events_after: Vec<ProtoEvent> = events_after
        .into_iter()
        .take(events_after_count)
        .map(domain_to_proto_event)
        .collect();

    let proto_grip = ProtoGrip {
        grip_id: grip.grip_id,
        excerpt: grip.excerpt,
        event_id_start: grip.event_id_start,
        event_id_end: grip.event_id_end,
        timestamp_ms: grip.timestamp.timestamp_millis(),
        source: grip.source,
        // Phase 16 fields - defaults for now
        salience_score: 0.5,
        memory_kind: ProtoMemoryKind::Observation as i32,
        is_pinned: false,
    };

    Ok(Response::new(ExpandGripResponse {
        grip: Some(proto_grip),
        events_before,
        excerpt_events,
        events_after,
    }))
}

// ===== Type Conversion Functions =====

fn domain_to_proto_node(node: DomainTocNode) -> ProtoTocNode {
    let level = match node.level {
        DomainTocLevel::Year => ProtoTocLevel::Year,
        DomainTocLevel::Month => ProtoTocLevel::Month,
        DomainTocLevel::Week => ProtoTocLevel::Week,
        DomainTocLevel::Day => ProtoTocLevel::Day,
        DomainTocLevel::Segment => ProtoTocLevel::Segment,
    };

    let bullets: Vec<ProtoTocBullet> = node
        .bullets
        .into_iter()
        .map(|b| ProtoTocBullet {
            text: b.text,
            grip_ids: b.grip_ids,
        })
        .collect();

    // Generate summary from first bullet text if available
    let summary = if !bullets.is_empty() {
        Some(
            bullets
                .iter()
                .map(|b| b.text.clone())
                .collect::<Vec<_>>()
                .join(" "),
        )
    } else {
        None
    };

    ProtoTocNode {
        node_id: node.node_id,
        level: level as i32,
        title: node.title,
        summary,
        bullets,
        keywords: node.keywords,
        child_node_ids: node.child_node_ids,
        start_time_ms: node.start_time.timestamp_millis(),
        end_time_ms: node.end_time.timestamp_millis(),
        version: node.version as i32,
        // Phase 16 fields - defaults for now
        salience_score: 0.5,
        memory_kind: ProtoMemoryKind::Observation as i32,
        is_pinned: false,
    }
}

fn domain_to_proto_event(event: Event) -> ProtoEvent {
    let event_type = match event.event_type {
        EventType::SessionStart => ProtoEventType::SessionStart,
        EventType::UserMessage => ProtoEventType::UserMessage,
        EventType::AssistantMessage => ProtoEventType::AssistantMessage,
        EventType::ToolResult => ProtoEventType::ToolResult,
        EventType::AssistantStop => ProtoEventType::AssistantStop,
        EventType::SubagentStart => ProtoEventType::SubagentStart,
        EventType::SubagentStop => ProtoEventType::SubagentStop,
        EventType::SessionEnd => ProtoEventType::SessionEnd,
    };

    let role = match event.role {
        EventRole::User => ProtoEventRole::User,
        EventRole::Assistant => ProtoEventRole::Assistant,
        EventRole::System => ProtoEventRole::System,
        EventRole::Tool => ProtoEventRole::Tool,
    };

    ProtoEvent {
        event_id: event.event_id,
        session_id: event.session_id,
        timestamp_ms: event.timestamp.timestamp_millis(),
        event_type: event_type as i32,
        role: role as i32,
        text: event.text,
        metadata: event.metadata,
        agent: event.agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_get_toc_root_empty() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(GetTocRootRequest {});
        let response = get_toc_root(storage, request).await.unwrap();
        assert!(response.into_inner().nodes.is_empty());
    }

    #[tokio::test]
    async fn test_get_node_not_found() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(GetNodeRequest {
            node_id: "nonexistent".to_string(),
        });
        let response = get_node(storage, request).await.unwrap();
        assert!(response.into_inner().node.is_none());
    }

    #[tokio::test]
    async fn test_get_node_empty_id() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(GetNodeRequest {
            node_id: "".to_string(),
        });
        let result = get_node(storage, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_browse_toc_empty() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(BrowseTocRequest {
            parent_id: "toc:year:2026".to_string(),
            limit: 10,
            continuation_token: None,
        });
        let response = browse_toc(storage, request).await.unwrap();
        let resp = response.into_inner();
        assert!(resp.children.is_empty());
        assert!(!resp.has_more);
    }

    #[tokio::test]
    async fn test_get_events_empty() {
        let (storage, _temp) = create_test_storage();
        let now = Utc::now().timestamp_millis();
        let request = Request::new(GetEventsRequest {
            from_timestamp_ms: now - 3600000,
            to_timestamp_ms: now,
            limit: 50,
        });
        let response = get_events(storage, request).await.unwrap();
        assert!(response.into_inner().events.is_empty());
    }

    #[tokio::test]
    async fn test_expand_grip_not_found() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(ExpandGripRequest {
            grip_id: "nonexistent".to_string(),
            events_before: None,
            events_after: None,
        });
        let response = expand_grip(storage, request).await.unwrap();
        let resp = response.into_inner();
        assert!(resp.grip.is_none());
    }

    #[test]
    fn test_domain_to_proto_node() {
        let node = DomainTocNode::new(
            "toc:year:2026".to_string(),
            DomainTocLevel::Year,
            "2026".to_string(),
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap(),
        );

        let proto = domain_to_proto_node(node);

        assert_eq!(proto.node_id, "toc:year:2026");
        assert_eq!(proto.level, ProtoTocLevel::Year as i32);
        assert_eq!(proto.title, "2026");
    }

    #[test]
    fn test_domain_to_proto_event() {
        let event = Event::new(
            "evt-1".to_string(),
            "session-1".to_string(),
            Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            "Hello!".to_string(),
        );

        let proto = domain_to_proto_event(event);

        assert_eq!(proto.event_id, "evt-1");
        assert_eq!(proto.role, ProtoEventRole::User as i32);
        assert_eq!(proto.event_type, ProtoEventType::UserMessage as i32);
    }
}
