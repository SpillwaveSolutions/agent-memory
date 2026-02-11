//! Teleport search handler.
//!
//! Provides BM25 keyword search over TOC nodes and grips.

use std::sync::Arc;

use memory_search::{DocType, SearchOptions, TeleportSearcher};
use tonic::{Request, Response, Status};
use tracing::debug;

use crate::pb::{
    TeleportDocType, TeleportSearchRequest, TeleportSearchResponse, TeleportSearchResult,
};

/// Handle TeleportSearch RPC.
pub async fn handle_teleport_search(
    searcher: Arc<TeleportSearcher>,
    request: Request<TeleportSearchRequest>,
) -> Result<Response<TeleportSearchResponse>, Status> {
    let req = request.into_inner();

    debug!(query = %req.query, "Processing teleport search");

    // Build search options
    let mut options = SearchOptions::new();

    // Set limit (default 10, max 100)
    let limit = if req.limit > 0 {
        (req.limit as usize).min(100)
    } else {
        10
    };
    options = options.with_limit(limit);

    // Set doc type filter
    if req.doc_type == TeleportDocType::TocNode as i32 {
        options = options.with_doc_type(DocType::TocNode);
    } else if req.doc_type == TeleportDocType::Grip as i32 {
        options = options.with_doc_type(DocType::Grip);
    }

    // Execute search (blocking operation, use spawn_blocking)
    let query = req.query.clone();
    let searcher_clone = searcher.clone();
    let results = tokio::task::spawn_blocking(move || searcher_clone.search(&query, options))
        .await
        .map_err(|e| Status::internal(format!("Search task failed: {}", e)))?
        .map_err(|e| Status::internal(format!("Search failed: {}", e)))?;

    // Get total docs
    let total_docs = searcher.num_docs();

    // Map to proto results
    let proto_results: Vec<TeleportSearchResult> = results
        .into_iter()
        .map(|r| TeleportSearchResult {
            doc_id: r.doc_id,
            doc_type: match r.doc_type {
                DocType::TocNode => TeleportDocType::TocNode as i32,
                DocType::Grip => TeleportDocType::Grip as i32,
            },
            score: r.score,
            keywords: r.keywords,
            timestamp_ms: r.timestamp_ms,
            agent: r.agent,
        })
        .collect();

    Ok(Response::new(TeleportSearchResponse {
        results: proto_results,
        total_docs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer};
    use memory_types::{Grip, TocBullet, TocLevel, TocNode};
    use tempfile::TempDir;

    fn sample_toc_node(id: &str, title: &str, bullet: &str) -> TocNode {
        let mut node = TocNode::new(
            id.to_string(),
            TocLevel::Day,
            title.to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.bullets = vec![TocBullet::new(bullet)];
        node.keywords = vec!["test".to_string()];
        node
    }

    fn sample_grip(id: &str, excerpt: &str) -> Grip {
        Grip::new(
            id.to_string(),
            excerpt.to_string(),
            "event-001".to_string(),
            "event-002".to_string(),
            Utc::now(),
            "test".to_string(),
        )
    }

    fn setup_searcher() -> (TempDir, Arc<TeleportSearcher>) {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();

        // Index some test data
        let indexer = SearchIndexer::new(&index).unwrap();
        indexer
            .index_toc_node(&sample_toc_node(
                "node-1",
                "Rust Memory Safety",
                "Discussed borrow checker",
            ))
            .unwrap();
        indexer
            .index_grip(&sample_grip("grip-1", "User asked about memory allocation"))
            .unwrap();
        indexer.commit().unwrap();

        let searcher = Arc::new(TeleportSearcher::new(&index).unwrap());
        (temp_dir, searcher)
    }

    #[tokio::test]
    async fn test_handle_teleport_search_all_types() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "memory".to_string(),
            doc_type: TeleportDocType::Unspecified as i32,
            limit: 10,
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        // Should find both node and grip
        assert_eq!(resp.results.len(), 2);
        assert!(resp.total_docs >= 2);
    }

    #[tokio::test]
    async fn test_handle_teleport_search_toc_only() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "memory".to_string(),
            doc_type: TeleportDocType::TocNode as i32,
            limit: 10,
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        // Should find only the node
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].doc_type, TeleportDocType::TocNode as i32);
    }

    #[tokio::test]
    async fn test_handle_teleport_search_grip_only() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "memory".to_string(),
            doc_type: TeleportDocType::Grip as i32,
            limit: 10,
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        // Should find only the grip
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].doc_type, TeleportDocType::Grip as i32);
    }

    #[tokio::test]
    async fn test_handle_teleport_search_limit() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "memory".to_string(),
            doc_type: TeleportDocType::Unspecified as i32,
            limit: 1,
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        // Should respect limit
        assert_eq!(resp.results.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_teleport_search_empty_query() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "".to_string(),
            doc_type: TeleportDocType::Unspecified as i32,
            limit: 10,
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        // Empty query returns empty results
        assert!(resp.results.is_empty());
    }

    #[tokio::test]
    async fn test_handle_teleport_search_no_matches() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "nonexistentterm12345".to_string(),
            doc_type: TeleportDocType::Unspecified as i32,
            limit: 10,
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        assert!(resp.results.is_empty());
    }

    #[tokio::test]
    async fn test_handle_teleport_search_default_limit() {
        let (_temp_dir, searcher) = setup_searcher();

        let request = Request::new(TeleportSearchRequest {
            query: "memory".to_string(),
            doc_type: TeleportDocType::Unspecified as i32,
            limit: 0, // Should default to 10
            agent_filter: None,
        });

        let response = handle_teleport_search(searcher, request).await.unwrap();
        let resp = response.into_inner();

        // Should still return results (limit defaults to 10)
        assert!(!resp.results.is_empty());
    }
}
