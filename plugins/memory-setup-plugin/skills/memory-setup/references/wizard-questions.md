# Wizard Questions Flow

Complete interactive wizard question flow for `/memory-setup`. Questions are asked progressively based on current state detection.

## Question Flow Overview

```
State Detection
      │
      ▼
┌─────────────────┐
│ Step 1: Install │ ← Skip if binaries exist (unless --fresh)
│ Method          │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 2: Install │ ← Skip if using cargo install (uses ~/.cargo/bin)
│ Location        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 3: LLM     │ ← Skip if config.toml exists (unless --fresh)
│ Provider        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 4: API Key │ ← Skip if env var set OR provider is "none"
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 5: Hook    │ ← Skip if hooks.yaml configured (unless --fresh)
│ Scope           │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 6: Daemon  │ ← Skip if daemon already running
│ Startup         │
└────────┬────────┘
         │
         ▼
    Execution
```

## Step 1: Installation Method

**Condition:** `memory-daemon` binary not found OR `--fresh` flag provided

**Skip if:** Binary exists AND no `--fresh` flag

### Question

```
How would you like to install agent-memory?

1. Cargo install (recommended) - Requires Rust toolchain
2. Download pre-built binary - No Rust required
3. Build from source - For development/customization
4. Skip installation - I'll install manually
```

### Options

| Option | Condition | Default |
|--------|-----------|---------|
| `cargo` | `cargo --version` succeeds | Yes, if cargo available |
| `binary` | Always available | Yes, if no cargo |
| `source` | `git --version` succeeds | No |
| `skip` | Always available | No |

### Default Selection Logic

```
IF cargo available THEN
  default = "cargo"
ELSE IF platform in [darwin-arm64, darwin-x86_64, linux-x86_64, linux-arm64, windows-x86_64] THEN
  default = "binary"
ELSE
  default = "source"
END
```

### Follow-up

- If `cargo`: Proceed to Step 2 (may skip)
- If `binary`: Ask about installation location (Step 2)
- If `source`: Clone repository, then proceed to Step 3
- If `skip`: Proceed to Step 3

---

## Step 2: Installation Location

**Condition:** Step 1 selected `binary` OR (`source` AND installing)

**Skip if:** Step 1 selected `cargo` (uses `~/.cargo/bin` automatically)

### Question

```
Where should the binaries be installed?

1. ~/.local/bin (recommended) - User-local, no sudo required
2. ~/.cargo/bin - Alongside cargo-installed tools
3. /usr/local/bin - System-wide, requires sudo
4. Other - Specify custom path
```

### Options

| Option | Requires sudo | In PATH typically |
|--------|---------------|-------------------|
| `~/.local/bin` | No | Often yes |
| `~/.cargo/bin` | No | If Rust installed |
| `/usr/local/bin` | Yes | Yes |
| Custom | Depends | Probably not |

### Default Selection Logic

```
IF ~/.local/bin exists AND is in PATH THEN
  default = "~/.local/bin"
ELSE IF ~/.cargo/bin exists AND is in PATH THEN
  default = "~/.cargo/bin"
ELSE
  default = "~/.local/bin" (and offer to create/add to PATH)
END
```

### Validation

```bash
# Check if directory exists
ls -d "$INSTALL_PATH" 2>/dev/null

# Check if in PATH
echo $PATH | grep -q "$INSTALL_PATH" && echo "IN_PATH" || echo "NOT_IN_PATH"

# Check write permissions
touch "$INSTALL_PATH/.write_test" 2>/dev/null && rm "$INSTALL_PATH/.write_test" && echo "WRITABLE" || echo "NOT_WRITABLE"
```

### Follow-up

- If path not in PATH: Warn user and offer to add to shell profile
- Proceed to Step 3

---

## Step 3: Summarizer Provider

**Condition:** `~/.config/memory-daemon/config.toml` does not exist OR `--fresh` flag

**Skip if:** Config exists AND no `--fresh` flag

