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

## Reference Files

For detailed information, see:

- [Installation Methods](references/installation-methods.md) - Cargo, binaries, building from source
- [Configuration Options](references/configuration-options.md) - All config.toml options
- [Troubleshooting Guide](references/troubleshooting-guide.md) - Common issues and solutions
- [Platform Specifics](references/platform-specifics.md) - macOS, Linux, Windows details
