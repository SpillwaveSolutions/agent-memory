//! Candle-based embedding implementation.
//!
//! Uses all-MiniLM-L6-v2 for 384-dimensional embeddings.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use tokenizers::Tokenizer;
use tracing::{debug, info};

use crate::cache::{get_or_download_model, ModelCache};
use crate::error::EmbeddingError;
use crate::model::{Embedding, EmbeddingModel, ModelInfo};

/// Embedding dimension for all-MiniLM-L6-v2
pub const EMBEDDING_DIM: usize = 384;

/// Maximum sequence length
pub const MAX_SEQ_LENGTH: usize = 256;

/// Default batch size for embedding
pub const DEFAULT_BATCH_SIZE: usize = 32;

/// Candle-based embedder using all-MiniLM-L6-v2.
pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    info: ModelInfo,
}

impl CandleEmbedder {
    /// Load the embedding model from cache (downloading if needed).
    pub fn load(cache: &ModelCache) -> Result<Self, EmbeddingError> {
        let paths = get_or_download_model(cache)?;
        Self::load_from_paths(&paths.config, &paths.tokenizer, &paths.weights)
    }

    /// Load with default cache settings
    pub fn load_default() -> Result<Self, EmbeddingError> {
        let cache = ModelCache::default();
        Self::load(&cache)
    }

    /// Load from explicit file paths
    pub fn load_from_paths(
        config_path: &std::path::Path,
        tokenizer_path: &std::path::Path,
        weights_path: &std::path::Path,
    ) -> Result<Self, EmbeddingError> {
        info!("Loading embedding model...");

        // Use CPU device (GPU support can be added later with feature flags)
        let device = Device::Cpu;

        // Load config
        let config_str = std::fs::read_to_string(config_path)?;
        let config: BertConfig = serde_json::from_str(&config_str)
            .map_err(|e| EmbeddingError::ModelNotFound(format!("Invalid config: {}", e)))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| EmbeddingError::Tokenizer(e.to_string()))?;

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path.to_path_buf()], DType::F32, &device)?
        };

        let model = BertModel::load(vb, &config)?;

        info!(
            dim = EMBEDDING_DIM,
            max_seq = MAX_SEQ_LENGTH,
            "Model loaded successfully"
        );

        Ok(Self {
            model,
            tokenizer,
            device,
            info: ModelInfo {
                name: "all-MiniLM-L6-v2".to_string(),
                dimension: EMBEDDING_DIM,
                max_sequence_length: MAX_SEQ_LENGTH,
            },
        })
    }

    /// Mean pooling over token embeddings (excluding padding)
    fn mean_pooling(
        &self,
        embeddings: &Tensor,
        attention_mask: &Tensor,
    ) -> Result<Tensor, EmbeddingError> {
        // Expand attention mask to embedding dimension
        let mask = attention_mask
            .unsqueeze(2)?
            .broadcast_as(embeddings.shape())?;
        let mask_f32 = mask.to_dtype(DType::F32)?;

        // Masked sum
        let masked = embeddings.broadcast_mul(&mask_f32)?;
        let sum = masked.sum(1)?;

        // Divide by sum of mask (number of real tokens)
        let mask_sum = mask_f32.sum(1)?;
        let mask_sum = mask_sum.clamp(1e-9, f64::MAX)?; // Avoid division by zero

        let mean = sum.broadcast_div(&mask_sum)?;
        Ok(mean)
    }
}

impl EmbeddingModel for CandleEmbedder {
    fn info(&self) -> &ModelInfo {
        &self.info
    }

    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let embeddings = self.embed_batch(&[text])?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        debug!(count = texts.len(), "Embedding batch");

        // Tokenize all texts
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| EmbeddingError::Tokenizer(e.to_string()))?;

        // Pad to same length
        let max_len = encodings
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0)
            .min(MAX_SEQ_LENGTH);

        let mut input_ids: Vec<Vec<u32>> = Vec::new();
        let mut attention_masks: Vec<Vec<u32>> = Vec::new();

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();

            let truncated_len = ids.len().min(max_len);
            let mut padded_ids = ids[..truncated_len].to_vec();
            let mut padded_mask = mask[..truncated_len].to_vec();

            // Pad to max_len
            padded_ids.resize(max_len, 0);
            padded_mask.resize(max_len, 0);

            input_ids.push(padded_ids);
            attention_masks.push(padded_mask);
        }

        // Convert to tensors
        let batch_size = texts.len();
        let input_ids_flat: Vec<u32> = input_ids.into_iter().flatten().collect();
        let mask_flat: Vec<u32> = attention_masks.into_iter().flatten().collect();

        let input_ids = Tensor::from_vec(input_ids_flat, (batch_size, max_len), &self.device)?;
        let attention_mask = Tensor::from_vec(mask_flat, (batch_size, max_len), &self.device)?;
        let token_type_ids = Tensor::zeros_like(&input_ids)?;

        // Forward pass
        let output = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))?;

        // Mean pooling
        let pooled = self.mean_pooling(&output, &attention_mask)?;

        // Convert to embeddings
        let pooled_vec: Vec<Vec<f32>> = pooled.to_vec2()?;

        let embeddings: Vec<Embedding> = pooled_vec
            .into_iter()
            .map(Embedding::new) // Normalizes the vector
            .collect();

        debug!(
            count = embeddings.len(),
            dim = EMBEDDING_DIM,
            "Batch complete"
        );

        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests require model download, run with:
    // cargo test -p memory-embeddings --features integration -- --ignored

    #[test]
    #[ignore = "requires model download"]
    fn test_load_model() {
        let embedder = CandleEmbedder::load_default().unwrap();
        assert_eq!(embedder.info().dimension, EMBEDDING_DIM);
    }

    #[test]
    #[ignore = "requires model download"]
    fn test_embed_single() {
        let embedder = CandleEmbedder::load_default().unwrap();
        let emb = embedder.embed("Hello, world!").unwrap();
        assert_eq!(emb.dimension(), EMBEDDING_DIM);
    }

    #[test]
    #[ignore = "requires model download"]
    fn test_embed_batch() {
        let embedder = CandleEmbedder::load_default().unwrap();
        let texts = vec!["Hello", "World", "Test"];
        let embeddings = embedder.embed_batch(&texts).unwrap();
        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.dimension(), EMBEDDING_DIM);
        }
    }

    #[test]
    #[ignore = "requires model download"]
    fn test_similar_texts_high_similarity() {
        let embedder = CandleEmbedder::load_default().unwrap();
        let emb1 = embedder.embed("The cat sat on the mat").unwrap();
        let emb2 = embedder.embed("A cat is sitting on a mat").unwrap();
        let emb3 = embedder.embed("Python programming language").unwrap();

        let sim_similar = emb1.cosine_similarity(&emb2);
        let sim_different = emb1.cosine_similarity(&emb3);

        // Similar sentences should have higher similarity
        assert!(sim_similar > sim_different);
        assert!(sim_similar > 0.7); // Should be quite similar
    }
}
