//! HybridSearch RPC implementation.
//!
//! Combines BM25 and vector search using the workspace's canonical
//! weighted rank-fusion in `memory_orchestrator::fusion`.

use std::collections::HashMap;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, info};

use memory_orchestrator::fuse_weighted;
use memory_retrieval::{RetrievalLayer, SearchResult};
use memory_search::{SearchOptions, TeleportSearcher};

use crate::pb::{
    HybridMode, HybridSearchRequest, HybridSearchResponse, VectorMatch, VectorTeleportRequest,
};
use crate::vector::VectorTeleportHandler;

/// Standard rank-fusion damping constant (Cormack et al.).
const FUSION_K: f64 = 60.0;

/// Handler for hybrid search operations.
pub struct HybridSearchHandler {
    vector_handler: Arc<VectorTeleportHandler>,
    searcher: Option<Arc<TeleportSearcher>>,
}

impl HybridSearchHandler {
    /// Create a new hybrid search handler.
    pub fn new(
        vector_handler: Arc<VectorTeleportHandler>,
        searcher: Option<Arc<TeleportSearcher>>,
    ) -> Self {
        Self {
            vector_handler,
            searcher,
        }
    }

    /// Check if BM25 search is available.
    pub fn bm25_available(&self) -> bool {
        self.searcher.is_some()
    }

    /// Check if vector search is available.
    pub fn vector_available(&self) -> bool {
        self.vector_handler.is_available()
    }

    /// Handle HybridSearch RPC request.
    pub async fn hybrid_search(
        &self,
        request: Request<HybridSearchRequest>,
    ) -> Result<Response<HybridSearchResponse>, Status> {
        let req = request.into_inner();
        let query = &req.query;
        let top_k = if req.top_k > 0 {
            req.top_k as usize
        } else {
            10
        };
        let mode = HybridMode::try_from(req.mode).unwrap_or(HybridMode::Hybrid);
        let bm25_weight = if req.bm25_weight > 0.0 {
            req.bm25_weight
        } else {
            0.5
        };
        let vector_weight = if req.vector_weight > 0.0 {
            req.vector_weight
        } else {
            0.5
        };

        debug!(query = %query, mode = ?mode, "HybridSearch request");

        // Determine actual mode based on availability
        let (actual_mode, matches) = match mode {
            HybridMode::VectorOnly => (
                HybridMode::VectorOnly,
                self.vector_search(query, top_k, &req).await?,
            ),
            HybridMode::Bm25Only => (HybridMode::Bm25Only, self.bm25_search(query, top_k).await?),
            HybridMode::Hybrid | HybridMode::Unspecified => {
                if self.vector_available() && self.bm25_available() {
                    let fused = self
                        .fuse_lists(query, top_k, bm25_weight, vector_weight, &req)
                        .await?;
                    (HybridMode::Hybrid, fused)
                } else if self.vector_available() {
                    (
                        HybridMode::VectorOnly,
                        self.vector_search(query, top_k, &req).await?,
                    )
                } else if self.bm25_available() {
                    (HybridMode::Bm25Only, self.bm25_search(query, top_k).await?)
                } else {
                    (HybridMode::Unspecified, vec![])
                }
            }
        };

        info!(query = %query, mode = ?actual_mode, results = matches.len(), "HybridSearch complete");

        Ok(Response::new(HybridSearchResponse {
            matches,
            mode_used: actual_mode as i32,
            bm25_available: self.bm25_available(),
            vector_available: self.vector_available(),
        }))
    }

    /// Perform vector-only search.
    async fn vector_search(
        &self,
        query: &str,
        top_k: usize,
        req: &HybridSearchRequest,
    ) -> Result<Vec<VectorMatch>, Status> {
        let vector_req = VectorTeleportRequest {
            query: query.to_string(),
            top_k: top_k as i32,
            min_score: 0.0,
            time_filter: req.time_filter,
            target: req.target,
            agent_filter: req.agent_filter.clone(),
        };
        let response = self
            .vector_handler
            .vector_teleport(Request::new(vector_req))
            .await?;
        Ok(response.into_inner().matches)
    }

    /// Perform BM25-only search.
    async fn bm25_search(&self, query: &str, top_k: usize) -> Result<Vec<VectorMatch>, Status> {
        let Some(searcher) = &self.searcher else {
            return Ok(vec![]);
        };

        let results = searcher
            .search(query, SearchOptions::new().with_limit(top_k))
            .map_err(|e| Status::internal(format!("BM25 search error: {e}")))?;

        Ok(results
            .into_iter()
            .map(|r| VectorMatch {
                doc_id: r.doc_id,
                doc_type: r.doc_type.as_str().to_string(),
                score: r.score,
                text_preview: r.keywords.unwrap_or_default(),
                timestamp_ms: r.timestamp_ms.unwrap_or(0),
                agent: r.agent,
            })
            .collect())
    }

    /// Fuse BM25 + vector lists via the canonical weighted rank-fusion.
    async fn fuse_lists(
        &self,
        query: &str,
        top_k: usize,
        bm25_weight: f32,
        vector_weight: f32,
        req: &HybridSearchRequest,
    ) -> Result<Vec<VectorMatch>, Status> {
        let fetch_k = top_k * 2;

        let vector_results = self.vector_search(query, fetch_k, req).await?;
        let bm25_results = self.bm25_search(query, fetch_k).await?;

        let mut extras: HashMap<String, VectorMatch> = HashMap::new();
        let mut vector_list = Vec::new();
        for m in vector_results {
            extras.entry(m.doc_id.clone()).or_insert_with(|| m.clone());
            vector_list.push(match_to_search(m, RetrievalLayer::Vector));
        }
        let mut bm25_list = Vec::new();
        for m in bm25_results {
            extras.entry(m.doc_id.clone()).or_insert_with(|| m.clone());
            bm25_list.push(match_to_search(m, RetrievalLayer::BM25));
        }

        let fused = fuse_weighted(
            vec![
                (f64::from(vector_weight), vector_list),
                (f64::from(bm25_weight), bm25_list),
            ],
            FUSION_K,
        );

        Ok(fused
            .into_iter()
            .take(top_k)
            .map(|f| {
                if let Some(orig) = extras.get(&f.inner.doc_id) {
                    VectorMatch {
                        score: f.fusion_score as f32,
                        ..orig.clone()
                    }
                } else {
                    VectorMatch {
                        doc_id: f.inner.doc_id,
                        doc_type: f.inner.doc_type,
                        score: f.fusion_score as f32,
                        text_preview: f.inner.text_preview,
                        timestamp_ms: 0,
                        agent: None,
                    }
                }
            })
            .collect())
    }
}

fn match_to_search(m: VectorMatch, layer: RetrievalLayer) -> SearchResult {
    SearchResult {
        doc_id: m.doc_id,
        doc_type: m.doc_type,
        score: m.score,
        text_preview: m.text_preview,
        source_layer: layer,
        metadata: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_k_constant() {
        assert!((FUSION_K - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_match_to_search_preserves_fields() {
        let m = VectorMatch {
            doc_id: "test-123".to_string(),
            doc_type: "toc_node".to_string(),
            score: 0.95,
            text_preview: "Test preview".to_string(),
            timestamp_ms: 1234567890,
            agent: None,
        };

        let sr = match_to_search(m, RetrievalLayer::Vector);
        assert_eq!(sr.doc_id, "test-123");
        assert_eq!(sr.doc_type, "toc_node");
        assert!((sr.score - 0.95).abs() < f32::EPSILON);
        assert_eq!(sr.source_layer, RetrievalLayer::Vector);
    }
}
