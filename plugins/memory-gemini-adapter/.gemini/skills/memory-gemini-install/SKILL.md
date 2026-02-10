---
name: memory-gemini-install
description: |
  Install and configure agent-memory integration for Gemini CLI. Use when asked to "install memory", "setup agent memory", "configure memory hooks", "enable memory capture", or "install gemini memory adapter". Automates hook configuration, skill deployment, and verification.
license: MIT
metadata:
  version: 2.1.0
  author: SpillwaveSolutions
---

# Memory Gemini Install Skill

Automates setup of agent-memory integration for Gemini CLI. This skill copies the hook handler script, merges hook configuration into settings.json, deploys commands and skills, and verifies the installation.

## When Not to Use

- **Querying memories:** Use `/memory-search`, `/memory-recent`, or `/memory-context` commands instead
- **Claude Code setup:** Use the `memory-setup` plugin for Claude Code (not this skill)
- **OpenCode setup:** Use the OpenCode `memory-capture.ts` plugin (not this skill)
- **Manual installation:** See the README.md for step-by-step manual instructions

## Overview

This skill performs a complete installation of the agent-memory Gemini CLI adapter. It:

1. Checks prerequisites (Gemini CLI, memory-daemon, memory-ingest, jq)
2. Creates required directories
3. Copies the hook handler script
4. Merges hook configuration into settings.json (preserving existing settings)
5. Copies slash commands
6. Copies query skills
7. Verifies the installation
8. Reports results

## Step 1: Prerequisites Check

Check that all required tools are available. Warn for each missing prerequisite but allow continuing.

### Gemini CLI

```bash
command -v gemini >/dev/null 2>&1 && echo "FOUND: $(gemini --version 2>/dev/null || echo 'version unknown')" || echo "NOT FOUND"
```

If Gemini CLI is not found, warn:
> "Gemini CLI not found on PATH. Install from https://github.com/google-gemini/gemini-cli or via npm: `npm install -g @google/gemini-cli`. You may be running from a different context -- continuing anyway."

If found, check the version. Gemini CLI requires hook support (available in versions with the hooks system). Parse the version output and verify it is recent enough to support the `hooks` feature in `settings.json`.

### memory-daemon

```bash
command -v memory-daemon >/dev/null 2>&1 && echo "FOUND: $(memory-daemon --version 2>/dev/null || echo 'version unknown')" || echo "NOT FOUND"
```

If not found, warn:
> "memory-daemon not found on PATH. Install from https://github.com/SpillwaveSolutions/agent-memory. Events will not be stored until memory-daemon is installed and running."

### memory-ingest

```bash
command -v memory-ingest >/dev/null 2>&1 && echo "FOUND: $(memory-ingest --version 2>/dev/null || echo 'version unknown')" || echo "NOT FOUND"
```

If not found, warn:
> "memory-ingest not found on PATH. Install from https://github.com/SpillwaveSolutions/agent-memory. Hook events will be silently dropped until memory-ingest is available."

### jq

```bash
command -v jq >/dev/null 2>&1 && echo "FOUND: $(jq --version 2>/dev/null || echo 'version unknown')" || echo "NOT FOUND"
```

If not found, warn:
> "jq not found on PATH. The hook handler requires jq for JSON processing. Install via: `brew install jq` (macOS), `apt install jq` (Debian/Ubuntu), `dnf install jq` (Fedora), or download from https://jqlang.github.io/jq/."

**CRITICAL:** jq is required for the hook handler to function. If jq is missing, display a prominent warning that event capture will not work until jq is installed.

### Summary

After checking all prerequisites, display a summary:

```
Prerequisites Check
-------------------
  Gemini CLI:    [FOUND/NOT FOUND] [version]
  memory-daemon: [FOUND/NOT FOUND] [version]
  memory-ingest: [FOUND/NOT FOUND] [version]
  jq:            [FOUND/NOT FOUND] [version]
```

## Step 2: Create Directories

Create the required directories under `~/.gemini/` for global installation:

```bash
mkdir -p ~/.gemini/hooks
mkdir -p ~/.gemini/commands
mkdir -p ~/.gemini/skills
```

Confirm each directory exists after creation:

