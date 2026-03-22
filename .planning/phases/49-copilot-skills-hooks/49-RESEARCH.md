# Phase 49: Copilot, Generic Skills & Hook Porting - Research

**Researched:** 2026-03-17
**Domain:** Rust converter implementations for Copilot CLI (.github/ format) and Generic Skills (pass-through), plus hook conversion for all 6 runtimes
**Confidence:** HIGH

## Summary

Phase 49 completes the final two converter stubs (CopilotConverter, SkillsConverter) and implements hook conversion across all 6 runtimes. The existing Copilot adapter in `plugins/memory-copilot-adapter/` provides an authoritative reference for the target output format: agents as `.github/agents/<name>.agent.md`, skills under `.github/skills/<name>/SKILL.md`, and hooks as `.github/hooks/memory-hooks.json` with companion shell scripts.

The hook conversion is the most substantial new work. Currently `convert_hook` returns `None` in all converters, and the parser returns an empty `hooks` vec. CANON-02 requires defining canonical hook definitions. The recommended approach is to define canonical hooks programmatically within each converter's `generate_guidance` method (already done for Gemini in Phase 48 -- it generates `settings.json` with hooks). This avoids adding YAML hook files to the canonical source and keeps the hook-generation logic per-runtime where it belongs. The Gemini converter already demonstrates this pattern. The Copilot converter needs to generate `memory-hooks.json` and `memory-capture.sh` similarly. Claude and Codex do not have hook systems. OpenCode hook format needs a decision (deferred or minimal stub).

The SkillsConverter is the simplest: near pass-through like ClaudeConverter but targeting a user-specified directory. Commands become skill directories, agents become orchestration skills, path rewriting from `~/.claude/` to `~/.config/agent-memory/`. No runtime-specific transforms beyond path rewriting.

**Primary recommendation:** Implement CopilotConverter following the Codex pattern (commands as skills, agents as `.agent.md` files). Implement SkillsConverter as a simplified ClaudeConverter with `--dir` targeting. For hooks, expand the `generate_guidance` approach (already used by Gemini) to Copilot. For hook scripts, embed the shell script content as a const string in the converter and emit as a ConvertedFile. Update Claude/Codex/OpenCode `convert_hook` to remain None (these runtimes either lack hook systems or hooks are out of scope).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| COP-01 | Commands converted to Copilot skill format under `.github/skills/` | Follow Codex pattern: commands become `skills/<name>/SKILL.md` with YAML frontmatter under `.github/` target dir |
| COP-02 | Agents converted to `.agent.md` format with Copilot tool names | Existing adapter shows format: `.github/agents/<name>.agent.md` with tools array, infer field, description |
| COP-03 | Hook definitions converted to `.github/hooks/` JSON format with shell scripts | Existing adapter provides exact JSON format (camelCase events, `bash` field, `timeoutSec`). Shell script is ~230 lines with fail-open, session synthesis, ANSI stripping |
| SKL-01 | `--agent skills --dir <path>` installs to user-specified directory | SkillsConverter `target_dir` returns `InstallScope::Custom(dir)` path; main.rs already validates `--dir` is required for skills |
| SKL-02 | Commands become skill directories, agents become orchestration skills | Same pattern as Codex: `skills/<name>/SKILL.md` for commands, `skills/<name>/SKILL.md` for agents with tool/sandbox sections |
| SKL-03 | No runtime-specific transforms beyond path rewriting | Tool names pass through unchanged (Skills maps same as Claude in tool_maps); only `~/.claude/` -> `~/.config/agent-memory/` path rewrite |
| HOOK-01 | Canonical YAML hook definitions converted to per-runtime formats | Rather than canonical YAML files, each converter generates hooks in `generate_guidance` (Gemini already does this). Hook scripts embedded as const strings |
| HOOK-02 | Hook event names mapped correctly per runtime (PascalCase/camelCase) | Gemini: PascalCase (`SessionStart`, `BeforeAgent`). Copilot: camelCase (`sessionStart`, `userPromptSubmitted`). Mapping table in research |
| HOOK-03 | Hook scripts generated with fail-open behavior and background execution | Both existing scripts use `trap fail_open ERR EXIT`, background `memory-ingest &`, exit 0 always. Pattern verified in both adapters |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_json | workspace | Frontmatter manipulation, hooks JSON generation | Already parsed by gray_matter |
| gray_matter | 0.3 | YAML frontmatter parsing (read-only, in parser) | Already used by Phase 46 |
| walkdir | workspace | Directory traversal for skills | Already used by Phase 46 |
| anyhow | workspace | Error handling | Project standard |
| tracing | workspace | Warnings for unmapped tools | Project standard |
| directories | workspace | Platform-specific config paths | Already used for target_dir |
| shellexpand | 3.1 | Tilde expansion | Already used for target_dir |
| tempfile | workspace | Test temp directories | Already in dev-dependencies |

