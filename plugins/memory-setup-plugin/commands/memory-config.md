---
name: memory-config
description: View and modify agent-memory configuration
parameters:
  - name: action
    description: Action to perform (show, set, reset)
    required: true
  - name: key
    description: Configuration key to modify (for set/reset)
    required: false
  - name: value
    description: New value (for set)
    required: false
skills:
  - memory-setup
---

# Memory Config

View and modify agent-memory configuration without editing files directly.

## Usage

```
/memory-config show
/memory-config show storage
/memory-config set storage.path ~/.my-memory
/memory-config set summarizer.model gpt-4o
/memory-config reset summarizer
/memory-config reset all
```

## Subcommands

### show

Display current configuration, optionally filtered by section.

```
/memory-config show              # Show all config
/memory-config show storage      # Show storage section
/memory-config show summarizer   # Show summarizer section
/memory-config show server       # Show server section
/memory-config show toc          # Show TOC section
/memory-config show logging      # Show logging section
```

**Process:**

```bash
# Read config file
CONFIG_FILE=~/.config/memory-daemon/config.toml
if [ ! -f "$CONFIG_FILE" ]; then
  echo "Config file not found. Run /memory-setup first."
  exit 1
fi

# Display full config or section
cat "$CONFIG_FILE"
# If section specified, filter to that section
```

**Output:**

```markdown
## Memory Configuration

### storage

| Key | Value |
|-----|-------|
| path | ~/.memory-store |
| write_buffer_size_mb | 64 |
| max_background_jobs | 4 |

### server

| Key | Value |
|-----|-------|
| host | [::1] |
| port | 50051 |
| timeout_secs | 30 |

### summarizer

| Key | Value |
|-----|-------|
| provider | openai |
| model | gpt-4o-mini |
| max_tokens | 1024 |
| temperature | 0.3 |

### toc

| Key | Value |
|-----|-------|
| segment_min_tokens | 500 |
| segment_max_tokens | 4000 |
| time_gap_minutes | 30 |

### logging

| Key | Value |
|-----|-------|
| level | info |
| format | pretty |

---
Config file: `~/.config/memory-daemon/config.toml`
```

### set

Set a configuration value with validation.

```
/memory-config set <key> <value>
```

**Process:**

1. **Parse key** - Extract section and field (e.g., `storage.path` -> section=`storage`, field=`path`)
2. **Validate key exists** - Check against schema
3. **Validate value type** - Ensure value matches expected type
4. **Validate value** - Run validation rules
5. **Read current config** - Parse TOML
6. **Update value** - Modify in memory
7. **Write config** - Save TOML file
8. **Check if restart needed** - Some values require daemon restart
9. **Report success** - Show old/new values

```bash
# Read current config
CONFIG_FILE=~/.config/memory-daemon/config.toml

# Validate key format (section.field)
if [[ ! "$KEY" =~ ^[a-z]+\.[a-z_]+$ ]]; then
  echo "Invalid key format. Use: section.field (e.g., storage.path)"
  exit 1
fi

# Parse section and field
SECTION=$(echo "$KEY" | cut -d. -f1)
FIELD=$(echo "$KEY" | cut -d. -f2)

# Get current value (for comparison)
CURRENT=$(grep -A20 "^\[$SECTION\]" "$CONFIG_FILE" | grep "^$FIELD" | cut -d'=' -f2 | tr -d ' "')

# Update config file (using sed or full rewrite)
# ... TOML manipulation logic ...

# Determine if restart needed
case "$KEY" in
  storage.path|server.host|server.port)
    NEEDS_RESTART=true
    ;;
  *)
    NEEDS_RESTART=false
    ;;
esac
```

**Output:**

```markdown
## Configuration Updated

**Key:** `summarizer.model`
**Old value:** `gpt-4o-mini`
**New value:** `gpt-4o`

Config file: `~/.config/memory-daemon/config.toml`

**Note:** This change takes effect immediately for new requests.
```

Or with restart requirement:

```markdown
## Configuration Updated

**Key:** `server.port`
**Old value:** `50051`
**New value:** `50052`

Config file: `~/.config/memory-daemon/config.toml`

**Restart Required**

This change requires a daemon restart to take effect:

```bash
memory-daemon stop && memory-daemon start
```
```

### reset

Reset a section or all config to defaults. Requires confirmation.

