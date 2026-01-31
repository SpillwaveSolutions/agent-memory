---
name: memory-setup
description: |
  Setup, configure, and troubleshoot agent-memory installation. Use when asked to "install agent-memory", "setup memory", "check memory status", "configure memory daemon", "fix memory issues", or "troubleshoot memory". Provides interactive setup wizard, health checks, and configuration management.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Setup Skill

Setup, configure, and troubleshoot agent-memory installation with guided workflows and autonomous diagnostics.

## When Not to Use

- Querying past conversations (use memory-query plugin)
- Real-time event ingestion (handled by CCH hooks automatically)
- Low-level storage operations (use memory-daemon admin commands directly)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `/memory-setup` | Interactive installation wizard | `/memory-setup` |
| `/memory-status` | Health check and diagnostics | `/memory-status --verbose` |
| `/memory-config` | View/modify configuration | `/memory-config show` |

## Installation Decision Tree

```
Is memory-daemon installed?
├── NO → /memory-setup
│   ├── --fresh: Clean installation
│   ├── --minimal: Just the daemon
│   └── --advanced: Custom paths/options
│
└── YES → Is it running?
    ├── NO → memory-daemon start
    │   └── Still failing? → /memory-status --verbose
    │
    └── YES → Is CCH hook configured?
        ├── NO → /memory-setup (detects existing install)
        └── YES → All good! Use /memory-search
```

## Workflow Overview

### 1. Initial Setup (`/memory-setup`)

```bash
# Check if Rust toolchain exists
rustc --version

# Install memory-daemon
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon

# Verify installation
memory-daemon --version
```

### 2. Health Check (`/memory-status`)

```bash
# Quick status
memory-daemon status

# Verbose diagnostics
memory-daemon status --verbose

# Check CCH hook
cat ~/.claude/code_agent_context_hooks/hooks.yaml
```

### 3. Configuration (`/memory-config`)

```bash
# Show current config
cat ~/.config/memory-daemon/config.toml

# Set a value (example)
# Edit ~/.config/memory-daemon/config.toml
```

### 4. Troubleshooting (Agent: setup-troubleshooter)

Automatic activation for:
- "memory-daemon won't start"
- "no data in memory"
- "connection refused"
- "events not being captured"

## Validation Checklist

Before confirming setup complete:
- [ ] `memory-daemon --version` returns version string
- [ ] `memory-daemon status` shows "running"
- [ ] `~/.config/memory-daemon/config.toml` exists
- [ ] `~/.memory-store/` directory exists (or custom path)
- [ ] CCH hooks.yaml includes memory-ingest handler (if using CCH)

## Platform Paths

| Platform | Config | Data | Logs |
|----------|--------|------|------|
| macOS | `~/.config/memory-daemon/` | `~/.memory-store/` | `~/Library/Logs/memory-daemon/` |
| Linux | `~/.config/memory-daemon/` | `~/.local/share/memory-daemon/` | `~/.local/state/memory-daemon/` |
| Windows | `%APPDATA%\memory-daemon\` | `%LOCALAPPDATA%\memory-daemon\` | `%LOCALAPPDATA%\memory-daemon\logs\` |

## Error Handling

| Error | Resolution |
|-------|------------|
| "command not found" | Cargo bin not in PATH, or not installed |
| "connection refused" | Daemon not running: `memory-daemon start` |
| "permission denied" | Check data directory permissions |
| "address in use" | Another process on port 50051 |

## State Detection

Before beginning setup, detect current system state to skip completed steps and offer appropriate options.

### 1. Prerequisites Check

```bash
# Claude Code detection (presence of .claude directory)
ls ~/.claude 2>/dev/null && echo "CLAUDE_CODE_DETECTED" || echo "CLAUDE_CODE_NOT_FOUND"

# Rust/Cargo availability
cargo --version 2>/dev/null && echo "CARGO_AVAILABLE" || echo "CARGO_NOT_AVAILABLE"

