---
name: memory-setup
description: Interactive wizard to install and configure agent-memory
parameters:
  - name: fresh
    description: Ignore existing configuration and re-ask all questions
    required: false
    type: flag
  - name: minimal
    description: Use all defaults without asking (only prompt for required inputs)
    required: false
    type: flag
  - name: advanced
    description: Show all configuration options including ports and tuning
    required: false
    type: flag
skills:
  - memory-setup
---

# Memory Setup

Interactive wizard to install and configure agent-memory from scratch or upgrade existing installation.

## Usage

```
/memory-setup
/memory-setup --fresh
/memory-setup --minimal
/memory-setup --advanced
```

## Wizard Execution Flow

The wizard proceeds through four distinct phases: State Detection, Question, Execution, and Verification.

---

## Phase 1: State Detection

**Goal:** Understand what's already installed/configured to skip unnecessary steps.

### Detection Commands

Run all detection commands in parallel to gather system state:

```bash
# Prerequisites
cargo --version 2>/dev/null || echo "CARGO_NOT_AVAILABLE"
ls ~/.claude 2>/dev/null && echo "CLAUDE_CODE_DETECTED" || echo "CLAUDE_CODE_NOT_FOUND"
uname -s && uname -m

# Installation
which memory-daemon 2>/dev/null || echo "NOT_INSTALLED"
which memory-ingest 2>/dev/null || echo "NOT_INSTALLED"
memory-daemon --version 2>/dev/null || echo "VERSION_UNKNOWN"

# Configuration
ls ~/.config/memory-daemon/config.toml 2>/dev/null && echo "CONFIG_EXISTS" || echo "NO_CONFIG"
grep -l "memory-ingest" ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null

# Environment
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI_KEY_SET"
[ -n "$ANTHROPIC_API_KEY" ] && echo "ANTHROPIC_KEY_SET"

# Runtime
memory-daemon status 2>/dev/null || echo "NOT_RUNNING"
```

### State Summary Output

After detection, display current state before asking questions:

```
Agent Memory Setup
==================

Checking prerequisites...
  [check] Claude Code detected
  [check] cargo available (1.75.0)
  [x] memory-daemon not found
  [x] memory-ingest not found

Checking configuration...
  [x] No config.toml found
  [x] CCH hook not configured
  [check] ANTHROPIC_API_KEY set

Recommended actions:
  - Install memory-daemon and memory-ingest
  - Create configuration
  - Configure CCH hook
  - Start daemon
```

### Flag Effects on Detection

| Flag | Effect on State Detection |
|------|---------------------------|
| `--fresh` | Ignore all existing state, treat as new installation |
| `--minimal` | Detect state but use defaults instead of asking |
| `--advanced` | Full detection, will ask additional tuning questions |

---

## Phase 2: Question Phase

**Goal:** Gather user preferences for anything not auto-detected or defaulted.

### Using User Interaction

Ask questions one step at a time to allow for dependent logic:

**Step 1: Installation Method** (if binaries not found)

```
How would you like to install agent-memory?

1. Cargo install (recommended) - Requires Rust toolchain
2. Download pre-built binary - No Rust required
3. Build from source - For development/customization
4. Skip installation - I'll install manually

Default: 1 (Cargo install)
```

**Step 2: Installation Location** (if binary/source selected)

```
Where should the binaries be installed?

1. ~/.local/bin (recommended)
2. ~/.cargo/bin
3. /usr/local/bin (requires sudo)
4. Other path

Default: 1 (~/.local/bin)
```

**Step 3: Summarizer Provider** (if no config.toml)

```
Which LLM provider should generate summaries?

1. Anthropic (Claude) - Best quality summaries
2. OpenAI (GPT-4o-mini) - Fast and cost-effective
3. Local (Ollama) - Private, runs locally
4. None - Skip summarization

Default: 1 (Anthropic) if ANTHROPIC_API_KEY set, else 2 (OpenAI)
```

**Step 4: API Key** (if provider requires key AND env var not set)

```
Enter your Anthropic API key (or press Enter to skip):
Note: You can set ANTHROPIC_API_KEY environment variable instead.
Get a key at: https://console.anthropic.com/settings/keys
```

**Step 5: Hook Scope** (if hooks.yaml not configured)

```
Where should conversation events be captured?

1. Global (all projects)
2. This project only
3. Skip for now

Default: 1 (Global)
```

**Step 6: Daemon Startup** (if daemon not running)

```
How should the daemon be started?

1. Start now + auto-start on login
2. Start now only
3. Don't start

Default: 1 (Start + auto-start)
```

