# Phase 9 Research: Setup & Installer Plugin

## Objective

Create an interactive setup wizard plugin that guides users through installing, configuring, and managing agent-memory. The plugin should make first-time setup effortless while providing power-user options for advanced configuration.

## User Journey Analysis

### Target Users

1. **First-time users** — Never used agent-memory, need guided installation
2. **Migrating users** — Moving from manual setup to plugin-managed
3. **Power users** — Want fine-grained control over configuration
4. **Troubleshooters** — Something broke, need diagnostics

### Key Questions Users Have

1. "How do I install agent-memory?"
2. "How do I configure it to capture my Claude Code sessions?"
3. "Is it working? How do I check?"
4. "How do I change settings after initial setup?"
5. "Something's not working — how do I fix it?"

## Installation Paths

### Binary Installation Options

| Method | Pros | Cons |
|--------|------|------|
| Cargo install | Works everywhere Rust is installed | Requires Rust toolchain |
| Pre-built binaries | Fast, no dependencies | Need to host releases |
| Homebrew (macOS) | Familiar to Mac users | Mac-only, need tap |
| Download script | One-liner install | Security concerns |

**Recommended approach**: Support multiple methods, detect what's available:
1. Check for existing installation
2. Check for cargo
3. Offer download from GitHub releases
4. Provide manual instructions as fallback

### Installation Targets

| Binary | Purpose | Install Location |
|--------|---------|------------------|
| memory-daemon | gRPC service | ~/.local/bin/ or ~/.cargo/bin/ |
| memory-ingest | CCH hook handler | ~/.local/bin/ or ~/.cargo/bin/ |
| memory-query (optional) | CLI tool | ~/.local/bin/ or ~/.cargo/bin/ |

## Configuration Components

### 1. Daemon Configuration

File: `~/.config/agent-memory/config.toml`

```toml
[daemon]
port = 50051
db_path = "~/.local/share/agent-memory/db"

[summarizer]
provider = "anthropic"  # or "openai", "local"
model = "claude-3-haiku-20240307"
# api_key loaded from ANTHROPIC_API_KEY env var

[segmentation]
time_threshold_mins = 30
token_threshold = 4000
overlap_mins = 5
overlap_tokens = 500
```

### 2. CCH Hook Configuration

File: `~/.claude/hooks.yaml` (or project-level)

```yaml
hooks:
  - event: "*"
    run: "~/.local/bin/memory-ingest"
```

### 3. Environment Variables

| Variable | Purpose | Required |
|----------|---------|----------|
| ANTHROPIC_API_KEY | Summarization API | Yes (if using Anthropic) |
| OPENAI_API_KEY | Summarization API | Yes (if using OpenAI) |
| MEMORY_DAEMON_PORT | Override default port | No |
| MEMORY_DB_PATH | Override database location | No |

## Interactive Wizard Flow

```
/memory-setup

┌─────────────────────────────────────────────────────┐
│  Agent Memory Setup Wizard                          │
└─────────────────────────────────────────────────────┘

Step 1: Check Prerequisites
  ✓ Claude Code detected
  ✗ memory-daemon not found
  ✗ memory-ingest not found

Step 2: Installation
  Q: How would you like to install?
  > [1] cargo install (recommended)
  > [2] Download pre-built binaries
  > [3] Manual installation (show instructions)

Step 3: Summarizer Configuration
  Q: Which AI provider for summarization?
  > [1] Anthropic Claude (recommended)
  > [2] OpenAI GPT
  > [3] Local model (ollama)
  > [4] Skip summarization (store events only)

  Q: Enter your API key (or press Enter to use env var):
  > [uses ANTHROPIC_API_KEY]

Step 4: Hook Configuration
  Q: Where should hooks be configured?
  > [1] Global (~/.claude/hooks.yaml)
  > [2] Current project only (.claude/hooks.yaml)
  > [3] Skip (I'll configure manually)

Step 5: Daemon Startup
  Q: How should the daemon start?
  > [1] Start now and on login (launchd/systemd)
  > [2] Start now only
  > [3] Manual start (show command)

Step 6: Verification
  ✓ Daemon started on port 50051
  ✓ Hook configuration written
  ✓ Test event ingested successfully

  Setup complete! Try: /memory-recent
```

## Plugin Structure

