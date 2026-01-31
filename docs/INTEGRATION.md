# Integration Guide

## Overview

Agent Memory integrates with AI coding agents through hook handlers that capture conversation events and send them to the memory daemon via gRPC.

## Using the Client Library

The `memory-client` crate provides a Rust client for integrating with the daemon.

### Add Dependency

```toml
[dependencies]
memory-client = { path = "../agent-memory/crates/memory-client" }
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use memory_client::{MemoryClient, HookEvent, HookEventType, map_hook_event};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to daemon
    let mut client = MemoryClient::connect("http://[::1]:50051").await?;

    // Create a hook event
    let hook_event = HookEvent::new(
        "session-123",
        HookEventType::UserPromptSubmit,
        "What is Rust?",
    );

    // Map to memory event and ingest
    let event = map_hook_event(hook_event);
    let (event_id, created) = client.ingest(event).await?;

    println!("Event {}: created={}", event_id, created);
    Ok(())
}
```

### Event Types

The `HookEventType` enum maps to memory event types:

| HookEventType | Memory EventType | Role | Description |
|---------------|------------------|------|-------------|
| `SessionStart` | SESSION_START | System | Session begins |
| `UserPromptSubmit` | USER_MESSAGE | User | User sends prompt |
| `AssistantResponse` | ASSISTANT_MESSAGE | Assistant | AI responds |
| `ToolUse` | TOOL_RESULT | Tool | Tool invoked |
| `ToolResult` | TOOL_RESULT | Tool | Tool output |
| `Stop` | SESSION_END | System | Session ends |
| `SubagentStart` | SUBAGENT_START | System | Subagent spawned |
| `SubagentStop` | SUBAGENT_STOP | System | Subagent completes |

### Adding Metadata

```rust
use std::collections::HashMap;

let mut metadata = HashMap::new();
metadata.insert("tool_name".to_string(), "Read".to_string());
metadata.insert("file_path".to_string(), "/path/to/file.rs".to_string());

let hook_event = HookEvent::new("session-123", HookEventType::ToolResult, "File contents...")
    .with_tool_name("Read")
    .with_metadata(metadata);

let event = map_hook_event(hook_event);
```

### Custom Timestamp

```rust
use chrono::{TimeZone, Utc};

let timestamp = Utc.with_ymd_and_hms(2026, 1, 30, 12, 0, 0).unwrap();
let hook_event = HookEvent::new("session-123", HookEventType::UserPromptSubmit, "Hello")
    .with_timestamp(timestamp);
```

### Query Operations

```rust
// Get TOC root (year nodes)
let root_nodes = client.get_toc_root().await?;

// Get specific node
let node = client.get_node("toc:year:2026").await?;

// Browse children with pagination
let result = client.browse_toc("toc:year:2026", 10, None).await?;
if result.has_more {
    let next_page = client.browse_toc("toc:year:2026", 10, result.continuation_token).await?;
}

// Get events in time range
let result = client.get_events(from_ms, to_ms, limit).await?;

// Expand grip context
let context = client.expand_grip("grip:123:abc", Some(3), Some(3)).await?;
```

## Claude Code Hook Integration

### Hook Configuration

In your Claude Code hooks configuration (`.claude/hooks.json`):

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "command": "memory-hook ingest user-prompt",
        "env": {
          "SESSION_ID": "${SESSION_ID}",
          "MEMORY_ENDPOINT": "http://[::1]:50051"
        }
      }
    ],
    "AssistantResponse": [
      {
        "command": "memory-hook ingest assistant-response"
      }
    ],
    "PostToolUse": [
      {
        "command": "memory-hook ingest tool-result"
      }
    ],
    "Stop": [
      {
        "command": "memory-hook ingest stop"
      }
    ]
  }
}
```

### Hook Handler Implementation

Example hook handler script (`memory-hook`):

```rust
use memory_client::{MemoryClient, HookEvent, HookEventType, map_hook_event};
use std::io::{self, Read};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Get event type from args
    let event_type = match args.get(2).map(|s| s.as_str()) {
        Some("user-prompt") => HookEventType::UserPromptSubmit,
        Some("assistant-response") => HookEventType::AssistantResponse,
        Some("tool-result") => HookEventType::ToolResult,
        Some("stop") => HookEventType::Stop,
        _ => return Err("Unknown event type".into()),
    };

    // Read content from stdin
    let mut content = String::new();
    io::stdin().read_to_string(&mut content)?;

    // Get session ID from env
    let session_id = std::env::var("SESSION_ID")?;
    let endpoint = std::env::var("MEMORY_ENDPOINT")
        .unwrap_or_else(|_| "http://[::1]:50051".to_string());

    // Connect and ingest
    let mut client = MemoryClient::connect(&endpoint).await?;
    let hook_event = HookEvent::new(session_id, event_type, content);
    let event = map_hook_event(hook_event);
    client.ingest(event).await?;

    Ok(())
}
```

## Error Handling

### Client Errors

```rust
use memory_client::ClientError;

