//! Scoring for the custom TOML-fixture harness.
//!
//! `recall_at_k` is computed against *labeled relevant items* found in the
//! top-k ranked result texts. It is not `accuracy` under another name.

use anyhow::{Context, Result};
use std::path::Path;

/// Returns true if result text contains at least one expected string (case-insensitive).
pub fn score_result(result: &str, expected_contains: &[String]) -> bool {
    if expected_contains.is_empty() {
        return false;
    }
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

/// Recall@k against a labeled relevant-item set.
///
/// For each relevant string, count a hit if it appears (case-insensitive) in
/// any of the top-`k` ranked result texts. Divide by `|relevant|`.
///
/// Returns `None` when `relevant` is empty so the caller can omit the test
/// from the aggregate rather than reporting a fake 0.0 or 1.0.
pub fn compute_recall_at_k(ranked_texts: &[String], relevant: &[String], k: usize) -> Option<f64> {
    if relevant.is_empty() {
        return None;
    }
    let top: Vec<String> = ranked_texts
        .iter()
        .take(k)
        .map(|t| t.to_lowercase())
        .collect();
    let found = relevant
        .iter()
        .filter(|item| {
            let needle = item.to_lowercase();
            top.iter().any(|t| t.contains(&needle))
        })
        .count();
    Some(found as f64 / relevant.len() as f64)
}

/// Return the value at the given percentile from a sorted slice.
pub fn percentile(sorted_values: &[u64], p: f64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (sorted_values.len() as f64 - 1.0)).round() as usize;
    sorted_values[idx.min(sorted_values.len() - 1)]
}

/// Compression ratio: how much smaller retrieved context is vs corpus file *contents*.
///
/// `1.0 - (context_tokens / raw_tokens)`. Returns 0.0 if `raw_tokens` is 0.
pub fn compute_compression_ratio(context_tokens: usize, raw_tokens: usize) -> f64 {
    if raw_tokens == 0 {
        return 0.0;
    }
    1.0 - (context_tokens as f64 / raw_tokens as f64)
}

/// Estimate tokens from a blob of text: `ceil(chars / 4)`.
pub fn estimate_tokens_from_text(text: &str) -> usize {
    (text.len() as f64 / 4.0).ceil() as usize
}

/// Estimate raw tokens from **file contents** of the setup corpus.
///
/// v3.0 summed the character lengths of the *path strings*. That is not a
/// token estimate of the corpus. This reads each file.
pub fn estimate_raw_tokens_from_files(paths: &[impl AsRef<Path>]) -> Result<usize> {
    let mut total_chars = 0usize;
    for p in paths {
        let path = p.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading setup corpus {}", path.display()))?;
        total_chars += content.len();
    }
    Ok((total_chars as f64 / 4.0).ceil() as usize)
}

/// Aggregated custom-harness report.
///
/// `recall_at_k` is `None` when no test supplied a labeled relevant set.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub backend: String,
    pub accuracy: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recall_at_k: Option<f64>,
    pub k: usize,
    pub token_usage_avg: usize,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub compression_ratio: f64,
    pub test_count: usize,
    pub pass_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_ids: Vec<String>,
    pub caveats: Vec<String>,
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
    fn recall_at_k_is_not_accuracy() {
        // Accuracy would be 1.0 (the query "hit" because JWT is present).
        // Recall@5 against three labeled items of which only one appears is 1/3.
        let ranked = vec![
            "We chose JWT with refresh rotation".to_string(),
            "Unrelated deploy notes".to_string(),
        ];
        let relevant = vec!["JWT".to_string(), "OAuth2".to_string(), "SAML".to_string()];
        let recall = compute_recall_at_k(&ranked, &relevant, 5).unwrap();
        assert!((recall - 1.0 / 3.0).abs() < 0.001);
        assert!(score_result(&ranked.join("\n"), &["JWT".to_string()]));
    }

    #[test]
    fn recall_at_k_empty_relevant_is_none() {
        assert!(compute_recall_at_k(&["anything".into()], &[], 5).is_none());
    }

    #[test]
    fn recall_at_k_respects_k_cutoff() {
        let ranked = vec![
            "alpha appears here".to_string(),
            "beta appears here".to_string(),
            "gamma appears here".to_string(),
        ];
        let relevant = vec!["gamma".to_string()];
        let at_2 = compute_recall_at_k(&ranked, &relevant, 2).unwrap();
        let at_3 = compute_recall_at_k(&ranked, &relevant, 3).unwrap();
        assert!((at_2 - 0.0).abs() < f64::EPSILON);
        assert!((at_3 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn estimate_raw_tokens_reads_file_contents_not_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corpus.jsonl");
        // 40 chars of content. Path string is much shorter.
        let content = "abcdefghijklmnopqrstuvwxyzabcdefghijklmn";
        assert_eq!(content.len(), 40);
        std::fs::write(&path, content).unwrap();
        let from_contents = estimate_raw_tokens_from_files(&[path.as_path()]).unwrap();
        let from_path_string = estimate_tokens_from_text(path.to_str().unwrap());
        assert_eq!(from_contents, 10); // ceil(40/4)
        assert_ne!(
            from_contents, from_path_string,
            "path-length estimate must not equal content estimate"
        );
    }
}