```
plugins/memory-setup-plugin/
├── .claude-plugin/
│   └── marketplace.json
├── skills/
│   └── memory-setup/
│       ├── SKILL.md
│       └── references/
│           ├── installation-methods.md
│           ├── configuration-options.md
│           ├── troubleshooting-guide.md
│           └── platform-specifics.md
├── commands/
│   ├── memory-setup.md      # Interactive wizard
│   ├── memory-status.md     # Health check
│   └── memory-config.md     # Modify settings
└── agents/
    └── setup-troubleshooter.md  # Complex diagnostics
```

## Commands Specification

### /memory-setup

**Purpose**: Interactive setup wizard for first-time installation

**Behavior**:
1. Detect current state (what's installed, what's configured)
2. Ask questions progressively based on context
3. Execute installation steps
4. Verify setup works
5. Provide next steps

**Flags**:
- `--fresh` — Start from scratch, ignore existing config
- `--minimal` — Quick setup with defaults, minimal questions
- `--advanced` — Show all configuration options

### /memory-status

**Purpose**: Check installation health and show current configuration

**Output**:
```
Agent Memory Status
───────────────────
Daemon:     ✓ Running (port 50051, PID 12345)
Database:   ✓ 1.2 GB, 15,432 events
Hooks:      ✓ Global hooks.yaml configured
API Key:    ✓ ANTHROPIC_API_KEY set

Recent Activity:
  Last event: 2 minutes ago
  Today: 47 events ingested
  This week: 312 events
```

**Flags**:
- `--json` — Output as JSON for scripting
- `--verbose` — Show full configuration details

### /memory-config

**Purpose**: View or modify configuration after initial setup

**Subcommands**:
- `/memory-config show` — Display current config
- `/memory-config set <key> <value>` — Update setting
- `/memory-config reset` — Reset to defaults

**Examples**:
```
/memory-config show
/memory-config set summarizer.provider openai
/memory-config set daemon.port 50052
```

## Agent: setup-troubleshooter

**Purpose**: Diagnose and fix setup issues autonomously

**Triggers**:
- User reports memory not working
- /memory-status shows failures
- Explicit `/memory-troubleshoot` invocation

**Capabilities**:
1. Check daemon process status
2. Verify port availability
3. Test gRPC connectivity
4. Validate configuration files
5. Check API key validity
6. Verify hook configuration
7. Inspect recent logs
8. Suggest and apply fixes

**Example Flow**:
```
User: "My conversations aren't being saved"

Agent:
1. Check if daemon is running → Not running
2. Attempt to start daemon → Port 50051 in use
3. Find process using port → Another memory-daemon
4. Offer to kill and restart → User approves
5. Start daemon → Success
6. Verify event ingestion → Working
7. Report resolution
```

## Platform Considerations

### macOS

- Install location: ~/.local/bin/ (add to PATH in ~/.zshrc)
- Auto-start: launchd plist in ~/Library/LaunchAgents/
- Config: ~/.config/agent-memory/

### Linux

- Install location: ~/.local/bin/
- Auto-start: systemd user service in ~/.config/systemd/user/
- Config: ~/.config/agent-memory/

### Windows

- Install location: %USERPROFILE%\.local\bin\
- Auto-start: Task Scheduler or startup folder
- Config: %APPDATA%\agent-memory\

## Security Considerations

1. **API Keys**: Never store in config files; use environment variables
2. **Binary Downloads**: Verify checksums from GitHub releases
3. **Hook Permissions**: Warn about hook execution permissions
4. **Database Location**: Default to user-private directory

## Integration with Existing Plugins

The setup plugin should detect and integrate with:
- **memory-query plugin**: Suggest installation after setup complete
- **CCH hooks**: Check for existing hooks.yaml before writing

## Success Metrics

1. User can go from zero to working memory in < 5 minutes
2. All common issues have automated diagnostics
3. Power users can access all options without wizard
4. No manual file editing required for basic setup

## Research Conclusions

1. **Plugin structure**: Full marketplace plugin with 3 commands + 1 agent
2. **Installation**: Support cargo install + pre-built binaries
3. **Wizard**: Progressive disclosure, detect context, minimal questions
4. **Platform**: macOS first, with Linux/Windows patterns documented
5. **Troubleshooting**: Autonomous agent with diagnostic capabilities
