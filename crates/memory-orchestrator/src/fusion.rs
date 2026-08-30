//! Reciprocal Rank Fusion (RRF) for merging results from multiple indexes.
//!
//! This is the **canonical** RRF implementation for the workspace (Phase 54-04).
//! Weighted form: `sum(weight_i / (k + rank_i))`. Unweighted fusion is the
//! special case `weight_i = 1.0`.
//!
//! Documents appearing in more lists receive a consensus boost.

use memory_retrieval::SearchResult;
use std::collections::HashMap;

/// A search result after RRF fusion with its cumulative RRF score.
#[derive(Debug, Clone)]
pub struct FusedResult {
    /// Cumulative RRF score across all input lists.
    pub fusion_score: f64,
    /// The original search result (from whichever list contributed it first).
    pub inner: SearchResult,
}

/// Fuse multiple ranked lists using unweighted Reciprocal Rank Fusion.
///
/// Each document's RRF score is `sum(1 / (k + rank))` across all lists in
/// which it appears. Duplicate `doc_id` values are deduplicated (first
/// occurrence kept). The output is sorted by descending RRF score.
///
/// # Arguments
/// * `lists` - Vector of ranked result lists (one per index/layer).
/// * `k` - RRF constant (typically 60.0). Higher values dampen rank differences.
pub fn fuse(lists: Vec<Vec<SearchResult>>, k: f64) -> Vec<FusedResult> {
    fuse_weighted(lists.into_iter().map(|list| (1.0, list)).collect(), k)
}

/// Fuse ranked lists using **weighted** Reciprocal Rank Fusion.
///
/// Each list is paired with a weight. Score contribution is
/// `weight / (k + rank)` (1-based rank). Empty input returns an empty vec
/// (fail-open).
pub fn fuse_weighted(lists: Vec<(f64, Vec<SearchResult>)>, k: f64) -> Vec<FusedResult> {
    let mut scores: HashMap<String, (f64, SearchResult)> = HashMap::new();

    for (weight, list) in &lists {
        for (rank, result) in list.iter().enumerate() {
            let contribution = *weight / (k + (rank + 1) as f64);
            scores
                .entry(result.doc_id.clone())
                .and_modify(|(s, _)| *s += contribution)
                .or_insert((contribution, result.clone()));
        }
    }

    let mut fused: Vec<FusedResult> = scores
        .into_values()
        .map(|(score, result)| FusedResult {
            fusion_score: score,
            inner: result,
        })
        .collect();

    fused.sort_by(|a, b| {
        b.fusion_score
            .partial_cmp(&a.fusion_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fused
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_retrieval::RetrievalLayer;

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
        let fused = fuse(vec![list], 60.0);
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
        let fused = fuse(vec![list1, list2, list3], 60.0);

        let b_pos = fused.iter().position(|r| r.inner.doc_id == "b").unwrap();
        let a_pos = fused.iter().position(|r| r.inner.doc_id == "a").unwrap();
        assert!(
            b_pos < a_pos,
            "consensus doc 'b' (pos {b_pos}) should rank above single-list doc 'a' (pos {a_pos})"
        );
    }

    #[test]
    fn test_rrf_empty_lists_handled() {
        let fused = fuse(vec![vec![], vec![]], 60.0);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_rrf_empty_input_fail_open() {
        let fused = fuse(Vec::new(), 60.0);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_rrf_deduplicates_same_doc() {
        let list = vec![
            make_result("x", 0.9, RetrievalLayer::BM25),
            make_result("x", 0.5, RetrievalLayer::BM25),
        ];
        let fused = fuse(vec![list], 60.0);
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].inner.doc_id, "x");
    }

    #[test]
    fn test_weighted_fusion_differs_from_either_input() {
        // BM25 ranks a > b; vector ranks b > a. Equal weights → b wins on
        // consensus if we add a third mention of b, but with two lists of
        // length 1 each the scores are equal. Use diverging two-item lists:
        // BM25: a, c     Vector: b, a
        // a appears in both → consensus; fused order must differ from BM25
        // (which has a then c, no b first) AND from vector (b then a).
        let bm25 = vec![
            make_result("a", 0.99, RetrievalLayer::BM25),
            make_result("c", 0.50, RetrievalLayer::BM25),
        ];
        let vector = vec![
            make_result("b", 0.99, RetrievalLayer::Vector),
            make_result("a", 0.50, RetrievalLayer::Vector),
        ];
        let fused = fuse_weighted(vec![(0.5, bm25.clone()), (0.5, vector.clone())], 60.0);
        let fused_ids: Vec<&str> = fused.iter().map(|r| r.inner.doc_id.as_str()).collect();
        let bm25_ids = ["a", "c"];
        let vector_ids = ["b", "a"];
        assert_ne!(
            fused_ids.as_slice(),
            &bm25_ids[..],
            "fusion must not equal BM25-only ranking"
        );
        assert_ne!(
            fused_ids.as_slice(),
            &vector_ids[..],
            "fusion must not equal vector-only ranking"
        );
        // a appears in both lists, should rank first
        assert_eq!(fused[0].inner.doc_id, "a");
    }

    #[test]
    fn test_higher_weight_shifts_ranking() {
        let bm25 = vec![make_result("bm25-winner", 0.9, RetrievalLayer::BM25)];
        let vector = vec![make_result("vec-winner", 0.9, RetrievalLayer::Vector)];
        let fused = fuse_weighted(vec![(0.9, bm25), (0.1, vector)], 60.0);
        assert_eq!(fused[0].inner.doc_id, "bm25-winner");
    }
}
