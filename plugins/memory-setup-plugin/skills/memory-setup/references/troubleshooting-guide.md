# Troubleshooting Guide

Comprehensive guide for diagnosing and resolving agent-memory issues. Covers 15+ common problems with step-by-step solutions.

## Quick Diagnostics

Run these commands for rapid assessment:

```bash
# Full diagnostic one-liner
echo "=== Binary ===" && which memory-daemon && memory-daemon --version && \
echo "=== Status ===" && memory-daemon status && \
echo "=== Port ===" && lsof -i :50051 && \
echo "=== Storage ===" && memory-daemon admin --db-path ~/.memory-store stats && \
echo "=== Hooks ===" && grep memory-ingest ~/.claude/code_agent_context_hooks/hooks.yaml
```

### Individual Checks

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

# Check storage stats
memory-daemon admin --db-path ~/.memory-store stats

# Check config
cat ~/.config/memory-daemon/config.toml

# Check CCH hooks
cat ~/.claude/code_agent_context_hooks/hooks.yaml
```

## Common Issues

### 1. Daemon Won't Start

**Symptoms:**
- `memory-daemon start` fails
- "Failed to start daemon" error
- Daemon exits immediately after starting

**Diagnosis:**

```bash
# Check if already running
memory-daemon status
ps aux | grep memory-daemon

# Check port availability
lsof -i :50051

# Check for stale PID file
ls -la ~/Library/Application\ Support/memory-daemon/daemon.pid 2>/dev/null || \
ls -la ~/.local/state/memory-daemon/daemon.pid

# Check logs for errors
tail -100 ~/Library/Logs/memory-daemon/daemon.log 2>/dev/null | grep -i error || \
tail -100 ~/.local/state/memory-daemon/daemon.log 2>/dev/null | grep -i error
```

**Solutions:**

**A. Port already in use:**
```bash
# Find what's using the port
lsof -i :50051

# Kill the process (if it's safe)
kill $(lsof -t -i :50051)

# Or use different port
memory-daemon start --port 50052
# Then update config: /memory-config set server.port 50052
```

**B. Stale PID file:**
```bash
# Remove stale PID
rm ~/Library/Application\ Support/memory-daemon/daemon.pid 2>/dev/null
rm ~/.local/state/memory-daemon/daemon.pid 2>/dev/null

# Try starting again
memory-daemon start
```

**C. Permission issues:**
```bash
# Fix data directory permissions
chmod 700 ~/.memory-store

# Fix config directory permissions
chmod 700 ~/.config/memory-daemon
```

**D. Missing config:**
```bash
# Create default config
mkdir -p ~/.config/memory-daemon
cat > ~/.config/memory-daemon/config.toml << 'EOF'
[storage]
path = "~/.memory-store"

[server]
host = "[::1]"
port = 50051

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
EOF

memory-daemon start
```

---

### 2. Events Not Being Captured

**Symptoms:**
- `memory-daemon admin stats` shows 0 events
- Memory queries return empty results
- "No events found" messages

**Diagnosis:**

```bash
# Check storage stats
memory-daemon admin --db-path ~/.memory-store stats

# Check CCH hooks
cat ~/.claude/code_agent_context_hooks/hooks.yaml

# Check memory-ingest binary
which memory-ingest
memory-ingest --version

# Test manual ingest
echo '{"type":"session_start","timestamp":"2026-01-31T12:00:00Z","data":{}}' | memory-ingest
```

**Solutions:**

**A. CCH hook not configured:**
```bash
# Create hooks directory
mkdir -p ~/.claude/code_agent_context_hooks

# Add memory-ingest hook
cat >> ~/.claude/code_agent_context_hooks/hooks.yaml << 'EOF'
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest
EOF
```

**B. memory-ingest not in PATH:**
```bash
# Check if installed
ls ~/.cargo/bin/memory-ingest

# Add cargo bin to PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Or create symlink
sudo ln -s ~/.cargo/bin/memory-ingest /usr/local/bin/
```

**C. Daemon not running:**
```bash
# Start daemon
memory-daemon start

# Verify
memory-daemon status
```

**D. Wrong endpoint in ingest:**
```bash
# Check config port
grep port ~/.config/memory-daemon/config.toml

# memory-ingest uses default port 50051, if daemon uses different port:
# Set MEMORY_ENDPOINT environment variable
export MEMORY_ENDPOINT="http://[::1]:50052"
```

---

### 3. Summarization Failing

**Symptoms:**
- TOC nodes have no summaries
- "Summarization error" in logs
- "API key not found" errors

**Diagnosis:**

```bash
# Check summarizer config
grep -A10 '\[summarizer\]' ~/.config/memory-daemon/config.toml

