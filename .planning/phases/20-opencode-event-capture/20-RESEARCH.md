# Phase 20: OpenCode Event Capture + Unified Queries - Research

**Researched:** 2026-02-09
**Domain:** OpenCode plugin event capture, multi-agent ingest, unified query infrastructure
**Confidence:** HIGH

## Summary

Phase 20 has two distinct work streams: (1) capturing OpenCode session events into agent-memory, and (2) enabling unified cross-agent queries with agent-aware output. The codebase is well-prepared for both thanks to Phase 18 (agent tagging infrastructure) and Phase 19 (OpenCode plugin structure).

For event capture, OpenCode uses a **TypeScript/JavaScript plugin system** rather than the CCH (code_agent_context_hooks) binary-based approach used by Claude Code. OpenCode plugins subscribe to lifecycle events (`session.created`, `session.idle`, `message.updated`, `tool.execute.after`) and can execute shell commands via Bun's `$` API. The plugin will call the existing `memory-ingest` binary (or directly call memory-daemon gRPC) to send events, with the agent field set to `"opencode"`.

For unified queries, the infrastructure is already 80% complete. Phase 18 added: `Event.agent` field, `--agent` CLI filter, `agent_filter` on all search RPCs (TeleportSearch, VectorTeleport, HybridSearch, RouteQuery), and `RetrievalResult.agent` field in proto. The remaining work is: (a) populating the `agent` field in retrieval results, (b) displaying source agent in CLI output, and (c) optional agent-affinity ranking.

**Primary recommendation:** Create an OpenCode TypeScript plugin (`.opencode/plugin/memory-capture.ts`) that hooks into session lifecycle events and calls `memory-ingest` with `agent:opencode` tagging. For unified queries, wire the existing `agent_filter` through the query pipeline and populate `RetrievalResult.agent` from stored event data. Add `--agent` display in CLI output formatting.

## Standard Stack

### Core

| Component | Format | Purpose | Why Standard |
|-----------|--------|---------|--------------|
| OpenCode Plugin | TypeScript (.ts) | Event capture via lifecycle hooks | OpenCode native plugin system; auto-discovered |
| memory-ingest binary | Rust binary (stdin JSON) | Convert events to gRPC IngestEvent | Already exists, proven for Claude Code |
| Bun shell API (`$`) | Shell execution | Call memory-ingest from plugin | Built into OpenCode plugin context |
| gRPC IngestEvent | Proto message | Event ingestion with agent field | Already supports `optional string agent = 8` |

### Supporting

| Component | Format | Purpose | When to Use |
|-----------|--------|---------|-------------|
| `@opencode-ai/plugin` | NPM package | Plugin type definitions | For TypeScript type safety |
| `client.app.log()` | OpenCode SDK | Structured logging from plugin | Debug/trace event capture |
| `.opencode/package.json` | JSON | Declare plugin dependencies | If using NPM packages |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Plugin calling memory-ingest binary | Plugin calling gRPC directly | Direct gRPC needs proto compilation in TS; binary is simpler |
| TypeScript plugin | Shell script hook | OpenCode uses plugins, not shell hooks; plugin is native |
| Bun `$` shell API | child_process.exec | `$` is built into plugin context; no import needed |
| Plugin event capture | SDK event streaming | SDK is for external monitoring; plugin is in-process, lower latency |

## Architecture Patterns

### Recommended Project Structure

```
plugins/memory-opencode-plugin/
├── .opencode/
│   ├── plugin/                       # NEW: Event capture plugin
│   │   └── memory-capture.ts         # Session lifecycle hooks
│   ├── package.json                  # NEW: Plugin dependencies (if needed)
│   ├── command/                      # Existing (Phase 19)
│   │   ├── memory-search.md
│   │   ├── memory-recent.md
│   │   └── memory-context.md
│   ├── skill/                        # Existing (Phase 19)
│   │   └── [5 skills]
│   └── agents/                       # Existing (Phase 19)
│       └── memory-navigator.md
├── README.md                         # Update with event capture docs
└── .gitignore
```

### Pattern 1: OpenCode Event Capture Plugin

**What:** TypeScript plugin that hooks session lifecycle events and forwards to memory-ingest.

**When to use:** For all OpenCode session event capture.

**Example:**