```
/memory-config reset <section>
/memory-config reset all
```

**Process:**

1. **Confirm with user** - Destructive operation requires explicit confirmation
2. **Get default values** - From embedded defaults
3. **Replace section** - Or entire file for `all`
4. **Write config** - Save TOML file
5. **Report success** - Show new default values

**Confirmation prompt:**

```markdown
## Confirm Reset

Are you sure you want to reset the `summarizer` section to defaults?

**Current values:**
| Key | Current | Default |
|-----|---------|---------|
| provider | anthropic | openai |
| model | claude-3-5-haiku-latest | gpt-4o-mini |
| max_tokens | 2048 | 1024 |
| temperature | 0.5 | 0.3 |

Type "yes" to confirm, or anything else to cancel.
```

**Output after confirmation:**

```markdown
## Configuration Reset

**Section:** `summarizer`

Reset to defaults:

| Key | Value |
|-----|-------|
| provider | openai |
| model | gpt-4o-mini |
| max_tokens | 1024 |
| temperature | 0.3 |

Config file: `~/.config/memory-daemon/config.toml`

**Restart Required**

Restart daemon to apply changes:

```bash
memory-daemon stop && memory-daemon start
```
```

## Configuration Schema

### storage

| Key | Type | Default | Validation | Description |
|-----|------|---------|------------|-------------|
| `storage.path` | string | `~/.memory-store` | Must be writable directory | Data directory path |
| `storage.write_buffer_size_mb` | integer | 64 | 1-256 | Write buffer size in MB |
| `storage.max_background_jobs` | integer | 4 | 1-16 | Compaction thread count |

### server

| Key | Type | Default | Validation | Description |
|-----|------|---------|------------|-------------|
| `server.host` | string | `[::1]` | Valid IP or hostname | Bind address |
| `server.port` | integer | 50051 | 1-65535, not in use | Listen port |
| `server.timeout_secs` | integer | 30 | 1-300 | Request timeout |

### summarizer

| Key | Type | Default | Validation | Description |
|-----|------|---------|------------|-------------|
| `summarizer.provider` | string | `openai` | `openai`, `anthropic`, `local` | LLM provider |
| `summarizer.model` | string | `gpt-4o-mini` | Valid for provider | Model name |
| `summarizer.max_tokens` | integer | 1024 | 1-8192 | Max response tokens |
| `summarizer.temperature` | float | 0.3 | 0.0-1.0 | Sampling temperature |

### toc

| Key | Type | Default | Validation | Description |
|-----|------|---------|------------|-------------|
| `toc.segment_min_tokens` | integer | 500 | 100-2000 | Min tokens per segment |
| `toc.segment_max_tokens` | integer | 4000 | 1000-16000 | Max tokens per segment |
| `toc.time_gap_minutes` | integer | 30 | 5-180 | Gap to trigger boundary |

### logging

| Key | Type | Default | Validation | Description |
|-----|------|---------|------------|-------------|
| `logging.level` | string | `info` | `error`, `warn`, `info`, `debug`, `trace` | Log level |
| `logging.format` | string | `pretty` | `pretty`, `json`, `compact` | Output format |

## Validation Rules

### Type Validation

| Type | Rule | Example |
|------|------|---------|
| string | Non-empty, no control chars | `gpt-4o` |
| integer | Numeric, within range | `50051` |
| float | Decimal, within range | `0.3` |
| enum | One of allowed values | `openai` |

### Value Validation

| Key | Rule | Error Message |
|-----|------|---------------|
| `storage.path` | Directory writable or creatable | "Path not writable: {path}" |
| `server.port` | Not in use (if daemon stopped) | "Port {port} already in use" |
| `server.host` | Valid IP/hostname | "Invalid host: {host}" |
| `summarizer.provider` | `openai`, `anthropic`, `local` | "Invalid provider. Choose: openai, anthropic, local" |
| `summarizer.model` | Valid for provider | "Model '{model}' not valid for {provider}" |
| `summarizer.temperature` | 0.0-1.0 | "Temperature must be between 0.0 and 1.0" |
| `logging.level` | Valid level | "Invalid level. Choose: error, warn, info, debug, trace" |

### Provider-Model Validation

| Provider | Valid Models |
|----------|--------------|
| openai | gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-3.5-turbo |
| anthropic | claude-3-5-sonnet-latest, claude-3-5-haiku-latest, claude-3-opus-latest |
| local | * (any model name accepted) |

