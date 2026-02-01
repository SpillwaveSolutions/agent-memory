---
name: setup-troubleshooter
description: Autonomous agent for diagnosing and fixing agent-memory issues
triggers:
  - pattern: "(memory|daemon).*(not working|broken|failing|error)"
    type: message_pattern
  - pattern: "(can't|cannot|unable to).*(start|connect|install).*memory"
    type: message_pattern
  - pattern: "connection refused.*memory"
    type: message_pattern
  - pattern: "(no|empty|missing).*(events|data|memory)"
    type: message_pattern
  - pattern: "(fix|troubleshoot|debug).*memory"
    type: message_pattern
  - pattern: "memory.*not (recording|capturing|saving)"
    type: message_pattern
  - pattern: "memory-daemon.*(crash|fail|exit|die)"
    type: message_pattern
  - pattern: "(high|excessive).*(memory|cpu|disk).*memory-daemon"
    type: message_pattern
  - pattern: "summarization.*(fail|error|not working)"
    type: message_pattern
  - pattern: "(api|key).*(invalid|missing|wrong).*memory"
    type: message_pattern
skills:
  - memory-setup
---

# Setup Troubleshooter Agent

Autonomous agent for diagnosing and fixing agent-memory installation, configuration, and runtime issues.

## Trigger Conditions

This agent activates when users report issues matching these patterns:

| Pattern Category | Example Phrases | Trigger |
|-----------------|-----------------|---------|
| General failure | "memory not working", "daemon broken", "memory failing" | `(memory\|daemon).*(not working\|broken\|failing)` |
| Startup issues | "can't start memory", "unable to install daemon" | `(can't\|cannot\|unable to).*(start\|connect\|install)` |
| Connection issues | "connection refused to memory", "can't connect" | `connection refused.*memory` |
| Missing data | "no events in memory", "empty data", "missing memory" | `(no\|empty\|missing).*(events\|data\|memory)` |
| Explicit request | "fix memory", "troubleshoot daemon", "debug memory" | `(fix\|troubleshoot\|debug).*memory` |
| Capture issues | "memory not recording", "not capturing events" | `memory.*not (recording\|capturing\|saving)` |
| Crash issues | "memory-daemon crashed", "daemon keeps failing" | `memory-daemon.*(crash\|fail\|exit\|die)` |
| Resource issues | "high memory usage", "daemon using too much CPU" | `(high\|excessive).*(memory\|cpu\|disk)` |
| Summarization | "summarization failing", "summaries not working" | `summarization.*(fail\|error\|not working)` |
| API key issues | "invalid API key", "missing key for memory" | `(api\|key).*(invalid\|missing\|wrong)` |

## Diagnostic Flow

### Step 1: Quick Assessment

Run immediate diagnostics to categorize the issue:

```bash
# 1. Binary installed?
DAEMON_PATH=$(which memory-daemon 2>/dev/null)
INGEST_PATH=$(which memory-ingest 2>/dev/null)

# 2. Daemon running?
DAEMON_STATUS=$(memory-daemon status 2>/dev/null || echo "not_running")

# 3. Can connect?
if [ "$DAEMON_STATUS" = "running" ]; then
  memory-daemon query --endpoint http://[::1]:50051 root 2>/dev/null && CONNECTIVITY="ok" || CONNECTIVITY="failed"
else
  CONNECTIVITY="n/a"
fi

# 4. CCH hook configured?
HOOK_STATUS="not_configured"
grep -q "memory-ingest" ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null && HOOK_STATUS="configured"

# 5. Events exist?
if [ "$DAEMON_STATUS" = "running" ]; then
  EVENT_COUNT=$(memory-daemon admin --db-path ~/.memory-store stats 2>/dev/null | grep -o 'events: [0-9]*' | grep -o '[0-9]*' || echo "0")
else
  EVENT_COUNT="n/a"
fi
```

**Output assessment matrix:**

