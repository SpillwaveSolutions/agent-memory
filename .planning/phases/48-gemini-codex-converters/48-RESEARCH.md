# Phase 48: Gemini & Codex Converters - Research

**Researched:** 2026-03-17
**Domain:** Rust converter implementations for Gemini CLI (TOML commands, settings.json hooks) and Codex CLI (SKILL.md directories, AGENTS.md)
**Confidence:** HIGH

## Summary

Phase 48 fills in the stub implementations of `GeminiConverter` and `CodexConverter` in the `memory-installer` crate. Phase 46 established all infrastructure (RuntimeConverter trait, ConvertedFile type, write_files with dry-run, merge_managed_section, tool_maps::map_tool, parser producing PluginBundle). Phase 47 established the converter pattern with ClaudeConverter as the reference implementation, including shared helpers (reconstruct_md, rewrite_paths, value_to_yaml) in `converters/helpers.rs`.

The Gemini converter has unique requirements: commands use TOML format (not YAML-frontmatter Markdown), agents have no separate file format (Gemini embeds agent logic into skill SKILL.md files), tool names use snake_case, MCP/Task tools are excluded, shell variables `${VAR}` must be escaped to `$VAR`, and hook definitions merge into `.gemini/settings.json`. The Codex converter converts commands to skill directories (each with SKILL.md), agents to orchestration skills, generates an AGENTS.md from metadata, and maps sandbox permissions per agent.

The `toml` crate is already in Cargo.toml dependencies. The existing Gemini adapter in `plugins/memory-gemini-adapter/` provides a reference for the target output format (TOML commands, settings.json hooks, SKILL.md skills). No existing Codex adapter exists, but the Codex documentation provides clear SKILL.md format and AGENTS.md conventions.

**Primary recommendation:** Implement both converters following the Phase 47 pattern (pure functions over serde_json::Value frontmatter). Add a `value_to_toml` helper in `converters/helpers.rs` for Gemini command serialization. Use `serde_json` for settings.json hook merge via the existing `merge_managed_section` infrastructure (JSON variant). For Codex, convert commands to SKILL.md format and generate AGENTS.md as a ConvertedFile from generate_guidance.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| GEM-01 | Command frontmatter converted from YAML to TOML format | Use `toml` crate (already in Cargo.toml) to serialize; Gemini commands are `.toml` files with `description` and `prompt` fields |
| GEM-02 | Agent `allowed-tools:` converted to `tools:` array with Gemini snake_case names | Map through `tool_maps::map_tool(Runtime::Gemini, ...)` for snake_case names; embed tool list in skill SKILL.md body since Gemini has no separate agent file |
| GEM-03 | MCP and Task tools excluded from converted output | `map_tool(Runtime::Gemini, "Task")` returns `None`; callers skip `mcp__*` prefixed tools before calling map_tool |
| GEM-04 | `color:` and `skills:` fields stripped from agent frontmatter | When converting agents to Gemini format, omit these fields entirely |
| GEM-05 | Shell variable `${VAR}` escaped to `$VAR` | Regex or string replace `${` to `$` in body content; Gemini uses `${VAR}` for its own template syntax |
| GEM-06 | Hook definitions merged into `.gemini/settings.json` using managed-section markers | Use JSON managed-section pattern; merge hooks object into existing settings.json preserving user content |
| CDX-01 | Commands converted to Codex skill directories (each command becomes a SKILL.md) | Each command becomes `skills/<cmd-name>/SKILL.md` with YAML frontmatter (name, description) and prompt body |
| CDX-02 | Agents converted to orchestration skill directories | Each agent becomes a skill directory with agent body as orchestration instructions |
| CDX-03 | `AGENTS.md` generated from agent metadata for project-level Codex guidance | `generate_guidance` produces AGENTS.md listing all agents, their triggers, and skill references |
| CDX-04 | Sandbox permissions mapped per agent (workspace-write vs read-only) | memory-navigator: read-only (query only); setup-troubleshooter: workspace-write (may modify config files) |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_json | workspace | Frontmatter manipulation, settings.json generation | Already parsed by gray_matter |
| toml | workspace | TOML serialization for Gemini commands | Already in Cargo.toml dependencies |
| gray_matter | 0.3 | YAML frontmatter parsing (read-only, in parser) | Already used by Phase 46 |
| walkdir | workspace | Directory traversal for skills | Already used by Phase 46 |
| anyhow | workspace | Error handling | Project standard |
| tracing | workspace | Warnings for unmapped tools | Project standard |
| directories | workspace | Platform-specific config paths | Already used for target_dir |
| shellexpand | 3.1 | Tilde expansion | Already used for target_dir |
| tempfile | workspace | Test temp directories | Already in dev-dependencies |

