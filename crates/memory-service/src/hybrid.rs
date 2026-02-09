//! HybridSearch RPC implementation.
//!
//! Combines BM25 and vector search using Reciprocal Rank Fusion (RRF).
//! RRF_score(doc) = sum(weight_i / (k + rank_i(doc)))
//! where k=60 is the standard constant.

use std::collections::HashMap;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, info};

use crate::pb::{
    HybridMode, HybridSearchRequest, HybridSearchResponse, VectorMatch, VectorTeleportRequest,
};
use crate::vector::VectorTeleportHandler;

/// Standard RRF constant (from original RRF paper)
const RRF_K: f32 = 60.0;

/// Handler for hybrid search operations.
pub struct HybridSearchHandler {
    vector_handler: Arc<VectorTeleportHandler>,
    // BM25 integration will be added when Phase 11 is complete
}

impl HybridSearchHandler {
    /// Create a new hybrid search handler.
    pub fn new(vector_handler: Arc<VectorTeleportHandler>) -> Self {
        Self { vector_handler }
    }

    /// Check if BM25 search is available.
    pub fn bm25_available(&self) -> bool {
        // TODO: Will be true when Phase 11 is integrated
        false
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
                        .fuse_rrf(query, top_k, bm25_weight, vector_weight, &req)
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
    async fn bm25_search(&self, _query: &str, _top_k: usize) -> Result<Vec<VectorMatch>, Status> {
        // TODO: Integrate with Phase 11 BM25 when complete
        Ok(vec![])
    }

    /// Fuse results using Reciprocal Rank Fusion.
    async fn fuse_rrf(
        &self,
        query: &str,
        top_k: usize,
        bm25_weight: f32,
        vector_weight: f32,
        req: &HybridSearchRequest,
    ) -> Result<Vec<VectorMatch>, Status> {
        // Fetch more results for fusion
        let fetch_k = top_k * 2;

        let vector_results = self.vector_search(query, fetch_k, req).await?;
        let bm25_results = self.bm25_search(query, fetch_k).await?;

        let mut rrf: HashMap<String, RrfEntry> = HashMap::new();

        // Accumulate vector RRF scores
        for (rank, m) in vector_results.into_iter().enumerate() {
            let score = vector_weight / (RRF_K + rank as f32 + 1.0);
            let entry = rrf
                .entry(m.doc_id.clone())
                .or_insert_with(|| RrfEntry::from(&m));
            entry.rrf_score += score;
        }

        // Accumulate BM25 RRF scores
        for (rank, m) in bm25_results.into_iter().enumerate() {
            let score = bm25_weight / (RRF_K + rank as f32 + 1.0);
            let entry = rrf
                .entry(m.doc_id.clone())
                .or_insert_with(|| RrfEntry::from(&m));
            entry.rrf_score += score;
        }

        // Sort by RRF score and truncate
        let mut entries: Vec<_> = rrf.into_values().collect();
        entries.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(top_k);

        // Convert to VectorMatch
        Ok(entries
            .into_iter()
            .map(|e| VectorMatch {
                doc_id: e.doc_id,
                doc_type: e.doc_type,
                score: e.rrf_score,
                text_preview: e.text_preview,
                timestamp_ms: e.timestamp_ms,
            })
            .collect())
    }
}

/// Entry for RRF accumulation.
struct RrfEntry {
    doc_id: String,
    doc_type: String,
    text_preview: String,
    timestamp_ms: i64,
    rrf_score: f32,
}

impl From<&VectorMatch> for RrfEntry {
    fn from(m: &VectorMatch) -> Self {
        Self {
            doc_id: m.doc_id.clone(),
            doc_type: m.doc_type.clone(),
            text_preview: m.text_preview.clone(),
            timestamp_ms: m.timestamp_ms,
            rrf_score: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_k_constant() {
        // Verify RRF_K is the standard value from the paper
        assert_eq!(RRF_K, 60.0);
    }

    #[test]
    fn test_rrf_entry_from_vector_match() {
        let m = VectorMatch {
            doc_id: "test-123".to_string(),
            doc_type: "toc_node".to_string(),
            score: 0.95,
            text_preview: "Test preview".to_string(),
            timestamp_ms: 1234567890,
        };

        let entry = RrfEntry::from(&m);
        assert_eq!(entry.doc_id, "test-123");
        assert_eq!(entry.doc_type, "toc_node");
        assert_eq!(entry.rrf_score, 0.0); // Should start at 0
    }
}
