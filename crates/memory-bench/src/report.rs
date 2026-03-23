use crate::baseline::Baselines;
use crate::scorer::BenchmarkReport;

/// Serialize a benchmark report to pretty-printed JSON.
pub fn to_json(report: &BenchmarkReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

/// Generate a markdown table from a benchmark report, optionally including competitor baselines.
pub fn to_markdown(report: &BenchmarkReport, baselines: Option<&Baselines>) -> String {
    let mut out = String::new();
    out.push_str("# Benchmark Results\n\n");

    if let Some(bl) = baselines {
        out.push_str("| Metric | Agent-Memory | MemMachine | Mem0 |\n");
        out.push_str("|--------|-------------|------------|------|\n");

        let mm = bl.memmachine.as_ref();
        let m0 = bl.mem0.as_ref();

        out.push_str(&format!(
            "| Accuracy | {:.1}% | {} | {} |\n",
            report.accuracy * 100.0,
            mm.and_then(|m| m.locomo_score)
                .map_or("-".to_string(), |v| format!("{:.1}%", v * 100.0)),
            m0.and_then(|m| m.accuracy_vs_openai_memory)
                .map_or("-".to_string(), |v| format!("+{:.0}%", v * 100.0)),
        ));
        out.push_str(&format!(
            "| Recall@5 | {:.1}% | - | - |\n",
            report.recall_at_5 * 100.0
        ));
        out.push_str(&format!(
            "| Avg Tokens | {} | - | - |\n",
            report.token_usage_avg
        ));
        out.push_str(&format!(
            "| Latency p50 | {}ms | {} | {} |\n",
            report.latency_p50_ms,
            mm.and_then(|m| m.latency_improvement)
                .map_or("-".to_string(), |v| format!("{:.0}% faster", v * 100.0)),
            m0.and_then(|m| m.latency_reduction)
                .map_or("-".to_string(), |v| format!("{:.0}% reduction", v * 100.0)),
        ));
        out.push_str(&format!(
            "| Latency p95 | {}ms | - | - |\n",
            report.latency_p95_ms
        ));
        out.push_str(&format!(
            "| Compression | {:.1}% | {} | {} |\n",
            report.compression_ratio * 100.0,
            mm.and_then(|m| m.token_reduction)
                .map_or("-".to_string(), |v| format!("{:.0}%", v * 100.0)),
            m0.and_then(|m| m.token_reduction)
                .map_or("-".to_string(), |v| format!("{:.0}%", v * 100.0)),
        ));
    } else {
        out.push_str("| Metric | Value |\n");
        out.push_str("|--------|-------|\n");
        out.push_str(&format!("| Accuracy | {:.1}% |\n", report.accuracy * 100.0));
        out.push_str(&format!(
            "| Recall@5 | {:.1}% |\n",
            report.recall_at_5 * 100.0
        ));
        out.push_str(&format!("| Avg Tokens | {} |\n", report.token_usage_avg));
        out.push_str(&format!("| Latency p50 | {}ms |\n", report.latency_p50_ms));
        out.push_str(&format!("| Latency p95 | {}ms |\n", report.latency_p95_ms));
        out.push_str(&format!(
            "| Compression | {:.1}% |\n",
            report.compression_ratio * 100.0
        ));
    }

    out.push_str(&format!(
        "\n**Tests:** {}/{} passed\n",
        report.pass_count, report.test_count
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> BenchmarkReport {
        BenchmarkReport {
            accuracy: 0.85,
            recall_at_5: 0.70,
            token_usage_avg: 300,
            latency_p50_ms: 45,
            latency_p95_ms: 120,
            compression_ratio: 0.75,
            test_count: 10,
            pass_count: 8,
        }
    }

    #[test]
    fn test_to_json_roundtrips() {
        let report = sample_report();
        let json = to_json(&report);
        let parsed: BenchmarkReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, report);
    }

    #[test]
    fn test_to_markdown_contains_headers() {
        let report = sample_report();
        let md = to_markdown(&report, None);
        assert!(md.contains("Accuracy"), "Should contain Accuracy header");
        assert!(md.contains("Recall@5"), "Should contain Recall@5 header");
    }
}