```typescript
// .opencode/plugin/memory-capture.ts
// Source: OpenCode Plugins Documentation (https://opencode.ai/docs/plugins/)

import type { Plugin } from "@opencode-ai/plugin"

export const MemoryCapturePlugin: Plugin = async ({ $, directory }) => {
  const MEMORY_INGEST = process.env.MEMORY_INGEST_PATH || "memory-ingest"

  // Helper: send event to memory-ingest via stdin
  async function captureEvent(event: {
    hook_event_name: string
    session_id: string
    message?: string
    cwd?: string
    timestamp?: string
  }) {
    try {
      const payload = JSON.stringify({
        ...event,
        agent: "opencode",
        cwd: event.cwd || directory,
        timestamp: event.timestamp || new Date().toISOString(),
      })
      await $`echo ${payload} | ${MEMORY_INGEST}`.quiet()
    } catch {
      // Fail-open: never block OpenCode
    }
  }

  return {
    // Session started
    "session.created": async (input) => {
      await captureEvent({
        hook_event_name: "SessionStart",
        session_id: input.id || "unknown",
        cwd: directory,
      })
    },

    // Session idle (agent finished responding) = session checkpoint
    "session.idle": async (input) => {
      await captureEvent({
        hook_event_name: "SessionEnd",
        session_id: (input as any).session_id || "unknown",
        cwd: directory,
      })
    },

    // Message updated (user or assistant message)
    "message.updated": async (input) => {
      const message = (input as any).properties?.message
      if (!message) return

      const eventName = message.role === "user"
        ? "UserPromptSubmit"
        : "AssistantResponse"

      await captureEvent({
        hook_event_name: eventName,
        session_id: (input as any).session_id || "unknown",
        message: typeof message.content === "string"
          ? message.content
          : JSON.stringify(message.content),
      })
    },

    // Tool execution completed
    "tool.execute.after": async (input) => {
      await captureEvent({
        hook_event_name: "PostToolUse",
        session_id: input.sessionID || "unknown",
        message: JSON.stringify({
          tool_name: input.tool,
          args: input.args,
        }),
      })
    },
  }
}
```

**Confidence:** MEDIUM - Event hook input shapes not fully typed in public docs; may need adjustment during implementation.

### Pattern 2: Agent-Aware Ingest Pipeline

**What:** The memory-ingest binary already accepts agent field; need to ensure OpenCode plugin sets it.

**When to use:** For auto-tagging events with `agent:opencode`.

**How it works:**

1. OpenCode plugin adds `"agent": "opencode"` to the JSON payload
2. memory-ingest binary currently reads from stdin as `CchEvent`
3. The `CchEvent` struct does NOT currently have an `agent` field
4. Need to add `agent: Option<String>` to `CchEvent` and pass through to `HookEvent`
5. `map_hook_event` needs to propagate agent to `Event.with_agent()`

**Current gap in memory-ingest/src/main.rs:**

```rust
// Current CchEvent struct (line 23-43) does NOT include agent field.
// Need to add:
//   #[serde(default)]
//   agent: Option<String>,
//
// And in map_cch_to_hook() (line 62-90), propagate to HookEvent.
// And in main() (line 97-139), set event.agent from cch.agent.
```

### Pattern 3: Unified Query Result Formatting

**What:** CLI output and gRPC results include source agent for multi-agent results.

**When to use:** For all query commands when results span multiple agents.

**Current state:**

- `RetrievalResult` proto already has `optional string agent = 7` (Phase 18)
- `RouteQueryResponse` already supports it
- CLI `--agent` filter already exists on teleport, retrieval, and query commands
- **Gap:** `RetrievalResult.agent` is always set to `None` in retrieval.rs (line 281)
- **Gap:** CLI output formatters don't display agent field

**Fix needed in retrieval.rs:**

```rust
// Current (line 270-282):
.map(|r| ProtoResult {
    doc_id: r.doc_id.clone(),
    // ...
    agent: None, // Phase 18: Agent populated when available
})

// Should become:
.map(|r| ProtoResult {
    doc_id: r.doc_id.clone(),
    // ...
    agent: r.metadata.get("agent").cloned(),
})
```

### Pattern 4: Agent Affinity Ranking (Optional P1)

**What:** Boost results from the "current" agent when not filtering.

**When to use:** Optional enhancement for relevance.

**Approach:**
- Add agent affinity weight to ranking (e.g., 1.1x boost for current agent)
- Detect current agent from environment or CLI context
- Apply as a post-processing step on retrieval results

### Anti-Patterns to Avoid

