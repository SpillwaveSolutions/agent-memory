//! Agent adapter trait definition.
//!
//! The `AgentAdapter` trait defines the interface that all agent-specific
//! adapters must implement to integrate with Agent Memory.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use memory_types::Event;

use crate::config::AdapterConfig;
use crate::error::AdapterError;

/// Raw event data before normalization.
///
/// This represents event data in the agent's native format,
/// before being converted to the unified Event type.
#[derive(Debug, Clone)]
pub struct RawEvent {
    /// Unique identifier from the source agent.
    pub id: String,

    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp_ms: i64,

    /// Event content/text.
    pub content: String,

    /// Event type in the source agent's terminology.
    pub event_type: String,

    /// Role identifier from the source agent.
    pub role: String,

    /// Session identifier from the source agent.
    pub session_id: String,

    /// Additional metadata from the source agent.
    pub metadata: HashMap<String, String>,
}

impl RawEvent {
    /// Create a new raw event.
    pub fn new(id: impl Into<String>, timestamp_ms: i64, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            timestamp_ms,
            content: content.into(),
            event_type: String::new(),
            role: String::new(),
            session_id: String::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the event type.
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = event_type.into();
        self
    }

    /// Set the role.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    /// Set the session ID.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Trait for agent-specific adapters.
///
/// Implement this trait to add support for a new AI agent CLI.
///
/// # Agent Identifier
///
/// The `agent_id()` method should return a lowercase, stable identifier
/// that uniquely identifies the agent. This identifier is stored with
/// events and used for filtering queries.
///
/// Canonical agent IDs:
/// - `"claude"` - Claude Code
/// - `"opencode"` - OpenCode CLI
/// - `"gemini"` - Gemini CLI
/// - `"copilot"` - GitHub Copilot CLI
///
/// # Example
///
/// ```rust,ignore
/// use memory_adapters::{AgentAdapter, AdapterConfig, AdapterError, RawEvent};
/// use memory_types::{Event, EventType, EventRole};
/// use chrono::{DateTime, Utc};
///
/// struct OpenCodeAdapter;
///
/// #[async_trait::async_trait]
/// impl AgentAdapter for OpenCodeAdapter {
///     fn agent_id(&self) -> &str {
///         "opencode"
///     }
///
///     fn display_name(&self) -> &str {
///         "OpenCode CLI"
///     }
///
///     fn normalize(&self, raw: RawEvent) -> Result<Event, AdapterError> {
///         // Convert OpenCode-specific event to unified format
///         let timestamp = DateTime::from_timestamp_millis(raw.timestamp_ms)
///             .unwrap_or_else(Utc::now);
///
///         Ok(Event::new(
///             raw.id,
///             raw.session_id,
///             timestamp,
///             EventType::UserMessage,
///             EventRole::User,
///             raw.content,
///         ).with_agent(self.agent_id()))
///     }
///
///     fn load_config(&self, path: Option<&std::path::Path>) -> Result<AdapterConfig, AdapterError> {
///         // Load from ~/.config/opencode/adapter.toml or default
///         Ok(AdapterConfig::default())
///     }
/// }
/// ```
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Canonical agent identifier (lowercase, e.g., "claude", "opencode").
    ///
    /// This identifier is stored with events and used for query filtering.
    /// It should be stable across versions.
    fn agent_id(&self) -> &str;

    /// Human-readable agent name (e.g., "Claude Code", "OpenCode CLI").
    ///
    /// Used for display purposes in logs and status messages.
    fn display_name(&self) -> &str;

    /// Convert raw event to unified Event format.
    ///
    /// This method is responsible for:
    /// 1. Mapping event types to unified EventType enum
    /// 2. Mapping roles to unified EventRole enum
    /// 3. Extracting/generating event IDs
    /// 4. Setting the agent identifier via with_agent()
    ///
    /// # Errors
    ///
    /// Returns `AdapterError::Normalize` if the raw event cannot be converted.
    fn normalize(&self, raw: RawEvent) -> Result<Event, AdapterError>;

    /// Load adapter configuration from path or default location.
    ///
    /// If `path` is None, use the agent's default config location.
    ///
    /// # Errors
    ///
    /// Returns `AdapterError::Config` if configuration cannot be loaded.
    fn load_config(&self, path: Option<&Path>) -> Result<AdapterConfig, AdapterError>;