### No New Dependencies Required
All needed crates are already in `crates/memory-installer/Cargo.toml`. The `toml` crate handles Gemini TOML serialization. The `serde_json` crate handles settings.json generation.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `toml` crate for TOML output | `format!` macro | `toml` handles quoting and escaping correctly; already a dependency |
| JSON managed-section for settings.json | Custom JSON merge logic | Managed-section markers already tested in writer.rs |
| AGENTS.md as ConvertedFile | Separate file-writing logic | ConvertedFile pattern keeps all output in one pipeline |

## Architecture Patterns

### Recommended Module Structure
```
crates/memory-installer/src/
  converters/
    gemini.rs          # GeminiConverter impl (fill stubs)
    codex.rs           # CodexConverter impl (fill stubs)
    helpers.rs         # Add value_to_toml, escape_shell_vars helpers
    claude.rs          # Reference implementation (unchanged)
    opencode.rs        # (unchanged)
    mod.rs             # select_converter (unchanged)
  converter.rs         # RuntimeConverter trait (unchanged)
  types.rs             # ConvertedFile, etc. (unchanged)
  writer.rs            # write_files, merge_managed_section (unchanged)
  tool_maps.rs         # map_tool (unchanged) -- Gemini/Codex mappings already present
```

### Pattern 1: Gemini TOML Command Conversion
**What:** Convert canonical YAML-frontmatter Markdown commands to Gemini `.toml` format with `description` and `prompt` fields.
**When to use:** GEM-01 requirement.
**Example:**
```rust
use toml::Value as TomlValue;

fn convert_command_to_toml(cmd: &PluginCommand) -> String {
    let mut table = toml::map::Map::new();

    // Extract description from frontmatter
    if let Some(desc) = cmd.frontmatter.get("description").and_then(|v| v.as_str()) {
        table.insert("description".to_string(), TomlValue::String(desc.to_string()));
    }

    // Body becomes the prompt field (with shell var escaping)
    let body = escape_shell_vars(&cmd.body);
    let body = rewrite_paths(&body, "~/.claude/", "~/.config/agent-memory/");
    table.insert("prompt".to_string(), TomlValue::String(body));

    toml::to_string_pretty(&TomlValue::Table(table))
        .unwrap_or_default()
}
```

**Target format (from existing adapter):**
```toml
description = "Search past conversations by topic or keyword using agent-memory"

prompt = """
Search past conversations by topic or keyword...
"""
```

### Pattern 2: Gemini Shell Variable Escaping (GEM-05)
**What:** Replace `${VAR}` with `$VAR` in body content because Gemini CLI uses `${...}` for its own template substitution syntax.
**When to use:** All Gemini content conversion.
**Example:**
```rust
/// Escape shell variables from ${VAR} to $VAR for Gemini compatibility.
/// Gemini CLI uses ${...} as template substitution syntax, so shell-style
/// ${VAR} would be interpreted as a template variable.
fn escape_shell_vars(content: &str) -> String {
    // Replace ${...} patterns that look like shell variables
    // but NOT Gemini template syntax like {{args}}
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            // Skip the '{', find closing '}'
            chars.next(); // consume '{'
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next(); // consume '}'
                    break;
                }
                var_name.push(c);
                chars.next();
            }
            // Emit as $VAR (without braces)
            result.push('$');
            result.push_str(&var_name);
        } else {
            result.push(ch);
        }
    }
    result
}
```

