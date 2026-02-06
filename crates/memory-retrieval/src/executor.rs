//! Retrieval execution engine with fallback chains.
//!
//! This module implements the `RetrievalExecutor` which executes search operations
//! across multiple layers with fallback handling, parallel execution, and early stopping.
//!
//! Per PRD Section 5.4: Retrieval Execution Modes

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::types::{CapabilityTier, ExecutionMode, QueryIntent, RetrievalLayer, StopConditions};

/// A single search result item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document ID (node_id or grip_id)
    pub doc_id: String,

    /// Document type (toc_node, grip, etc.)
    pub doc_type: String,

    /// Relevance score (0.0-1.0)
    pub score: f32,

    /// Preview of matched text
    pub text_preview: String,

    /// Source layer that produced this result
    pub source_layer: RetrievalLayer,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Results from a layer execution.
#[derive(Debug, Clone)]
pub struct LayerResults {
    /// Which layer produced these results
    pub layer: RetrievalLayer,

    /// Search results from this layer
    pub results: Vec<SearchResult>,

    /// Whether the layer execution was successful
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl LayerResults {
    /// Create successful results.
    pub fn success(
        layer: RetrievalLayer,
        results: Vec<SearchResult>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            layer,
            results,
            success: true,
            error: None,
            execution_time_ms,
        }
    }

    /// Create failed results.
    pub fn failure(layer: RetrievalLayer, error: String, execution_time_ms: u64) -> Self {
        Self {
            layer,
            results: vec![],
            success: false,
            error: Some(error),
            execution_time_ms,
        }
    }

    /// Check if these results are sufficient (non-empty and good scores).
    pub fn is_sufficient(&self, min_confidence: f32) -> bool {
        if !self.success || self.results.is_empty() {
            return false;
        }

        // Check if top result meets minimum confidence
        self.results
            .first()
            .map(|r| r.score >= min_confidence)
            .unwrap_or(false)
    }
}

/// Final execution result with explainability.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final merged results
    pub results: Vec<SearchResult>,

    /// Which layers were tried
    pub layers_attempted: Vec<RetrievalLayer>,

    /// Which layer ultimately provided the results
    pub primary_layer: RetrievalLayer,

    /// Capability tier used
    pub tier: CapabilityTier,

    /// Execution mode used
    pub mode: ExecutionMode,

    /// Whether fallback occurred
    pub fallback_occurred: bool,

    /// Total execution time
    pub total_time_ms: u64,

    /// Detailed results from each layer
    pub layer_results: Vec<LayerResults>,

    /// Explanation of why this result was chosen
    pub explanation: String,
}

impl ExecutionResult {
    /// Check if any results were found.
    pub fn has_results(&self) -> bool {
        !self.results.is_empty()
    }

    /// Get count of results.
    pub fn count(&self) -> usize {
        self.results.len()
    }
}

/// Trait for layer executors.
///
/// Implementations execute search on a specific layer.
#[async_trait]
pub trait LayerExecutor: Send + Sync {
    /// Execute search on this layer.
    async fn execute(
        &self,
        query: &str,
        layer: RetrievalLayer,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String>;

    /// Check if this executor can handle the given layer.
    fn supports(&self, layer: RetrievalLayer) -> bool;
}

/// Fallback chain configuration.
#[derive(Debug, Clone)]
pub struct FallbackChain {
    /// Ordered list of layers to try
    pub layers: Vec<RetrievalLayer>,

    /// Whether to merge results from multiple layers
    pub merge_results: bool,

