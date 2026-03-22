//! Reciprocal Rank Fusion (RRF) for merging results from multiple indexes.
//!
//! RRF fuses ranked lists by summing `1/(k + rank)` for each document across
//! all lists, then sorting by cumulative score. Documents appearing in more
//! lists receive a consensus boost.

use memory_retrieval::{RetrievalLayer, SearchResult};
use std::collections::HashMap;

/// A search result after RRF fusion with its cumulative RRF score.
#[derive(Debug, Clone)]
pub struct FusedResult {
    /// Cumulative RRF score across all input lists.
    pub rrf_score: f64,
    /// The original search result (from whichever list contributed it first).
    pub inner: SearchResult,
}

/// Fuse multiple ranked lists using Reciprocal Rank Fusion.
///
/// Each document's RRF score is `sum(1 / (k + rank))` across all lists in
/// which it appears. Duplicate `doc_id` values are deduplicated (first
/// occurrence kept). The output is sorted by descending RRF score.
///
/// # Arguments
/// * `lists` - Vector of ranked result lists (one per index/layer).
/// * `k` - RRF constant (typically 60.0). Higher values dampen rank differences.
pub fn rrf_fuse(lists: Vec<Vec<SearchResult>>, k: f64) -> Vec<FusedResult> {
    let mut scores: HashMap<String, (f64, SearchResult)> = HashMap::new();

    for list in &lists {
        for (rank, result) in list.iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank + 1) as f64);
            scores
                .entry(result.doc_id.clone())
                .and_modify(|(s, _)| *s += rrf_score)
                .or_insert((rrf_score, result.clone()));
        }
    }

    let mut fused: Vec<FusedResult> = scores
        .into_values()
        .map(|(score, result)| FusedResult {
            rrf_score: score,
            inner: result,
        })
        .collect();

    fused.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, score: f32, layer: RetrievalLayer) -> SearchResult {
        SearchResult {
            doc_id: id.to_string(),
            doc_type: "toc_node".to_string(),
            score,
            text_preview: id.to_string(),
            source_layer: layer,
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_rrf_single_list_preserves_order() {
        let list = vec![
            make_result("a", 0.9, RetrievalLayer::BM25),
            make_result("b", 0.7, RetrievalLayer::BM25),
            make_result("c", 0.5, RetrievalLayer::BM25),
        ];
        let fused = rrf_fuse(vec![list], 60.0);
        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].inner.doc_id, "a");
        assert_eq!(fused[1].inner.doc_id, "b");
        assert_eq!(fused[2].inner.doc_id, "c");
    }

    #[test]
    fn test_rrf_consensus_boosts_result() {
        // "a" appears only in list 1 at rank 1 (highest individual)
        // "b" appears in all 3 lists at various ranks — consensus should win
        let list1 = vec![
            make_result("a", 0.95, RetrievalLayer::BM25),
            make_result("b", 0.6, RetrievalLayer::BM25),
        ];
        let list2 = vec![
            make_result("b", 0.8, RetrievalLayer::Vector),
            make_result("c", 0.5, RetrievalLayer::Vector),
        ];
        let list3 = vec![
            make_result("b", 0.7, RetrievalLayer::Topics),
            make_result("d", 0.4, RetrievalLayer::Topics),
        ];
        let fused = rrf_fuse(vec![list1, list2, list3], 60.0);

        // "b" should be ranked higher than "a" due to consensus across 3 lists
        let b_pos = fused.iter().position(|r| r.inner.doc_id == "b").unwrap();
        let a_pos = fused.iter().position(|r| r.inner.doc_id == "a").unwrap();
        assert!(
            b_pos < a_pos,
            "consensus doc 'b' (pos {b_pos}) should rank above single-list doc 'a' (pos {a_pos})"
        );
    }

    #[test]
    fn test_rrf_empty_lists_handled() {
        let fused = rrf_fuse(vec![vec![], vec![]], 60.0);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_rrf_deduplicates_same_doc() {
        let list = vec![
            make_result("x", 0.9, RetrievalLayer::BM25),
            make_result("x", 0.5, RetrievalLayer::BM25),
        ];
        let fused = rrf_fuse(vec![list], 60.0);
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].inner.doc_id, "x");
    }
}
