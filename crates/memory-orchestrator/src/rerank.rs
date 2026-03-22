//! Result reranking (heuristic and LLM-based).
//!
//! Provides a `Reranker` trait with two implementations:
//! - `HeuristicReranker`: score-based sorting and top-K trimming (default).
//! - `CrossEncoderReranker`: stub that falls back to heuristic reranking
//!   (extension point for future LLM-based reranking).

use anyhow::Result;
use async_trait::async_trait;

use crate::fusion::FusedResult;

/// A reranked result ready for context assembly.
#[derive(Debug, Clone)]
pub struct RerankedResult {
    /// Document identifier.
    pub doc_id: String,
    /// Final relevance score after reranking (0.0 - 1.0).
    pub score: f64,
    /// Text content or preview.
    pub text: String,
    /// Which retrieval layer produced this result (stringified).
    pub source_layer: String,
}

/// Trait for result reranking strategies.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank fused results, returning a sorted and potentially trimmed list.
    async fn rerank(&self, query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>>;
}

/// Default reranker: sorts by RRF score descending and trims to top 10.
#[derive(Debug, Default)]
pub struct HeuristicReranker;

impl HeuristicReranker {
    /// Maximum number of results to retain after reranking.
    const MAX_RESULTS: usize = 10;

    fn rerank_sync(&self, results: Vec<FusedResult>) -> Vec<RerankedResult> {
        let mut sorted = results;
        sorted.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
            .into_iter()
            .take(Self::MAX_RESULTS)
            .map(|r| RerankedResult {
                doc_id: r.inner.doc_id,
                score: r.rrf_score,
                text: r.inner.text_preview,
                source_layer: format!("{:?}", r.inner.source_layer),
            })
            .collect()
    }
}

#[async_trait]
impl Reranker for HeuristicReranker {
    async fn rerank(&self, _query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>> {
        Ok(self.rerank_sync(results))
    }
}

/// Stub cross-encoder reranker. Falls back to heuristic reranking.
///
/// This is the extension point (ORCH-05) for future LLM-based reranking.
/// When implemented, it will call an LLM to score query-document relevance
/// before sorting.
#[derive(Debug, Default)]
pub struct CrossEncoderReranker {
    fallback: HeuristicReranker,
}

#[async_trait]
impl Reranker for CrossEncoderReranker {
    async fn rerank(&self, query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>> {
        tracing::warn!(
            "CrossEncoderReranker not yet implemented, falling back to heuristic reranking"
        );
        self.fallback.rerank(query, results).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_retrieval::{RetrievalLayer, SearchResult};

    fn make_fused(id: &str, rrf_score: f64) -> FusedResult {
        FusedResult {
            rrf_score,
            inner: SearchResult {
                doc_id: id.to_string(),
                doc_type: "toc_node".to_string(),
                score: rrf_score as f32,
                text_preview: format!("text for {id}"),
                source_layer: RetrievalLayer::BM25,
                metadata: Default::default(),
            },
        }
    }

    #[tokio::test]
    async fn test_heuristic_preserves_order_and_trims() {
        let mut results: Vec<FusedResult> = (0..20)
            .map(|i| make_fused(&format!("doc-{i}"), 1.0 - i as f64 * 0.01))
            .collect();
        // Shuffle to verify sorting works
        results.reverse();

        let reranker = HeuristicReranker;
        let reranked = reranker.rerank("test query", results).await.unwrap();

        assert_eq!(reranked.len(), 10, "should trim to top 10");
        assert_eq!(reranked[0].doc_id, "doc-0", "highest score should be first");
        assert!(
            reranked[0].score > reranked[9].score,
            "first should score higher than last"
        );
    }

    #[tokio::test]
    async fn test_cross_encoder_falls_back_to_heuristic() {
        let results = vec![make_fused("a", 0.9), make_fused("b", 0.5)];

        let reranker = CrossEncoderReranker::default();
        let reranked = reranker.rerank("test query", results).await.unwrap();

        // Should not panic and should produce results
        assert_eq!(reranked.len(), 2);
        assert_eq!(reranked[0].doc_id, "a");
        assert_eq!(reranked[1].doc_id, "b");
    }
}