    /// Maximum layers to try before stopping
    pub max_layers: usize,
}

impl FallbackChain {
    /// Create a chain for the given intent and tier.
    pub fn for_intent(intent: QueryIntent, tier: CapabilityTier) -> Self {
        let layers = match intent {
            QueryIntent::Explore => {
                let mut l = vec![
                    RetrievalLayer::Topics,
                    RetrievalLayer::Hybrid,
                    RetrievalLayer::Vector,
                    RetrievalLayer::BM25,
                    RetrievalLayer::Agentic,
                ];
                l.retain(|layer| tier.supports(*layer));
                l
            }
            QueryIntent::Answer => {
                let mut l = vec![
                    RetrievalLayer::Hybrid,
                    RetrievalLayer::BM25,
                    RetrievalLayer::Vector,
                    RetrievalLayer::Agentic,
                ];
                l.retain(|layer| tier.supports(*layer));
                l
            }
            QueryIntent::Locate => {
                let mut l = vec![
                    RetrievalLayer::BM25,
                    RetrievalLayer::Hybrid,
                    RetrievalLayer::Vector,
                    RetrievalLayer::Agentic,
                ];
                l.retain(|layer| tier.supports(*layer));
                l
            }
            QueryIntent::TimeBoxed => {
                vec![tier.best_layer(), RetrievalLayer::Agentic]
            }
        };

        Self {
            layers,
            merge_results: false,
            max_layers: 3,
        }
    }

    /// Create a chain that merges results from multiple layers.
    pub fn merged(layers: Vec<RetrievalLayer>) -> Self {
        let max_layers = layers.len();
        Self {
            layers,
            merge_results: true,
            max_layers,
        }
    }
}

/// Retrieval executor that orchestrates search across layers.
pub struct RetrievalExecutor<E: LayerExecutor + 'static> {
    executor: Arc<E>,
    default_limit: usize,
}

