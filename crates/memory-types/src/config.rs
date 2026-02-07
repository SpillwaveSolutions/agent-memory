//! Configuration loading for agent-memory.
//!
//! Per CFG-01: Layered config: defaults -> config file -> env vars -> CLI flags
//! Per CFG-02: Config includes db_path, grpc_port, summarizer settings
//! Per CFG-03: Config file at ~/.config/agent-memory/config.toml

use config::{Config, Environment, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::MemoryError;

/// Configuration for novelty detection (opt-in, disabled by default).
///
/// Per Phase 16 Plan 03: Novelty check is DISABLED by default.
/// When disabled, all events are stored without similarity check.
/// This respects the append-only model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoveltyConfig {
    /// MUST be explicitly set to true to enable (default: false).
    /// When false, all events are stored without similarity check.
    #[serde(default)]
    pub enabled: bool,

    /// Similarity threshold - events above this are considered duplicates.
    /// Range: 0.0-1.0, higher = stricter (more duplicates detected).
    #[serde(default = "default_novelty_threshold")]
    pub threshold: f32,

    /// Maximum time for novelty check (ms).
    /// If exceeded, event is stored anyway (fail-open).
    #[serde(default = "default_novelty_timeout")]
    pub timeout_ms: u64,

    /// Minimum event text length to check (skip very short events).
    #[serde(default = "default_min_text_length")]
    pub min_text_length: usize,
}

fn default_novelty_threshold() -> f32 {
    0.82
}

fn default_novelty_timeout() -> u64 {
    50
}

fn default_min_text_length() -> usize {
    50
}

impl Default for NoveltyConfig {
    fn default() -> Self {
        Self {
            enabled: false, // DISABLED by default - explicit opt-in required
            threshold: default_novelty_threshold(),
            timeout_ms: default_novelty_timeout(),
            min_text_length: default_min_text_length(),
        }
    }
}

impl NoveltyConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.threshold) {
            return Err(format!("threshold must be 0.0-1.0, got {}", self.threshold));
        }
        if self.timeout_ms == 0 {
            return Err("timeout_ms must be > 0".to_string());
        }
        Ok(())
    }
}

/// Summarizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizerSettings {
    /// Provider name (e.g., "openai", "anthropic", "local")
    #[serde(default = "default_summarizer_provider")]
    pub provider: String,

    /// Model name (e.g., "gpt-4o-mini", "claude-3-haiku")
    #[serde(default = "default_summarizer_model")]
    pub model: String,

    /// API key (loaded from env var, not stored in config file)
    #[serde(default)]
    pub api_key: Option<String>,

    /// API base URL (for custom endpoints)
    #[serde(default)]
    pub api_base_url: Option<String>,
}

fn default_summarizer_provider() -> String {
    "openai".to_string()
}

fn default_summarizer_model() -> String {
    "gpt-4o-mini".to_string()
}

impl Default for SummarizerSettings {
    fn default() -> Self {
        Self {
            provider: default_summarizer_provider(),
            model: default_summarizer_model(),
            api_key: None,
            api_base_url: None,
        }
    }
}

/// Multi-agent storage mode (STOR-06)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiAgentMode {
    /// Each project gets its own RocksDB instance (default)
    #[default]
    Separate,
    /// Single unified store with agent_id tags for isolation
    Unified,
}

/// Main application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Path to RocksDB storage directory
    #[serde(default = "default_db_path")]
    pub db_path: String,

    /// gRPC server port
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,

    /// gRPC server host
    #[serde(default = "default_grpc_host")]
    pub grpc_host: String,

    /// Multi-agent mode: separate stores per project OR unified store with tags (STOR-06)
    #[serde(default)]
    pub multi_agent_mode: MultiAgentMode,

    /// Agent ID for unified mode (used as tag prefix)
    #[serde(default)]
    pub agent_id: Option<String>,

    /// Summarizer configuration
    #[serde(default)]
    pub summarizer: SummarizerSettings,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Path to BM25 search index directory
    #[serde(default = "default_search_index_path")]
    pub search_index_path: String,

    /// Path to HNSW vector index directory
    #[serde(default = "default_vector_index_path")]
    pub vector_index_path: String,
}

fn default_db_path() -> String {
    ProjectDirs::from("", "", "agent-memory")
        .map(|p| p.data_local_dir().join("db"))
        .unwrap_or_else(|| PathBuf::from("./data"))
        .to_string_lossy()
        .to_string()
}

fn default_grpc_port() -> u16 {
    50051
}