- **Don't create a separate ingest binary for OpenCode:** Reuse `memory-ingest` by adding agent field support
- **Don't use blocking event handlers:** OpenCode plugin hooks must be non-blocking; use fail-open pattern
- **Don't capture every message.updated event:** Filter by role to avoid duplicate captures
- **Don't assume event input shapes:** OpenCode event payloads are loosely typed; use defensive access
- **Don't modify existing query semantics:** Default queries MUST still return all agents (R4.2.1)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Event capture mechanism | Custom event polling | OpenCode plugin system | Native, auto-loaded, async |
| Shell execution from plugin | child_process | Bun `$` API | Built into plugin context |
| Agent detection | Environment sniffing | Hardcode "opencode" in plugin | Plugin IS OpenCode; detection is trivial |
| gRPC client in TypeScript | Proto compilation pipeline | memory-ingest binary via stdin | Binary already exists, proven, simple |
| Session ID tracking | Custom state management | Use OpenCode's session.id from events | Built into event payloads |

**Key insight:** The memory-ingest binary already handles the hard part (gRPC client, event mapping, fail-open behavior). The OpenCode plugin just needs to format JSON and pipe it to the binary.

## Common Pitfalls

### Pitfall 1: Plugin Event Input Shapes Are Loosely Typed

**What goes wrong:** Accessing properties that don't exist on the event input object, causing silent failures.

**Why it happens:** OpenCode event payloads are not fully typed in the plugin API; different events have different shapes.

**How to avoid:**
- Use defensive property access: `(input as any).properties?.message?.role`
- Add null checks before using any event property
- Log unknown event shapes during development for discovery
- Test with actual OpenCode sessions, not just unit tests

**Warning signs:** Plugin runs but captures no events; events have empty content.

### Pitfall 2: Fail-Open Must Be Absolute

**What goes wrong:** memory-ingest timeout blocks OpenCode, making the AI agent hang.

**Why it happens:** The `$` shell call has no timeout by default; network issues cause gRPC to hang.

**How to avoid:**
- Wrap every `$` call in try/catch with empty catch block
- Set `MEMORY_ENDPOINT` timeout in memory-ingest (already has fail-open)
- Use `.quiet()` on all shell calls to suppress output
- Consider `timeout` command wrapper: `timeout 3 memory-ingest`

**Warning signs:** OpenCode becomes sluggish after plugin install.

### Pitfall 3: Session ID Inconsistency Between Events

**What goes wrong:** Different event hooks provide session ID in different fields, causing events to scatter across sessions.

**Why it happens:** OpenCode's event system uses `input.id` for some events, `input.sessionID` for others, `event.session_id` for yet others.

**How to avoid:**
- Create a helper function that extracts session ID from multiple possible locations
- Fall back to a plugin-level session tracker if needed
- Log session IDs during development to verify consistency

**Warning signs:** TOC shows many 1-event sessions instead of continuous conversations.

### Pitfall 4: Duplicate Event Capture

**What goes wrong:** The same message gets captured twice (e.g., both `message.updated` and `session.updated` fire for the same content).

**Why it happens:** OpenCode fires multiple events for related state changes.

**How to avoid:**
- Only capture specific event types for specific content types
- Use `message.updated` for user/assistant messages (not `session.updated`)
- Use `tool.execute.after` for tool results (not `message.part.updated`)
- Consider deduplication via event_id (ULID uniqueness)

**Warning signs:** TOC segments have duplicate bullet points.

### Pitfall 5: Retrieval Results Missing Agent Field

**What goes wrong:** Unified query returns results but agent field is always `None`.

**Why it happens:** The agent field is stored on Events but never propagated to search index documents or retrieval results.

**How to avoid:**
- When building TocNode, set `contributing_agents` from child events' agent fields
- When returning RetrievalResult, look up agent from the source document metadata
- Consider adding agent to BM25/vector index documents during indexing

**Warning signs:** `--agent` filter works but displayed results don't show source agent.

## Code Examples

### OpenCode Plugin: Minimal Event Capture

```typescript
// .opencode/plugin/memory-capture.ts
// Minimal version focused on session lifecycle capture

export const MemoryCapturePlugin = async ({ $, directory }) => {
  const ingest = (payload: Record<string, unknown>) => {
    const json = JSON.stringify({
      ...payload,
      cwd: directory,
      timestamp: new Date().toISOString(),
    })
    return $`echo ${json} | memory-ingest`.quiet().catch(() => {})
  }

  return {
    "session.created": async (input) => {
      await ingest({
        hook_event_name: "SessionStart",
        session_id: input.id,
      })
    },
    "session.idle": async (input) => {
      await ingest({
        hook_event_name: "Stop",
        session_id: (input as any).session_id || (input as any).id,
      })
    },
    "tool.execute.after": async (input) => {
      await ingest({
        hook_event_name: "PostToolUse",
        session_id: input.sessionID,
        tool_name: input.tool,
        tool_input: input.args,
      })
    },
  }
}
```

