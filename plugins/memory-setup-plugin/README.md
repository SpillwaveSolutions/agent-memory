# Memory Setup Plugin

A Claude Code plugin for installing, configuring, and troubleshooting agent-memory.

## Overview

This plugin enables Claude to help you set up and manage your agent-memory installation through interactive commands and autonomous troubleshooting. It handles everything from first-time installation to ongoing configuration management.

## Installation

```bash
# Clone to your Claude skills directory
cd ~/.claude/skills
git clone https://github.com/SpillwaveSolutions/memory-setup-agentic-plugin.git
```

Or install from the agent-memory monorepo:

```bash
ln -s /path/to/agent-memory/plugins/memory-setup-plugin ~/.claude/skills/memory-setup-plugin
```

## Commands

| Command | Description |
|---------|-------------|
| `/memory-setup` | Interactive installation wizard |
| `/memory-status` | Health check and diagnostics |
| `/memory-config` | View and modify configuration |

### /memory-setup

Interactive wizard for installing agent-memory from scratch.

```
/memory-setup              # Standard installation
/memory-setup --fresh      # Fresh install (ignore existing)
/memory-setup --minimal    # Daemon only (no CCH hooks)
/memory-setup --advanced   # Show all configuration options
```

### /memory-status

Check health and status of your installation.

```
/memory-status             # Quick status
/memory-status --verbose   # Full diagnostics
/memory-status --json      # JSON output
```

### /memory-config

View and modify configuration without editing files.

```
/memory-config show                           # Show all config
/memory-config show summarizer                # Show section
/memory-config set summarizer.model gpt-4o    # Set value
/memory-config reset all                      # Reset to defaults
```

## Agent

The **setup-troubleshooter** agent handles complex issues autonomously:

- Diagnoses installation problems
- Fixes common configuration issues
- Guides through error resolution

Triggered by patterns like:
- "memory-daemon won't start"
- "no events in memory"
- "connection refused"
- "can't install memory-daemon"

## Quick Start

1. **Install the plugin** (see Installation above)

2. **Run setup:**
   ```
   /memory-setup
   ```

3. **Set your API key:**
   ```bash
   export OPENAI_API_KEY="your-key-here"
   ```

4. **Verify installation:**
   ```
   /memory-status
   ```

5. **Start using memory:**
   ```
   /memory-search authentication  # Search past conversations
   /memory-recent                 # Recent summaries
   ```

## Skills

Use these dedicated skills for focused setup tasks:

| Skill | When to Use | Notes |
|-------|-------------|-------|
| `memory-install` | Install binaries and set PATH | Wizard-style, confirms before edits, provides verify commands only |
| `memory-configure` | Create or update single-agent config | Shows defaults + full sample config, dry-run check step |
| `memory-verify` | Validate install/config/daemon/ingest | Commands only, no auto-run |
| `memory-troubleshoot` | Diagnose common setup failures | Safe fixes with confirmation before changes |

Agent-specific setup (Claude Code, OpenCode, Gemini CLI, Copilot CLI) lives in
separate guides and is intentionally outside the core install flow.

## Architecture

```
memory-setup-plugin/
├── .claude-plugin/
│   └── marketplace.json     # Plugin manifest
├── skills/
│   └── memory-setup/        # Core skill
│       ├── SKILL.md
│       └── references/
│           ├── installation-methods.md
│           ├── configuration-options.md
│           ├── troubleshooting-guide.md
│           └── platform-specifics.md
├── commands/                # Slash commands
│   ├── memory-setup.md
│   ├── memory-status.md
│   └── memory-config.md
├── agents/                  # Autonomous agents
│   └── setup-troubleshooter.md
└── README.md
```

## Prerequisites

- **Rust toolchain** - For building from source
- **Claude Code** - The plugin host environment
- **LLM API key** - OpenAI or Anthropic for summarization

## Configuration

Configuration is stored in `~/.config/memory-daemon/config.toml`:

```toml
[storage]
path = "~/.memory-store"

[server]
host = "[::1]"
port = 50051

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

See [Configuration Options](skills/memory-setup/references/configuration-options.md) for full reference.

## Troubleshooting

Common issues and quick fixes:

| Issue | Solution |
|-------|----------|
| Daemon not running | `memory-daemon start` |
| Command not found | Add `~/.cargo/bin` to PATH |
| Connection refused | Check port 50051 availability |
| No events captured | Configure CCH hooks |

See [Troubleshooting Guide](skills/memory-setup/references/troubleshooting-guide.md) for detailed solutions.

## Related

- [agent-memory](https://github.com/SpillwaveSolutions/agent-memory) - The memory daemon and storage system
- [memory-query-plugin](../memory-query-plugin/) - Query past conversations
- [code_agent_context_hooks](https://github.com/SpillwaveSolutions/code_agent_context_hooks) - Event capture integration

## License

MIT