### No New Dependencies Required
All needed crates are already in `crates/memory-installer/Cargo.toml`. No new libraries needed.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Embedding hook scripts as const &str | Reading from canonical source files | Embedding keeps converter self-contained; existing adapters are the authoritative source |
| YAML canonical hook files + parser | Per-converter hook generation in `generate_guidance` | Gemini already generates hooks this way; consistency wins over separate canonical format |
| Separate hook converter functions | Inline in `generate_guidance` | Keep hook logic co-located with the converter that knows the target format |

## Architecture Patterns

### Recommended Module Structure
```
crates/memory-installer/src/
  converters/
    copilot.rs         # CopilotConverter impl (fill stubs)
    skills.rs          # SkillsConverter impl (fill stubs)
    helpers.rs         # Add hook script helpers if needed
    claude.rs          # Unchanged
    opencode.rs        # Unchanged (stubs remain for Phase 47 scope)
    gemini.rs          # Update convert_hook or keep as-is (hooks already in generate_guidance)
    codex.rs           # Unchanged
    mod.rs             # select_converter (unchanged)
  converter.rs         # RuntimeConverter trait (unchanged)
  types.rs             # Unchanged
  writer.rs            # Unchanged
  tool_maps.rs         # Unchanged -- Copilot/Skills mappings already present
```

### Pattern 1: Copilot Agent Conversion (COP-02)
**What:** Convert canonical agents to `.agent.md` format with YAML frontmatter containing `name`, `description`, `tools` array (Copilot tool names), and `infer: true`.
**When to use:** COP-02 requirement.
**Reference:** Existing adapter `plugins/memory-copilot-adapter/.github/agents/memory-navigator.agent.md`
**Example:**
```rust
fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);

    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), json!(agent.name));
    if let Some(desc) = agent.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    // Build tools array with Copilot names
    let tools = build_copilot_tools(agent);
    fm.insert("tools".to_string(), json!(tools));
    fm.insert("infer".to_string(), json!(true));

    let body = rewrite_paths(&agent.body, COPILOT_PATH_FROM, COPILOT_PATH_TO);
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: target_dir
            .join("agents")
            .join(format!("{}.agent.md", agent.name)),
        content,
    }]
}
```

