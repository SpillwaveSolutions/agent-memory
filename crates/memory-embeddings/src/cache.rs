//! Model file caching.
//!
//! Downloads and caches model files from HuggingFace Hub.

use std::path::PathBuf;
use tracing::{debug, info};

use crate::error::EmbeddingError;

/// Default model repository on HuggingFace
pub const DEFAULT_MODEL_REPO: &str = "sentence-transformers/all-MiniLM-L6-v2";

/// Required model files
pub const MODEL_FILES: &[&str] = &["config.json", "tokenizer.json", "model.safetensors"];

/// Model cache configuration
#[derive(Debug, Clone)]
pub struct ModelCache {
    /// Cache directory path
    pub cache_dir: PathBuf,
    /// Model repository ID
    pub repo_id: String,
}

impl Default for ModelCache {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("agent-memory")
            .join("models");

        Self {
            cache_dir,
            repo_id: DEFAULT_MODEL_REPO.to_string(),
        }
    }
}

impl ModelCache {
    /// Create a new model cache with custom settings
    pub fn new(cache_dir: impl Into<PathBuf>, repo_id: impl Into<String>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            repo_id: repo_id.into(),
        }
    }

    /// Get the model directory path
    pub fn model_dir(&self) -> PathBuf {
        self.cache_dir.join(self.repo_id.replace('/', "_"))
    }

    /// Check if all model files are cached
    pub fn is_cached(&self) -> bool {
        let model_dir = self.model_dir();
        MODEL_FILES.iter().all(|f| model_dir.join(f).exists())
    }

    /// Get path to a specific model file
    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.model_dir().join(filename)
    }
}

/// Paths to model files
#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub config: PathBuf,
    pub tokenizer: PathBuf,
    pub weights: PathBuf,
}

/// Get or download model files.
///
/// Returns paths to config.json, tokenizer.json, and model.safetensors.
pub fn get_or_download_model(cache: &ModelCache) -> Result<ModelPaths, EmbeddingError> {
    let model_dir = cache.model_dir();

    if cache.is_cached() {
        debug!(path = ?model_dir, "Using cached model");
    } else {
        info!(repo = %cache.repo_id, "Downloading model files...");
        download_model_files(cache)?;
    }

    Ok(ModelPaths {
        config: model_dir.join("config.json"),
        tokenizer: model_dir.join("tokenizer.json"),
        weights: model_dir.join("model.safetensors"),
    })
}

/// Download model files from HuggingFace Hub
fn download_model_files(cache: &ModelCache) -> Result<(), EmbeddingError> {
    use hf_hub::api::sync::Api;

    let api = Api::new().map_err(|e| EmbeddingError::Download(e.to_string()))?;
    let repo = api.model(cache.repo_id.clone());

    std::fs::create_dir_all(cache.model_dir())?;

    for filename in MODEL_FILES {
        info!(file = filename, "Downloading...");
        let source_path = repo
            .get(filename)
            .map_err(|e| EmbeddingError::Download(format!("{}: {}", filename, e)))?;

        let dest_path = cache.file_path(filename);
        std::fs::copy(&source_path, &dest_path)?;
        debug!(file = filename, "Downloaded to {:?}", dest_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_default() {
        let cache = ModelCache::default();
        assert!(cache.cache_dir.to_string_lossy().contains("agent-memory"));
        assert_eq!(cache.repo_id, DEFAULT_MODEL_REPO);
    }

    #[test]
    fn test_is_cached_empty() {
        let temp = TempDir::new().unwrap();
        let cache = ModelCache::new(temp.path(), "test/model");
        assert!(!cache.is_cached());
    }
}