```bash
[ -d ~/.gemini/hooks ] && echo "OK: ~/.gemini/hooks" || echo "FAIL: ~/.gemini/hooks"
[ -d ~/.gemini/commands ] && echo "OK: ~/.gemini/commands" || echo "FAIL: ~/.gemini/commands"
[ -d ~/.gemini/skills ] && echo "OK: ~/.gemini/skills" || echo "FAIL: ~/.gemini/skills"
```

## Step 3: Copy Hook Handler Script

Determine the source path of the adapter files. The adapter is located at the path where this skill was loaded from. Look for the `memory-capture.sh` file relative to the skill directory:

```
<skill-root>/../../hooks/memory-capture.sh
```

Where `<skill-root>` is the directory containing this SKILL.md. The adapter root is two directories up from the skill directory (`.gemini/skills/memory-gemini-install/` -> `.gemini/`).

Copy the hook handler:

```bash
# Determine adapter root (adjust ADAPTER_ROOT based on where files are located)
# If installed from the agent-memory repository:
ADAPTER_ROOT="<path-to-agent-memory>/plugins/memory-gemini-adapter/.gemini"

# Copy hook handler
cp "$ADAPTER_ROOT/hooks/memory-capture.sh" ~/.gemini/hooks/memory-capture.sh

# Make executable
chmod +x ~/.gemini/hooks/memory-capture.sh
```

Verify the copy:

```bash
[ -x ~/.gemini/hooks/memory-capture.sh ] && echo "OK: Hook script copied and executable" || echo "FAIL: Hook script not found or not executable"
```

## Step 4: Merge Hook Configuration into settings.json

**CRITICAL: Do NOT overwrite existing settings.json. MERGE hook entries into the existing configuration.**

This is the most important step. The user may have existing Gemini CLI configuration that must be preserved.

### Merge Strategy

Read the existing `~/.gemini/settings.json` (or start with `{}` if it does not exist), then merge the memory-capture hook entries into the `hooks` section.

```bash
# Read existing settings or start empty
EXISTING=$(cat ~/.gemini/settings.json 2>/dev/null || echo '{}')

# Define the hook configuration to merge
HOOK_CONFIG='{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "name": "memory-capture-session-start",
            "type": "command",
            "command": "$HOME/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture session start into agent-memory"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "name": "memory-capture-session-end",
            "type": "command",
            "command": "$HOME/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture session end into agent-memory"
          }
        ]
      }
    ],
    "BeforeAgent": [
      {
        "hooks": [
          {
            "name": "memory-capture-user-prompt",
            "type": "command",
            "command": "$HOME/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture user prompts into agent-memory"
          }
        ]
      }
    ],
    "AfterAgent": [
      {
        "hooks": [
          {
            "name": "memory-capture-assistant-response",
            "type": "command",
            "command": "$HOME/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture assistant responses into agent-memory"
          }
        ]
      }
    ],
    "BeforeTool": [
      {
        "matcher": "*",
        "hooks": [
          {
            "name": "memory-capture-pre-tool-use",
            "type": "command",
            "command": "$HOME/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture tool usage into agent-memory"
          }
        ]
      }
    ],
    "AfterTool": [
      {
        "matcher": "*",
        "hooks": [
          {
            "name": "memory-capture-post-tool-result",
            "type": "command",
            "command": "$HOME/.gemini/hooks/memory-capture.sh",
            "timeout": 5000,
            "description": "Capture tool results into agent-memory"
          }
        ]
      }
    ]
  }
}'

# Extract hooks from the config template
HOOKS=$(echo "$HOOK_CONFIG" | jq '.hooks')

# Merge hooks into existing settings
# This uses jq's * operator for recursive merge:
# - Existing non-hook settings are preserved
# - Existing hooks for OTHER events are preserved
# - Memory-capture hooks are added/replaced for the 6 event types
echo "$EXISTING" | jq --argjson hooks "$HOOKS" '
  .hooks = ((.hooks // {}) * $hooks)
' > ~/.gemini/settings.json
```

### Validate the merge result

```bash
# Ensure the result is valid JSON
jq . ~/.gemini/settings.json > /dev/null 2>&1 && echo "OK: settings.json is valid JSON" || echo "FAIL: settings.json is invalid JSON"

# Verify memory-capture hooks are present
jq '.hooks | keys[]' ~/.gemini/settings.json 2>/dev/null
```

