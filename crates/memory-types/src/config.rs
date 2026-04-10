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

/// Configuration for semantic dedup gate (opt-in, disabled by default).
///
/// Replaces the former `NoveltyConfig`. Controls whether incoming events
/// are checked for near-duplicate content before storage.
/// When disabled, all events are stored without similarity check.
/// This respects the append-only model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupConfig {
    /// MUST be explicitly set to true to enable (default: false).
    /// When false, all events are stored without similarity check.
    #[serde(default)]
    pub enabled: bool,

    /// Similarity threshold - events above this are considered duplicates.
    /// Range: 0.0-1.0, higher = stricter (more duplicates detected).
    #[serde(default = "default_dedup_threshold")]
    pub threshold: f32,

    /// Maximum time for dedup check (ms).
    /// If exceeded, event is stored anyway (fail-open).
    #[serde(default = "default_dedup_timeout")]
    pub timeout_ms: u64,

    /// Minimum event text length to check (skip very short events).
    #[serde(default = "default_min_text_length")]
    pub min_text_length: usize,

    /// Capacity of the in-flight ring buffer for recent embeddings.
    #[serde(default = "default_buffer_capacity")]
    pub buffer_capacity: usize,
}

/// Backward-compatible type alias for code that still references `NoveltyConfig`.
pub type NoveltyConfig = DedupConfig;

fn default_dedup_threshold() -> f32 {
    0.85
}

fn default_dedup_timeout() -> u64 {
    50
}

fn default_min_text_length() -> usize {
    50
}

fn default_buffer_capacity() -> usize {
    256
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            enabled: false, // DISABLED by default - explicit opt-in required
            threshold: default_dedup_threshold(),
            timeout_ms: default_dedup_timeout(),
            min_text_length: default_min_text_length(),
            buffer_capacity: default_buffer_capacity(),
        }
    }
}

impl DedupConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.threshold) {
            return Err(format!("threshold must be 0.0-1.0, got {}", self.threshold));
        }
        if self.timeout_ms == 0 {
            return Err("timeout_ms must be > 0".to_string());
        }
        if self.buffer_capacity == 0 {
            return Err("buffer_capacity must be > 0".to_string());
        }
        Ok(())
    }
}

/// Configuration for staleness-based score decay at query time.
///
/// Controls how query results are downranked based on age relative to
/// the newest result. High-salience memory kinds (Constraint, Definition,
/// Procedure, Preference) are exempt from decay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessConfig {
    /// Whether staleness scoring is enabled (default: true).
    #[serde(default = "default_staleness_enabled")]
    pub enabled: bool,

    /// Half-life for time-decay in days (default: 14.0).
    /// After this many days, decay reaches ~50% of max_penalty.
    #[serde(default = "default_half_life_days")]
    pub half_life_days: f32,

    /// Maximum score penalty from time-decay (default: 0.30).
    /// Asymptotic bound -- never fully reached.
    #[serde(default = "default_max_penalty")]
    pub max_penalty: f32,

    /// Penalty applied to superseded results (default: 0.15).
    /// Used in Plan 37-02 when supersession detection is wired.
    #[serde(default = "default_supersession_penalty")]
    pub supersession_penalty: f32,

    /// Similarity threshold for supersession detection (default: 0.80).
    /// Used in Plan 37-02 when supersession detection is wired.
    #[serde(default = "default_supersession_threshold")]
    pub supersession_threshold: f32,
}

fn default_staleness_enabled() -> bool {
    true
}

fn default_half_life_days() -> f32 {
    14.0
}

fn default_max_penalty() -> f32 {
    0.30
}

fn default_supersession_penalty() -> f32 {
    0.15
}

fn default_supersession_threshold() -> f32 {
    0.80
}