### Pattern 2: Copilot Command-to-Skill Conversion (COP-01)
**What:** Convert canonical commands to skill directories under `.github/skills/`.
**When to use:** COP-01 requirement.
**Example:**
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let skill_dir = target_dir.join("skills").join(&cmd.name);

    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), json!(cmd.name));
    if let Some(desc) = cmd.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    let body = rewrite_paths(&cmd.body, COPILOT_PATH_FROM, COPILOT_PATH_TO);
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: skill_dir.join("SKILL.md"),
        content,
    }]
}
```

### Pattern 3: Copilot Hook JSON Generation (COP-03)
**What:** Generate `.github/hooks/memory-hooks.json` and `.github/hooks/scripts/memory-capture.sh`.
**When to use:** COP-03 requirement.
**Key format details from existing adapter:**
```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [{ "type": "command", "bash": ".github/hooks/scripts/memory-capture.sh sessionStart", "timeoutSec": 10 }],
    "sessionEnd": [{ "type": "command", "bash": ".github/hooks/scripts/memory-capture.sh sessionEnd", "timeoutSec": 10 }],
    "userPromptSubmitted": [{ "type": "command", "bash": ".github/hooks/scripts/memory-capture.sh userPromptSubmitted", "timeoutSec": 10 }],
    "preToolUse": [{ "type": "command", "bash": ".github/hooks/scripts/memory-capture.sh preToolUse", "timeoutSec": 10 }],
    "postToolUse": [{ "type": "command", "bash": ".github/hooks/scripts/memory-capture.sh postToolUse", "timeoutSec": 10 }]
  }
}
```
**Note:** Copilot hooks use `bash` field (not `command`), `timeoutSec` (not `timeout`), and `comment` (not `description`/`name`). The script path is relative to project root. Event names are camelCase.

### Pattern 4: SkillsConverter Pass-Through (SKL-01, SKL-02, SKL-03)
**What:** Near-identical to ClaudeConverter but targeting user-specified directory. Commands become skill directories (like Codex), agents become orchestration skills. No tool name remapping -- Skills uses same names as Claude (pass-through in tool_maps).
**When to use:** All SKL-* requirements.
**Example:**
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let skill_dir = target_dir.join("skills").join(&cmd.name);

    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), json!(cmd.name));
    if let Some(desc) = cmd.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    let body = rewrite_paths(&cmd.body, SKILLS_PATH_FROM, SKILLS_PATH_TO);
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: skill_dir.join("SKILL.md"),
        content,
    }]
}
```

### Pattern 5: Hook Script Embedding
**What:** Embed the hook shell script content as a Rust const string and emit as a ConvertedFile.
**When to use:** HOOK-01 and HOOK-03 requirements.
**Design choice:** The existing adapter hook scripts are ~230 lines of carefully crafted bash with fail-open behavior, session ID synthesis, ANSI stripping, and redaction. Rather than generating these programmatically, embed the proven script content as a const.
**Example:**
```rust
/// Copilot hook capture script.
/// Source: plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh
const COPILOT_HOOK_SCRIPT: &str = include_str!("../../../plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh");

// In generate_guidance:
files.push(ConvertedFile {
    target_path: target_dir.join("hooks/scripts/memory-capture.sh"),
    content: COPILOT_HOOK_SCRIPT.to_string(),
});
```
**Alternative if `include_str!` path is too fragile:** Inline the script as a const string literal.

### Pattern 6: Hook Event Name Mapping (HOOK-02)
**What:** Map canonical event names to runtime-specific casing.
**Mapping table:**

| Canonical Event | Gemini (PascalCase) | Copilot (camelCase) | Claude | OpenCode | Codex |
|----------------|--------------------|--------------------|--------|----------|-------|
| SessionStart | SessionStart | sessionStart | N/A | N/A | N/A |
| SessionEnd | SessionEnd | sessionEnd | N/A | N/A | N/A |
| UserPrompt | BeforeAgent | userPromptSubmitted | N/A | N/A | N/A |
| AssistantResponse | AfterAgent | (not captured) | N/A | N/A | N/A |
| PreToolUse | BeforeTool | preToolUse | N/A | N/A | N/A |
| PostToolUse | AfterTool | postToolUse | N/A | N/A | N/A |

**Important:** Copilot does NOT capture assistant text responses. The Gemini adapter captures `AfterAgent` (assistant response) but Copilot has no equivalent hook. This is a documented limitation in both existing adapters.