Expected output should include: `SessionStart`, `SessionEnd`, `BeforeAgent`, `AfterAgent`, `BeforeTool`, `AfterTool`.

### Important Notes

- The `*` merge operator in jq performs recursive merge. For hooks arrays, this replaces the entire array for each event type. If the user had OTHER hooks on the same events, they would need to be manually re-added. This is acceptable because hook arrays are typically managed per-tool.
- The `$HOME` variable in command paths is expanded at runtime by Gemini CLI (which supports environment variable expansion in settings.json strings).
- A backup of the original settings.json is recommended before merging. Create one:
  ```bash
  cp ~/.gemini/settings.json ~/.gemini/settings.json.bak 2>/dev/null || true
  ```

## Step 5: Copy Commands

Copy all TOML command files from the adapter to the global commands directory:

```bash
# Copy command files
cp "$ADAPTER_ROOT/commands/memory-search.toml" ~/.gemini/commands/
cp "$ADAPTER_ROOT/commands/memory-recent.toml" ~/.gemini/commands/
cp "$ADAPTER_ROOT/commands/memory-context.toml" ~/.gemini/commands/
```

Verify:

```bash
ls ~/.gemini/commands/memory-*.toml 2>/dev/null && echo "OK: Commands copied" || echo "FAIL: No command files found"
```

## Step 6: Copy Skills

Copy all skill directories from the adapter to the global skills directory, EXCLUDING the install skill itself (no need to install the installer globally):

```bash
# Copy query and retrieval skills
cp -r "$ADAPTER_ROOT/skills/memory-query" ~/.gemini/skills/
cp -r "$ADAPTER_ROOT/skills/retrieval-policy" ~/.gemini/skills/
cp -r "$ADAPTER_ROOT/skills/topic-graph" ~/.gemini/skills/
cp -r "$ADAPTER_ROOT/skills/bm25-search" ~/.gemini/skills/
cp -r "$ADAPTER_ROOT/skills/vector-search" ~/.gemini/skills/
```

Note: The `memory-gemini-install` skill is NOT copied to the global directory. It is only needed during installation.

Verify:

```bash
[ -f ~/.gemini/skills/memory-query/SKILL.md ] && echo "OK: memory-query" || echo "FAIL: memory-query"
[ -f ~/.gemini/skills/retrieval-policy/SKILL.md ] && echo "OK: retrieval-policy" || echo "FAIL: retrieval-policy"
[ -f ~/.gemini/skills/topic-graph/SKILL.md ] && echo "OK: topic-graph" || echo "FAIL: topic-graph"
[ -f ~/.gemini/skills/bm25-search/SKILL.md ] && echo "OK: bm25-search" || echo "FAIL: bm25-search"
[ -f ~/.gemini/skills/vector-search/SKILL.md ] && echo "OK: vector-search" || echo "FAIL: vector-search"
```

## Step 7: Verify Installation

Run a comprehensive verification of the entire installation:

### Hook script

```bash
[ -x ~/.gemini/hooks/memory-capture.sh ] && echo "PASS: Hook script executable" || echo "FAIL: Hook script missing or not executable"
```

### Settings.json hooks

```bash
# Check that all 6 event types have memory-capture hooks
for event in SessionStart SessionEnd BeforeAgent AfterAgent BeforeTool AfterTool; do
  if jq -e ".hooks.${event}" ~/.gemini/settings.json >/dev/null 2>&1; then
    echo "PASS: $event hook configured"
  else
    echo "FAIL: $event hook missing"
  fi
done
```

### Commands

```bash
for cmd in memory-search memory-recent memory-context; do
  [ -f ~/.gemini/commands/${cmd}.toml ] && echo "PASS: ${cmd} command" || echo "FAIL: ${cmd} command missing"
done
```

### Skills

```bash
for skill in memory-query retrieval-policy topic-graph bm25-search vector-search; do
  [ -f ~/.gemini/skills/${skill}/SKILL.md ] && echo "PASS: ${skill} skill" || echo "FAIL: ${skill} skill missing"
done
```