impl Default for StalenessConfig {
    fn default() -> Self {
        Self {
            enabled: default_staleness_enabled(),
            half_life_days: default_half_life_days(),
            max_penalty: default_max_penalty(),
            supersession_penalty: default_supersession_penalty(),
            supersession_threshold: default_supersession_threshold(),
        }
    }
}

impl StalenessConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), String> {
        if self.half_life_days <= 0.0 {
            return Err(format!(
                "half_life_days must be > 0, got {}",
                self.half_life_days
            ));
        }
        if !(0.0..=1.0).contains(&self.max_penalty) {
            return Err(format!(
                "max_penalty must be 0.0-1.0, got {}",
                self.max_penalty
            ));
        }
        if !(0.0..=1.0).contains(&self.supersession_penalty) {
            return Err(format!(
                "supersession_penalty must be 0.0-1.0, got {}",
                self.supersession_penalty
            ));
        }
        if !(0.0..=1.0).contains(&self.supersession_threshold) {
            return Err(format!(
                "supersession_threshold must be 0.0-1.0, got {}",
                self.supersession_threshold
            ));
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

    /// Name of the environment variable to read the API key from.
    /// If unset, defaults to "OPENAI_API_KEY" for openai and
    /// "ANTHROPIC_API_KEY" for anthropic.
    #[serde(default)]
    pub api_key_env: Option<String>,
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
            api_key_env: None,
        }
    }
}

/// Configuration for episodic memory (Phase 43).
///
/// Controls whether episodic memory is enabled and how episodes are
/// scored and retained. Disabled by default -- must be explicitly enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicConfig {
    /// Whether episodic memory is enabled (default: false).
    #[serde(default)]
    pub enabled: bool,

    /// Minimum value score for an episode to be retained in long-term storage.
    /// Episodes below this threshold may be pruned.
    #[serde(default = "default_episodic_value_threshold")]
    pub value_threshold: f32,

    /// Target midpoint for value scoring (default: 0.65).
    /// Episodes with outcome scores near this value are considered most valuable.
    #[serde(default = "default_episodic_midpoint_target")]
    pub midpoint_target: f32,

    /// Maximum number of episodes to retain (default: 1000).
    /// Oldest low-value episodes are pruned first when this limit is reached.
    #[serde(default = "default_episodic_max_episodes")]
    pub max_episodes: usize,
}

fn default_episodic_value_threshold() -> f32 {
    0.18
}

fn default_episodic_midpoint_target() -> f32 {
    0.65
}

fn default_episodic_max_episodes() -> usize {
    1000
}

impl Default for EpisodicConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            value_threshold: default_episodic_value_threshold(),
            midpoint_target: default_episodic_midpoint_target(),
            max_episodes: default_episodic_max_episodes(),
        }
    }
}

impl EpisodicConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.value_threshold) {
            return Err(format!(
                "value_threshold must be 0.0-1.0, got {}",
                self.value_threshold
            ));
        }
        if !(0.0..=1.0).contains(&self.midpoint_target) {
            return Err(format!(
                "midpoint_target must be 0.0-1.0, got {}",
                self.midpoint_target
            ));
        }
        if self.max_episodes == 0 {
            return Err("max_episodes must be > 0".to_string());
        }
        Ok(())
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

/// Cross-project federation configuration (v3.0).
///
/// Controls whether queries can span multiple registered project stores.
/// Disabled by default — must be explicitly enabled (opt-in).
/// If a registered project store is unavailable, it is silently skipped (fail-open).
///
/// Maps to `[projects]` section in config.toml:
/// ```toml
/// [projects]
/// registered = ["/path/to/project-a/db", "/path/to/project-b/db"]
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrossProjectConfig {
    /// Paths to additional project RocksDB stores.
    /// Each path is opened read-only for cross-project queries.
    #[serde(default)]
    pub registered: Vec<PathBuf>,
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

    /// Semantic dedup gate configuration.
    /// Accepts `[dedup]` or legacy `[novelty]` TOML section.
    #[serde(default, alias = "novelty")]
    pub dedup: DedupConfig,

    /// Staleness-based score decay configuration.
    #[serde(default)]
    pub staleness: StalenessConfig,

    /// Salience scoring configuration.
    #[serde(default)]
    pub salience: crate::SalienceConfig,

    /// Usage decay configuration.
    #[serde(default)]
    pub usage: crate::UsageConfig,

    /// Lifecycle automation configuration.
    #[serde(default)]
    pub lifecycle: LifecycleConfig,

    /// Episodic memory configuration (Phase 43).
    #[serde(default)]
    pub episodic: EpisodicConfig,

    /// Cross-project federation configuration (v3.0).
    /// Lists additional project stores to include in federated queries.
    #[serde(default)]
    pub projects: CrossProjectConfig,
}

