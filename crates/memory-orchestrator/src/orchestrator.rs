//! Top-level retrieval orchestrator.
//!
//! Wires the complete retrieval pipeline: query expansion, fan-out across
//! multiple indexes, RRF fusion, reranking, and context assembly.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use memory_retrieval::{
    ExecutionMode, LayerExecutor, RetrievalLayer, SearchResult, StopConditions,
};

use crate::context_builder::ContextBuilder;
use crate::expand::expand_query;
use crate::fusion::fuse;
use crate::rerank::{HeuristicReranker, Reranker};
use crate::types::{MemoryContext, OrchestratorConfig};

/// Output of the ranked retrieval pipeline (before context assembly).
#[derive(Debug, Clone)]
pub struct OrchestratorOutput {
    /// Fused + reranked hits, original SearchResult preserved.
    pub results: Vec<memory_retrieval::SearchResult>,
    /// Fusion stage name for explainability (always `"rank_fusion"`).
    pub fusion_stage: &'static str,
    /// Reranker that ran (`"heuristic"` or `"llm"`).
    pub rerank_mode: String,
    /// Layers that were supported and invoked (empty hits still count).
    pub layers_attempted: Vec<RetrievalLayer>,
    /// Wall-clock milliseconds for the pipeline.
    pub retrieval_ms: u64,
}

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
        let top_k = config.top_k;
        Self {
            executor,
            config,
            reranker: Box::new(HeuristicReranker::new(top_k)),
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
        let output = self.query_ranked(query).await?;
        let reranked = output
            .results
            .into_iter()
            .map(|r| crate::rerank::RerankedResult {
                doc_id: r.doc_id.clone(),
                score: f64::from(r.score),
                text: r.text_preview.clone(),
                source_layer: format!("{:?}", r.source_layer),
                inner: r,
            })
            .collect();
        let mut ctx = ContextBuilder::build(query, reranked);
        ctx.retrieval_ms = output.retrieval_ms;
        Ok(ctx)
    }

    /// Execute expand → fan-out → RRF → rerank and return ranked hits.
    ///
    /// Fan-out runs supported layers concurrently (independent lists for
    /// rank fusion). Use [`Self::query_ranked_with`] to honor client stop
    /// conditions and execution mode.
    pub async fn query_ranked(&self, query: &str) -> Result<OrchestratorOutput> {
        self.query_ranked_with(query, &StopConditions::default(), ExecutionMode::Parallel)
            .await
    }

    /// Like [`Self::query_ranked`] with caller-supplied stop conditions and mode.
    pub async fn query_ranked_with(
        &self,
        query: &str,
        conditions: &StopConditions,
        mode: ExecutionMode,
    ) -> Result<OrchestratorOutput> {
        let start = Instant::now();
        let timeout = conditions.timeout();
        let limit = self.config.top_k.min(conditions.max_nodes as usize).max(1);

        let queries = if self.config.expand_query {
            expand_query(query)
        } else {
            vec![query.to_string()]
        };

        let all_layers = [
            RetrievalLayer::Topics,
            RetrievalLayer::Vector,
            RetrievalLayer::BM25,
            RetrievalLayer::Agentic,
        ];
        let layers: Vec<RetrievalLayer> = all_layers
            .into_iter()
            .filter(|&layer| self.executor.supports(layer))
            .collect();

        let mut all_lists: Vec<Vec<SearchResult>> = Vec::new();
        let mut layers_attempted: Vec<RetrievalLayer> = Vec::new();

        let parallel = !matches!(mode, ExecutionMode::Sequential);

        if parallel {
            let mut tasks = Vec::new();
            for q in &queries {
                for &layer in &layers {
                    let exec = Arc::clone(&self.executor);
                    let q = q.clone();
                    tasks.push(async move {
                        match exec.execute(&q, layer, limit).await {
                            Ok(results) => (layer, results),
                            Err(e) => {
                                tracing::debug!(layer = ?layer, error = %e, "layer failed; fail-open");
                                (layer, Vec::new())
                            }
                        }
                    });
                }
            }
            match tokio::time::timeout(timeout, futures::future::join_all(tasks)).await {
                Ok(pairs) => {
                    for (layer, results) in pairs {
                        if !layers_attempted.contains(&layer) {
                            layers_attempted.push(layer);
                        }
                        if !results.is_empty() {
                            all_lists.push(results);
                        }
                    }
                }
                Err(_) => {
                    tracing::warn!("orchestrator fan-out timed out");
                }
            }
        } else {
            'fanout: for q in &queries {
                for &layer in &layers {
                    if start.elapsed() >= timeout {
                        tracing::warn!("orchestrator sequential fan-out hit stop timeout");
                        break 'fanout;
                    }
                    if !layers_attempted.contains(&layer) {
                        layers_attempted.push(layer);
                    }
                    match self.executor.execute(q, layer, limit).await {
                        Ok(results) if !results.is_empty() => all_lists.push(results),
                        Ok(_) => {}
                        Err(e) => {
                            tracing::debug!(layer = ?layer, error = %e, "layer failed; fail-open");
                        }
                    }
                }
            }
        }

        let fused = fuse(all_lists, self.config.fusion_k);
        let reranked = self.reranker.rerank(query, fused).await?;
        let rerank_mode = self.reranker.mode_name().to_string();

        let results: Vec<SearchResult> = reranked
            .into_iter()
            .map(|r| {
                let mut inner = r.inner;
                inner.score = r.score as f32;
                inner
            })
            .collect();

        Ok(OrchestratorOutput {
            results,
            fusion_stage: "rank_fusion",
            rerank_mode,
            layers_attempted,
            retrieval_ms: start.elapsed().as_millis() as u64,
        })
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
                    score: r.fusion_score,
                    text: r.inner.text_preview.clone(),
                    source_layer: format!("{:?}", r.inner.source_layer),
                    inner: r.inner,
                })
                .collect();
            out.reverse();
            Ok(out)
        }

        fn mode_name(&self) -> &'static str {
            "llm"
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

    #[tokio::test]
    async fn test_query_ranked_names_fusion_stage() {
        let executor = MockLayerExecutor::default().with_results(
            RetrievalLayer::BM25,
            vec![mock_result("doc-x", 0.7, RetrievalLayer::BM25)],
        );
        let config = OrchestratorConfig::default();
        let orch = MemoryOrchestrator::new(Arc::new(executor), config);
        let output = orch.query_ranked("test").await.unwrap();
        assert_eq!(output.fusion_stage, "rank_fusion");
        assert_eq!(output.rerank_mode, "heuristic");
        assert!(!output.results.is_empty());
        assert!(output.layers_attempted.contains(&RetrievalLayer::BM25));
    }

    #[tokio::test]
    async fn test_layers_attempted_omits_unconfigured_layers() {
        let executor = MockLayerExecutor::default().with_results(
            RetrievalLayer::BM25,
            vec![mock_result("doc-x", 0.7, RetrievalLayer::BM25)],
        );
        let orch = MemoryOrchestrator::new(Arc::new(executor), OrchestratorConfig::default());
        let output = orch.query_ranked("test").await.unwrap();
        assert_eq!(output.layers_attempted, vec![RetrievalLayer::BM25]);
    }

    #[tokio::test]
    async fn test_parallel_fan_out_runs_layers_concurrently() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        struct CountingExecutor {
            in_flight: AtomicUsize,
            max_in_flight: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl LayerExecutor for CountingExecutor {
            async fn execute(
                &self,
                _query: &str,
                layer: RetrievalLayer,
                _limit: usize,
            ) -> Result<Vec<SearchResult>, String> {
                let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                self.max_in_flight.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(40)).await;
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                Ok(vec![mock_result("doc", 0.5, layer)])
            }

            fn supports(&self, layer: RetrievalLayer) -> bool {
                matches!(
                    layer,
                    RetrievalLayer::BM25 | RetrievalLayer::Vector | RetrievalLayer::Topics
                )
            }
        }

        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let executor = CountingExecutor {
            in_flight: AtomicUsize::new(0),
            max_in_flight: Arc::clone(&max_in_flight),
        };
        let orch = MemoryOrchestrator::new(Arc::new(executor), OrchestratorConfig::default());
        let output = orch
            .query_ranked_with("test", &StopConditions::default(), ExecutionMode::Parallel)
            .await
            .unwrap();
        assert!(
            output.layers_attempted.len() >= 3,
            "expected BM25+Vector+Topics, got {:?}",
            output.layers_attempted
        );
        assert!(
            max_in_flight.load(Ordering::SeqCst) >= 2,
            "parallel fan-out must overlap at least two layers (max in-flight {})",
            max_in_flight.load(Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn test_sequential_timeout_stops_remaining_layers() {
        use std::time::Duration;

        let executor = MockLayerExecutor::default()
            .with_results(
                RetrievalLayer::BM25,
                vec![mock_result("doc-a", 0.9, RetrievalLayer::BM25)],
            )
            .with_delay(RetrievalLayer::BM25, Duration::from_millis(80))
            .with_results(
                RetrievalLayer::Vector,
                vec![mock_result("doc-b", 0.8, RetrievalLayer::Vector)],
            )
            .with_delay(RetrievalLayer::Vector, Duration::from_millis(80));

        let orch = MemoryOrchestrator::new(Arc::new(executor), OrchestratorConfig::default());
        let output = orch
            .query_ranked_with(
                "test",
                &StopConditions::with_timeout(Duration::from_millis(30)),
                ExecutionMode::Sequential,
            )
            .await
            .unwrap();
        // Layer order is Topics, Vector, BM25, Agentic. Vector is first supported
        // layer; its delay trips the timeout so BM25 must not run.
        assert_eq!(output.layers_attempted, vec![RetrievalLayer::Vector]);
        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].doc_id, "doc-b");
    }

    #[tokio::test]
    async fn test_mode_override_sequential_is_honored() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        struct CountingExecutor {
            in_flight: AtomicUsize,
            max_in_flight: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl LayerExecutor for CountingExecutor {
            async fn execute(
                &self,
                _query: &str,
                layer: RetrievalLayer,
                _limit: usize,
            ) -> Result<Vec<SearchResult>, String> {
                let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                self.max_in_flight.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(25)).await;
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                Ok(vec![mock_result("doc", 0.5, layer)])
            }

            fn supports(&self, layer: RetrievalLayer) -> bool {
                matches!(layer, RetrievalLayer::BM25 | RetrievalLayer::Vector)
            }
        }

        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let executor = CountingExecutor {
            in_flight: AtomicUsize::new(0),
            max_in_flight: Arc::clone(&max_in_flight),
        };
        let orch = MemoryOrchestrator::new(Arc::new(executor), OrchestratorConfig::default());
        let output = orch
            .query_ranked_with(
                "test",
                &StopConditions::default(),
                ExecutionMode::Sequential,
            )
            .await
            .unwrap();
        assert_eq!(output.layers_attempted.len(), 2);
        assert_eq!(
            max_in_flight.load(Ordering::SeqCst),
            1,
            "sequential mode must never overlap layer executions"
        );
    }
}
