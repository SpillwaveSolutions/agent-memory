# Phase 21: Gemini CLI Adapter - Research

**Researched:** 2026-02-09
**Domain:** Gemini CLI hook system, TOML commands, agent skills, event capture adapter
**Confidence:** HIGH

## Summary

Gemini CLI provides a comprehensive hook system with 11 lifecycle events configured via `settings.json`. Unlike Claude Code's CCH binary-pipe approach or OpenCode's TypeScript plugin system, Gemini CLI hooks are configured declaratively in JSON and executed as shell commands that receive JSON via stdin and return JSON via stdout. This maps almost perfectly to the existing `memory-ingest` binary's interface -- both use stdin JSON pipes with fail-open behavior.

The event mapping from Gemini to Agent Memory is strong. Gemini provides `SessionStart`, `SessionEnd`, `BeforeAgent` (user prompt submitted), `AfterAgent` (agent response complete), `BeforeTool`, and `AfterTool` events, all of which include `session_id`, `cwd`, `hook_event_name`, and `timestamp` in their base input schema. The `memory-ingest` binary already accepts these exact fields. The key gap is that Gemini hooks run **synchronously** by default (blocking the agent loop), so the hook scripts must be fast or use background execution to avoid latency impact.

For commands and skills, Gemini CLI uses TOML files for custom slash commands (stored in `.gemini/commands/`) and the standard `SKILL.md` format for skills (stored in `.gemini/skills/`). Gemini explicitly supports Claude Code skill format compatibility, meaning the existing SKILL.md files from the query plugin can be reused directly. TOML commands have a simple `prompt` + optional `description` format with `{{args}}` placeholder support. The navigator agent concept maps to a skill with embedded agent logic, since Gemini does not have a separate agent definition file format like Claude Code or OpenCode.

**Primary recommendation:** Create a Gemini adapter that (1) configures hook entries in `.gemini/settings.json` pointing to a thin shell wrapper that calls `memory-ingest` with `agent:gemini` tagging, (2) provides TOML command wrappers in `.gemini/commands/` that reference shared skill instructions, (3) includes an install skill that auto-generates the settings.json hook configuration.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Hook format & event mapping
- Gemini CLI has a hook system in its latest release -- target the actual hook API, no wrapper scripts
- Research Gemini's hook format from scratch (user has no prior knowledge of specifics)
- Map Gemini hook events to existing Agent Memory event types (session_start, user_message, tool_result, assistant_stop, etc.) as closely as possible
- If 1:1 mapping isn't possible for some events, create Gemini-specific event types as fallback
- Hooks should call `memory-ingest` binary directly (same binary as Claude hooks); if Gemini's hook format makes that impractical, fall back to a Gemini-specific ingest binary

#### Command & skill porting
- Create TOML command wrappers for query commands (memory-search, memory-recent, memory-context)
- TOML commands reference the same SKILL.md files -- skills are the shared format across agents
- No separate navigator agent definition -- embed navigator logic inside the skill, tell Gemini to invoke in parallel
- Skill file sharing strategy: Claude's discretion (separate copies vs shared references based on practical constraints)

#### Installation & setup UX
- Hook handler calls the compiled `memory-ingest` Rust binary directly -- no TypeScript/Bun runtime dependency
- Provide both: an `agent-memory-gemini-install-skill` for automated setup AND manual documentation
- Install skill auto-detects Gemini CLI presence and warns if not found
- Setup writes Gemini hook config files automatically

#### Adapter boundary & parity
- Target maximum Claude parity -- event capture + query commands + navigator equivalent + install skill
- Fail-open philosophy: hooks silently fail if memory daemon is unreachable (same as Claude/OpenCode)
- For missing hook events: Claude's discretion per event -- document trivial gaps, work around important ones
- Automated E2E testing with real Gemini CLI sessions (not just unit tests + manual)