/// Lifecycle automation configuration for index pruning and rebuilding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Vector index lifecycle settings.
    #[serde(default)]
    pub vector: VectorLifecycleSettings,

    /// BM25 index lifecycle settings.
    #[serde(default)]
    pub bm25: Bm25LifecycleSettings,
}

/// Vector index lifecycle settings.
///
/// Maps to `[lifecycle.vector]` section in config.toml.
/// Enabled by default - vector indexes grow unbounded without pruning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorLifecycleSettings {
    /// Enable automatic vector pruning (default: true).
    #[serde(default = "default_vector_enabled")]
    pub enabled: bool,

    /// Retention days for segment-level vectors (default: 30).
    #[serde(default = "default_segment_retention")]
    pub segment_retention_days: u32,

    /// Retention days for grip-level vectors (default: 30).
    #[serde(default = "default_grip_retention")]
    pub grip_retention_days: u32,

    /// Retention days for day-level vectors (default: 365).
    #[serde(default = "default_day_retention")]
    pub day_retention_days: u32,

    /// Retention days for week-level vectors (default: 1825 = 5 years).
    #[serde(default = "default_week_retention")]
    pub week_retention_days: u32,

    /// Cron schedule for prune job (default: "0 3 * * *" = daily 3 AM).
    #[serde(default = "default_vector_prune_schedule")]
    pub prune_schedule: String,
}

fn default_vector_enabled() -> bool {
    true
}

fn default_segment_retention() -> u32 {
    30
}
fn default_grip_retention() -> u32 {
    30
}
fn default_day_retention() -> u32 {
    365
}
fn default_week_retention() -> u32 {
    1825
}

fn default_vector_prune_schedule() -> String {
    "0 3 * * *".to_string()
}

impl Default for VectorLifecycleSettings {
    fn default() -> Self {
        Self {
            enabled: default_vector_enabled(),
            segment_retention_days: default_segment_retention(),
            grip_retention_days: default_grip_retention(),
            day_retention_days: default_day_retention(),
            week_retention_days: default_week_retention(),
            prune_schedule: default_vector_prune_schedule(),
        }
    }
}

/// BM25 index lifecycle settings.
///
/// Maps to `[lifecycle.bm25]` section in config.toml.
/// DISABLED by default per PRD "append-only, no eviction" philosophy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25LifecycleSettings {
    /// Whether BM25 lifecycle is enabled (default: false, opt-in).
    #[serde(default)]
    pub enabled: bool,

    /// Minimum TOC level to keep after rollup rebuild (default: "day").
    /// Segments and grips below this level are excluded from rebuilt index.
    #[serde(default = "default_min_level")]
    pub min_level_after_rollup: String,

    /// Cron schedule for rebuild job (default: "0 4 * * 0" = weekly Sunday 4 AM).
    #[serde(default = "default_bm25_rebuild_schedule")]
    pub rebuild_schedule: String,

    /// Retention days for segment-level docs (default: 30).
    #[serde(default = "default_segment_retention")]
    pub segment_retention_days: u32,

    /// Retention days for grip-level docs (default: 30).
    #[serde(default = "default_grip_retention")]
    pub grip_retention_days: u32,

    /// Retention days for day-level docs (default: 180).
    #[serde(default = "default_bm25_day_retention")]
    pub day_retention_days: u32,

    /// Retention days for week-level docs (default: 1825 = 5 years).
    #[serde(default = "default_week_retention")]
    pub week_retention_days: u32,
}

