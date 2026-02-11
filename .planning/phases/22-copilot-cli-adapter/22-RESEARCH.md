# Phase 22: Copilot CLI Adapter - Research

**Researched:** 2026-02-10
**Domain:** GitHub Copilot CLI hook system, skills, custom agents, plugin architecture, event capture adapter
**Confidence:** HIGH

## Summary

GitHub Copilot CLI (v0.0.406+ as of February 2026) has a comprehensive hook system, plugin architecture, skills format, and custom agent support. Unlike the Gemini CLI adapter where hooks use `settings.json` with nested objects, Copilot CLI hooks are defined in JSON files at `.github/hooks/*.json` (per-repo) and loaded from the current working directory. The hook system supports 6 event types: `sessionStart`, `sessionEnd`, `userPromptSubmitted`, `preToolUse`, `postToolUse`, and `errorOccurred`. Hooks receive JSON via stdin and must output JSON to stdout (or nothing for events whose output is ignored).

The most critical difference from Gemini CLI is that **Copilot CLI hooks do NOT include a `session_id` field** in their input JSON. The input only contains `timestamp` (milliseconds), `cwd`, and event-specific fields. This means the hook script must generate or derive a session identifier. The recommended approach is to generate a UUID at `sessionStart`, persist it to a temp file, and read it for subsequent events within the same session. The `sessionEnd` hook cleans up the temp file.

For commands and skills, Copilot CLI uses the standard `SKILL.md` format with YAML frontmatter -- the same format used by Claude Code. Skills are stored at `~/.copilot/skills/<name>/SKILL.md` (personal) or `.github/skills/<name>/SKILL.md` (project). Custom agents use `.agent.md` files at `~/.copilot/agents/` (personal) or `.github/agents/` (project). Copilot CLI does NOT use TOML commands like Gemini -- it uses skills (auto-activated) and agents (via `/agent` command). The plugin system (`/plugin install`) supports GitHub repos, URLs, and local paths, with auto-discovery of agents, skills, and hooks.

**Primary recommendation:** Create a Copilot adapter that (1) provides a `.github/hooks/memory-hooks.json` configuration file pointing to a shell hook handler that synthesizes session IDs and calls `memory-ingest` with `agent:copilot` tagging, (2) provides SKILL.md-based skills in `.github/skills/` (or installable via `~/.copilot/skills/`), (3) provides a custom agent definition at `.github/agents/memory-navigator.agent.md`, and (4) packages the whole thing as a plugin installable via `/plugin install`.

## Standard Stack

### Core

| Component | Format | Purpose | Why Standard |
|-----------|--------|---------|--------------|
| Copilot CLI hooks | `.github/hooks/*.json` (JSON v1) | Event capture via lifecycle hooks | Copilot native hook system; loaded from CWD |
| memory-ingest binary | Rust binary (stdin JSON) | Convert hook events to gRPC IngestEvent | Already exists, proven for Claude Code + OpenCode + Gemini |
| Shell wrapper script | Bash script | Transform Copilot hook JSON to memory-ingest format, synthesize session ID | Bridges Copilot's lean hook schema to memory-ingest expected format |
| SKILL.md files | Markdown+YAML in `.github/skills/` or `~/.copilot/skills/` | Skills for memory query/retrieval | Copilot natively supports, same format as Claude Code |
| Agent .md files | Markdown+YAML in `.github/agents/` or `~/.copilot/agents/` | Navigator agent for complex queries | Copilot native custom agent format |

### Supporting

| Component | Format | Purpose | When to Use |
|-----------|--------|---------|-------------|
| `plugin.json` | JSON manifest | Plugin metadata for `/plugin install` | When packaging as installable plugin |
| Session ID temp file | `/tmp/copilot-memory-session-*` | Persist synthesized session ID across hook calls | Every hook call (generated at sessionStart) |
| `~/.copilot/config.json` | JSON config | Global CLI configuration | Reference only; not modified by adapter |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `.github/hooks/` with shell script | Plugin-provided hooks (v0.0.402+) | Plugin hooks are newer and less documented; `.github/hooks/` is the standard documented path |
| Synthesized session ID via temp file | Use `cwd + date` as session key | Temp file is more reliable for multi-prompt sessions; cwd+date could collide |
| Separate skill copies in `.github/skills/` | Plugin that auto-installs skills to `~/.copilot/skills/` | Plugin install is cleaner UX but copies work immediately without `/plugin install` step |
| Markdown SKILL.md skills as "commands" | Copilot has no TOML commands; skills auto-activate | Skills are the correct Copilot mechanism (not TOML like Gemini) |

## Architecture Patterns

### Recommended Project Structure

**Recommendation:** Use a separate `plugins/memory-copilot-adapter/` directory, matching the existing `plugins/memory-gemini-adapter/` and `plugins/memory-opencode-plugin/` pattern.

