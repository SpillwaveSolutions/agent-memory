//! Staleness-based score decay for query results.
//!
//! Applies exponential time-decay to search results relative to the newest
//! result in the set. High-salience memory kinds (Constraint, Definition,
//! Procedure, Preference) are exempt from decay.
//!
//! ## Formula
//!
//! ```text
//! adjusted_score = score * (1.0 - max_penalty * (1.0 - exp(-age_days / half_life)))
//! ```
//!
//! The decay is asymptotic: it approaches but never reaches `max_penalty`.

use std::collections::HashMap;

use memory_types::config::StalenessConfig;

use crate::executor::SearchResult;

/// Applies staleness-based time-decay to search results.
///
/// Results are scored relative to the newest result in the set.
/// Results without timestamps pass through unchanged (fail-open).
/// High-salience memory kinds are exempt from decay.
pub struct StaleFilter {
    config: StalenessConfig,
}

impl StaleFilter {
    /// Create a new StaleFilter with the given configuration.
    pub fn new(config: StalenessConfig) -> Self {
        Self { config }
    }

    /// Apply staleness scoring to a set of search results (time-decay only).
    ///
    /// If results are empty or staleness is disabled, returns unchanged.
    /// Otherwise, applies time-decay relative to the newest timestamp
    /// and re-sorts by adjusted score.
    ///
    /// For supersession detection, use [`apply_with_supersession`](Self::apply_with_supersession).
    pub fn apply(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        self.apply_with_supersession(results, None)
    }

    /// Apply time-decay and supersession detection.
    ///
    /// `embeddings`: optional doc_id -> embedding map for supersession.
    /// If `None` or empty, only time-decay is applied.
    pub fn apply_with_supersession(
        &self,
        results: Vec<SearchResult>,
        embeddings: Option<&HashMap<String, Vec<f32>>>,
    ) -> Vec<SearchResult> {
        if results.is_empty() || !self.config.enabled {
            return results;
        }

        let newest_ts = self.find_newest_timestamp(&results);
        let mut adjusted = match newest_ts {
            Some(ts) => self.apply_time_decay(results, ts),
            None => results, // No timestamps at all -- pass through
        };

        // Apply supersession if embeddings available
        if let Some(embs) = embeddings {
            if !embs.is_empty() {
                self.apply_supersession(&mut adjusted, embs);
            }
        }

        adjusted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        adjusted
    }

    /// Find the newest (maximum) timestamp_ms across all results.
    fn find_newest_timestamp(&self, results: &[SearchResult]) -> Option<i64> {
        results
            .iter()
            .filter_map(|r| {
                r.metadata
                    .get("timestamp_ms")
                    .and_then(|v| v.parse::<i64>().ok())
            })
            .max()
    }

    /// Apply time-decay to each result based on age relative to newest_ts.
    fn apply_time_decay(&self, results: Vec<SearchResult>, newest_ts: i64) -> Vec<SearchResult> {
        let half_life = self.config.half_life_days as f64;
        let max_penalty = self.config.max_penalty as f64;

        results
            .into_iter()
            .map(|mut r| {
                // Parse timestamp; if missing, no penalty (fail-open)
                let ts = match r
                    .metadata
                    .get("timestamp_ms")
                    .and_then(|v| v.parse::<i64>().ok())
                {
                    Some(ts) => ts,
                    None => return r,
                };

                // Parse memory_kind; default to "observation"
                let kind_str = r
                    .metadata
                    .get("memory_kind")
                    .cloned()
                    .unwrap_or_else(|| "observation".to_string());

                // Exempt high-salience kinds
                if Self::is_exempt(&kind_str) {
                    return r;
                }

                // Calculate age in days
                let age_days = (newest_ts - ts) as f64 / 86_400_000.0;

                // Apply decay formula:
                // score * (1.0 - max_penalty * (1.0 - exp(-age_days / half_life)))
                let decay_factor = 1.0 - max_penalty * (1.0 - (-age_days / half_life).exp());
                r.score = (r.score as f64 * decay_factor) as f32;

                r
            })
            .collect()
    }