### Pattern 3: Gemini settings.json Hook Merge (GEM-06)
**What:** Merge hook definitions into `.gemini/settings.json` using JSON managed-section pattern.
**When to use:** GEM-06 requirement -- hooks need to coexist with user settings.
**Important:** The existing Gemini adapter uses a full settings.json with hooks. The converter should produce hook JSON that can be merged into existing settings.json without clobbering user settings.
**Example:**
```rust
fn generate_guidance(&self, bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);

    // Build settings.json hooks section
    let hooks = build_gemini_hooks(&target);
    let settings = serde_json::json!({
        MANAGED_JSON_KEY: MANAGED_JSON_VALUE,
        "hooks": hooks
    });

    let content = serde_json::to_string_pretty(&settings).unwrap();
    vec![ConvertedFile {
        target_path: target.join("settings.json"),
        content,
    }]
}
```

**Note:** The writer.rs `merge_managed_section` uses text-based markers which work for line-oriented config files. For JSON files like settings.json, the approach should be: if the file exists, parse it as JSON, merge the hooks key, and write back. If it does not exist, write the full managed JSON. This is a design decision -- either (a) use text-based managed markers embedded as JSON comments (the `_comment` field pattern from the existing adapter), or (b) treat the entire settings.json as a ConvertedFile and warn if user has existing settings. Option (a) is recommended since it preserves user content.

### Pattern 4: Codex Command-to-Skill Conversion (CDX-01)
**What:** Convert each canonical command to a Codex skill directory with SKILL.md.
**When to use:** CDX-01 requirement.
**Example:**
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let skill_dir = target_dir.join("skills").join(&cmd.name);

    // Build SKILL.md with Codex-compatible frontmatter
    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), serde_json::Value::String(cmd.name.clone()));
    if let Some(desc) = cmd.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    let body = rewrite_paths(&cmd.body, "~/.claude/", "~/.config/agent-memory/");
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: skill_dir.join("SKILL.md"),
        content,
    }]
}
```

### Pattern 5: Codex AGENTS.md Generation (CDX-03)
**What:** Generate AGENTS.md from agent metadata for project-level guidance.
**When to use:** CDX-03 requirement.
**Example:**
```rust
fn generate_guidance(&self, bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);

    let mut content = String::from("# Agent Memory - Codex Agents\n\n");
    content.push_str("This file was generated by memory-installer. ");
    content.push_str("It provides agent-memory context to Codex.\n\n");

    for agent in &bundle.agents {
        if let Some(desc) = agent.frontmatter.get("description").and_then(|v| v.as_str()) {
            content.push_str(&format!("## {}\n\n", agent.name));
            content.push_str(&format!("{}\n\n", desc));

            // Add sandbox permission guidance
            let sandbox = sandbox_for_agent(&agent.name);
            content.push_str(&format!("**Sandbox:** `{}`\n\n", sandbox));
        }
    }

    vec![ConvertedFile {
        target_path: target.join("AGENTS.md"),
        content,
    }]
}