# Check API keys
echo "OPENAI_API_KEY: ${OPENAI_API_KEY:+set (${#OPENAI_API_KEY} chars)}"
echo "ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY:+set (${#ANTHROPIC_API_KEY} chars)}"

# Test API connectivity
curl -s -o /dev/null -w "%{http_code}" https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Check logs for API errors
grep -i "api\|summar\|llm" ~/Library/Logs/memory-daemon/daemon.log | tail -20
```

**Solutions:**

**A. Missing API key:**
```bash
# Set OpenAI key
export OPENAI_API_KEY="sk-your-key-here"

# Add to shell profile for persistence
echo 'export OPENAI_API_KEY="sk-your-key-here"' >> ~/.zshrc

# Restart daemon
memory-daemon stop && memory-daemon start
```

**B. Wrong provider configured:**
```bash
# If using Anthropic but config says OpenAI
/memory-config set summarizer.provider anthropic
/memory-config set summarizer.model claude-3-5-haiku-latest
```

**C. Invalid API key:**
```bash
# Test key validity
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# If 401 error: key is invalid, get new key from provider
```

**D. Rate limiting:**
```bash
# Check logs for rate limit errors
grep -i "rate\|limit\|429" ~/Library/Logs/memory-daemon/daemon.log

# Solution: wait and retry, or upgrade API plan
```

---

### 4. High Disk Usage

**Symptoms:**
- ~/.memory-store growing large
- Disk space warnings
- Slow queries

**Diagnosis:**

```bash
# Check disk usage
du -sh ~/.memory-store

# Check storage stats
memory-daemon admin --db-path ~/.memory-store stats

