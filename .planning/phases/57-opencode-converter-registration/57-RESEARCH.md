# Phase 57: OpenCode Converter + Registration - Research

**Researched:** 2026-03-25
**Domain:** Rust runtime converter implementation (memory-installer crate)
**Confidence:** HIGH

## Summary

Phase 57 replaces the stub `OpenCodeConverter` in `crates/memory-installer/src/converters/opencode.rs` with a full implementation that converts canonical Claude-format plugin files into OpenCode-native format. The converter follows the well-established `RuntimeConverter` trait pattern used by 5 working converters (claude, gemini, codex, copilot, skills). Additionally, the `generate_guidance` method must produce an `opencode.json` permissions file that merges with any existing configuration.

The codebase already has all infrastructure in place: the `RuntimeConverter` trait, the `tool_maps::map_tool(Runtime::OpenCode, ...)` mappings (all 11 tools mapped), the `helpers` module (`value_to_yaml`, `reconstruct_md`, `rewrite_paths`), and the `writer` module (`write_files`, `merge_managed_section`). The Python reference implementation in `codebase-mentor/converters/opencode.py` provides a clear template for the conversion logic.

**Primary recommendation:** Implement OpenCode converter by following the Gemini converter pattern (frontmatter transformation + tool mapping + JSON config generation) with OpenCode-specific differences: singular directory names, tools-as-object format, color hex conversion, and `opencode.json` permission merging.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Modify existing `crates/memory-installer/src/converters/opencode.rs` -- replace stub with real implementation
- Follow patterns from `claude.rs`, `gemini.rs`, `codex.rs` converters (all in same directory)
- Reuse existing helpers: `value_to_yaml`, `reconstruct_md`, `rewrite_paths` from `converters/helpers.rs`
- Reuse existing tool mapping: `tool_maps::map_tool(Runtime::OpenCode, tool_name)` already has mappings
- Commands flattened: `commands/memory-search.md` -> `command/memory-search.md` (singular directory)
- `allowed-tools:` array -> `tools:` object with `{ tool_name: true }` entries
- Tool names converted via `tool_maps::map_tool(Runtime::OpenCode, name)`
- `name:` field removed (OpenCode derives from filename)
- Color values: named colors -> hex (`cyan` -> `#00FFFF`, `blue` -> `#0000FF`, etc.)
- `subagent_type: "general-purpose"` -> `"general"`
- Path rewriting: `~/.claude/plugins/` -> `~/.config/opencode/`, `~/.claude/` -> `~/.config/opencode/`
- `~/.config/agent-memory/` paths left unchanged (runtime-neutral)
- Permission format: `{ "permission": { "read": { "glob": "allow" }, "external_directory": { "glob": "allow" } } }`
- Must MERGE with existing `opencode.json` (not overwrite)
- Existing E2E test `opencode_stub` in `tests/e2e_converters.rs` should be updated to verify real output

### Claude's Discretion
- Exact hex color map (named colors to hex values)
- Whether to include `description:` field in agent `tools:` object or just `true`
- Error handling for malformed frontmatter in canonical source

### Deferred Ideas (OUT OF SCOPE)
- OpenCode hook support (hooks not yet standardized in OpenCode)
- Interactive mode for OpenCode installer
- Gemini/Codex/Copilot registration (separate phases)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| OC-01 | Commands flattened from `commands/` to `command/` (singular) | `convert_command` emits to `command/` directory; follows claude.rs pattern with path change |
| OC-02 | Agent frontmatter converts `allowed-tools:` array to `tools:` object | Build tools object from frontmatter; follows gemini.rs `build_gemini_tools` pattern |
| OC-03 | Tool names converted to lowercase with special mappings | `tool_maps::map_tool(Runtime::OpenCode, name)` already complete -- all 11 tools mapped |
| OC-04 | Color names normalized to hex values | New `color_to_hex()` function using COLOR_MAP from Python reference |
| OC-05 | Paths rewritten from `~/.claude/` to `~/.config/opencode/` | `helpers::rewrite_paths` with ordered path replacements (longer first) |
| OC-06 | Auto-configure `opencode.json` read permissions | `generate_guidance` produces opencode.json ConvertedFile with permission structure |
| OREG-01 | `--agent opencode` writes `opencode.json` with read permissions | `generate_guidance` called during install pipeline; emits to target_dir parent |
| OREG-02 | Permission entries use glob patterns matching installed directories | Glob pattern: `~/.config/opencode/agent-memory/*` (global) or `.opencode/agent-memory/*` (project) |
| OREG-03 | Existing `opencode.json` preserved (merge, not overwrite) | JSON merge logic in `generate_guidance` -- read existing, deep-merge permission keys |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_json | (workspace) | JSON manipulation for frontmatter and opencode.json | Already used by all converters |
| serde | (workspace) | Serialization | Already in workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| helpers module | internal | `value_to_yaml`, `reconstruct_md`, `rewrite_paths` | Every convert method |
| tool_maps module | internal | `map_tool(Runtime::OpenCode, name)` | Agent tool conversion |
| writer module | internal | `write_files`, `merge_managed_section` | File output |
| tempfile | (workspace) | Temp dirs for testing | E2E and unit tests |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom YAML emitter (helpers) | serde_yaml crate | Custom emitter already works, no new dependency needed |
| Manual JSON merge | json_patch crate | Simple shallow merge is sufficient; no need for RFC 6902 |

