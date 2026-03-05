# Memory Adapter for Codex CLI

A skills-only adapter for [Codex CLI](https://github.com/openai/codex) that enables intelligent memory retrieval, integrating Codex CLI sessions into the agent-memory ecosystem.

**Version:** 2.1.0

## Overview

This adapter brings agent-memory query capabilities to Codex CLI through skills. Unlike other adapters (Claude Code, Gemini CLI, Copilot CLI), Codex CLI does **not support hooks** ([GitHub Discussion #2150](https://github.com/openai/codex/discussions/2150)). This means:

- **Skills/commands**: Fully supported -- query, search, explore conversation history
- **Automatic event capture**: NOT supported -- no hook handler, no automatic session recording
- **Event ingestion**: Possible via direct `memory-ingest` CLI with CchEvent JSON format

For automatic event capture, use one of the other adapters (Claude Code, Gemini CLI, or Copilot CLI) which support hooks.

## Quickstart

```bash
# Copy skills to your project
cp -r adapters/codex-cli/.codex .codex

# Verify skills are loaded (inside Codex)
codex exec --full-auto "ls .codex/skills/"

# Query memory (requires running memory-daemon)
memory-daemon retrieval route "your query" --agent codex
```

## Installation

### Per-Project (Recommended)

Copy the `.codex/skills/` directory to your project root:

```bash
# From the agent-memory repository root
cp -r adapters/codex-cli/.codex .codex
```

### Alternative Path

Codex CLI also supports skills under `.agents/skills/`:

```bash
mkdir -p .agents/skills
cp -r adapters/codex-cli/.codex/skills/* .agents/skills/
```

Both `.codex/skills/` and `.agents/skills/` paths are recognized by Codex CLI.

## Skills

| Skill | Purpose | When Auto-Activated |
|-------|---------|---------------------|
| `memory-query` | Core query capability with tier awareness | "recall", "search conversations", "find previous session" |
| `retrieval-policy` | Tier detection, intent classification, fallbacks | "which search method", "available capabilities" |
| `topic-graph` | Topic exploration and discovery | "what topics", "explore subjects", "topic map" |
| `bm25-search` | Keyword search via BM25 index | "keyword search", "exact match", "find term" |
| `vector-search` | Semantic similarity search | "semantic search", "similar concepts", "find related" |

Skills auto-activate when the user's prompt matches the skill's description. Each SKILL.md uses YAML frontmatter with `name` and `description` fields (Codex format).

**Note:** There is no install skill because Codex has no hooks to install.

## Why No Hooks?

Codex CLI does not support lifecycle hooks as of the current release. This is a known limitation discussed in [GitHub Discussion #2150](https://github.com/openai/codex/discussions/2150). Without hooks:

- Session events (start, end, prompts, tool usage) cannot be automatically captured
- The adapter is limited to query-only functionality
- Events can still be manually ingested using the `memory-ingest` binary with CchEvent JSON format

If/when Codex CLI adds hook support, this adapter will be updated to include a hook handler similar to the Copilot and Gemini adapters.

## Cross-Agent Queries

Query conversations from any agent using the memory-daemon CLI:

```bash
# Search across ALL agents
memory-daemon retrieval route "your query"

# Search Codex-ingested sessions only
memory-daemon retrieval route "your query" --agent codex

# Search Claude Code sessions
memory-daemon retrieval route "your query" --agent claude

# Search Gemini sessions
memory-daemon retrieval route "your query" --agent gemini

# Search Copilot sessions
memory-daemon retrieval route "your query" --agent copilot
```

## Manual Event Ingestion

While Codex lacks hooks for automatic capture, you can manually ingest events:

```bash
# Pipe CchEvent JSON to memory-ingest
echo '{"hook_event_name":"SessionStart","session_id":"codex-001","timestamp":"2026-03-05T10:00:00Z","cwd":"/my/project","agent":"codex"}' | memory-ingest

# Ingest a user prompt
echo '{"hook_event_name":"UserPromptSubmit","session_id":"codex-001","message":"Explain the project","timestamp":"2026-03-05T10:01:00Z","agent":"codex"}' | memory-ingest
```

## Sandbox Configuration

Codex CLI runs commands in a sandbox that may block network access needed by memory-daemon. See [SANDBOX-WORKAROUND.md](SANDBOX-WORKAROUND.md) for platform-specific solutions.

**Quick fix for macOS:**
```bash
codex exec --sandbox danger-full-access "memory-daemon status"
```

## Prerequisites

| Component | Required | Purpose |
|-----------|----------|---------|
| memory-daemon | Yes | Stores and indexes conversation events |
| memory-ingest | Yes | Receives events via stdin pipe (manual ingestion) |
| Codex CLI | Yes | The CLI tool being integrated |

```bash
memory-daemon status  # Check daemon
memory-daemon start   # Start if needed
```

## Architecture

```
adapters/codex-cli/
+-- .codex/
|   +-- skills/
|       +-- memory-query/                  # Core query + command instructions
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- retrieval-policy/              # Tier detection + intent routing
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- topic-graph/                   # Topic exploration
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- bm25-search/                   # BM25 keyword search
|       |   +-- SKILL.md
|       |   +-- references/command-reference.md
|       +-- vector-search/                 # Semantic similarity search
|           +-- SKILL.md
|           +-- references/command-reference.md
+-- SANDBOX-WORKAROUND.md                  # macOS sandbox workaround
+-- README.md
+-- .gitignore
```

## Codex CLI vs Other Adapters

| Aspect | Codex CLI | Copilot CLI | Gemini CLI | Claude Code |
|--------|-----------|-------------|-----------|-------------|
| Hook support | None | `.github/hooks/` | `settings.json` | `.claude/hooks.yaml` |
| Skills | `.codex/skills/` | `.github/skills/` | `.gemini/skills/` | `.claude/skills/` |
| Auto capture | No | Yes (hook script) | Yes (hook script) | Yes (hook handler) |
| Commands | Skills only | Skills only | TOML + skills | Commands + skills |
| Sandbox | Seatbelt/Landlock | None | None | None |
| Location | `adapters/` | `plugins/` | `plugins/` | `plugins/` |

## Troubleshooting

### Daemon not running

```bash
memory-daemon start
memory-daemon status   # Verify "running"
```

### Skills not loading

Verify the `.codex/skills/` directory exists in your project root:

```bash
ls -la .codex/skills/
# Should show: memory-query, retrieval-policy, topic-graph, bm25-search, vector-search
```

### Network blocked by sandbox

See [SANDBOX-WORKAROUND.md](SANDBOX-WORKAROUND.md) for solutions.

### No results found

- Verify data exists: `memory-daemon query root` should show year nodes
- Codex has no automatic capture -- events must be ingested manually or via another adapter
- Broaden search terms or try a different time period

## Related

- [agent-memory](https://github.com/SpillwaveSolutions/agent-memory) -- The memory daemon and storage system
- [memory-copilot-adapter](../../plugins/memory-copilot-adapter/) -- Copilot CLI adapter with hook-based capture
- [memory-gemini-adapter](../../plugins/memory-gemini-adapter/) -- Gemini CLI adapter with hook-based capture
- [memory-query-plugin](../../plugins/memory-query-plugin/) -- Claude Code query commands and skills
- [memory-opencode-plugin](../../plugins/memory-opencode-plugin/) -- OpenCode query and capture plugin

## License

MIT
