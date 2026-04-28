//! Top-level retrieval orchestrator.
//!
//! Wires the complete retrieval pipeline: query expansion, fan-out across
//! multiple indexes, RRF fusion, reranking, and context assembly.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use memory_retrieval::{
    CapabilityTier, ExecutionMode, FallbackChain, LayerExecutor, RetrievalExecutor, RetrievalLayer,
    SearchResult, StopConditions,
};

use crate::context_builder::ContextBuilder;
use crate::expand::expand_query;
use crate::fusion::rrf_fuse;
use crate::rerank::{HeuristicReranker, Reranker};
use crate::types::{MemoryContext, OrchestratorConfig};

/// Retrieval orchestrator that coordinates query expansion, multi-index
/// search, fusion, reranking, and context assembly.
pub struct MemoryOrchestrator<E: LayerExecutor> {
    executor: Arc<E>,
    config: OrchestratorConfig,
    reranker: Box<dyn Reranker>,
}

impl<E: LayerExecutor + Send + Sync + 'static> MemoryOrchestrator<E> {
    /// Create a new orchestrator with the default `HeuristicReranker`.
    pub fn new(executor: Arc<E>, config: OrchestratorConfig) -> Self {
        Self {
            executor,
            config,
            reranker: Box::new(HeuristicReranker),
        }
    }

    /// Create a new orchestrator with an injected reranker.
    ///
    /// Use this constructor in tests to supply a `MockLlmReranker` or any
    /// custom `Box<dyn Reranker>`.
    pub fn with_reranker(
        executor: Arc<E>,
        config: OrchestratorConfig,
        reranker: Box<dyn Reranker>,
    ) -> Self {
        Self {
            executor,
            config,
            reranker,
        }
    }

    /// Execute the full retrieval pipeline and return assembled context.
    ///
    /// Pipeline stages:
    /// 1. Query expansion (if `expand_query` is enabled)
    /// 2. Fan-out: each query variant against each layer
    /// 3. RRF fusion across all result lists
    /// 4. Reranking (heuristic or injected)
    /// 5. Context assembly
    pub async fn query(&self, query: &str) -> Result<MemoryContext> {
        let start = Instant::now();

        // 1. Query expansion
        let queries = if self.config.expand_query {
            expand_query(query)
        } else {
            vec![query.to_string()]
        };

        // 2. Fan-out: each query against each layer
        let layers = [
            RetrievalLayer::Topics,
            RetrievalLayer::Vector,
            RetrievalLayer::BM25,
            RetrievalLayer::Agentic,
        ];

        let re = RetrievalExecutor::new(self.executor.clone());
        let mut all_lists: Vec<Vec<SearchResult>> = Vec::new();

        for q in &queries {
            for &layer in &layers {
                let chain = FallbackChain {
                    layers: vec![layer],
                    merge_results: false,
                    max_layers: 1,
                };
                let conds = StopConditions::default();
                let result = re
                    .execute(
                        q,
                        chain,
                        &conds,
                        ExecutionMode::Sequential,
                        CapabilityTier::Full,
                    )
                    .await;
                if result.has_results() {
                    all_lists.push(result.results);
                }
                // fail-open: skip empty/failed layers silently
            }
        }

        // 3. RRF fusion
        let fused = rrf_fuse(all_lists, self.config.rrf_k);

        // 4. Reranking — always use self.reranker (injected or default)
        let reranked = self.reranker.rerank(query, fused).await?;

        // 5. Build context
        let mut ctx = ContextBuilder::build(query, reranked);
        ctx.retrieval_ms = start.elapsed().as_millis() as u64;

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use memory_retrieval::MockLayerExecutor;

    use crate::fusion::FusedResult;
    use crate::rerank::RerankedResult;

    fn mock_result(id: &str, score: f32, layer: RetrievalLayer) -> SearchResult {
        SearchResult {
            doc_id: id.to_string(),
            doc_type: "toc_node".to_string(),
            score,
            text_preview: format!("preview for {id}"),
            source_layer: layer,
            metadata: Default::default(),
        }
    }

    /// Mock LLM reranker that reverses the RRF order.
    /// Used to prove that injected reranker's reorder is honored.
    struct MockLlmReranker;

    #[async_trait]
    impl Reranker for MockLlmReranker {
        async fn rerank(
            &self,
            _query: &str,
            results: Vec<FusedResult>,
        ) -> anyhow::Result<Vec<RerankedResult>> {
            let mut out: Vec<RerankedResult> = results
                .into_iter()
                .map(|r| RerankedResult {
                    doc_id: r.inner.doc_id.clone(),
                    score: r.rrf_score,
                    text: r.inner.text_preview.clone(),
                    source_layer: format!("{:?}", r.inner.source_layer),
                })
                .collect();
            out.reverse();
            Ok(out)
        }
    }

    #[tokio::test]
    async fn test_orchestrator_returns_fused_results() {
        // doc-1 appears in two lists (BM25 and Vector) -> should rank highest via RRF consensus
        let executor = MockLayerExecutor::default()
            .with_results(
                RetrievalLayer::BM25,
                vec![mock_result("doc-1", 0.9, RetrievalLayer::BM25)],
            )
            .with_results(
                RetrievalLayer::Vector,
                vec![mock_result("doc-1", 0.8, RetrievalLayer::Vector)],
            )
            .with_results(
                RetrievalLayer::Topics,
                vec![mock_result("doc-2", 0.7, RetrievalLayer::Topics)],
            )
            .with_results(
                RetrievalLayer::Agentic,
                vec![mock_result("doc-3", 0.6, RetrievalLayer::Agentic)],
            );

        let config = OrchestratorConfig::default();
        let orch = MemoryOrchestrator::new(Arc::new(executor), config);

        let ctx = orch.query("test query").await.unwrap();
        assert!(!ctx.relevant_events.is_empty());
        // doc-1 appears in two lists, RRF consensus should place it first
        assert_eq!(ctx.relevant_events[0].doc_id, "doc-1");
    }

    #[tokio::test]
    async fn test_orchestrator_fail_open_when_one_layer_fails() {
        let executor = MockLayerExecutor::default()
            .with_failure(RetrievalLayer::BM25)
            .with_results(
                RetrievalLayer::Vector,
                vec![mock_result("doc-a", 0.8, RetrievalLayer::Vector)],
            );

        let config = OrchestratorConfig::default();
        let orch = MemoryOrchestrator::new(Arc::new(executor), config);

        let result = orch.query("test query").await;
        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert!(!ctx.relevant_events.is_empty());
    }

    #[tokio::test]
    async fn test_llm_rerank_reorders_results() {
        // RRF natural order: doc-alpha first (higher score), doc-beta second
        let executor = MockLayerExecutor::default().with_results(
            RetrievalLayer::BM25,
            vec![
                mock_result("doc-alpha", 0.9, RetrievalLayer::BM25),
                mock_result("doc-beta", 0.5, RetrievalLayer::BM25),
            ],
        );

        let config = OrchestratorConfig::default();
        let orch = MemoryOrchestrator::with_reranker(
            Arc::new(executor),
            config,
            Box::new(MockLlmReranker),
        );

        let ctx = orch.query("test query").await.unwrap();
        // MockLlmReranker reverses order: doc-beta should now be first
        assert_eq!(ctx.relevant_events[0].doc_id, "doc-beta");
        assert_eq!(ctx.relevant_events[1].doc_id, "doc-alpha");
    }

    #[tokio::test]
    async fn test_orchestrator_query_expansion() {
        let executor = MockLayerExecutor::default().with_results(
            RetrievalLayer::BM25,
            vec![mock_result("doc-x", 0.7, RetrievalLayer::BM25)],
        );

        let config = OrchestratorConfig {
            expand_query: true,
            ..OrchestratorConfig::default()
        };
        let orch = MemoryOrchestrator::new(Arc::new(executor), config);

        let result = orch.query("What happened with auth").await;
        assert!(result.is_ok());
    }
}