### Question

```
Which LLM provider should generate summaries?

1. Anthropic (Claude) - Best quality summaries
2. OpenAI (GPT-4o-mini) - Fast and cost-effective
3. Local (Ollama) - Private, runs on your machine
4. None - Skip summarization (TOC only)
```

### Options

| Option | Requires | Default Model |
|--------|----------|---------------|
| `anthropic` | ANTHROPIC_API_KEY | claude-3-5-haiku-latest |
| `openai` | OPENAI_API_KEY | gpt-4o-mini |
| `ollama` | Ollama running | llama3.2:3b |
| `none` | Nothing | N/A |

### Default Selection Logic

```
IF ANTHROPIC_API_KEY set THEN
  default = "anthropic"
ELSE IF OPENAI_API_KEY set THEN
  default = "openai"
ELSE IF ollama running (curl localhost:11434) THEN
  default = "ollama"
ELSE
  default = "openai" (will prompt for key)
END
```

### Provider-Specific Questions

**If `anthropic` or `openai`:**
- Proceed to Step 4 (API Key)

**If `ollama`:**
- Check if Ollama running: `curl -s http://localhost:11434/api/tags`
- If not running: Offer to start or skip to Step 5
- If running: List available models, ask for selection

**If `none`:**
- Skip to Step 5

---

## Step 4: API Key

**Condition:** Step 3 selected provider that requires API key AND env var not set

**Skip if:**
- Provider is `ollama` or `none`
- Relevant env var already set (ANTHROPIC_API_KEY or OPENAI_API_KEY)

### Question (Anthropic)

```
Enter your Anthropic API key (starts with sk-ant-):

Note: You can also set ANTHROPIC_API_KEY environment variable.
Get a key at: https://console.anthropic.com/settings/keys
```

### Question (OpenAI)

```
Enter your OpenAI API key (starts with sk-):

Note: You can also set OPENAI_API_KEY environment variable.
Get a key at: https://platform.openai.com/api-keys
```

### Input Validation

```bash
# Anthropic key format
[[ "$KEY" =~ ^sk-ant-[a-zA-Z0-9_-]+$ ]] && echo "VALID_FORMAT" || echo "INVALID_FORMAT"

# OpenAI key format
[[ "$KEY" =~ ^sk-[a-zA-Z0-9_-]+$ ]] && echo "VALID_FORMAT" || echo "INVALID_FORMAT"
```

### Test Call (Optional)

```bash
# Test Anthropic key
curl -s -H "x-api-key: $KEY" -H "anthropic-version: 2023-06-01" \
  https://api.anthropic.com/v1/messages \
  -d '{"model":"claude-3-5-haiku-latest","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'

# Test OpenAI key
curl -s -H "Authorization: Bearer $KEY" \
  https://api.openai.com/v1/models
```

### Storage Options

```
How should the API key be stored?

1. Environment variable only - Add export to shell profile
2. Config file - Store in config.toml (less secure)
3. Skip storage - I'll manage it myself
```

### Follow-up

- Add to appropriate location based on choice
- Proceed to Step 5

---

## Step 5: Hook Scope

**Condition:** `hooks.yaml` not configured with memory-ingest OR `--fresh` flag

**Skip if:**
- `--minimal` flag (explicitly skip CCH integration)
- hooks.yaml already contains memory-ingest entry

### Question

```
Where should conversation events be captured?

1. Global (all projects) - Add hook to ~/.claude/code_agent_context_hooks/hooks.yaml
2. This project only - Add hook to ./.claude/code_agent_context_hooks/hooks.yaml
3. Skip for now - I'll configure hooks manually later
```

### Options

| Option | Path | Effect |
|--------|------|--------|
| `global` | `~/.claude/code_agent_context_hooks/hooks.yaml` | All conversations captured |
| `project` | `./.claude/code_agent_context_hooks/hooks.yaml` | Only this project |
| `skip` | None | No automatic capture |