fn sandbox_for_agent(name: &str) -> &'static str {
    match name {
        "setup-troubleshooter" => "workspace-write",
        _ => "read-only",
    }
}
```

### Pattern 6: Gemini Agent-to-Skill Embedding
**What:** Gemini does not have separate agent definitions. Agent logic is embedded into skill SKILL.md files (see existing memory-query/SKILL.md which has "Navigator Mode" embedded). For Gemini, `convert_agent` either (a) returns empty (agent logic already in skill), or (b) generates a supplementary SKILL.md that wraps the agent's body as a skill.
**When to use:** GEM-02, GEM-03, GEM-04 requirements.
**Recommendation:** Since the canonical agent body contains orchestration instructions, convert agents to Gemini skill directories. The agent becomes a skill where the body is the instruction content, and tool references are mapped to snake_case names. Strip `color:` and `skills:` from frontmatter, exclude MCP/Task tools.

### Anti-Patterns to Avoid
- **Writing raw TOML strings with format!:** Use the `toml` crate for serialization -- it handles quoting, multi-line strings, and escaping correctly.
- **Overwriting settings.json without merge:** Always merge hooks into existing settings.json to preserve user configuration.
- **Including MCP/Task tools in Gemini output:** `map_tool(Gemini, "Task")` returns None by design. Callers must check for `mcp__*` prefix before calling map_tool.
- **Hardcoding Gemini hook event names:** Use the existing adapter's settings.json as reference -- event names are PascalCase (SessionStart, BeforeAgent, etc.).
- **Generating Codex config.toml:** Codex configuration is user-managed. The installer only generates SKILL.md files and AGENTS.md.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML serialization | format! string building | `toml::to_string_pretty` | Handles multi-line strings, quoting, escaping |
| Tool name mapping | Inline match in converter | `tool_maps::map_tool(Runtime::Gemini/Codex, ...)` | Centralized, tested, exhaustive |
| File writing + dry-run | Per-converter write logic | `writer::write_files(&files, dry_run)` | Already tested |
| JSON config merge | Custom settings.json parser | `serde_json::from_str` + merge + `to_string_pretty` | Robust JSON handling |
| YAML frontmatter reconstruction | Custom serializer | `helpers::reconstruct_md` | Already tested in Phase 47 |
| Path rewriting | Multiple replace calls | `helpers::rewrite_paths` | Already tested |
| Shell var escaping | Simple string replace | Dedicated `escape_shell_vars` function | Must handle edge cases (nested braces, non-var patterns) |

**Key insight:** Phase 46 built all infrastructure, Phase 47 established converter patterns. Phase 48 follows the same pattern with Gemini-specific (TOML, settings.json merge, shell var escaping) and Codex-specific (command-to-skill, AGENTS.md generation, sandbox mapping) transformations.

## Common Pitfalls

### Pitfall 1: TOML Multi-line String Quoting
**What goes wrong:** Gemini command prompts are long multi-line strings. If serialized incorrectly, TOML parsing breaks.
**Why it happens:** TOML uses `"""..."""` for multi-line basic strings. The `toml` crate handles this automatically with `toml::to_string_pretty`.
**How to avoid:** Use the `toml` crate for serialization, not format! macros.
**Warning signs:** Generated .toml files fail to parse or have garbled content.

### Pitfall 2: Shell Variable Escaping Scope (GEM-05)
**What goes wrong:** Over-aggressive escaping replaces Gemini template syntax `{{args}}` or under-escaping leaves `${HOME}` which Gemini interprets as a template variable.
**Why it happens:** Both `${VAR}` (shell) and `{{args}}` (Gemini templates) appear in command content.
**How to avoid:** Only escape `${...}` patterns (single braces), not `{{...}}` (double braces). The existing adapter uses `$HOME` not `${HOME}` in hook scripts, confirming the escaping need.
**Warning signs:** `{{args}}` template variables stop working, or `$HOME` resolves incorrectly.

### Pitfall 3: Gemini settings.json Clobbering
**What goes wrong:** Writing a fresh settings.json destroys user's existing Gemini configuration (theme, model preferences, etc.).
**Why it happens:** The settings.json file contains both hooks and other user settings.
**How to avoid:** Read existing settings.json if present, merge only the `hooks` key, preserve all other user settings. If file does not exist, create with just the hooks section plus managed marker.
**Warning signs:** User reports lost Gemini settings after running installer.

### Pitfall 4: Codex SKILL.md vs Command Format Confusion
**What goes wrong:** Codex SKILL.md files use YAML frontmatter (name + description), but Gemini commands use TOML (description + prompt). Mixing up the formats.
**Why it happens:** Both are target formats for the same canonical source, but Codex is YAML-frontmatter Markdown while Gemini is pure TOML.
**How to avoid:** Gemini converter uses `toml::to_string_pretty` for commands. Codex converter uses `reconstruct_md` for SKILL.md. Keep the serialization logic clearly separated per converter.
**Warning signs:** Codex generates TOML files or Gemini generates Markdown files.

### Pitfall 5: The `unimplemented_converters_return_empty_results` Test
**What goes wrong:** The existing test in `converter.rs` (line 78-105) asserts that Gemini and Codex converters return empty results. Filling in stubs will break this test.
**Why it happens:** Phase 46 added the test to verify stubs were in place.
**How to avoid:** Update the test to only check Copilot and Skills (which remain stubs). Or replace with specific positive tests.
**Warning signs:** `cargo test` fails immediately on the stub assertion.

### Pitfall 6: Gemini Agent Handling -- No Separate Agent Format
**What goes wrong:** Attempting to produce `.md` agent files for Gemini when Gemini has no agent file concept.
**Why it happens:** Claude and OpenCode have `agents/` directories. Gemini does not -- it embeds agent behavior in skill SKILL.md files.
**How to avoid:** For Gemini, `convert_agent` should produce a skill directory (wrapping the agent body as a SKILL.md), or return empty if the agent logic is already embedded in a canonical skill (as the memory-query skill already contains Navigator Mode).
**Warning signs:** Files generated to non-existent Gemini directory structure that Gemini CLI cannot discover.

### Pitfall 7: Codex Sandbox Permissions Scope
**What goes wrong:** Setting wrong sandbox level for agents (e.g., giving read-only to setup-troubleshooter which needs to write files).
**Why it happens:** Sandbox permissions are per-agent configuration, not file metadata.
**How to avoid:** Document sandbox recommendations in AGENTS.md rather than trying to generate config.toml entries. Codex config.toml is user-managed. The AGENTS.md can include guidance like "setup-troubleshooter requires workspace-write sandbox".
**Warning signs:** Agents fail with permission errors when running.

## Code Examples

### Gemini Command TOML Conversion (GEM-01)
```rust
// Source: Existing adapter plugins/memory-gemini-adapter/.gemini/commands/memory-search.toml
// Target format:
// description = "Search past conversations..."
// prompt = """
// Search past conversations by topic or keyword...
// """

fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);

    let desc = cmd.frontmatter.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or(&cmd.name);

    let body = escape_shell_vars(&cmd.body);
    let body = rewrite_paths(&body, GEMINI_PATH_FROM, GEMINI_PATH_TO);

    let mut table = toml::map::Map::new();
    table.insert("description".to_string(), toml::Value::String(desc.to_string()));
    table.insert("prompt".to_string(), toml::Value::String(body));

    let content = toml::to_string_pretty(&toml::Value::Table(table))
        .unwrap_or_default();

    vec![ConvertedFile {
        target_path: target_dir.join("commands").join(format!("{}.toml", cmd.name)),
        content,
    }]
}
```

### Gemini Agent to Skill Conversion (GEM-02, GEM-03, GEM-04)
```rust
fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let skill_dir = target_dir.join("skills").join(&agent.name);

    // Build SKILL.md frontmatter -- strip color: and skills: (GEM-04)
    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), json!(agent.name));
    if let Some(desc) = agent.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    // Build tools list with Gemini names, excluding MCP/Task (GEM-02, GEM-03)
    let tools = build_gemini_tools(agent);
    if !tools.is_empty() {
        // Embed as metadata note in body rather than frontmatter
        // since Gemini skills don't have a tools frontmatter field
    }

    let body = escape_shell_vars(&agent.body);
    let body = rewrite_paths(&body, GEMINI_PATH_FROM, GEMINI_PATH_TO);
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: skill_dir.join("SKILL.md"),
        content,
    }]
}

