//! Configuration types for agent adapters.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for an agent adapter.
///
/// Each adapter can have its own settings in addition to common fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Path to agent's event log or history file.
    ///
    /// This is where the adapter reads raw events from.
    #[serde(default)]
    pub event_source_path: Option<PathBuf>,

    /// Path to output/ingest events.
    ///
    /// Usually the daemon's gRPC endpoint or a file path.
    #[serde(default)]
    pub ingest_target: Option<String>,

    /// Whether this adapter is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Additional agent-specific settings.
    ///
    /// Use this for settings that don't fit the common fields.
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

fn default_enabled() -> bool {
    true
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            event_source_path: None,
            ingest_target: None,
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

impl AdapterConfig {
    /// Create a new config with the given event source path.
    pub fn with_event_source(path: impl Into<PathBuf>) -> Self {
        Self {
            event_source_path: Some(path.into()),
            enabled: true,
            ..Default::default()
        }
    }

    /// Set the ingest target.
    pub fn with_ingest_target(mut self, target: impl Into<String>) -> Self {
        self.ingest_target = Some(target.into());
        self
    }

    /// Add a custom setting.
    pub fn with_setting(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.settings.insert(key.into(), value.into());
        self
    }

    /// Get a custom setting value.
    pub fn get_setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str())
    }

    /// Check if the adapter is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AdapterConfig::default();
        assert!(config.enabled);
        assert!(config.event_source_path.is_none());
        assert!(config.settings.is_empty());
    }

    #[test]
    fn test_config_builder() {
        let config = AdapterConfig::with_event_source("/var/log/agent.log")
            .with_ingest_target("http://localhost:50051")
            .with_setting("poll_interval_ms", "1000");

        assert_eq!(
            config.event_source_path,
            Some(PathBuf::from("/var/log/agent.log"))
        );
        assert_eq!(
            config.ingest_target,
            Some("http://localhost:50051".to_string())
        );
        assert_eq!(config.get_setting("poll_interval_ms"), Some("1000"));
    }

    #[test]
    fn test_config_serialization() {
        let config = AdapterConfig::with_event_source("/tmp/events.log");
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AdapterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.event_source_path, parsed.event_source_path);
    }

    #[test]
    fn test_config_deserialization_defaults() {
        // Empty JSON should use defaults
        let json = "{}";
        let config: AdapterConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(config.event_source_path.is_none());
    }

    #[test]
    fn test_config_is_enabled() {
        let config = AdapterConfig::default();
        assert!(config.is_enabled());

        let config = AdapterConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!config.is_enabled());
    }
}
