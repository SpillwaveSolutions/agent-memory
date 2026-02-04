//! Search RPC implementations.
//!
//! Per SEARCH-01, SEARCH-02: TOC node search via term matching.

use std::cmp::Ordering;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::debug;

use memory_storage::Storage;
use memory_toc::search::{
    search_node as core_search_node, SearchField as DomainSearchField,
    SearchMatch as DomainSearchMatch,
};
use memory_types::TocLevel as DomainTocLevel;

use crate::pb::{
    SearchChildrenRequest, SearchChildrenResponse, SearchField as ProtoSearchField,
    SearchMatch as ProtoSearchMatch, SearchNodeRequest, SearchNodeResponse,
    SearchNodeResult as ProtoSearchNodeResult, TocLevel as ProtoTocLevel,
};

/// Convert proto SearchField to domain SearchField.
fn proto_to_domain_field(proto: i32) -> Option<DomainSearchField> {
    match ProtoSearchField::try_from(proto) {
        Ok(ProtoSearchField::Title) => Some(DomainSearchField::Title),
        Ok(ProtoSearchField::Summary) => Some(DomainSearchField::Summary),
        Ok(ProtoSearchField::Bullets) => Some(DomainSearchField::Bullets),
        Ok(ProtoSearchField::Keywords) => Some(DomainSearchField::Keywords),
        _ => None, // Unspecified or invalid
    }
}

/// Convert domain SearchField to proto SearchField.
fn domain_to_proto_field(domain: DomainSearchField) -> i32 {
    match domain {
        DomainSearchField::Title => ProtoSearchField::Title as i32,
        DomainSearchField::Summary => ProtoSearchField::Summary as i32,
        DomainSearchField::Bullets => ProtoSearchField::Bullets as i32,
        DomainSearchField::Keywords => ProtoSearchField::Keywords as i32,
    }
}

/// Convert domain SearchMatch to proto SearchMatch.
fn domain_to_proto_match(m: DomainSearchMatch) -> ProtoSearchMatch {
    ProtoSearchMatch {
        field: domain_to_proto_field(m.field),
        text: m.text,
        grip_ids: m.grip_ids,
        score: m.score,
    }
}

/// Convert domain TocLevel to proto TocLevel.
fn domain_to_proto_level(level: DomainTocLevel) -> i32 {
    match level {
        DomainTocLevel::Year => ProtoTocLevel::Year as i32,
        DomainTocLevel::Month => ProtoTocLevel::Month as i32,
        DomainTocLevel::Week => ProtoTocLevel::Week as i32,
        DomainTocLevel::Day => ProtoTocLevel::Day as i32,
        DomainTocLevel::Segment => ProtoTocLevel::Segment as i32,
    }
}

/// Search within a single TOC node.
///
/// Per SEARCH-01: SearchNode searches node's fields for query terms.
pub async fn search_node(
    storage: Arc<Storage>,
    request: Request<SearchNodeRequest>,
) -> Result<Response<SearchNodeResponse>, Status> {
    let req = request.into_inner();
    debug!(
        "SearchNode request: node_id={}, query={}",
        req.node_id, req.query
    );

    if req.node_id.is_empty() {
        return Err(Status::invalid_argument("node_id is required"));
    }

    if req.query.trim().is_empty() {
        return Err(Status::invalid_argument("query is required"));
    }

    // Load the node
    let node = storage
        .get_toc_node(&req.node_id)
        .map_err(|e| Status::internal(format!("Storage error: {}", e)))?
        .ok_or_else(|| Status::not_found("Node not found"))?;

    // Convert proto fields to domain fields
    let fields: Vec<DomainSearchField> = req
        .fields
        .iter()
        .filter_map(|f| proto_to_domain_field(*f))
        .collect();

    // Execute search
    let matches = core_search_node(&node, &req.query, &fields);

    // Apply limit
    let limit = if req.limit > 0 {
        req.limit as usize
    } else {
        10
    };
    let matches: Vec<ProtoSearchMatch> = matches
        .into_iter()
        .take(limit)
        .map(domain_to_proto_match)
        .collect();

    Ok(Response::new(SearchNodeResponse {
        matched: !matches.is_empty(),
        matches,
        node_id: req.node_id,
        level: domain_to_proto_level(node.level),
    }))
}