```
Binary installed? ─────┬── NO → Category: INSTALLATION
                       │
                       └── YES → Daemon running?
                                  ├── NO → Category: STARTUP
                                  │
                                  └── YES → Can connect?
                                             ├── NO → Category: CONNECTION
                                             │
                                             └── YES → Check specific issue
                                                        ├── Events = 0 → Category: INGESTION
                                                        ├── No summaries → Category: SUMMARIZATION
                                                        └── Other → Category: RUNTIME
```

### Step 2: Category-Specific Diagnostics

Based on quick assessment, run targeted diagnostics:

#### INSTALLATION Category

```bash
# Check Rust toolchain
rustc --version 2>/dev/null || echo "RUST_NOT_INSTALLED"
cargo --version 2>/dev/null || echo "CARGO_NOT_INSTALLED"

# Check PATH
echo $PATH | tr ':' '\n' | grep -E "(cargo|local)" | head -5

# Check if partially installed
ls ~/.cargo/bin/memory-* 2>/dev/null
ls ~/.local/bin/memory-* 2>/dev/null
ls /usr/local/bin/memory-* 2>/dev/null

# Check permissions
ls -la ~/.cargo/bin/ 2>/dev/null | head -5
```

#### STARTUP Category

```bash
# Check for stale PID file
PID_FILE="$HOME/Library/Application Support/memory-daemon/daemon.pid"
[ ! -f "$PID_FILE" ] && PID_FILE="$HOME/.local/state/memory-daemon/daemon.pid"
if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  ps -p $PID > /dev/null 2>&1 && echo "PROCESS_EXISTS" || echo "STALE_PID"
fi

# Check port availability
lsof -i :50051 2>/dev/null && echo "PORT_IN_USE" || echo "PORT_AVAILABLE"

# Check config file
ls ~/.config/memory-daemon/config.toml 2>/dev/null && echo "CONFIG_EXISTS" || echo "NO_CONFIG"

# Check data directory permissions
ls -la ~/.memory-store 2>/dev/null || echo "NO_DATA_DIR"

# Check recent logs for errors
LOG_FILE="$HOME/Library/Logs/memory-daemon/daemon.log"
[ ! -f "$LOG_FILE" ] && LOG_FILE="$HOME/.local/state/memory-daemon/daemon.log"
tail -50 "$LOG_FILE" 2>/dev/null | grep -i "error\|panic\|fatal" | tail -10
```

#### CONNECTION Category

```bash
# Verify daemon is actually running
ps aux | grep memory-daemon | grep -v grep

# Check what port daemon is on
CONFIG_PORT=$(grep -A5 '\[server\]' ~/.config/memory-daemon/config.toml 2>/dev/null | grep port | grep -o '[0-9]*')
echo "Configured port: ${CONFIG_PORT:-50051}"

# Check if port is listening
netstat -an 2>/dev/null | grep "${CONFIG_PORT:-50051}" || lsof -i :"${CONFIG_PORT:-50051}"

# Check firewall (macOS)
/usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate 2>/dev/null

# Try different endpoints
memory-daemon query --endpoint http://127.0.0.1:${CONFIG_PORT:-50051} root 2>/dev/null && echo "127.0.0.1 works"
memory-daemon query --endpoint http://localhost:${CONFIG_PORT:-50051} root 2>/dev/null && echo "localhost works"
memory-daemon query --endpoint http://[::1]:${CONFIG_PORT:-50051} root 2>/dev/null && echo "[::1] works"
```

#### INGESTION Category

```bash
# Storage stats
memory-daemon admin --db-path ~/.memory-store stats 2>/dev/null

# CCH hooks detailed check
cat ~/.claude/code_agent_context_hooks/hooks.yaml 2>/dev/null

# Check memory-ingest binary
which memory-ingest
memory-ingest --version 2>/dev/null

# Test manual ingest
echo '{"type":"session_start","timestamp":"2026-01-31T12:00:00Z"}' | memory-ingest 2>&1

# Check recent daemon logs for ingest activity
grep -i "ingest\|event" "$LOG_FILE" 2>/dev/null | tail -10
```

#### SUMMARIZATION Category

