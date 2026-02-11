# CLOD: Cross-Language Operation Definition

## Overview

CLOD is a TOML-based format for defining agent-memory commands in a platform-neutral way. A single CLOD file can be converted to adapter-specific files for Claude Code, OpenCode, Gemini CLI, and Copilot CLI.

CLOD eliminates the need to hand-maintain four separate command definitions per operation. Write once in CLOD, generate all adapter artifacts with `memory-daemon clod convert`.

## Format

A CLOD file is a valid TOML document with the following sections:

### `[command]` Section (Required)

Defines the command identity and its parameters.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | String | Yes | - | Command identifier in kebab-case (e.g., `"memory-search"`) |
| `description` | String | Yes | - | Human-readable description of the command |
| `version` | String | No | `"1.0.0"` | Semantic version of the command definition |

### `[[command.parameters]]` Sections (Required)

Each parameter is defined as a TOML array-of-tables entry.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | String | Yes | - | Parameter name (used in substitution) |
| `description` | String | Yes | - | What the parameter does |
| `required` | Boolean | Yes | - | Whether the parameter is mandatory |
| `position` | Integer | No | - | Positional argument index (0-based) |
| `flag` | String | No | - | CLI flag syntax (e.g., `"--period"`) |

Parameters with `position` are treated as positional arguments. Parameters with `flag` are treated as named flags. A parameter may have both (positional with optional flag override).

### `[process]` Section (Optional)

Defines execution steps for the command.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `steps` | Array of Strings | Yes | - | Execution steps in order |

Steps can include:
- CLI commands in backtick notation: `` `memory-daemon retrieval route "<query>"` ``
- Parameter substitution: `<param_name>` is replaced with the parameter value
- Prose instructions for the agent to follow

### `[output]` Section (Optional)

Defines output formatting.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `format` | String | Yes | - | Multi-line template for output formatting |

The format string can reference parameter names and result fields.

### `[adapters]` Section (Optional)

Per-adapter configuration for generation targets. Each sub-table configures one target.

| Sub-table | Field | Type | Default | Description |
|-----------|-------|------|---------|-------------|
| `[adapters.claude]` | `directory` | String | `"commands"` | Output directory relative to adapter root |
| | `extension` | String | `"md"` | File extension |
| `[adapters.opencode]` | `directory` | String | `"command"` | Output directory |
| | `extension` | String | `"md"` | File extension |
| `[adapters.gemini]` | `directory` | String | `"commands"` | Output directory |
| | `extension` | String | `"toml"` | File extension |
| `[adapters.copilot]` | `directory` | String | `"skills"` | Output directory |
| | `extension` | String | `"md"` | File extension |

## Examples

### Example 1: Memory Search Command

This defines the primary search command used across all adapters.

```toml
[command]
name = "memory-search"
description = "Search past conversations and memories using intelligent retrieval"
version = "1.0.0"

[[command.parameters]]
name = "query"
description = "Search query (natural language or keywords)"
required = true
position = 0

[[command.parameters]]
name = "period"
description = "Time period filter (e.g., 'last week', '2026-02-01')"
required = false
flag = "--period"

[[command.parameters]]
name = "agent"
description = "Filter by agent (claude, opencode, gemini, copilot)"
required = false
flag = "--agent"

[process]
steps = [
    "Parse the user's search query: <query>",
    "If --period is provided, apply time filter",
    "If --agent is provided, filter to that agent",
    "Run: `memory-daemon retrieval route \"<query>\" --agent <agent>`",
    "Present top results with source citations",
    "Offer to expand any grip for full context",
]

[output]
format = """
## Search Results for: <query>

Found {count} results:

{results}

Use /memory-context to expand any result for full conversation context.
"""

[adapters.claude]
directory = "commands"
extension = "md"

[adapters.opencode]
directory = "command"
extension = "md"

[adapters.gemini]
directory = "commands"
extension = "toml"

[adapters.copilot]
directory = "skills"
extension = "md"
```

### Example 2: Memory Recent Command

A simpler command for viewing recent activity.

```toml
[command]
name = "memory-recent"
description = "Show recent conversations and activity"
version = "1.0.0"

[[command.parameters]]
name = "count"
description = "Number of recent items to show"
required = false
flag = "--count"

[[command.parameters]]
name = "agent"
description = "Filter by agent (claude, opencode, gemini, copilot)"
required = false
flag = "--agent"

[process]
steps = [
    "Run: `memory-daemon query root` to get TOC root",
    "Navigate to the most recent time period",
    "If --agent is provided, filter to that agent",
    "Show the last <count> segments with summaries",
]

