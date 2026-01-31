# Troubleshooting Guide

Common issues and solutions for agent-memory.

## Quick Diagnostics

Run these commands to quickly assess the situation:

```bash
# Check if daemon binary exists
which memory-daemon

# Check daemon status
memory-daemon status

# Check if port is in use
lsof -i :50051

# Check process
ps aux | grep memory-daemon

# Check logs (macOS)
tail -50 ~/Library/Logs/memory-daemon/daemon.log

# Check logs (Linux)
tail -50 ~/.local/state/memory-daemon/daemon.log
```

## Common Issues

### 1. "command not found: memory-daemon"

**Cause:** Binary not installed or not in PATH.

**Solution:**

```bash
# Check if installed via cargo
ls ~/.cargo/bin/memory-daemon

# Add cargo bin to PATH if needed
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Or reinstall
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon
```

### 2. "Connection refused" / "Failed to connect"

**Cause:** Daemon not running or wrong endpoint.

**Solution:**

```bash
# Start the daemon
memory-daemon start

# Verify it's running
memory-daemon status

# Check which port it's on
ps aux | grep memory-daemon

# Try explicit endpoint
memory-daemon query --endpoint http://[::1]:50051 root
```

### 3. "Address already in use"

**Cause:** Another process using port 50051.

**Solution:**

```bash
# Find what's using the port
lsof -i :50051

# Kill the process (if it's a stale daemon)
kill $(lsof -t -i :50051)

# Or use a different port
memory-daemon start --port 50052
```

### 4. "Permission denied" on data directory

**Cause:** Incorrect permissions on ~/.memory-store.

**Solution:**

```bash
# Check permissions
ls -la ~/.memory-store

# Fix permissions
chmod 700 ~/.memory-store

# If directory doesn't exist
mkdir -p ~/.memory-store
chmod 700 ~/.memory-store
```

### 5. "No events found" / Empty TOC

**Cause:** Events not being ingested or TOC not built.

**Solution:**

```bash
# Check if events exist
memory-daemon admin --db-path ~/.memory-store stats

# If events=0, check CCH integration
cat ~/.claude/code_agent_context_hooks/hooks.yaml

# Test manual ingest
echo '{"type":"user_message","timestamp":"2026-01-31T12:00:00Z","text":"test"}' | memory-ingest

# Check again
memory-daemon admin --db-path ~/.memory-store stats
```

### 6. "API key not found" / Summarization failing

**Cause:** Missing or invalid API key for LLM provider.

**Solution:**

```bash
# Check if environment variable is set
echo $OPENAI_API_KEY

# Set it
export OPENAI_API_KEY="sk-your-key-here"

# Or add to shell profile
echo 'export OPENAI_API_KEY="sk-your-key-here"' >> ~/.zshrc

# Verify config
cat ~/.config/memory-daemon/config.toml | grep -A5 summarizer
```

### 7. "Invalid grip format"

**Cause:** Malformed grip ID in query.

**Solution:**

Grip IDs must match format: `grip:{13-digit-timestamp}:{26-char-ulid}`

Example valid grip: `grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE`

```bash
# Validate grip format
echo "grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE" | grep -E '^grip:[0-9]{13}:[A-Z0-9]{26}$'
```

### 8. Daemon crashes on startup

**Cause:** Corrupted storage or configuration.

**Solution:**

```bash
# Check for corruption
memory-daemon admin --db-path ~/.memory-store stats

# If corrupted, try repair
memory-daemon admin --db-path ~/.memory-store repair

# Last resort: reset storage (DESTROYS DATA)
# rm -rf ~/.memory-store
# mkdir ~/.memory-store
# memory-daemon start
```

### 9. CCH hooks not triggering

**Cause:** hooks.yaml misconfigured or CCH not running.

**Solution:**

```bash
# Verify hooks.yaml exists
cat ~/.claude/code_agent_context_hooks/hooks.yaml

# Should contain:
# hooks:
#   - event: all
#     handler:
#       type: pipe
#       command: memory-ingest

# Check memory-ingest is in PATH
which memory-ingest

# Test manually
echo '{"type":"session_start"}' | memory-ingest
```

### 10. High memory usage

**Cause:** Large write buffer or too many background jobs.

**Solution:**

Edit `~/.config/memory-daemon/config.toml`:

```toml
[storage]
write_buffer_size_mb = 32  # Reduce from default 64
max_background_jobs = 2    # Reduce from default 4
```

Then restart:

```bash
memory-daemon stop
memory-daemon start
```

## Log Locations

| Platform | Log Path |
|----------|----------|
| macOS | `~/Library/Logs/memory-daemon/` |
| Linux | `~/.local/state/memory-daemon/` |
| Windows | `%LOCALAPPDATA%\memory-daemon\logs\` |

## Enabling Debug Logging

```bash
# Temporary (current session)
MEMORY_LOG_LEVEL=debug memory-daemon start

# Permanent (config file)
# Add to ~/.config/memory-daemon/config.toml:
# [logging]
# level = "debug"
```

## Reporting Bugs

If you encounter an issue not covered here:

1. **Gather diagnostics:**
   ```bash
   memory-daemon --version
   memory-daemon status
   memory-daemon admin --db-path ~/.memory-store stats
   cat ~/.config/memory-daemon/config.toml
   ```

2. **Check logs:**
   ```bash
   tail -100 ~/Library/Logs/memory-daemon/daemon.log  # macOS
   tail -100 ~/.local/state/memory-daemon/daemon.log  # Linux
   ```

3. **Open an issue:**
   https://github.com/SpillwaveSolutions/agent-memory/issues

   Include:
   - OS and version
   - memory-daemon version
   - Steps to reproduce
   - Full error message
   - Relevant log output