    /// Attempt to auto-detect this adapter from environment.
    ///
    /// Override this to enable automatic adapter selection based on
    /// environment variables, running processes, or other signals.
    ///
    /// Default implementation returns false (explicit selection required).
    fn detect(&self) -> bool {
        false
    }

    /// Check if the adapter is available and properly configured.
    ///
    /// Override this for adapters that require external services or binaries.
    ///
    /// Default implementation returns true.
    fn is_available(&self) -> bool {
        true
    }

    /// Normalize agent identifier to lowercase.
    ///
    /// This helper ensures consistent agent IDs across the system.
    fn normalize_agent_id(id: &str) -> String {
        id.to_lowercase().trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use memory_types::{EventRole, EventType};

    // Mock adapter for testing
    struct MockAdapter;

    #[async_trait]
    impl AgentAdapter for MockAdapter {
        fn agent_id(&self) -> &str {
            "mock"
        }

        fn display_name(&self) -> &str {
            "Mock Agent"
        }

        fn normalize(&self, raw: RawEvent) -> Result<Event, AdapterError> {
            let timestamp =
                chrono::DateTime::from_timestamp_millis(raw.timestamp_ms).unwrap_or_else(Utc::now);

            Ok(Event::new(
                raw.id,
                raw.session_id,
                timestamp,
                EventType::UserMessage,
                EventRole::User,
                raw.content,
            )
            .with_agent(self.agent_id()))
        }

        fn load_config(&self, _path: Option<&Path>) -> Result<AdapterConfig, AdapterError> {
            Ok(AdapterConfig::default())
        }
    }

    #[test]
    fn test_raw_event_builder() {
        let raw = RawEvent::new("evt-1", 1704067200000, "Hello")
            .with_event_type("user_message")
            .with_role("user")
            .with_session_id("session-123")
            .with_metadata("tool", "Read");

        assert_eq!(raw.id, "evt-1");
        assert_eq!(raw.timestamp_ms, 1704067200000);
        assert_eq!(raw.content, "Hello");
        assert_eq!(raw.event_type, "user_message");
        assert_eq!(raw.role, "user");
        assert_eq!(raw.session_id, "session-123");
        assert_eq!(raw.metadata.get("tool"), Some(&"Read".to_string()));
    }

    #[test]
    fn test_mock_adapter_normalize() {
        let adapter = MockAdapter;
        let raw =
            RawEvent::new("evt-1", 1704067200000, "Test message").with_session_id("session-123");

        let event = adapter.normalize(raw).unwrap();

        assert_eq!(event.event_id, "evt-1");
        assert_eq!(event.session_id, "session-123");
        assert_eq!(event.text, "Test message");
        assert_eq!(event.agent, Some("mock".to_string()));
    }

    #[test]
    fn test_normalize_agent_id() {
        assert_eq!(MockAdapter::normalize_agent_id("Claude"), "claude");
        assert_eq!(MockAdapter::normalize_agent_id("  OpenCode  "), "opencode");
        assert_eq!(MockAdapter::normalize_agent_id("GEMINI"), "gemini");
    }

    #[test]
    fn test_adapter_default_methods() {
        let adapter = MockAdapter;
        assert!(!adapter.detect());
        assert!(adapter.is_available());
    }

    #[test]
    fn test_adapter_agent_id() {
        let adapter = MockAdapter;
        assert_eq!(adapter.agent_id(), "mock");
    }

    #[test]
    fn test_adapter_display_name() {
        let adapter = MockAdapter;
        assert_eq!(adapter.display_name(), "Mock Agent");
    }

    #[test]
    fn test_adapter_load_config() {
        let adapter = MockAdapter;
        let config = adapter.load_config(None).unwrap();
        assert!(config.is_enabled());
    }

    #[test]
    fn test_raw_event_new_defaults() {
        let raw = RawEvent::new("id-1", 1000, "content");
        assert_eq!(raw.id, "id-1");
        assert_eq!(raw.timestamp_ms, 1000);
        assert_eq!(raw.content, "content");
        assert!(raw.event_type.is_empty());
        assert!(raw.role.is_empty());
        assert!(raw.session_id.is_empty());
        assert!(raw.metadata.is_empty());
    }
}
