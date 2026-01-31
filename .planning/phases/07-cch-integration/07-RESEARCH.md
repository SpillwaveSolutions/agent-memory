# Phase 7: CCH Integration - Research

**Researched:** 2026-01-30
**Domain:** CCH hook integration, gRPC client binaries, Claude Code skills
**Confidence:** HIGH

## Summary

This phase integrates agent-memory with Code-Agent Context Hooks (CCH) for automatic event capture, plus provides an agentic skill for querying past conversations. The research covers two distinct components:

1. **memory-ingest binary** - A CLI tool that CCH can invoke via `run:` action to capture events and send them to memory-daemon via gRPC
2. **Agentic skill** - A Claude Code skill that provides natural language commands for querying memory

The existing codebase already has `memory-client` crate with `HookEvent` mapping and `MemoryClient` for gRPC communication. CCH uses a `hooks.yaml` configuration with `run:` actions that invoke external scripts/binaries, passing JSON events on stdin and reading JSON responses on stdout.

**Primary recommendation:** Create a `memory-ingest` binary that reads CCH events from stdin, maps them to memory events using the existing `hook_mapping` module, and ingests via `MemoryClient`. For the skill, follow the PDA (Progressive Disclosure Architecture) pattern with commands like `/memory-search`, `/memory-recent`, and `/memory-context`.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| memory-client | local | gRPC client for memory-daemon | Already exists in workspace |
| clap | 4.5+ | CLI argument parsing | Used throughout project |
| serde_json | 1.0 | JSON stdin/stdout processing | CCH event format is JSON |
| tokio | 1.43+ | Async runtime for gRPC | Already in workspace |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1 | Logging (optional) | Debug mode |
| directories | 6.0 | Find socket paths | Already in workspace |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Dedicated binary | Shell script wrapper | Binary is faster, can embed mapping logic |
| gRPC | Direct RocksDB access | gRPC maintains clean separation, daemon handles locking |

**Installation:**
```bash
# No new dependencies needed - uses workspace crates
cargo build --release -p memory-ingest
```

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── memory-ingest/       # CCH integration binary (NEW)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs      # Reads stdin, maps events, calls gRPC
└── memory-client/       # Existing - provides client API
    └── src/
        ├── client.rs
        └── hook_mapping.rs
```

### Pattern 1: CCH Run Action Binary

**What:** CCH's `run:` action invokes a binary that receives JSON on stdin and returns JSON on stdout

**When to use:** Integrating external processing with CCH hooks

**Example:**
```yaml
# .claude/hooks.yaml
rules:
  - name: capture-to-memory
    description: Send events to agent-memory daemon
    matchers:
      operations:
        - SessionStart
        - UserPromptSubmit
        - PostToolUse
        - SessionEnd
    actions:
      run: "/path/to/memory-ingest"
```

The binary:
```rust
// Source: CCH hooks.rs pattern
use std::io::{self, BufRead, Write};

fn main() {
    // Read JSON from stdin
    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input).unwrap();

    // Parse CCH event
    let cch_event: CchEvent = serde_json::from_str(&input).unwrap();

    // Map and ingest (async runtime needed for gRPC)
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut client = MemoryClient::connect_default().await?;
        let event = map_cch_to_memory(cch_event);
        client.ingest(event).await?;
        Ok::<_, anyhow::Error>(())
    })?;

    // Return success JSON to CCH
    let response = json!({ "continue": true });
    println!("{}", response);
}
```

### Pattern 2: Claude Code Skill with PDA

**What:** Skill YAML frontmatter + markdown body with progressive disclosure

**When to use:** Creating agentic skills for Claude Code

**Example:**
```markdown
---
name: memory-query
description: Query past conversations from agent-memory. Use when asked to "search memory", "what did we discuss", "show recent sessions", or "find context about".
allowed-tools:
  - Bash
  - Read
metadata:
  version: 1.0.0
---

# Agent Memory Query Skill

Query your conversation history with natural language.

## Commands

| Command | Description |
|---------|-------------|
| `/memory-search <query>` | Search for topics across all conversations |
| `/memory-recent [days]` | Show recent conversation summaries |
| `/memory-context <topic>` | Get full context around a specific topic |

## How It Works

[Progressive disclosure - show summary first, drill down on request]
```

### Anti-Patterns to Avoid

- **Blocking the CCH pipeline:** Keep processing fast (<100ms), defer heavy work
- **Ignoring daemon connection failures:** Use graceful fallback, log errors
- **Hardcoding paths:** Use XDG/platform-aware path resolution
- **Returning non-JSON from run action:** CCH expects valid JSON response

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CCH event parsing | Custom parser | serde_json with CCH model types | Well-tested, matches CCH exactly |
| gRPC communication | Raw HTTP/socket | memory-client crate | Already handles proto conversion |
| Hook event mapping | New mapping logic | hook_mapping.rs HookEvent types | Already implements 1:1 mapping |
| Path resolution | Manual path joining | directories crate | Cross-platform, handles XDG |

**Key insight:** The memory-client crate already has all the mapping and client logic needed. The new binary is just a thin stdin/stdout wrapper around existing functionality.

## Common Pitfalls

### Pitfall 1: CCH Event Field Names

**What goes wrong:** CCH sends `hook_event_name` not `event_type` - field name mismatch causes parsing failure

**Why it happens:** CCH models.rs uses `#[serde(alias = "event_type")]` for backward compatibility

**How to avoid:** Use exact CCH event model from CCH source or define with proper serde attributes

**Warning signs:** JSON parse errors, missing event_type field

### Pitfall 2: Blocking gRPC Calls

**What goes wrong:** Synchronous gRPC call blocks the tokio runtime

**Why it happens:** Using `block_on` incorrectly or not using async properly