fn build_gemini_tools(agent: &PluginAgent) -> Vec<String> {
    // Extract tool names from agent body or known agent tool requirements
    // Map through tool_maps::map_tool(Runtime::Gemini, ...) and collect Some values
    let mut tools = Vec::new();
    for tool_name in KNOWN_TOOLS {
        if tool_name.starts_with("mcp__") {
            continue; // MCP tools excluded for Gemini
        }
        if let Some(mapped) = map_tool(Runtime::Gemini, tool_name) {
            tools.push(mapped.to_string());
        }
    }
    tools
}
```

### Shell Variable Escaping (GEM-05)
```rust
/// Escape `${VAR}` to `$VAR` for Gemini compatibility.
/// Gemini CLI uses `${...}` for template substitution,
/// so shell-style `${HOME}` would conflict.
/// Does NOT touch `{{args}}` (Gemini's own template syntax).
pub fn escape_shell_vars(content: &str) -> String {
    // Simple regex-free approach: find ${ and replace with $
    // then strip the matching }
    let mut result = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // Find matching }
            result.push('$');
            i += 2; // skip ${
            while i < bytes.len() && bytes[i] != b'}' {
                result.push(bytes[i] as char);
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // skip }
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}
```

### Codex Command-to-Skill (CDX-01)
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let skill_dir = target_dir.join("skills").join(&cmd.name);

    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), json!(cmd.name));
    if let Some(desc) = cmd.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    let body = rewrite_paths(&cmd.body, "~/.claude/", "~/.config/agent-memory/");
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: skill_dir.join("SKILL.md"),
        content,
    }]
}
```

### Codex AGENTS.md Generation (CDX-03, CDX-04)
```rust
fn generate_guidance(&self, bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);
    let mut md = String::new();

    md.push_str("# Agent Memory\n\n");
    md.push_str("Memory plugin for cross-session conversation recall.\n\n");
    md.push_str("## Available Skills\n\n");

    for cmd in &bundle.commands {
        let desc = cmd.frontmatter.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description");
        md.push_str(&format!("- **{}**: {}\n", cmd.name, desc));
    }

    md.push_str("\n## Agents\n\n");
    for agent in &bundle.agents {
        let desc = agent.frontmatter.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description");
        let sandbox = sandbox_for_agent(&agent.name);
        md.push_str(&format!("### {}\n\n", agent.name));
        md.push_str(&format!("{}\n\n", desc));
        md.push_str(&format!("**Recommended sandbox:** `{}`\n\n", sandbox));
    }

    vec![ConvertedFile {
        target_path: target.join("AGENTS.md"),
        content: md,
    }]
}
```

### Gemini settings.json Hook Merge (GEM-06)
```rust
fn merge_gemini_hooks(target_dir: &Path) -> serde_json::Value {
    let hook_script = target_dir.join("hooks").join("memory-capture.sh");
    let cmd = format!("$HOME/.gemini/hooks/memory-capture.sh");

    let hook_entry = |name: &str, desc: &str| -> serde_json::Value {
        json!({
            "hooks": [{
                "name": name,
                "type": "command",
                "command": cmd,
                "timeout": 5000,
                "description": desc
            }]
        })
    };

    json!({
        "SessionStart": [hook_entry("memory-capture-session-start",
            "Capture session start into agent-memory")],
        "SessionEnd": [hook_entry("memory-capture-session-end",
            "Capture session end into agent-memory")],
        "BeforeAgent": [hook_entry("memory-capture-user-prompt",
            "Capture user prompts into agent-memory")],
        "AfterAgent": [hook_entry("memory-capture-assistant-response",
            "Capture assistant responses into agent-memory")],
        "BeforeTool": [{
            "matcher": "*",
            "hooks": [{
                "name": "memory-capture-pre-tool-use",
                "type": "command",
                "command": cmd,
                "timeout": 5000,
                "description": "Capture tool invocations into agent-memory"
            }]
        }],
        "AfterTool": [{
            "matcher": "*",
            "hooks": [{
                "name": "memory-capture-post-tool-result",
                "type": "command",
                "command": cmd,
                "timeout": 5000,
                "description": "Capture tool results into agent-memory"
            }]
        }]
    })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual Gemini adapter in `plugins/memory-gemini-adapter/` | Auto-generated by `memory-installer` | Phase 48 (now) | Single canonical source, automated TOML conversion |
| No Codex adapter | Auto-generated SKILL.md + AGENTS.md | Phase 48 (now) | Codex users get skill directories from canonical source |
| YAML-only command format | TOML for Gemini, YAML for Codex | Phase 48 | Each runtime gets its native format |

**Deprecated/outdated:**
- `plugins/memory-gemini-adapter/`: Will be replaced by installer output. Archive (not delete) per project decision.
- Manual TOML command files: Installer auto-generates from canonical YAML source.

## Open Questions

1. **Gemini Agent Handling Strategy**
   - What we know: Gemini has no separate agent file format. The existing adapter embeds Navigator Mode directly into the memory-query skill SKILL.md.
   - What's unclear: Should `convert_agent` produce a separate skill directory for each agent, or should it return empty (since agent logic is already in canonical skills)?
   - Recommendation: Produce a supplementary skill directory for each agent. The canonical skills already have the detailed instructions, but a thin skill wrapping the agent metadata ensures Gemini can discover the agent capability. If the agent name matches an existing skill (e.g., memory-navigator -> memory-query), consider merging.

2. **Gemini settings.json Deep Merge vs Replace**
   - What we know: settings.json contains hooks AND other user settings (theme, model, etc.). The existing adapter writes the entire file.
   - What's unclear: Whether to deep-merge the `hooks` key or replace the entire file.
   - Recommendation: If settings.json exists, parse it, merge only the `hooks` key (adding/replacing memory-capture hooks), and write back preserving other user settings. If it does not exist, create with just the hooks section. Use `serde_json` for parse/merge/write.

3. **Codex config.toml Sandbox Configuration**
   - What we know: Codex uses `sandbox_mode` in config.toml for per-agent permissions. The installer should not modify config.toml (user-managed).
   - What's unclear: Whether sandbox guidance belongs in AGENTS.md or as a separate configuration file.
   - Recommendation: Include sandbox recommendations in the generated AGENTS.md as human-readable guidance. Do not auto-generate config.toml entries.

4. **Hook Script Copying for Gemini**
   - What we know: The existing adapter includes `.gemini/hooks/memory-capture.sh`. The canonical source will need hook definitions (deferred to Phase 49 per CANON-02).
   - What's unclear: Should Phase 48 copy the hook script, or wait for Phase 49?
   - Recommendation: `convert_hook` returns None (hooks deferred to Phase 49). The settings.json hook merge in `generate_guidance` can reference the hook script path but the actual script copying is Phase 49 scope.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `tempfile` |
| Config file | `crates/memory-installer/Cargo.toml` (dev-dependencies) |
| Quick run command | `cargo test -p memory-installer` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GEM-01 | Command frontmatter -> TOML format | unit | `cargo test -p memory-installer converters::gemini::tests::command_to_toml -x` | No - Wave 0 |
| GEM-02 | Agent tools -> snake_case Gemini names | unit | `cargo test -p memory-installer converters::gemini::tests::agent_tool_names -x` | No - Wave 0 |
| GEM-03 | MCP/Task tools excluded | unit | `cargo test -p memory-installer converters::gemini::tests::mcp_task_excluded -x` | No - Wave 0 |
| GEM-04 | color/skills fields stripped | unit | `cargo test -p memory-installer converters::gemini::tests::fields_stripped -x` | No - Wave 0 |
| GEM-05 | Shell var ${VAR} -> $VAR escaping | unit | `cargo test -p memory-installer converters::helpers::tests::shell_var_escaping -x` | No - Wave 0 |
| GEM-06 | Hook definitions merged into settings.json | unit | `cargo test -p memory-installer converters::gemini::tests::settings_json_merge -x` | No - Wave 0 |
| CDX-01 | Commands -> Codex skill directories | unit | `cargo test -p memory-installer converters::codex::tests::command_to_skill -x` | No - Wave 0 |
| CDX-02 | Agents -> orchestration skill dirs | unit | `cargo test -p memory-installer converters::codex::tests::agent_to_skill -x` | No - Wave 0 |
| CDX-03 | AGENTS.md generated from metadata | unit | `cargo test -p memory-installer converters::codex::tests::agents_md_generation -x` | No - Wave 0 |
| CDX-04 | Sandbox permissions per agent | unit | `cargo test -p memory-installer converters::codex::tests::sandbox_mapping -x` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`
- **Phase gate:** `task pr-precheck` (format + clippy + test + doc)

### Wave 0 Gaps
- [ ] `converters/gemini.rs` tests -- unit tests for all GEM-* converter methods
- [ ] `converters/codex.rs` tests -- unit tests for all CDX-* converter methods
- [ ] `converters/helpers.rs` -- add `escape_shell_vars` function + tests
- [ ] Update `converter::tests::unimplemented_converters_return_empty_results` -- remove Gemini/Codex from stub assertion list (only Copilot/Skills remain stubs)

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `crates/memory-installer/src/` -- all Phase 46/47 infrastructure and Claude converter reference
- Existing Gemini adapter: `plugins/memory-gemini-adapter/.gemini/` -- TOML command format, settings.json hook format, SKILL.md structure
- Canonical plugin files: `plugins/memory-query-plugin/`, `plugins/memory-setup-plugin/` -- source format
- tool_maps.rs: Gemini and Codex mappings already present and tested

### Secondary (MEDIUM confidence)
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/) -- hook event names, settings.json format
- [Gemini CLI custom commands](https://geminicli.com/docs/cli/custom-commands/) -- TOML command format with description + prompt fields
- [Gemini CLI writing hooks](https://geminicli.com/docs/hooks/writing-hooks/) -- complete settings.json example
- [Codex skills documentation](https://developers.openai.com/codex/skills) -- SKILL.md format, directory structure
- [Codex AGENTS.md guide](https://developers.openai.com/codex/guides/agents-md) -- AGENTS.md discovery, concatenation
- [Codex configuration reference](https://developers.openai.com/codex/config-reference) -- sandbox_mode options (read-only, workspace-write)

### Tertiary (LOW confidence)
- Codex agent-to-skill mapping -- no existing adapter to reference; skill directory structure inferred from Codex docs
- Settings.json deep merge strategy -- based on general best practice, not verified against Gemini CLI behavior with merged files

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies already in Cargo.toml, `toml` crate already present
- Architecture: HIGH - follows established Phase 47 converter pattern, existing Gemini adapter provides target format reference
- Gemini TOML conversion: HIGH - `toml` crate handles serialization; existing adapter confirms format
- Gemini settings.json merge: MEDIUM - deep merge approach is sound but not validated against Gemini CLI's config loading behavior
- Codex SKILL.md conversion: HIGH - Codex docs clearly specify format; straightforward YAML-frontmatter Markdown
- Codex AGENTS.md: MEDIUM - format is flexible free-form Markdown; our generation approach is reasonable but untested against Codex
- Shell var escaping: HIGH - existing adapter confirms `$HOME` (not `${HOME}`) in hook scripts
- Sandbox permissions: MEDIUM - Codex docs confirm sandbox_mode options; mapping per agent is our design choice

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable domain, no external dependency changes expected)
