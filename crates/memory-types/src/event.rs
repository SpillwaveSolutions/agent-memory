//! Event type for conversation storage.
//!
//! Events are immutable records of conversation turns, tool calls,
//! session boundaries, and other agent interactions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of the message author
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventRole {
    /// User input
    User,
    /// Assistant response
    Assistant,
    /// System message
    System,
    /// Tool invocation or result
    Tool,
}

impl std::fmt::Display for EventRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventRole::User => write!(f, "user"),
            EventRole::Assistant => write!(f, "assistant"),
            EventRole::System => write!(f, "system"),
            EventRole::Tool => write!(f, "tool"),
        }
    }
}

/// Event type indicating the kind of conversation event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Session started
    SessionStart,
    /// User message submitted
    UserMessage,
    /// Assistant response
    AssistantMessage,
    /// Tool was called and returned result
    ToolResult,
    /// Assistant finished responding
    AssistantStop,
    /// Subagent started
    SubagentStart,
    /// Subagent stopped
    SubagentStop,
    /// Session ended
    SessionEnd,
}

/// A conversation event.
///
/// Events are the fundamental unit of storage. They are immutable and
/// stored with time-prefixed keys for efficient range queries.
///
/// Per ING-02: Includes session_id, timestamp, role, text, metadata.
/// Per ING-04: Uses source timestamp for ordering, not ingestion time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier (ULID string)
    pub event_id: String,

    /// Session this event belongs to
    pub session_id: String,

    /// Source timestamp (when the event occurred, not when ingested)
    /// Per ING-04: Used for ordering
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: EventType,

    /// Role of the author
    pub role: EventRole,

    /// Event content/text
    pub text: String,

    /// Additional metadata (tool names, file paths, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Event {
    /// Create a new event with the given parameters
    pub fn new(
        event_id: String,
        session_id: String,
        timestamp: DateTime<Utc>,
        event_type: EventType,
        role: EventRole,
        text: String,
    ) -> Self {
        Self {
            event_id,
            session_id,
            timestamp,
            event_type,
            role,
            text,
            metadata: HashMap::new(),
        }
    }

    /// Create a new event with metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get timestamp as milliseconds since Unix epoch
    pub fn timestamp_ms(&self) -> i64 {
        self.timestamp.timestamp_millis()
    }

    /// Serialize event to JSON bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize event from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = Event::new(
            "01HN4QXKN6YWXVKZ3JMHP4BCDE".to_string(),
            "session-123".to_string(),
            Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            "Hello, world!".to_string(),
        );

        let bytes = event.to_bytes().unwrap();
        let decoded = Event::from_bytes(&bytes).unwrap();

        assert_eq!(event.event_id, decoded.event_id);
        assert_eq!(event.session_id, decoded.session_id);
        assert_eq!(event.text, decoded.text);
    }

    #[test]
    fn test_event_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("tool_name".to_string(), "Read".to_string());
        metadata.insert("file_path".to_string(), "/tmp/test.rs".to_string());

        let event = Event::new(
            "01HN4QXKN6YWXVKZ3JMHP4BCDE".to_string(),
            "session-123".to_string(),
            Utc::now(),
            EventType::ToolResult,
            EventRole::Tool,
            "File contents here".to_string(),
        )
        .with_metadata(metadata);

        assert_eq!(event.metadata.get("tool_name"), Some(&"Read".to_string()));
    }
}
