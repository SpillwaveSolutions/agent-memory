//! Labelled-corpus evaluation of `TopicExtractor::cluster` (QUAL-02).
//!
//! Vectors are capped TF-IDF, not Candle embeddings. The corpus is synthetic
//! (80 short documents, 8 known clusters) so HDBSCAN has a known gold labeling.

use serde::Serialize;

use crate::config::ExtractionConfig;
use crate::error::TopicsError;
use crate::extraction::{NodeEmbedding, TopicExtractor};
use crate::metrics::{adjusted_rand_index, purity};
use crate::tfidf::TfIdf;

/// Report written to `benchmarks/results/topics-quality.json`.
#[derive(Debug, Clone, Serialize)]
pub struct TopicQualityReport {
    pub n_docs: usize,
    pub n_gold_clusters: usize,
    pub n_pred_clusters: usize,
    pub n_noise: usize,
    pub min_cluster_size: usize,
    pub embedding: String,
    pub purity: f64,
    pub adjusted_rand_index: f64,
    pub caveats: Vec<String>,
}

/// Eight clusters × ten short documents. Keywords are cluster-private;
/// shared filler ("team", "discussed") is low-IDF on purpose.
pub fn labelled_topic_corpus() -> (Vec<String>, Vec<i32>) {
    const CLUSTERS: &[(&str, &[&str])] = &[
        (
            "kubernetes",
            &[
                "helm",
                "ingress",
                "sidecar",
                "replica",
                "chart",
                "canary",
                "controller",
                "surge",
            ],
        ),
        (
            "postgres",
            &[
                "vacuum",
                "wal",
                "deadlock",
                "tablespace",
                "autovacuum",
                "bloat",
                "hotstandby",
                "checkpoint",
            ],
        ),
        (
            "oauth",
            &[
                "oidc",
                "refresh",
                "claims",
                "issuer",
                "scopes",
                "introspection",
                "pkce",
                "audience",
            ],
        ),
        (
            "terraform",
            &[
                "hcl",
                "provider",
                "statelock",
                "workspace",
                "module",
                "planfile",
                "backend",
                "apply",
            ],
        ),
        (
            "prometheus",
            &[
                "grafana",
                "alertmanager",
                "histogram",
                "scrape",
                "promql",
                "recording",
                "exporters",
                "rules",
            ],
        ),
        (
            "kafka",
            &[
                "partition",
                "consumer",
                "offset",
                "broker",
                "rebalance",
                "compaction",
                "isr",
                "topiclog",
            ],
        ),
        (
            "rustlang",
            &[
                "ownership",
                "borrow",
                "lifetime",
                "clippy",
                "cargo",
                "unsafe",
                "traitbound",
                "pinning",
            ],
        ),
        (
            "incident",
            &[
                "pager",
                "runbook",
                "postmortem",
                "sevone",
                "handoff",
                "warroom",
                "timeline",
                "blameless",
            ],
        ),
    ];

    let mut docs = Vec::new();
    let mut gold = Vec::new();
    for (ci, (name, kws)) in CLUSTERS.iter().enumerate() {
        for i in 0..10 {
            let a = kws[i % kws.len()];
            let b = kws[(i + 1) % kws.len()];
            let c = kws[(i + 2) % kws.len()];
            let d = kws[(i + 3) % kws.len()];
            let e = kws[(i + 4) % kws.len()];
            let text = format!(
                "{name} topic {i}: the team discussed {a} and {b} while {c} met {d}; \
                 later {e} came up again with {a} {b} {c}."
            );
            docs.push(text);
            gold.push(ci as i32);
        }
    }
    (docs, gold)
}

/// Cluster the labelled corpus via `TopicExtractor::cluster` on capped TF-IDF.
pub fn evaluate_labelled_corpus(
    min_cluster_size: usize,
) -> Result<TopicQualityReport, TopicsError> {
    let (docs, gold) = labelled_topic_corpus();
    let refs: Vec<&str> = docs.iter().map(String::as_str).collect();
    let tfidf = TfIdf::new(&refs);
    let vectors = tfidf.document_vectors_capped(&refs, 32);

    let nodes: Vec<NodeEmbedding> = docs
        .iter()
        .enumerate()
        .map(|(i, summary)| NodeEmbedding {
            node_id: i.to_string(),
            embedding: vectors[i].clone(),
            summary: summary.clone(),
        })
        .collect();

    let extractor = TopicExtractor::new(ExtractionConfig {
        min_cluster_size,
        ..Default::default()
    });
    let clusters = extractor.cluster(&nodes)?;

    let mut pred = vec![-1i32; docs.len()];
    for cluster in &clusters {
        for id in &cluster.node_ids {
            if let Ok(idx) = id.parse::<usize>() {
                if idx < pred.len() {
                    pred[idx] = cluster.label;
                }
            }
        }
    }

    let n_noise = pred.iter().filter(|&&l| l == -1).count();
    let mut pred_ids = pred.clone();
    pred_ids.retain(|&l| l >= 0);
    pred_ids.sort_unstable();
    pred_ids.dedup();

    Ok(TopicQualityReport {
        n_docs: docs.len(),
        n_gold_clusters: 8,
        n_pred_clusters: pred_ids.len(),
        n_noise,
        min_cluster_size,
        embedding: "tfidf-l2-top32".into(),
        purity: purity(&pred, &gold),
        adjusted_rand_index: adjusted_rand_index(&pred, &gold),
        caveats: vec![
            "synthetic 80-doc / 8-cluster corpus, not live TOC summaries".into(),
            "vectors are capped TF-IDF (top 32 terms), not Candle embeddings".into(),
            "cluster() is HDBSCAN via TopicExtractor; noise label is -1".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labelled_corpus_is_eighty_docs_eight_clusters() {
        let (docs, gold) = labelled_topic_corpus();
        assert_eq!(docs.len(), 80);
        assert_eq!(gold.len(), 80);
        let mut ids = gold.clone();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids, (0..8).collect::<Vec<i32>>());
    }

    #[test]
    fn cluster_quality_beats_chance() {
        let report = evaluate_labelled_corpus(5).expect("cluster");
        assert_eq!(report.n_docs, 80);
        assert!(
            report.purity >= 0.6,
            "purity={} (need ≥0.6 on this synthetic set)",
            report.purity
        );
        assert!(
            report.adjusted_rand_index >= 0.4,
            "ARI={} (need ≥0.4 on this synthetic set)",
            report.adjusted_rand_index
        );
    }
}