**No new dependencies required.** All libraries are already in the workspace.

## Architecture Patterns

### Recommended Project Structure
```
crates/memory-installer/src/converters/
    opencode.rs       # MODIFY: Replace stub with full implementation
crates/memory-installer/tests/
    e2e_converters.rs  # MODIFY: Update opencode_stub test to verify real output
```

### Pattern 1: Converter Method Structure (from claude.rs, gemini.rs)
**What:** Each `convert_*` method follows: get target_dir -> transform frontmatter -> rewrite paths -> reconstruct markdown -> return ConvertedFile vec
**When to use:** Every convert method
**Example:**
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let body = rewrite_paths(&cmd.body, OC_PATH_FROM, OC_PATH_TO);
    let content = reconstruct_md(&cmd.frontmatter, &body);
    vec![ConvertedFile {
        target_path: target_dir.join("command").join(format!("{}.md", cmd.name)),
        content,
    }]
}
```

### Pattern 2: Agent Tool Mapping (from gemini.rs `build_gemini_tools`)
**What:** Extract `allowed-tools` array from frontmatter, map through `tool_maps`, build runtime-specific format
**When to use:** `convert_agent` -- OpenCode needs `tools:` object instead of list
**Example:**
```rust
fn build_opencode_tools(agent: &PluginAgent) -> serde_json::Map<String, serde_json::Value> {
    let mut tools = serde_json::Map::new();
    if let Some(arr) = agent.frontmatter.get("allowed-tools").and_then(|v| v.as_array()) {
        for tool_val in arr {
            if let Some(name) = tool_val.as_str() {
                if name.starts_with("mcp__") {
                    tools.insert(name.to_string(), json!(true));
                    continue;
                }
                if let Some(mapped) = map_tool(Runtime::OpenCode, name) {
                    tools.insert(mapped.to_string(), json!(true));
                }
            }
        }
    }
    tools
}
```

### Pattern 3: JSON Permission Merge (from Python reference)
**What:** Read existing `opencode.json`, deep-merge permission entries, write back
**When to use:** `generate_guidance` for OREG-01..03
**Example:**
```rust
fn generate_guidance(&self, _bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);
    let perm_path = match &cfg.scope {
        InstallScope::Global => "~/.config/opencode/agent-memory/*",
        _ => ".opencode/agent-memory/*",
    };
    let permissions = json!({
        "permission": {
            "read": { perm_path: "allow" },
            "external_directory": { perm_path: "allow" }
        }
    });
    // Note: actual merge with existing file happens at write time
    vec![ConvertedFile {
        target_path: target.parent().unwrap_or(&target).join("opencode.json"),
        content: serde_json::to_string_pretty(&permissions).unwrap_or_default(),
    }]
}
```

### Pattern 4: Path Rewriting with Order (from Python reference)
**What:** Apply longer path rewrites first to avoid partial replacements
**When to use:** All convert methods
**Key detail:** `~/.claude/plugins/` must be replaced before `~/.claude/` to avoid `~/.config/opencode/plugins/` artifacts
```rust
const OC_PATHS: &[(&str, &str)] = &[
    ("~/.claude/plugins/", "~/.config/opencode/"),
    ("~/.claude/", "~/.config/opencode/"),
];
```

### Pattern 5: Color Hex Conversion (new for OpenCode)
**What:** Convert named CSS colors to hex values in agent frontmatter
**When to use:** `convert_agent` when processing `color:` field
```rust
fn color_to_hex(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "cyan" => Some("#00FFFF"),
        "red" => Some("#FF0000"),
        "green" => Some("#00FF00"),
        "blue" => Some("#0000FF"),
        "yellow" => Some("#FFFF00"),
        "magenta" => Some("#FF00FF"),
        "orange" => Some("#FFA500"),
        "purple" => Some("#800080"),
        "pink" => Some("#FFC0CB"),
        "white" => Some("#FFFFFF"),
        "black" => Some("#000000"),
        "gray" | "grey" => Some("#808080"),
        _ => None, // Already hex or unknown -- pass through
    }
}
```

### Anti-Patterns to Avoid
- **Re-parsing YAML frontmatter manually:** The frontmatter is already parsed into `serde_json::Value` by the time it reaches the converter. Do NOT re-parse from string like the Python reference does.
- **Modifying tool_maps.rs:** OpenCode mappings are already complete (all 11 tools). Do not add new entries.
- **Using `merge_managed_section` for JSON:** That function uses text markers for plain-text files. For JSON merge, implement proper `serde_json::Value` merge in `generate_guidance`.
- **Forgetting path rewrite ordering:** Must replace `~/.claude/plugins/` before `~/.claude/` to avoid double-replacement.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML frontmatter emission | Custom YAML serializer | `helpers::value_to_yaml` + `reconstruct_md` | Already handles quoting, block scalars, arrays |
| Tool name mapping | Hardcoded match in converter | `tool_maps::map_tool(Runtime::OpenCode, name)` | Centralized, tested, all 11 tools covered |
| File writing + dry run | Custom fs::write wrapper | `writer::write_files` | Already handles dry-run, parent dirs, reporting |
| Path string replacement | Regex-based rewriter | `helpers::rewrite_paths` | Simple string replace, sufficient for all cases |

**Key insight:** The converter only transforms data and returns `Vec<ConvertedFile>`. All I/O is handled by the writer module. Keep converters pure transformation functions.

## Common Pitfalls

### Pitfall 1: Path Rewrite Order Matters
**What goes wrong:** `~/.claude/plugins/foo` gets rewritten to `~/.config/opencode/plugins/foo` instead of `~/.config/opencode/foo`
**Why it happens:** `~/.claude/` is replaced before `~/.claude/plugins/`
**How to avoid:** Apply longer matches first: `~/.claude/plugins/` -> `~/.config/opencode/` THEN `~/.claude/` -> `~/.config/opencode/`
**Warning signs:** Test paths containing `plugins` in the middle of the rewritten path

### Pitfall 2: Tools Object vs Tools Array
**What goes wrong:** Emitting `tools:` as a YAML array (`- tool`) instead of YAML object (`tool: true`)
**Why it happens:** Other converters (gemini, codex) use tool arrays; OpenCode uses an object
**How to avoid:** Build a `serde_json::Map<String, Value>` with `tool_name: true` entries and emit via `value_to_yaml`
**Warning signs:** OpenCode failing to recognize tool permissions

### Pitfall 3: MCP Tools Pass Through for OpenCode
**What goes wrong:** Filtering out `mcp__*` tools like Gemini/Codex do
**Why it happens:** Copy-pasting the Gemini `build_gemini_tools` pattern which skips MCP tools
**How to avoid:** For OpenCode, `mcp__*` tools pass through unchanged (see tool_maps.rs doc comment and CONTEXT.md)
**Warning signs:** Missing MCP tool entries in converted agent files

### Pitfall 4: opencode.json Merge vs Overwrite
**What goes wrong:** Writing a fresh `opencode.json` that destroys user's existing configuration
**Why it happens:** Using `write_files` which overwrites by default
**How to avoid:** In `generate_guidance`, read existing file, merge permission keys, then emit merged content. Or handle merge at a higher level.
**Warning signs:** User's OpenCode settings disappearing after install

### Pitfall 5: Singular Directory Names
**What goes wrong:** Using `commands/` or `agents/` (plural) instead of `command/` or `agent/` (singular)
**Why it happens:** All other converters use plural directory names
**How to avoid:** OpenCode uses singular: `command/`, `agent/`, `skill/`
**Warning signs:** Files not found by OpenCode runtime

### Pitfall 6: Name Field Removal
**What goes wrong:** Including `name:` in agent frontmatter
**Why it happens:** Claude/Gemini/Codex converters keep or add `name:` field
**How to avoid:** When building OpenCode agent frontmatter, skip the `name` key. OpenCode derives name from filename.
**Warning signs:** OpenCode showing duplicate or wrong agent names

## Code Examples

### OpenCode Command Conversion (OC-01, OC-05)
```rust
// Source: Pattern from claude.rs + CONTEXT.md decisions
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    // Apply ordered path rewrites
    let body = rewrite_paths(&cmd.body, "~/.claude/plugins/", "~/.config/opencode/");
    let body = rewrite_paths(&body, "~/.claude/", "~/.config/opencode/");
    let content = reconstruct_md(&cmd.frontmatter, &body);
    vec![ConvertedFile {
        // OC-01: singular "command" directory
        target_path: target_dir.join("command").join(format!("{}.md", cmd.name)),
        content,
    }]
}
```

### OpenCode Agent Conversion (OC-02, OC-03, OC-04)
```rust
// Source: Pattern from gemini.rs build_gemini_tools + Python reference
fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);

    // Build new frontmatter: skip name, convert tools, convert color
    let mut fm = serde_json::Map::new();

    // Add tools object (OC-02, OC-03)
    let tools = build_opencode_tools(agent);
    if !tools.is_empty() {
        fm.insert("tools".to_string(), serde_json::Value::Object(tools));
    }

    // Copy non-excluded fields from original frontmatter
    if let Some(obj) = agent.frontmatter.as_object() {
        for (key, val) in obj {
            match key.as_str() {
                "name" | "allowed-tools" => continue, // Skip
                "color" => {
                    // OC-04: Convert named colors to hex
                    if let Some(color_name) = val.as_str() {
                        let hex = color_to_hex(color_name).unwrap_or(color_name);
                        fm.insert(key.clone(), serde_json::Value::String(hex.to_string()));
                    } else {
                        fm.insert(key.clone(), val.clone());
                    }
                }
                "subagent_type" => {
                    // Normalize "general-purpose" -> "general"
                    if val.as_str() == Some("general-purpose") {
                        fm.insert(key.clone(), serde_json::Value::String("general".to_string()));
                    } else {
                        fm.insert(key.clone(), val.clone());
                    }
                }
                _ => { fm.insert(key.clone(), val.clone()); }
            }
        }
    }

    // Apply path rewrites to body
    let body = rewrite_paths(&agent.body, "~/.claude/plugins/", "~/.config/opencode/");
    let body = rewrite_paths(&body, "~/.claude/", "~/.config/opencode/");

    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: target_dir.join("agent").join(format!("{}.md", agent.name)),
        content,
    }]
}
```

### OpenCode Permission JSON (OC-06, OREG-01..03)
```rust
// Source: Python reference _write_opencode_permissions
fn generate_guidance(&self, _bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);
    let json_path = target.parent()
        .unwrap_or(&target)
        .join("opencode.json");

    let perm_path = match &cfg.scope {
        InstallScope::Global => "~/.config/opencode/agent-memory/*",
        InstallScope::Project(_) => ".opencode/agent-memory/*",
        InstallScope::Custom(_) => return Vec::new(), // No registration for custom
    };

    // Read existing opencode.json if present
    let mut data: serde_json::Value = if json_path.exists() {
        std::fs::read_to_string(&json_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| json!({}))
    } else {
        json!({})
    };

    // Merge permission entries
    let perms = data.as_object_mut()
        .unwrap()
        .entry("permission")
        .or_insert_with(|| json!({}));

    if let Some(pobj) = perms.as_object_mut() {
        pobj.entry("read")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .map(|m| m.insert(perm_path.to_string(), json!("allow")));
        pobj.entry("external_directory")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .map(|m| m.insert(perm_path.to_string(), json!("allow")));
    }

    let content = serde_json::to_string_pretty(&data).unwrap_or_default() + "\n";

    vec![ConvertedFile {
        target_path: json_path,
        content,
    }]
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| OpenCode stub (empty Vecs) | Full converter implementation | Phase 57 (now) | Enables OpenCode runtime support |
| No registration | opencode.json permission writing | Phase 57 (now) | Auto-configures read access |

