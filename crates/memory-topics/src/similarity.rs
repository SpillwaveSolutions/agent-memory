//! Vector similarity functions.
//!
//! Pure Rust implementations without external dependencies.

/// Calculate cosine similarity between two vectors.
///
/// Returns value in [-1.0, 1.0] where 1.0 = identical direction.
///
/// # Panics
/// Panics if vectors have different dimensions.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Calculate the centroid of multiple embeddings.
///
/// Returns a normalized vector representing the center of the cluster.
pub fn calculate_centroid(embeddings: &[&[f32]]) -> Vec<f32> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let dim = embeddings[0].len();
    let n = embeddings.len() as f32;
    let mut centroid = vec![0.0f32; dim];

    for embedding in embeddings {
        assert_eq!(
            embedding.len(),
            dim,
            "All embeddings must have same dimension"
        );
        for (i, &val) in embedding.iter().enumerate() {
            centroid[i] += val;
        }
    }

    // Average
    for val in centroid.iter_mut() {
        *val /= n;
    }

    // Normalize
    normalize(&mut centroid);

    centroid
}

/// Normalize a vector to unit length in place.
pub fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in v.iter_mut() {
            *val /= norm;
        }
    }
}

/// Calculate pairwise distances between embeddings.
///
/// Returns a distance matrix where distance = 1 - cosine_similarity.
pub fn pairwise_distances(embeddings: &[Vec<f32>]) -> Vec<Vec<f64>> {
    let n = embeddings.len();
    let mut distances = vec![vec![0.0f64; n]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            let sim = cosine_similarity(&embeddings[i], &embeddings[j]);
            let dist = (1.0 - sim) as f64;
            distances[i][j] = dist;
            distances[j][i] = dist;
        }
    }

    distances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_similar() {
        let a = vec![0.8, 0.6];
        let b = vec![0.6, 0.8];
        let sim = cosine_similarity(&a, &b);
        assert!(sim > 0.9); // Should be similar
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
    }

    #[test]
    fn test_calculate_centroid() {
        let e1 = vec![1.0, 0.0, 0.0];
        let e2 = vec![0.0, 1.0, 0.0];
        let embeddings: Vec<&[f32]> = vec![&e1, &e2];
        let centroid = calculate_centroid(&embeddings);
        // Average of [1,0,0] and [0,1,0] = [0.5, 0.5, 0] normalized
        let expected_norm = (0.5f32.powi(2) * 2.0).sqrt();
        assert!((centroid[0] - 0.5 / expected_norm).abs() < 0.001);
        assert!((centroid[1] - 0.5 / expected_norm).abs() < 0.001);
        assert!(centroid[2].abs() < 0.001);
    }

    #[test]
    fn test_calculate_centroid_empty() {
        let embeddings: Vec<&[f32]> = vec![];
        let centroid = calculate_centroid(&embeddings);
        assert!(centroid.is_empty());
    }

    #[test]
    fn test_calculate_centroid_single() {
        let e1 = vec![3.0, 4.0];
        let embeddings: Vec<&[f32]> = vec![&e1];
        let centroid = calculate_centroid(&embeddings);
        // Single embedding normalized: [3,4]/5 = [0.6, 0.8]
        assert!((centroid[0] - 0.6).abs() < 0.001);
        assert!((centroid[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 0.001);
        assert!((v[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let mut v = vec![0.0, 0.0];
        normalize(&mut v);
        assert!((v[0]).abs() < 0.001);
        assert!((v[1]).abs() < 0.001);
    }

    #[test]
    fn test_pairwise_distances() {
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 0.0]];
        let distances = pairwise_distances(&embeddings);
        assert!((distances[0][2]).abs() < 0.001); // Identical
        assert!((distances[0][1] - 1.0).abs() < 0.001); // Orthogonal
    }

    #[test]
    fn test_pairwise_distances_self() {
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let distances = pairwise_distances(&embeddings);
        // Self-distance should be 0
        assert!((distances[0][0]).abs() < 0.001);
        assert!((distances[1][1]).abs() < 0.001);
    }

    #[test]
    #[should_panic(expected = "Vectors must have same dimension")]
    fn test_cosine_similarity_different_dimensions() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        cosine_similarity(&a, &b);
    }
}
