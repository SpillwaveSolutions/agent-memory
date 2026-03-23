# Phase 47: Claude & OpenCode Converters - Research

**Researched:** 2026-03-17
**Domain:** Rust converter implementations for YAML-frontmatter Markdown plugin format
**Confidence:** HIGH

## Summary

Phase 47 fills in the stub implementations of `ClaudeConverter` and `OpenCodeConverter` in the `memory-installer` crate. Phase 46 established all the infrastructure: the `RuntimeConverter` trait, `ConvertedFile` type, `write_files` with dry-run, `merge_managed_section`, `tool_maps::map_tool`, and the parser that produces `PluginBundle` with `serde_json::Value` frontmatter. The remaining work is purely converter logic -- manipulating `serde_json::Value` frontmatter objects, performing string replacements for path rewriting, and emitting `ConvertedFile` values with correct target paths.

The canonical source plugins have been analyzed: 6 commands, 2 agents, 13 skills across two plugin directories. Frontmatter is flat YAML (key-value with occasional arrays), already deserialized into `serde_json::Value` by the parser. No `allowed-tools` field exists in canonical agents (it is a concept for Claude's format), but agents do have `skills` arrays and `triggers` arrays. The OpenCode conversion needs to add `tools` object format and `mode: subagent` to agent frontmatter, and the Claude conversion is near pass-through with path rewriting.

**Primary recommendation:** Implement converters as pure functions over `serde_json::Value` frontmatter. Use `format!` macro for YAML serialization of flat frontmatter (avoid adding `serde_yaml` dependency). Keep the color-to-hex map as a private function in `opencode.rs`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Claude converter: near pass-through, path rewriting only (`~/.claude/` -> `~/.config/agent-memory/`)
- Claude: `convert_command` reconstructs frontmatter YAML + body, emits to `commands/<name>.md`
- Claude: `convert_agent` reconstructs frontmatter YAML + body, emits to `agents/<name>.md`
- Claude: `convert_skill` copies SKILL.md + additional_files into `skills/<name>/`
- Claude: `convert_hook` returns None (hooks deferred to Phase 49)
- Claude: `generate_guidance` returns empty Vec (no extra config needed)
- Claude target dir: `.claude/plugins/memory-plugin/{commands,agents,skills}/`
- OpenCode OC-01: `commands/` -> `command/` (singular), agents -> `agent/`
- OpenCode OC-02: Convert agent frontmatter to `tools:` object with `{tool_name: true}` entries
- OpenCode OC-03: Use `tool_maps::map_tool(Runtime::OpenCode, name)` for tool references
- OpenCode OC-04: Named colors to hex values in agent `color:` field
- OpenCode OC-05: Replace `~/.claude/` with `~/.config/opencode/` in all content
- OpenCode OC-06: `generate_guidance` produces managed section for `opencode.json` with read permissions
- OpenCode: `convert_hook` returns None (hooks deferred to Phase 49)
- Frontmatter manipulation via `serde_json::Value` (already parsed by Phase 46 parser)
- Unknown color names pass through unchanged with tracing::warn

### Claude's Discretion
- Whether to use `serde_yaml` for serialization or write a minimal YAML emitter
- Exact format of the opencode.json permissions block
- Whether color map lives in opencode.rs or a shared utility module
- Error handling for malformed frontmatter fields (e.g., non-string color values)

### Deferred Ideas (OUT OF SCOPE)
- Hook conversion for both Claude and OpenCode -- Phase 49
- Gemini converter -- Phase 48
- Codex/Copilot converters -- Phase 49
- E2E install tests with temp directories -- Phase 50
- --uninstall command -- v2.8
- Interactive mode -- v2.8
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLAUDE-01 | Claude converter copies canonical source with minimal transformation (path rewriting only) | Frontmatter is `serde_json::Value`, body is `String` -- reconstruct YAML frontmatter with `format!`, apply path replace on body |
| CLAUDE-02 | Storage paths rewritten to `~/.config/agent-memory/` | String `.replace("~/.claude/", "~/.config/agent-memory/")` on body content; frontmatter has no path references in canonical source |
| OC-01 | Commands flattened from `commands/` to `command/` | Target path uses `command/<name>.md` instead of `commands/<name>.md` |
| OC-02 | Agent frontmatter converts to `tools:` object with `tool: true` entries | Build `serde_json::Map` from `skills` array, map through `tool_maps::map_tool`, emit as YAML object |
| OC-03 | Tool names converted to lowercase with special mappings | `tool_maps::map_tool(Runtime::OpenCode, ...)` already handles this (e.g., AskUserQuestion -> question) |
| OC-04 | Color names normalized to hex values | Static lookup table for ~17 CSS named colors; pass through unknowns with warning |
| OC-05 | Paths rewritten from `~/.claude/` to `~/.config/opencode/` | String `.replace("~/.claude/", "~/.config/opencode/")` on body content |
| OC-06 | Auto-configure `opencode.json` read permissions for installed skill paths | `generate_guidance` emits a `ConvertedFile` targeting `opencode.json` using JSON managed-section pattern |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_json | workspace | Frontmatter manipulation as `Value` | Already parsed by gray_matter, zero new deps |
| gray_matter | 0.3 | YAML frontmatter parsing (read-only, in parser) | Already used by Phase 46 parser |
| walkdir | workspace | Directory traversal for skills | Already used by Phase 46 parser |
| clap | workspace | CLI argument parsing | Already used in main.rs |
| anyhow | workspace | Error handling | Project standard |
| tracing | workspace | Warnings for unmapped tools/colors | Project standard |
| directories | workspace | Platform-specific config paths | Already used for target_dir |
| shellexpand | 3.1 | Tilde expansion | Already used for target_dir |
| tempfile | workspace | Test temp directories | Already in dev-dependencies |

### No New Dependencies Required
The phase needs no new crate dependencies. YAML serialization uses `format!` macros for the flat frontmatter structures. JSON serialization for `opencode.json` uses `serde_json::to_string_pretty`.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| format! YAML emitter | serde_yaml | serde_yaml is deprecated; our frontmatter is flat enough for format! |
| Static color map in opencode.rs | Separate colors.rs module | Only OpenCode needs color mapping; keep it co-located |

## Architecture Patterns

### Recommended Module Structure
```
crates/memory-installer/src/
  converters/
    claude.rs          # ClaudeConverter impl (fill stubs)
    opencode.rs        # OpenCodeConverter impl (fill stubs)
    mod.rs             # select_converter (unchanged)
    ...                # other stubs (unchanged)
  converter.rs         # RuntimeConverter trait (unchanged)
  parser.rs            # parse_sources (unchanged)
  tool_maps.rs         # map_tool (unchanged)
  types.rs             # ConvertedFile, PluginBundle, etc. (unchanged)
  writer.rs            # write_files, merge_managed_section (unchanged)
  lib.rs               # module exports (unchanged)
  main.rs              # CLI + pipeline (unchanged)
```

### Pattern 1: Frontmatter Reconstruction via format!
**What:** Serialize `serde_json::Value` frontmatter back to YAML using a helper function that iterates over the JSON object's keys and emits `key: value` lines.
**When to use:** For commands and agents where frontmatter is flat key-value with occasional arrays.
**Example:**
```rust
/// Reconstruct YAML frontmatter + body into a markdown file string.
fn reconstruct_md(frontmatter: &serde_json::Value, body: &str) -> String {
    let yaml = value_to_yaml(frontmatter);
    if yaml.is_empty() {
        body.to_string()
    } else {
        format!("---\n{yaml}---\n\n{body}")
    }
}

/// Convert serde_json::Value to simple YAML string.
/// Handles: strings, numbers, booleans, arrays of strings, objects.
fn value_to_yaml(value: &serde_json::Value) -> String {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return String::new(),
    };
    let mut out = String::new();
    for (key, val) in obj {
        match val {
            Value::String(s) => writeln!(out, "{key}: {s}"),
            Value::Number(n) => writeln!(out, "{key}: {n}"),
            Value::Bool(b) => writeln!(out, "{key}: {b}"),
            Value::Array(arr) => {
                writeln!(out, "{key}:");
                for item in arr {
                    // Handle array of strings, objects, etc.
                }
            }
            Value::Object(map) => {
                writeln!(out, "{key}:");
                for (k, v) in map {
                    writeln!(out, "  {k}: {v}");
                }
            }
            _ => {}
        }
    }
    out
}
```

### Pattern 2: Path Rewriting
**What:** Replace runtime-specific paths in file content.
**When to use:** Both converters apply path rewriting on body content.
**Example:**
```rust
fn rewrite_paths(content: &str, from: &str, to: &str) -> String {
    content.replace(from, to)
}

// Claude: rewrite_paths(&body, "~/.claude/", "~/.config/agent-memory/")
// OpenCode: rewrite_paths(&body, "~/.claude/", "~/.config/opencode/")
```

### Pattern 3: OpenCode Tools Object Construction
**What:** Build `tools:` YAML object from canonical frontmatter.
**When to use:** OpenCode agent conversion (OC-02 + OC-03).
**Example:**
```rust
fn build_tools_object(frontmatter: &Value) -> Value {
    // Extract skills array -> determine which tools agents need
    // Map each tool name through tool_maps::map_tool(Runtime::OpenCode, ...)
    // Build {"read": true, "bash": true, ...} object
    let mut tools = serde_json::Map::new();
    // For memory-navigator: read, bash (tools it needs)
    // For setup-troubleshooter: read, bash, write, edit (broader tool access)
    tools.into()
}
```

### Pattern 4: OpenCode JSON Managed Section
**What:** `generate_guidance` produces an `opencode.json` with read permissions.
**When to use:** OC-06 requirement.
**Example:**
```rust
fn generate_guidance(&self, bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);
    // Collect skill paths
    let skill_paths: Vec<String> = bundle.skills.iter()
        .map(|s| target.join("skill").join(&s.name).display().to_string())
        .collect();
    // Build JSON with managed section marker
    let json = serde_json::json!({
        MANAGED_JSON_KEY: MANAGED_JSON_VALUE,
        "permissions": {
            "read": skill_paths
        }
    });
    let content = serde_json::to_string_pretty(&json).unwrap();
    // Return as ConvertedFile targeting opencode.json
    vec![ConvertedFile {
        target_path: target.parent().unwrap_or(&target).join("opencode.json"),
        content,
    }]
}
```

### Anti-Patterns to Avoid
- **Adding serde_yaml dependency:** It is deprecated and our frontmatter is flat enough for `format!`-based serialization. gray_matter handles the parsing side.
- **Modifying the parser or types:** Phase 47 should only fill in converter stubs. The parser, types, writer, and tool_maps are Phase 46 artifacts.
- **Deep-cloning frontmatter for every conversion:** Use references where possible; clone only when mutation is needed for OpenCode transformations.
- **Hardcoding tool lists in converters:** Always use `tool_maps::map_tool(Runtime::OpenCode, ...)` for tool name mapping. Never duplicate the mapping logic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tool name mapping | Inline match in converter | `tool_maps::map_tool(Runtime::OpenCode, ...)` | Centralized, tested, exhaustive |
| File writing + dry-run | Per-converter write logic | `writer::write_files(&files, dry_run)` | Already tested, handles mkdir, reporting |
| Managed config sections | Custom JSON merge | `writer::merge_managed_section` or return `ConvertedFile` | Already handles create/replace/append |
| Platform home directory | Hardcoded `~` expansion | `directories::BaseDirs` + `shellexpand` | Already used in target_dir |
| YAML parsing | Custom parser | `gray_matter` via `parser::parse_md_file` | Already parsed before converter sees it |

**Key insight:** Phase 46 built all the infrastructure. Phase 47 is purely converter logic -- transform `serde_json::Value` frontmatter and `String` body, return `Vec<ConvertedFile>`.

## Common Pitfalls

### Pitfall 1: YAML String Quoting
**What goes wrong:** YAML values containing colons, special chars, or multi-line strings break when serialized without quoting.
**Why it happens:** `format!("{key}: {value}")` works for simple strings but fails for values like `"search: conversations"` or multiline descriptions.
**How to avoid:** Quote string values that contain `: `, `#`, or newlines. Use YAML block scalar `|` for multi-line strings. The `description` field on skills uses `|` in canonical source.
**Warning signs:** Parsed-and-re-serialized files differ from originals when re-parsed.

### Pitfall 2: Frontmatter Key Ordering
**What goes wrong:** `serde_json::Map` iterates in insertion order (from gray_matter parse), but if you rebuild the Map for OpenCode, keys may reorder.
**Why it happens:** OpenCode agent frontmatter needs new keys (`tools`, `mode`) and removal of Claude-specific keys (`triggers`, `skills`).
**How to avoid:** Build OpenCode frontmatter from scratch with explicit key ordering: description, mode, tools, permission. Don't try to preserve canonical key order since the format is different.
**Warning signs:** Diff of generated files shows cosmetic-only changes.

### Pitfall 3: Skills Array vs Tools Object Confusion
**What goes wrong:** Canonical agents have `skills: [memory-query, topic-graph]` which lists skill dependencies. OpenCode needs `tools: {read: true, bash: true}` which lists runtime tool permissions.
**Why it happens:** These are different concepts. Skills are content packages; tools are runtime capabilities.
**How to avoid:** The `tools` object for OpenCode agents should be derived from what the agent actually needs (based on the existing manual OpenCode plugin), not from the skills array. Consider hardcoding tool permissions per agent or inferring from body content (bash commands imply `bash: true`).
**Warning signs:** Agent installed with wrong tool permissions (e.g., missing `bash` when agent runs shell commands).

### Pitfall 4: OpenCode Singular vs Plural Directory Names
**What goes wrong:** Using `commands/` instead of `command/`, `agents/` instead of `agent/`, `skills/` instead of `skill/`.
**Why it happens:** Claude uses plural (`commands/`, `agents/`, `skills/`). OpenCode uses singular (`command/`, `agent/`, `skill/`).
**How to avoid:** Hardcode the directory names per converter. Claude: `commands`, `agents`, `skills`. OpenCode: `command`, `agent` (note: from the existing OpenCode plugin, agents go in `agents/` not `agent/` -- verify this).
**Warning signs:** OpenCode CLI does not discover installed commands or agents.

### Pitfall 5: The `all_converters_return_empty_results_for_stubs` Test
**What goes wrong:** The existing test in `converter.rs` asserts that converters return empty results. Filling in stubs will break this test.
**Why it happens:** Phase 46 added the test to verify stubs were in place.
**How to avoid:** Update or remove this test when implementing the converter logic. It should be replaced with tests that verify actual conversion output.
**Warning signs:** `cargo test` fails immediately on the stub assertion.

## Code Examples

### Claude Converter: convert_command
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let body = rewrite_paths(&cmd.body, "~/.claude/", "~/.config/agent-memory/");
    let content = reconstruct_md(&cmd.frontmatter, &body);
    vec![ConvertedFile {
        target_path: target_dir.join("commands").join(format!("{}.md", cmd.name)),
        content,
    }]
}
```

### OpenCode Converter: convert_command
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let body = rewrite_paths(&cmd.body, "~/.claude/", "~/.config/opencode/");
    // OpenCode commands have simpler frontmatter (just description)
    let mut fm = serde_json::Map::new();
    if let Some(desc) = cmd.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }
    let content = reconstruct_md(&Value::Object(fm), &body);
    vec![ConvertedFile {
        target_path: target_dir.join("command").join(format!("{}.md", cmd.name)),
        content,
    }]
}
```