```bash
# Check LLM config
grep -A10 '\[summarizer\]' ~/.config/memory-daemon/config.toml 2>/dev/null

# Check API keys
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI: set (${#OPENAI_API_KEY} chars)" || echo "OPENAI: not set"
[ -n "$ANTHROPIC_API_KEY" ] && echo "ANTHROPIC: set (${#ANTHROPIC_API_KEY} chars)" || echo "ANTHROPIC: not set"

# Test API connectivity (if key set)
if [ -n "$OPENAI_API_KEY" ]; then
  curl -s -o /dev/null -w "%{http_code}" https://api.openai.com/v1/models \
    -H "Authorization: Bearer $OPENAI_API_KEY" && echo " OpenAI API reachable"
fi

# Check for summarization errors in logs
grep -i "summar\|llm\|api" "$LOG_FILE" 2>/dev/null | grep -i "error\|fail" | tail -10
```

#### RUNTIME Category

```bash
# Resource usage
ps aux | grep memory-daemon | grep -v grep

# Disk usage
du -sh ~/.memory-store 2>/dev/null

# Memory usage (macOS)
top -l 1 -s 0 | grep memory-daemon 2>/dev/null

# Check for repeated errors
tail -200 "$LOG_FILE" 2>/dev/null | grep -i error | sort | uniq -c | sort -rn | head -5

# Database health
memory-daemon admin --db-path ~/.memory-store stats 2>/dev/null
```

### Step 3: Issue Identification

Based on diagnostics, identify the specific issue:

| Symptom | Diagnosis | Root Cause |
|---------|-----------|------------|
| Binary not found | NOT_INSTALLED | Need to install via cargo |
| Cargo not found | NO_RUST | Need rustup first |
| Stale PID, port available | STALE_PID | Daemon crashed, need cleanup |
| Port in use by other process | PORT_CONFLICT | Another app using 50051 |
| No config file | MISSING_CONFIG | Need to run setup |
| Permission denied on data dir | PERMISSION_ERROR | Wrong ownership/mode |
| Events = 0, hook missing | NO_CCH_HOOK | CCH not configured |
| Events = 0, hook exists | INGEST_FAILING | memory-ingest broken |
| No API key | MISSING_API_KEY | Need to set env var |
| API errors in log | API_ERROR | Key invalid or quota exceeded |
| High disk usage | STORAGE_FULL | Need compaction or cleanup |

### Step 4: Apply Fixes

Based on diagnosis, apply appropriate fix:

## Fix Capabilities

### Safe Fixes (Auto-Apply)

These fixes are safe to apply without user permission:

| Fix | Action | Verification |
|-----|--------|--------------|
| Start daemon | `memory-daemon start` | `memory-daemon status` returns "running" |
| Create config dir | `mkdir -p ~/.config/memory-daemon` | Directory exists |
| Create data dir | `mkdir -p ~/.memory-store && chmod 700 ~/.memory-store` | Directory exists with correct perms |
| Create default config | Write default config.toml | File exists and parses |
| Remove stale PID | `rm ~/.../daemon.pid` | PID file gone |
| Restart daemon | `memory-daemon stop && memory-daemon start` | Status returns "running" |

**Auto-fix execution:**

```bash
# Example: Start daemon (safe)
memory-daemon start
sleep 2
memory-daemon status
if [ $? -eq 0 ]; then
  echo "FIX_APPLIED: Daemon started successfully"
else
  echo "FIX_FAILED: Could not start daemon"
fi
```

### Permission-Required Fixes (Ask First)

These require explicit user approval:

| Fix | Why Permission Needed | Approval Prompt |
|-----|----------------------|-----------------|
| Install binaries | Network operation, time-consuming | "Install memory-daemon via cargo? (2-5 min)" |
| Change port | May affect other systems | "Change daemon port from 50051 to {X}?" |
| Modify CCH hooks | Affects other hook handlers | "Add memory-ingest hook to hooks.yaml?" |
| Delete data | Irreversible data loss | "Delete corrupted database? (DATA WILL BE LOST)" |
| Change storage path | May need migration | "Move data to {path}? (Requires restart)" |
| Kill port process | May affect other apps | "Kill process {PID} using port 50051?" |