### Claude's Discretion
- Plugin directory structure (separate `plugins/memory-gemini-adapter/` vs shared structure)
- Whether to use separate skill copies or shared references
- Specific workaround strategies for missing Gemini hook events
- TOML command structure details (based on Gemini's actual format)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

## Standard Stack

### Core

| Component | Format | Purpose | Why Standard |
|-----------|--------|---------|--------------|
| Gemini CLI hooks | `settings.json` (JSON) | Event capture via lifecycle hooks | Gemini native hook system; declarative configuration |
| memory-ingest binary | Rust binary (stdin JSON) | Convert hook events to gRPC IngestEvent | Already exists, proven for Claude Code + OpenCode |
| Shell wrapper script | Bash script | Transform Gemini hook JSON to memory-ingest format | Bridges Gemini's synchronous hooks to fail-open ingest |
| TOML command files | `.toml` in `.gemini/commands/` | Slash commands for memory queries | Gemini CLI native command format |
| SKILL.md files | Markdown in `.gemini/skills/` | Agent skills for query/retrieval logic | Claude Code compatible, Gemini natively supports |

### Supporting

| Component | Format | Purpose | When to Use |
|-----------|--------|---------|-------------|
| `settings.json` | JSON config | Hook definitions + skill enablement | During installation and configuration |
| `.gemini/hooks/` | Directory of scripts | Hook handler scripts | Event capture implementation |
| `GEMINI_SESSION_ID` env var | Environment variable | Session identification in hooks | Passed by Gemini CLI to all hooks |
| `GEMINI_PROJECT_DIR` env var | Environment variable | Project directory context | Passed by Gemini CLI to all hooks |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shell wrapper calling memory-ingest | Direct memory-ingest in command field | Shell wrapper needed because Gemini provides richer JSON than memory-ingest expects; wrapper extracts/transforms fields |
| Separate TOML commands per query | Single combined command | Separate commands match Claude Code plugin UX parity |
| Separate skill copies in .gemini/skills/ | Symlinks to shared skills | Copies are self-contained and portable; symlinks break if paths change |

## Architecture Patterns

### Recommended Project Structure

**Recommendation (Claude's Discretion):** Use a separate `plugins/memory-gemini-adapter/` directory, matching the existing `plugins/memory-opencode-plugin/` pattern. This keeps each agent's adapter self-contained.

```
plugins/memory-gemini-adapter/
├── .gemini/
│   ├── settings.json              # Hook configuration (installed by skill)
│   ├── hooks/
│   │   └── memory-capture.sh      # Hook handler script
│   ├── commands/
│   │   ├── memory-search.toml     # Search command
│   │   ├── memory-recent.toml     # Recent conversations command
│   │   └── memory-context.toml    # Context expansion command
│   └── skills/
│       ├── memory-query/          # Core query skill (shared SKILL.md content)
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── retrieval-policy/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── bm25-search/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── vector-search/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       ├── topic-graph/
│       │   ├── SKILL.md
│       │   └── references/
│       │       └── command-reference.md
│       └── memory-gemini-install/  # Install skill
│           └── SKILL.md
├── README.md
└── .gitignore
```

**Skill sharing recommendation (Claude's Discretion):** Use **separate copies** of SKILL.md files in `.gemini/skills/`. Rationale:
1. Gemini CLI skills use identical SKILL.md format to Claude Code (confirmed by official docs and community)
2. Separate copies make the adapter fully self-contained and portable
3. Skills content is identical -- just copy from Claude Code plugin
4. Symlinks would break portability and complicate installation
5. Any skill updates can be synchronized via a simple copy script

### Pattern 1: Gemini Hook Event Capture

**What:** Shell script hook handler that receives Gemini lifecycle events via stdin JSON and forwards them to `memory-ingest` with `agent:gemini` tagging.

**When to use:** For all Gemini session event capture.

**How it works:**

Gemini CLI sends JSON via stdin to the hook command. The hook script:
1. Reads the JSON from stdin
2. Extracts relevant fields (`session_id`, `hook_event_name`, `timestamp`, `cwd`)
3. Adds event-specific content (prompt text from `BeforeAgent`, tool name from `AfterTool`, etc.)
4. Adds `"agent": "gemini"` to the payload
5. Pipes the transformed JSON to `memory-ingest`
6. Outputs `{}` to stdout (success, no modifications)
7. Exits with code 0

**Key design choice:** The hook script must handle the synchronous execution model. Since Gemini waits for hooks to complete, the script should:
- Use fast paths (jq for JSON transform, direct pipe to memory-ingest)
- Background the memory-ingest call if latency is a concern
- Always exit quickly with `{}` output

### Pattern 2: TOML Command Wrappers

**What:** TOML files that define slash commands referencing skill instructions.

**When to use:** For `/memory-search`, `/memory-recent`, `/memory-context` commands.

**Key insight:** Gemini TOML commands contain a `prompt` field that IS the instruction to the model. Unlike Claude Code commands (which are markdown with YAML frontmatter), Gemini commands are purely TOML with the prompt as the main content. The skill is activated by describing the workflow in the prompt text.

### Pattern 3: Navigator as Skill (Not Agent)

**What:** Embed the navigator agent logic directly in a skill's SKILL.md rather than as a separate agent definition.

**When to use:** Because Gemini CLI does not have a separate agent definition format like Claude Code's `agents/memory-navigator.md`. The navigator capability is delivered as a skill that Gemini activates when the user asks memory-related questions.

**How it works:** The `memory-query` skill's SKILL.md already contains the full navigator logic (tier detection, intent classification, fallback chains, explainability). When Gemini activates this skill, it gets the same comprehensive retrieval instructions that the Claude Code navigator agent uses.

### Anti-Patterns to Avoid

- **Blocking hooks with slow operations:** Gemini hooks run synchronously. Never do network I/O that could take >1s in the main hook thread. Background `memory-ingest` if needed.
- **Printing to stdout from hook scripts:** Any non-JSON output to stdout breaks Gemini CLI parsing. All logging MUST go to stderr.
- **Using TypeScript/Node.js for hook handlers:** The user explicitly requires no TypeScript/Bun dependency. Use shell scripts only.
- **Creating a separate navigator agent file:** Gemini has no agent definition format. Embed navigator logic in the skill.

## Event Mapping: Gemini to Agent Memory

### Complete Event Mapping

| Gemini Event | Agent Memory Event | memory-ingest hook_event_name | Mapping Quality | Notes |
|-------------|-------------------|------------------------------|-----------------|-------|
| `SessionStart` | `SessionStart` | `"SessionStart"` | EXACT | `source` field indicates startup/resume/clear |
| `SessionEnd` | `SessionEnd` / `Stop` | `"Stop"` | EXACT | `reason` field indicates exit/clear/logout |
| `BeforeAgent` | `UserPromptSubmit` | `"UserPromptSubmit"` | GOOD | `prompt` field contains user text |
| `AfterAgent` | `AssistantResponse` | `"AssistantResponse"` | GOOD | `prompt_response` field contains assistant text |
| `BeforeTool` | `PreToolUse` | `"PreToolUse"` | EXACT | `tool_name` and `tool_input` available |
| `AfterTool` | `PostToolUse` / `ToolResult` | `"PostToolUse"` | EXACT | `tool_name`, `tool_input`, `tool_response` available |
| `BeforeModel` | (no mapping) | N/A | SKIP | Internal LLM request; not a conversation event |
| `AfterModel` | (no mapping) | N/A | SKIP | Internal LLM response chunks; too granular |
| `BeforeToolSelection` | (no mapping) | N/A | SKIP | Tool planning; not a conversation event |
| `PreCompress` | (no mapping) | N/A | SKIP | Context management; not a conversation event |
| `Notification` | (no mapping) | N/A | SKIP | System alerts; not conversation content |

### Gemini Base Input Schema (All Events)

Every hook receives these fields via stdin:

```json
{
  "session_id": "string",
  "transcript_path": "string",
  "cwd": "string",
  "hook_event_name": "string",
  "timestamp": "string (ISO 8601)"
}
```

### Event-Specific Input Fields

**SessionStart** additional: `{ "source": "startup" | "resume" | "clear" }`
**SessionEnd** additional: `{ "reason": "exit" | "clear" | "logout" | "prompt_input_exit" | "other" }`
**BeforeAgent** additional: `{ "prompt": "string" }`
**AfterAgent** additional: `{ "prompt": "string", "prompt_response": "string", "stop_hook_active": boolean }`
**BeforeTool** additional: `{ "tool_name": "string", "tool_input": object, "mcp_context": object }`
**AfterTool** additional: `{ "tool_name": "string", "tool_input": object, "tool_response": { "llmContent", "returnDisplay", "error" }, "mcp_context": object }`

### Parity Assessment

| Claude Code Event | Gemini Equivalent | Parity |
|------------------|-------------------|--------|
| SessionStart | SessionStart | FULL |
| UserPromptSubmit | BeforeAgent (prompt field) | FULL |
| AssistantResponse | AfterAgent (prompt_response field) | FULL |
| PreToolUse | BeforeTool | FULL |
| PostToolUse | AfterTool | FULL |
| Stop / SessionEnd | SessionEnd | FULL |
| SubagentStart | (none) | GAP - Gemini has no subagent lifecycle events |
| SubagentStop | (none) | GAP - Gemini has no subagent lifecycle events |

**Gap analysis (Claude's Discretion):** SubagentStart/SubagentStop have no Gemini equivalent. This is a trivial gap -- subagent events are low-priority metadata, not core conversation capture. Document the gap but no workaround needed.

## Environment Variables Available in Hooks

| Variable | Content | Use in Hook |
|----------|---------|-------------|
| `GEMINI_SESSION_ID` | Unique session identifier | Map to `session_id` field |
| `GEMINI_PROJECT_DIR` | Absolute path to project root | Map to `cwd` metadata |
| `GEMINI_CWD` | Current working directory | Alternative cwd source |
| `CLAUDE_PROJECT_DIR` | Compatibility alias | Fallback for shared tooling |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing in shell | Custom awk/sed parsing | `jq` utility | JSON from Gemini is complex; jq handles edge cases |
| Event type mapping | Custom type converter | Existing `memory-ingest` mapping | memory-ingest already maps hook_event_name to EventType |
| Fail-open hook behavior | Custom error handling | Shell `|| true` + `{}` output | Gemini treats exit code 0 + `{}` as "proceed normally" |
| Skill file format | Custom Gemini skill format | Standard SKILL.md (Claude compatible) | Gemini explicitly supports Claude Code skill format |
| Session ID extraction | Custom extraction logic | `GEMINI_SESSION_ID` env var + stdin `session_id` | Both available; env var is simpler, stdin JSON is authoritative |

**Key insight:** The `memory-ingest` binary already handles the heavy lifting. The hook script's only job is to extract the right fields from Gemini's richer JSON schema and pipe them to `memory-ingest` in the expected format.

## Common Pitfalls

### Pitfall 1: Synchronous Hook Blocking

**What goes wrong:** Gemini hooks run synchronously -- the agent loop pauses until ALL matching hooks complete. If `memory-ingest` takes >1 second (e.g., daemon is slow or unreachable), every agent turn is delayed.

**Why it happens:** Gemini's hook system is designed for validation/policy hooks that MUST complete before proceeding. Event capture hooks don't need this guarantee.

**How to avoid:** Background the `memory-ingest` call:
```bash
echo "$payload" | memory-ingest &>/dev/null &
echo '{}'
```
This returns immediately with `{}` (success) while ingestion happens in background.

**Warning signs:** Users report slow Gemini response times after installing hooks.

### Pitfall 2: stdout Pollution Breaking JSON Parsing

**What goes wrong:** Gemini CLI expects the hook's stdout to contain ONLY a JSON object. Any `echo`, debug print, or error message to stdout causes parsing failure and potentially blocks the agent.

**Why it happens:** Developers add debug logging to stdout during development and forget to remove it, or memory-ingest's own stdout output (`{"continue":true}`) leaks through.

**How to avoid:** Redirect ALL output to stderr or /dev/null. The hook script must output exactly one JSON object (`{}`) and nothing else.
```bash
# CORRECT: memory-ingest output goes to /dev/null
echo "$payload" | memory-ingest >/dev/null 2>/dev/null &
echo '{}'

# WRONG: memory-ingest output leaks to stdout
echo "$payload" | memory-ingest
```

**Warning signs:** Gemini shows "hook parse error" warnings or unexpected behavior.

### Pitfall 3: Incorrect settings.json Hook Configuration

**What goes wrong:** Hooks not firing because the settings.json structure uses the wrong nesting level or event name.

**Why it happens:** The settings.json structure requires hooks to be nested inside arrays of matcher groups, not directly under the event name.

**How to avoid:** Follow the exact structure:
```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/hook.sh"
          }
        ]
      }
    ]
  }
}
```
Note the array-of-objects wrapping.

**Warning signs:** Hooks silently don't fire; no error messages.

### Pitfall 4: BeforeAgent vs UserPromptSubmit Confusion

**What goes wrong:** Using `BeforeAgent` to capture user prompts but the hook blocks the agent turn by returning `decision: "deny"` accidentally.

**Why it happens:** `BeforeAgent` supports blocking behavior. If the hook script returns anything other than `{}` or `{"decision":"allow"}`, it may interfere with the agent.

**How to avoid:** Always return `{}` from event capture hooks. Never include `decision`, `continue`, or other control fields in the output.

### Pitfall 5: SessionEnd Hook Not Completing

**What goes wrong:** SessionEnd hooks don't reliably capture data because Gemini CLI uses best-effort execution and may exit before the hook completes.

**Why it happens:** Per the docs: "CLI will not wait for completion" for SessionEnd hooks. The hook process may be killed mid-execution.

**How to avoid:** Keep SessionEnd hook extremely fast. Consider capturing the final state in `AfterAgent` as a backup, and use SessionEnd only for a lightweight "session ended" marker.

### Pitfall 6: Settings.json Merge Conflicts

**What goes wrong:** User's existing settings.json gets overwritten by install script, losing their custom configuration.

**Why it happens:** The install skill writes a complete settings.json instead of merging hook entries into the existing file.

**How to avoid:** The install skill must READ existing settings.json first, then MERGE hook entries into the existing configuration. Use `jq` to safely merge JSON.

## Code Examples

### Example 1: Hook Handler Shell Script

```bash
#!/usr/bin/env bash
# .gemini/hooks/memory-capture.sh
# Captures Gemini CLI lifecycle events into agent-memory.
# Fail-open: never blocks Gemini CLI, even if memory-ingest fails.

# Source: Gemini CLI Hooks Reference (https://geminicli.com/docs/hooks/reference/)

set -euo pipefail

# Read JSON from stdin
INPUT=$(cat)

# Extract base fields available in all hook events
HOOK_EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // empty')
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
TIMESTAMP=$(echo "$INPUT" | jq -r '.timestamp // empty')
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

# Skip if no event name (malformed input)
if [ -z "$HOOK_EVENT" ]; then
  echo '{}'
  exit 0
fi

# Build memory-ingest payload based on event type
case "$HOOK_EVENT" in
  SessionStart)
    PAYLOAD=$(jq -n \
      --arg event "SessionStart" \
      --arg sid "$SESSION_ID" \
      --arg ts "$TIMESTAMP" \
      --arg cwd "$CWD" \
      --arg agent "gemini" \
      '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
    ;;
  SessionEnd)
    PAYLOAD=$(jq -n \
      --arg event "Stop" \
      --arg sid "$SESSION_ID" \
      --arg ts "$TIMESTAMP" \
      --arg cwd "$CWD" \
      --arg agent "gemini" \
      '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
    ;;
  BeforeAgent)
    MESSAGE=$(echo "$INPUT" | jq -r '.prompt // empty')
    PAYLOAD=$(jq -n \
      --arg event "UserPromptSubmit" \
      --arg sid "$SESSION_ID" \
      --arg ts "$TIMESTAMP" \
      --arg cwd "$CWD" \
      --arg msg "$MESSAGE" \
      --arg agent "gemini" \
      '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, message: $msg, agent: $agent}')
    ;;
  AfterAgent)
    MESSAGE=$(echo "$INPUT" | jq -r '.prompt_response // empty')
    PAYLOAD=$(jq -n \
      --arg event "AssistantResponse" \
      --arg sid "$SESSION_ID" \
      --arg ts "$TIMESTAMP" \
      --arg cwd "$CWD" \
      --arg msg "$MESSAGE" \
      --arg agent "gemini" \
      '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, message: $msg, agent: $agent}')
    ;;
  BeforeTool)
    TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
    TOOL_INPUT=$(echo "$INPUT" | jq -c '.tool_input // {}')
    PAYLOAD=$(jq -n \
      --arg event "PreToolUse" \
      --arg sid "$SESSION_ID" \
      --arg ts "$TIMESTAMP" \
      --arg cwd "$CWD" \
      --arg tool "$TOOL_NAME" \
      --argjson tinput "$TOOL_INPUT" \
      --arg agent "gemini" \
      '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
    ;;
  AfterTool)
    TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
    TOOL_INPUT=$(echo "$INPUT" | jq -c '.tool_input // {}')
    PAYLOAD=$(jq -n \
      --arg event "PostToolUse" \
      --arg sid "$SESSION_ID" \
      --arg ts "$TIMESTAMP" \
      --arg cwd "$CWD" \
      --arg tool "$TOOL_NAME" \
      --argjson tinput "$TOOL_INPUT" \
      --arg agent "gemini" \
      '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
    ;;
  *)
    # Unknown event type -- skip silently
    echo '{}'
    exit 0
    ;;
esac

# Send to memory-ingest in background (fail-open, non-blocking)
echo "$PAYLOAD" | memory-ingest >/dev/null 2>/dev/null &

# Return empty JSON to Gemini (no modifications, proceed normally)
echo '{}'
exit 0
```

### Example 2: settings.json Hook Configuration

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "name": "memory-capture-session-start",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture session start into agent-memory"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "name": "memory-capture-session-end",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture session end into agent-memory"
          }
        ]
      }
    ],
    "BeforeAgent": [
      {
        "hooks": [
          {
            "name": "memory-capture-user-prompt",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture user prompts into agent-memory"
          }
        ]
      }
    ],
    "AfterAgent": [
      {
        "hooks": [
          {
            "name": "memory-capture-assistant-response",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture assistant responses into agent-memory"
          }
        ]
      }
    ],
    "BeforeTool": [
      {
        "matcher": "*",
        "hooks": [
          {
            "name": "memory-capture-tool-use",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture tool usage into agent-memory"
          }
        ]
      }
    ],
    "AfterTool": [
      {
        "matcher": "*",
        "hooks": [
          {
            "name": "memory-capture-tool-result",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture tool results into agent-memory"
          }
        ]
      }
    ]
  }
}
```

### Example 3: TOML Command (memory-search)

```toml
# .gemini/commands/memory-search.toml
description = "Search past conversations by topic or keyword using agent-memory"