### OpenCode Converter: convert_agent (OC-02, OC-03, OC-04)
```rust
fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let body = rewrite_paths(&agent.body, "~/.claude/", "~/.config/opencode/");

    let mut fm = serde_json::Map::new();
    // Copy description
    if let Some(desc) = agent.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }
    // Add mode
    fm.insert("mode".to_string(), Value::String("subagent".to_string()));
    // Build tools object (OC-02 + OC-03)
    let tools = build_agent_tools(agent);
    fm.insert("tools".to_string(), tools);
    // Color normalization (OC-04)
    if let Some(color) = agent.frontmatter.get("color").and_then(|v| v.as_str()) {
        fm.insert("color".to_string(), Value::String(normalize_color(color)));
    }

    let content = reconstruct_md(&Value::Object(fm), &body);
    vec![ConvertedFile {
        target_path: target_dir.join("agents").join(format!("{}.md", agent.name)),
        content,
    }]
}
```

### Color Normalization (OC-04)
```rust
fn normalize_color(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "black" => "#000000",
        "white" => "#FFFFFF",
        "red" => "#FF0000",
        "green" => "#008000",
        "blue" => "#0000FF",
        "yellow" => "#FFFF00",
        "cyan" | "aqua" => "#00FFFF",
        "magenta" | "fuchsia" => "#FF00FF",
        "orange" => "#FFA500",
        "purple" => "#800080",
        "gray" | "grey" => "#808080",
        "lime" => "#00FF00",
        "navy" => "#000080",
        "teal" => "#008080",
        "maroon" => "#800000",
        "olive" => "#808000",
        "silver" => "#C0C0C0",
        other => {
            // Already hex or unknown -- pass through
            if other.starts_with('#') {
                return name.to_string();
            }
            tracing::warn!("unknown color name '{name}' -- passing through unchanged");
            name.to_string()
        }
    }.to_string()
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual OpenCode plugin in `plugins/memory-opencode-plugin/` | Auto-generated by `memory-installer` | Phase 47 (now) | Single canonical source, automated conversion |
| serde_yaml for YAML serialization | format!-based emitter for flat frontmatter | Phase 47 decision | Avoids deprecated dependency |

**Deprecated/outdated:**
- `serde_yaml`: Archived/deprecated. gray_matter handles YAML parsing. For serialization, use format!-based approach for flat structures.
- Manual adapter plugins (`memory-opencode-plugin/`, `memory-copilot-adapter/`, etc.): Will be replaced by installer output. Archived (not deleted) per project decision.

## Open Questions

1. **OpenCode Agent Tool Permissions**
   - What we know: The existing manual `memory-navigator.md` for OpenCode has `tools: {read: true, bash: true, write: false, edit: false}`. The canonical source has no `tools` or `allowed-tools` field.
   - What's unclear: How to derive tool permissions programmatically. The canonical `skills` field lists content dependencies, not tool permissions.
   - Recommendation: Define a default tool set per agent type. Memory-navigator needs `read: true, bash: true` (reads files, runs memory-daemon commands). Setup-troubleshooter needs `read: true, bash: true, write: true, edit: true` (may fix config files). Or: scan body content for `bash` code blocks to infer `bash: true`, presence of file operations for `write/edit`. Simplest: hardcode sensible defaults and allow override later.

2. **OpenCode `agents/` vs `agent/` Directory**
   - What we know: The existing manual OpenCode plugin uses `agents/` (plural) for agents but `command/` (singular) for commands and `skill/` (singular) for skills. The CONTEXT.md says OC-01 specifies `agents -> agent/` (singular).
   - What's unclear: Whether OpenCode actually requires singular `agent/` or uses plural `agents/`.
   - Recommendation: Follow the existing manual OpenCode plugin structure which uses `agents/` (plural) for the agents directory. This is the tested working format. Only `command/` and `skill/` are singular.

3. **YAML Multi-line String Serialization**
   - What we know: The `description` field in skill SKILL.md files uses YAML `|` block scalar for multi-line text.
   - What's unclear: Whether `format!`-based YAML serializer handles this correctly.
   - Recommendation: Detect newlines in string values and use `|` block scalar notation. For single-line strings, emit inline.

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
| CLAUDE-01 | Claude converter produces correct command/agent/skill files | unit | `cargo test -p memory-installer converters::claude -x` | No - Wave 0 |
| CLAUDE-02 | Path rewriting `~/.claude/` -> `~/.config/agent-memory/` | unit | `cargo test -p memory-installer converters::claude::tests::path_rewriting -x` | No - Wave 0 |
| OC-01 | Commands use `command/` not `commands/` in target path | unit | `cargo test -p memory-installer converters::opencode::tests::flat_naming -x` | No - Wave 0 |
| OC-02 | Agent frontmatter has `tools:` object format | unit | `cargo test -p memory-installer converters::opencode::tests::tools_object -x` | No - Wave 0 |
| OC-03 | Tool names lowercase with special mappings | unit | `cargo test -p memory-installer converters::opencode::tests::tool_mapping -x` | No - Wave 0 |
| OC-04 | Color names normalized to hex | unit | `cargo test -p memory-installer converters::opencode::tests::color_hex -x` | No - Wave 0 |
| OC-05 | Paths rewritten `~/.claude/` -> `~/.config/opencode/` | unit | `cargo test -p memory-installer converters::opencode::tests::path_rewriting -x` | No - Wave 0 |
| OC-06 | `opencode.json` has read permissions for skill paths | unit | `cargo test -p memory-installer converters::opencode::tests::guidance_permissions -x` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`
- **Phase gate:** `task pr-precheck` (format + clippy + test + doc)