[output]
format = """
## Recent Activity

{segments}

Use /memory-search to search across all time periods.
"""
```

## Generated Output

When you run `memory-daemon clod convert --input memory-search.toml --target all --out ./adapters`, the converter generates one file per target:

### Claude Code Output (`commands/memory-search.md`)

```markdown
---
name: memory-search
description: Search past conversations and memories using intelligent retrieval
parameters:
  - name: query
    description: Search query (natural language or keywords)
    required: true
  - name: period
    description: "Time period filter (e.g., 'last week', '2026-02-01')"
    required: false
  - name: agent
    description: "Filter by agent (claude, opencode, gemini, copilot)"
    required: false
---

Search past conversations using intelligent retrieval.

## Process

1. Parse the user's search query: <query>
2. If --period is provided, apply time filter
3. If --agent is provided, filter to that agent
4. Run: `memory-daemon retrieval route "<query>" --agent <agent>`
5. Present top results with source citations
6. Offer to expand any grip for full context
```

### OpenCode Output (`command/memory-search.md`)

```markdown
---
name: memory-search
description: Search past conversations and memories using intelligent retrieval
---

Search past conversations using intelligent retrieval.

Arguments: $ARGUMENTS

## Parameters

- **query** (required): Search query (natural language or keywords)
- **period** (optional): Time period filter
- **agent** (optional): Filter by agent (claude, opencode, gemini, copilot)

## Process

1. Parse the user's search query from $ARGUMENTS
2. If --period is provided, apply time filter
3. If --agent is provided, filter to that agent
4. Run: `memory-daemon retrieval route "<query>" --agent <agent>`
5. Present top results with source citations
6. Offer to expand any grip for full context
```

### Gemini CLI Output (`commands/memory-search.toml`)

```toml
[prompt]
description = "Search past conversations and memories using intelligent retrieval"
command = """
Search past conversations using intelligent retrieval.

Arguments: {{args}}

Parameters:
- query (required): Search query (natural language or keywords)
- period (optional): Time period filter
- agent (optional): Filter by agent (claude, opencode, gemini, copilot)

Process:
1. Parse the user's search query from {{args}}
2. If --period is provided, apply time filter
3. If --agent is provided, filter to that agent
4. Run: `memory-daemon retrieval route "<query>" --agent <agent>`
5. Present top results with source citations
6. Offer to expand any grip for full context
"""
```

### Copilot CLI Output (`skills/memory-search/SKILL.md`)

```markdown
---
name: memory-search
description: Search past conversations and memories using intelligent retrieval
---

# Memory Search

Search past conversations using intelligent retrieval.

## Parameters

- **query** (required): Search query (natural language or keywords)
- **period** (optional): Time period filter
- **agent** (optional): Filter by agent (claude, opencode, gemini, copilot)

## Process

1. Parse the user's search query
2. If --period is provided, apply time filter
3. If --agent is provided, filter to that agent
4. Run: `memory-daemon retrieval route "<query>" --agent <agent>`
5. Present top results with source citations
6. Offer to expand any grip for full context
```

## Validation

Use `memory-daemon clod validate <file>` to check a CLOD definition for errors:

```bash
$ memory-daemon clod validate memory-search.toml
Valid CLOD definition: memory-search v1.0.0
  Parameters: 3 (1 required, 2 optional)
  Steps: 6
  Adapters: claude, opencode, gemini, copilot
```

Common validation errors:
- Missing `name` or `description` in `[command]`
- Parameter without `name` or `description`
- Duplicate parameter names
- Invalid TOML syntax

## Design Rationale

### Why TOML?

- Gemini CLI already uses `.toml` for command definitions
- TOML is more readable than JSON for multi-line strings
- TOML supports comments (unlike JSON)
- TOML is already used in the Rust ecosystem (Cargo.toml)

### Why Not YAML?

- Claude Code uses YAML frontmatter, but the CLOD body needs richer structure
- YAML's whitespace sensitivity causes subtle bugs
- TOML is unambiguous for the nested structures CLOD needs

### Adapter-Specific Differences

Each adapter has its own command format:

| Adapter | Format | Substitution | Notes |
|---------|--------|--------------|-------|
| Claude Code | Markdown + YAML frontmatter | Parameter names in frontmatter | Uses `parameters` YAML list |
| OpenCode | Markdown + YAML frontmatter | `$ARGUMENTS` | Single arguments string |
| Gemini CLI | TOML with `[prompt]` | `{{args}}` | Self-contained prompt |
| Copilot CLI | Markdown skill | Parameters in body | Uses skills, not commands |

The CLOD converter handles these differences automatically, generating idiomatic output for each target.

## CLI Reference

### Convert

```bash
memory-daemon clod convert --input <file.toml> --target <target> --out <dir>
```

| Flag | Description |
|------|-------------|
| `--input` | Path to CLOD definition file (.toml) |
| `--target` | Target adapter: `claude`, `opencode`, `gemini`, `copilot`, `all` |
| `--out` | Output directory (created if it does not exist) |

### Validate

```bash
memory-daemon clod validate <file.toml>
```

Validates the CLOD definition and reports any errors. Exits with code 0 on success, 1 on validation failure.