match client.ingest(event).await {
    Ok((event_id, created)) => {
        println!("Success: {} (created: {})", event_id, created);
    }
    Err(ClientError::Connection(e)) => {
        eprintln!("Connection failed: {}", e);
        // Retry or queue for later
    }
    Err(ClientError::InvalidArgument(msg)) => {
        eprintln!("Bad input: {}", msg);
        // Fix input and retry
    }
    Err(ClientError::Internal(msg)) => {
        eprintln!("Server error: {}", msg);
        // Log and alert
    }
}
```

### Retry with Backoff

```rust
use tokio::time::{sleep, Duration};

async fn ingest_with_retry(
    client: &mut MemoryClient,
    event: Event,
    max_attempts: u32,
) -> Result<String, ClientError> {
    let mut attempts = 0;
    let mut delay = Duration::from_millis(100);

    loop {
        match client.ingest(event.clone()).await {
            Ok((event_id, _)) => return Ok(event_id),
            Err(ClientError::Connection(_)) if attempts < max_attempts => {
                attempts += 1;
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Direct gRPC Integration

For non-Rust clients, use gRPC directly.

### Proto Definition

```protobuf
service MemoryService {
    rpc IngestEvent(IngestEventRequest) returns (IngestEventResponse);
    rpc GetTocRoot(GetTocRootRequest) returns (GetTocRootResponse);
    rpc GetNode(GetNodeRequest) returns (GetNodeResponse);
    rpc BrowseToc(BrowseTocRequest) returns (BrowseTocResponse);
    rpc GetEvents(GetEventsRequest) returns (GetEventsResponse);
    rpc ExpandGrip(ExpandGripRequest) returns (ExpandGripResponse);
}
```

### grpcurl Examples

```bash
# Ingest event
grpcurl -plaintext -d '{
  "event": {
    "event_id": "01HXYZ...",
    "session_id": "session-123",
    "timestamp_ms": 1706600000000,
    "event_type": 2,
    "role": 1,
    "text": "What is Rust?"
  }
}' localhost:50051 memory.MemoryService/IngestEvent

# Get TOC root
grpcurl -plaintext localhost:50051 memory.MemoryService/GetTocRoot

# Get specific node
grpcurl -plaintext -d '{"node_id": "toc:year:2026"}' \
  localhost:50051 memory.MemoryService/GetNode
```

### Python Example

```python
import grpc
import memory_pb2
import memory_pb2_grpc

# Connect
channel = grpc.insecure_channel('localhost:50051')
stub = memory_pb2_grpc.MemoryServiceStub(channel)

# Ingest
event = memory_pb2.Event(
    event_id="01HXYZ...",
    session_id="session-123",
    timestamp_ms=1706600000000,
    event_type=memory_pb2.EVENT_TYPE_USER_MESSAGE,
    role=memory_pb2.EVENT_ROLE_USER,
    text="What is Rust?",
)
request = memory_pb2.IngestEventRequest(event=event)
response = stub.IngestEvent(request)
print(f"Event {response.event_id}: created={response.created}")

# Query
root_response = stub.GetTocRoot(memory_pb2.GetTocRootRequest())
for node in root_response.nodes:
    print(f"{node.node_id}: {node.title}")
```

## Best Practices

### Session Management

- Use consistent session IDs across a conversation
- Generate new session ID on explicit restart
- Include session context in metadata

### Event Ordering

- Events are ordered by timestamp, not ingestion time
- Use accurate timestamps for proper ordering
- ULID event_id provides secondary ordering within millisecond

### Idempotency

- IngestEvent is idempotent on event_id
- Safe to retry on transient failures
- `created=false` indicates duplicate

### Connection Pooling

For high-throughput applications:

```rust
// Create client once, reuse for multiple operations
let mut client = MemoryClient::connect(endpoint).await?;

// All operations on same connection
for event in events {
    client.ingest(event).await?;
}
```

### Graceful Shutdown

Handle shutdown signals to complete in-flight operations:

```rust
use tokio::signal;

tokio::select! {
    result = client.ingest(event) => {
        // Handle result
    }
    _ = signal::ctrl_c() => {
        // Cleanup and exit
    }
}
```