```
plugins/memory-copilot-adapter/
+-- .github/
|   +-- hooks/
|   |   +-- memory-hooks.json         # Hook configuration (version: 1)
|   |   +-- scripts/
|   |       +-- memory-capture.sh     # Hook handler script
|   +-- agents/
|   |   +-- memory-navigator.agent.md # Navigator agent definition
|   +-- skills/
|       +-- memory-query/
|       |   +-- SKILL.md
|       |   +-- references/
|       |       +-- command-reference.md
|       +-- retrieval-policy/
|       |   +-- SKILL.md
|       |   +-- references/
|       |       +-- command-reference.md
|       +-- bm25-search/
|       |   +-- SKILL.md
|       |   +-- references/
|       |       +-- command-reference.md
|       +-- vector-search/
|       |   +-- SKILL.md
|       |   +-- references/
|       |       +-- command-reference.md
|       +-- topic-graph/
|       |   +-- SKILL.md
|       |   +-- references/
|       |       +-- command-reference.md
|       +-- memory-copilot-install/
|           +-- SKILL.md
+-- plugin.json                       # Plugin manifest (for /plugin install)
+-- README.md
+-- .gitignore
```

**Skill sharing strategy:** Use **separate copies** of SKILL.md files, same as Gemini adapter. The SKILL.md format is identical between Claude Code and Copilot CLI (confirmed by official docs). Copilot CLI also reads from `.claude/skills/` for backward compatibility, but the canonical path is `.github/skills/` for project or `~/.copilot/skills/` for personal.

**Agent definition:** Copilot CLI uses `.agent.md` files. The navigator agent from Claude Code (`agents/memory-navigator.md`) can be adapted to the Copilot agent format. The frontmatter uses `description` (required), `name` (optional), `tools` (optional), and `infer` (optional).

### Pattern 1: Hook Event Capture with Session ID Synthesis

**What:** Shell script hook handler that receives Copilot lifecycle events via stdin JSON, synthesizes a session ID (since Copilot does not provide one), and forwards events to `memory-ingest` with `agent:copilot` tagging.

**When to use:** For all Copilot session event capture.

**How it works:**

Copilot CLI passes JSON via stdin to the hook's bash command. The hook script:
1. Reads the JSON from stdin
2. Extracts event-specific fields (timestamp, cwd, prompt, toolName, toolArgs, etc.)
3. For `sessionStart`: generates a UUID session ID and writes it to a temp file keyed by CWD
4. For other events: reads the session ID from the temp file
5. For `sessionEnd`: reads session ID, then removes the temp file
6. Builds memory-ingest compatible JSON payload with `agent: "copilot"`
7. Pipes to `memory-ingest` in background
8. Outputs `{}` to stdout (or nothing, since output is ignored for most events)

**Session ID synthesis approach:**
```bash
# Generate session ID at sessionStart
SESSION_FILE="/tmp/copilot-memory-session-$(echo "$CWD" | md5sum | cut -d' ' -f1)"
if [ "$EVENT_TYPE" = "sessionStart" ]; then
  SESSION_ID="copilot-$(uuidgen 2>/dev/null || cat /proc/sys/kernel/random/uuid 2>/dev/null || date +%s%N)"
  echo "$SESSION_ID" > "$SESSION_FILE"
else
  SESSION_ID=$(cat "$SESSION_FILE" 2>/dev/null || echo "copilot-unknown")
fi
```

### Pattern 2: SKILL.md Skills (Not TOML Commands)

**What:** Markdown files with YAML frontmatter that define skills for memory queries.

**When to use:** For `/memory-search`, `/memory-recent`, `/memory-context` equivalent functionality.

**Key insight:** Copilot CLI does NOT use TOML commands like Gemini. It uses SKILL.md files that auto-activate based on the skill's description matching the user's prompt. There are no explicit slash commands like `/memory-search` -- instead, skills are auto-loaded when relevant. However, skills can be invoked via the skill name once loaded.

The skill body contains the same instructions that TOML commands contained, but in Markdown format with YAML frontmatter.

### Pattern 3: Custom Agent for Navigator

**What:** An `.agent.md` file that defines the memory navigator as a custom agent.

**When to use:** Copilot CLI supports custom agents natively via `.agent.md` files, unlike Gemini which required embedding navigator logic in a skill.

**How it works:** The agent file uses the same Markdown + YAML frontmatter format. The frontmatter requires `description` field. Optional fields include `name`, `tools`, `target`, `infer`, and `metadata`. The agent body contains the navigator instructions.

The user invokes the agent via `/agent memory-navigator` or Copilot auto-selects it when `infer: true` (default).

### Pattern 4: Plugin Packaging

**What:** Package the entire adapter as a Copilot CLI plugin installable via `/plugin install`.

**When to use:** For streamlined installation experience.

**How it works:** Create a `plugin.json` manifest at the root of the adapter. The plugin system auto-discovers:
- `agents/*.agent.md` files
- `skills/*/SKILL.md` files
- Hook configurations (via plugin-provided hooks, added in v0.0.402)

Users install via: `/plugin install /path/to/memory-copilot-adapter` or from a GitHub repo URL.

### Anti-Patterns to Avoid

