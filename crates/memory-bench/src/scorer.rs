/// Returns true if result text contains at least one expected string (case-insensitive).
pub fn score_result(result: &str, expected_contains: &[String]) -> bool {
    let lower = result.to_lowercase();
    expected_contains
        .iter()
        .any(|e| lower.contains(&e.to_lowercase()))
}

/// Compute accuracy as fraction of hits that are true.
pub fn compute_accuracy(hits: &[bool]) -> f64 {
    if hits.is_empty() {
        return 0.0;
    }
    hits.iter().filter(|&&h| h).count() as f64 / hits.len() as f64
}

/// Compute recall@k: fraction of relevant items found in top-k results.
pub fn compute_recall_at_k(hits_in_top_k: &[bool], total_relevant: usize) -> f64 {
    if total_relevant == 0 {
        return 0.0;
    }
    hits_in_top_k.iter().filter(|&&h| h).count() as f64 / total_relevant as f64
}

/// Return the value at the given percentile from a sorted slice.
pub fn percentile(sorted_values: &[u64], p: f64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (sorted_values.len() as f64 - 1.0)).round() as usize;
    sorted_values[idx.min(sorted_values.len() - 1)]
}

/// Compute compression ratio: how much smaller the context_tokens are compared to raw input.
///
/// Formula: `1.0 - (context_tokens as f64 / raw_tokens as f64)`
///
/// - `context_tokens`: tokens_estimated returned by the memory search JSON envelope.
/// - `raw_tokens`: derived by counting total characters across all JSONL setup lines,
///   divided by 4.0 as a standard chars-per-token approximation.
///
/// Returns 0.0 if raw_tokens is 0 (prevents divide-by-zero).
pub fn compute_compression_ratio(context_tokens: usize, raw_tokens: usize) -> f64 {
    if raw_tokens == 0 {
        return 0.0;
    }
    1.0 - (context_tokens as f64 / raw_tokens as f64)
}

/// Estimate raw token count from JSONL setup strings (TestCase.setup lines).
/// Sums character lengths of all setup strings and divides by 4 (chars-per-token approximation).
pub fn estimate_raw_tokens(setup_lines: &[String]) -> usize {
    let total_chars: usize = setup_lines.iter().map(|s| s.len()).sum();
    (total_chars as f64 / 4.0).ceil() as usize
}

/// Aggregated benchmark report with all computed metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub accuracy: f64,
    pub recall_at_5: f64,
    pub token_usage_avg: usize,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub compression_ratio: f64,
    pub test_count: usize,
    pub pass_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_hit_when_expected_present() {
        assert!(score_result(
            "We chose JWT for stateless auth",
            &["JWT".to_string()]
        ));
    }

    #[test]
    fn test_score_miss_when_none_present() {
        assert!(!score_result(
            "We chose sessions with cookies",
            &["JWT".to_string()]
        ));
    }

    #[test]
    fn test_score_case_insensitive() {
        assert!(score_result("JWT tokens are great", &["jwt".to_string()]));
    }

    #[test]
    fn test_accuracy_all_hits() {
        let hits = vec![true, true, true];
        assert!((compute_accuracy(&hits) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_accuracy_partial() {
        let hits = vec![true, false, true];
        let acc = compute_accuracy(&hits);
        assert!((acc - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_accuracy_empty() {
        assert!((compute_accuracy(&[]) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_percentile_p50() {
        let values = vec![10, 20, 30, 40, 50];
        assert_eq!(percentile(&values, 50.0), 30);
    }

    #[test]
    fn test_percentile_p95() {
        let values = vec![10, 20, 30, 40, 50];
        assert_eq!(percentile(&values, 95.0), 50);
    }

    #[test]
    fn test_compression_ratio_typical() {
        let ratio = compute_compression_ratio(250, 1000);
        assert!((ratio - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compression_ratio_zero_raw() {
        assert!((compute_compression_ratio(100, 0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_raw_tokens() {
        let lines = vec!["hello world".to_string()];
        // ceil(11/4) = 3
        assert_eq!(estimate_raw_tokens(&lines), 3);
    }
}