### Default Selection Logic

```
IF global hooks.yaml exists THEN
  default = "global"
ELSE IF project hooks.yaml exists THEN
  default = "project"
ELSE
  default = "global"
END
```

### Pre-check

```bash
# Check if CCH is available
ls ~/.claude/code_agent_context_hooks/ 2>/dev/null

# Check existing hook content
cat ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null
```

### Hook Configuration

**For global or project:**

```yaml
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
```

**Merge strategy:** If hooks.yaml exists, append to existing hooks array. If not, create new file.

---

## Step 6: Daemon Startup

**Condition:** Daemon not currently running

**Skip if:** `memory-daemon status` shows "running"

### Question

```
How should the daemon be started?

1. Start now + auto-start on login - Recommended for daily use
2. Start now only - Manual startup after reboots
3. Don't start - I'll start it manually when needed
```

### Options

| Option | Starts Now | Auto-start |
|--------|------------|------------|
| `auto` | Yes | Yes (launchd/systemd) |
| `manual` | Yes | No |
| `skip` | No | No |

### Default Selection

Default: `auto` (start now + auto-start)

### Auto-start Configuration

**macOS (launchd):**

```bash
# Create plist
cat > ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.spillwave.memory-daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/USERNAME/.cargo/bin/memory-daemon</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/Users/USERNAME/Library/Logs/memory-daemon/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/USERNAME/Library/Logs/memory-daemon/stderr.log</string>
</dict>
</plist>
EOF

# Load service
launchctl load ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist
```

**Linux (systemd user service):**

```bash
# Create service file
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

# Enable and start
systemctl --user enable memory-daemon
systemctl --user start memory-daemon
```

---

## Question Dependencies

```
Step 1 ──────────────────────────────────────────────────┐
   │                                                     │
   ├─[cargo]────────► Skip Step 2 ──────────────────────►│
   │                                                     │
   ├─[binary]───────► Step 2 (location) ────────────────►│
   │                                                     │
   └─[source/skip]──► Skip Step 2 ──────────────────────►│
                                                         │
Step 3 ◄─────────────────────────────────────────────────┘
   │
   ├─[anthropic/openai]─► Step 4 (API key) ─────────────►│
   │                                                     │
   ├─[ollama]───────────► Skip Step 4 ──────────────────►│
   │                                                     │
   └─[none]─────────────► Skip Step 4 ──────────────────►│
                                                         │
Step 5 ◄─────────────────────────────────────────────────┘
   │
   └─[all options]──────► Step 6 (daemon)
```

## Minimal Mode (`--minimal`)

When `--minimal` flag is set:

1. **Skip questions entirely** for steps with defaults
2. **Only ask** for required inputs that cannot be defaulted:
   - API key (if chosen provider requires it AND env var not set)
3. **Use defaults:**
   - Installation: cargo if available, else binary
   - Location: ~/.cargo/bin or ~/.local/bin
   - Provider: anthropic (if ANTHROPIC_API_KEY set), else openai
   - Hooks: global
   - Daemon: auto-start

## Advanced Mode (`--advanced`)

When `--advanced` flag is set, add these questions:

### Advanced Step 3a: Server Configuration

```
Configure server settings:

Port: [50051]
Host: [[::1]]
```

### Advanced Step 3b: Storage Configuration

```
Configure storage settings:

Data path: [~/.memory-store]
```

### Advanced Step 3c: Segmentation Tuning

```
Configure TOC segmentation:

Minimum tokens per segment: [500]
Maximum tokens per segment: [4000]
Time gap threshold (minutes): [30]
```

## Fresh Mode (`--fresh`)

When `--fresh` flag is set:

1. **Ignore existing state** for all checks
2. **Ask all questions** regardless of what's configured
3. **Overwrite** existing configuration files
4. **Warn before overwriting:**

```
Warning: This will overwrite existing configuration.
Existing config will be backed up to config.toml.bak

Continue? [y/N]
```
