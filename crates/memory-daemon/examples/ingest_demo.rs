//! Demo: Ingest sample conversation events
//!
//! Usage:
//! ```bash
//! cargo run --example ingest_demo
//! ```
//!
//! This example connects to a running daemon and ingests a sample
//! conversation about the agent-memory system.

use memory_client::{map_hook_event, HookEvent, HookEventType, MemoryClient};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let endpoint =
        std::env::var("MEMORY_ENDPOINT").unwrap_or_else(|_| "http://[::1]:50051".to_string());

    println!("Connecting to memory daemon at {}...", endpoint);

    let mut client = MemoryClient::connect(&endpoint).await?;

    let session_id = format!("demo-{}", chrono::Utc::now().timestamp());
    println!("Session ID: {}", session_id);
    println!();

    // Simulate a conversation
    let conversation = vec![
        (HookEventType::SessionStart, "Demo session started"),
        (
            HookEventType::UserPromptSubmit,
            "What is the agent-memory system?",
        ),
        (
            HookEventType::AssistantResponse,
            "Agent-memory is a local, append-only storage system for AI agent conversations. \
             It provides TOC-based navigation for efficient recall without scanning everything.",
        ),
        (HookEventType::UserPromptSubmit, "How does the TOC work?"),
        (
            HookEventType::AssistantResponse,
            "The TOC (Table of Contents) organizes events in a time hierarchy: \
             Year -> Month -> Week -> Day -> Segment. Each level has summaries \
             generated from the content below it.",
        ),
        (HookEventType::UserPromptSubmit, "What are grips?"),
        (
            HookEventType::AssistantResponse,
            "Grips are anchors that link summaries back to source evidence. \
             Each grip contains an excerpt and pointers to the original events, \
             providing provenance for summary claims.",
        ),
        (
            HookEventType::UserPromptSubmit,
            "Can you show me a tool use?",
        ),
        (HookEventType::ToolUse, "Reading file /tmp/example.txt..."),
        (
            HookEventType::ToolResult,
            "File contents: Hello from the example file!",
        ),
        (
            HookEventType::AssistantResponse,
            "I've demonstrated a tool use. The Read tool was used to access a file, \
             and the result was captured as a ToolResult event.",
        ),
        (HookEventType::Stop, "Demo session ended"),
    ];

    println!("Ingesting {} events...", conversation.len());
    println!();

    for (event_type, content) in &conversation {
        let hook_event = HookEvent::new(session_id.clone(), event_type.clone(), *content);

        // Add tool name for tool events
        let hook_event = if matches!(
            event_type,
            HookEventType::ToolUse | HookEventType::ToolResult
        ) {
            hook_event.with_tool_name("Read")
        } else {
            hook_event
        };

        let event = map_hook_event(hook_event);
        let (event_id, created) = client.ingest(event).await?;

        if created {
            println!(
                "  [{:15}] {} -> {}",
                event_type_name(event_type),
                truncate(content, 50),
                &event_id[..8]
            );
        }

        // Small delay between events for realistic timing
        sleep(Duration::from_millis(50)).await;
    }

    println!();
    println!(
        "Successfully ingested {} events for session {}",
        conversation.len(),
        &session_id
    );
    println!();
    println!("You can now query the events using:");
    println!(
        "  cargo run --bin memory-daemon -- query --endpoint {} root",
        endpoint
    );
    println!("  cargo run --bin memory-daemon -- query --endpoint {} events --from <timestamp> --to <timestamp>", endpoint);

    Ok(())
}

fn event_type_name(t: &HookEventType) -> &'static str {
    match t {
        HookEventType::SessionStart => "SessionStart",
        HookEventType::UserPromptSubmit => "UserPrompt",
        HookEventType::AssistantResponse => "Assistant",
        HookEventType::ToolUse => "ToolUse",
        HookEventType::ToolResult => "ToolResult",
        HookEventType::Stop => "Stop",
        HookEventType::SubagentStart => "SubagentStart",
        HookEventType::SubagentStop => "SubagentStop",
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
