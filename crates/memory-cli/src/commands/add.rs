//! `memory add` command -- ingest a new memory event via gRPC.

use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use ulid::Ulid;

use memory_types::{Event, EventRole, EventType};

use crate::cli::{AddArgs, GlobalArgs};
use crate::output::{estimate_tokens, print_output, should_force_json, JsonEnvelope, Meta};

/// Map a CLI kind string to the corresponding `EventType`.
fn kind_to_event_type(kind: &str) -> EventType {
    match kind {
        "episodic" | "user_message" => EventType::UserMessage,
        "tool_result" => EventType::ToolResult,
        "assistant" | "assistant_message" => EventType::AssistantMessage,
        "session_start" => EventType::SessionStart,
        "session_end" => EventType::SessionEnd,
        _ => EventType::UserMessage,
    }
}

/// Build an `Event` from CLI arguments.
fn build_event(content: &str, kind: &str, agent: Option<&str>) -> Event {
    let event_id = Ulid::new().to_string();
    let session_id = format!("cli-{}", Ulid::new());
    let timestamp = Utc::now();
    let event_type = kind_to_event_type(kind);
    let role = EventRole::User;

    let event = Event::new(event_id, session_id, timestamp, event_type, role, content.to_string());

    match agent {
        Some(a) => event.with_agent(a),
        None => event,
    }
}

/// Run the `memory add` command.
pub async fn run(args: AddArgs, global: &GlobalArgs) -> Result<()> {
    let force_json = should_force_json(&global.format, &None);

    let mut client = match crate::client::connect_client(&global.endpoint).await {
        Ok(c) => c,
        Err(err) => {
            let envelope = JsonEnvelope::error(&format!("{err:#}"));
            print_output(&envelope, force_json);
            std::process::exit(1);
        }
    };

    let event = build_event(&args.content, &args.kind, args.agent.as_deref());
    let event_id = event.event_id.clone();

    match client.ingest(event).await {
        Ok((id, created)) => {
            let result_id = if id.is_empty() { event_id } else { id };
            let envelope = JsonEnvelope::ok(
                "add",
                json!({
                    "event_id": result_id,
                    "created": created,
                }),
            )
            .with_meta(Meta {
                retrieval_ms: 0,
                tokens_estimated: estimate_tokens(&args.content),
                confidence: 1.0,
            });
            print_output(&envelope, force_json);
            Ok(())
        }
        Err(err) => {
            let envelope = JsonEnvelope::error(&format!("{err:#}"));
            print_output(&envelope, force_json);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_to_event_type_episodic() {
        assert!(matches!(kind_to_event_type("episodic"), EventType::UserMessage));
    }

    #[test]
    fn test_kind_to_event_type_user_message() {
        assert!(matches!(kind_to_event_type("user_message"), EventType::UserMessage));
    }

    #[test]
    fn test_kind_to_event_type_tool_result() {
        assert!(matches!(kind_to_event_type("tool_result"), EventType::ToolResult));
    }

    #[test]
    fn test_kind_to_event_type_assistant() {
        assert!(matches!(kind_to_event_type("assistant"), EventType::AssistantMessage));
    }

    #[test]
    fn test_kind_to_event_type_assistant_message() {
        assert!(matches!(kind_to_event_type("assistant_message"), EventType::AssistantMessage));
    }

    #[test]
    fn test_kind_to_event_type_session_start() {
        assert!(matches!(kind_to_event_type("session_start"), EventType::SessionStart));
    }

    #[test]
    fn test_kind_to_event_type_session_end() {
        assert!(matches!(kind_to_event_type("session_end"), EventType::SessionEnd));
    }

    #[test]
    fn test_kind_to_event_type_unknown_defaults() {
        assert!(matches!(kind_to_event_type("unknown_kind"), EventType::UserMessage));
    }

    #[test]
    fn test_build_event_episodic_no_agent() {
        let event = build_event("hello", "episodic", None);
        assert!(!event.event_id.is_empty());
        assert!(event.session_id.starts_with("cli-"));
        assert!(matches!(event.event_type, EventType::UserMessage));
        assert!(matches!(event.role, EventRole::User));
        assert_eq!(event.text, "hello");
        assert!(event.agent.is_none());
    }

    #[test]
    fn test_build_event_tool_result_with_agent() {
        let event = build_event("note", "tool_result", Some("claude"));
        assert!(matches!(event.event_type, EventType::ToolResult));
        assert_eq!(event.agent.as_deref(), Some("claude"));
        assert_eq!(event.text, "note");
    }

    #[test]
    fn test_build_event_generates_unique_ids() {
        let e1 = build_event("a", "episodic", None);
        let e2 = build_event("b", "episodic", None);
        assert_ne!(e1.event_id, e2.event_id);
        assert_ne!(e1.session_id, e2.session_id);
    }
}
