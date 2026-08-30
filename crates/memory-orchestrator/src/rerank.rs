//! Result reranking (heuristic and LLM-based).
//!
//! Provides a `Reranker` trait with three implementations:
//! - `HeuristicReranker`: score-based sorting and top-K trimming (default).
//! - `LlmReranker`: prompt an LLM over top-k candidates and honor the returned
//!   order. Selected when an API key / `Completer` is configured.
//! - `CrossEncoderReranker`: extension point that returns `RerankError::NotImplemented`
//!   — never warn-and-fallback.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use memory_retrieval::SearchResult;
use thiserror::Error;

use crate::fusion::FusedResult;

/// Errors from reranking.
#[derive(Debug, Error)]
pub enum RerankError {
    /// Cross-encoder path is an extension point, not a silent fallback.
    #[error("cross-encoder reranking is not implemented")]
    NotImplemented,
}

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
    /// Original search result (doc_type, metadata, layer preserved).
    pub inner: SearchResult,
}

impl RerankedResult {
    fn from_fused(fused: FusedResult, score: f64) -> Self {
        Self {
            doc_id: fused.inner.doc_id.clone(),
            score,
            text: fused.inner.text_preview.clone(),
            source_layer: format!("{:?}", fused.inner.source_layer),
            inner: fused.inner,
        }
    }
}

/// Trait for result reranking strategies.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank fused results, returning a sorted and potentially trimmed list.
    async fn rerank(&self, query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>>;

    /// Label of the strategy that actually produced the last `rerank` output.
    ///
    /// LLM fail-open reports `"heuristic"` so explainability cannot claim a
    /// rerank that did not run.
    fn mode_name(&self) -> &'static str {
        "heuristic"
    }
}

/// LLM text completion used by [`LlmReranker`].
///
/// Production wiring wraps `memory_toc::ApiSummarizer::complete`. Tests inject
/// a mock that returns a known JSON ordering.
#[async_trait]
pub trait Completer: Send + Sync {
    /// Complete `prompt` and return the model text.
    async fn complete(&self, prompt: &str) -> Result<String>;
}

/// Default reranker: sorts by RRF score descending and trims to `max_results`.
#[derive(Debug, Clone)]
pub struct HeuristicReranker {
    max_results: usize,
}

impl HeuristicReranker {
    /// Construct a heuristic reranker that keeps `max_results` hits.
    pub fn new(max_results: usize) -> Self {
        Self {
            max_results: max_results.max(1),
        }
    }

    fn rerank_sync(&self, results: Vec<FusedResult>) -> Vec<RerankedResult> {
        let mut sorted = results;
        sorted.sort_by(|a, b| {
            b.fusion_score
                .partial_cmp(&a.fusion_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
            .into_iter()
            .take(self.max_results)
            .map(|r| {
                let score = r.fusion_score;
                RerankedResult::from_fused(r, score)
            })
            .collect()
    }
}

impl Default for HeuristicReranker {
    fn default() -> Self {
        Self::new(10)
    }
}

#[async_trait]
impl Reranker for HeuristicReranker {
    async fn rerank(&self, _query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>> {
        Ok(self.rerank_sync(results))
    }
}

/// LLM reranker: asks a completer to order candidate doc ids.
pub struct LlmReranker {
    completer: Arc<dyn Completer>,
    max_results: usize,
    fell_back: AtomicBool,
}

impl LlmReranker {
    /// Create an LLM reranker around a completer.
    pub fn new(completer: Arc<dyn Completer>, max_results: usize) -> Self {
        Self {
            completer,
            max_results: max_results.max(1),
            fell_back: AtomicBool::new(false),
        }
    }

    fn build_prompt(query: &str, results: &[FusedResult]) -> String {
        let mut docs = String::new();
        for (i, r) in results.iter().enumerate() {
            docs.push_str(&format!(
                "{}. id={}\n{}\n\n",
                i + 1,
                r.inner.doc_id,
                r.inner.text_preview
            ));
        }
        format!(
            r#"Rank these memory documents for the search query. Return JSON only:
{{"order": ["doc_id_most_relevant", "..."]}}

Query: {query}

Documents:
{docs}
Include every doc_id exactly once, most relevant first."#
        )
    }

    fn parse_order(text: &str) -> Option<Vec<String>> {
        let json_str = extract_json_object(text);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        let arr = parsed.get("order")?.as_array()?;
        let ids: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(ToOwned::to_owned))
            .collect();
        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    }
}

fn extract_json_object(text: &str) -> String {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                return text[start..=end].to_string();
            }
        }
    }
    text.to_string()
}

