//! Combined ranking formula for retrieval results.
//!
//! Applies salience boosting and usage decay to search results.
//!
//! ## Formula
//!
//! ```text
//! salience_factor = 0.55 + 0.45 * salience_score
//! usage_penalty   = 1.0 / (1.0 + decay_factor * access_count)
//! combined_score  = similarity * salience_factor * usage_penalty
//! final_score     = max(combined_score, similarity * 0.50)  // 50% floor
//! ```

use crate::executor::SearchResult;

/// Configuration for combined ranking.
#[derive(Debug, Clone)]
pub struct RankingConfig {
    /// Whether salience boosting is enabled.
    pub salience_enabled: bool,
    /// Whether usage decay is enabled.
    pub usage_decay_enabled: bool,
    /// Decay factor for usage penalty (higher = more aggressive).
    pub decay_factor: f32,
    /// Minimum score floor as fraction of original similarity (0.0-1.0).
    pub score_floor: f32,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            salience_enabled: true,
            usage_decay_enabled: false, // Off by default until validated
            decay_factor: 0.15,
            score_floor: 0.50,
        }
    }
}

/// Applies combined ranking formula to search results.
///
/// Reads `salience_score` and `access_count` from result metadata.
/// Re-sorts results by adjusted score after applying the formula.
pub fn apply_combined_ranking(
    mut results: Vec<SearchResult>,
    config: &RankingConfig,
) -> Vec<SearchResult> {
    if results.is_empty() {
        return results;
    }

    for result in &mut results {
        let original_score = result.score;

        // Salience factor: 0.55 + 0.45 * salience_score
        let salience_factor = if config.salience_enabled {
            let salience_score: f32 = result
                .metadata
                .get("salience_score")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5); // Default neutral
            0.55 + 0.45 * salience_score
        } else {
            1.0
        };

        // Usage penalty: 1 / (1 + decay_factor * access_count)
        let usage_penalty = if config.usage_decay_enabled {
            let access_count: u32 = result
                .metadata
                .get("access_count")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            1.0 / (1.0 + config.decay_factor * access_count as f32)
        } else {
            1.0
        };

        // Combined score with floor
        let combined = original_score * salience_factor * usage_penalty;
        let floor = original_score * config.score_floor;
        result.score = combined.max(floor);
    }

    // Re-sort by adjusted score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::types::RetrievalLayer;

    fn make_result(doc_id: &str, score: f32, salience: f32, access_count: u32) -> SearchResult {
        let mut metadata = HashMap::new();
        metadata.insert("salience_score".to_string(), salience.to_string());
        metadata.insert("access_count".to_string(), access_count.to_string());
        SearchResult {
            doc_id: doc_id.to_string(),
            doc_type: "toc_node".to_string(),
            score,
            text_preview: format!("Preview for {doc_id}"),
            source_layer: RetrievalLayer::BM25,
            metadata,
        }
    }

    #[test]
    fn test_empty_results() {
        let config = RankingConfig::default();
        let results = apply_combined_ranking(vec![], &config);
        assert!(results.is_empty());
    }

    #[test]
    fn test_salience_boost() {
        let config = RankingConfig {
            salience_enabled: true,
            usage_decay_enabled: false,
            ..Default::default()
        };

        let results = vec![
            make_result("high_sal", 0.8, 1.0, 0), // salience_factor = 0.55 + 0.45 = 1.0
            make_result("low_sal", 0.8, 0.0, 0),  // salience_factor = 0.55
            make_result("mid_sal", 0.8, 0.5, 0),  // salience_factor = 0.55 + 0.225 = 0.775
        ];

        let ranked = apply_combined_ranking(results, &config);

        assert_eq!(ranked[0].doc_id, "high_sal");
        assert_eq!(ranked[1].doc_id, "mid_sal");
        assert_eq!(ranked[2].doc_id, "low_sal");
    }

    #[test]
    fn test_usage_decay() {
        let config = RankingConfig {
            salience_enabled: false,
            usage_decay_enabled: true,
            decay_factor: 0.15,
            ..Default::default()
        };

        let results = vec![
            make_result("fresh", 0.8, 0.5, 0),    // penalty = 1.0
            make_result("used_1", 0.8, 0.5, 5),   // penalty = 1/(1+0.75) = 0.571
            make_result("used_10", 0.8, 0.5, 10), // penalty = 1/(1+1.5) = 0.4
        ];

        let ranked = apply_combined_ranking(results, &config);

        assert_eq!(ranked[0].doc_id, "fresh");
        assert_eq!(ranked[1].doc_id, "used_1");
        assert_eq!(ranked[2].doc_id, "used_10");
    }

    #[test]
    fn test_score_floor_prevents_collapse() {
        let config = RankingConfig {
            salience_enabled: true,
            usage_decay_enabled: true,
            decay_factor: 0.15,
            score_floor: 0.50,
        };

        // Very low salience + high usage: combined would be very low
        // but floor prevents collapse
        let results = vec![make_result("heavily_used", 0.9, 0.0, 100)];

        let ranked = apply_combined_ranking(results, &config);

        // Floor = 0.9 * 0.50 = 0.45
        // Combined = 0.9 * 0.55 * (1/16) = 0.031 -> floored to 0.45
        assert!(
            ranked[0].score >= 0.44,
            "Score should be at or above floor, got {}",
            ranked[0].score
        );
    }

    #[test]
    fn test_combined_formula() {
        let config = RankingConfig {
            salience_enabled: true,
            usage_decay_enabled: true,
            decay_factor: 0.15,
            score_floor: 0.50,
        };

        let results = vec![make_result("test", 0.8, 0.7, 3)];
        // salience_factor = 0.55 + 0.45 * 0.7 = 0.55 + 0.315 = 0.865
        // usage_penalty = 1 / (1 + 0.15 * 3) = 1 / 1.45 = 0.6897
        // combined = 0.8 * 0.865 * 0.6897 = 0.477
        // floor = 0.8 * 0.50 = 0.4
        // final = max(0.477, 0.4) = 0.477

        let ranked = apply_combined_ranking(results, &config);
        assert!(
            (ranked[0].score - 0.477).abs() < 0.01,
            "Expected ~0.477, got {}",
            ranked[0].score
        );
    }

    #[test]
    fn test_disabled_passthrough() {
        let config = RankingConfig {
            salience_enabled: false,
            usage_decay_enabled: false,
            ..Default::default()
        };

        let results = vec![make_result("test", 0.8, 1.0, 100)];
        let ranked = apply_combined_ranking(results, &config);

        // Both disabled, score should be unchanged
        assert!(
            (ranked[0].score - 0.8).abs() < f32::EPSILON,
            "Score should be unchanged when both disabled"
        );
    }
}
