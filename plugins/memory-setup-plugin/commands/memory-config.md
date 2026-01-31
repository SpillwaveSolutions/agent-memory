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
```

## Subcommands

### show

Display current configuration.

```
/memory-config show              # Show all config
/memory-config show storage      # Show storage section
/memory-config show summarizer   # Show summarizer section
```

### set

Set a configuration value.

```
/memory-config set <key> <value>
```

### reset

Reset a section to defaults.

```
/memory-config reset <section>
/memory-config reset all         # Reset entire config
```

## Configuration Keys

### storage

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `storage.path` | string | `~/.memory-store` | Data directory path |
| `storage.write_buffer_size_mb` | integer | 64 | Write buffer size |
| `storage.max_background_jobs` | integer | 4 | Compaction threads |

### server

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `server.host` | string | `[::1]` | Bind address |
| `server.port` | integer | 50051 | Listen port |
| `server.timeout_secs` | integer | 30 | Request timeout |

### summarizer

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `summarizer.provider` | string | `openai` | LLM provider |
| `summarizer.model` | string | `gpt-4o-mini` | Model name |
| `summarizer.max_tokens` | integer | 1024 | Max response tokens |
| `summarizer.temperature` | float | 0.3 | Sampling temperature |

### toc

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `toc.segment_min_tokens` | integer | 500 | Min tokens per segment |
| `toc.segment_max_tokens` | integer | 4000 | Max tokens per segment |
| `toc.time_gap_minutes` | integer | 30 | Gap to trigger boundary |

### logging

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `logging.level` | string | `info` | Log level |
| `logging.format` | string | `pretty` | Output format |

## Process

### show

```bash
# Read config file
cat ~/.config/memory-daemon/config.toml
```

Parse and display in structured format.

### set

1. **Validate key exists** in schema
2. **Validate value type** matches expected
3. **Read current config:**
   ```bash
   cat ~/.config/memory-daemon/config.toml
   ```
4. **Update value** in TOML
5. **Write back:**
   ```bash
   cat > ~/.config/memory-daemon/config.toml << 'EOF'
   [section]
   key = "value"
   EOF
   ```
6. **Notify about restart:**
   ```
   Configuration updated. Restart daemon to apply:
   memory-daemon stop && memory-daemon start
   ```

### reset

1. **Get default values** for section
2. **Replace section** in config file
3. **Write back**
4. **Notify about restart**

## Output Format

### show

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

---
Config file: `~/.config/memory-daemon/config.toml`
```

### set

```markdown
## Configuration Updated

**Key:** `summarizer.model`
**Old value:** `gpt-4o-mini`
**New value:** `gpt-4o`

Restart daemon to apply changes:
```bash
memory-daemon stop && memory-daemon start
```
```

### reset

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

Restart daemon to apply changes:
```bash
memory-daemon stop && memory-daemon start
```
```

## Validation Rules

| Key | Rule |
|-----|------|
| `storage.path` | Must be writable directory path |
| `server.port` | 1-65535, not in use |
| `server.host` | Valid IP or hostname |
| `summarizer.provider` | `openai` or `anthropic` |
| `summarizer.model` | Valid for chosen provider |
| `summarizer.temperature` | 0.0 - 1.0 |
| `logging.level` | `error`, `warn`, `info`, `debug`, `trace` |

## Error Handling

| Error | Resolution |
|-------|------------|
| Invalid key | Show valid keys for section |
| Invalid value type | Show expected type |
| Config file not found | Run `/memory-setup` first |
| Permission denied | Check file permissions |

## Examples

**View all configuration:**
```
/memory-config show
```

**View specific section:**
```
/memory-config show summarizer
```

**Change model:**
```
/memory-config set summarizer.model gpt-4o
```

**Change storage path:**
```
/memory-config set storage.path /data/memory
```

**Reset to defaults:**
```
/memory-config reset all
```