fn default_grpc_host() -> String {
    "0.0.0.0".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_search_index_path() -> String {
    ProjectDirs::from("", "", "agent-memory")
        .map(|p| p.data_local_dir().join("bm25-index"))
        .unwrap_or_else(|| PathBuf::from("./bm25-index"))
        .to_string_lossy()
        .to_string()
}

fn default_vector_index_path() -> String {
    ProjectDirs::from("", "", "agent-memory")
        .map(|p| p.data_local_dir().join("vector-index"))
        .unwrap_or_else(|| PathBuf::from("./vector-index"))
        .to_string_lossy()
        .to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            grpc_port: default_grpc_port(),
            grpc_host: default_grpc_host(),
            multi_agent_mode: MultiAgentMode::default(),
            agent_id: None,
            summarizer: SummarizerSettings::default(),
            log_level: default_log_level(),
            search_index_path: default_search_index_path(),
            vector_index_path: default_vector_index_path(),
        }
    }
}

impl Settings {
    /// Load settings with layered precedence:
    /// 1. Built-in defaults
    /// 2. Config file (~/.config/agent-memory/config.toml)
    /// 3. CLI-specified config file (optional)
    /// 4. Environment variables (MEMORY_*)
    ///
    /// CLI flags should be applied by the caller after this returns.
    pub fn load(cli_config_path: Option<&str>) -> Result<Self, MemoryError> {
        // Get default config file location (CFG-03)
        let config_dir = ProjectDirs::from("", "", "agent-memory")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let default_config_path = config_dir.join("config");

        let mut builder = Config::builder()
            // 1. Built-in defaults
            .set_default("db_path", default_db_path())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("grpc_port", default_grpc_port() as i64)
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("grpc_host", default_grpc_host())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("log_level", default_log_level())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("summarizer.provider", default_summarizer_provider())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("summarizer.model", default_summarizer_model())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("search_index_path", default_search_index_path())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            .set_default("vector_index_path", default_vector_index_path())
            .map_err(|e| MemoryError::Config(e.to_string()))?
            // 2. Default config file (~/.config/agent-memory/config.toml)
            .add_source(File::with_name(&default_config_path.to_string_lossy()).required(false));

        // 3. CLI-specified config file (higher precedence than default)
        if let Some(path) = cli_config_path {
            builder = builder.add_source(File::with_name(path).required(true));
        }

        // 4. Environment variables (highest precedence before CLI flags)
        // Format: MEMORY_DB_PATH, MEMORY_GRPC_PORT, MEMORY_SUMMARIZER_PROVIDER, etc.
        builder = builder.add_source(
            Environment::with_prefix("MEMORY")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder
            .build()
            .map_err(|e| MemoryError::Config(e.to_string()))?;

        config
            .try_deserialize()
            .map_err(|e| MemoryError::Config(e.to_string()))
    }

    /// Get the socket address for the gRPC server
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.grpc_host, self.grpc_port)
    }

    /// Expand ~ in db_path to actual home directory
    pub fn expanded_db_path(&self) -> PathBuf {
        if self.db_path.starts_with("~/") {
            if let Some(home) = dirs_home() {
                return home.join(&self.db_path[2..]);
            }
        }
        PathBuf::from(&self.db_path)
    }
}

/// Get user's home directory
fn dirs_home() -> Option<PathBuf> {
    ProjectDirs::from("", "", "agent-memory")
        .map(|p| {
            p.config_dir()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.grpc_port, 50051);
        assert_eq!(settings.grpc_host, "0.0.0.0");
        assert_eq!(settings.summarizer.provider, "openai");
    }

    #[test]
    fn test_load_with_defaults() {
        // Note: This test verifies the defaults load correctly
        let settings = Settings::load(None).unwrap();
        assert_eq!(settings.grpc_port, 50051);
    }

    #[test]
    fn test_grpc_addr() {
        let settings = Settings::default();
        assert_eq!(settings.grpc_addr(), "0.0.0.0:50051");
    }

    #[test]
    fn test_multi_agent_mode_default() {
        let settings = Settings::default();
        assert_eq!(settings.multi_agent_mode, MultiAgentMode::Separate);
    }

    #[test]
    fn test_novelty_config_disabled_by_default() {
        let config = NoveltyConfig::default();
        assert!(!config.enabled);
        assert!((config.threshold - 0.82).abs() < f32::EPSILON);
        assert_eq!(config.timeout_ms, 50);
        assert_eq!(config.min_text_length, 50);
    }

    #[test]
    fn test_novelty_config_validation() {
        let mut config = NoveltyConfig::default();
        assert!(config.validate().is_ok());

        config.threshold = 1.5;
        assert!(config.validate().is_err());

        config.threshold = 0.5;
        config.timeout_ms = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_novelty_config_serialization() {
        let config = NoveltyConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: NoveltyConfig = serde_json::from_str(&json).unwrap();
        assert!(!decoded.enabled);
        assert!((decoded.threshold - 0.82).abs() < f32::EPSILON);
    }
}