### Daemon connectivity (optional)

If memory-daemon is available, test connectivity:

```bash
if command -v memory-daemon >/dev/null 2>&1; then
  memory-daemon status 2>/dev/null && echo "PASS: Daemon running" || echo "INFO: Daemon not running (start with: memory-daemon start)"
fi
```

## Step 8: Report Results

Present a complete installation report:

```
==================================================
  Agent Memory - Gemini CLI Adapter Installation
==================================================

Hook Script:    [PASS/FAIL]
Settings.json:  [PASS/FAIL] (6 event hooks configured)
Commands:       [PASS/FAIL] (3 TOML commands)
Skills:         [PASS/FAIL] (5 query skills)
Daemon:         [RUNNING/NOT RUNNING/NOT INSTALLED]

Installed Files:
  ~/.gemini/hooks/memory-capture.sh
  ~/.gemini/settings.json (hooks merged)
  ~/.gemini/commands/memory-search.toml
  ~/.gemini/commands/memory-recent.toml
  ~/.gemini/commands/memory-context.toml
  ~/.gemini/skills/memory-query/SKILL.md
  ~/.gemini/skills/retrieval-policy/SKILL.md
  ~/.gemini/skills/topic-graph/SKILL.md
  ~/.gemini/skills/bm25-search/SKILL.md
  ~/.gemini/skills/vector-search/SKILL.md

Warnings:
  [list any missing prerequisites]

Next Steps:
  1. Start a new Gemini CLI session to activate hooks
  2. Use /memory-search <topic> to search past conversations
  3. Use /memory-recent to see recent activity
  4. Verify events are being captured after a session

Note: SubagentStart/SubagentStop events have no Gemini CLI
equivalent. This is a trivial gap -- all core conversation
events (prompts, responses, tools, sessions) are captured.
```

## Uninstall

To remove the agent-memory Gemini CLI integration, run these commands:

### Remove installed files

```bash
# Remove hook script
rm -f ~/.gemini/hooks/memory-capture.sh

# Remove commands
rm -f ~/.gemini/commands/memory-search.toml
rm -f ~/.gemini/commands/memory-recent.toml
rm -f ~/.gemini/commands/memory-context.toml

# Remove skills
rm -rf ~/.gemini/skills/memory-query
rm -rf ~/.gemini/skills/retrieval-policy
rm -rf ~/.gemini/skills/topic-graph
rm -rf ~/.gemini/skills/bm25-search
rm -rf ~/.gemini/skills/vector-search
```

### Remove hook configuration from settings.json

Use jq to remove only the memory-capture hook entries, preserving all other settings:

```bash
# Backup first
cp ~/.gemini/settings.json ~/.gemini/settings.json.bak

# Remove memory-capture hooks from each event type
# This removes hook entries where the name starts with "memory-capture"
jq '
  .hooks |= (if . then
    with_entries(
      .value |= [
        .[] | .hooks |= [.[] | select(.name | startswith("memory-capture") | not)]
      ] | [.[] | select(.hooks | length > 0)]
    ) | if . == {} then null else . end
  else . end)
' ~/.gemini/settings.json > ~/.gemini/settings.json.tmp \
  && mv ~/.gemini/settings.json.tmp ~/.gemini/settings.json
```

If settings.json becomes empty after removing hooks, you can safely delete it:

```bash
# Check if settings.json only contains hooks (nothing else to preserve)
REMAINING=$(jq 'del(.hooks) | length' ~/.gemini/settings.json 2>/dev/null || echo "0")
if [ "$REMAINING" = "0" ]; then
  rm -f ~/.gemini/settings.json
  echo "settings.json removed (was only hook configuration)"
else
  echo "settings.json retained (has non-hook settings)"
fi
```

### Verify uninstall

```bash
[ ! -f ~/.gemini/hooks/memory-capture.sh ] && echo "OK: Hook script removed"
[ ! -f ~/.gemini/commands/memory-search.toml ] && echo "OK: Commands removed"
[ ! -d ~/.gemini/skills/memory-query ] && echo "OK: Skills removed"
```

Note: Uninstalling the Gemini adapter does NOT remove the memory-daemon, memory-ingest binaries, or any stored conversation data. Those are managed separately.
