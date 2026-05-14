//! `memory timeline` command -- browse events by time range via gRPC.

use anyhow::Result;
use chrono::Utc;
use serde_json::json;

use memory_client::ProtoEvent;

use crate::cli::{GlobalArgs, TimelineArgs};
use crate::output::{estimate_tokens, print_output, should_force_json, JsonEnvelope, Meta};

/// Parse a range string like "7d", "30d", "1w" into `(from_ms, to_ms)`.
pub(crate) fn parse_range(range: &str) -> (i64, i64) {
    let now = Utc::now().timestamp_millis();
    let range = range.trim();

    let duration_ms = if let Some(stripped) = range.strip_suffix('d') {
        stripped
            .parse::<i64>()
            .unwrap_or(7)
            .max(1)
            .saturating_mul(86_400_000)
    } else if let Some(stripped) = range.strip_suffix('w') {
        stripped
            .parse::<i64>()
            .unwrap_or(1)
            .max(1)
            .saturating_mul(7 * 86_400_000)
    } else {
        // Default: 7 days
        7 * 86_400_000
    };

    (now - duration_ms, now)
}

/// Map a proto event_type i32 to a human-readable string.
fn event_type_to_string(event_type: i32) -> &'static str {
    match event_type {
        1 => "session_start",
        2 => "user_message",
        3 => "assistant_message",
        4 => "tool_result",
        5 => "assistant_stop",
        6 => "subagent_start",
        7 => "subagent_stop",
        8 => "session_end",
        _ => "unknown",
    }
}

/// Map a proto role i32 to a human-readable string.
fn role_to_string(role: i32) -> &'static str {
    match role {
        1 => "user",
        2 => "assistant",
        3 => "system",
        4 => "tool",
        _ => "unknown",
    }
}

/// Map a `ProtoEvent` to a JSON value.
pub(crate) fn map_proto_event(e: &ProtoEvent) -> serde_json::Value {
    json!({
        "event_id": e.event_id,
        "session_id": e.session_id,
        "timestamp_ms": e.timestamp_ms,
        "event_type": event_type_to_string(e.event_type),
        "role": role_to_string(e.role),
        "text": e.text,
        "agent": e.agent.as_deref().unwrap_or(""),
    })
}

/// Run the `memory timeline` command.
pub async fn run(args: TimelineArgs, global: &GlobalArgs) -> Result<()> {
    let force_json = should_force_json(&global.format, &args.format);

    let mut client = match crate::client::connect_client(&global.endpoint).await {
        Ok(c) => c,
        Err(err) => {
            let envelope = JsonEnvelope::error(&format!("{err:#}"));
            print_output(&envelope, force_json);
            std::process::exit(1);
        }
    };

    let (from_ms, to_ms) = parse_range(&args.range);

    match client.get_events(from_ms, to_ms, 100).await {
        Ok(result) => {
            let mut events: Vec<serde_json::Value> =
                result.events.iter().map(map_proto_event).collect();

            // Client-side entity filtering if requested
            if let Some(ref entity) = args.entity {
                let entity_lower = entity.to_lowercase();
                events.retain(|e| {
                    e["text"]
                        .as_str()
                        .map(|t| t.to_lowercase().contains(&entity_lower))
                        .unwrap_or(false)
                });
            }

            let total_tokens: usize = result.events.iter().map(|e| estimate_tokens(&e.text)).sum();

            let envelope = JsonEnvelope::ok("timeline", json!(events)).with_meta(Meta {
                retrieval_ms: 0,
                tokens_estimated: total_tokens,
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
    fn test_parse_range_7d() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_range("7d");
        let expected_from = now - 7 * 86_400_000;
        // Allow 1 second tolerance
        assert!((to - now).abs() < 1000, "to should be ~now");
        assert!(
            (from - expected_from).abs() < 1000,
            "from should be ~7 days ago"
        );
    }

    #[test]
    fn test_parse_range_30d() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_range("30d");
        let expected_from = now - 30 * 86_400_000;
        assert!((to - now).abs() < 1000);
        assert!((from - expected_from).abs() < 1000);
    }

    #[test]
    fn test_parse_range_1w() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_range("1w");
        let expected_from = now - 7 * 86_400_000;
        assert!((to - now).abs() < 1000);
        assert!((from - expected_from).abs() < 1000);
    }

    #[test]
    fn test_parse_range_invalid_defaults_to_7d() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_range("invalid");
        let expected_from = now - 7 * 86_400_000;
        assert!((to - now).abs() < 1000);
        assert!((from - expected_from).abs() < 1000);
    }

    #[test]
    fn test_map_proto_event_fields() {
        let event = ProtoEvent {
            event_id: "evt-1".to_string(),
            session_id: "sess-1".to_string(),
            timestamp_ms: 1700000000000,
            event_type: 2, // user_message
            role: 1,       // user
            text: "hello world".to_string(),
            metadata: Default::default(),
            agent: Some("claude".to_string()),
        };
        let val = map_proto_event(&event);
        assert_eq!(val["event_id"], "evt-1");
        assert_eq!(val["session_id"], "sess-1");
        assert_eq!(val["timestamp_ms"], 1700000000000_i64);
        assert_eq!(val["event_type"], "user_message");
        assert_eq!(val["role"], "user");
        assert_eq!(val["text"], "hello world");
        assert_eq!(val["agent"], "claude");
    }

    #[test]
    fn test_event_type_to_string_all() {
        assert_eq!(event_type_to_string(0), "unknown");
        assert_eq!(event_type_to_string(1), "session_start");
        assert_eq!(event_type_to_string(2), "user_message");
        assert_eq!(event_type_to_string(3), "assistant_message");
        assert_eq!(event_type_to_string(4), "tool_result");
        assert_eq!(event_type_to_string(5), "assistant_stop");
        assert_eq!(event_type_to_string(6), "subagent_start");
        assert_eq!(event_type_to_string(7), "subagent_stop");
        assert_eq!(event_type_to_string(8), "session_end");
    }

    #[test]
    fn test_role_to_string_all() {
        assert_eq!(role_to_string(0), "unknown");
        assert_eq!(role_to_string(1), "user");
        assert_eq!(role_to_string(2), "assistant");
        assert_eq!(role_to_string(3), "system");
        assert_eq!(role_to_string(4), "tool");
    }
}