**Note:** The existing E2E test `opencode_stub` (line 517-561 of `e2e_converters.rs`) asserts that all convert methods return empty. This test MUST be rewritten to verify real output instead.

## Open Questions

1. **opencode.json merge timing**
   - What we know: `generate_guidance` returns `Vec<ConvertedFile>` which goes through `write_files` (which always overwrites)
   - What's unclear: The merge logic must read the existing file BEFORE generating the ConvertedFile content. This means `generate_guidance` must read the filesystem.
   - Recommendation: Implement the read-merge-emit pattern inside `generate_guidance` itself, similar to how the Python reference does it. The `generate_guidance` method already has access to `cfg` which provides the scope. If `json_path` doesn't exist yet, emit a fresh JSON. Other converters (gemini) already emit JSON via `generate_guidance` without merge -- OpenCode is the first to need merge.

2. **`value_to_yaml` with nested objects for tools**
   - What we know: `value_to_yaml` handles nested objects by emitting `key:\n  inner_key: value`
   - What's unclear: Whether `tools:\n  read: true\n  write: true` renders correctly via `value_to_yaml` (booleans in nested objects)
   - Recommendation: Verify with a unit test. The `write_yaml_value` function handles `Bool(b)` at any indent level, so this should work. The tools object `{"tools": {"read": true, "write": true}}` should emit correctly.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in + pretty_assertions) |