/// Search across children of a parent node.
///
/// Per SEARCH-02: SearchChildren searches all children of parent.
pub async fn search_children(
    storage: Arc<Storage>,
    request: Request<SearchChildrenRequest>,
) -> Result<Response<SearchChildrenResponse>, Status> {
    let req = request.into_inner();
    debug!(
        "SearchChildren request: parent_id={}, query={}",
        req.parent_id, req.query
    );

    if req.query.trim().is_empty() {
        return Err(Status::invalid_argument("query is required"));
    }

    // Get children of parent (empty parent_id = root level years)
    let children = if req.parent_id.is_empty() {
        storage
            .get_toc_nodes_by_level(DomainTocLevel::Year, None, None)
            .map_err(|e| Status::internal(format!("Storage error: {}", e)))?
    } else {
        storage
            .get_child_nodes(&req.parent_id)
            .map_err(|e| Status::internal(format!("Storage error: {}", e)))?
    };

    // Convert proto fields to domain fields
    let fields: Vec<DomainSearchField> = req
        .fields
        .iter()
        .filter_map(|f| proto_to_domain_field(*f))
        .collect();

    // Search each child and collect results
    let mut results: Vec<ProtoSearchNodeResult> = Vec::new();
    for child in children {
        let matches = core_search_node(&child, &req.query, &fields);
        if !matches.is_empty() {
            // Calculate aggregate score (average of match scores)
            let relevance = matches.iter().map(|m| m.score).sum::<f32>() / matches.len() as f32;

            results.push(ProtoSearchNodeResult {
                node_id: child.node_id.clone(),
                title: child.title.clone(),
                level: domain_to_proto_level(child.level),
                matches: matches.into_iter().map(domain_to_proto_match).collect(),
                relevance_score: relevance,
            });
        }
    }

    // Sort by relevance score descending
    results.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(Ordering::Equal)
    });

    // Apply limit
    let limit = if req.limit > 0 {
        req.limit as usize
    } else {
        10
    };
    let has_more = results.len() > limit;
    let results: Vec<ProtoSearchNodeResult> = results.into_iter().take(limit).collect();

    Ok(Response::new(SearchChildrenResponse { results, has_more }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_search_node_not_found() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(SearchNodeRequest {
            node_id: "nonexistent".to_string(),
            query: "test".to_string(),
            fields: vec![],
            limit: 10,
            token_budget: 0,
        });
        let result = search_node(storage, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_search_node_empty_node_id() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(SearchNodeRequest {
            node_id: "".to_string(),
            query: "test".to_string(),
            fields: vec![],
            limit: 10,
            token_budget: 0,
        });
        let result = search_node(storage, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_search_node_empty_query() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(SearchNodeRequest {
            node_id: "toc:year:2026".to_string(),
            query: "".to_string(),
            fields: vec![],
            limit: 10,
            token_budget: 0,
        });
        let result = search_node(storage, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_search_node_whitespace_query() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(SearchNodeRequest {
            node_id: "toc:year:2026".to_string(),
            query: "   ".to_string(),
            fields: vec![],
            limit: 10,
            token_budget: 0,
        });
        let result = search_node(storage, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_search_children_empty_query() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(SearchChildrenRequest {
            parent_id: "".to_string(),
            query: "   ".to_string(),
            child_level: 0,
            fields: vec![],
            limit: 10,
            token_budget: 0,
        });
        let result = search_children(storage, request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_search_children_empty_results() {
        let (storage, _temp) = create_test_storage();
        let request = Request::new(SearchChildrenRequest {
            parent_id: "".to_string(),
            query: "jwt token".to_string(),
            child_level: 0,
            fields: vec![],
            limit: 10,
            token_budget: 0,
        });
        // Should succeed with empty results (no nodes in storage)
        let result = search_children(storage, request).await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert!(response.results.is_empty());
        assert!(!response.has_more);
    }

    #[test]
    fn test_proto_to_domain_field_title() {
        let result = proto_to_domain_field(ProtoSearchField::Title as i32);
        assert_eq!(result, Some(DomainSearchField::Title));
    }

    #[test]
    fn test_proto_to_domain_field_summary() {
        let result = proto_to_domain_field(ProtoSearchField::Summary as i32);
        assert_eq!(result, Some(DomainSearchField::Summary));
    }

    #[test]
    fn test_proto_to_domain_field_bullets() {
        let result = proto_to_domain_field(ProtoSearchField::Bullets as i32);
        assert_eq!(result, Some(DomainSearchField::Bullets));
    }

    #[test]
    fn test_proto_to_domain_field_keywords() {
        let result = proto_to_domain_field(ProtoSearchField::Keywords as i32);
        assert_eq!(result, Some(DomainSearchField::Keywords));
    }

    #[test]
    fn test_proto_to_domain_field_unspecified() {
        let result = proto_to_domain_field(ProtoSearchField::Unspecified as i32);
        assert_eq!(result, None);
    }

    #[test]
    fn test_proto_to_domain_field_invalid() {
        let result = proto_to_domain_field(999);
        assert_eq!(result, None);
    }

    #[test]
    fn test_domain_to_proto_field_roundtrip() {
        for domain in [
            DomainSearchField::Title,
            DomainSearchField::Summary,
            DomainSearchField::Bullets,
            DomainSearchField::Keywords,
        ] {
            let proto = domain_to_proto_field(domain);
            let back = proto_to_domain_field(proto);
            assert_eq!(back, Some(domain));
        }
    }

    #[test]
    fn test_domain_to_proto_level() {
        assert_eq!(
            domain_to_proto_level(DomainTocLevel::Year),
            ProtoTocLevel::Year as i32
        );
        assert_eq!(
            domain_to_proto_level(DomainTocLevel::Month),
            ProtoTocLevel::Month as i32
        );
        assert_eq!(
            domain_to_proto_level(DomainTocLevel::Week),
            ProtoTocLevel::Week as i32
        );
        assert_eq!(
            domain_to_proto_level(DomainTocLevel::Day),
            ProtoTocLevel::Day as i32
        );
        assert_eq!(
            domain_to_proto_level(DomainTocLevel::Segment),
            ProtoTocLevel::Segment as i32
        );
    }
}