#[async_trait]
impl Reranker for LlmReranker {
    fn mode_name(&self) -> &'static str {
        if self.fell_back.load(Ordering::Relaxed) {
            "heuristic"
        } else {
            "llm"
        }
    }

    async fn rerank(&self, query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>> {
        self.fell_back.store(false, Ordering::Relaxed);
        if results.is_empty() {
            return Ok(Vec::new());
        }
        let prompt = Self::build_prompt(query, &results);
        let response = match self.completer.complete(&prompt).await {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!(error = %e, "LLM rerank failed; keeping RRF order");
                self.fell_back.store(true, Ordering::Relaxed);
                return HeuristicReranker::new(self.max_results)
                    .rerank(query, results)
                    .await;
            }
        };
        let Some(order) = Self::parse_order(&response) else {
            tracing::warn!("LLM rerank returned unparseable order; keeping RRF order");
            self.fell_back.store(true, Ordering::Relaxed);
            return HeuristicReranker::new(self.max_results)
                .rerank(query, results)
                .await;
        };
        let mut by_id: HashMap<String, FusedResult> = results
            .into_iter()
            .map(|r| (r.inner.doc_id.clone(), r))
            .collect();
        let mut out = Vec::new();
        for id in order {
            if let Some(fused) = by_id.remove(&id) {
                let score = fused.fusion_score;
                out.push(RerankedResult::from_fused(fused, score));
            }
        }
        // Append any ids the model omitted (sort leftover by fusion score).
        let mut leftover: Vec<FusedResult> = by_id.into_values().collect();
        leftover.sort_by(|a, b| {
            b.fusion_score
                .partial_cmp(&a.fusion_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for fused in leftover {
            out.push(RerankedResult::from_fused(fused, 0.0));
        }
        out.truncate(self.max_results);
        // Positional scores so downstream ranking cannot undo the LLM order.
        let n = out.len() as f64;
        for (i, item) in out.iter_mut().enumerate() {
            item.score = ((n - i as f64) / n).clamp(0.0, 1.0);
        }
        Ok(out)
    }
}

/// Stub cross-encoder reranker. Returns a hard error — never a silent fallback.
#[derive(Debug, Default)]
pub struct CrossEncoderReranker;

#[async_trait]
impl Reranker for CrossEncoderReranker {
    fn mode_name(&self) -> &'static str {
        "cross-encoder"
    }

    async fn rerank(
        &self,
        _query: &str,
        _results: Vec<FusedResult>,
    ) -> Result<Vec<RerankedResult>> {
        Err(RerankError::NotImplemented.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_retrieval::RetrievalLayer;

    fn make_fused(id: &str, fusion_score: f64) -> FusedResult {
        FusedResult {
            fusion_score,
            inner: SearchResult {
                doc_id: id.to_string(),
                doc_type: "toc_node".to_string(),
                score: fusion_score as f32,
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
        results.reverse();

        let reranker = HeuristicReranker::new(10);
        let reranked = reranker.rerank("test query", results).await.unwrap();

        assert_eq!(reranked.len(), 10, "should trim to top 10");
        assert_eq!(reranked[0].doc_id, "doc-0", "highest score should be first");
        assert!(
            reranked[0].score > reranked[9].score,
            "first should score higher than last"
        );
        assert_eq!(reranked[0].inner.doc_type, "toc_node");
    }

    #[tokio::test]
    async fn test_cross_encoder_returns_not_implemented() {
        let results = vec![make_fused("a", 0.9), make_fused("b", 0.5)];
        let reranker = CrossEncoderReranker;
        let err = reranker.rerank("test query", results).await.unwrap_err();
        assert!(
            err.to_string().contains("not implemented"),
            "cross-encoder must hard-error, got: {err}"
        );
    }

    struct ReverseCompleter;

    #[async_trait]
    impl Completer for ReverseCompleter {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok(r#"{"order": ["b", "a"]}"#.to_string())
        }
    }

    #[tokio::test]
    async fn test_llm_reranker_honors_completer_order() {
        let results = vec![make_fused("a", 0.9), make_fused("b", 0.5)];
        let reranker = LlmReranker::new(Arc::new(ReverseCompleter), 10);
        let reranked = reranker.rerank("q", results).await.unwrap();
        assert_eq!(reranked[0].doc_id, "b");
        assert_eq!(reranked[1].doc_id, "a");
        assert_eq!(reranker.mode_name(), "llm");
    }

    struct BrokenCompleter;

    #[async_trait]
    impl Completer for BrokenCompleter {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            anyhow::bail!("network down");
        }
    }

    #[tokio::test]
    async fn test_llm_reranker_fail_open_on_completer_error() {
        let results = vec![make_fused("a", 0.9), make_fused("b", 0.5)];
        let reranker = LlmReranker::new(Arc::new(BrokenCompleter), 10);
        let reranked = reranker.rerank("q", results).await.unwrap();
        assert_eq!(reranked[0].doc_id, "a");
        assert_eq!(reranked[1].doc_id, "b");
        assert_eq!(
            reranker.mode_name(),
            "heuristic",
            "fail-open must not claim llm ran"
        );
    }

    struct GarbageCompleter;

    #[async_trait]
    impl Completer for GarbageCompleter {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok("not json".to_string())
        }
    }

    #[tokio::test]
    async fn test_llm_reranker_fail_open_on_unparseable_order() {
        let results = vec![make_fused("a", 0.9), make_fused("b", 0.5)];
        let reranker = LlmReranker::new(Arc::new(GarbageCompleter), 10);
        let reranked = reranker.rerank("q", results).await.unwrap();
        assert_eq!(reranked[0].doc_id, "a");
        assert_eq!(reranker.mode_name(), "heuristic");
    }
}