### Handling "Other" Responses

If user types a response not matching options, clarify:

```
I didn't understand that response. Please enter:
- A number (1-4) for the corresponding option
- "skip" to skip this step
- "help" for more information about each option
```

### Question Grouping

For `--advanced` mode, additional questions can be grouped:

```
Configure server settings:
- Port: [50051]
- Host: [[::1]]

Configure storage:
- Data path: [~/.memory-store]

Configure segmentation:
- Min tokens: [500]
- Max tokens: [4000]
- Time gap (minutes): [30]
```

---

## Phase 3: Execution Phase

**Goal:** Execute installation and configuration based on gathered preferences.

### Installation Execution

**Cargo Install:**

```bash
# Show progress
echo "Installing memory-daemon..."
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon

echo "Installing memory-ingest..."
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-ingest

# Verify
memory-daemon --version
memory-ingest --version
```

**Binary Download:**

```bash
# Determine platform
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Download
RELEASE_URL="https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-daemon-${PLATFORM}-${ARCH}.tar.gz"
curl -L "$RELEASE_URL" | tar xz -C /tmp

# Install to chosen location
mv /tmp/memory-daemon ~/.local/bin/
mv /tmp/memory-ingest ~/.local/bin/
chmod +x ~/.local/bin/memory-daemon ~/.local/bin/memory-ingest
```

### Configuration Writing

Create config directory and write config.toml:

```bash
mkdir -p ~/.config/memory-daemon

cat > ~/.config/memory-daemon/config.toml << 'EOF'
[storage]
path = "~/.memory-store"

[server]
host = "[::1]"
port = 50051

[summarizer]
provider = "anthropic"
model = "claude-3-5-haiku-latest"
# API key loaded from ANTHROPIC_API_KEY env var
EOF
```

### Hook Configuration

**Global hooks:**

```bash
# Ensure directory exists
mkdir -p ~/.claude/code_agent_context_hooks

# Check if hooks.yaml exists
if [ -f ~/.claude/code_agent_context_hooks/hooks.yaml ]; then
  # Backup existing
  cp ~/.claude/code_agent_context_hooks/hooks.yaml ~/.claude/code_agent_context_hooks/hooks.yaml.bak

  # Append hook (if not already present)
  if ! grep -q "memory-ingest" ~/.claude/code_agent_context_hooks/hooks.yaml; then
    cat >> ~/.claude/code_agent_context_hooks/hooks.yaml << 'EOF'

  - event: all
    handler:
      type: pipe
      command: memory-ingest
EOF
  fi
else
  # Create new hooks.yaml
  cat > ~/.claude/code_agent_context_hooks/hooks.yaml << 'EOF'
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
EOF
fi
```

### Daemon Startup

```bash
# Create data directory
mkdir -p ~/.memory-store

# Start daemon
memory-daemon start

# Verify running
sleep 2
memory-daemon status
```

### Auto-start Configuration

**macOS (launchd):**

```bash
mkdir -p ~/Library/LaunchAgents
mkdir -p ~/Library/Logs/memory-daemon

# Get actual binary path
DAEMON_PATH=$(which memory-daemon)

cat > ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.spillwave.memory-daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>${DAEMON_PATH}</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$HOME/Library/Logs/memory-daemon/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$HOME/Library/Logs/memory-daemon/stderr.log</string>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist
```

**Linux (systemd):**

```bash
mkdir -p ~/.config/systemd/user

cat > ~/.config/systemd/user/memory-daemon.service << 'EOF'
[Unit]
Description=Agent Memory Daemon
After=network.target

[Service]
ExecStart=%h/.cargo/bin/memory-daemon start --foreground
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable memory-daemon
systemctl --user start memory-daemon
```

---

## Phase 4: Verification Phase

**Goal:** Confirm setup succeeded and provide next steps.

### Verification Checks

```bash
# Check binaries installed
memory-daemon --version && echo "[check] memory-daemon installed"
memory-ingest --version && echo "[check] memory-ingest installed"

# Check config exists
ls ~/.config/memory-daemon/config.toml && echo "[check] Configuration created"

# Check daemon running
memory-daemon status | grep -q "running" && echo "[check] Daemon running"

# Check hook configured
grep -q "memory-ingest" ~/.claude/code_agent_context_hooks/hooks.yaml && echo "[check] CCH hook configured"

# Test connectivity
memory-daemon query root 2>/dev/null && echo "[check] gRPC connectivity verified"
```

### Success Output

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

### Partial Success Output

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

### Failure Output

