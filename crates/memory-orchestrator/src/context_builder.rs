//! Context assembly for LLM consumption.
//!
//! Converts reranked retrieval results into a structured `MemoryContext`
//! suitable for injection into LLM prompts.

use crate::rerank::RerankedResult;
use crate::types::{MemoryContext, RankedResult};

/// Builds a `MemoryContext` from reranked retrieval results.
///
/// Token estimation uses a 0.75 chars-per-token ratio with a fixed 50-token
/// overhead for framing. `key_entities` and `open_questions` are currently
/// empty (populated in Phase C).
pub struct ContextBuilder;

impl ContextBuilder {
    /// Build a `MemoryContext` from reranked results and the original query.
    ///
    /// # Arguments
    /// * `query` - The original user query (used in the summary).
    /// * `results` - Reranked results from the reranker stage.
    pub fn build(query: &str, results: Vec<RerankedResult>) -> MemoryContext {
        let confidence = results.first().map(|r| r.score).unwrap_or(0.0);

        let relevant_events: Vec<RankedResult> = results
            .iter()
            .map(|r| RankedResult {
                score: r.score,
                doc_id: r.doc_id.clone(),
                text: r.text.clone(),
                source_layer: r.source_layer.clone(),
                confidence: r.score,
            })
            .collect();

        let total_chars: usize = relevant_events.iter().map(|r| r.text.len()).sum();
        let tokens_estimated = (total_chars as f64 * 0.75) as usize + 50;

        let summary = if relevant_events.is_empty() {
            "No relevant memory found.".to_string()
        } else {
            format!(
                "Found {} relevant memory entries for: \"{}\"",
                relevant_events.len(),
                query
            )
        };

        MemoryContext {
            summary,
            relevant_events,
            key_entities: vec![],
            open_questions: vec![],
            retrieval_ms: 0, // Set by orchestrator after timing
            tokens_estimated,
            confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rerank::RerankedResult;

    fn make_reranked(id: &str, text: &str, score: f64) -> RerankedResult {
        RerankedResult {
            doc_id: id.to_string(),
            score,
            text: text.to_string(),
            source_layer: "bm25".to_string(),
        }
    }

    #[test]
    fn test_context_builder_empty_results() {
        let ctx = ContextBuilder::build("test query", vec![]);
        assert!(ctx.relevant_events.is_empty());
        assert!((ctx.confidence - 0.0).abs() < f64::EPSILON);
        assert_eq!(ctx.summary, "No relevant memory found.");
    }

    #[test]
    fn test_context_builder_confidence_from_top_score() {
        let results = vec![make_reranked("a", "hello world", 0.75)];
        let ctx = ContextBuilder::build("test", results);
        assert!(
            (ctx.confidence - 0.75).abs() < f64::EPSILON,
            "confidence should match top result score"
        );
        assert_eq!(ctx.relevant_events.len(), 1);
        assert!(ctx.summary.contains("Found 1 relevant"));
    }

    #[test]
    fn test_context_builder_tokens_estimated_nonzero() {
        let results = vec![make_reranked("a", "hello world", 0.8)];
        let ctx = ContextBuilder::build("test", results);
        // "hello world" = 11 chars, 11 * 0.75 = 8.25 -> 8 + 50 = 58
        assert!(
            ctx.tokens_estimated > 0,
            "tokens should be nonzero for non-empty results"
        );
        assert_eq!(ctx.tokens_estimated, 58);
    }
}