# Platform detection
uname -s  # Returns: Darwin, Linux, or MINGW*/MSYS* for Windows
uname -m  # Returns: arm64, x86_64, etc.
```

**State Categories:**
- `READY`: All prerequisites met, can proceed with cargo install
- `NEEDS_RUST`: cargo not found, offer rustup installation
- `MANUAL_ONLY`: No cargo, must use pre-built binaries

### 2. Existing Installation Check

```bash
# Binary locations
which memory-daemon 2>/dev/null || echo "NOT_INSTALLED"
which memory-ingest 2>/dev/null || echo "NOT_INSTALLED"

# Version check (if installed)
memory-daemon --version 2>/dev/null || echo "VERSION_UNKNOWN"
memory-ingest --version 2>/dev/null || echo "VERSION_UNKNOWN"

# Check common installation paths
ls ~/.cargo/bin/memory-daemon 2>/dev/null
ls ~/.local/bin/memory-daemon 2>/dev/null
ls /usr/local/bin/memory-daemon 2>/dev/null
```

**State Categories:**
- `NOT_INSTALLED`: No binaries found
- `INSTALLED`: Binaries found, capture version
- `OUTDATED`: Installed version < latest release
- `PARTIAL`: Only some binaries installed

### 3. Configuration State Check

```bash
# Config file existence
ls ~/.config/memory-daemon/config.toml 2>/dev/null && echo "CONFIG_EXISTS" || echo "NO_CONFIG"

# CCH hooks check (global)
grep -l "memory-ingest" ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null && echo "HOOK_GLOBAL" || echo "NO_GLOBAL_HOOK"

# CCH hooks check (project - current directory)
grep -l "memory-ingest" .claude/code_agent_context_hooks/hooks.yaml 2>/dev/null && echo "HOOK_PROJECT" || echo "NO_PROJECT_HOOK"

# Environment variables
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI_KEY_SET" || echo "OPENAI_KEY_MISSING"
[ -n "$ANTHROPIC_API_KEY" ] && echo "ANTHROPIC_KEY_SET" || echo "ANTHROPIC_KEY_MISSING"
```

**State Categories:**
- `UNCONFIGURED`: No config.toml
- `CONFIGURED`: config.toml exists
- `HOOKED_GLOBAL`: CCH hook configured globally
- `HOOKED_PROJECT`: CCH hook configured for project
- `API_READY`: At least one API key available

### 4. Runtime State Check

```bash
# Daemon process running
memory-daemon status 2>/dev/null
# Returns: running/stopped/error

# Port availability (if daemon not running)
lsof -i :50051 2>/dev/null && echo "PORT_IN_USE" || echo "PORT_AVAILABLE"

# Recent activity check
memory-daemon query root 2>/dev/null && echo "DAEMON_RESPONSIVE" || echo "DAEMON_UNRESPONSIVE"

# Event count (if daemon running)
memory-daemon admin status 2>/dev/null | grep -o 'events: [0-9]*'
```

**State Categories:**
- `RUNNING`: Daemon active and responsive
- `STOPPED`: Daemon not running, port available
- `PORT_BLOCKED`: Port in use by another process
- `UNRESPONSIVE`: Process exists but not responding

### State Summary Format

After detection, present state summary:

```
Current State
─────────────
Prerequisites:  ✓ Claude Code, ✓ Cargo (1.75.0), macOS arm64
Installation:   ✓ memory-daemon (1.0.0), ✓ memory-ingest (1.0.0)
Configuration:  ✓ config.toml, ✗ CCH hook not configured
Runtime:        ✗ Daemon not running (port available)

Recommended:    Configure CCH hook, then start daemon
```

## Output Formatting

Use consistent visual formatting throughout the wizard for clear user communication.

### Progress Display

Show wizard progress with step indicators:

```
Agent Memory Setup
==================

Checking prerequisites...
  [check] Claude Code detected
  [check] cargo available (1.75.0)
  [x] memory-daemon not found
  [x] memory-ingest not found