**How to avoid:** Use `tokio::runtime::Runtime::new().unwrap().block_on()` pattern for main binary

**Warning signs:** Hangs, timeouts, deadlocks

### Pitfall 3: Daemon Not Running

**What goes wrong:** memory-ingest fails silently when daemon is not running

**Why it happens:** Connection refused, no error handling

**How to avoid:** Check daemon status first, return JSON error to CCH with `continue: true` (fail open)

**Warning signs:** Events not appearing in memory, silent failures

### Pitfall 4: Skill Not Loading Reference Docs

**What goes wrong:** Skill tries to call memory-daemon directly without CLI wrapper

**Why it happens:** Claude Code skills can't spawn async gRPC clients

**How to avoid:** Use `memory-daemon query` CLI subcommands from Bash tool

**Warning signs:** "Cannot connect" errors, undefined behavior

## Code Examples

### CCH Event Struct (from CCH source)

```rust
// Source: /tmp/cch/cch_cli/src/models.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(alias = "event_type")]
    pub hook_event_name: EventType,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub session_id: String,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub cwd: Option<String>,
    pub transcript_path: Option<String>,
    // ... other fields
}

pub enum EventType {
    PreToolUse,
    PostToolUse,
    UserPromptSubmit,
    SessionStart,
    SessionEnd,
    Stop,
    SubagentStart,
    SubagentStop,
    // ...
}
```

### Memory Event Mapping (existing)

```rust
// Source: crates/memory-client/src/hook_mapping.rs
pub fn map_hook_event(hook: HookEvent) -> Event {
    let event_type = match hook.event_type {
        HookEventType::SessionStart => EventType::SessionStart,
        HookEventType::UserPromptSubmit => EventType::UserMessage,
        HookEventType::ToolResult => EventType::ToolResult,
        HookEventType::Stop => EventType::SessionEnd,
        // ...
    };
    // Creates Event with ULID, timestamp, role mapping
}
```

### hooks.yaml Template for Agent Memory

```yaml
# Source: Pattern from /tmp/cch/.claude/hooks.yaml
version: "1.0"

settings:
  fail_open: true    # If memory-ingest fails, don't block Claude
  script_timeout: 5  # 5 second timeout for memory-ingest

rules:
  - name: capture-session-start
    description: Capture session start to agent-memory
    matchers:
      operations:
        - SessionStart
    actions:
      run: "~/.local/bin/memory-ingest"

  - name: capture-user-prompts
    description: Capture user messages to agent-memory
    matchers:
      operations:
        - UserPromptSubmit
    actions:
      run: "~/.local/bin/memory-ingest"

  - name: capture-tool-results
    description: Capture tool results to agent-memory
    matchers:
      operations:
        - PostToolUse
    actions:
      run: "~/.local/bin/memory-ingest"

  - name: capture-session-end
    description: Capture session end to agent-memory
    matchers:
      operations:
        - SessionEnd
    actions:
      run: "~/.local/bin/memory-ingest"
```

### Skill Command Implementation Pattern

```bash
# memory-search implementation (called via Bash tool)
# Query uses existing CLI: memory-daemon query ...
memory-daemon query root  # Get year nodes
memory-daemon query browse "toc:year:2026" --limit 10  # Browse children
memory-daemon query node "toc:day:2026-01-30"  # Get day summary
memory-daemon query expand "grip:1706620800000:01ARZ3NDEKTSV4RRFFQ69G5FAV"  # Expand grip
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Custom JSON-RPC | CCH run action pattern | CCH 1.0 (2025) | Standard hook integration |
| HTTP webhook | gRPC ingest | Project decision | Lower latency, type safety |
| Skill spawns daemon | Skill uses existing CLI | Phase 7 | Simplifies skill implementation |

**Deprecated/outdated:**
- HTTP server in memory-daemon: Never implemented, gRPC only per project decision

## Open Questions

1. **Binary installation path**
   - What we know: Binary needs to be in PATH or absolute path in hooks.yaml
   - What's unclear: Should it be `~/.local/bin/memory-ingest` or packaged with CCH?
   - Recommendation: Use `~/.local/bin/` with installation script, document in skill

2. **Multi-project isolation**
   - What we know: Per-project stores is the design (from STATE.md)
   - What's unclear: How does memory-ingest know which project store to use?
   - Recommendation: Use `cwd` field from CCH event to determine project, fall back to config

3. **Skill reference documentation**
   - What we know: Skills load guides/references on demand (PDA pattern)
   - What's unclear: Should skill include inline examples or load from filesystem?
   - Recommendation: Inline examples for commands, load references for troubleshooting

## Sources

### Primary (HIGH confidence)

- `/tmp/cch/cch_cli/src/models.rs` - CCH Event model definition
- `/tmp/cch/cch_cli/src/hooks.rs` - CCH run action execution pattern
- `/tmp/cch/.claude/hooks.yaml` - Working hooks.yaml example
- `crates/memory-client/src/hook_mapping.rs` - Existing HookEvent mapping
- `crates/memory-client/src/client.rs` - MemoryClient API
- `/tmp/cch/.claude/skills/architect-agent/SKILL.md` - Skill structure example

### Secondary (MEDIUM confidence)

- `/tmp/cch/docs/README.md` - CCH documentation
- `/tmp/cch/docs/USER_GUIDE_CLI.md` - CCH CLI usage guide

### Tertiary (LOW confidence)

- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Uses existing workspace crates, CCH patterns verified from source
- Architecture: HIGH - Binary pattern verified from CCH hooks.rs, skill pattern from architect-agent skill
- Pitfalls: MEDIUM - Based on code analysis and gRPC experience, some edge cases speculative

**Research date:** 2026-01-30
**Valid until:** 60 days (CCH and memory-client are stable)