    /// Apply supersession detection to adjusted results.
    ///
    /// For each older result, checks if a newer result is semantically similar
    /// (cosine similarity >= threshold). If so, marks the older result as
    /// superseded and applies an additional penalty. Each result is superseded
    /// at most once (no transitivity).
    fn apply_supersession(
        &self,
        results: &mut [SearchResult],
        embeddings: &HashMap<String, Vec<f32>>,
    ) {
        let threshold = self.config.supersession_threshold;
        let penalty_factor = 1.0 - self.config.supersession_penalty;

        // Build index sorted by timestamp descending (newest first).
        // Each element is (index_into_results, timestamp_ms).
        let mut by_time: Vec<(usize, i64)> = results
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                r.metadata
                    .get("timestamp_ms")
                    .and_then(|v| v.parse::<i64>().ok())
                    .map(|ts| (i, ts))
            })
            .collect();
        by_time.sort_by_key(|b| std::cmp::Reverse(b.1)); // newest first

        // For each pair (older vs newer), check supersession
        for older_pos in (0..by_time.len()).rev() {
            let (older_idx, older_ts) = by_time[older_pos];

            // Skip if already superseded
            if results[older_idx].metadata.contains_key("superseded_by") {
                continue;
            }

            // Skip exempt kinds
            let kind_str = results[older_idx]
                .metadata
                .get("memory_kind")
                .cloned()
                .unwrap_or_else(|| "observation".to_string());
            if Self::is_exempt(&kind_str) {
                continue;
            }

            // Get embedding for older result
            let older_emb = match embeddings.get(&results[older_idx].doc_id) {
                Some(e) => e,
                None => continue,
            };

            // Check against newer results
            for &(newer_idx, newer_ts) in &by_time {
                if newer_ts <= older_ts {
                    break; // No more newer results
                }

                let newer_emb = match embeddings.get(&results[newer_idx].doc_id) {
                    Some(e) => e,
                    None => continue,
                };

                let similarity = dot_product(older_emb, newer_emb);
                if similarity >= threshold {
                    // Mark superseded
                    results[older_idx].metadata.insert(
                        "superseded_by".to_string(),
                        results[newer_idx].doc_id.clone(),
                    );
                    results[older_idx].score *= penalty_factor;
                    break; // No transitivity
                }
            }
        }
    }

    /// Check if a memory kind is exempt from staleness decay.
    ///
    /// High-salience kinds are: constraint, definition, procedure, preference.
    fn is_exempt(kind_str: &str) -> bool {
        matches!(
            kind_str.to_lowercase().as_str(),
            "constraint" | "definition" | "procedure" | "preference"
        )
    }
}

