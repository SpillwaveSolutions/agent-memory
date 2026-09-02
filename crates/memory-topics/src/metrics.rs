//! Clustering quality metrics: purity and adjusted rand index.
//!
//! These measure a predicted labeling against gold labels. They do not
//! know about embeddings or HDBSCAN; callers pass integer cluster ids.

use std::collections::HashMap;

/// Cluster purity: for each predicted cluster, take the majority gold
/// label count, sum, divide by `n`. Range `[0, 1]`; 1 is perfect.
///
/// `pred` and `gold` must be the same length. Empty input returns 1.0.
pub fn purity(pred: &[i32], gold: &[i32]) -> f64 {
    assert_eq!(
        pred.len(),
        gold.len(),
        "pred and gold must be the same length"
    );
    let n = pred.len();
    if n == 0 {
        return 1.0;
    }
    let mut clusters: HashMap<i32, HashMap<i32, usize>> = HashMap::new();
    for (&p, &g) in pred.iter().zip(gold.iter()) {
        *clusters.entry(p).or_default().entry(g).or_insert(0) += 1;
    }
    let majority_sum: usize = clusters
        .values()
        .map(|counts| counts.values().copied().max().unwrap_or(0))
        .sum();
    majority_sum as f64 / n as f64
}

/// Adjusted Rand Index. Chance-adjusted pairwise agreement.
/// Range roughly `[-1, 1]`; 1 is perfect, 0 is random.
///
/// Empty or singleton input returns 1.0 (undefined, treated as agreement).
pub fn adjusted_rand_index(pred: &[i32], gold: &[i32]) -> f64 {
    assert_eq!(
        pred.len(),
        gold.len(),
        "pred and gold must be the same length"
    );
    let n = pred.len();
    if n < 2 {
        return 1.0;
    }

    let mut pred_ids: Vec<i32> = pred.to_vec();
    pred_ids.sort_unstable();
    pred_ids.dedup();
    let mut gold_ids: Vec<i32> = gold.to_vec();
    gold_ids.sort_unstable();
    gold_ids.dedup();

    let p_index: HashMap<i32, usize> = pred_ids.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let g_index: HashMap<i32, usize> = gold_ids.iter().enumerate().map(|(i, &v)| (v, i)).collect();

    let mut table = vec![vec![0usize; gold_ids.len()]; pred_ids.len()];
    for (&p, &g) in pred.iter().zip(gold.iter()) {
        table[p_index[&p]][g_index[&g]] += 1;
    }

    let comb2 = |x: usize| -> f64 {
        if x < 2 {
            0.0
        } else {
            (x * (x - 1)) as f64 / 2.0
        }
    };

    let mut index = 0.0;
    let mut row_comb = 0.0;
    let mut col_comb = 0.0;
    for row in &table {
        let row_sum: usize = row.iter().sum();
        row_comb += comb2(row_sum);
        for &cell in row {
            index += comb2(cell);
        }
    }
    for j in 0..gold_ids.len() {
        let col_sum: usize = table.iter().map(|row| row[j]).sum();
        col_comb += comb2(col_sum);
    }

    let total_pairs = comb2(n);
    if total_pairs == 0.0 {
        return 1.0;
    }
    let expected = row_comb * col_comb / total_pairs;
    let max = 0.5 * (row_comb + col_comb);
    if (max - expected).abs() < 1e-12 {
        return 1.0;
    }
    (index - expected) / (max - expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-computed 3-cluster example used as the QUAL-02 unit fixture:
    //
    //   gold: 0 0 0  1 1 1  2 2 2
    //   pred: 0 0 1  1 1 1  2 2 2
    //
    // Contingency:
    //        g0 g1 g2     row
    //   p0    2  0  0       2
    //   p1    1  3  0       4
    //   p2    0  0  3       3
    //   col   3  3  3
    //
    // Purity: majority per pred cluster = 2 + 3 + 3 = 8; 8/9.
    //
    // C(n,2) = n(n-1)/2
    //   index   = C(2,2 pairs)=1 + C(3,2)=3 + C(3,2)=3 = 7
    //   row     = C(2,2p)=1 + C(4,2)=6 + C(3,2)=3 = 10
    //   col     = 3 * C(3,2) = 9
    //   total   = C(9,2) = 36
    //   expected = 10*9/36 = 2.5
    //   max      = 0.5*(10+9) = 9.5
    //   ARI      = (7-2.5)/(9.5-2.5) = 4.5/7 ≈ 0.642857142857

    const GOLD: [i32; 9] = [0, 0, 0, 1, 1, 1, 2, 2, 2];
    const PRED: [i32; 9] = [0, 0, 1, 1, 1, 1, 2, 2, 2];

    #[test]
    fn purity_hand_computed_three_cluster() {
        let p = purity(&PRED, &GOLD);
        assert!((p - 8.0 / 9.0).abs() < 1e-12, "purity={p}");
    }

    #[test]
    fn ari_hand_computed_three_cluster() {
        let a = adjusted_rand_index(&PRED, &GOLD);
        assert!(
            (a - 4.5 / 7.0).abs() < 1e-12,
            "ARI={a}, expected {}",
            4.5 / 7.0
        );
    }

    #[test]
    fn perfect_agreement_is_one() {
        let labels = [0, 0, 1, 1, 2, 2];
        assert!((purity(&labels, &labels) - 1.0).abs() < 1e-12);
        assert!((adjusted_rand_index(&labels, &labels) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn empty_is_one() {
        assert_eq!(purity(&[], &[]), 1.0);
        assert_eq!(adjusted_rand_index(&[], &[]), 1.0);
    }

    #[test]
    fn singleton_ari_is_one() {
        assert_eq!(adjusted_rand_index(&[7], &[3]), 1.0);
        assert_eq!(purity(&[7], &[3]), 1.0);
    }
}