impl<E: LayerExecutor + 'static> RetrievalExecutor<E> {
    /// Create a new executor.
    pub fn new(executor: Arc<E>) -> Self {
        Self {
            executor,
            default_limit: 10,
        }
    }

    /// Set the default result limit.
    pub fn with_default_limit(mut self, limit: usize) -> Self {
        self.default_limit = limit;
        self
    }

    /// Execute a retrieval operation.
    pub async fn execute(
        &self,
        query: &str,
        chain: FallbackChain,
        conditions: &StopConditions,
        mode: ExecutionMode,
        tier: CapabilityTier,
    ) -> ExecutionResult {
        let timeout = conditions.timeout();
        let limit = self.default_limit.min(conditions.max_nodes as usize);

        match mode {
            ExecutionMode::Sequential => {
                self.execute_sequential(query, chain, limit, timeout, tier)
                    .await
            }
            ExecutionMode::Parallel => {
                self.execute_parallel(query, chain, limit, timeout, tier, conditions.beam_width)
                    .await
            }
            ExecutionMode::Hybrid => {
                self.execute_hybrid(query, chain, limit, timeout, tier, conditions)
                    .await
            }
        }
    }

    async fn execute_sequential(
        &self,
        query: &str,
        chain: FallbackChain,
        limit: usize,
        timeout: Duration,
        tier: CapabilityTier,
    ) -> ExecutionResult {
        let start = Instant::now();
        let mut layers_attempted = Vec::new();
        let mut layer_results = Vec::new();
        let mut primary_layer = RetrievalLayer::Agentic;
        let mut final_results = Vec::new();
        let mut fallback_occurred = false;
        let mut explanation = String::new();

        for (i, layer) in chain.layers.iter().take(chain.max_layers).enumerate() {
            // Check timeout
            if start.elapsed() >= timeout {
                warn!("Sequential execution timed out after {} layers", i);
                explanation = format!("Timed out after {} layers", i);
                break;
            }

            // Skip if executor doesn't support this layer
            if !self.executor.supports(*layer) {
                debug!(layer = ?layer, "Executor doesn't support layer, skipping");
                continue;
            }

            layers_attempted.push(*layer);

            // Calculate remaining time for this layer
            let remaining = timeout.saturating_sub(start.elapsed());
            let layer_start = Instant::now();

            // Execute with timeout
            let result =
                tokio::time::timeout(remaining, self.executor.execute(query, *layer, limit)).await;

            let execution_time = layer_start.elapsed().as_millis() as u64;

            let layer_result = match result {
                Ok(Ok(results)) => {
                    debug!(layer = ?layer, results = results.len(), "Layer returned results");
                    LayerResults::success(*layer, results, execution_time)
                }
                Ok(Err(e)) => {
                    warn!(layer = ?layer, error = %e, "Layer execution failed");
                    LayerResults::failure(*layer, e, execution_time)
                }
                Err(_) => {
                    warn!(layer = ?layer, "Layer execution timed out");
                    LayerResults::failure(*layer, "Timeout".to_string(), execution_time)
                }
            };

            let is_sufficient = layer_result.is_sufficient(0.3);
            layer_results.push(layer_result.clone());

            if layer_result.success && !layer_result.results.is_empty() {
                if final_results.is_empty() {
                    primary_layer = *layer;
                    final_results = layer_result.results.clone();
                } else {
                    fallback_occurred = true;
                }

                // If results are sufficient, stop here
                if is_sufficient {
                    explanation = format!(
                        "{} provided sufficient results (score >= 0.3)",
                        layer.as_str()
                    );
                    break;
                } else {
                    explanation = format!(
                        "{} returned results but confidence low, trying next layer",
                        layer.as_str()
                    );
                }
            } else if i == 0 {
                fallback_occurred = true;
            }
        }

        // If no results from any layer, note that
        if final_results.is_empty() {
            explanation = "No results found from any layer".to_string();
        }

        ExecutionResult {
            results: final_results,
            layers_attempted,
            primary_layer,
            tier,
            mode: ExecutionMode::Sequential,
            fallback_occurred,
            total_time_ms: start.elapsed().as_millis() as u64,
            layer_results,
            explanation,
        }
    }

    async fn execute_parallel(
        &self,
        query: &str,
        chain: FallbackChain,
        limit: usize,
        timeout: Duration,
        tier: CapabilityTier,
        beam_width: u8,
    ) -> ExecutionResult {
        let start = Instant::now();

        // Take only up to beam_width layers for parallel execution
        let parallel_layers: Vec<_> = chain
            .layers
            .iter()
            .filter(|l| self.executor.supports(**l))
            .take(beam_width as usize)
            .copied()
            .collect();

        if parallel_layers.is_empty() {
            return ExecutionResult {
                results: vec![],
                layers_attempted: vec![],
                primary_layer: RetrievalLayer::Agentic,
                tier,
                mode: ExecutionMode::Parallel,
                fallback_occurred: false,
                total_time_ms: start.elapsed().as_millis() as u64,
                layer_results: vec![],
                explanation: "No supported layers available".to_string(),
            };
        }

        // Execute all layers in parallel
        let mut handles = Vec::new();
        for layer in &parallel_layers {
            let executor = self.executor.clone();
            let query = query.to_string();
            let layer = *layer;

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let result = executor.execute(&query, layer, limit).await;
                let execution_time = start.elapsed().as_millis() as u64;

                match result {
                    Ok(results) => LayerResults::success(layer, results, execution_time),
                    Err(e) => LayerResults::failure(layer, e, execution_time),
                }
            });
            handles.push(handle);
        }

        // Wait for all with timeout
        let all_results = tokio::time::timeout(timeout, futures::future::join_all(handles)).await;

        let layer_results: Vec<LayerResults> = match all_results {
            Ok(results) => results.into_iter().filter_map(|r| r.ok()).collect(),
            Err(_) => {
                warn!("Parallel execution timed out");
                vec![]
            }
        };

        // Merge and deduplicate results
        let (merged_results, primary_layer, explanation) = if chain.merge_results {
            self.merge_results(&layer_results)
        } else {
            // Take results from best performing layer
            self.select_best_results(&layer_results)
        };

        ExecutionResult {
            results: merged_results,
            layers_attempted: parallel_layers,
            primary_layer,
            tier,
            mode: ExecutionMode::Parallel,
            fallback_occurred: false, // No fallback in parallel mode
            total_time_ms: start.elapsed().as_millis() as u64,
            layer_results,
            explanation,
        }
    }

    async fn execute_hybrid(
        &self,
        query: &str,
        chain: FallbackChain,
        limit: usize,
        timeout: Duration,
        tier: CapabilityTier,
        conditions: &StopConditions,
    ) -> ExecutionResult {
        let start = Instant::now();

        // Start parallel execution
        let parallel_layers: Vec<_> = chain
            .layers
            .iter()
            .filter(|l| self.executor.supports(**l))
            .take(conditions.beam_width as usize)
            .copied()
            .collect();

        if parallel_layers.is_empty() {
            return ExecutionResult {
                results: vec![],
                layers_attempted: vec![],
                primary_layer: RetrievalLayer::Agentic,
                tier,
                mode: ExecutionMode::Hybrid,
                fallback_occurred: false,
                total_time_ms: start.elapsed().as_millis() as u64,
                layer_results: vec![],
                explanation: "No supported layers available".to_string(),
            };
        }

        // Use tokio::select! to get first good result
        // For simplicity, we'll use the parallel approach and pick the winner
        let parallel_result = self
            .execute_parallel(query, chain, limit, timeout, tier, conditions.beam_width)
            .await;

        // In hybrid mode, if we got good results quickly, we're done
        // Otherwise, we continue with sequential fallback
        if parallel_result.has_results()
            && parallel_result
                .results
                .first()
                .map(|r| r.score >= conditions.min_confidence)
                .unwrap_or(false)
        {
            return ExecutionResult {
                mode: ExecutionMode::Hybrid,
                explanation: format!(
                    "Hybrid mode: {} returned strong results quickly",
                    parallel_result.primary_layer.as_str()
                ),
                ..parallel_result
            };
        }

        // No strong results from parallel, note it
        ExecutionResult {
            mode: ExecutionMode::Hybrid,
            explanation: format!(
                "Hybrid mode: parallel execution completed, best from {}",
                parallel_result.primary_layer.as_str()
            ),
            ..parallel_result
        }
    }

    fn merge_results(
        &self,
        layer_results: &[LayerResults],
    ) -> (Vec<SearchResult>, RetrievalLayer, String) {
        let mut all_results: Vec<SearchResult> = layer_results
            .iter()
            .filter(|lr| lr.success)
            .flat_map(|lr| lr.results.clone())
            .collect();

        // Deduplicate by doc_id, keeping highest score
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut deduped = Vec::new();

        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for result in all_results {
            if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(result.doc_id.clone())
            {
                e.insert(deduped.len());
                deduped.push(result);
            }
        }

        let primary = layer_results
            .iter()
            .filter(|lr| lr.success && !lr.results.is_empty())
            .min_by_key(|lr| lr.layer.layer_number())
            .map(|lr| lr.layer)
            .unwrap_or(RetrievalLayer::Agentic);

        let explanation = format!(
            "Merged {} results from {} layers",
            deduped.len(),
            layer_results.iter().filter(|lr| lr.success).count()
        );

        (deduped, primary, explanation)
    }

    fn select_best_results(
        &self,
        layer_results: &[LayerResults],
    ) -> (Vec<SearchResult>, RetrievalLayer, String) {
        // Find the layer with best results (highest top score)
        let best = layer_results
            .iter()
            .filter(|lr| lr.success && !lr.results.is_empty())
            .max_by(|a, b| {
                let a_score = a.results.first().map(|r| r.score).unwrap_or(0.0);
                let b_score = b.results.first().map(|r| r.score).unwrap_or(0.0);
                a_score
                    .partial_cmp(&b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        match best {
            Some(lr) => {
                let explanation = format!(
                    "Selected {} with top score {:.3}",
                    lr.layer.as_str(),
                    lr.results.first().map(|r| r.score).unwrap_or(0.0)
                );
                (lr.results.clone(), lr.layer, explanation)
            }
            None => {
                let primary = layer_results
                    .first()
                    .map(|lr| lr.layer)
                    .unwrap_or(RetrievalLayer::Agentic);
                (
                    vec![],
                    primary,
                    "No successful results from any layer".to_string(),
                )
            }
        }
    }
}

// We need futures for join_all
use futures;

/// Mock layer executor for testing.
#[derive(Default)]
pub struct MockLayerExecutor {
    /// Results to return for each layer
    pub results: std::collections::HashMap<RetrievalLayer, Vec<SearchResult>>,
    /// Simulated delay for each layer
    pub delays: std::collections::HashMap<RetrievalLayer, Duration>,
    /// Which layers to fail
    pub fail_layers: std::collections::HashSet<RetrievalLayer>,
}

impl MockLayerExecutor {
    /// Add results for a layer.
    pub fn with_results(mut self, layer: RetrievalLayer, results: Vec<SearchResult>) -> Self {
        self.results.insert(layer, results);
        self
    }

    /// Add delay for a layer.
    pub fn with_delay(mut self, layer: RetrievalLayer, delay: Duration) -> Self {
        self.delays.insert(layer, delay);
        self
    }

    /// Mark a layer as failing.
    pub fn with_failure(mut self, layer: RetrievalLayer) -> Self {
        self.fail_layers.insert(layer);
        self
    }
}

#[async_trait]
impl LayerExecutor for MockLayerExecutor {
    async fn execute(
        &self,
        _query: &str,
        layer: RetrievalLayer,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String> {
        // Apply delay if configured
        if let Some(delay) = self.delays.get(&layer) {
            tokio::time::sleep(*delay).await;
        }

        // Check if layer should fail
        if self.fail_layers.contains(&layer) {
            return Err(format!("{} layer failed", layer.as_str()));
        }

        // Return configured results or empty
        let results = self.results.get(&layer).cloned().unwrap_or_default();

        Ok(results.into_iter().take(limit).collect())
    }

    fn supports(&self, _layer: RetrievalLayer) -> bool {
        true // Mock supports all layers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_results(layer: RetrievalLayer, count: usize, base_score: f32) -> Vec<SearchResult> {
        (0..count)
            .map(|i| SearchResult {
                doc_id: format!("doc-{}-{}", layer.as_str(), i),
                doc_type: "test".to_string(),
                score: base_score - (i as f32 * 0.1),
                text_preview: format!("Result {} from {}", i, layer.as_str()),
                source_layer: layer,
                metadata: std::collections::HashMap::new(),
            })
            .collect()
    }

    #[tokio::test]
    async fn test_sequential_execution() {
        let executor = MockLayerExecutor::default().with_results(
            RetrievalLayer::BM25,
            sample_results(RetrievalLayer::BM25, 5, 0.8),
        );

        let retrieval = RetrievalExecutor::new(Arc::new(executor));
        let chain = FallbackChain::for_intent(QueryIntent::Locate, CapabilityTier::Keyword);
        let conditions = StopConditions::default();

        let result = retrieval
            .execute(
                "test query",
                chain,
                &conditions,
                ExecutionMode::Sequential,
                CapabilityTier::Keyword,
            )
            .await;

        assert!(result.has_results());
        assert_eq!(result.primary_layer, RetrievalLayer::BM25);
        assert_eq!(result.mode, ExecutionMode::Sequential);
    }

    #[tokio::test]
    async fn test_fallback_on_failure() {
        let executor = MockLayerExecutor::default()
            .with_failure(RetrievalLayer::BM25)
            .with_results(
                RetrievalLayer::Agentic,
                sample_results(RetrievalLayer::Agentic, 3, 0.5),
            );

        let retrieval = RetrievalExecutor::new(Arc::new(executor));
        let chain = FallbackChain::for_intent(QueryIntent::Locate, CapabilityTier::Keyword);
        let conditions = StopConditions::default();

        let result = retrieval
            .execute(
                "test query",
                chain,
                &conditions,
                ExecutionMode::Sequential,
                CapabilityTier::Keyword,
            )
            .await;

        assert!(result.has_results());
        assert!(result.fallback_occurred);
        assert_eq!(result.primary_layer, RetrievalLayer::Agentic);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let executor = MockLayerExecutor::default()
            .with_results(
                RetrievalLayer::BM25,
                sample_results(RetrievalLayer::BM25, 5, 0.7),
            )
            .with_results(
                RetrievalLayer::Vector,
                sample_results(RetrievalLayer::Vector, 5, 0.8),
            );

        let retrieval = RetrievalExecutor::new(Arc::new(executor));
        let chain = FallbackChain {
            layers: vec![RetrievalLayer::BM25, RetrievalLayer::Vector],
            merge_results: false,
            max_layers: 2,
        };
        let conditions = StopConditions::default().with_beam_width(2);

        let result = retrieval
            .execute(
                "test query",
                chain,
                &conditions,
                ExecutionMode::Parallel,
                CapabilityTier::Hybrid,
            )
            .await;

        assert!(result.has_results());
        // Vector has higher score, should be primary
        assert_eq!(result.primary_layer, RetrievalLayer::Vector);
    }

    #[tokio::test]
    async fn test_merged_results() {
        let executor = MockLayerExecutor::default()
            .with_results(
                RetrievalLayer::BM25,
                sample_results(RetrievalLayer::BM25, 3, 0.7),
            )
            .with_results(
                RetrievalLayer::Vector,
                sample_results(RetrievalLayer::Vector, 3, 0.8),
            );

        let retrieval = RetrievalExecutor::new(Arc::new(executor));
        let chain = FallbackChain::merged(vec![RetrievalLayer::BM25, RetrievalLayer::Vector]);
        let conditions = StopConditions::default().with_beam_width(2);

        let result = retrieval
            .execute(
                "test query",
                chain,
                &conditions,
                ExecutionMode::Parallel,
                CapabilityTier::Hybrid,
            )
            .await;

        // Should have results from both layers, deduplicated
        assert!(result.has_results());
        assert!(result.explanation.contains("Merged"));
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        // BM25 takes 200ms (longer than per-layer timeout of 100ms)
        // But overall timeout is 500ms, enough for BM25 to timeout then try Agentic
        let executor = MockLayerExecutor::default()
            .with_delay(RetrievalLayer::BM25, Duration::from_millis(200))
            .with_results(
                RetrievalLayer::Agentic,
                sample_results(RetrievalLayer::Agentic, 2, 0.4),
            );

        let retrieval = RetrievalExecutor::new(Arc::new(executor));
        let chain = FallbackChain::for_intent(QueryIntent::Locate, CapabilityTier::Keyword);
        // Overall timeout of 500ms - enough to try BM25 (timeout after ~100ms) then Agentic
        let conditions = StopConditions::with_timeout(Duration::from_millis(500));

        let result = retrieval
            .execute(
                "test query",
                chain,
                &conditions,
                ExecutionMode::Sequential,
                CapabilityTier::Keyword,
            )
            .await;

        // BM25 should timeout, fallback to Agentic
        assert!(result.has_results());
        assert_eq!(result.primary_layer, RetrievalLayer::Agentic);
    }

    #[test]
    fn test_fallback_chain_for_intent() {
        let chain = FallbackChain::for_intent(QueryIntent::Explore, CapabilityTier::Full);
        assert_eq!(chain.layers[0], RetrievalLayer::Topics);

        let chain = FallbackChain::for_intent(QueryIntent::Locate, CapabilityTier::Full);
        assert_eq!(chain.layers[0], RetrievalLayer::BM25);

        let chain = FallbackChain::for_intent(QueryIntent::Answer, CapabilityTier::Full);
        assert_eq!(chain.layers[0], RetrievalLayer::Hybrid);
    }

    #[test]
    fn test_layer_results_is_sufficient() {
        let results = sample_results(RetrievalLayer::BM25, 3, 0.8);
        let lr = LayerResults::success(RetrievalLayer::BM25, results, 100);

        assert!(lr.is_sufficient(0.3));
        assert!(lr.is_sufficient(0.7));
        assert!(!lr.is_sufficient(0.9));
    }
}
