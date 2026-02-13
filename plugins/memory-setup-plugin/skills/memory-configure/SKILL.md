---
name: memory-configure
description: |
  Wizard-style configuration guidance for a single-agent agent-memory setup.
  Shows defaults inline, provides a full sample config, and includes a dry-run
  config check step. Always confirm before creating or editing files.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Configure Skill

Guide users through configuring agent-memory for a single-agent setup. This
skill does not apply multi-agent configuration. Always ask for confirmation
before creating or editing configuration files. Provide verification commands
only; do not run them.

## When to Use

- User needs a first-time config file
- User wants to reset to defaults
- User wants to update the summarizer provider/model

## When Not to Use

- Installation (use `memory-install`)
- Verification checks (use `memory-verify`)
- Troubleshooting failures (use `memory-troubleshoot`)

## Wizard Principles

- Ask one question at a time
- Confirm before any file creation or edit
- Provide verification commands only
- Keep scope to single-agent defaults

## Defaults (single-agent)

- `db_path`: `~/.local/share/agent-memory/db`
- `grpc_port`: `50051`
- `grpc_host`: `0.0.0.0`
- `log_level`: `info`
- `summarizer.provider`: `openai`
- `summarizer.model`: `gpt-4o-mini`

## Wizard Flow

### Step 1: Check for existing config

```
Do you already have a config file at:
  ~/.config/agent-memory/config.toml

1. Yes
2. No

Enter selection [1-2]:
```

If yes, ask if they want to keep it, overwrite, or edit.

### Step 2: Choose summarizer provider

```
Which summarizer provider should be used?

1. OpenAI (gpt-4o-mini)
2. Anthropic (claude-3-5-haiku-latest)
3. Local (Ollama or compatible)
4. None (disable summarization)

Default: 1

Enter selection [1-4]:
```

If provider needs a key, remind user to set the environment variable:

```
OpenAI:   export OPENAI_API_KEY="sk-..."
Anthropic: export ANTHROPIC_API_KEY="sk-ant-..."
```

### Step 3: Show the full sample config

```
# ~/.config/agent-memory/config.toml

db_path = "~/.local/share/agent-memory/db"
grpc_port = 50051
grpc_host = "0.0.0.0"
log_level = "info"
multi_agent_mode = "separate"

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

### Step 4: Confirm file creation or edit

```
I can create or update:
  ~/.config/agent-memory/config.toml

Proceed? (yes/no)
```

If yes, ask whether to create directories as needed, then proceed.

### Step 5: Dry-run config check (optional)

Provide command only:

```
Optional verify command:
  memory-daemon config check
```

### Step 6: Next steps

- Use `memory-verify` for health checks
- Start the daemon with `memory-daemon start`
- Configure agent hooks using the agent-specific setup guide

## Confirmation Language

Any time a file change is requested, ask explicitly:

```
This will edit ~/.config/agent-memory/config.toml.
Proceed? (yes/no)
```

If the user says no, provide the commands and stop.