### memory-ingest: Add Agent Field Support

```rust
// Addition to CchEvent struct in crates/memory-ingest/src/main.rs:

/// Agent identifier (e.g., "opencode", "claude")
/// Auto-detected from source if not provided.
#[serde(default)]
agent: Option<String>,
```

```rust
// In map_cch_to_hook(), after building the hook event:

// Set agent on the resulting event
let mut event = map_hook_event(hook);
if let Some(agent) = &cch.agent {
    event = event.with_agent(agent.to_lowercase());
}
```

### Unified Query: Populate Agent in Results

```rust
// In crates/memory-service/src/retrieval.rs, RouteQuery handler:
// Replace line 281: agent: None,
// With:
agent: r.metadata.get("agent").cloned(),
```

### CLI Output: Show Agent Source

```rust
// In CLI output formatting for query results:
if let Some(agent) = &result.agent {
    println!("  [agent: {}]", agent);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CCH binary hooks only | Plugin-based event capture | Phase 20 | Works with agents that use plugins, not just CCH |
| Single-agent queries | Unified multi-agent results | Phase 18 proto, Phase 20 implementation | Users see all conversations regardless of source agent |
| Agent field unused | Agent field populated and displayed | Phase 20 | Results show provenance |

**Deprecated/outdated:**
- OpenCode `.opencode/hooks.yaml` mentioned in examples/hooks.yaml is NOT a real OpenCode feature. OpenCode uses the plugin system, not YAML hooks files.

## Codebase Inventory: What Exists vs What's Needed

### Already Complete (Phase 18)

| Component | File | Status |
|-----------|------|--------|
| Event.agent field | `crates/memory-types/src/event.rs:95` | Done |
| Event.with_agent() builder | `crates/memory-types/src/event.rs:127` | Done |
| Proto Event.agent | `proto/memory.proto:177` | Done |
| TocNode.contributing_agents | `crates/memory-types/src/toc.rs:168` | Done |
| AgentAdapter trait | `crates/memory-adapters/src/adapter.rs` | Done |
| RawEvent type | `crates/memory-adapters/src/adapter.rs:19` | Done |
| AdapterConfig | `crates/memory-adapters/src/config.rs` | Done |
| agent_filter on TeleportSearch | `proto/memory.proto:517` | Done |
| agent_filter on VectorTeleport | `proto/memory.proto:573` | Done |
| agent_filter on HybridSearch | `proto/memory.proto:624` | Done |
| agent_filter on RouteQuery | `proto/memory.proto:929` | Done |
| --agent CLI flag | `crates/memory-daemon/src/cli.rs:288,315,350,506` | Done |
| StopConditions.agent_filter | `crates/memory-retrieval/src/types.rs:232` | Done |
| Ingest agent extraction | `crates/memory-service/src/ingest.rs:280-283` | Done |
| RetrievalResult.agent proto field | `proto/memory.proto:942` | Done |

### Needs Implementation (Phase 20)

| Component | File | What's Needed |
|-----------|------|---------------|
| OpenCode capture plugin | `plugins/.../plugin/memory-capture.ts` | NEW: TypeScript plugin |
| memory-ingest agent support | `crates/memory-ingest/src/main.rs` | ADD: agent field to CchEvent struct |
| Agent in map_hook_event | `crates/memory-client/src/hook_mapping.rs` | ADD: agent propagation to Event |
| RetrievalResult.agent population | `crates/memory-service/src/retrieval.rs:281` | FIX: populate from metadata |
| CLI output agent display | `crates/memory-daemon/src/` (output formatting) | ADD: show agent in results |
| Plugin README update | `plugins/.../README.md` | UPDATE: add event capture docs |
| Plugin .gitignore update | `plugins/.../.gitignore` | UPDATE: add node_modules |

### Possibly Needed (Optional/P1)

| Component | File | What's Needed |
|-----------|------|---------------|
| Agent affinity ranking | `crates/memory-retrieval/` | OPTIONAL: boost current agent results |
| Checkpoint capture | Plugin | OPTIONAL: mid-session ingest on `session.idle` |
| Project context in events | Plugin | Auto-add project directory to metadata |

## Open Questions

### 1. Event Input Shape Discovery

**What we know:** OpenCode plugin events have different shapes per event type. The `session.created` event has `.id`, while `tool.execute.after` has `.sessionID`.

**What's unclear:** The exact TypeScript types for all event payloads. The `@opencode-ai/plugin` package may or may not export these.

**Recommendation:** Use `(input as any)` with defensive access during initial implementation. Add runtime logging to discover shapes. Refine types after first working version.

**Confidence:** MEDIUM

### 2. memory-ingest vs Direct gRPC from Plugin

**What we know:** The memory-ingest binary is simple, proven, and handles fail-open. An alternative is to compile proto for TypeScript and call gRPC directly from the plugin.

**What's unclear:** Whether Bun supports gRPC natively, and the complexity of adding proto compilation to the plugin build.

**Recommendation:** Use memory-ingest binary via `$` shell. It is already proven, handles fail-open, and avoids adding a TS build pipeline. Evaluate direct gRPC only if shell overhead becomes a performance issue.

**Confidence:** HIGH

### 3. Message Content Access

**What we know:** `message.updated` events contain message content. The exact payload structure for accessing message text varies.

**What's unclear:** Whether `input.properties.message.content` is always a string, or sometimes an array of parts (like Claude's content blocks).

**Recommendation:** Handle both string and array content. Use `JSON.stringify()` as fallback for non-string content.

**Confidence:** MEDIUM

### 4. Agent Affinity Ranking Complexity

**What we know:** The requirement (R4.2.3) is P1 (optional). Agent affinity means boosting results from the "current" agent.

**What's unclear:** How to detect the "current" agent from the daemon side (the daemon doesn't know which agent is querying).

**Recommendation:** Defer agent affinity to a later PR or make it opt-in via a `--prefer-agent opencode` flag. The `--agent` filter already handles the hard case (single-agent view). Cross-agent results without affinity already work via default behavior.

**Confidence:** HIGH

### 5. Bun Dependency

**What we know:** OpenCode uses Bun as its plugin runtime. Plugins are TypeScript files that Bun executes.

**What's unclear:** Whether all OpenCode users have Bun installed (OpenCode likely bundles it).

**Recommendation:** Assume Bun is available since OpenCode requires it for its plugin system. Document this in README prerequisites.

**Confidence:** HIGH

## Sources

### Primary (HIGH confidence)

- [OpenCode Plugins Documentation](https://opencode.ai/docs/plugins/) - Plugin format, event hooks, context API
- [OpenCode Plugins Guide (GitHub Gist)](https://gist.github.com/johnlindquist/0adf1032b4e84942f3e1050aba3c5e4a) - Complete reference with code examples
- Codebase: `crates/memory-ingest/src/main.rs` - Existing CCH handler pattern
- Codebase: `crates/memory-types/src/event.rs` - Event.agent field (Phase 18)
- Codebase: `crates/memory-adapters/src/adapter.rs` - AgentAdapter trait (Phase 18)
- Codebase: `crates/memory-service/src/ingest.rs` - Ingest with agent extraction (Phase 18)
- Codebase: `crates/memory-service/src/retrieval.rs` - Retrieval handler with agent filter (Phase 17/18)
- Codebase: `proto/memory.proto` - Proto definitions with agent fields (Phase 18)
- Codebase: `plugins/memory-opencode-plugin/` - Existing OpenCode plugin structure (Phase 19)

### Secondary (MEDIUM confidence)

- [OpenCode Hooks Support (DEV Community)](https://dev.to/einarcesar/does-opencode-support-hooks-a-complete-guide-to-extensibility-k3p) - Extensibility comparison
- [OpenCode Hooks Issue #1473](https://github.com/sst/opencode/issues/1473) - Plugin system as hook replacement
- [oh-my-opencode DeepWiki](https://deepwiki.com/code-yeongyu/oh-my-opencode/7.1-context-management-hooks) - Advanced hook patterns

### Tertiary (LOW confidence)

- OpenCode event payload shapes - Based on community examples, not official type definitions
- Agent affinity ranking patterns - No established standard; custom design needed

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - OpenCode plugin system well-documented, memory-ingest proven
- Architecture: HIGH - Phase 18/19 infrastructure already handles most of the work
- Event capture: MEDIUM - Plugin event shapes not fully typed; needs runtime discovery
- Unified queries: HIGH - Proto and CLI infrastructure complete; just need wiring
- Agent affinity: LOW - Optional feature, no standard pattern, deferred recommended

**Research date:** 2026-02-09
**Valid until:** 2026-03-09 (30 days - stable architecture)