```
[x] Setup Failed
--------------

Error: Could not start daemon - port 50051 in use

To fix:
  1. Run: lsof -i :50051
  2. Kill the process using the port
  3. Run: /memory-setup --fresh

Need help? Run: /memory-status --verbose
```

---

## Flags

### `--fresh`

Ignore existing configuration and re-ask all questions.

**Behavior:**
- Ignore all existing state during detection
- Ask all questions regardless of what's configured
- Overwrite existing configuration files
- Stop and restart daemon if running
- Warn before overwriting (with backup)

**State Detection Changes:**
- Treat all checks as "not found" or "not configured"
- Still detect platform/architecture for installation

**Warning Prompt:**
```
Warning: This will overwrite existing configuration.
Existing files will be backed up with .bak extension:
  - ~/.config/memory-daemon/config.toml.bak
  - ~/.claude/code_agent_context_hooks/hooks.yaml.bak

Continue? [y/N]
```

**Use case:** Reset to clean state, fix corrupted configuration, upgrade with new defaults

### `--minimal`

Use all defaults without asking questions.

**Behavior:**
- Detect state but skip questions with defaults
- Only prompt for truly required inputs (API key if not in env)
- Use sensible defaults for all options:
  - Installation: cargo if available, else binary
  - Location: ~/.cargo/bin (cargo) or ~/.local/bin (binary)
  - Provider: anthropic if ANTHROPIC_API_KEY set, else openai
  - Hooks: global
  - Daemon: start + auto-start

**Silent Mode:**
- Minimal output during execution
- Only show final success/failure summary
- No confirmations (assumes "yes" to all)

**API Key Handling:**
```
IF ANTHROPIC_API_KEY set THEN
  provider = anthropic
ELSE IF OPENAI_API_KEY set THEN
  provider = openai
ELSE
  PROMPT for API key (required)
END
```

**Use case:** Quick setup for experienced users, CI/CD environments, scripted deployments

### `--advanced`

Show all configuration options including expert settings.

**Behavior:**
- Ask standard questions plus:
  - Server port selection (default: 50051)
  - Server host configuration (default: [::1])
  - Database path selection (default: ~/.memory-store)
  - Segmentation tuning parameters

**Additional Questions:**

```
Configure server settings:
  Port: [50051]
  Host: [[::1]]

Configure storage:
  Data path: [~/.memory-store]

Configure TOC segmentation:
  Minimum tokens per segment: [500]
  Maximum tokens per segment: [4000]
  Time gap threshold (minutes): [30]
```

**Extended Status Output:**
- Show all configuration values after completion
- Include performance recommendations
- Display disk usage projections

**Use case:** Power users, custom deployments, non-standard environments

---

## Flag Combinations

| Combination | Effect |
|-------------|--------|
| `--fresh --minimal` | Clean install with all defaults (fast reset) |
| `--fresh --advanced` | Clean install with all options exposed |
| `--minimal --advanced` | Invalid: mutually exclusive (error) |

### Invalid Combinations

```bash
/memory-setup --minimal --advanced
```

**Error:**
```
Error: --minimal and --advanced are mutually exclusive.

  --minimal: Use defaults, skip questions
  --advanced: Show all options, ask more questions

Choose one or use neither for standard setup.
```

### Flag Priority

When determining behavior, flags are evaluated in order:

1. **`--fresh`** - Applied first (affects state detection)
2. **`--minimal` or `--advanced`** - Applied second (affects question flow)

### Combining with State Detection

| Existing State | No Flags | --fresh | --minimal | --advanced |
|----------------|----------|---------|-----------|------------|
| Nothing installed | Full wizard | Full wizard | Defaults | Extended wizard |
| Binaries only | Config wizard | Full wizard | Complete defaults | Extended wizard |
| Fully configured | "Already setup" | Full wizard | Skip all | Extended wizard |

---

## Error Handling

| Error | Detection | Resolution |
|-------|-----------|------------|
| Rust not installed | `cargo --version` fails | Provide rustup installation command |
| Cargo install fails | Non-zero exit code | Show error, suggest retry or binary download |
| Port in use | `lsof -i :50051` returns process | Show process, suggest kill or different port |
| API key invalid | Test call fails | Explain format, link to key creation |
| Permission denied | Write fails | Suggest different path or fix permissions |
| Daemon won't start | Start command fails | Check logs, suggest troubleshooting |

---

## Examples

**Standard setup (first time):**
```
/memory-setup
```

**Quick reinstall with defaults:**
```
/memory-setup --fresh --minimal
```

**Custom configuration:**
```
/memory-setup --advanced
```

**Reset everything:**
```
/memory-setup --fresh
```
