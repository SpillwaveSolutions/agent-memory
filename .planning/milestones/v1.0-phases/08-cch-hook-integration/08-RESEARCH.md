# Phase 8: CCH Hook Integration - Research

**Researched:** 2026-01-30
**Domain:** CCH hooks, stdin/stdout binary, event ingestion
**Confidence:** HIGH

## Summary

This phase creates a `memory-ingest` binary that integrates with Code-Agent Context Hooks (CCH) to automatically capture conversation events. The binary reads CCH events from stdin, maps them using existing `memory-client` code, and ingests them via gRPC.

**Key insight:** All the hard work is done. The `memory-client` crate already has:
- `HookEvent` type matching CCH event types
- `map_hook_event()` function for conversion
- `MemoryClient` with `ingest()` for gRPC communication

The binary is just a thin stdin/stdout wrapper (~50 lines of Rust).

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| memory-client | local | HookEvent, map_hook_event, MemoryClient | Already built in Phase 5 |
| serde_json | 1.0 | Parse CCH JSON from stdin | Standard JSON handling |
| tokio | 1.43+ | Async runtime for gRPC | Workspace standard |
| clap | 4.5+ | CLI argument parsing | Workspace standard |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1 | Debug logging | Development |
| anyhow | 1.0 | Error handling | Simplifies main() |

## Architecture

```
CCH (hooks.yaml)                    Agent Memory
┌─────────────────┐                 ┌─────────────────┐
│  SessionStart   │─────┐           │                 │
│  UserPrompt     │     │   stdin   │  memory-ingest  │──────► memory-daemon
│  PostToolUse    │─────┼──────────►│  (reads JSON,   │  gRPC  (gRPC :50051)
│  SessionEnd     │     │           │   maps, sends)  │
└─────────────────┘─────┘           └────────┬────────┘
                                             │
                                             ▼ stdout
                                    {"continue": true}
```

## CCH Event Format

CCH sends JSON events on stdin (from CCH source code):

```json
{
  "hook_event_name": "UserPromptSubmit",
  "session_id": "abc123",
  "tool_name": null,
  "tool_input": null,
  "timestamp": "2026-01-30T12:00:00Z",
  "cwd": "/path/to/project",
  "transcript_path": "/path/to/transcript.jsonl",
  "message": "Hello, how are you?"
}
```

**Event types from CCH:**
- `SessionStart` - Session began
- `UserPromptSubmit` - User sent a message
- `PreToolUse` - Tool about to be used
- `PostToolUse` - Tool completed
- `Stop` - Session ended
- `SubagentStart` - Subagent spawned
- `SubagentStop` - Subagent completed

## Implementation Pattern

```rust
// crates/memory-ingest/src/main.rs

use std::io::{self, BufRead, Write};
use memory_client::{MemoryClient, HookEvent, HookEventType, map_hook_event};
use serde::Deserialize;

#[derive(Deserialize)]
struct CchEvent {
    hook_event_name: String,
    session_id: String,
    message: Option<String>,
    tool_name: Option<String>,
    timestamp: Option<String>,
    // ... other fields
}

fn main() -> anyhow::Result<()> {
    // Read JSON from stdin
    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;

    // Parse CCH event
    let cch: CchEvent = serde_json::from_str(&input)?;

    // Map to HookEvent
    let hook_event = map_cch_to_hook(&cch);

    // Map to memory Event
    let event = map_hook_event(hook_event);

    // Ingest via gRPC (requires tokio runtime)
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match MemoryClient::connect_default().await {
            Ok(mut client) => {
                let _ = client.ingest(event).await;
            }
            Err(_) => {
                // Fail open - don't block Claude if daemon is down
            }
        }
    });

    // Return success JSON to CCH
    let response = serde_json::json!({ "continue": true });
    println!("{}", response);

    Ok(())
}
```

## hooks.yaml Configuration

```yaml
# .claude/hooks.yaml
version: "1.0"

settings:
  fail_open: true     # If memory-ingest fails, don't block Claude
  script_timeout: 5   # 5 second timeout

rules:
  - name: capture-to-memory
    description: Send all events to agent-memory daemon
    matchers:
      operations:
        - SessionStart
        - UserPromptSubmit
        - PostToolUse
        - SessionEnd
        - SubagentStart
        - SubagentStop
    actions:
      run: "~/.local/bin/memory-ingest"
```

## Event Mapping

| CCH Event | HookEventType | Memory EventType |
|-----------|---------------|------------------|
| SessionStart | SessionStart | SessionStart |
| UserPromptSubmit | UserPromptSubmit | UserMessage |
| PreToolUse | ToolUse | ToolResult |
| PostToolUse | ToolResult | ToolResult |
| Stop | Stop | SessionEnd |
| SubagentStart | SubagentStart | SubagentStart |
| SubagentStop | SubagentStop | SubagentStop |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Event parsing | Custom struct | Existing HookEvent | Already tested |
| Event mapping | New logic | map_hook_event() | Phase 5 work |
| gRPC client | Raw HTTP | MemoryClient | Handles proto conversion |

## Common Pitfalls

### Pitfall 1: Blocking on gRPC

**What goes wrong:** Using async incorrectly causes hangs

**Solution:** Use `tokio::runtime::Runtime::new().block_on()` pattern

### Pitfall 2: Daemon Not Running

**What goes wrong:** Binary fails when daemon is down

**Solution:** Catch connection errors, fail open with `{"continue": true}`

### Pitfall 3: CCH Field Names

**What goes wrong:** CCH uses `hook_event_name` not `event_type`

**Solution:** Use proper serde field aliases or custom struct

## Verification

```bash
# Build binary
cargo build --release -p memory-ingest

# Test with sample event
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test","message":"Hello"}' | ./target/release/memory-ingest

# Should output: {"continue":true}

# Install to local bin
cp target/release/memory-ingest ~/.local/bin/

# Test with CCH (manual)
claude # Run Claude Code with hooks.yaml configured
```

## Sources

### Primary (HIGH confidence)

- `crates/memory-client/src/hook_mapping.rs` - HookEvent types and mapping
- `crates/memory-client/src/client.rs` - MemoryClient API
- CCH documentation (hooks.yaml format)

### Secondary (MEDIUM confidence)

- CCH source code (event format details)

## Metadata

**Confidence breakdown:**
- Event mapping: HIGH - Already built and tested in memory-client
- Binary structure: HIGH - Standard stdin/stdout pattern
- CCH format: MEDIUM - Based on documentation, verify with real CCH

**Research date:** 2026-01-30
**Valid until:** Stable (builds on existing code)
