use crate::baseline::Baselines;
use crate::scorer::BenchmarkReport;

/// Serialize a benchmark report to pretty-printed JSON.
pub fn to_json(report: &BenchmarkReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

/// Markdown report. Competitor rows never share a unified "Accuracy" column
/// across incommensurable measurement regimes.
pub fn to_markdown(report: &BenchmarkReport, baselines: Option<&Baselines>) -> String {
    let mut out = String::new();
    out.push_str("# Custom harness results\n\n");
    out.push_str(&format!("**Backend:** {}\n\n", report.backend));

    out.push_str("| Metric | Value | Notes |\n");
    out.push_str("|--------|-------|-------|\n");
    out.push_str(&format!(
        "| expected_contains accuracy | {:.1}% | this fixture suite only |\n",
        report.accuracy * 100.0
    ));
    match report.recall_at_k {
        Some(r) => out.push_str(&format!(
            "| recall@{} | {:.1}% | labeled relevant items in top-k |\n",
            report.k,
            r * 100.0
        )),
        None => out.push_str(&format!(
            "| recall@{} | n/a | no test supplied a relevant set |\n",
            report.k
        )),
    }
    out.push_str(&format!(
        "| avg retrieved tokens | {} | envelope / mock estimate |\n",
        report.token_usage_avg
    ));
    out.push_str(&format!(
        "| latency p50 | {}ms | |\n",
        report.latency_p50_ms
    ));
    out.push_str(&format!(
        "| latency p95 | {}ms | |\n",
        report.latency_p95_ms
    ));
    out.push_str(&format!(
        "| compression (1 - ctx/raw) | {:.1}% | raw = setup file *contents* |\n",
        report.compression_ratio * 100.0
    ));

    out.push_str(&format!(
        "\n**Tests:** {}/{} passed\n",
        report.pass_count, report.test_count
    ));
    if !report.failed_ids.is_empty() {
        out.push_str(&format!("**Failed:** {}\n", report.failed_ids.join(", ")));
    }

    if !report.caveats.is_empty() {
        out.push_str("\n## Caveats\n\n");
        for c in &report.caveats {
            out.push_str(&format!("- {c}\n"));
        }
    }

    if let Some(bl) = baselines {
        out.push_str("\n## Published competitor numbers (NOT the same metric)\n\n");
        out.push_str("| System | Metric | Value | Commensurable with this row? |\n");
        out.push_str("|--------|--------|-------|------------------------------|\n");
        out.push_str(&format!(
            "| Agent-Memory custom harness | expected_contains accuracy | {:.1}% | this suite only |\n",
            report.accuracy * 100.0
        ));
        if let Some(mm) = bl.memmachine.as_ref() {
            if let Some(v) = mm.locomo_score {
                out.push_str(&format!(
                    "| MemMachine | {} | {:.1}% | no — different dataset, judge, and protocol |\n",
                    mm.metric
                        .as_deref()
                        .unwrap_or("LOCOMO LLM-judge (their paper)"),
                    v * 100.0
                ));
            }
        }
        if let Some(m0) = bl.mem0.as_ref() {
            if let Some(v) = m0.accuracy_vs_openai_memory {
                out.push_str(&format!(
                    "| Mem0 | {} | +{:.0}% | no — relative delta vs OpenAI memory, not LOCOMO |\n",
                    m0.metric
                        .as_deref()
                        .unwrap_or("relative delta vs OpenAI memory"),
                    v * 100.0
                ));
            }
        }
        out.push_str(
            "\nIncommensurable metrics never share a unified Accuracy column. \
             A LOCOMO LLM-judge number belongs next to other LOCOMO LLM-judge numbers only.\n",
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::CompetitorScore;

    fn sample_report() -> BenchmarkReport {
        BenchmarkReport {
            backend: "mock".into(),
            accuracy: 0.85,
            recall_at_k: Some(0.70),
            k: 5,
            token_usage_avg: 300,
            latency_p50_ms: 45,
            latency_p95_ms: 120,
            compression_ratio: 0.75,
            test_count: 10,
            pass_count: 8,
            failed_ids: vec![],
            caveats: vec!["mock retrieval".into()],
            layers: "bm25".into(),
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
        assert!(md.contains("expected_contains accuracy"));
        assert!(md.contains("recall@5"));
        assert!(!md.contains("Recall@5 | 0.70"), "old unified column gone");
    }

    #[test]
    fn compare_table_does_not_unify_accuracy() {
        let report = sample_report();
        let bl = Baselines {
            memmachine: Some(CompetitorScore {
                locomo_score: Some(0.91),
                token_reduction: None,
                latency_improvement: None,
                accuracy_vs_openai_memory: None,
                latency_reduction: None,
                metric: Some("LOCOMO LLM-judge".into()),
            }),
            mem0: Some(CompetitorScore {
                locomo_score: None,
                token_reduction: None,
                latency_improvement: None,
                accuracy_vs_openai_memory: Some(0.26),
                latency_reduction: None,
                metric: Some("relative delta vs OpenAI memory".into()),
            }),
        };
        let md = to_markdown(&report, Some(&bl));
        assert!(md.contains("NOT the same metric"));
        assert!(md.contains("LOCOMO LLM-judge"));
        assert!(md.contains("relative delta vs OpenAI memory"));
        assert!(
            !md.contains("| Accuracy |"),
            "must not emit a unified Accuracy column: {md}"
        );
    }
}