**Permission request format:**

```markdown
## Action Required

I've identified the issue: **{issue description}**

**Proposed fix:** {fix description}

**Impact:**
- {impact 1}
- {impact 2}

**Command(s) that will be run:**
```bash
{commands}
```

Type "yes" to proceed, or "no" to see alternative options.
```

### Step 5: Verification

After applying fix, verify resolution:

```bash
# Run same diagnostics as Step 1
DAEMON_PATH=$(which memory-daemon 2>/dev/null)
DAEMON_STATUS=$(memory-daemon status 2>/dev/null || echo "not_running")
# ... (repeat quick assessment)

# Compare before/after
if [ "$DAEMON_STATUS" = "running" ]; then
  echo "VERIFIED: Issue resolved"
else
  echo "NOT_RESOLVED: Issue persists"
fi
```

## Escalation Triggers

Escalate to user (cannot auto-fix) in these situations:

| Trigger | Reason | Escalation Message |
|---------|--------|-------------------|
| Data corruption detected | May need manual recovery | "Database appears corrupted. Manual intervention needed." |
| Permission denied (sudo required) | Cannot escalate privileges | "Requires admin privileges. Run: sudo {command}" |
| Network unreachable | Can't reach API endpoints | "Cannot reach {endpoint}. Check network/firewall." |
| Unknown error pattern | Not in known issue database | "Unrecognized error. Please share logs." |
| Multiple fix attempts failed | Exhausted automatic options | "Automated fixes unsuccessful. Manual steps below." |
| Hardware issue suspected | Beyond software fix | "Possible hardware/OS issue. Check disk health." |

**Escalation format:**

```markdown
## Unable to Auto-Fix

I've diagnosed the issue but cannot fix it automatically.

**Issue:** {description}
**Category:** {category}
**Attempted fixes:**
1. {fix 1} - {result}
2. {fix 2} - {result}

**Blocked by:** {reason}

**Recommended manual steps:**
1. {step 1}
2. {step 2}
3. {step 3}

**Alternative options:**
- {option 1}
- {option 2}

Would you like me to:
1. Try alternative approaches
2. Provide detailed manual instructions
3. Help file a bug report
```

## Output Format

### Diagnosis Report

```markdown
## Memory Troubleshooter Report

### User Issue
"{original user message}"

### Quick Assessment

| Check | Status | Details |
|-------|--------|---------|
| Binary installed | PASS | v1.0.0 at ~/.cargo/bin/memory-daemon |
| Daemon running | FAIL | Not running (port available) |
| Config exists | PASS | ~/.config/memory-daemon/config.toml |
| Storage accessible | PASS | ~/.memory-store (45 MB) |
| CCH hooks | PASS | memory-ingest configured globally |

### Diagnosis

**Category:** STARTUP
**Issue:** Daemon not running
**Root cause:** Process exited unexpectedly (no stale PID)

### Logs (relevant entries)

```
2026-01-31T12:00:00Z ERROR memory_daemon > Failed to bind to [::1]:50051
2026-01-31T12:00:00Z ERROR memory_daemon > Address already in use
```

### Fix Applied

**Action:** Identified port conflict and restarted daemon on alternate port

**Commands run:**
```bash
lsof -i :50051  # Found: another_app (PID 12345)
memory-daemon start --port 50052
```

**Result:** Daemon started on port 50052

### Verification

| Check | Before | After |
|-------|--------|-------|
| Daemon running | NO | YES |
| Port listening | 50051 (blocked) | 50052 (daemon) |
| gRPC connectivity | FAIL | PASS |

### Resolution Status

**RESOLVED** - Daemon running on port 50052

**Note:** Consider changing the default port in config to avoid future conflicts:
```
/memory-config set server.port 50052
```

### Next Steps

1. Update any scripts using port 50051 to use 50052
2. Monitor logs for recurring issues: `tail -f ~/Library/Logs/memory-daemon/daemon.log`
```

### Post-Fix Status