Step 1 of 6: Installation
-------------------------
```

### Step Headers

Each wizard step uses consistent header formatting:

```
Step N of 6: Step Name
----------------------
[Question or action content]
```

### Status Indicators

Use consistent symbols for status:

| Symbol | Meaning | When to Use |
|--------|---------|-------------|
| `[check]` | Success/Complete | Item verified or action succeeded |
| `[x]` | Missing/Failed | Item not found or action failed |
| `[!]` | Warning | Non-critical issue, can continue |
| `[?]` | Unknown | Could not determine status |
| `[>]` | In Progress | Currently executing |

### Success Display

Final success message format:

```
==================================================
 Setup Complete!
==================================================

[check] Binaries installed to ~/.cargo/bin/
[check] Configuration written to ~/.config/memory-daemon/
[check] Hooks configured in ~/.claude/code_agent_context_hooks/hooks.yaml
[check] Daemon started on port 50051

Next steps:
  * Start a conversation and it will be recorded
  * Use /memory-recent to see captured events
  * Use /memory-search <topic> to find past discussions
```

### Partial Success Display

When some steps succeed but others are skipped or fail:

```
==================================================
 Setup Partially Complete
==================================================

[check] Binaries installed to ~/.cargo/bin/
[check] Configuration written to ~/.config/memory-daemon/
[x] CCH hook not configured (manual setup required)
[check] Daemon started on port 50051

What's missing:
  * CCH integration not configured - events won't be captured automatically

To complete setup manually:
  1. Add to ~/.claude/code_agent_context_hooks/hooks.yaml:
     hooks:
       - event: all
         handler:
           type: pipe
           command: memory-ingest

  2. Verify with: /memory-status
```

### Error Display

When setup fails, provide actionable error information:

```
[x] Setup Failed
----------------

Error: Could not start daemon - port 50051 in use

To fix:
  1. Run: lsof -i :50051
  2. Kill the process using the port
  3. Run: /memory-setup --fresh

Need help? Run: /memory-status --verbose
```

### Question Format

When asking user for input:

```
Which LLM provider should generate summaries?

1. Anthropic (Claude) - Best quality summaries
2. OpenAI (GPT-4o-mini) - Fast and cost-effective
3. Local (Ollama) - Private, runs locally
4. None - Skip summarization

Default: 1 (Anthropic)

Enter selection [1-4]:
```

### Summary Tables

Use tables for displaying configuration and status summaries:

```
### Installation Summary

| Component | Status | Version | Path |
|-----------|--------|---------|------|
| memory-daemon | Installed | 1.0.0 | ~/.cargo/bin/ |
| memory-ingest | Installed | 1.0.0 | ~/.cargo/bin/ |

### Configuration

| Setting | Value |
|---------|-------|
| Storage Path | ~/.memory-store |
| Server | [::1]:50051 |
| LLM Provider | Anthropic (claude-3-5-haiku-latest) |
| CCH Hooks | Global |
```

### Progress Bar (Optional)

For long operations like cargo install:

```
Installing memory-daemon...
[################----] 80% - Compiling memory-daemon v1.0.0
```

### Color Guidelines (Terminal Support)

When terminal supports colors:

| Element | Color | ANSI Code |
|---------|-------|-----------|
| Success | Green | `\033[32m` |
| Error | Red | `\033[31m` |
| Warning | Yellow | `\033[33m` |
| Info | Blue | `\033[34m` |
| Headers | Bold | `\033[1m` |
| Reset | - | `\033[0m` |

**Note:** Always check terminal capability before using colors. Fall back to text indicators ([check], [x], etc.) when colors unavailable.

## Reference Files

For detailed information, see:

- [Installation Methods](references/installation-methods.md) - Cargo, binaries, building from source
- [Configuration Options](references/configuration-options.md) - All config.toml options
- [Troubleshooting Guide](references/troubleshooting-guide.md) - Common issues and solutions
- [Platform Specifics](references/platform-specifics.md) - macOS, Linux, Windows details
- [Wizard Questions](references/wizard-questions.md) - Complete interactive wizard question flow
