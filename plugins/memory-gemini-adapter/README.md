# Memory Adapter for Gemini CLI

A plugin for [Gemini CLI](https://github.com/google-gemini/gemini-cli) that enables intelligent memory retrieval and automatic event capture, integrating Gemini sessions into the agent-memory ecosystem.

**Version:** 2.1.0

## Overview

This adapter brings the full agent-memory experience to Gemini CLI: tier-aware query routing, intent classification, automatic fallback chains, and transparent session event capture. Conversations in Gemini CLI become searchable alongside Claude Code and OpenCode sessions, enabling true cross-agent memory.

## Quickstart

```bash
# 1. Copy the install skill into your project (or globally)
cp -r plugins/memory-gemini-adapter/.gemini/skills/memory-gemini-install ~/.gemini/skills/

# 2. In Gemini CLI, ask it to install:
#    "install agent memory"

# 3. Verify capture (after a Gemini session):
memory-daemon query root
memory-daemon retrieval route "your topic" --agent gemini
```

## Compatibility

- **Gemini CLI:** Requires a version with hook support (`settings.json` hooks system). See [Gemini CLI Hooks Documentation](https://geminicli.com/docs/hooks/).
- **agent-memory:** v2.1.0 or later (memory-daemon and memory-ingest binaries)
- **jq:** Required for the hook handler script (JSON processing)

## Prerequisites

| Component | Required | Purpose |
|-----------|----------|---------|
| memory-daemon | Yes | Stores and indexes conversation events |
| memory-ingest | Yes | Receives hook events via stdin pipe |
| Gemini CLI | Yes | The CLI tool being integrated |
| jq | Yes | JSON processing in the hook handler script |

Verify the daemon is running:

```bash
memory-daemon status
memory-daemon start   # Start if not running
```

## Installation

### Automated: Install Skill

The recommended approach. Copy the install skill to your Gemini CLI skills directory, then ask Gemini to run it.

**Global install (all projects):**

```bash
# Copy the install skill
cp -r plugins/memory-gemini-adapter/.gemini/skills/memory-gemini-install ~/.gemini/skills/

# Then in Gemini CLI, say:
#   "install agent memory"
#   or "setup memory hooks"
#   or "configure memory capture"
```

The install skill will:
1. Check prerequisites (Gemini CLI, memory-daemon, memory-ingest, jq)
2. Copy the hook handler script to `~/.gemini/hooks/`
3. Merge hook configuration into `~/.gemini/settings.json` (preserving existing settings)
4. Copy slash commands to `~/.gemini/commands/`
5. Copy query skills to `~/.gemini/skills/`
6. Verify the installation

### Manual: Global Installation

Copy all adapter files to the global Gemini CLI configuration directory:

```bash
# Create directories
mkdir -p ~/.gemini/hooks ~/.gemini/commands ~/.gemini/skills

# Copy hook handler
cp plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh ~/.gemini/hooks/
chmod +x ~/.gemini/hooks/memory-capture.sh

# Merge hook configuration into settings.json
# IMPORTANT: Do NOT overwrite -- merge hooks into existing settings
EXISTING=$(cat ~/.gemini/settings.json 2>/dev/null || echo '{}')
HOOKS=$(cat plugins/memory-gemini-adapter/.gemini/settings.json | jq '.hooks')
echo "$EXISTING" | jq --argjson hooks "$HOOKS" '.hooks = ((.hooks // {}) * $hooks)' > ~/.gemini/settings.json

# Copy commands
cp plugins/memory-gemini-adapter/.gemini/commands/*.toml ~/.gemini/commands/

# Copy skills (excluding install skill)
for skill in memory-query retrieval-policy topic-graph bm25-search vector-search; do
  cp -r "plugins/memory-gemini-adapter/.gemini/skills/$skill" ~/.gemini/skills/
done
```

### Manual: Per-Project Installation

Copy the `.gemini/` directory into your project root. Project-level settings take precedence over global settings.

```bash
cp -r plugins/memory-gemini-adapter/.gemini .gemini
```

Note: For per-project installs, the hook handler path in `settings.json` should reference the project-relative path. Edit the command paths from `$HOME/.gemini/hooks/memory-capture.sh` to `.gemini/hooks/memory-capture.sh` (or use `$GEMINI_PROJECT_DIR/.gemini/hooks/memory-capture.sh`).

## Commands

| Command | Description | Example |
|---------|-------------|---------|
| `/memory-search <topic>` | Search conversations by topic or keyword | `/memory-search authentication` |
| `/memory-recent` | Show recent conversation summaries | `/memory-recent --days 3` |
| `/memory-context <grip-id>` | Expand an excerpt to see full context | `/memory-context grip:170654...` |

### /memory-search

Search past conversations by topic or keyword with tier-aware retrieval.

```
/memory-search <topic> [--period <value>] [--agent <value>]
```

**Examples:**

```
/memory-search authentication
/memory-search "JWT tokens" --period "last week"
/memory-search "database migration" --agent gemini
```

### /memory-recent

Display recent conversation summaries.

```
/memory-recent [--days N] [--limit N] [--agent <value>]
```

**Examples:**

```
/memory-recent
/memory-recent --days 3
/memory-recent --days 14 --limit 20
```

### /memory-context

Expand a grip ID to see full conversation context around an excerpt.

```
/memory-context <grip-id> [--before N] [--after N]
```

**Examples:**

```
/memory-context grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
/memory-context grip:1706540400000:01HN4QXKN6 --before 10 --after 10
```

## Skills

| Skill | Purpose | When Used |
|-------|---------|-----------|
| `memory-query` | Core query capability with tier awareness and embedded Navigator logic | All memory retrieval operations |
| `retrieval-policy` | Tier detection, intent classification, fallbacks | Query routing and capability detection |
| `topic-graph` | Topic exploration and discovery | Tier 1 (Full) -- when topic index is available |
| `bm25-search` | Keyword search via BM25 index | Tier 1-4 -- when BM25 index is available |
| `vector-search` | Semantic similarity search | Tier 1-3 -- when vector index is available |
| `memory-gemini-install` | Automated installation and setup | Initial setup only |

The `memory-query` skill includes embedded Navigator Mode with intent classification, parallel invocation strategy, tier-aware layer routing, and explainability output. This provides the same retrieval intelligence as the Claude Code navigator agent.

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

The hook handler script (`memory-capture.sh`) is registered in `settings.json` for 6 Gemini lifecycle events. When Gemini CLI fires a hook, it sends JSON via stdin to the script. The script extracts relevant fields, transforms them into the `memory-ingest` format, and pipes them to the `memory-ingest` binary in the background.

All events are automatically tagged with `agent:gemini` for cross-agent query support.

### Event Mapping

| Gemini Event | Agent Memory Event | Mapping Quality | Content Captured |
|-------------|-------------------|-----------------|------------------|
| SessionStart | SessionStart | Exact | Session ID, working directory |
| SessionEnd | Stop | Exact | Session boundary marker |
| BeforeAgent | UserPromptSubmit | Good | User prompt text |
| AfterAgent | AssistantResponse | Good | Assistant response text |
| BeforeTool | PreToolUse | Exact | Tool name, tool input (redacted) |
| AfterTool | PostToolUse | Exact | Tool name, tool input (redacted) |

### Gap: SubagentStart / SubagentStop

Gemini CLI does not provide subagent lifecycle hooks. This is a **trivial gap** -- subagent events are low-priority metadata, not core conversation content. All essential conversation events (prompts, responses, tool usage, session boundaries) are fully captured.

### Behavior

- **Fail-open:** The hook handler never blocks Gemini CLI. If `memory-ingest` is unavailable or the daemon is down, events are silently dropped. The script always outputs `{}` and exits 0.
- **Backgrounded:** The `memory-ingest` call runs in the background to minimize hook latency.
- **Agent tagging:** All events include `agent: "gemini"` for cross-agent filtering.
- **Sensitive field redaction:** Fields matching `api_key`, `token`, `secret`, `password`, `credential`, `authorization` (case-insensitive) are automatically stripped from `tool_input` and JSON-formatted message payloads.
- **ANSI stripping:** The hook handler strips ANSI escape sequences from input to handle colored terminal output.

### Verifying Capture

After a Gemini CLI session, verify events were captured:

```bash
# Check recent events
memory-daemon query events --from $(date -v-1H +%s000) --to $(date +%s000) --limit 5

# Search with agent filter
memory-daemon retrieval route "your query" --agent gemini

# Check TOC for recent data
memory-daemon query root
```

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEMORY_INGEST_PATH` | `memory-ingest` | Override path to memory-ingest binary |
| `MEMORY_INGEST_DRY_RUN` | `0` | Set to `1` to skip actual ingest (testing) |

## Cross-Agent Queries

One of the key benefits of agent-memory is searching across all agent sessions. After installing the Gemini adapter alongside the Claude Code hooks or OpenCode plugin, you can query conversations from any agent:

```bash
# Search across ALL agents (Claude Code, OpenCode, Gemini)
memory-daemon retrieval route "your query"

# Search Gemini sessions only
memory-daemon retrieval route "your query" --agent gemini

# Search Claude Code sessions only
memory-daemon retrieval route "your query" --agent claude

# Search OpenCode sessions only
memory-daemon retrieval route "your query" --agent opencode
```

## Architecture

```
plugins/memory-gemini-adapter/
├── .gemini/
│   ├── settings.json                              # Hook configuration template
│   ├── hooks/
│   │   └── memory-capture.sh                      # Hook handler (fail-open, backgrounded)
│   ├── commands/
│   │   ├── memory-search.toml                     # /memory-search slash command
│   │   ├── memory-recent.toml                     # /memory-recent slash command
│   │   └── memory-context.toml                    # /memory-context slash command
│   └── skills/
│       ├── memory-query/                          # Core query + Navigator logic
│       │   ├── SKILL.md
│       │   └── references/command-reference.md
│       ├── retrieval-policy/                      # Tier detection + intent routing
│       │   ├── SKILL.md
│       │   └── references/command-reference.md
│       ├── topic-graph/                           # Topic exploration
│       │   ├── SKILL.md
│       │   └── references/command-reference.md
│       ├── bm25-search/                           # BM25 keyword search
│       │   ├── SKILL.md
│       │   └── references/command-reference.md
│       ├── vector-search/                         # Semantic similarity search
│       │   ├── SKILL.md
│       │   └── references/command-reference.md
│       └── memory-gemini-install/                 # Install skill (setup only)
│           └── SKILL.md
├── README.md
└── .gitignore
```

## Settings.json Precedence

Gemini CLI loads configuration in this order (highest precedence first):

1. **`GEMINI_CONFIG` environment variable** -- Overrides the default config path entirely
2. **`--config` CLI flag** -- Specifies a custom config file for the current session
3. **Project `.gemini/settings.json`** -- Per-project configuration (in the project root)
4. **User `~/.gemini/settings.json`** -- Global user configuration
5. **System `/etc/gemini-cli/settings.json`** -- System-wide defaults

**When to use global vs project-level:**

- **Global (`~/.gemini/settings.json`):** Recommended for most users. Captures events from all Gemini sessions automatically. Use the install skill for automated global setup.
- **Project-level (`.gemini/settings.json`):** Use when you want memory capture only for specific projects, or when different projects need different hook configurations.

**Important:** If you have both global and project-level `settings.json` with hooks, the project-level hooks take full precedence for that project (they do NOT merge). Ensure your project-level settings include the memory-capture hooks if you want capture in that project.

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
- Try `/memory-recent` to see what data is available

### Hooks not firing

**Symptom:** Gemini sessions run but no events appear in agent-memory.

**Check settings.json structure:**

```bash
# Verify settings.json exists and has hooks
jq '.hooks | keys' ~/.gemini/settings.json

# Expected: ["AfterAgent","AfterTool","BeforeAgent","BeforeTool","SessionEnd","SessionStart"]
```

**Check Gemini CLI version:**

```bash
gemini --version
```

Ensure you have a version that supports the hooks system. If your version is too old, update:

```bash
npm update -g @google/gemini-cli
```

**Check hook script is executable:**

```bash
ls -la ~/.gemini/hooks/memory-capture.sh
# Should show -rwxr-xr-x
```

### Slow responses

**Symptom:** Gemini CLI feels slow after installing hooks.

**Cause:** Hook handler is not backgrounding the memory-ingest call properly.

**Solution:** Verify the hook script contains the backgrounded call:

```bash
grep '&$' ~/.gemini/hooks/memory-capture.sh
# Should show a line ending with & (backgrounded)
```

The hook handler should complete in under 50ms. If latency persists, check if jq is slow on your system.

### stdout pollution

**Symptom:** Gemini shows "hook parse error" or garbled output.

**Cause:** Something is printing to stdout besides the expected `{}` JSON.

**Solution:** The hook handler redirects all memory-ingest output to `/dev/null`. If you have modified the script, ensure no `echo` or `printf` statements write to stdout (use stderr for debugging).

### jq not installed

**Symptom:** Hook handler silently drops all events.

**Solution:**

```bash
# macOS
brew install jq

# Debian/Ubuntu
sudo apt install jq

# Fedora
sudo dnf install jq

# Verify
jq --version
```

### ANSI/color codes in output

**Symptom:** Events contain garbled escape sequences.

**Cause:** Gemini CLI may include ANSI color codes in hook input.

**Solution:** The hook handler strips ANSI escape sequences automatically using sed. If you see garbled data, verify you are using the latest version of `memory-capture.sh`.

### Gemini CLI version too old

**Symptom:** settings.json hooks have no effect.

**Solution:** Ensure your Gemini CLI version supports the hooks system. Update to the latest version:

```bash
npm update -g @google/gemini-cli
```

## Related

- [agent-memory](https://github.com/SpillwaveSolutions/agent-memory) -- The memory daemon and storage system
- [memory-query-plugin](../memory-query-plugin/) -- Claude Code query commands and skills
- [memory-opencode-plugin](../memory-opencode-plugin/) -- OpenCode query commands, skills, and event capture
- [memory-setup-plugin](../memory-setup-plugin/) -- Claude Code installation wizard

## Version History

- **v2.1.0**: Initial release -- hook-based event capture, TOML commands, 5 query skills with Navigator, install skill, cross-agent query support

## License

MIT
