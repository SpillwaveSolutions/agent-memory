//! Embedding model trait and types.
//!
//! Defines the interface for generating vector embeddings from text.

use crate::error::EmbeddingError;

/// Vector embedding - a normalized float array.
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The embedding vector (normalized to unit length)
    pub values: Vec<f32>,
}

impl Embedding {
    /// Create a new embedding from a vector.
    /// Normalizes the vector to unit length.
    pub fn new(values: Vec<f32>) -> Self {
        let norm: f32 = values.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized = if norm > 0.0 {
            values.iter().map(|x| x / norm).collect()
        } else {
            values
        };
        Self { values: normalized }
    }

    /// Create embedding without normalization (for pre-normalized vectors)
    pub fn from_normalized(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.values.len()
    }

    /// Compute cosine similarity with another embedding.
    /// Returns value in [-1, 1] range (1 = identical).
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        if self.values.len() != other.values.len() {
            return 0.0;
        }
        // Since both are normalized, dot product = cosine similarity
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum()
    }
}

/// Model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model name (e.g., "all-MiniLM-L6-v2")
    pub name: String,
    /// Embedding dimension
    pub dimension: usize,
    /// Maximum sequence length in tokens
    pub max_sequence_length: usize,
}

/// Trait for embedding models.
///
/// Implementations must be thread-safe (Send + Sync) for concurrent use.
pub trait EmbeddingModel: Send + Sync {
    /// Get model information
    fn info(&self) -> &ModelInfo;

    /// Generate embedding for a single text.
    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    /// Generate embeddings for multiple texts (batch).
    /// Default implementation calls embed() for each text.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        texts.iter().map(|text| self.embed(text)).collect()
    }

    /// Generate embeddings for multiple owned strings.
    fn embed_texts(&self, texts: &[String]) -> Result<Vec<Embedding>, EmbeddingError> {
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_normalization() {
        let emb = Embedding::new(vec![3.0, 4.0]);
        // 3-4-5 triangle: normalized should be [0.6, 0.8]
        assert!((emb.values[0] - 0.6).abs() < 0.001);
        assert!((emb.values[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0]);
        let emb2 = Embedding::new(vec![1.0, 0.0, 0.0]);
        assert!((emb1.cosine_similarity(&emb2) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let emb1 = Embedding::new(vec![1.0, 0.0]);
        let emb2 = Embedding::new(vec![0.0, 1.0]);
        assert!(emb1.cosine_similarity(&emb2).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let emb1 = Embedding::new(vec![1.0, 0.0]);
        let emb2 = Embedding::new(vec![-1.0, 0.0]);
        assert!((emb1.cosine_similarity(&emb2) + 1.0).abs() < 0.001);
    }
}