## Side Effects

Some configuration changes have side effects:

| Key | Side Effect | Action |
|-----|-------------|--------|
| `storage.path` | Requires restart, may need data migration | Warn user, offer migration |
| `server.host` | Requires restart | Show restart command |
| `server.port` | Requires restart | Show restart command |
| `summarizer.provider` | Requires matching API key | Check environment variable |
| `summarizer.model` | May affect API costs | Show model info link |
| `logging.level` | Immediate (if using config reload) | Note: debug creates large logs |

### Restart Required Matrix

| Section | Field | Restart Required |
|---------|-------|------------------|
| storage | path | Yes |
| storage | write_buffer_size_mb | Yes |
| storage | max_background_jobs | Yes |
| server | host | Yes |
| server | port | Yes |
| server | timeout_secs | No |
| summarizer | * | No |
| toc | * | No |
| logging | * | No (hot reload) |

## Error Handling

| Error | Condition | Resolution |
|-------|-----------|------------|
| Config file not found | No config.toml | Run `/memory-setup` first |
| Invalid key | Key not in schema | Show valid keys for section |
| Invalid value type | String for integer, etc. | Show expected type |
| Validation failed | Value out of range | Show validation rules |
| Permission denied | Can't write config | Check file permissions |
| Parse error | Malformed TOML | Offer to reset section |

### Error Output Format

```markdown
## Configuration Error

**Key:** `summarizer.temperature`
**Value:** `1.5`
**Error:** Temperature must be between 0.0 and 1.0

**Valid range:** 0.0 to 1.0
**Current value:** 0.3
```

## Default Configuration

Full default config.toml:

```toml
# Memory Daemon Configuration
# Generated by /memory-setup

[storage]
path = "~/.memory-store"
write_buffer_size_mb = 64
max_background_jobs = 4

[server]
host = "[::1]"
port = 50051
timeout_secs = 30

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
max_tokens = 1024
temperature = 0.3

[toc]
segment_min_tokens = 500
segment_max_tokens = 4000
time_gap_minutes = 30

[logging]
level = "info"
format = "pretty"
```

## Examples

### View all configuration

```
/memory-config show
```

### View specific section

```
/memory-config show summarizer
```

Output:
```markdown
## Memory Configuration: summarizer

| Key | Value |
|-----|-------|
| provider | openai |
| model | gpt-4o-mini |
| max_tokens | 1024 |
| temperature | 0.3 |
```

### Change LLM model

```
/memory-config set summarizer.model gpt-4o
```

### Change to Anthropic provider

```
/memory-config set summarizer.provider anthropic
/memory-config set summarizer.model claude-3-5-haiku-latest
```

### Change storage path

```
/memory-config set storage.path /data/memory
```

Output:
```markdown
## Configuration Updated

**Key:** `storage.path`
**Old value:** `~/.memory-store`
**New value:** `/data/memory`

**Warning:** Data Migration Required

Your existing data is in `~/.memory-store`. To migrate:

1. Stop the daemon: `memory-daemon stop`
2. Copy data: `cp -r ~/.memory-store/* /data/memory/`
3. Start daemon: `memory-daemon start`

Or start fresh with an empty database.
```

### Change server port

```
/memory-config set server.port 50052
```

### Reset summarizer to defaults

```
/memory-config reset summarizer
```

### Reset entire configuration

```
/memory-config reset all
```

### Enable debug logging

```
/memory-config set logging.level debug
```

Output:
```markdown
## Configuration Updated

**Key:** `logging.level`
**Old value:** `info`
**New value:** `debug`

**Note:** Debug logging creates verbose output. Remember to set back to `info` after troubleshooting.

This change takes effect immediately.
```

## Integration Notes

### With /memory-status

After config changes, suggest running status check:

```markdown
Configuration updated. Run `/memory-status` to verify changes took effect.
```

### With /memory-setup

If config file doesn't exist:

```markdown
Configuration file not found.

Run `/memory-setup` to create initial configuration, or create manually at:
`~/.config/memory-daemon/config.toml`
```

### With setup-troubleshooter

When validation fails repeatedly, suggest troubleshooter:

```markdown
Multiple configuration issues detected. Would you like me to run diagnostics?

Say "troubleshoot" or "fix it" to start automated diagnosis.
```