fn default_min_level() -> String {
    "day".to_string()
}

fn default_bm25_rebuild_schedule() -> String {
    "0 4 * * 0".to_string()
}

fn default_bm25_day_retention() -> u32 {
    180
}

impl Default for Bm25LifecycleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            min_level_after_rollup: default_min_level(),
            rebuild_schedule: default_bm25_rebuild_schedule(),
            segment_retention_days: default_segment_retention(),
            grip_retention_days: default_grip_retention(),
            day_retention_days: default_bm25_day_retention(),
            week_retention_days: default_week_retention(),
        }
    }
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
            dedup: DedupConfig::default(),
            staleness: StalenessConfig::default(),
            salience: crate::SalienceConfig::default(),
            usage: crate::UsageConfig::default(),
            lifecycle: LifecycleConfig::default(),
            episodic: EpisodicConfig::default(),
            projects: CrossProjectConfig::default(),
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
        assert!((config.threshold - 0.85).abs() < f32::EPSILON);
        assert_eq!(config.timeout_ms, 50);
        assert_eq!(config.min_text_length, 50);
        assert_eq!(config.buffer_capacity, 256);
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
        assert!((decoded.threshold - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dedup_config_buffer_capacity_validation() {
        let mut config = DedupConfig::default();
        assert!(config.validate().is_ok());

        config.buffer_capacity = 0;
        let err = config.validate().unwrap_err();
        assert!(
            err.contains("buffer_capacity"),
            "expected buffer_capacity error, got: {err}"
        );
    }

    #[test]
    fn test_settings_dedup_default() {
        let settings = Settings::default();
        assert!(!settings.dedup.enabled);
        assert!((settings.dedup.threshold - 0.85).abs() < f32::EPSILON);
        assert_eq!(settings.dedup.buffer_capacity, 256);
        assert_eq!(settings.dedup.timeout_ms, 50);
        assert_eq!(settings.dedup.min_text_length, 50);
    }

    #[test]
    fn test_staleness_config_defaults() {
        let config = StalenessConfig::default();
        assert!(config.enabled);
        assert!((config.half_life_days - 14.0).abs() < f32::EPSILON);
        assert!((config.max_penalty - 0.30).abs() < f32::EPSILON);
        assert!((config.supersession_penalty - 0.15).abs() < f32::EPSILON);
        assert!((config.supersession_threshold - 0.80).abs() < f32::EPSILON);
    }

    #[test]
    fn test_staleness_config_validation_pass() {
        let config = StalenessConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_staleness_config_validation_fail_half_life() {
        let config = StalenessConfig {
            half_life_days: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = StalenessConfig {
            half_life_days: -1.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_staleness_config_validation_fail_penalties() {
        let config = StalenessConfig {
            max_penalty: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = StalenessConfig {
            supersession_penalty: -0.1,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = StalenessConfig {
            supersession_threshold: 1.1,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_staleness_config_serialization() {
        let config = StalenessConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: StalenessConfig = serde_json::from_str(&json).unwrap();
        assert!(decoded.enabled);
        assert!((decoded.half_life_days - 14.0).abs() < f32::EPSILON);
        assert!((decoded.max_penalty - 0.30).abs() < f32::EPSILON);
    }

    #[test]
    fn test_settings_staleness_default() {
        let settings = Settings::default();
        assert!(settings.staleness.enabled);
        assert!((settings.staleness.half_life_days - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dedup_config_novelty_alias() {
        // Deserialize using old field names -- NoveltyConfig is a type alias for DedupConfig
        let json = r#"{"enabled":true,"threshold":0.9,"timeout_ms":100,"min_text_length":30,"buffer_capacity":128}"#;
        let config: NoveltyConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!((config.threshold - 0.9).abs() < f32::EPSILON);
        assert_eq!(config.buffer_capacity, 128);

        // Deserialize with defaults (missing buffer_capacity should default to 256)
        let json_minimal = r#"{"enabled":false}"#;
        let config2: DedupConfig = serde_json::from_str(json_minimal).unwrap();
        assert_eq!(config2.buffer_capacity, 256);
    }

    #[test]
    fn test_lifecycle_config_defaults() {
        let config = LifecycleConfig::default();

        // Vector: enabled by default
        assert!(config.vector.enabled);
        assert_eq!(config.vector.segment_retention_days, 30);
        assert_eq!(config.vector.grip_retention_days, 30);
        assert_eq!(config.vector.day_retention_days, 365);
        assert_eq!(config.vector.week_retention_days, 1825);
        assert_eq!(config.vector.prune_schedule, "0 3 * * *");

        // BM25: disabled by default (opt-in)
        assert!(!config.bm25.enabled);
        assert_eq!(config.bm25.min_level_after_rollup, "day");
        assert_eq!(config.bm25.rebuild_schedule, "0 4 * * 0");
        assert_eq!(config.bm25.segment_retention_days, 30);
        assert_eq!(config.bm25.grip_retention_days, 30);
        assert_eq!(config.bm25.day_retention_days, 180);
        assert_eq!(config.bm25.week_retention_days, 1825);
    }

    #[test]
    fn test_lifecycle_config_serialization() {
        let config = LifecycleConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: LifecycleConfig = serde_json::from_str(&json).unwrap();
        assert!(decoded.vector.enabled);
        assert!(!decoded.bm25.enabled);
        assert_eq!(decoded.bm25.min_level_after_rollup, "day");
        assert_eq!(decoded.vector.prune_schedule, "0 3 * * *");
    }

    #[test]
    fn test_settings_lifecycle_default() {
        let settings = Settings::default();
        assert!(settings.lifecycle.vector.enabled);
        assert!(!settings.lifecycle.bm25.enabled);
    }

    #[test]
    fn test_episodic_config_defaults() {
        let config = EpisodicConfig::default();
        assert!(!config.enabled);
        assert!((config.value_threshold - 0.18).abs() < f32::EPSILON);
        assert!((config.midpoint_target - 0.65).abs() < f32::EPSILON);
        assert_eq!(config.max_episodes, 1000);
    }

    #[test]
    fn test_episodic_config_validation_pass() {
        let config = EpisodicConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_episodic_config_validation_fail() {
        let config = EpisodicConfig {
            value_threshold: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = EpisodicConfig {
            midpoint_target: -0.1,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = EpisodicConfig {
            max_episodes: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_episodic_config_serialization() {
        let config = EpisodicConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: EpisodicConfig = serde_json::from_str(&json).unwrap();
        assert!(!decoded.enabled);
        assert!((decoded.value_threshold - 0.18).abs() < f32::EPSILON);
        assert!((decoded.midpoint_target - 0.65).abs() < f32::EPSILON);
        assert_eq!(decoded.max_episodes, 1000);
    }

    #[test]
    fn test_episodic_config_backward_compat() {
        // Deserialize with missing episodic section (pre-phase-43 config)
        let json = r#"{}"#;
        let config: EpisodicConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.max_episodes, 1000);
    }

    #[test]
    fn test_settings_episodic_default() {
        let settings = Settings::default();
        assert!(!settings.episodic.enabled);
        assert!((settings.episodic.value_threshold - 0.18).abs() < f32::EPSILON);
        assert!((settings.episodic.midpoint_target - 0.65).abs() < f32::EPSILON);
        assert_eq!(settings.episodic.max_episodes, 1000);
    }
}
