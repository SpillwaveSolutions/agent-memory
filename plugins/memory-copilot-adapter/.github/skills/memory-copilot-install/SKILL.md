---
name: memory-copilot-install
description: |
  Install and configure agent-memory integration for GitHub Copilot CLI. Use when asked to "install memory", "setup agent memory", "configure memory hooks", "enable memory capture", or "install copilot memory adapter". Automates hook configuration, skill deployment, agent setup, and verification.
license: MIT
metadata:
  version: 2.1.0
  author: SpillwaveSolutions
---

# Memory Copilot Install Skill

Automates setup of agent-memory integration for GitHub Copilot CLI. This skill copies the hook configuration file, hook handler script, deploys skills, copies the navigator agent, and verifies the installation.

**CRITICAL:** Unlike the Gemini adapter (which modifies settings.json), Copilot CLI hooks use standalone `.github/hooks/*.json` files. This skill copies the hook config file directly -- no JSON merging is required. Copilot CLI also does NOT support global hooks (`~/.copilot/hooks/`). Installation is per-project via `.github/hooks/` or via `/plugin install`.

## When Not to Use

- **Querying memories:** Use the memory-query skill or the memory-navigator agent instead (not this skill)
- **Claude Code setup:** Use the `memory-setup` plugin for Claude Code (not this skill)
- **OpenCode setup:** Use the OpenCode `memory-capture.ts` plugin (not this skill)
- **Gemini CLI setup:** Use the `memory-gemini-install` skill (not this skill)
- **Manual installation:** See the README.md for step-by-step manual instructions
- **Already installed:** If `.github/hooks/memory-hooks.json` already exists in the project, verify rather than re-install

## Overview

This skill performs a complete installation of the agent-memory Copilot CLI adapter. It:

1. Checks prerequisites (Copilot CLI, memory-daemon, memory-ingest, jq)
2. Determines install mode (per-project or plugin install)
3. Creates required directories
4. Copies the hook configuration file and hook handler script
5. Copies query skills
6. Copies the navigator agent
7. Verifies the installation
8. Reports results

## Step 1: Prerequisites Check

Check that all required tools are available. Warn for each missing prerequisite but allow continuing.

### Copilot CLI

```bash
command -v copilot >/dev/null 2>&1 && echo "FOUND: $(copilot --version 2>/dev/null || echo 'version unknown')" || echo "NOT FOUND"
```

If Copilot CLI is not found, warn:
> "Copilot CLI not found on PATH. Install from https://github.com/github/copilot-cli or via npm: `npm install -g @anthropic-ai/copilot-cli`. You may be running from a different context -- continuing anyway."

If found, check the version. Copilot CLI v0.0.383+ is required for hook support. v0.0.406+ is recommended for plugin support:

```bash
COPILOT_VERSION=$(copilot --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
echo "Copilot CLI version: $COPILOT_VERSION"
# v0.0.383+ required for hooks, v0.0.406+ recommended for plugin support
```

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

If jq is found, also check its version for `walk` support (needed for recursive redaction):

```bash
if ! jq -n 'walk(.)' >/dev/null 2>&1; then
  echo "NOTE: jq $(jq --version 2>&1) does not support walk(). The hook handler will use a simplified del()-based redaction filter (top level + one level deep). Consider upgrading to jq 1.6+ for full recursive redaction."
else
  echo "OK: jq supports walk() -- full recursive redaction available"
fi
```

### Summary

After checking all prerequisites, display a summary:

```
Prerequisites Check
-------------------
  Copilot CLI:   [FOUND/NOT FOUND] [version] [min v0.0.383, recommended v0.0.406+]
  memory-daemon: [FOUND/NOT FOUND] [version]
  memory-ingest: [FOUND/NOT FOUND] [version]
  jq:            [FOUND/NOT FOUND] [version] [walk() support: YES/NO]
```

## Step 2: Determine Install Mode

Two installation modes are available:

### Per-Project (Default)

Copy files to `.github/` in the current project directory. Hooks only fire when Copilot CLI runs in THIS project directory.

### Plugin Install

User runs `/plugin install /path/to/adapter` from within Copilot CLI. The plugin system auto-discovers hooks, skills, and agents from the adapter directory structure.

Ask the user which mode they prefer. Default to per-project.