# Check for WAL files
ls -la ~/.memory-store/*.log 2>/dev/null
ls -la ~/.memory-store/wal/ 2>/dev/null
```

**Solutions:**

**A. Run compaction:**
```bash
# Compact all column families
memory-daemon admin --db-path ~/.memory-store compact

# Compact specific column family
memory-daemon admin --db-path ~/.memory-store compact --cf events
```

**B. Reduce write buffer:**
```bash
# Reduce write buffer from 64MB to 32MB
/memory-config set storage.write_buffer_size_mb 32

# Restart daemon
memory-daemon stop && memory-daemon start
```

**C. Archive old data (if needed):**
```bash
# Export old events (not implemented yet - future feature)
# For now, consider backing up and recreating
cp -r ~/.memory-store ~/.memory-store.backup
rm -rf ~/.memory-store
memory-daemon start
```

---

### 5. Port Conflicts

**Symptoms:**
- "Address already in use" error
- Daemon can't bind to port
- Connection refused to expected port

**Diagnosis:**

```bash
# Check what's using port 50051
lsof -i :50051

# Check all listening ports
netstat -an | grep LISTEN | grep 5005

# Check daemon config
grep port ~/.config/memory-daemon/config.toml
```

**Solutions:**

**A. Kill conflicting process:**
```bash
# Find PID
lsof -t -i :50051

# Kill it (if safe)
kill $(lsof -t -i :50051)
```

**B. Use different port:**
```bash
# Change port in config
/memory-config set server.port 50052

# Restart daemon
memory-daemon stop && memory-daemon start

# Update any scripts using old port
```

**C. Check for stale daemon:**
```bash
# Multiple daemon instances?
ps aux | grep memory-daemon

# Kill all and restart
pkill memory-daemon
memory-daemon start
```

---

### 6. Hook Errors

**Symptoms:**
- CCH reports hook failures
- memory-ingest errors
- Events not piping correctly

**Diagnosis:**

```bash
# Check hooks.yaml syntax
cat ~/.claude/code_agent_context_hooks/hooks.yaml

# Test memory-ingest directly
echo '{"type":"test"}' | memory-ingest

# Check daemon is accepting connections
memory-daemon query root
```

**Solutions:**

**A. Fix hooks.yaml syntax:**
```yaml
# Correct format:
hooks:
  - event: all
    handler:
      type: pipe
      command: memory-ingest

# Common mistakes:
# - Wrong indentation (use 2 spaces)
# - Missing 'hooks:' key
# - Wrong handler type
```

**B. Fix binary path:**
```bash
# If memory-ingest not found
which memory-ingest

# Add full path to hooks.yaml if needed:
hooks:
  - event: all
    handler:
      type: pipe
      command: /Users/yourname/.cargo/bin/memory-ingest
```

**C. Daemon not running:**
```bash
# memory-ingest needs daemon
memory-daemon start
```

---

### 7. API Key Issues

**Symptoms:**
- "API key not found" errors
- "Invalid API key" errors
- "Unauthorized" responses

**Diagnosis:**

```bash
# Check if variables are set
env | grep -E "(OPENAI|ANTHROPIC)_API_KEY"

# Check key format (without revealing full key)
echo "OPENAI length: ${#OPENAI_API_KEY}"
echo "OPENAI prefix: ${OPENAI_API_KEY:0:7}..."

# Test key
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY" 2>&1 | head -5
```

**Solutions:**

**A. Set environment variable:**
```bash
# In current shell
export OPENAI_API_KEY="sk-..."

# Persist in shell profile
echo 'export OPENAI_API_KEY="sk-..."' >> ~/.zshrc
source ~/.zshrc

# Or use .env file
echo 'OPENAI_API_KEY=sk-...' >> ~/.config/memory-daemon/.env
```

**B. Wrong key format:**
```bash
# OpenAI keys start with "sk-"
# Anthropic keys start with "sk-ant-"
# Make sure no extra whitespace:
export OPENAI_API_KEY="$(echo $OPENAI_API_KEY | tr -d '[:space:]')"
```

**C. Key expired/revoked:**
- Generate new key from provider dashboard
- Update environment variable
- Restart daemon

---

### 8. Permission Problems

**Symptoms:**
- "Permission denied" errors
- Can't write to storage
- Can't read config

**Diagnosis:**

```bash
# Check data directory
ls -la ~/.memory-store
ls -la ~/.memory-store/LOCK 2>/dev/null

# Check config directory
ls -la ~/.config/memory-daemon

# Check running user
whoami
ps aux | grep memory-daemon
```

**Solutions:**

**A. Fix directory permissions:**
```bash
# Data directory
chmod 700 ~/.memory-store
chown $(whoami) ~/.memory-store

# Config directory
chmod 700 ~/.config/memory-daemon
chown $(whoami) ~/.config/memory-daemon
```

**B. Fix lock file:**
```bash
# Remove stale lock
rm ~/.memory-store/LOCK

# Start daemon
memory-daemon start
```

**C. Running as wrong user:**
```bash
# Check ownership
ls -la ~/Library/Application\ Support/memory-daemon/

# Fix ownership
chown -R $(whoami) ~/Library/Application\ Support/memory-daemon/
```

---

### 9. Connection Timeouts

**Symptoms:**
- Queries hang then fail
- "Connection timeout" errors
- Slow responses

**Diagnosis:**

```bash
# Check daemon responsiveness
time memory-daemon query root

# Check daemon load
ps aux | grep memory-daemon

# Check network
ping -c 1 localhost
```

**Solutions:**

**A. Increase timeout:**
```bash
# In config
/memory-config set server.timeout_secs 60
```

**B. Reduce load:**
```bash
# Reduce background jobs
/memory-config set storage.max_background_jobs 2
```

**C. Restart daemon:**
```bash
memory-daemon stop
sleep 2
memory-daemon start
```

---

### 10. Database Corruption

**Symptoms:**
- "Corruption" errors in logs
- Queries fail with storage errors
- Daemon crashes on startup

**Diagnosis:**

```bash
# Check logs for corruption
grep -i "corrupt" ~/Library/Logs/memory-daemon/daemon.log

# Try reading stats
memory-daemon admin --db-path ~/.memory-store stats
```

**Solutions:**

**A. Try repair (if available):**
```bash
memory-daemon admin --db-path ~/.memory-store repair
```

**B. Restore from backup:**
```bash
# If you have backups
rm -rf ~/.memory-store
cp -r ~/.memory-store.backup ~/.memory-store
memory-daemon start
```

**C. Reset database (DATA LOSS):**
```bash
# Last resort - loses all data
rm -rf ~/.memory-store
mkdir -p ~/.memory-store
chmod 700 ~/.memory-store
memory-daemon start
```

---

### 11. "Command Not Found: memory-daemon"

**Symptoms:**
- Shell can't find command
- "command not found" error
- Works in some shells but not others

**Diagnosis:**

```bash
# Check if installed
ls ~/.cargo/bin/memory-daemon

# Check PATH
echo $PATH | tr ':' '\n' | grep cargo

# Check shell config
grep cargo ~/.zshrc ~/.bashrc 2>/dev/null
```

**Solutions:**

**A. Add to PATH:**
```bash
# For zsh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# For bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**B. Reinstall:**
```bash
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon
```

**C. Create symlink:**
```bash
sudo ln -s ~/.cargo/bin/memory-daemon /usr/local/bin/
sudo ln -s ~/.cargo/bin/memory-ingest /usr/local/bin/
```

---

### 12. Startup Service Not Running

**Symptoms:**
- Daemon doesn't start on login
- Have to manually start after reboot
- launchd/systemd errors

**Diagnosis:**

```bash
# macOS - check launchd
launchctl list | grep memory

# Linux - check systemd
systemctl --user status memory-daemon

# Check service files exist
ls ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist 2>/dev/null
ls ~/.config/systemd/user/memory-daemon.service 2>/dev/null
```

**Solutions:**

**A. macOS - install launchd service:**
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
        <string>/Users/YOUR_USERNAME/.cargo/bin/memory-daemon</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
EOF

# Load service
launchctl load ~/Library/LaunchAgents/com.spillwave.memory-daemon.plist
```

**B. Linux - install systemd service:**
```bash
# Create service file
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/memory-daemon.service << 'EOF'
[Unit]
Description=Agent Memory Daemon
After=network.target

[Service]
ExecStart=%h/.cargo/bin/memory-daemon start --foreground
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

# Enable and start
systemctl --user daemon-reload
systemctl --user enable memory-daemon
systemctl --user start memory-daemon
```

---

### 13. Query Returns Empty Results

**Symptoms:**
- Queries return no data
- TOC is empty
- Searches find nothing

**Diagnosis:**

```bash
# Check event count
memory-daemon admin --db-path ~/.memory-store stats

# Check TOC root
memory-daemon query root

# Check if events exist but TOC not built
memory-daemon admin --db-path ~/.memory-store stats | grep -E "events|toc"
```

**Solutions:**

**A. Wait for TOC build:**
TOC builds asynchronously. If events exist but TOC is empty, wait a few minutes.

**B. Check time range:**
```bash
# Query specific time range
memory-daemon query browse --from "2026-01-01" --to "2026-02-01"
```

**C. Rebuild TOC:**
```bash
memory-daemon admin --db-path ~/.memory-store rebuild-toc
```

---

### 14. Slow Queries

**Symptoms:**
- Queries take seconds to respond
- UI feels sluggish
- Timeouts on complex queries

**Diagnosis:**

```bash
# Time a simple query
time memory-daemon query root

# Check storage size
du -sh ~/.memory-store

# Check system resources
top -l 1 | grep memory-daemon
```

**Solutions:**

**A. Run compaction:**
```bash
memory-daemon admin --db-path ~/.memory-store compact
```

**B. Optimize config:**
```bash
# Increase background jobs for faster compaction
/memory-config set storage.max_background_jobs 8

# Increase write buffer for batch efficiency
/memory-config set storage.write_buffer_size_mb 128
```

**C. Check disk I/O:**
```bash
# Use SSD if possible
# Move storage to faster disk
/memory-config set storage.path /ssd/memory-store
```

---

### 15. Update/Upgrade Issues

**Symptoms:**
- New version won't install
- Conflicts with old version
- Missing features after update

**Diagnosis:**

```bash
# Check current version
memory-daemon --version

# Check installed binaries
ls -la ~/.cargo/bin/memory-*
```

**Solutions:**

**A. Clean install:**
```bash
# Remove old binaries
rm ~/.cargo/bin/memory-daemon
rm ~/.cargo/bin/memory-ingest

# Fresh install
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon --force
```

**B. Clear cargo cache:**
```bash
cargo cache -a  # If cargo-cache installed
# Or manually
rm -rf ~/.cargo/registry/cache/
rm -rf ~/.cargo/git/
```

**C. Stop daemon before update:**
```bash
memory-daemon stop
cargo install --git https://github.com/SpillwaveSolutions/agent-memory memory-daemon --force
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
/memory-config set logging.level debug

# View debug logs
tail -f ~/Library/Logs/memory-daemon/daemon.log
```

**Remember to reset to info level after debugging:**
```bash
/memory-config set logging.level info
```

## Getting Help

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

3. **Use troubleshooter:**
   Say "troubleshoot memory" or "fix memory issues" to activate the setup-troubleshooter agent.

4. **Open an issue:**
   https://github.com/SpillwaveSolutions/agent-memory/issues

   Include:
   - OS and version
   - memory-daemon version
   - Steps to reproduce
   - Full error message
   - Relevant log output