prompt = """
Search past conversations by topic or keyword using hierarchical TOC navigation.

## Arguments

The user's query follows this instruction. Parse arguments:
- First argument: Topic or keyword to search (required)
- --period <value>: Time period filter (optional, e.g., "last week", "january")
- --agent <value>: Filter by agent (optional, e.g., "gemini", "claude", "opencode")

## Process

1. Check daemon status:
   ```bash
   memory-daemon status
   ```

2. Use tier-aware retrieval to search:
   ```bash
   memory-daemon retrieval status
   memory-daemon retrieval route "{{args}}"
   ```

3. If retrieval route returns results, present them with grip IDs.

4. If no results via retrieval, fall back to TOC navigation:
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 root
   memory-daemon query --endpoint http://[::1]:50051 browse --parent-id "toc:month:2026-02" --limit 20
   ```

5. Search node summaries for matching keywords.

6. Present results with grip IDs for drill-down.

## Output Format

```markdown
## Memory Search: [topic]

### [Time Period]
**Summary:** [matching bullet points]

**Excerpts:**
- "[excerpt text]" `grip:ID`
  _Source: [timestamp]_

---
Expand any excerpt: /memory-context grip:ID
```
"""
```

### Example 4: TOML Command (memory-recent)

```toml
# .gemini/commands/memory-recent.toml
description = "Show recent conversation summaries from agent-memory"