- **Creating TOML command files:** Copilot CLI does not use TOML. Use SKILL.md files instead.
- **Expecting `session_id` in hook input:** Copilot does not provide session_id. You MUST synthesize one.
- **Expecting global hooks at `~/.copilot/hooks/`:** Global hooks are NOT supported yet (Issue #1157 is open). Hooks are per-repo only (`.github/hooks/`) or via plugins.
- **Writing to stdout in non-preToolUse hooks:** Only `preToolUse` hooks process stdout output. For other hooks, stdout is ignored. However, the script should still be clean (no stray output).
- **Using `.claude/skills/` path for Copilot:** While Copilot CLI reads `.claude/skills/` for backward compatibility, the canonical path is `.github/skills/`. Use `.github/skills/` for clarity.
- **Relying on sessionStart firing once per session:** Bug #991 reports that `sessionStart`/`sessionEnd` fire per-prompt in interactive mode (v0.0.383). The session ID synthesis must handle this by checking if a session file already exists and reusing it.

## Event Mapping: Copilot CLI to Agent Memory

### Complete Event Mapping

| Copilot CLI Event | Agent Memory Event | memory-ingest hook_event_name | Mapping Quality | Notes |
|-------------------|-------------------|------------------------------|-----------------|-------|
| `sessionStart` | `SessionStart` | `"SessionStart"` | GOOD | No session_id from Copilot; must synthesize. `source` field ("new"/"resume"/"startup") available |
| `sessionEnd` | `SessionEnd` / `Stop` | `"Stop"` | GOOD | `reason` field ("complete"/"error"/"abort"/"timeout"/"user_exit") available |
| `userPromptSubmitted` | `UserPromptSubmit` | `"UserPromptSubmit"` | EXACT | `prompt` field contains user text |
| `preToolUse` | `PreToolUse` | `"PreToolUse"` | EXACT | `toolName` and `toolArgs` (JSON string) available |
| `postToolUse` | `PostToolUse` | `"PostToolUse"` | EXACT | `toolName`, `toolArgs`, `toolResult` available |
| `errorOccurred` | (no mapping) | N/A | SKIP | Error diagnostics; not conversation content |

### Copilot CLI Base Input Schema (All Events)

Every hook receives these fields via stdin:

```json
{
  "timestamp": 1704614400000,
  "cwd": "/path/to/project"
}
```

**CRITICAL: No `session_id` field. No `hook_event_name` field.**

The event type is NOT in the JSON payload -- the hook script knows which event it handles because each event type is configured separately in the hooks JSON file. This is a fundamental difference from Gemini CLI (which sends `hook_event_name` in the JSON).

### Event-Specific Input Fields

**sessionStart** additional: `{ "source": "new" | "resume" | "startup", "initialPrompt": "string" }`
**sessionEnd** additional: `{ "reason": "complete" | "error" | "abort" | "timeout" | "user_exit" }`
**userPromptSubmitted** additional: `{ "prompt": "string" }`
**preToolUse** additional: `{ "toolName": "string", "toolArgs": "string (JSON)" }`
**postToolUse** additional: `{ "toolName": "string", "toolArgs": "string (JSON)", "toolResult": { "resultType": "success" | "failure" | "denied", "textResultForLlm": "string" } }`
**errorOccurred** additional: `{ "error": { "message": "string", "name": "string", "stack": "string" } }`

### Parity Assessment

| Claude Code Event | Copilot CLI Equivalent | Parity |
|------------------|----------------------|--------|
| SessionStart | sessionStart | GOOD (no session_id) |
| UserPromptSubmit | userPromptSubmitted | FULL |
| AssistantResponse | (none) | GAP - No assistant response hook |
| PreToolUse | preToolUse | FULL |
| PostToolUse | postToolUse | FULL |
| Stop / SessionEnd | sessionEnd | GOOD (no session_id) |
| SubagentStart | (none) | GAP - No subagent events |
| SubagentStop | (none) | GAP - No subagent events |

**Gap analysis:** The most significant gap is **no AssistantResponse hook**. Copilot CLI has `userPromptSubmitted` and tool hooks, but no event fires when the assistant generates a response. This means assistant responses are NOT captured. Workaround options:
1. **Accept the gap:** Capture user prompts and tool usage only. This still provides useful conversation history.
2. **Use postToolUse as partial proxy:** Tool results contain `textResultForLlm` which captures tool output. This covers much of what the assistant "does" but not its text responses.
3. **Wait for future hooks:** Issue #1157 requests a "Stop" event (equivalent to AssistantResponse/AgentStop). This may be added in future versions.

**Recommendation:** Accept the gap for v1. Capture sessionStart, sessionEnd, userPromptSubmitted, preToolUse, and postToolUse. Document that assistant text responses are not captured. This is similar to how Gemini's SubagentStart/SubagentStop gaps were handled -- document the gap, no workaround needed for v1.

## Hook Configuration Design

### Key Design Decision: Separate Scripts Per Event vs Single Script

**Gemini approach:** Single `memory-capture.sh` script handles all events, switching on `hook_event_name` from the JSON payload.

**Copilot constraint:** The event type is NOT in the JSON. Each hook event type maps to a separate entry in the hooks JSON config. Two approaches:

**Option A: Single script with event type as argument**
```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [{
      "type": "command",
      "bash": ".github/hooks/scripts/memory-capture.sh sessionStart"
    }],
    "userPromptSubmitted": [{
      "type": "command",
      "bash": ".github/hooks/scripts/memory-capture.sh userPromptSubmitted"
    }]
  }
}
```
The script receives the event type as `$1` and JSON via stdin.

**Option B: Separate scripts per event**
Separate scripts: `memory-session-start.sh`, `memory-session-end.sh`, etc.

**Recommendation:** Option A (single script with event type argument). This mirrors the Gemini adapter pattern, reduces code duplication, and is easier to maintain. The `$1` argument replaces Gemini's `hook_event_name` field.

### Hook Configuration Structure

```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh sessionStart",
        "timeoutSec": 10
      }
    ],
    "sessionEnd": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh sessionEnd",
        "timeoutSec": 10
      }
    ],
    "userPromptSubmitted": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh userPromptSubmitted",
        "timeoutSec": 10
      }
    ],
    "preToolUse": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh preToolUse",
        "timeoutSec": 10
      }
    ],
    "postToolUse": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh postToolUse",
        "timeoutSec": 10
      }
    ]
  }
}
```

Note: `errorOccurred` is intentionally omitted. Error events are not conversation content.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing in shell | Custom awk/sed parsing | `jq` utility | JSON from Copilot includes nested objects (toolArgs, toolResult) |
| Event type mapping | Custom type converter | Existing `memory-ingest` mapping | memory-ingest already maps hook_event_name to EventType |
| Fail-open hook behavior | Custom error handling | Shell `|| true` + trap | Copilot treats non-zero exit as warning but continues |
| Skill file format | Custom command format | Standard SKILL.md (Claude compatible) | Copilot uses identical SKILL.md format to Claude Code |
| Session ID persistence | Custom database/state | Temp file per CWD | Simple, reliable, cleaned up on sessionEnd |
| Plugin packaging | Custom installer | Copilot `/plugin install` | Native plugin system handles discovery + registration |
| UUID generation | Custom ID scheme | `uuidgen` or `/proc/sys/kernel/random/uuid` | Standard, unique, portable |

**Key insight:** The `memory-ingest` binary handles the heavy lifting. The hook script's job is to (1) synthesize a session ID, (2) pass the event type from `$1`, (3) extract event-specific fields from stdin JSON, and (4) pipe the transformed payload to `memory-ingest`.

## Common Pitfalls

### Pitfall 1: Missing Session ID

**What goes wrong:** Copilot CLI does NOT include `session_id` in hook input JSON. Without a session ID, memory-ingest cannot group events into sessions.

**Why it happens:** Copilot's hook system was designed for security/validation (preToolUse deny/allow), not for conversation tracking. Session management is internal to the CLI.

**How to avoid:** Synthesize a session ID at `sessionStart`:
- Generate a UUID (via `uuidgen` or fallback)
- Write it to a temp file keyed by the CWD hash
- Read it back for all subsequent events
- Clean up on `sessionEnd`

**Warning signs:** All events have `session_id: "unknown"` or empty. Events from the same session appear as separate sessions in queries.

### Pitfall 2: sessionStart/sessionEnd Firing Per-Prompt (Bug #991)

**What goes wrong:** In interactive mode (v0.0.383+), `sessionStart` and `sessionEnd` hooks fire for EVERY prompt/response cycle, not once per session.

**Why it happens:** Known bug (Issue #991, open as of January 2026). Copilot CLI treats each prompt round as a mini-session internally.

**How to avoid:** When handling `sessionStart`, check if a session file already exists for this CWD:
- If yes: reuse the existing session ID (do NOT create a new one)
- If no: create a new session ID and write to file
- Only delete the session file on explicit user exit (check `sessionEnd` reason = "user_exit")

**Warning signs:** Hundreds of 1-event "sessions" in memory queries. Each user prompt appears as a separate session.

### Pitfall 3: toolArgs is a JSON String, Not an Object

**What goes wrong:** The `toolArgs` field in `preToolUse` and `postToolUse` is a **JSON-encoded string**, not a JSON object. Treating it as an object causes parsing errors.

**Why it happens:** Copilot serializes tool arguments as a string for transport in the hook payload.

**How to avoid:** Parse `toolArgs` twice: first as a field of the outer JSON, then parse the string value as JSON:
```bash
TOOL_ARGS_STR=$(echo "$INPUT" | jq -r '.toolArgs // "{}"')
TOOL_ARGS=$(echo "$TOOL_ARGS_STR" | jq -c '.' 2>/dev/null || echo '{}')
```

**Warning signs:** `tool_input` in memory events contains literal escaped JSON strings instead of parsed objects.

### Pitfall 4: No Global Hooks Support

**What goes wrong:** Hooks defined at `~/.copilot/hooks/` do not work. Hooks are only loaded from the CWD's `.github/hooks/` directory.

**Why it happens:** Copilot CLI's hook system is repo-centric by design. Global hooks are an open feature request (Issue #1157).

**How to avoid:** Two installation strategies:
1. **Per-project:** Copy `.github/hooks/` into each project (the install skill automates this)
2. **Plugin-based:** Package as a plugin that provides hooks (v0.0.402+ supports plugin-provided hooks)

**Warning signs:** Hooks work in one project but not others. User expects global capture like Gemini adapter.

### Pitfall 5: Timestamps in Milliseconds, Not ISO 8601

**What goes wrong:** Copilot provides `timestamp` as Unix milliseconds (e.g., `1704614400000`), but memory-ingest expects ISO 8601 strings.

**Why it happens:** Different timestamp conventions between Copilot and the memory-ingest interface.

**How to avoid:** Convert in the hook script:
```bash
TS_MS=$(echo "$INPUT" | jq -r '.timestamp // 0')
TS_ISO=$(date -d @$((TS_MS / 1000)) -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || \
         date -r $((TS_MS / 1000)) -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || \
         date -u +"%Y-%m-%dT%H:%M:%SZ")
```
Note: `date -d` is Linux, `date -r` is macOS. Handle both.

**Warning signs:** Events have epoch timestamps or "1970-01-01" dates in memory queries.

### Pitfall 6: Script Path Resolution in Hooks JSON

**What goes wrong:** Hook `bash` commands use relative paths that resolve relative to the hooks file location, but might not resolve correctly from CWD.

**Why it happens:** The `cwd` field in hook config defaults to `"."` (current working directory of the CLI), but the script path in `bash` is also relative to CWD.

**How to avoid:** Use paths relative to the project root where `.github/hooks/` lives:
```json
"bash": ".github/hooks/scripts/memory-capture.sh sessionStart"
```
Or use the `cwd` field:
```json
"bash": "./scripts/memory-capture.sh sessionStart",
"cwd": ".github/hooks"
```

**Warning signs:** "command not found" errors in hook execution.

## Code Examples

### Example 1: Hook Configuration (memory-hooks.json)

```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh sessionStart",
        "timeoutSec": 10,
        "comment": "Capture session start into agent-memory"
      }
    ],
    "sessionEnd": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh sessionEnd",
        "timeoutSec": 10,
        "comment": "Capture session end into agent-memory"
      }
    ],
    "userPromptSubmitted": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh userPromptSubmitted",
        "timeoutSec": 10,
        "comment": "Capture user prompts into agent-memory"
      }
    ],
    "preToolUse": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh preToolUse",
        "timeoutSec": 10,
        "comment": "Capture tool invocations into agent-memory"
      }
    ],
    "postToolUse": [
      {
        "type": "command",
        "bash": ".github/hooks/scripts/memory-capture.sh postToolUse",
        "timeoutSec": 10,
        "comment": "Capture tool results into agent-memory"
      }
    ]
  }
}
```

### Example 2: Hook Handler Shell Script (memory-capture.sh)

```bash
#!/usr/bin/env bash
# .github/hooks/scripts/memory-capture.sh
# Captures Copilot CLI lifecycle events into agent-memory.
# Fail-open: never blocks Copilot CLI, even if memory-ingest fails.
#
# Usage: Called by Copilot CLI hooks with event type as $1.
#   .github/hooks/scripts/memory-capture.sh sessionStart < <(stdin JSON)
#
# CRITICAL DIFFERENCES FROM GEMINI ADAPTER:
#   1. No session_id in hook input -- synthesized via temp file
#   2. No hook_event_name in hook input -- passed as $1 argument
#   3. Timestamps are Unix milliseconds, not ISO 8601
#   4. toolArgs is a JSON string, not an object

set -euo pipefail

fail_open() {
  echo '{}'
  exit 0
}
trap fail_open ERR EXIT

main_logic() {
  EVENT_TYPE="${1:-}"

  # Guard: check required tools
  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  # Guard: need event type
  if [ -z "$EVENT_TYPE" ]; then
    return 0
  fi

  # Read stdin
  INPUT=$(cat) || return 0
  if [ -z "$INPUT" ]; then
    return 0
  fi

  # Strip ANSI escape sequences
  INPUT=$(printf '%s' "$INPUT" | sed $'s/\x1b\[[0-9;]*[a-zA-Z]//g') || return 0

  # Validate JSON
  if ! echo "$INPUT" | jq empty 2>/dev/null; then
    return 0
  fi

  # Extract base fields
  CWD=$(echo "$INPUT" | jq -r '.cwd // empty') || return 0
  TS_MS=$(echo "$INPUT" | jq -r '.timestamp // 0') || return 0

  # Convert timestamp from milliseconds to ISO 8601
  if [ "$TS_MS" != "0" ] && [ -n "$TS_MS" ]; then
    TS_SEC=$((TS_MS / 1000))
    TIMESTAMP=$(date -r "$TS_SEC" -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || \
                date -d "@$TS_SEC" -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || \
                date -u +"%Y-%m-%dT%H:%M:%SZ")
  else
    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  fi

  # Session ID synthesis via temp file
  CWD_HASH=$(printf '%s' "${CWD:-unknown}" | md5sum 2>/dev/null | cut -d' ' -f1 || \
             printf '%s' "${CWD:-unknown}" | md5 2>/dev/null || \
             echo "default")
  SESSION_FILE="/tmp/copilot-memory-session-${CWD_HASH}"

  case "$EVENT_TYPE" in
    sessionStart)
      if [ -f "$SESSION_FILE" ]; then
        SESSION_ID=$(cat "$SESSION_FILE")
      else
        SESSION_ID="copilot-$(uuidgen 2>/dev/null | tr '[:upper:]' '[:lower:]' || \
                    cat /proc/sys/kernel/random/uuid 2>/dev/null || \
                    echo "$(date +%s)-$$")"
        echo "$SESSION_ID" > "$SESSION_FILE"
      fi
      ;;
    sessionEnd)
      SESSION_ID=$(cat "$SESSION_FILE" 2>/dev/null || echo "copilot-unknown")
      REASON=$(echo "$INPUT" | jq -r '.reason // empty')
      if [ "$REASON" = "user_exit" ] || [ "$REASON" = "complete" ]; then
        rm -f "$SESSION_FILE" 2>/dev/null
      fi
      ;;
    *)
      SESSION_ID=$(cat "$SESSION_FILE" 2>/dev/null || echo "copilot-unknown")
      ;;
  esac

  # Redaction filter for sensitive fields
  REDACT_FILTER='walk(if type == "object" then with_entries(select(.key | test("api_key|token|secret|password|credential|authorization"; "i") | not)) else . end)'

  # Build payload based on event type
  local PAYLOAD=""
  case "$EVENT_TYPE" in
    sessionStart)
      PAYLOAD=$(jq -n \
        --arg event "SessionStart" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
      ;;
    sessionEnd)
      PAYLOAD=$(jq -n \
        --arg event "Stop" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, agent: $agent}')
      ;;
    userPromptSubmitted)
      MESSAGE=$(echo "$INPUT" | jq -r '.prompt // empty')
      PAYLOAD=$(jq -n \
        --arg event "UserPromptSubmit" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg msg "$MESSAGE" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, message: $msg, agent: $agent}')
      ;;
    preToolUse)
      TOOL_NAME=$(echo "$INPUT" | jq -r '.toolName // empty')
      # toolArgs is a JSON string, parse it
      TOOL_ARGS_STR=$(echo "$INPUT" | jq -r '.toolArgs // "{}"')
      TOOL_INPUT=$(echo "$TOOL_ARGS_STR" | jq -c "$REDACT_FILTER" 2>/dev/null || echo '{}')
      PAYLOAD=$(jq -n \
        --arg event "PreToolUse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg tool "$TOOL_NAME" \
        --argjson tinput "$TOOL_INPUT" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
      ;;
    postToolUse)
      TOOL_NAME=$(echo "$INPUT" | jq -r '.toolName // empty')
      TOOL_ARGS_STR=$(echo "$INPUT" | jq -r '.toolArgs // "{}"')
      TOOL_INPUT=$(echo "$TOOL_ARGS_STR" | jq -c "$REDACT_FILTER" 2>/dev/null || echo '{}')
      PAYLOAD=$(jq -n \
        --arg event "PostToolUse" \
        --arg sid "$SESSION_ID" \
        --arg ts "$TIMESTAMP" \
        --arg cwd "$CWD" \
        --arg tool "$TOOL_NAME" \
        --argjson tinput "$TOOL_INPUT" \
        --arg agent "copilot" \
        '{hook_event_name: $event, session_id: $sid, timestamp: $ts, cwd: $cwd, tool_name: $tool, tool_input: $tinput, agent: $agent}')
      ;;
    *)
      return 0
      ;;
  esac

  if [ -z "$PAYLOAD" ]; then
    return 0
  fi

  local INGEST_BIN="${MEMORY_INGEST_PATH:-memory-ingest}"

  if [ "${MEMORY_INGEST_DRY_RUN:-0}" = "1" ]; then
    return 0
  fi

  echo "$PAYLOAD" | "$INGEST_BIN" >/dev/null 2>/dev/null &

  return 0
}

main_logic "$@"
```

### Example 3: SKILL.md for Memory Query (Copilot format)

```markdown
---
name: memory-query
description: |
  Query past conversations from the agent-memory system. Use when asked to
  "recall what we discussed", "search conversation history", "find previous
  session", "what did we talk about last week", or "get context from earlier".
  Provides tier-aware retrieval with automatic fallback chains and intent-based
  routing.
license: MIT
metadata:
  version: 2.1.0
  author: SpillwaveSolutions
---

# Memory Query Skill

[Same content as Gemini/Claude Code skill]
```

### Example 4: Agent Definition (memory-navigator.agent.md)

```markdown
---
name: memory-navigator
description: |
  Autonomous agent for intelligent memory retrieval with tier-aware routing,
  intent classification, and automatic fallback chains. Invoke when asked about
  past conversations, previous sessions, or historical code discussions.
tools: ["execute", "read", "search"]
infer: true
---

# Memory Navigator Agent

[Same content as Claude Code navigator agent, adapted for Copilot CLI tool names]
```

### Example 5: Plugin Manifest (plugin.json)

```json
{
  "name": "memory-copilot-adapter",
  "version": "2.1.0",
  "description": "Agent memory adapter for GitHub Copilot CLI - enables intelligent memory retrieval and automatic event capture",
  "author": "SpillwaveSolutions",
  "repository": "https://github.com/SpillwaveSolutions/agent-memory"
}
```

## Copilot CLI vs Gemini CLI: Key Differences for Adapter Design

| Aspect | Gemini CLI | Copilot CLI | Impact |
|--------|-----------|-------------|--------|
| Hook config format | `settings.json` with nested arrays | `.github/hooks/*.json` with version field | Different JSON structure |
| Hook input | `hook_event_name` in JSON | Event type NOT in JSON; use separate config entries | Must pass event type as script argument |
| Session ID | `session_id` in JSON payload | NOT provided | Must synthesize via temp file |
| Timestamps | ISO 8601 strings | Unix milliseconds | Must convert in hook script |
| Commands | TOML files in `.gemini/commands/` | No TOML; use SKILL.md files instead | Skills replace explicit commands |
| Skills | `.gemini/skills/` | `.github/skills/` or `~/.copilot/skills/` | Different path convention |
| Custom agents | Not supported (embed in skill) | `.agent.md` files in `.github/agents/` | Can define proper navigator agent |
| Global hooks | `~/.gemini/settings.json` | NOT supported (Issue #1157 open) | Per-repo only; plugin can help |
| Assistant response capture | AfterAgent event | NOT available | Gap -- no assistant text capture |
| Tool args format | `tool_input` (JSON object) | `toolArgs` (JSON string) | Must parse string as JSON |
| Plugin system | None | `/plugin install` with plugin.json | Can package as installable plugin |
| Hook output | Must return `{}` (parsed) | Output ignored (except preToolUse) | Less strict stdout requirements |
| sessionStart bug | N/A | Fires per-prompt (Bug #991) | Session ID reuse logic needed |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `gh copilot` extension | Standalone `copilot` CLI | Oct 2025 | Completely new tool with agentic capabilities |
| No hook system | Full lifecycle hooks via `.github/hooks/*.json` | v0.0.383 (Jan 2026) | Enables event capture |
| No plugins | Plugin system with `/plugin install` | v0.0.392 (Jan 2026) | Enables installable adapters |
| No custom agents | `.agent.md` files with YAML frontmatter | v0.0.396 (Jan 2026) | Enables navigator agent |
| No skills | SKILL.md in `.github/skills/` | v0.0.401 (Jan 2026) | Enables memory query skills |
| Plugin hooks not supported | Plugins can provide hooks | v0.0.402 (Feb 2026) | Enables plugin-based hook installation |
| Commands from plugins separate | Plugin commands translate to skills | v0.0.406 (Feb 2026) | Skills are the unified command mechanism |

**Deprecated/outdated:**
- `gh copilot` extension: Deprecated October 2025. Replaced by standalone `copilot` CLI.
- `.copilot-hooks.json` at repo root: Replaced by `.github/hooks/*.json` directory pattern.

## Open Questions

1. **Plugin-provided hooks vs `.github/hooks/` files**
   - What we know: v0.0.402 added "plugins can provide hooks for session lifecycle events"
   - What's unclear: Exact format for plugin-provided hooks. Does the plugin simply include `.github/hooks/` in its directory structure, or is there a separate plugin hook registration mechanism?
   - Recommendation: Use `.github/hooks/` for v1 (well-documented). Explore plugin-provided hooks as a future enhancement.
   - Confidence: LOW -- plugin hook mechanism is underdocumented

2. **sessionStart/sessionEnd per-prompt bug (Issue #991)**
   - What we know: Bug reported on v0.0.383. sessionStart and sessionEnd fire per-prompt in interactive mode.
   - What's unclear: Whether this is fixed in v0.0.406+. Issue is still open.
   - Recommendation: Implement session ID reuse logic (check if session file exists before creating new one). This works regardless of whether the bug is fixed.
   - Confidence: MEDIUM -- workaround handles both cases

3. **Global hooks support timeline**
   - What we know: Issue #1157 requests global hooks at `~/.copilot/hooks.json`. Still open, no developer response.
   - What's unclear: Whether/when this will be implemented.
   - Recommendation: Design for per-repo installation now. The install skill copies `.github/hooks/` into each project. If global hooks arrive later, the adapter can be updated.
   - Confidence: HIGH -- per-repo hooks are the current documented mechanism

4. **Assistant response capture**
   - What we know: No `assistantResponse` or `afterAgent` hook event exists.
   - What's unclear: Whether this will be added. Issue #1157 requests a "Stop" event but no "AssistantResponse" equivalent.
   - Recommendation: Accept the gap. Capture prompts and tool use only. Document the limitation. The postToolUse `textResultForLlm` field captures some assistant "output" indirectly.
   - Confidence: HIGH -- documented hook types do not include assistant response

5. **md5sum availability on macOS**
   - What we know: macOS uses `md5` instead of `md5sum` for hashing.
   - What's unclear: Whether all target platforms handle the CWD hash correctly.
   - Recommendation: Use fallback chain: `md5sum | cut -d' ' -f1 || md5 || echo default`
   - Confidence: HIGH -- standard cross-platform shell pattern

6. **Hook script CWD resolution**
   - What we know: The `bash` field in hook config is executed with CWD defaulting to `"."`. The `cwd` field can override this.
   - What's unclear: Whether `.github/hooks/scripts/memory-capture.sh` resolves relative to repo root or the hook JSON file location.
   - Recommendation: Test during implementation. If relative paths don't work, use `$(dirname "$0")/../scripts/memory-capture.sh` or absolute paths.
   - Confidence: MEDIUM -- needs implementation-time validation

7. **SKILL.md frontmatter compatibility**
   - What we know: Copilot uses `name` + `description` (required) in YAML frontmatter. Claude Code uses `name` + `description` + optional fields.
   - What's unclear: Whether Copilot accepts extra frontmatter fields (like `license`, `metadata`) without errors.
   - Recommendation: v0.0.403 added "Skills with unknown frontmatter fields now load with warnings instead of errors." Include all frontmatter fields from existing skills; they will load with warnings at worst.
   - Confidence: HIGH -- documented behavior since v0.0.403

## Sources

### Primary (HIGH confidence)
- [GitHub Docs: Using hooks with GitHub Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-hooks) - Hook usage guide for CLI
- [GitHub Docs: Hooks configuration reference](https://docs.github.com/en/copilot/reference/hooks-configuration) - Complete input/output schemas for all events
- [GitHub Docs: About GitHub Copilot CLI](https://docs.github.com/en/copilot/concepts/agents/about-copilot-cli) - CLI overview, features, modes
- [GitHub Docs: About hooks](https://docs.github.com/en/copilot/concepts/agents/coding-agent/about-hooks) - Hook concepts and event types
- [GitHub Docs: Custom agents configuration](https://docs.github.com/en/copilot/reference/custom-agents-configuration) - Agent file format, frontmatter fields
- [GitHub Docs: About Agent Skills](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills) - SKILL.md format, discovery paths
- [GitHub Docs: Using GitHub Copilot CLI](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/use-copilot-cli) - Slash commands, configuration, agent/skill loading

### Secondary (MEDIUM confidence)
- [DeepWiki: Plugin System](https://deepwiki.com/github/copilot-cli/5.4-plugin-system) - Plugin directory structure, installation, component types
- [DeepWiki: Plugin & MCP Integration Architecture](https://deepwiki.com/github/copilot-cli/6.6-plugin-and-mcp-integration-architecture) - Plugin loading pipeline, manifest format
- [GitHub copilot-cli changelog](https://github.com/github/copilot-cli/blob/main/changelog.md) - Version history, feature additions
- [GitHub copilot-cli releases](https://github.com/github/copilot-cli/releases) - Release notes with hook/plugin changes

### Tertiary (LOW confidence)
- [GitHub Issue #991: sessionStart fires per-prompt](https://github.com/github/copilot-cli/issues/991) - Bug report, open, may be fixed in newer versions
- [GitHub Issue #1157: Global hooks request](https://github.com/github/copilot-cli/issues/1157) - Feature request, no developer response
- [GitHub Issue #1139: Hook output injection](https://github.com/github/copilot-cli/issues/1139) - Feature request for hook context injection
- [GitHub Issue #971: Hooks system request](https://github.com/github/copilot-cli/issues/971) - Original hooks request, COMPLETED
- [GitHub Issue #1310: Hook functions request](https://github.com/github/copilot-cli/issues/1310) - Additional hooks request, open

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Verified via official GitHub Docs and copilot-cli repo
- Architecture (hooks): HIGH - Hook format, event types, and input schemas confirmed in official docs
- Architecture (skills/agents): HIGH - SKILL.md and .agent.md formats confirmed in official docs
- Event mapping: HIGH - All input schemas verified against hooks configuration reference
- Session ID synthesis: MEDIUM - Approach is sound but implementation needs runtime testing
- Plugin packaging: MEDIUM - Plugin system documented but plugin-provided hooks are newer/less documented
- Pitfalls: HIGH - Bug #991, missing session_id, toolArgs format all verified through official sources
- Open questions: MEDIUM - Several items need implementation-time validation

**Research date:** 2026-02-10
**Valid until:** 2026-03-10 (30 days -- Copilot CLI is fast-moving with weekly releases; hook system could change)