| Config file | Cargo.toml workspace |
| Quick run command | `cargo test -p memory-installer` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OC-01 | Command -> `command/` singular dir | unit | `cargo test -p memory-installer opencode::tests::convert_command -x` | Wave 0 |
| OC-02 | `allowed-tools` -> `tools:` object | unit | `cargo test -p memory-installer opencode::tests::convert_agent_tools -x` | Wave 0 |
| OC-03 | Tool name lowercase mapping | unit | `cargo test -p memory-installer opencode::tests::tool_mapping -x` | Wave 0 |
| OC-04 | Color name -> hex | unit | `cargo test -p memory-installer opencode::tests::color_hex -x` | Wave 0 |
| OC-05 | Path rewriting | unit | `cargo test -p memory-installer opencode::tests::path_rewrite -x` | Wave 0 |
| OC-06 | opencode.json permissions | unit | `cargo test -p memory-installer opencode::tests::generate_guidance -x` | Wave 0 |
| OREG-01 | opencode.json written | integration | `cargo test -p memory-installer opencode_full_bundle -x` | Wave 0 (update existing) |
| OREG-02 | Glob patterns correct | unit | `cargo test -p memory-installer opencode::tests::permission_glob -x` | Wave 0 |
| OREG-03 | JSON merge preserves existing | unit | `cargo test -p memory-installer opencode::tests::merge_existing -x` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo test --workspace --all-features`
- **Phase gate:** Full suite green + `task pr-precheck`

### Wave 0 Gaps
- [ ] Unit tests in `opencode.rs` -- covers OC-01 through OC-06, OREG-02, OREG-03 (follow claude.rs test pattern)
- [ ] Update `opencode_stub` E2E test in `e2e_converters.rs` -> `opencode_full_bundle` -- covers OREG-01
- [ ] Test `value_to_yaml` with tools object format (nested bool values)

## Sources

### Primary (HIGH confidence)
- `crates/memory-installer/src/converters/opencode.rs` -- existing stub, line-by-line analysis
- `crates/memory-installer/src/converters/claude.rs` -- pass-through converter pattern
- `crates/memory-installer/src/converters/gemini.rs` -- tool mapping + JSON config pattern
- `crates/memory-installer/src/converters/codex.rs` -- command-to-skill + tool mapping pattern
- `crates/memory-installer/src/converters/helpers.rs` -- shared helper functions
- `crates/memory-installer/src/tool_maps.rs` -- OpenCode tool mappings (all 11 verified)
- `crates/memory-installer/src/types.rs` -- ConvertedFile, PluginAgent, etc.
- `crates/memory-installer/src/converter.rs` -- RuntimeConverter trait definition
- `crates/memory-installer/src/writer.rs` -- write_files, merge_managed_section
- `crates/memory-installer/tests/e2e_converters.rs` -- existing opencode_stub test

### Secondary (MEDIUM confidence)
- `codebase-mentor/converters/opencode.py` -- Python reference implementation (same project, different codebase)

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in workspace, no new dependencies
- Architecture: HIGH -- 5 working converter examples to follow, trait interface locked
- Pitfalls: HIGH -- Python reference implementation reveals exact edge cases
- Test architecture: HIGH -- existing E2E test framework and unit test patterns well established

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable internal codebase, not externally dependent)