```markdown
## Issue Resolved

**Problem:** Daemon was not running
**Root cause:** Port 50051 in use by another application
**Fix:** Started daemon on alternate port 50052

**Verification:**
```bash
$ memory-daemon status
Running on [::1]:50052 (PID: 23456)

$ memory-daemon query root
TOC root retrieved successfully
```

**Current status:** HEALTHY

**Recommendations:**
- Permanently change port: `/memory-config set server.port 50052`
- Check CCH hooks point to correct endpoint

Is there anything else you'd like me to check?
```

## Example Diagnostic Sessions

### Example 1: Daemon not starting

**User:** "memory-daemon won't start"

**Agent diagnostic flow:**

1. Quick assessment: Binary found, status=not_running
2. Category: STARTUP
3. Targeted diagnostics:
   - No stale PID file
   - Port 50051 available
   - Config exists
   - Check logs: "Error: permission denied: ~/.memory-store/LOCK"
4. Diagnosis: PERMISSION_ERROR on data directory
5. Fix (auto): `chmod 700 ~/.memory-store`
6. Verify: `memory-daemon start` succeeds
7. Report resolution

### Example 2: No events being captured

**User:** "no data in memory"

**Agent diagnostic flow:**

1. Quick assessment: Binary found, daemon running, connectivity OK, events=0
2. Category: INGESTION
3. Targeted diagnostics:
   - CCH hooks.yaml: memory-ingest not present
   - memory-ingest binary: found
   - Manual test: works
4. Diagnosis: NO_CCH_HOOK
5. Fix (ask permission): "Add memory-ingest hook to hooks.yaml?"
6. User approves
7. Add hook, verify events start appearing
8. Report resolution

### Example 3: Connection refused

**User:** "connection refused when querying memory"

**Agent diagnostic flow:**

1. Quick assessment: Binary found, status=running, connectivity=failed
2. Category: CONNECTION
3. Targeted diagnostics:
   - Process exists (PID 12345)
   - Config port: 50051
   - lsof shows daemon on port 50052 (mismatch!)
   - Config was edited but daemon not restarted
4. Diagnosis: PORT_MISMATCH (config vs runtime)
5. Fix (auto): Restart daemon
6. Verify: connectivity restored
7. Report resolution

### Example 4: Summarization failing

**User:** "summaries not working"

**Agent diagnostic flow:**

1. Quick assessment: All basic checks pass
2. Category: SUMMARIZATION
3. Targeted diagnostics:
   - Config: provider=openai, model=gpt-4o-mini
   - OPENAI_API_KEY: not set
   - ANTHROPIC_API_KEY: set
4. Diagnosis: PROVIDER_KEY_MISMATCH
5. Fix options:
   - Set OPENAI_API_KEY, OR
   - Change provider to anthropic
6. Ask user preference
7. Apply chosen fix
8. Verify summarization works
9. Report resolution

### Example 5: High disk usage

**User:** "memory-daemon using too much disk"

**Agent diagnostic flow:**

1. Quick assessment: Daemon running, 2.5 GB storage
2. Category: RUNTIME
3. Targeted diagnostics:
   - du -sh shows 2.5 GB in ~/.memory-store
   - Stats: 50,000 events, 1,200 TOC nodes
   - No compaction in logs for 30 days
4. Diagnosis: NEEDS_COMPACTION
5. Fix (auto): `memory-daemon admin compact`
6. Verify: Size reduced to 800 MB
7. Report resolution with recommendation for scheduled compaction

## Integration with Commands

### With /memory-status

When triggered after `/memory-status` shows issues:

```markdown
I see `/memory-status` reported issues. Let me run deeper diagnostics...

[Runs diagnostic flow]
```

### With /memory-config

When configuration changes cause issues:

```markdown
The recent config change may have caused this issue. Let me check...

[Compares before/after config, identifies misconfiguration]
```

### With /memory-setup

When setup was incomplete:

```markdown
It looks like setup didn't complete fully. Missing components:
- CCH hook not configured

Would you like me to complete the setup? Run `/memory-setup` to continue.
```
