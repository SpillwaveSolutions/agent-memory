# Memory Adapter for GitHub Copilot CLI

A plugin for [GitHub Copilot CLI](https://github.com/github/copilot-cli) that enables intelligent memory retrieval and automatic event capture, integrating Copilot CLI sessions into the agent-memory ecosystem.

**Version:** 2.1.0

## Overview

This adapter brings the full agent-memory experience to GitHub Copilot CLI: tier-aware query routing, intent classification, automatic fallback chains, and transparent session event capture. Conversations in Copilot CLI become searchable alongside Claude Code, OpenCode, and Gemini CLI sessions, enabling true cross-agent memory.

## Quickstart

```bash
# Option A: Plugin install (recommended, requires Copilot CLI v0.0.406+)
copilot /plugin install /path/to/plugins/memory-copilot-adapter

# Option B: Per-project install (run inside Copilot CLI)
# Copy install skill, then ask Copilot to "install agent memory"
mkdir -p .github/skills
cp -r plugins/memory-copilot-adapter/.github/skills/memory-copilot-install .github/skills/

# Option C: Manual per-project (copy all files directly)
cp -r plugins/memory-copilot-adapter/.github .github

# Verify capture (after a Copilot session):
memory-daemon query root
memory-daemon retrieval route "your topic" --agent copilot
```

## Compatibility

- **Copilot CLI:** v0.0.383+ required (hook support). v0.0.406+ recommended (plugin support, improved skill loading).
- **agent-memory:** v2.1.0 or later (memory-daemon and memory-ingest binaries)
- **jq:** Required for the hook handler script (JSON processing)
  - jq 1.6+ recommended (full recursive redaction via `walk`). jq 1.5 is supported with a simplified del()-based redaction filter (top level + one level deep).

Pin your Copilot CLI version in production environments. The hook system is relatively new and evolving with weekly releases.

## Prerequisites

| Component | Required | Purpose |
|-----------|----------|---------|
| memory-daemon | Yes | Stores and indexes conversation events |
| memory-ingest | Yes | Receives hook events via stdin pipe |
| Copilot CLI | Yes | The CLI tool being integrated (v0.0.383+) |
| jq | Yes | JSON processing in the hook handler script (1.6+ recommended for full redaction; 1.5 works with simplified filter) |

Verify the daemon is running:

```bash
memory-daemon status
memory-daemon start   # Start if not running
```

## Installation

### Plugin Install (Recommended)

The simplest approach. From within Copilot CLI, run:

```
/plugin install /path/to/plugins/memory-copilot-adapter
```

Or from a GitHub repository:

```
/plugin install https://github.com/SpillwaveSolutions/agent-memory/tree/main/plugins/memory-copilot-adapter
```

The plugin system auto-discovers hooks, skills, and agents from the adapter directory structure. Requires Copilot CLI v0.0.406+.

### Automated: Install Skill (Per-Project)

Copy the install skill to your project, then ask Copilot CLI to run it:

```bash
# Copy the install skill to your project
mkdir -p .github/skills
cp -r plugins/memory-copilot-adapter/.github/skills/memory-copilot-install .github/skills/

# Then in Copilot CLI, say:
#   "install agent memory"
#   or "setup memory hooks"
#   or "configure memory capture"
```

The install skill will:
1. Check prerequisites (Copilot CLI version, memory-daemon, memory-ingest, jq)
2. Create directories (`.github/hooks/scripts/`, `.github/skills/`, `.github/agents/`)
3. Copy the hook configuration file and hook handler script
4. Copy query skills
5. Copy the navigator agent
6. Verify the installation

### Manual: Per-Project

Copy the `.github/` directory from the adapter into your project root:

```bash
# Copy all adapter files
cp -r plugins/memory-copilot-adapter/.github .github

# Verify hook script is executable
chmod +x .github/hooks/scripts/memory-capture.sh
```

### No Global Install

**Important:** Copilot CLI does NOT support global hooks (Issue #1157 is open). There is no `~/.copilot/hooks/` directory. Each project needs its own installation, either via:
- **Plugin install** (convenient, one command, applies everywhere)
- **Per-project install** (explicit, files visible in `.github/`)

## Skills

| Skill | Purpose | When Auto-Activated |
|-------|---------|---------------------|
| `memory-query` | Core query capability with tier awareness and command-equivalent instructions | "recall", "search conversations", "find previous session", "what did we discuss" |
| `retrieval-policy` | Tier detection, intent classification, fallbacks | "which search method", "available capabilities", "retrieval tier" |
| `topic-graph` | Topic exploration and discovery | "what topics", "explore subjects", "topic map" |
| `bm25-search` | Keyword search via BM25 index | "keyword search", "exact match", "find term" |
| `vector-search` | Semantic similarity search | "semantic search", "similar concepts", "find related" |
| `memory-copilot-install` | Automated installation and setup | "install memory", "setup agent memory", "configure hooks" |

Skills auto-activate when the user's prompt matches the skill's description. No explicit slash commands are needed -- Copilot CLI infers which skills to use based on context. The `memory-query` skill includes command-equivalent instructions for search, recent, and context operations.

## Navigator Agent

The **memory-navigator** agent provides intelligent memory retrieval with tier-aware routing, intent classification, and automatic fallback chains.

### Invocation

```
/agent memory-navigator
```

Or let Copilot auto-select it (the agent has `infer: true` in its frontmatter, meaning Copilot will invoke it automatically when your query matches its description).

### Capabilities

- **Tier routing:** Detects available search capabilities and routes through the optimal tier
- **Intent classification:** Classifies queries as explore, answer, locate, or time-boxed
- **Fallback chains:** Automatically falls back through retrieval layers when primary methods return insufficient results
- **Explainability:** Every response includes metadata showing the method used, tier level, and layers consulted
- **Cross-agent search:** Queries span all agents by default; filter with `--agent copilot`

### Example Queries

```
@memory-navigator What topics have we discussed recently?
@memory-navigator What approaches have we tried for caching?
@memory-navigator Find the exact error message from JWT validation
@memory-navigator What happened in yesterday's debugging session?
```

The agent uses Copilot CLI tools: `execute` (run CLI commands), `read` (read files), `search` (search codebase).

## Retrieval Tiers

The adapter automatically detects available search capabilities and routes queries through the optimal tier. Higher tiers provide more search layers; lower tiers gracefully degrade.

| Tier | Name | Capabilities | Best For |
|------|------|--------------|----------|
| 1 | Full | Topics + Hybrid + Agentic | Semantic exploration, topic discovery |
| 2 | Hybrid | BM25 + Vector + Agentic | Balanced keyword + semantic search |
| 3 | Semantic | Vector + Agentic | Conceptual similarity queries |
| 4 | Keyword | BM25 + Agentic | Exact term matching |
| 5 | Agentic | TOC navigation only | Always works (no indices required) |

Check your current tier:

```bash
memory-daemon retrieval status
```

Tier 5 (Agentic) is always available and requires no indices. As you build BM25 and vector indices, the system automatically upgrades to higher tiers with more powerful search capabilities.

## Event Capture

### How It Works

The hook handler script (`memory-capture.sh`) is registered via `memory-hooks.json` for 5 Copilot CLI lifecycle events. When Copilot CLI fires a hook, it sends JSON via stdin to the script. The script extracts relevant fields, synthesizes a session ID, transforms the payload into the `memory-ingest` format, and pipes it to the `memory-ingest` binary in the background.

All events are automatically tagged with `agent:copilot` for cross-agent query support.

### Session ID Synthesis

Copilot CLI does NOT provide a `session_id` field in hook input JSON. The adapter generates one:

1. At `sessionStart`: generates a UUID and writes it to a temp file keyed by the CWD hash
2. For subsequent events: reads the session ID from the temp file
3. At `sessionEnd` (with reason "user_exit" or "complete"): reads the session ID, then removes the temp file

Temp files are stored at `/tmp/copilot-memory-session-<cwd-hash>`.

### Event Mapping

| Copilot CLI Event | Agent Memory Event | Mapping Quality | Content Captured |
|-------------------|-------------------|-----------------|------------------|
| sessionStart | SessionStart | Good | Session boundary, working directory |
| sessionEnd | Stop | Good | Session boundary, exit reason |
| userPromptSubmitted | UserPromptSubmit | Exact | User prompt text |
| preToolUse | PreToolUse | Exact | Tool name, tool input (redacted) |
| postToolUse | PostToolUse | Exact | Tool name, tool input (redacted), result |

### Gaps

**1. No AssistantResponse capture.**
Copilot CLI does not provide an `afterAgent` or `assistantResponse` hook. Assistant text responses are NOT captured. The `postToolUse` event's `textResultForLlm` field provides partial coverage (tool output that the assistant used), but the assistant's synthesized text is not available.

**2. No SubagentStart / SubagentStop.**
Copilot CLI does not provide subagent lifecycle hooks. This is a trivial gap -- subagent events are low-priority metadata, not core conversation content.

**3. sessionStart fires per-prompt (Bug #991).**
In interactive mode (reported on v0.0.383), `sessionStart` and `sessionEnd` may fire for every prompt/response cycle instead of once per session. The adapter handles this gracefully by checking if a session file already exists before creating a new session ID. If the file exists, the existing session ID is reused. Session files are only cleaned up when `sessionEnd` fires with reason "user_exit" or "complete".

### Behavior

- **Fail-open:** The hook handler never blocks Copilot CLI. If `memory-ingest` is unavailable or the daemon is down, events are silently dropped. The script always outputs `{}` and exits 0.
- **Backgrounded:** The `memory-ingest` call runs in the background to minimize hook latency.
- **Agent tagging:** All events include `agent: "copilot"` for cross-agent filtering.
- **Sensitive field redaction:** Fields matching `api_key`, `token`, `secret`, `password`, `credential`, `authorization` (case-insensitive) are automatically stripped from `tool_input` and JSON-formatted payloads. Uses `walk()` for jq 1.6+ or `del()` fallback for older versions.
- **ANSI stripping:** The hook handler strips ANSI escape sequences (CSI, OSC, SS2/SS3) from input using perl (preferred) with sed fallback.
- **No stdout pollution:** The hook script outputs only `{}` to stdout. All memory-ingest output is redirected to `/dev/null`.

### Verifying Capture

After a Copilot CLI session, verify events were captured:

```bash
# Check recent events
memory-daemon query events --from $(date -v-1H +%s000) --to $(date +%s000) --limit 5

# Search with agent filter
memory-daemon retrieval route "your query" --agent copilot

# Check TOC for recent data
memory-daemon query root
```

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEMORY_INGEST_PATH` | `memory-ingest` | Override path to memory-ingest binary |
| `MEMORY_INGEST_DRY_RUN` | `0` | Set to `1` to skip actual ingest (testing) |

## Architecture

```
plugins/memory-copilot-adapter/
+-- .github/
|   +-- hooks/
|   |   +-- memory-hooks.json                         # Hook configuration (version: 1, standalone JSON)
|   |   +-- scripts/
|   |       +-- memory-capture.sh                     # Hook handler (fail-open, backgrounded)
|   +-- agents/
|   |   +-- memory-navigator.agent.md                 # Navigator agent (infer: true)
|   +-- skills/
|       +-- memory-query/                             # Core query + command-equivalent instructions
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- retrieval-policy/                         # Tier detection + intent routing
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- topic-graph/                              # Topic exploration
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- bm25-search/                              # BM25 keyword search
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- vector-search/                            # Semantic similarity search
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- memory-copilot-install/                   # Install skill (setup only)
|           +-- SKILL.md
+-- plugin.json                                       # Plugin manifest (for /plugin install)
+-- README.md
+-- .gitignore
```

## Copilot CLI vs Other Adapters

| Aspect | Copilot CLI | Gemini CLI | Claude Code |
|--------|-------------|-----------|-------------|
| Hook config | Standalone `.github/hooks/*.json` | `settings.json` merge | `.claude/hooks.yaml` |
| Commands | Skills only (auto-activated) | TOML + skills | Commands + skills |
| Agent | Proper `.agent.md` file | Embedded in memory-query skill | Separate agent file |
| Global install | Not available (Issue #1157) | `~/.gemini/settings.json` | `~/.claude/hooks.yaml` |
| Session ID | Synthesized via temp file | Provided by CLI | Provided by CLI |
| Assistant response | Not captured (no hook) | Captured (AfterAgent) | Captured (AssistantResponse) |
| Subagent events | Not captured | Not captured | Captured |
| Plugin system | `/plugin install` | None | Plugin marketplace |
| Tool args format | JSON string (double-parse) | JSON object | JSON object |
| Timestamps | Unix milliseconds | ISO 8601 | ISO 8601 |
| sessionStart bug | Per-prompt (Bug #991) | N/A | N/A |

## Troubleshooting

### Daemon not running

**Symptom:** No events being captured; queries return empty results.

**Solution:**

```bash
memory-daemon start
memory-daemon status   # Verify it shows "running"
```

### No results found

**Symptom:** Commands return empty results.

**Possible causes:**
- No conversation data has been ingested yet
- Search terms do not match any stored content
- Time period filter is too narrow

**Solution:**
- Verify data exists: `memory-daemon query root` should show year nodes
- Broaden your search terms
- Try a recent-events query to see what data is available

### Hooks not firing

**Symptom:** Copilot sessions run but no events appear in agent-memory.

**Check `.github/hooks/` exists in the project root:**

```bash
ls -la .github/hooks/memory-hooks.json
# Should show the hook configuration file

ls -la .github/hooks/scripts/memory-capture.sh
# Should show -rwxr-xr-x (executable)
```

**Check Copilot CLI version:**

```bash
copilot --version
# Requires v0.0.383+ for hook support
```

**Verify hook config is valid JSON:**

```bash
jq '.hooks | keys' .github/hooks/memory-hooks.json
# Expected: ["postToolUse","preToolUse","sessionEnd","sessionStart","userPromptSubmitted"]
```

### Sessions fragmented (many 1-event sessions)

**Symptom:** Each user prompt appears as a separate session in memory queries.

**Cause:** Bug #991 -- `sessionStart`/`sessionEnd` fire per-prompt in interactive mode (reported on v0.0.383).

**Solution:** The adapter handles this automatically by reusing session IDs. If a session temp file already exists for the current CWD, the existing session ID is reused instead of generating a new one. Session files are only cleaned up on explicit "user_exit" or "complete" reasons. If you still see fragmented sessions, check that `/tmp/copilot-memory-session-*` files are being created and persisting across prompts.

### jq not installed or too old

**Symptom:** Hook handler silently drops all events (jq missing) or uses simplified redaction (jq < 1.6).

**Solution:**

```bash
# Install jq
brew install jq          # macOS
sudo apt install jq      # Debian/Ubuntu
sudo dnf install jq      # Fedora

# Verify version and walk() support
jq --version
jq -n 'walk(.)' >/dev/null 2>&1 && echo "walk() supported" || echo "walk() not supported (upgrade to 1.6+)"
```

### No global hooks

**Symptom:** Hooks work in one project but not others.

**Cause:** Copilot CLI does not support global hooks (Issue #1157 is open). Hooks are loaded from the project's `.github/hooks/` directory only.

**Solution:** Either:
- Use **plugin install** (`/plugin install /path/to/adapter`) for convenience
- Run the **install skill** in each project where you want memory capture
- **Copy** `.github/hooks/` and `.github/skills/` to each project manually

### ANSI codes in output

**Symptom:** Events contain garbled escape sequences or binary data.

**Cause:** Copilot CLI may include ANSI color codes in hook input.

**Solution:** The adapter strips ANSI escape sequences automatically using perl (CSI+OSC+SS2/SS3 coverage) with sed fallback. If you see garbled data, verify you are using the latest version of `memory-capture.sh`.

### toolArgs parsing errors

**Symptom:** `tool_input` in memory events contains literal escaped JSON strings instead of parsed objects.

**Cause:** Copilot CLI sends `toolArgs` as a JSON-encoded string, not a JSON object.

**Solution:** The adapter handles this automatically by double-parsing `toolArgs` (first extract the string from the outer JSON, then parse the string as JSON). If you see escaped JSON in tool_input, verify the hook script contains the double-parse logic.

### Assistant responses missing

**Symptom:** User prompts and tool usage are captured, but assistant text responses are not.

**Cause:** This is an expected gap. Copilot CLI does not provide an `assistantResponse` or `afterAgent` hook. The adapter cannot capture what the assistant says in text form.

**Workaround:** The `postToolUse` event captures `textResultForLlm`, which contains tool output that the assistant incorporated. This provides partial coverage of assistant "actions" but not synthesized text responses.

## Cross-Agent Queries

One of the key benefits of agent-memory is searching across all agent sessions. After installing the Copilot adapter alongside Claude Code hooks, the OpenCode plugin, or the Gemini adapter, you can query conversations from any agent:

```bash
# Search across ALL agents (Claude Code, OpenCode, Gemini, Copilot)
memory-daemon retrieval route "your query"

# Search Copilot sessions only
memory-daemon retrieval route "your query" --agent copilot

# Search Claude Code sessions only
memory-daemon retrieval route "your query" --agent claude

# Search OpenCode sessions only
memory-daemon retrieval route "your query" --agent opencode

# Search Gemini sessions only
memory-daemon retrieval route "your query" --agent gemini
```

## Related

- [agent-memory](https://github.com/SpillwaveSolutions/agent-memory) -- The memory daemon and storage system
- [memory-gemini-adapter](../memory-gemini-adapter/) -- Gemini CLI adapter with hook-based event capture
- [memory-opencode-plugin](../memory-opencode-plugin/) -- OpenCode query commands, skills, and event capture
- [memory-query-plugin](../memory-query-plugin/) -- Claude Code query commands and skills
- [memory-setup-plugin](../memory-setup-plugin/) -- Claude Code installation wizard

## Version History

- **v2.1.0**: Initial release -- hook-based event capture with session ID synthesis, 5 query skills with Navigator agent, install skill, plugin manifest, cross-agent query support

## License

MIT