prompt = """
Display recent conversation summaries from the past N days.

## Arguments

Parse from user input after the command:
- --days <N>: Number of days to look back (default: 7)
- --limit <N>: Maximum number of segments to show (default: 10)
- --agent <value>: Filter by agent (optional)

## Process

1. Check daemon status:
   ```bash
   memory-daemon status
   ```

2. Get TOC root to find current year:
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 root
   ```

3. Navigate to current period and collect recent day nodes.

4. Present summaries with timestamps and grip IDs.

## Output Format

```markdown
## Recent Conversations (Last [N] Days)

### [Date]
**Topics:** [keywords from node]
**Segments:**
1. **[Time]** - [segment summary]
   - [bullet] `grip:ID`

---
Total: [N] segments across [M] days
Expand any excerpt: /memory-context grip:ID
```
"""
```

### Example 5: TOML Command (memory-context)

```toml
# .gemini/commands/memory-context.toml
description = "Expand a grip to see full conversation context around an excerpt"

prompt = """
Expand a grip ID to retrieve full conversation context around a specific excerpt.

## Arguments

Parse from user input after the command:
- First argument: Grip ID to expand (required, format: grip:{timestamp}:{ulid})
- --before <N>: Events to include before excerpt (default: 3)
- --after <N>: Events to include after excerpt (default: 3)

## Process

1. Validate grip ID format (must match: grip:{13-digit-timestamp}:{26-char-ulid}).

2. Expand the grip:
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 expand \
     --grip-id "{{args}}" \
     --before 3 \
     --after 3
   ```

3. Format and present the conversation thread.

## Output Format

```markdown
## Conversation Context

**Grip:** `grip:ID`
**Timestamp:** [human-readable date/time]

### Before (N events)
| Role | Message |
|------|---------|
| user | [message text] |
| assistant | [response text] |

### Excerpt (Referenced)
> [The excerpt text]

### After (N events)
| Role | Message |
|------|---------|
| assistant | [continuation] |
| user | [follow-up] |

---
**Source:** [segment ID]
**Session:** [session ID]
```
"""
```

### Example 6: Global Hook Configuration (for ~/.gemini/settings.json)

For global installation (captures events across ALL projects):

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "name": "memory-capture",
            "type": "command",
            "command": "~/.gemini/hooks/memory-capture.sh",
            "timeout": 5000
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "name": "memory-capture",
            "type": "command",
            "command": "~/.gemini/hooks/memory-capture.sh",
            "timeout": 5000
          }
        ]
      }
    ],
    "BeforeAgent": [
      {
        "hooks": [
          {
            "name": "memory-capture",
            "type": "command",
            "command": "~/.gemini/hooks/memory-capture.sh",
            "timeout": 5000
          }
        ]
      }
    ],
    "AfterAgent": [
      {
        "hooks": [
          {
            "name": "memory-capture",
            "type": "command",
            "command": "~/.gemini/hooks/memory-capture.sh",
            "timeout": 5000
          }
        ]
      }
    ],
    "AfterTool": [
      {
        "matcher": "*",
        "hooks": [
          {
            "name": "memory-capture",
            "type": "command",
            "command": "~/.gemini/hooks/memory-capture.sh",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

## Gemini Hook System Details

### Hook Communication Protocol

| Aspect | Detail |
|--------|--------|
| Input | JSON via stdin |
| Output | JSON via stdout (MUST be valid JSON, even `{}`) |
| Logging | stderr only (NEVER stdout) |
| Exit code 0 | Success; stdout parsed as JSON |
| Exit code 2 | System block; stderr becomes rejection reason |
| Other exit codes | Warning; CLI continues normally |
| Timeout default | 60000ms (1 minute) |
| Execution model | Synchronous (blocks agent loop) |

### Configuration Scope

| Scope | Location | Precedence |
|-------|----------|------------|
| Project | `.gemini/settings.json` | Highest |
| User (global) | `~/.gemini/settings.json` | Lower |
| System | `/etc/gemini-cli/settings.json` | Lowest |

### Environment Variable Expansion

Strings in `settings.json` support `$VAR_NAME` or `${VAR_NAME}` syntax for environment variable expansion. This means `$GEMINI_PROJECT_DIR` in a command path resolves at runtime.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No hook system | Full lifecycle hooks via settings.json | Gemini CLI 2025-2026 | Enables event capture without wrapper scripts |
| Shell-only hooks | Command type hooks (shell scripts) | Current | Only `command` type supported; plugin type planned |
| No Claude compatibility | SKILL.md format compatibility | 2026 | Skills from Claude Code work directly in Gemini |
| No session lifecycle hooks | SessionStart/SessionEnd events | Recent PR #14151 | Enables full session boundary capture |

## Open Questions

1. **Hook script path resolution with tilde (~)**
   - What we know: settings.json supports `$GEMINI_PROJECT_DIR` env var expansion
   - What's unclear: Whether `~` (tilde) expands in the `command` field or if `$HOME` must be used
   - Recommendation: Use `$HOME/.gemini/hooks/memory-capture.sh` for global install; verify during implementation

2. **Memory-ingest stdout output interplay**
   - What we know: memory-ingest outputs `{"continue":true}` to stdout; Gemini expects only the hook's JSON on stdout
   - What's unclear: Whether piping to memory-ingest in background fully prevents stdout leakage
   - Recommendation: Use `memory-ingest >/dev/null 2>/dev/null &` to fully suppress; test thoroughly

3. **jq dependency**
   - What we know: The hook script uses `jq` for JSON manipulation; jq is not universally installed
   - What's unclear: Whether all target platforms will have jq available
   - Recommendation: Check for jq in install skill; fall back to Python json module or provide jq installation instructions. Most dev environments have jq. Could also use `node -e` or `python3 -c` as alternatives.

4. **BeforeTool vs AfterTool for event capture**
   - What we know: Claude Code uses both PreToolUse and PostToolUse; Gemini has both BeforeTool and AfterTool
   - What's unclear: Whether capturing BOTH adds too much latency (two synchronous hook calls per tool use)
   - Recommendation: Start with AfterTool only (has both input and output); add BeforeTool if users need pre-execution capture. This reduces hook calls per tool from 2 to 1.

5. **Global vs project-level hook installation**
   - What we know: Both scopes work; project-level has higher precedence
   - What's unclear: Whether global hooks + project hooks create duplicates
   - Recommendation: Default to global install (~/.gemini/settings.json) for convenience; document project-level as alternative. Test for duplicate behavior.

## Sources

### Primary (HIGH confidence)
- [Gemini CLI Hooks Documentation](https://geminicli.com/docs/hooks/) - Complete hook system overview
- [Gemini CLI Hooks Reference](https://geminicli.com/docs/hooks/reference/) - All event types, input/output schemas
- [Gemini CLI Writing Hooks](https://geminicli.com/docs/hooks/writing-hooks/) - Implementation patterns
- [Gemini CLI Custom Commands](https://geminicli.com/docs/cli/custom-commands/) - TOML command format
- [Gemini CLI Skills](https://geminicli.com/docs/cli/skills/) - Agent skills format and discovery
- [Gemini CLI Configuration](https://geminicli.com/docs/get-started/configuration/) - settings.json structure
- [GitHub: google-gemini/gemini-cli hooks docs](https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/index.md) - Source documentation
- [Gemini CLI Hooks Best Practices](https://geminicli.com/docs/hooks/best-practices/) - Performance and security

### Secondary (MEDIUM confidence)
- [Google Developers Blog: Tailor Gemini CLI with hooks](https://developers.googleblog.com/tailor-gemini-cli-to-your-workflow-with-hooks/) - Official blog post on hooks
- [Medium: Gemini CLI Skills Are Here](https://medium.com/ai-software-engineer/gemini-cli-skills-are-here-works-with-your-claude-code-skills-dont-miss-this-update-0ed0d181f73b) - Claude Code skill compatibility confirmation
- [GitHub PR #14151: Hook Session Lifecycle](https://github.com/google-gemini/gemini-cli/pull/14151) - SessionStart/SessionEnd implementation

### Tertiary (LOW confidence)
- [GitHub Issue #16697: SessionStart hooks on 0.24.0](https://github.com/google-gemini/gemini-cli/issues/16697) - Possible SessionStart bug report; may be resolved

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Verified via official Gemini CLI documentation and GitHub source
- Architecture: HIGH - Hook system is well-documented; event mapping verified against memory-ingest interface
- Event mapping: HIGH - All base fields (session_id, hook_event_name, timestamp, cwd) confirmed in official hook reference
- TOML commands: HIGH - Format verified via official docs and GitHub source
- Skills compatibility: HIGH - Multiple sources confirm Claude Code SKILL.md format works in Gemini
- Pitfalls: MEDIUM - Synchronous execution concern verified in docs; specific latency impact needs runtime testing
- Open questions: MEDIUM - jq dependency and stdout interplay need implementation-time validation

**Research date:** 2026-02-09
**Valid until:** 2026-03-09 (30 days - Gemini CLI hook system is stable post-release)
