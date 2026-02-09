//! CCH hook handler for agent-memory.
//!
//! This binary reads CCH (code_agent_context_hooks) JSON events from stdin,
//! converts them to memory events, and sends them to the memory-daemon via gRPC.
//!
//! It always outputs `{"continue":true}` to stdout, even if ingestion fails,
//! to avoid blocking Claude Code (fail-open behavior).
//!
//! # Usage
//!
//! ```bash
//! echo '{"hook_event_name":"UserPromptSubmit","session_id":"test","message":"Hello"}' | memory-ingest
//! ```

use std::io::{self, BufRead};

use chrono::{DateTime, Utc};
use memory_client::{map_hook_event, HookEvent, HookEventType, MemoryClient};
use serde::Deserialize;

/// CCH event format from code_agent_context_hooks.
#[derive(Debug, Deserialize)]
struct CchEvent {
    /// Event type name (e.g., "SessionStart", "UserPromptSubmit")
    hook_event_name: String,
    /// Session identifier
    session_id: String,
    /// Message content (for prompts/responses)
    #[serde(default)]
    message: Option<String>,
    /// Tool name (for tool events)
    #[serde(default)]
    tool_name: Option<String>,
    /// Tool input JSON (for tool events)
    #[serde(default)]
    tool_input: Option<serde_json::Value>,
    /// Event timestamp
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
    /// Current working directory
    #[serde(default)]
    cwd: Option<String>,
    /// Agent identifier (e.g., "opencode", "claude")
    #[serde(default)]
    agent: Option<String>,
}

/// Map CCH event name to HookEventType.
fn map_cch_event_type(name: &str) -> HookEventType {
    match name {
        "SessionStart" => HookEventType::SessionStart,
        "UserPromptSubmit" => HookEventType::UserPromptSubmit,
        "AssistantResponse" => HookEventType::AssistantResponse,
        "PreToolUse" => HookEventType::ToolUse,
        "PostToolUse" => HookEventType::ToolResult,
        "Stop" | "SessionEnd" => HookEventType::Stop,
        "SubagentStart" => HookEventType::SubagentStart,
        "SubagentStop" => HookEventType::SubagentStop,
        // Default to user prompt for unknown types
        _ => HookEventType::UserPromptSubmit,
    }
}

/// Convert CchEvent to HookEvent.
fn map_cch_to_hook(cch: &CchEvent) -> HookEvent {
    let event_type = map_cch_event_type(&cch.hook_event_name);

    // Build content from message or tool_input
    let content = if let Some(msg) = &cch.message {
        msg.clone()
    } else if let Some(input) = &cch.tool_input {
        serde_json::to_string(input).unwrap_or_default()
    } else {
        String::new()
    };

    let mut hook = HookEvent::new(&cch.session_id, event_type, content);

    // Add optional fields
    if let Some(ts) = cch.timestamp {
        hook = hook.with_timestamp(ts);
    }
    if let Some(tool) = &cch.tool_name {
        hook = hook.with_tool_name(tool);
    }
    if let Some(cwd) = &cch.cwd {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("cwd".to_string(), cwd.clone());
        hook = hook.with_metadata(metadata);
    }
    if let Some(agent) = &cch.agent {
        hook = hook.with_agent(agent.clone());
    }

    hook
}