### Anti-Patterns to Avoid
- **Generating hook scripts programmatically:** The existing scripts are battle-tested with edge cases (Bug #991 session reuse, ANSI stripping, jq version detection). Do not rewrite; embed or copy the existing scripts.
- **Using `convert_hook` for hook generation:** The per-hook-definition approach does not work because hook JSON is a single file containing all events. Use `generate_guidance` to produce the hooks JSON file and companion script as ConvertedFiles.
- **Adding YAML canonical hook files to the source tree:** This would require parser changes and is unnecessary since each runtime's hook format is radically different. Keep hooks as converter-generated output.
- **Overcomplicating SkillsConverter:** It should be the simplest converter. No tool remapping, no format transformation, just path rewriting and directory structure.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tool name mapping | Inline match in converter | `tool_maps::map_tool(Runtime::Copilot/Skills, ...)` | Centralized, tested, exhaustive |
| File writing + dry-run | Per-converter write logic | `writer::write_files(&files, dry_run)` | Already tested |
| YAML frontmatter reconstruction | Custom serializer | `helpers::reconstruct_md` | Already tested in Phase 47 |
| Path rewriting | Multiple replace calls | `helpers::rewrite_paths` | Already tested |
| Hook capture scripts | New shell scripts | Embed existing proven scripts from adapters | Battle-tested with edge cases |
| Hook JSON format | Guess the format | Copy structure from existing `memory-hooks.json` | Exact format verified against existing adapter |

**Key insight:** Phase 49 is mostly filling in the last two converter stubs plus moving hook generation from `None` to actual output. The Copilot converter follows the Codex pattern closely. The Skills converter is even simpler. The hook work is the most nuanced but has authoritative references in the existing adapters.

## Common Pitfalls

### Pitfall 1: Copilot Agent File Extension
**What goes wrong:** Using `.md` instead of `.agent.md` for Copilot agents.
**Why it happens:** Other runtimes use plain `.md` for agents. Copilot requires the `.agent.md` suffix for auto-discovery.
**How to avoid:** Target path must be `agents/{name}.agent.md`, not `agents/{name}.md`.
**Warning signs:** Agent not discovered by Copilot CLI.

### Pitfall 2: Copilot Hook JSON Field Names
**What goes wrong:** Using Gemini field names (`command`, `timeout`, `name`) instead of Copilot names (`bash`, `timeoutSec`, `comment`).
**Why it happens:** Copy-pasting from Gemini converter.
**How to avoid:** Use the existing `memory-hooks.json` as the authoritative reference. Key differences:
- Gemini: `"command": "..."`, `"timeout": 5000`, `"name": "..."`, `"description": "..."`
- Copilot: `"bash": "..."`, `"timeoutSec": 10`, `"comment": "..."`
**Warning signs:** Copilot CLI ignores hooks or throws parse errors.

### Pitfall 3: Copilot Hook Script Path
**What goes wrong:** Using absolute `$HOME/.github/hooks/...` path instead of relative `.github/hooks/scripts/...` path.
**Why it happens:** Gemini uses `$HOME/.gemini/hooks/` (absolute). Copilot uses relative paths from project root.
**How to avoid:** Copilot hooks `bash` field uses project-relative paths: `.github/hooks/scripts/memory-capture.sh sessionStart`.
**Warning signs:** Hook script not found when Copilot CLI tries to execute.

### Pitfall 4: Missing Copilot `version` Field in Hooks JSON
**What goes wrong:** Generating hooks JSON without the `"version": 1` field.
**Why it happens:** Gemini settings.json does not have a version field.
**How to avoid:** Include `"version": 1` at the top level of `memory-hooks.json`.
**Warning signs:** Copilot CLI rejects hooks JSON.

### Pitfall 5: SkillsConverter Tool Name Pass-Through
**What goes wrong:** Remapping tool names for Skills when they should pass through unchanged.
**Why it happens:** Other converters remap tools. Skills is supposed to be generic/runtime-agnostic.
**How to avoid:** `tool_maps::map_tool(Runtime::Skills, "Read")` already returns `Some("Read")` (same as Claude). Use this or skip tool mapping entirely for Skills.
**Warning signs:** Tool references in output use runtime-specific names instead of canonical Claude names.

### Pitfall 6: The Stub Test in converter.rs
**What goes wrong:** The `unimplemented_converters_return_empty_results` test (line 78-100 of converter.rs) asserts Copilot and Skills return empty. Filling in stubs breaks this test.
**Why it happens:** Phase 48 updated the test to only check Copilot and Skills (previously all 4 stubs).
**How to avoid:** Remove or update this test. Replace with positive assertions that Copilot and Skills produce correct output.
**Warning signs:** `cargo test` fails immediately.

### Pitfall 7: Hook Script Should Not Be Path-Rewritten
**What goes wrong:** Applying `rewrite_paths` to hook script content, replacing `~/.claude/` references.
**Why it happens:** Habit from command/agent conversion where path rewriting is standard.
**How to avoid:** Hook scripts reference `memory-ingest` (on PATH) and use `$HOME` or relative paths. They do not contain `~/.claude/` references. The script content should be emitted verbatim.
**Warning signs:** Broken shell script after path rewrite.

### Pitfall 8: Copilot Event Name for User Prompt
**What goes wrong:** Using `userPrompt` instead of `userPromptSubmitted`.
**Why it happens:** Abbreviating the event name.
**How to avoid:** The exact event name is `userPromptSubmitted` per the existing hooks JSON.
**Warning signs:** User prompts not captured.

## Code Examples

### Copilot Agent Conversion (COP-02)
```rust
// Source: plugins/memory-copilot-adapter/.github/agents/memory-navigator.agent.md
// Target format:
// ---
// name: memory-navigator
// description: |
//   Autonomous agent for intelligent memory retrieval...
// tools: ["execute", "read", "search"]
// infer: true
// ---

fn build_copilot_tools(agent: &PluginAgent) -> Vec<String> {
    let tools_val = match agent.frontmatter.get("allowed-tools") {
        Some(v) => v,
        None => return Vec::new(),
    };
    let tools_arr = match tools_val.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let mut result = Vec::new();
    for tool in tools_arr {
        let name = match tool.as_str() {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with("mcp__") {
            continue;
        }
        if let Some(mapped) = map_tool(Runtime::Copilot, name) {
            result.push(mapped.to_string());
        }
    }
    // Deduplicate (Write and Edit both map to "edit")
    result.sort();
    result.dedup();
    result
}
```

### Copilot Hooks JSON Generation (COP-03)
```rust
fn generate_copilot_hooks_json(script_path: &str) -> serde_json::Value {
    let hook_entry = |event: &str, comment: &str| -> serde_json::Value {
        json!([{
            "type": "command",
            "bash": format!("{script_path} {event}"),
            "timeoutSec": 10,
            "comment": comment
        }])
    };

    json!({
        "version": 1,
        "hooks": {
            "sessionStart": hook_entry("sessionStart",
                "Capture session start into agent-memory with synthesized session ID"),
            "sessionEnd": hook_entry("sessionEnd",
                "Capture session end into agent-memory and clean up session temp file"),
            "userPromptSubmitted": hook_entry("userPromptSubmitted",
                "Capture user prompts into agent-memory"),
            "preToolUse": hook_entry("preToolUse",
                "Capture tool invocations into agent-memory"),
            "postToolUse": hook_entry("postToolUse",
                "Capture tool results into agent-memory")
        }
    })
}
```

### SkillsConverter Command-to-Skill (SKL-01, SKL-02)
```rust
fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target_dir = self.target_dir(&cfg.scope);
    let skill_dir = target_dir.join("skills").join(&cmd.name);

    let mut fm = serde_json::Map::new();
    fm.insert("name".to_string(), json!(cmd.name));
    if let Some(desc) = cmd.frontmatter.get("description") {
        fm.insert("description".to_string(), desc.clone());
    }

    let body = rewrite_paths(&cmd.body, SKILLS_PATH_FROM, SKILLS_PATH_TO);
    let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

    vec![ConvertedFile {
        target_path: skill_dir.join("SKILL.md"),
        content,
    }]
}
```

### Copilot generate_guidance (Full Hook Pipeline)
```rust
fn generate_guidance(&self, _bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
    let target = self.target_dir(&cfg.scope);
    let mut files = Vec::new();

    // 1. Generate memory-hooks.json
    let script_path = ".github/hooks/scripts/memory-capture.sh";
    let hooks_json = generate_copilot_hooks_json(script_path);
    files.push(ConvertedFile {
        target_path: target.join("hooks/memory-hooks.json"),
        content: serde_json::to_string_pretty(&hooks_json).unwrap_or_default(),
    });

    // 2. Generate hook capture script
    files.push(ConvertedFile {
        target_path: target.join("hooks/scripts/memory-capture.sh"),
        content: COPILOT_HOOK_SCRIPT.to_string(),
    });

    files
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual Copilot adapter in `plugins/memory-copilot-adapter/` | Auto-generated by `memory-installer` | Phase 49 (now) | Single canonical source, automated conversion |
| No generic skills converter | `--agent skills` installs to any directory | Phase 49 (now) | Any compatible runtime can use the output |
| `convert_hook` returns None everywhere | Hooks generated via `generate_guidance` | Phase 49 (now) | Gemini and Copilot get hook configs + scripts |
| Manual hook scripts per adapter | Scripts embedded/copied by installer | Phase 49 (now) | Consistent hook deployment |

**Deprecated/outdated:**
- `plugins/memory-copilot-adapter/`: Will be replaced by installer output. Archive (not delete) per project decision.

## Open Questions

1. **Hook Script Embedding vs include_str!**
   - What we know: The existing Copilot and Gemini hook scripts are ~200-230 lines of carefully crafted bash. They need to be emitted verbatim.
   - What's unclear: Whether to use `include_str!("../path/to/script")` (fragile path) or inline as a const string literal.
   - Recommendation: Use `include_str!` with the path to the existing adapter script. If the path is fragile across builds, fall back to inlining the script content. The `include_str!` approach is preferable because it keeps the script as a single source of truth.

2. **CANON-02: Canonical Hook Definition Format**
   - What we know: CANON-02 requires "canonical hook definitions in YAML format." However, each runtime's hook format is radically different (JSON structures, shell scripts, event names, field names).
   - What's unclear: Whether CANON-02 means a YAML definition that gets converted, or just that hooks are well-defined per runtime.
   - Recommendation: Satisfy CANON-02 by defining canonical hook metadata (event names, descriptions, script behavior) as constants/structs in the converter code, not as YAML files. The per-runtime hook generation in `generate_guidance` is the practical approach. A YAML file would add complexity without benefit since conversion is per-runtime anyway.

3. **OpenCode Hooks**
   - What we know: OpenCode is still a stub (Phase 47 scope, not yet implemented). Its hook API shape was flagged as needing verification.
   - What's unclear: What OpenCode's hook format looks like.
   - Recommendation: Leave OpenCode `convert_hook` as None. OpenCode hooks are out of scope for Phase 49 (Phase 47 owns the OpenCode converter). Phase 49 focuses on Copilot and generic hooks only.

4. **Claude Hooks**
   - What we know: Claude Code has a hooks system but the canonical source is the Claude plugin format itself. No separate hook conversion needed.
   - What's unclear: Nothing -- Claude hooks are handled natively.
   - Recommendation: Claude `convert_hook` remains None. Claude uses its own hook format natively.

5. **Windows Hook Script Strategy**
   - What we know: STATE.md flags "Windows hook script strategy (WSL vs .bat/.ps1) must be decided before Phase 49."
   - What's unclear: Whether to generate `.bat`/`.ps1` wrappers alongside `.sh` scripts.
   - Recommendation: Per REQUIREMENTS.md "Out of Scope" section: "Windows PowerShell hooks -- Shell scripts with WSL sufficient for MVP; PS1 hooks deferred." Generate bash scripts only. Windows users use WSL or Git Bash.

6. **Copilot Target Directory Structure**
   - What we know: Existing adapter uses `.github/` as root with `agents/`, `skills/`, `hooks/` subdirectories. The CopilotConverter stub has `target_dir` returning `.github/copilot`.
   - What's unclear: Whether the target should be `.github/` (matching existing adapter) or `.github/copilot/` (current stub).
   - Recommendation: Change to `.github/` as root to match the existing adapter's proven structure. Copilot CLI discovers agents in `.github/agents/`, skills in `.github/skills/`, hooks in `.github/hooks/`. The `copilot` subdirectory would break discovery.

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
| COP-01 | Commands -> `.github/skills/<name>/SKILL.md` | unit | `cargo test -p memory-installer converters::copilot::tests::command_to_skill -x` | No - Wave 0 |
| COP-02 | Agents -> `.github/agents/<name>.agent.md` with Copilot tool names | unit | `cargo test -p memory-installer converters::copilot::tests::agent_to_agent_md -x` | No - Wave 0 |
| COP-03 | Hooks -> `.github/hooks/memory-hooks.json` + script | unit | `cargo test -p memory-installer converters::copilot::tests::hooks_json_generation -x` | No - Wave 0 |
| SKL-01 | Custom dir targeting via --dir | unit | `cargo test -p memory-installer converters::skills::tests::custom_dir_targeting -x` | No - Wave 0 |
| SKL-02 | Commands/agents -> skill directories | unit | `cargo test -p memory-installer converters::skills::tests::command_to_skill -x` | No - Wave 0 |
| SKL-03 | No tool name remapping (pass-through) | unit | `cargo test -p memory-installer converters::skills::tests::tool_names_passthrough -x` | No - Wave 0 |
| HOOK-01 | Per-runtime hook format generation | unit | `cargo test -p memory-installer converters::copilot::tests::hooks_json_format -x` | No - Wave 0 |
| HOOK-02 | Event name casing per runtime | unit | `cargo test -p memory-installer converters::copilot::tests::event_name_casing -x` | No - Wave 0 |
| HOOK-03 | Fail-open + background execution in script | unit | `cargo test -p memory-installer converters::copilot::tests::hook_script_failopen -x` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`
- **Phase gate:** `task pr-precheck` (format + clippy + test + doc)

### Wave 0 Gaps
- [ ] `converters/copilot.rs` tests -- unit tests for all COP-* converter methods
- [ ] `converters/skills.rs` tests -- unit tests for all SKL-* converter methods
- [ ] Hook generation tests for Copilot (JSON format, script content, event names)
- [ ] Update `converter::tests::unimplemented_converters_return_empty_results` -- remove Copilot/Skills from stub assertion list (no stubs remain after this phase)
- [ ] Verify Copilot `target_dir` change from `.github/copilot` to `.github/` does not break existing test expectations

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `crates/memory-installer/src/` -- all Phase 46-48 infrastructure and converter reference implementations
- Existing Copilot adapter: `plugins/memory-copilot-adapter/.github/` -- `.agent.md` format, `memory-hooks.json` structure, `memory-capture.sh` script (230 lines)
- Existing Gemini adapter: `plugins/memory-gemini-adapter/.gemini/` -- `settings.json` hook format, `memory-capture.sh` script (200 lines)
- tool_maps.rs: Copilot and Skills mappings already present and tested (Copilot same as Codex; Skills same as Claude)
- converter.rs: RuntimeConverter trait and stub test confirmed

### Secondary (MEDIUM confidence)
- Hook event name differences documented in existing scripts (Gemini PascalCase, Copilot camelCase)
- Copilot target directory `.github/` based on existing adapter structure

### Tertiary (LOW confidence)
- `include_str!` path viability for embedding hook scripts -- needs verification against cargo build from workspace root
- CANON-02 interpretation as programmatic constants vs YAML files -- needs user validation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies already in Cargo.toml, no new crates needed
- CopilotConverter architecture: HIGH - existing adapter provides exact target format reference
- SkillsConverter architecture: HIGH - simplest converter, follows established patterns
- Hook generation: HIGH - both existing adapters provide authoritative hook format references
- Event name mapping: HIGH - verified from existing hook scripts and JSON configs
- Hook script embedding: MEDIUM - `include_str!` path feasibility needs build verification
- CANON-02 interpretation: MEDIUM - pragmatic approach (constants not YAML) may not match literal requirement

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable domain, no external dependency changes expected)
