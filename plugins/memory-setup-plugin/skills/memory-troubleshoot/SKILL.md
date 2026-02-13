---
name: memory-troubleshoot
description: |
  Diagnostic and troubleshooting workflow for agent-memory. Guides users through
  common install/config/daemon/ingest issues. Requires confirmation before any
  edits or restarts. Provides verification commands only.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Troubleshoot Skill

Diagnose and resolve common agent-memory issues safely. This skill never runs
verification commands automatically and always asks for confirmation before
edits or restarts.

## When to Use

- Daemon will not start
- Commands are not found
- Connection refused
- No events captured
- Summarization failures

## Safety Rules

- Do not edit files or restart services without confirmation
- Provide verification commands only
- Keep fixes scoped to the reported issue

## Diagnostic Flow

### Step 1: Identify the symptom

```
Which issue are you seeing?

1. "command not found" for memory-daemon or memory-ingest
2. "connection refused" or cannot query daemon
3. Daemon starts then stops
4. No events captured
5. Summarization not working
6. Other / not sure

Enter selection [1-6]:
```

### Step 2: Provide targeted diagnostics (commands only)

**1) Command not found**

```
which memory-daemon
which memory-ingest
echo $PATH
```

**2) Connection refused**

```
memory-daemon status
lsof -i :50051
memory-daemon query --endpoint http://[::1]:50051 root
```

**3) Daemon stops immediately**

```
memory-daemon status
tail -50 ~/Library/Logs/memory-daemon/daemon.log
tail -50 ~/.local/state/memory-daemon/daemon.log
```

**4) No events captured**

```
grep -n "memory-ingest" ~/.claude/hooks.yaml
which memory-ingest
memory-daemon status
```

**5) Summarization failing**

```
echo "OPENAI: ${OPENAI_API_KEY:+set}"
echo "ANTHROPIC: ${ANTHROPIC_API_KEY:+set}"
```

### Step 3: Offer safe fixes (with confirmation)

For each fix, ask:

```
I can apply the following fix:
  [describe the change]

Proceed? (yes/no)
```

Examples:

**PATH update**

```
Add ~/.local/bin to PATH in your shell profile
```

**Restart daemon**

```
Stop and restart the daemon:
  memory-daemon stop
  memory-daemon start
```

**Fix hooks file**

```
Add memory-ingest handler to ~/.claude/hooks.yaml
```

### Step 4: Provide verification commands (optional)

```
memory-daemon status
memory-daemon query --endpoint http://[::1]:50051 root
```

## Common Fix Playbooks

### "command not found"

- Ensure binaries are installed
- Add install directory to PATH
- If missing, re-run `memory-install`

### "connection refused"

- Confirm daemon is running
- Check port conflicts on 50051
- Update `grpc_port` in config if needed

### "no events captured"

- Confirm hooks file contains `memory-ingest`
- Confirm `memory-ingest` is on PATH
- Restart the agent session after hook changes

### "summarization not working"

- Set provider API key
- Ensure config provider matches key

## Escalation

If diagnostics do not resolve the issue, recommend:

- Re-run `memory-verify` for a full check
- Re-run `memory-install` with a fresh setup