**Note:** Copilot CLI does NOT support global hooks at `~/.copilot/hooks/` (Issue #1157 is open). There is no global install option. For multi-project coverage, either:
- Use plugin install (convenient, installs once)
- Run the per-project install in each project

## Step 3: Create Directories (Per-Project Mode)

Create the required directories under `.github/` in the current project:

```bash
mkdir -p .github/hooks/scripts
mkdir -p .github/skills
mkdir -p .github/agents
```

Confirm each directory exists after creation:

```bash
[ -d .github/hooks/scripts ] && echo "OK: .github/hooks/scripts" || echo "FAIL: .github/hooks/scripts"
[ -d .github/skills ] && echo "OK: .github/skills" || echo "FAIL: .github/skills"
[ -d .github/agents ] && echo "OK: .github/agents" || echo "FAIL: .github/agents"
```

## Step 4: Copy Hook Files

Determine the source path of the adapter files. The adapter is located at the path where this skill was loaded from. Look for the hook files relative to the skill directory:

```
<skill-root>/../../hooks/memory-hooks.json
<skill-root>/../../hooks/scripts/memory-capture.sh
```

Where `<skill-root>` is the directory containing this SKILL.md. The adapter `.github/` root is two directories up from the skill directory (`.github/skills/memory-copilot-install/` -> `.github/`).

### Copy hook configuration

```bash
# Determine adapter root (adjust ADAPTER_ROOT based on where files are located)
# If installed from the agent-memory repository:
ADAPTER_ROOT="<path-to-agent-memory>/plugins/memory-copilot-adapter/.github"

# Copy hook configuration file
cp "$ADAPTER_ROOT/hooks/memory-hooks.json" .github/hooks/memory-hooks.json
```

**IMPORTANT:** Do NOT modify settings.json. Copilot CLI hooks use standalone `.github/hooks/*.json` files. The hook configuration is a self-contained JSON file, not a merge target.

### Copy hook handler script

```bash
# Copy hook handler script
cp "$ADAPTER_ROOT/hooks/scripts/memory-capture.sh" .github/hooks/scripts/memory-capture.sh

# Make executable
chmod +x .github/hooks/scripts/memory-capture.sh
```

### Verify hook files

```bash
# Verify hook config exists and is valid JSON
[ -f .github/hooks/memory-hooks.json ] && jq empty .github/hooks/memory-hooks.json 2>/dev/null && echo "OK: Hook config is valid JSON" || echo "FAIL: Hook config missing or invalid"

# Verify hook script exists and is executable
[ -x .github/hooks/scripts/memory-capture.sh ] && echo "OK: Hook script is executable" || echo "FAIL: Hook script missing or not executable"
```

## Step 5: Copy Skills

Copy all skill directories from the adapter to the project's `.github/skills/` directory, EXCLUDING the install skill itself (no need to install the installer):

```bash
# Copy query and retrieval skills
cp -r "$ADAPTER_ROOT/skills/memory-query" .github/skills/
cp -r "$ADAPTER_ROOT/skills/retrieval-policy" .github/skills/
cp -r "$ADAPTER_ROOT/skills/topic-graph" .github/skills/
cp -r "$ADAPTER_ROOT/skills/bm25-search" .github/skills/
cp -r "$ADAPTER_ROOT/skills/vector-search" .github/skills/
```

Note: The `memory-copilot-install` skill is NOT copied to the target project. It is only needed during installation.

Verify:

```bash
[ -f .github/skills/memory-query/SKILL.md ] && echo "OK: memory-query" || echo "FAIL: memory-query"
[ -f .github/skills/retrieval-policy/SKILL.md ] && echo "OK: retrieval-policy" || echo "FAIL: retrieval-policy"
[ -f .github/skills/topic-graph/SKILL.md ] && echo "OK: topic-graph" || echo "FAIL: topic-graph"
[ -f .github/skills/bm25-search/SKILL.md ] && echo "OK: bm25-search" || echo "FAIL: bm25-search"
[ -f .github/skills/vector-search/SKILL.md ] && echo "OK: vector-search" || echo "FAIL: vector-search"
```

## Step 6: Copy Navigator Agent

Copy the navigator agent to the project's `.github/agents/` directory:

```bash
cp "$ADAPTER_ROOT/agents/memory-navigator.agent.md" .github/agents/memory-navigator.agent.md
```

Verify:

```bash
[ -f .github/agents/memory-navigator.agent.md ] && echo "OK: Navigator agent copied" || echo "FAIL: Navigator agent missing"
```

## Step 7: Verify Installation

Run a comprehensive verification of the entire installation:

### Hook configuration

```bash
# Check hook config exists and has the expected event types
if [ -f .github/hooks/memory-hooks.json ]; then
  EVENTS=$(jq -r '.hooks | keys[]' .github/hooks/memory-hooks.json 2>/dev/null || echo "")
  for event in sessionStart sessionEnd userPromptSubmitted preToolUse postToolUse; do
    if echo "$EVENTS" | grep -q "$event"; then
      echo "PASS: $event hook configured"
    else
      echo "FAIL: $event hook missing"
    fi
  done
else
  echo "FAIL: Hook config file missing"
fi
```

### Hook script

```bash
[ -x .github/hooks/scripts/memory-capture.sh ] && echo "PASS: Hook script executable" || echo "FAIL: Hook script missing or not executable"
```

### Skills

```bash
for skill in memory-query retrieval-policy topic-graph bm25-search vector-search; do
  [ -f ".github/skills/${skill}/SKILL.md" ] && echo "PASS: ${skill} skill" || echo "FAIL: ${skill} skill missing"
done
```

### Navigator agent

```bash
[ -f .github/agents/memory-navigator.agent.md ] && echo "PASS: Navigator agent" || echo "FAIL: Navigator agent missing"
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
  Agent Memory - Copilot CLI Adapter Installation
==================================================

Hook Config:    [PASS/FAIL] (.github/hooks/memory-hooks.json)
Hook Script:    [PASS/FAIL] (.github/hooks/scripts/memory-capture.sh)
Skills:         [PASS/FAIL] (5 query skills in .github/skills/)
Navigator:      [PASS/FAIL] (.github/agents/memory-navigator.agent.md)
Daemon:         [RUNNING/NOT RUNNING/NOT INSTALLED]

Installed Files:
  .github/hooks/memory-hooks.json
  .github/hooks/scripts/memory-capture.sh
  .github/skills/memory-query/SKILL.md
  .github/skills/retrieval-policy/SKILL.md
  .github/skills/topic-graph/SKILL.md
  .github/skills/bm25-search/SKILL.md
  .github/skills/vector-search/SKILL.md
  .github/agents/memory-navigator.agent.md

Warnings:
  [list any missing prerequisites]

Important Notes:
  - Per-project installation means hooks only fire in THIS
    project directory. For other projects, re-run the install
    skill or use `/plugin install`.
  - AssistantResponse events are not captured (Copilot CLI
    does not provide this hook). SubagentStart/SubagentStop
    are also not available.
  - sessionStart may fire per-prompt in interactive mode
    (Bug #991). The hook handler reuses session IDs to
    handle this gracefully.

Next Steps:
  1. Ensure memory-daemon is running: memory-daemon start
  2. Start a new Copilot CLI session in this project
  3. Verify events are captured: memory-daemon query root
  4. Search with agent filter: memory-daemon retrieval route "topic" --agent copilot
```

## Plugin Install Alternative

Instead of per-project installation, users can install the adapter as a Copilot CLI plugin. This is convenient for users who want memory capture without copying files into each project.

### Install via plugin system

From within Copilot CLI, run:

```
/plugin install /path/to/plugins/memory-copilot-adapter
```

Or from a GitHub repository:

```
/plugin install https://github.com/SpillwaveSolutions/agent-memory/tree/main/plugins/memory-copilot-adapter
```

The plugin system auto-discovers:
- `.github/hooks/memory-hooks.json` -- Hook configuration
- `.github/skills/*/SKILL.md` -- All skills (including this install skill)
- `.github/agents/*.agent.md` -- Navigator agent
- `plugin.json` -- Plugin metadata

### Advantages of plugin install

- **One command:** Single `/plugin install` sets up everything
- **Auto-discovery:** Hooks, skills, and agents are found automatically
- **Updates:** `/plugin update` can pull new versions
- **No file copying:** Files stay in the plugin directory

### Limitations of plugin install

- Requires Copilot CLI v0.0.406+ (plugin support)
- Plugin-provided hooks require v0.0.402+
- Less transparent than seeing files in `.github/`

## Uninstall

To remove the agent-memory Copilot CLI integration from a project:

### Remove installed files (per-project)

```bash
# Remove hook files
rm -f .github/hooks/memory-hooks.json
rm -f .github/hooks/scripts/memory-capture.sh
rmdir .github/hooks/scripts 2>/dev/null
# Only remove hooks dir if empty (other hooks may exist)
rmdir .github/hooks 2>/dev/null

# Remove skills
rm -rf .github/skills/memory-query
rm -rf .github/skills/retrieval-policy
rm -rf .github/skills/topic-graph
rm -rf .github/skills/bm25-search
rm -rf .github/skills/vector-search

# Remove navigator agent
rm -f .github/agents/memory-navigator.agent.md
# Only remove agents dir if empty (other agents may exist)
rmdir .github/agents 2>/dev/null
```

### Remove plugin (if installed via /plugin)

```
/plugin uninstall memory-copilot-adapter
```

### Clean up session temp files

```bash
rm -f /tmp/copilot-memory-session-*
```

### Verify uninstall

```bash
[ ! -f .github/hooks/memory-hooks.json ] && echo "OK: Hook config removed"
[ ! -f .github/hooks/scripts/memory-capture.sh ] && echo "OK: Hook script removed"
[ ! -d .github/skills/memory-query ] && echo "OK: Skills removed"
[ ! -f .github/agents/memory-navigator.agent.md ] && echo "OK: Navigator removed"
```

Note: Uninstalling the Copilot adapter does NOT remove the memory-daemon, memory-ingest binaries, or any stored conversation data. Those are managed separately.
