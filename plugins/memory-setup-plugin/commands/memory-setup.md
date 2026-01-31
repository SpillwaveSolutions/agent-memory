---
name: memory-setup
description: Interactive wizard to install and configure agent-memory
parameters:
  - name: fresh
    description: Fresh installation (ignore existing install)
    required: false
    type: flag
  - name: minimal
    description: Install only the daemon (no CCH integration)
    required: false
    type: flag
  - name: advanced
    description: Show advanced configuration options
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

## Flags

| Flag | Description |
|------|-------------|
| `--fresh` | Ignore existing installation, start fresh |
| `--minimal` | Install only memory-daemon (skip CCH hooks) |
| `--advanced` | Prompt for all configuration options |

## Process

### Phase 1: Environment Check

```bash
# Check Rust toolchain
rustc --version 2>/dev/null || echo "NOT_INSTALLED"

# Check if memory-daemon exists
which memory-daemon 2>/dev/null || echo "NOT_INSTALLED"

# Check if already running
memory-daemon status 2>/dev/null || echo "NOT_RUNNING"
```

**Decision tree:**

- Rust not installed -> Prompt to install via rustup
- memory-daemon not installed -> Proceed to installation
- memory-daemon already installed:
  - `--fresh` flag -> Reinstall
  - No flag -> Ask: upgrade, configure, or abort?

### Phase 2: Installation

```bash
# Install memory-daemon
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon

# Verify
memory-daemon --version
```

If `--minimal` NOT set:

```bash
# Install memory-ingest for CCH integration
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-ingest

# Verify
memory-ingest --version
```

### Phase 3: Configuration

Create config directory:

```bash
mkdir -p ~/.config/memory-daemon
```

**Default config (`~/.config/memory-daemon/config.toml`):**

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

If `--advanced`:

1. Prompt for storage path (default: ~/.memory-store)
2. Prompt for server port (default: 50051)
3. Prompt for LLM provider (openai/anthropic)
4. Prompt for model (varies by provider)
5. Prompt for API key (or env var name)

### Phase 4: CCH Integration (if not --minimal)

Check if CCH is installed:

```bash
ls ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null || echo "NO_CCH"
```

If CCH exists, add hook:

```bash
# Backup existing hooks
cp ~/.claude/code_agent_context_hooks/hooks.yaml ~/.claude/code_agent_context_hooks/hooks.yaml.bak

# Add memory-ingest hook (merge with existing)
cat >> ~/.claude/code_agent_context_hooks/hooks.yaml << 'EOF'

  - event: all
    handler:
      type: pipe
      command: memory-ingest
EOF
```

If CCH doesn't exist:

- Inform user about CCH integration benefits
- Provide manual setup instructions
- Continue without CCH

### Phase 5: Start Daemon

```bash
# Create data directory
mkdir -p ~/.memory-store

# Start daemon
memory-daemon start

# Verify
memory-daemon status
```

### Phase 6: Validation

```bash
# Test gRPC connectivity
memory-daemon query --endpoint http://[::1]:50051 root

# If CCH configured, verify hook
echo '{"type":"session_start","timestamp":"2026-01-31T12:00:00Z"}' | memory-ingest
```

## Output Format

```markdown
## Memory Setup Complete

### Installation Summary

| Component | Status | Version |
|-----------|--------|---------|
| memory-daemon | Installed | 1.0.0 |
| memory-ingest | Installed | 1.0.0 |

### Configuration

| Setting | Value |
|---------|-------|
| Storage Path | ~/.memory-store |
| Server | [::1]:50051 |
| LLM Provider | OpenAI (gpt-4o-mini) |

### Next Steps

1. **Set API key** (if not already):
   ```bash
   export OPENAI_API_KEY="your-key-here"
   ```

2. **Start using memory**:
   - Events captured automatically via CCH hooks
   - Query with: `/memory-search <topic>`

3. **Check status anytime**:
   ```
   /memory-status
   ```
```

## Error Handling

| Error | Resolution |
|-------|------------|
| Rust not installed | Provide rustup installation command |
| Cargo install fails | Check network, try again |
| Port in use | Suggest alternative port |
| API key missing | Show how to set environment variable |

## Examples

**Fresh installation:**
```
/memory-setup --fresh
```

**Minimal setup (no CCH):**
```
/memory-setup --minimal
```

**Advanced options:**
```
/memory-setup --advanced
```
