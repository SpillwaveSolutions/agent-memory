//! Core types for the retrieval orchestrator.
//!
//! Defines configuration, ranked results, memory context, and reranking modes
//! used across all orchestrator submodules.

use serde::{Deserialize, Serialize};

/// Reranking strategy to apply after initial retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RerankMode {
    /// Score-based heuristic reranking (default, no LLM call).
    #[default]
    Heuristic,
    /// LLM-based reranking for higher quality (slower, costs tokens).
    Llm,
}

/// Configuration for the retrieval orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Number of final results to return.
    pub top_k: usize,
    /// Reranking strategy.
    pub rerank_mode: RerankMode,
    /// Whether to expand the query into multiple variants before search.
    pub expand_query: bool,
    /// Reciprocal Rank Fusion constant (higher = more weight to lower-ranked docs).
    pub rrf_k: f64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            rerank_mode: RerankMode::Heuristic,
            expand_query: false,
            rrf_k: 60.0,
        }
    }
}

/// A scored and ranked retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    /// Fused relevance score (0.0 - 1.0).
    pub score: f64,
    /// Document identifier.
    pub doc_id: String,
    /// Text content or preview.
    pub text: String,
    /// Which retrieval layer produced this result.
    pub source_layer: String,
    /// Confidence in the ranking (0.0 - 1.0).
    pub confidence: f64,
}

/// Assembled memory context ready for LLM consumption.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryContext {
    /// Natural-language summary of relevant memories.
    pub summary: String,
    /// Ranked retrieval results included in context.
    pub relevant_events: Vec<RankedResult>,
    /// Key entities mentioned across results.
    pub key_entities: Vec<String>,
    /// Open questions identified from conversation history.
    pub open_questions: Vec<String>,
    /// Wall-clock milliseconds spent on retrieval.
    pub retrieval_ms: u64,
    /// Estimated token count for the assembled context.
    pub tokens_estimated: usize,
    /// Overall confidence in the assembled context.
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rerank_mode_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.rerank_mode, RerankMode::Heuristic);
        assert_eq!(config.top_k, 10);
        assert!(!config.expand_query);
    }

    #[test]
    fn test_orchestrator_config_rrf_k() {
        let config = OrchestratorConfig::default();
        assert!((config.rrf_k - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ranked_result_ordering() {
        let mut results = [
            RankedResult {
                score: 0.5,
                doc_id: "a".into(),
                text: "a".into(),
                source_layer: "bm25".into(),
                confidence: 0.5,
            },
            RankedResult {
                score: 0.9,
                doc_id: "b".into(),
                text: "b".into(),
                source_layer: "vector".into(),
                confidence: 0.9,
            },
        ];
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        assert_eq!(results[0].doc_id, "b");
    }

    #[test]
    fn test_memory_context_default() {
        let ctx = MemoryContext::default();
        assert!(ctx.relevant_events.is_empty());
        assert!(ctx.summary.is_empty());
        assert!(ctx.key_entities.is_empty());
        assert_eq!(ctx.retrieval_ms, 0);
        assert_eq!(ctx.tokens_estimated, 0);
    }
}
