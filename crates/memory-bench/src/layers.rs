//! Retrieval-layer switch for the custom harness (BENCH-13).
//!
//! Mock BM25 is token overlap. Mock vector is lexicon-expanded TF-IDF cosine.
//! Mock hybrid is RRF (k=60) of the two lists. CLI backend always calls
//! `memory search` (RouteQuery / hybrid); `--layers` is a mock-backend switch.

use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};

use crate::lexicon;

/// Which retrieval layer the custom harness should drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalLayer {
    Bm25,
    Vector,
    Hybrid,
}

impl RetrievalLayer {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "bm25" => Ok(Self::Bm25),
            "vector" => Ok(Self::Vector),
            "hybrid" => Ok(Self::Hybrid),
            other => bail!("unknown layers '{other}' (expected bm25|vector|hybrid)"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bm25 => "bm25",
            Self::Vector => "vector",
            Self::Hybrid => "hybrid",
        }
    }
}

/// Rank `docs` for `query` under `layer`. Returns `(score, doc_index)` desc.
pub fn rank(docs: &[String], query: &str, layer: RetrievalLayer) -> Vec<(f64, usize)> {
    match layer {
        RetrievalLayer::Bm25 => bm25_rank(docs, query),
        RetrievalLayer::Vector => vector_rank(docs, query),
        RetrievalLayer::Hybrid => {
            let a = bm25_rank(docs, query);
            let b = vector_rank(docs, query);
            rrf_merge(&[&a, &b], 60.0, docs.len())
        }
    }
}

fn tokenize(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(|t| t.to_lowercase())
        .collect()
}

fn bm25_rank(docs: &[String], query: &str) -> Vec<(f64, usize)> {
    let terms = tokenize(query);
    let mut scored: Vec<(f64, usize)> = docs
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let hay = d.to_lowercase();
            let score = terms.iter().filter(|t| hay.contains(t.as_str())).count() as f64;
            (score, i)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn vector_rank(docs: &[String], query: &str) -> Vec<(f64, usize)> {
    let q = lexicon::expand(query);
    let expanded: Vec<String> = docs.iter().map(|d| lexicon::expand(d)).collect();
    tfidf_cosine_rank(&expanded, &q)
}

fn tfidf_cosine_rank(docs: &[String], query: &str) -> Vec<(f64, usize)> {
    let q_toks = tokenize(query);
    let doc_toks: Vec<Vec<String>> = docs.iter().map(|d| tokenize(d)).collect();
    let mut df: HashMap<String, usize> = HashMap::new();
    for toks in &doc_toks {
        let uniq: HashSet<&String> = toks.iter().collect();
        for t in uniq {
            *df.entry(t.clone()).or_insert(0) += 1;
        }
    }
    let n = docs.len() as f64;
    let idf = |t: &str| -> f64 {
        let d = *df.get(t).unwrap_or(&0) as f64;
        if d == 0.0 {
            0.0
        } else {
            ((n + 1.0) / (d + 1.0)).ln() + 1.0
        }
    };
    let vec_of = |toks: &[String]| -> HashMap<String, f64> {
        let mut tf: HashMap<String, usize> = HashMap::new();
        for t in toks {
            *tf.entry(t.clone()).or_insert(0) += 1;
        }
        let len = toks.len().max(1) as f64;
        tf.into_iter()
            .map(|(t, c)| {
                let v = (c as f64 / len) * idf(&t);
                (t, v)
            })
            .collect()
    };
    let cosine = |a: &HashMap<String, f64>, b: &HashMap<String, f64>| -> f64 {
        let mut dot = 0.0;
        for (t, av) in a {
            if let Some(bv) = b.get(t) {
                dot += av * bv;
            }
        }
        let na = a.values().map(|x| x * x).sum::<f64>().sqrt();
        let nb = b.values().map(|x| x * x).sum::<f64>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    };
    let qv = vec_of(&q_toks);
    let mut scored: Vec<(f64, usize)> = doc_toks
        .iter()
        .enumerate()
        .map(|(i, toks)| (cosine(&qv, &vec_of(toks)), i))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn rrf_merge(lists: &[&Vec<(f64, usize)>], k_rrf: f64, n_docs: usize) -> Vec<(f64, usize)> {
    let mut acc = vec![0.0; n_docs];
    for list in lists {
        for (rank, (_score, idx)) in list.iter().enumerate() {
            acc[*idx] += 1.0 / (k_rrf + rank as f64 + 1.0);
        }
    }
    let mut out: Vec<(f64, usize)> = acc.into_iter().enumerate().map(|(i, s)| (s, i)).collect();
    out.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (Vec<String>, &'static str) {
        let docs = vec![
            "We set JWT lifetime to fifteen minutes with rotating refresh credentials.".into(),
            "The cafeteria token of appreciation, and the HR policy, have an expiry of one year."
                .into(),
            "Parking tokens and the visitor policy share an expiry date in June.".into(),
            "A gift-token scheme, a refund policy, and milk expiry in the fridge.".into(),
            "Token booths, a museum policy binder, and the expiry of a coupon.".into(),
            "The policy on lunch tokens ignores expiry of dessert vouchers.".into(),
            "Office tokens for the printer sit under a policy with no expiry at all.".into(),
        ];
        (docs, "token expiry policy")
    }

    #[test]
    fn parse_layers() {
        assert_eq!(RetrievalLayer::parse("bm25").unwrap(), RetrievalLayer::Bm25);
        assert_eq!(
            RetrievalLayer::parse("vector").unwrap(),
            RetrievalLayer::Vector
        );
        assert_eq!(
            RetrievalLayer::parse("hybrid").unwrap(),
            RetrievalLayer::Hybrid
        );
        assert!(RetrievalLayer::parse("ann").is_err());
    }

    #[test]
    fn bm25_prefers_lexical_distractor() {
        let (docs, q) = pair();
        let ranked = rank(&docs, q, RetrievalLayer::Bm25);
        assert_eq!(ranked[0].1, 1, "top BM25 hit should be a distractor");
        let top5: Vec<usize> = ranked.iter().take(5).map(|(_, i)| *i).collect();
        assert!(!top5.contains(&0), "relevant doc must not be in BM25 top-5");
    }

    #[test]
    fn vector_prefers_paraphrase() {
        let (docs, q) = pair();
        let ranked = rank(&docs, q, RetrievalLayer::Vector);
        assert_eq!(ranked[0].1, 0, "vector top hit should be the paraphrase");
    }

    #[test]
    fn hybrid_puts_paraphrase_in_top5() {
        let (docs, q) = pair();
        let ranked = rank(&docs, q, RetrievalLayer::Hybrid);
        let top5: Vec<usize> = ranked.iter().take(5).map(|(_, i)| *i).collect();
        assert!(
            top5.contains(&0),
            "hybrid top-5 must include the paraphrase"
        );
    }
}