/// Output success response to CCH.
fn output_success() {
    println!(r#"{{"continue":true}}"#);
}

fn main() {
    // Read single line from stdin
    let stdin = io::stdin();
    let mut input = String::new();
    if stdin.lock().read_line(&mut input).is_err() {
        // Can't read stdin, but still succeed (fail-open)
        output_success();
        return;
    }

    // Parse CCH event
    let cch: CchEvent = match serde_json::from_str(&input) {
        Ok(event) => event,
        Err(_) => {
            // Invalid JSON, but still succeed (fail-open)
            output_success();
            return;
        }
    };

    // Map to memory event
    let hook_event = map_cch_to_hook(&cch);
    let event = map_hook_event(hook_event);

    // Attempt to ingest via gRPC (fail-open)
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => {
            output_success();
            return;
        }
    };

    rt.block_on(async {
        if let Ok(mut client) = MemoryClient::connect_default().await {
            // Ignore result - fail-open
            let _ = client.ingest(event).await;
        }
    });

    // Always return success to CCH
    output_success();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_start() {
        let json = r#"{"hook_event_name":"SessionStart","session_id":"test-123"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();

        assert_eq!(cch.hook_event_name, "SessionStart");
        assert_eq!(cch.session_id, "test-123");
        assert!(cch.message.is_none());
    }

    #[test]
    fn test_parse_user_prompt() {
        let json = r#"{"hook_event_name":"UserPromptSubmit","session_id":"test-123","message":"Hello world"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();

        assert_eq!(cch.hook_event_name, "UserPromptSubmit");
        assert_eq!(cch.session_id, "test-123");
        assert_eq!(cch.message, Some("Hello world".to_string()));
    }

    #[test]
    fn test_parse_tool_use() {
        let json = r#"{"hook_event_name":"PreToolUse","session_id":"test-123","tool_name":"Read","tool_input":{"path":"/test.rs"}}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();

        assert_eq!(cch.hook_event_name, "PreToolUse");
        assert_eq!(cch.tool_name, Some("Read".to_string()));
        assert!(cch.tool_input.is_some());
    }

    #[test]
    fn test_parse_with_timestamp() {
        let json = r#"{"hook_event_name":"UserPromptSubmit","session_id":"test-123","message":"Hello","timestamp":"2026-01-30T12:00:00Z"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();

        assert!(cch.timestamp.is_some());
    }

    #[test]
    fn test_parse_with_cwd() {
        let json = r#"{"hook_event_name":"SessionStart","session_id":"test-123","cwd":"/home/user/project"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();

        assert_eq!(cch.cwd, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_map_cch_event_type_all_types() {
        assert!(matches!(
            map_cch_event_type("SessionStart"),
            HookEventType::SessionStart
        ));
        assert!(matches!(
            map_cch_event_type("UserPromptSubmit"),
            HookEventType::UserPromptSubmit
        ));
        assert!(matches!(
            map_cch_event_type("AssistantResponse"),
            HookEventType::AssistantResponse
        ));
        assert!(matches!(
            map_cch_event_type("PreToolUse"),
            HookEventType::ToolUse
        ));
        assert!(matches!(
            map_cch_event_type("PostToolUse"),
            HookEventType::ToolResult
        ));
        assert!(matches!(map_cch_event_type("Stop"), HookEventType::Stop));
        assert!(matches!(
            map_cch_event_type("SessionEnd"),
            HookEventType::Stop
        ));
        assert!(matches!(
            map_cch_event_type("SubagentStart"),
            HookEventType::SubagentStart
        ));
        assert!(matches!(
            map_cch_event_type("SubagentStop"),
            HookEventType::SubagentStop
        ));
        // Unknown defaults to UserPromptSubmit
        assert!(matches!(
            map_cch_event_type("UnknownType"),
            HookEventType::UserPromptSubmit
        ));
    }

    #[test]
    fn test_map_cch_to_hook_basic() {
        let cch = CchEvent {
            hook_event_name: "UserPromptSubmit".to_string(),
            session_id: "test-123".to_string(),
            message: Some("Hello world".to_string()),
            tool_name: None,
            tool_input: None,
            timestamp: None,
            cwd: None,
            agent: None,
        };

        let hook = map_cch_to_hook(&cch);

        assert_eq!(hook.session_id, "test-123");
        assert_eq!(hook.content, "Hello world");
        assert!(matches!(hook.event_type, HookEventType::UserPromptSubmit));
    }

    #[test]
    fn test_map_cch_to_hook_with_tool() {
        let cch = CchEvent {
            hook_event_name: "PreToolUse".to_string(),
            session_id: "test-123".to_string(),
            message: None,
            tool_name: Some("Read".to_string()),
            tool_input: Some(serde_json::json!({"path": "/test.rs"})),
            timestamp: None,
            cwd: None,
            agent: None,
        };

        let hook = map_cch_to_hook(&cch);

        assert!(matches!(hook.event_type, HookEventType::ToolUse));
        assert_eq!(hook.tool_name, Some("Read".to_string()));
        // Content should be serialized tool_input
        assert!(hook.content.contains("path"));
    }

    #[test]
    fn test_map_cch_to_hook_with_timestamp() {
        use chrono::TimeZone;
        let ts = Utc.with_ymd_and_hms(2026, 1, 30, 12, 0, 0).unwrap();

        let cch = CchEvent {
            hook_event_name: "UserPromptSubmit".to_string(),
            session_id: "test-123".to_string(),
            message: Some("Hello".to_string()),
            tool_name: None,
            tool_input: None,
            timestamp: Some(ts),
            cwd: None,
            agent: None,
        };

        let hook = map_cch_to_hook(&cch);

        assert_eq!(hook.timestamp, Some(ts));
    }

    #[test]
    fn test_map_cch_to_hook_with_cwd() {
        let cch = CchEvent {
            hook_event_name: "SessionStart".to_string(),
            session_id: "test-123".to_string(),
            message: None,
            tool_name: None,
            tool_input: None,
            timestamp: None,
            cwd: Some("/home/user".to_string()),
            agent: None,
        };

        let hook = map_cch_to_hook(&cch);

        assert!(hook.metadata.is_some());
        let metadata = hook.metadata.unwrap();
        assert_eq!(metadata.get("cwd"), Some(&"/home/user".to_string()));
    }

    #[test]
    fn test_end_to_end_mapping() {
        // Test full pipeline: CCH JSON -> CchEvent -> HookEvent -> Event
        let json = r#"{"hook_event_name":"UserPromptSubmit","session_id":"test-123","message":"Hello world"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();
        let hook = map_cch_to_hook(&cch);
        let event = map_hook_event(hook);

        assert_eq!(event.session_id, "test-123");
        assert_eq!(event.text, "Hello world");
        assert_eq!(event.event_type, memory_types::EventType::UserMessage);
        assert_eq!(event.role, memory_types::EventRole::User);
    }

    #[test]
    fn test_parse_with_agent() {
        let json = r#"{"hook_event_name":"SessionStart","session_id":"test-123","agent":"opencode"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();
        assert_eq!(cch.agent, Some("opencode".to_string()));
    }

    #[test]
    fn test_parse_without_agent_backward_compat() {
        let json = r#"{"hook_event_name":"SessionStart","session_id":"test-123"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();
        assert!(cch.agent.is_none());
    }

    #[test]
    fn test_end_to_end_with_agent() {
        let json = r#"{"hook_event_name":"UserPromptSubmit","session_id":"test-123","message":"Hello","agent":"opencode"}"#;
        let cch: CchEvent = serde_json::from_str(json).unwrap();
        let hook = map_cch_to_hook(&cch);
        let event = map_hook_event(hook);
        assert_eq!(event.agent, Some("opencode".to_string()));
    }
}