### Wave 0 Gaps
- [ ] `converters/claude.rs` tests -- unit tests for Claude converter methods
- [ ] `converters/opencode.rs` tests -- unit tests for OpenCode converter methods
- [ ] Update/remove `converter::tests::all_converters_return_empty_results_for_stubs` -- it will fail once stubs are filled
- [ ] Shared test helper: `reconstruct_md` and `rewrite_paths` helper functions need tests (could be in a shared module or inline in each converter)

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `crates/memory-installer/src/` -- all Phase 46 infrastructure files read directly
- Canonical plugin files: `plugins/memory-query-plugin/`, `plugins/memory-setup-plugin/` -- frontmatter formats verified
- Existing OpenCode plugin: `plugins/memory-opencode-plugin/.opencode/` -- target format reference

### Secondary (MEDIUM confidence)
- CONTEXT.md decisions -- derived from user discussion, may have ambiguity on `agent/` vs `agents/` directory naming

### Tertiary (LOW confidence)
- OpenCode directory naming convention (singular vs plural for agents) -- only verified against existing manual plugin, not official OpenCode docs

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies already in Cargo.toml, no new crates needed
- Architecture: HIGH - Phase 46 infrastructure is complete and tested, converter pattern is clear
- Pitfalls: HIGH - verified against actual canonical source files and existing OpenCode plugin
- YAML serialization: MEDIUM - format!-based approach works for flat structures but needs careful handling of multi-line strings and special characters

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable domain, no external dependency changes expected)