/// Compute dot product of two vectors.
///
/// For pre-normalized vectors (as produced by CandleEmbedder),
/// this equals cosine similarity.
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_result(
        doc_id: &str,
        score: f32,
        timestamp_ms: Option<i64>,
        memory_kind: Option<&str>,
    ) -> SearchResult {
        let mut metadata = HashMap::new();
        if let Some(ts) = timestamp_ms {
            metadata.insert("timestamp_ms".to_string(), ts.to_string());
        }
        if let Some(kind) = memory_kind {
            metadata.insert("memory_kind".to_string(), kind.to_string());
        }
        SearchResult {
            doc_id: doc_id.to_string(),
            doc_type: "toc_node".to_string(),
            score,
            text_preview: format!("Preview for {doc_id}"),
            source_layer: crate::types::RetrievalLayer::BM25,
            metadata,
        }
    }

    fn default_config() -> StalenessConfig {
        StalenessConfig::default()
    }

    const DAY_MS: i64 = 86_400_000;

    #[test]
    fn test_empty_results_unchanged() {
        let filter = StaleFilter::new(default_config());
        let results = filter.apply(vec![]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_disabled_returns_unchanged() {
        let mut config = default_config();
        config.enabled = false;
        let filter = StaleFilter::new(config);

        let results = vec![
            make_result("a", 0.9, Some(1000), Some("observation")),
            make_result("b", 0.8, Some(500), Some("observation")),
        ];
        let output = filter.apply(results);
        assert!((output[0].score - 0.9).abs() < f32::EPSILON);
        assert!((output[1].score - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_no_timestamps_no_penalty() {
        let filter = StaleFilter::new(default_config());

        let results = vec![
            make_result("a", 0.9, None, Some("observation")),
            make_result("b", 0.8, None, Some("observation")),
        ];
        let output = filter.apply(results);
        assert!((output[0].score - 0.9).abs() < f32::EPSILON);
        assert!((output[1].score - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_same_timestamp_no_penalty() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("a", 0.9, Some(now), Some("observation")),
            make_result("b", 0.8, Some(now), Some("observation")),
        ];
        let output = filter.apply(results);
        // Both have age_days = 0, so exp(0) = 1.0, factor = 1.0
        assert!((output[0].score - 0.9).abs() < f32::EPSILON);
        assert!((output[1].score - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_time_decay_formula() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        // half_life = 14 days, max_penalty = 0.30
        // At 14 days: factor = 1.0 - 0.30 * (1.0 - exp(-1)) ~= 1.0 - 0.30 * 0.6321 ~= 0.8104
        // penalty ~= 18.96%
        let results_14d = vec![
            make_result("new", 1.0, Some(now), Some("observation")),
            make_result("old", 1.0, Some(now - 14 * DAY_MS), Some("observation")),
        ];
        let output = filter.apply(results_14d);
        let old_score = output.iter().find(|r| r.doc_id == "old").unwrap().score;
        let penalty_14 = 1.0 - old_score;
        // ~19% penalty at 14 days
        assert!(
            (penalty_14 - 0.19).abs() < 0.02,
            "14-day penalty should be ~19%, got {:.1}%",
            penalty_14 * 100.0
        );

        // At 28 days: factor = 1.0 - 0.30 * (1.0 - exp(-2)) ~= 1.0 - 0.30 * 0.8647 ~= 0.7406
        // penalty ~= 25.9%
        let results_28d = vec![
            make_result("new", 1.0, Some(now), Some("observation")),
            make_result("old", 1.0, Some(now - 28 * DAY_MS), Some("observation")),
        ];
        let output = filter.apply(results_28d);
        let old_score = output.iter().find(|r| r.doc_id == "old").unwrap().score;
        let penalty_28 = 1.0 - old_score;
        assert!(
            (penalty_28 - 0.26).abs() < 0.02,
            "28-day penalty should be ~26%, got {:.1}%",
            penalty_28 * 100.0
        );

        // At 42 days: factor = 1.0 - 0.30 * (1.0 - exp(-3)) ~= 1.0 - 0.30 * 0.9502 ~= 0.7149
        // penalty ~= 28.5%
        let results_42d = vec![
            make_result("new", 1.0, Some(now), Some("observation")),
            make_result("old", 1.0, Some(now - 42 * DAY_MS), Some("observation")),
        ];
        let output = filter.apply(results_42d);
        let old_score = output.iter().find(|r| r.doc_id == "old").unwrap().score;
        let penalty_42 = 1.0 - old_score;
        assert!(
            (penalty_42 - 0.285).abs() < 0.02,
            "42-day penalty should be ~28.5%, got {:.1}%",
            penalty_42 * 100.0
        );
    }

    #[test]
    fn test_kind_exemption() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        for kind in &["constraint", "definition", "procedure", "preference"] {
            let results = vec![
                make_result("new", 1.0, Some(now), Some("observation")),
                make_result("exempt", 0.9, Some(now - 30 * DAY_MS), Some(kind)),
            ];
            let output = filter.apply(results);
            let exempt = output.iter().find(|r| r.doc_id == "exempt").unwrap();
            assert!(
                (exempt.score - 0.9).abs() < f32::EPSILON,
                "{kind} should be exempt from decay"
            );
        }
    }

    #[test]
    fn test_observation_gets_decay() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("new", 1.0, Some(now), Some("observation")),
            make_result("old", 0.9, Some(now - 14 * DAY_MS), Some("observation")),
        ];
        let output = filter.apply(results);
        let old = output.iter().find(|r| r.doc_id == "old").unwrap();
        // Should be decayed below 0.9
        assert!(old.score < 0.9, "Observation should be decayed");
    }

    #[test]
    fn test_results_reordered_by_score() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        // "old_high" starts higher but is much older, should drop below "new_low"
        let results = vec![
            make_result(
                "old_high",
                0.95,
                Some(now - 60 * DAY_MS),
                Some("observation"),
            ),
            make_result("new_low", 0.70, Some(now), Some("observation")),
        ];
        let output = filter.apply(results);
        // After decay, new_low (0.70) should be above old_high (~0.95 * 0.717 ~ 0.681)
        assert_eq!(
            output[0].doc_id, "new_low",
            "Newer result should be ranked first"
        );
    }

    #[test]
    fn test_mixed_kinds_and_timestamps() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("recent_obs", 0.85, Some(now), Some("observation")),
            make_result(
                "old_obs",
                0.90,
                Some(now - 28 * DAY_MS),
                Some("observation"),
            ),
            make_result(
                "old_constraint",
                0.80,
                Some(now - 28 * DAY_MS),
                Some("constraint"),
            ),
            make_result("no_ts", 0.75, None, Some("observation")),
        ];
        let output = filter.apply(results);

        // recent_obs: no decay (age=0)
        let recent = output.iter().find(|r| r.doc_id == "recent_obs").unwrap();
        assert!((recent.score - 0.85).abs() < f32::EPSILON);

        // old_obs: decayed
        let old = output.iter().find(|r| r.doc_id == "old_obs").unwrap();
        assert!(old.score < 0.90);

        // old_constraint: exempt
        let constraint = output
            .iter()
            .find(|r| r.doc_id == "old_constraint")
            .unwrap();
        assert!((constraint.score - 0.80).abs() < f32::EPSILON);

        // no_ts: no penalty (fail-open)
        let no_ts = output.iter().find(|r| r.doc_id == "no_ts").unwrap();
        assert!((no_ts.score - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_max_penalty_bounded() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        // Very old result (365 days) should approach but not exceed 30%
        let results = vec![
            make_result("new", 1.0, Some(now), Some("observation")),
            make_result(
                "ancient",
                1.0,
                Some(now - 365 * DAY_MS),
                Some("observation"),
            ),
        ];
        let output = filter.apply(results);
        let ancient = output.iter().find(|r| r.doc_id == "ancient").unwrap();
        let penalty = 1.0 - ancient.score;

        // Penalty should be very close to 30% but not exceed it
        assert!(
            penalty < 0.30 + f32::EPSILON,
            "Penalty should not exceed 30%, got {:.4}%",
            penalty * 100.0
        );
        assert!(
            penalty > 0.29,
            "365-day penalty should be close to 30%, got {:.4}%",
            penalty * 100.0
        );
    }

    #[test]
    fn test_is_exempt_case_insensitive() {
        assert!(StaleFilter::is_exempt("Constraint"));
        assert!(StaleFilter::is_exempt("DEFINITION"));
        assert!(StaleFilter::is_exempt("Procedure"));
        assert!(StaleFilter::is_exempt("PREFERENCE"));
        assert!(!StaleFilter::is_exempt("observation"));
        assert!(!StaleFilter::is_exempt("unknown"));
    }

    // -- Supersession tests --

    /// Helper: create a normalized embedding vector with a specific direction.
    /// Uses a simple approach: set one dimension high, normalize.
    fn make_embedding(dim: usize, primary_axis: usize) -> Vec<f32> {
        let mut v = vec![0.1; dim];
        v[primary_axis % dim] = 10.0;
        let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter_mut().for_each(|x| *x /= mag);
        v
    }

    /// Create two nearly identical (normalized) embeddings with cosine similarity > 0.80.
    fn similar_pair(dim: usize) -> (Vec<f32>, Vec<f32>) {
        let a = make_embedding(dim, 0);
        // b is very close to a
        let mut b = a.clone();
        b[1] += 0.05;
        let mag: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        b.iter_mut().for_each(|x| *x /= mag);
        (a, b)
    }

    /// Create two dissimilar embeddings with cosine similarity < 0.80.
    fn dissimilar_pair(dim: usize) -> (Vec<f32>, Vec<f32>) {
        let a = make_embedding(dim, 0);
        let b = make_embedding(dim, dim / 2);
        (a, b)
    }

    #[test]
    fn test_supersession_marks_older_similar() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("newer", 0.9, Some(now), Some("observation")),
            make_result("older", 0.85, Some(now - DAY_MS), Some("observation")),
        ];

        let (emb_a, emb_b) = similar_pair(16);
        let mut embeddings = HashMap::new();
        embeddings.insert("newer".to_string(), emb_a);
        embeddings.insert("older".to_string(), emb_b);

        let output = filter.apply_with_supersession(results, Some(&embeddings));

        let older = output.iter().find(|r| r.doc_id == "older").unwrap();
        assert!(
            older.metadata.contains_key("superseded_by"),
            "Older similar result should be marked superseded"
        );
        assert_eq!(
            older.metadata.get("superseded_by").unwrap(),
            "newer",
            "Should be superseded by the newer result"
        );
    }

    #[test]
    fn test_supersession_no_transitivity() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        // Three results: C newest, B middle, A oldest
        // All similar to each other
        let results = vec![
            make_result("C", 0.9, Some(now), Some("observation")),
            make_result("B", 0.85, Some(now - DAY_MS), Some("observation")),
            make_result("A", 0.80, Some(now - 2 * DAY_MS), Some("observation")),
        ];

        let (emb, _) = similar_pair(16);
        let mut embeddings = HashMap::new();
        embeddings.insert("C".to_string(), emb.clone());
        embeddings.insert("B".to_string(), emb.clone());
        embeddings.insert("A".to_string(), emb);

        let output = filter.apply_with_supersession(results, Some(&embeddings));

        let a = output.iter().find(|r| r.doc_id == "A").unwrap();
        let b = output.iter().find(|r| r.doc_id == "B").unwrap();

        // A should be superseded by exactly one result (whichever newer one it finds first)
        assert!(
            a.metadata.contains_key("superseded_by"),
            "A should be superseded"
        );
        // B should also be superseded by C
        assert!(
            b.metadata.contains_key("superseded_by"),
            "B should be superseded by C"
        );

        // A is only superseded once (no double penalty)
        let a_superseder = a.metadata.get("superseded_by").unwrap();
        assert!(
            a_superseder == "C" || a_superseder == "B",
            "A should be superseded by C or B"
        );
    }

    #[test]
    fn test_supersession_exempt_kinds_skipped() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("newer_obs", 0.9, Some(now), Some("observation")),
            make_result(
                "older_constraint",
                0.85,
                Some(now - DAY_MS),
                Some("constraint"),
            ),
        ];

        let (emb_a, emb_b) = similar_pair(16);
        let mut embeddings = HashMap::new();
        embeddings.insert("newer_obs".to_string(), emb_a);
        embeddings.insert("older_constraint".to_string(), emb_b);

        let output = filter.apply_with_supersession(results, Some(&embeddings));

        let constraint = output
            .iter()
            .find(|r| r.doc_id == "older_constraint")
            .unwrap();
        assert!(
            !constraint.metadata.contains_key("superseded_by"),
            "Constraint kind should not be superseded"
        );
    }

    #[test]
    fn test_supersession_without_embeddings() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("a", 0.9, Some(now), Some("observation")),
            make_result("b", 0.8, Some(now - 14 * DAY_MS), Some("observation")),
        ];

        // None embeddings -> only time-decay applies
        let output = filter.apply_with_supersession(results, None);

        let b = output.iter().find(|r| r.doc_id == "b").unwrap();
        // Should be decayed by time but NOT superseded
        assert!(b.score < 0.8, "Should have time-decay applied");
        assert!(
            !b.metadata.contains_key("superseded_by"),
            "Should not be superseded without embeddings"
        );
    }

    #[test]
    fn test_supersession_combined_penalty() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        // Older result at 14 days ago, similar to newer
        let results = vec![
            make_result("newer", 1.0, Some(now), Some("observation")),
            make_result("older", 1.0, Some(now - 14 * DAY_MS), Some("observation")),
        ];

        let (emb_a, emb_b) = similar_pair(16);
        let mut embeddings = HashMap::new();
        embeddings.insert("newer".to_string(), emb_a);
        embeddings.insert("older".to_string(), emb_b);

        let output = filter.apply_with_supersession(results, Some(&embeddings));

        let older = output.iter().find(|r| r.doc_id == "older").unwrap();
        // Should have both time-decay AND supersession penalty
        // Time-decay at 14 days: ~0.81, then * 0.85 for supersession = ~0.689
        assert!(
            older.score < 0.75,
            "Combined penalty should reduce score significantly, got {:.4}",
            older.score
        );
        assert!(
            older.score > 0.55,
            "Score should not be excessively penalized, got {:.4}",
            older.score
        );
    }

    #[test]
    fn test_supersession_metadata_explainability() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("doc-new", 0.9, Some(now), Some("observation")),
            make_result("doc-old", 0.85, Some(now - DAY_MS), Some("observation")),
        ];

        let (emb_a, emb_b) = similar_pair(16);
        let mut embeddings = HashMap::new();
        embeddings.insert("doc-new".to_string(), emb_a);
        embeddings.insert("doc-old".to_string(), emb_b);

        let output = filter.apply_with_supersession(results, Some(&embeddings));

        let old = output.iter().find(|r| r.doc_id == "doc-old").unwrap();
        assert_eq!(
            old.metadata.get("superseded_by"),
            Some(&"doc-new".to_string()),
            "superseded_by should contain the doc_id of the superseding result"
        );
    }

    #[test]
    fn test_supersession_dissimilar_not_superseded() {
        let filter = StaleFilter::new(default_config());
        let now = 1_700_000_000_000i64;

        let results = vec![
            make_result("newer", 0.9, Some(now), Some("observation")),
            make_result("older", 0.85, Some(now - DAY_MS), Some("observation")),
        ];

        let (emb_a, emb_b) = dissimilar_pair(16);
        let mut embeddings = HashMap::new();
        embeddings.insert("newer".to_string(), emb_a);
        embeddings.insert("older".to_string(), emb_b);

        let output = filter.apply_with_supersession(results, Some(&embeddings));

        let older = output.iter().find(|r| r.doc_id == "older").unwrap();
        assert!(
            !older.metadata.contains_key("superseded_by"),
            "Dissimilar results should not be superseded"
        );
    }
}
