//! Hook event mapping to memory events.
//!
//! Per HOOK-03: Event types map 1:1 from hook events
//! (SessionStart, UserPromptSubmit, PostToolUse, Stop, etc.)

use chrono::{DateTime, Utc};
use memory_types::{Event, EventRole, EventType};

/// Hook event types from code_agent_context_hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookEventType {
    /// Session started
    SessionStart,
    /// User submitted a prompt
    UserPromptSubmit,
    /// Assistant generated a response
    AssistantResponse,
    /// Tool was used (e.g., Read, Write, Bash)
    ToolUse,
    /// Tool result received
    ToolResult,
    /// Session stopped/ended
    Stop,
    /// Subagent started
    SubagentStart,
    /// Subagent stopped
    SubagentStop,
}

/// A hook event to be mapped to a memory event.
#[derive(Debug, Clone)]
pub struct HookEvent {
    /// Session identifier
    pub session_id: String,
    /// Type of hook event
    pub event_type: HookEventType,
    /// Event content/text
    pub content: String,
    /// Optional timestamp (uses current time if None)
    pub timestamp: Option<DateTime<Utc>>,
    /// Optional tool name for ToolUse/ToolResult events
    pub tool_name: Option<String>,
    /// Optional metadata
    pub metadata: Option<std::collections::HashMap<String, String>>,
    /// Optional agent identifier (e.g., "opencode", "claude", "gemini")
    pub agent: Option<String>,
}

impl HookEvent {
    /// Create a new hook event.
    pub fn new(
        session_id: impl Into<String>,
        event_type: HookEventType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            event_type,
            content: content.into(),
            timestamp: None,
            tool_name: None,
            metadata: None,
            agent: None,
        }
    }

    /// Set the timestamp.
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the tool name.
    pub fn with_tool_name(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set the agent identifier.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }
}

/// Map a hook event to a memory event.
///
/// Per HOOK-03: Event types map 1:1 from hook events.
pub fn map_hook_event(hook: HookEvent) -> Event {
    let event_type = match hook.event_type {
        HookEventType::SessionStart => EventType::SessionStart,
        HookEventType::UserPromptSubmit => EventType::UserMessage,
        HookEventType::AssistantResponse => EventType::AssistantMessage,
        HookEventType::ToolUse => EventType::ToolResult, // Tool invocation
        HookEventType::ToolResult => EventType::ToolResult,
        HookEventType::Stop => EventType::SessionEnd,
        HookEventType::SubagentStart => EventType::SubagentStart,
        HookEventType::SubagentStop => EventType::SubagentStop,
    };

    let role = match hook.event_type {
        HookEventType::UserPromptSubmit => EventRole::User,
        HookEventType::AssistantResponse => EventRole::Assistant,
        HookEventType::ToolUse | HookEventType::ToolResult => EventRole::Tool,
        HookEventType::SessionStart | HookEventType::Stop => EventRole::System,
        HookEventType::SubagentStart | HookEventType::SubagentStop => EventRole::System,
    };

    let timestamp = hook.timestamp.unwrap_or_else(Utc::now);
    let event_id = ulid::Ulid::new().to_string();

    let mut event = Event::new(
        event_id,
        hook.session_id,
        timestamp,
        event_type,
        role,
        hook.content,
    );

    // Add tool name to metadata if present
    if let Some(tool_name) = hook.tool_name {
        let mut metadata = hook.metadata.unwrap_or_default();
        metadata.insert("tool_name".to_string(), tool_name);
        event = event.with_metadata(metadata);
    } else if let Some(metadata) = hook.metadata {
        event = event.with_metadata(metadata);
    }

    // Propagate agent identifier
    if let Some(agent) = hook.agent {
        event = event.with_agent(agent);
    }

    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_session_start() {
        let hook = HookEvent::new("session-1", HookEventType::SessionStart, "Session started");
        let event = map_hook_event(hook);

        assert_eq!(event.session_id, "session-1");
        assert_eq!(event.event_type, EventType::SessionStart);
        assert_eq!(event.role, EventRole::System);
    }

    #[test]
    fn test_map_user_prompt() {
        let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Hello!");
        let event = map_hook_event(hook);

        assert_eq!(event.event_type, EventType::UserMessage);
        assert_eq!(event.role, EventRole::User);
        assert_eq!(event.text, "Hello!");
    }

    #[test]
    fn test_map_assistant_response() {
        let hook = HookEvent::new("session-1", HookEventType::AssistantResponse, "Hi there!");
        let event = map_hook_event(hook);

        assert_eq!(event.event_type, EventType::AssistantMessage);
        assert_eq!(event.role, EventRole::Assistant);
    }

    #[test]
    fn test_map_tool_use_with_name() {
        let hook = HookEvent::new("session-1", HookEventType::ToolUse, "Reading file...")
            .with_tool_name("Read");
        let event = map_hook_event(hook);

        assert_eq!(event.event_type, EventType::ToolResult);
        assert_eq!(event.role, EventRole::Tool);
        assert_eq!(event.metadata.get("tool_name"), Some(&"Read".to_string()));
    }

    #[test]
    fn test_map_stop() {
        let hook = HookEvent::new("session-1", HookEventType::Stop, "Session ended");
        let event = map_hook_event(hook);

        assert_eq!(event.event_type, EventType::SessionEnd);
        assert_eq!(event.role, EventRole::System);
    }

    #[test]
    fn test_with_custom_timestamp() {
        use chrono::TimeZone;
        let timestamp = Utc.with_ymd_and_hms(2026, 1, 30, 12, 0, 0).unwrap();
        let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Test")
            .with_timestamp(timestamp);
        let event = map_hook_event(hook);

        assert_eq!(event.timestamp, timestamp);
    }

    #[test]
    fn test_with_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());

        let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Test")
            .with_metadata(metadata);
        let event = map_hook_event(hook);

        assert_eq!(event.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_map_with_agent() {
        let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Test")
            .with_agent("opencode");
        let event = map_hook_event(hook);
        assert_eq!(event.agent, Some("opencode".to_string()));
    }

    #[test]
    fn test_map_without_agent() {
        let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Test");
        let event = map_hook_event(hook);
        assert!(event.agent.is_none());
    }
}
