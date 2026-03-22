//! End-to-end integration tests for all 6 runtime converters.
//!
//! Each test exercises the full pipeline: canonical bundle -> convert all
//! artifact types -> write to temp dir -> verify file structure and content
//! on disk.

use std::path::{Path, PathBuf};

use memory_installer::converters::select_converter;
use memory_installer::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill, Runtime, SkillFile,
};
use memory_installer::writer::write_files;
use tempfile::TempDir;

/// Build the canonical test bundle used by all runtime tests.
///
/// Contains exactly 1 command, 1 agent, 1 skill (with 1 additional file), and 1 hook.
fn canonical_bundle() -> PluginBundle {
    PluginBundle {
        commands: vec![PluginCommand {
            name: "memory-search".to_string(),
            frontmatter: serde_json::json!({
                "description": "Search past conversations for relevant memories",
                "allowed-tools": ["Read", "Bash", "Grep", "mcp__memory", "Task"]
            }),
            body: "Search for memories in ~/.claude/data and return results.".to_string(),
            source_path: PathBuf::from("commands/memory-search.md"),
        }],
        agents: vec![PluginAgent {
            name: "memory-navigator".to_string(),
            frontmatter: serde_json::json!({
                "description": "Navigate and explore stored memories",
                "allowed-tools": ["Read", "Bash", "Grep", "mcp__memory", "Task"]
            }),
            body: "Navigate through ~/.claude/skills for memory lookup.".to_string(),
            source_path: PathBuf::from("agents/memory-navigator.md"),
        }],
        skills: vec![PluginSkill {
            name: "memory-query".to_string(),
            frontmatter: serde_json::json!({
                "description": "Query memories with semantic search"
            }),
            body: "Query ~/.claude/data for semantic matches.".to_string(),
            source_path: PathBuf::from("skills/memory-query/SKILL.md"),
            additional_files: vec![SkillFile {
                relative_path: PathBuf::from("rules/search.md"),
                content: "Rule: use ~/.claude/db for all database searches.".to_string(),
            }],
        }],
        hooks: vec![HookDefinition {
            name: "session-start".to_string(),
            frontmatter: serde_json::json!({"event": "session_start"}),
            body: "Hook body".to_string(),
            source_path: PathBuf::from("hooks/session-start.md"),
        }],
    }
}

/// Convert all artifacts in the canonical bundle for a given runtime,
/// write to disk, and return the collected files for assertion.
fn convert_and_write(runtime: Runtime, dir: &Path) -> Vec<ConvertedFile> {
    let bundle = canonical_bundle();
    let cfg = InstallConfig {
        scope: InstallScope::Project(dir.to_path_buf()),
        dry_run: false,
        source_root: PathBuf::from("/src"),
    };

    let converter = select_converter(runtime);

    let mut all_files: Vec<ConvertedFile> = Vec::new();

    for cmd in &bundle.commands {
        all_files.extend(converter.convert_command(cmd, &cfg));
    }
    for agent in &bundle.agents {
        all_files.extend(converter.convert_agent(agent, &cfg));
    }
    for skill in &bundle.skills {
        all_files.extend(converter.convert_skill(skill, &cfg));
    }
    for hook in &bundle.hooks {
        if let Some(f) = converter.convert_hook(hook, &cfg) {
            all_files.push(f);
        }
    }
    all_files.extend(converter.generate_guidance(&bundle, &cfg));

    write_files(&all_files, false).expect("write_files should succeed");

    all_files
}

// ---------------------------------------------------------------------------
// 1. Claude full bundle (MIG-01 + MIG-02)
// ---------------------------------------------------------------------------

#[test]
fn claude_full_bundle() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let files = convert_and_write(Runtime::Claude, dir);

    // -- File structure --
    let base = dir.join(".claude/plugins/memory-plugin");

    let cmd_path = base.join("commands/memory-search.md");
    assert!(cmd_path.exists(), "expected {cmd_path:?} to exist");

    let agent_path = base.join("agents/memory-navigator.md");
    assert!(agent_path.exists(), "expected {agent_path:?} to exist");

    let skill_path = base.join("skills/memory-query/SKILL.md");
    assert!(skill_path.exists(), "expected {skill_path:?} to exist");

    let rule_path = base.join("skills/memory-query/rules/search.md");
    assert!(rule_path.exists(), "expected {rule_path:?} to exist");

    // -- Content: paths rewritten --
    let cmd_content = std::fs::read_to_string(&cmd_path).unwrap();
    assert!(
        cmd_content.contains("~/.config/agent-memory/data"),
        "command body should have rewritten paths"
    );
    assert!(
        !cmd_content.contains("~/.claude/data"),
        "command body should not contain original paths"
    );

    let agent_content = std::fs::read_to_string(&agent_path).unwrap();
    assert!(
        agent_content.contains("~/.config/agent-memory/skills"),
        "agent body should have rewritten paths"
    );

    let rule_content = std::fs::read_to_string(&rule_path).unwrap();
    assert!(
        rule_content.contains("~/.config/agent-memory/db"),
        "rule file should have rewritten paths"
    );

    // -- Content: frontmatter preserved as YAML --
    assert!(
        cmd_content.contains("description:"),
        "command should have YAML frontmatter"
    );

    // -- No guidance files --
    // Claude converter returns empty guidance; just confirm no extra files
    let guidance_count = files
        .iter()
        .filter(|f| {
            !f.target_path.to_string_lossy().contains("commands")
                && !f.target_path.to_string_lossy().contains("agents")
                && !f.target_path.to_string_lossy().contains("skills")
        })
        .count();
    assert_eq!(guidance_count, 0, "Claude should produce no guidance files");
}

// ---------------------------------------------------------------------------
// 2. Codex full bundle (MIG-01 + MIG-02)
// ---------------------------------------------------------------------------

#[test]
fn codex_full_bundle() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let _files = convert_and_write(Runtime::Codex, dir);

    let base = dir.join(".codex");

    // -- File structure --
    let cmd_skill = base.join("skills/memory-search/SKILL.md");
    assert!(cmd_skill.exists(), "expected {cmd_skill:?} to exist");

    let agent_skill = base.join("skills/memory-navigator/SKILL.md");
    assert!(agent_skill.exists(), "expected {agent_skill:?} to exist");

    let skill_md = base.join("skills/memory-query/SKILL.md");
    assert!(skill_md.exists(), "expected {skill_md:?} to exist");

    let rule_file = base.join("skills/memory-query/rules/search.md");
    assert!(rule_file.exists(), "expected {rule_file:?} to exist");

    let agents_md = base.join("AGENTS.md");
    assert!(agents_md.exists(), "expected {agents_md:?} to exist");

    // -- Content: path rewriting --
    let cmd_content = std::fs::read_to_string(&cmd_skill).unwrap();
    assert!(cmd_content.contains("~/.config/agent-memory/data"));
    assert!(!cmd_content.contains("~/.claude/data"));

    // -- Content: tool names mapped and MCP excluded --
    let agent_content = std::fs::read_to_string(&agent_skill).unwrap();
    assert!(
        !agent_content.contains("mcp__"),
        "MCP tools should be excluded"
    );
    // Read -> read, Bash -> execute for Codex
    assert!(
        agent_content.contains("- execute"),
        "Bash should map to execute"
    );
    assert!(agent_content.contains("- read"), "Read should map to read");

    // -- Content: deduplication applied --
    // Grep maps to a unique name in Codex, so count occurrences
    let tool_lines: Vec<&str> = agent_content
        .lines()
        .filter(|l| l.starts_with("- ") && !l.contains("**"))
        .collect();
    let unique_tools: std::collections::HashSet<&&str> = tool_lines.iter().collect();
    assert_eq!(
        tool_lines.len(),
        unique_tools.len(),
        "tool list should have no duplicates"
    );

    // -- Content: AGENTS.md --
    let agents_content = std::fs::read_to_string(&agents_md).unwrap();
    assert!(
        agents_content.contains("## Available Skills"),
        "AGENTS.md should list skills"
    );
    assert!(
        agents_content.contains("memory-search"),
        "AGENTS.md should reference command-as-skill"
    );
    assert!(
        agents_content.contains("## Agents"),
        "AGENTS.md should have agents section"
    );
    assert!(
        agents_content.contains("memory-navigator"),
        "AGENTS.md should reference agent"
    );

    // -- Content: sandbox recommendations --
    assert!(
        agent_content.contains("## Sandbox"),
        "agent skill should have sandbox section"
    );
    assert!(
        agent_content.contains("read-only"),
        "memory-navigator should get read-only sandbox"
    );
}

// ---------------------------------------------------------------------------
// 3. Gemini full bundle (MIG-01 + MIG-02)
// ---------------------------------------------------------------------------

#[test]
fn gemini_full_bundle() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let _files = convert_and_write(Runtime::Gemini, dir);

    let base = dir.join(".gemini");

    // -- File structure --
    let cmd_toml = base.join("commands/memory-search.toml");
    assert!(cmd_toml.exists(), "expected {cmd_toml:?} to exist");

    let agent_skill = base.join("skills/memory-navigator/SKILL.md");
    assert!(agent_skill.exists(), "expected {agent_skill:?} to exist");

    let skill_md = base.join("skills/memory-query/SKILL.md");
    assert!(skill_md.exists(), "expected {skill_md:?} to exist");

    let rule_file = base.join("skills/memory-query/rules/search.md");
    assert!(rule_file.exists(), "expected {rule_file:?} to exist");

    let settings = base.join("settings.json");
    assert!(settings.exists(), "expected {settings:?} to exist");

    // -- Content: command is TOML --
    let toml_content = std::fs::read_to_string(&cmd_toml).unwrap();
    let toml_val: toml::Value =
        toml::from_str(&toml_content).expect("command file should be valid TOML");
    assert!(
        toml_val.get("description").is_some(),
        "TOML should have description"
    );
    assert!(
        toml_val.get("prompt").is_some(),
        "TOML should have prompt field"
    );

    // -- Content: agent frontmatter lacks color and skills --
    let agent_content = std::fs::read_to_string(&agent_skill).unwrap();
    assert!(
        !agent_content.contains("color:"),
        "Gemini agent should not have color field"
    );
    assert!(
        !agent_content.contains("skills:"),
        "Gemini agent should not have skills field"
    );

    // -- Content: Task tool excluded (maps to None) --
    // Task should not appear in the tools section
    // Check that the tools section doesn't contain Task
    if agent_content.contains("## Tools") {
        assert!(
            !agent_content.contains("- Task"),
            "Task should be excluded for Gemini"
        );
    }

    // -- Content: MCP tools excluded --
    assert!(
        !agent_content.contains("mcp__"),
        "MCP tools should be excluded"
    );

    // -- Content: settings.json --
    let settings_content = std::fs::read_to_string(&settings).unwrap();
    let settings_json: serde_json::Value =
        serde_json::from_str(&settings_content).expect("settings.json should be valid JSON");
    assert!(
        settings_json.get("__managed_by").is_some(),
        "settings.json should have __managed_by marker"
    );

    // -- Content: ${HOME} escaped to $HOME --
    // The canonical bundle doesn't use ${HOME}, but verify path rewriting works
    let prompt_str = toml_val
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        prompt_str.contains("~/.config/agent-memory/data"),
        "TOML prompt should have rewritten paths"
    );
}

// ---------------------------------------------------------------------------
// 4. Copilot full bundle (MIG-01 + MIG-02)
// ---------------------------------------------------------------------------

#[test]
fn copilot_full_bundle() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let _files = convert_and_write(Runtime::Copilot, dir);

    let base = dir.join(".github");

    // -- File structure --
    let cmd_skill = base.join("skills/memory-search/SKILL.md");
    assert!(cmd_skill.exists(), "expected {cmd_skill:?} to exist");

    let agent_file = base.join("agents/memory-navigator.agent.md");
    assert!(agent_file.exists(), "expected {agent_file:?} to exist");

    let skill_md = base.join("skills/memory-query/SKILL.md");
    assert!(skill_md.exists(), "expected {skill_md:?} to exist");

    let rule_file = base.join("skills/memory-query/rules/search.md");
    assert!(rule_file.exists(), "expected {rule_file:?} to exist");

    let hooks_json = base.join("hooks/memory-hooks.json");
    assert!(hooks_json.exists(), "expected {hooks_json:?} to exist");

    let capture_script = base.join("hooks/scripts/memory-capture.sh");
    assert!(
        capture_script.exists(),
        "expected {capture_script:?} to exist"
    );

    // -- Content: agent file named .agent.md --
    // (already verified by path above)

    // -- Content: agent frontmatter has infer and tools --
    let agent_content = std::fs::read_to_string(&agent_file).unwrap();
    assert!(
        agent_content.contains("infer: true"),
        "agent should have infer: true"
    );
    assert!(
        agent_content.contains("tools:"),
        "agent should have tools array"
    );

    // -- Content: hooks JSON --
    let hooks_content = std::fs::read_to_string(&hooks_json).unwrap();
    let hooks_val: serde_json::Value =
        serde_json::from_str(&hooks_content).expect("hooks JSON should be valid");

    // camelCase events
    let hooks_obj = &hooks_val["hooks"];
    assert!(
        hooks_obj.get("sessionStart").is_some(),
        "should have sessionStart"
    );
    assert!(
        hooks_obj.get("sessionEnd").is_some(),
        "should have sessionEnd"
    );

    // Copilot-specific field names: bash/timeoutSec/comment (NOT command/timeout/description)
    let entry = &hooks_obj["sessionStart"][0];
    assert!(
        entry.get("bash").is_some(),
        "hook entry must have 'bash' field"
    );
    assert!(
        entry.get("timeoutSec").is_some(),
        "hook entry must have 'timeoutSec' field"
    );
    assert!(
        entry.get("comment").is_some(),
        "hook entry must have 'comment' field"
    );
    assert!(
        entry.get("command").is_none(),
        "hook entry must NOT have Gemini's 'command' field"
    );
    assert!(
        entry.get("timeout").is_none(),
        "hook entry must NOT have Gemini's 'timeout' field"
    );
    assert!(
        entry.get("description").is_none(),
        "hook entry must NOT have Gemini's 'description' field"
    );

    // -- Content: capture script --
    let script = std::fs::read_to_string(&capture_script).unwrap();
    assert!(!script.is_empty(), "capture script should be non-empty");
    assert!(
        script.contains("trap"),
        "capture script should contain trap for fail-open"
    );
    assert!(
        script.contains("exit 0"),
        "capture script should contain exit 0 for fail-open"
    );

    // -- Content: path rewriting --
    let cmd_content = std::fs::read_to_string(&cmd_skill).unwrap();
    assert!(cmd_content.contains("~/.config/agent-memory/data"));
    assert!(!cmd_content.contains("~/.claude/data"));
}

// ---------------------------------------------------------------------------
// 5. Skills full bundle (MIG-01 + MIG-02)
// ---------------------------------------------------------------------------

#[test]
fn skills_full_bundle() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let files = convert_and_write(Runtime::Skills, dir);

    let base = dir.join("skills");

    // -- File structure --
    let cmd_skill = base.join("memory-search/SKILL.md");
    assert!(cmd_skill.exists(), "expected {cmd_skill:?} to exist");

    let agent_skill = base.join("memory-navigator/SKILL.md");
    assert!(agent_skill.exists(), "expected {agent_skill:?} to exist");

    let skill_md = base.join("memory-query/SKILL.md");
    assert!(skill_md.exists(), "expected {skill_md:?} to exist");

    let rule_file = base.join("memory-query/rules/search.md");
    assert!(rule_file.exists(), "expected {rule_file:?} to exist");

    // -- Content: canonical Claude tool names (NOT remapped) --
    let agent_content = std::fs::read_to_string(&agent_skill).unwrap();
    assert!(
        agent_content.contains("- Read"),
        "Skills should use canonical Read"
    );
    assert!(
        agent_content.contains("- Bash"),
        "Skills should use canonical Bash"
    );
    assert!(
        agent_content.contains("- Grep"),
        "Skills should use canonical Grep"
    );

    // -- Content: MCP tools excluded --
    assert!(
        !agent_content.contains("mcp__"),
        "MCP tools should be excluded"
    );

    // -- No guidance files --
    let guidance_count = files
        .iter()
        .filter(|f| {
            !f.target_path.to_string_lossy().contains("SKILL.md")
                && !f.target_path.to_string_lossy().contains("rules/")
        })
        .count();
    assert_eq!(guidance_count, 0, "Skills should produce no guidance files");

    // -- Content: path rewriting --
    let cmd_content = std::fs::read_to_string(&cmd_skill).unwrap();
    assert!(cmd_content.contains("~/.config/agent-memory/data"));
    assert!(!cmd_content.contains("~/.claude/data"));
}

// ---------------------------------------------------------------------------
// 6. OpenCode stub (MIG-01)
// ---------------------------------------------------------------------------

#[test]
fn opencode_stub() {
    let bundle = canonical_bundle();
    let cfg = InstallConfig {
        scope: InstallScope::Project(PathBuf::from("/tmp/opencode-test")),
        dry_run: false,
        source_root: PathBuf::from("/src"),
    };

    let converter = select_converter(Runtime::OpenCode);

    // Converter name
    assert_eq!(converter.name(), "opencode");

    // All convert methods return empty
    for cmd in &bundle.commands {
        assert!(
            converter.convert_command(cmd, &cfg).is_empty(),
            "OpenCode convert_command should return empty"
        );
    }
    for agent in &bundle.agents {
        assert!(
            converter.convert_agent(agent, &cfg).is_empty(),
            "OpenCode convert_agent should return empty"
        );
    }
    for skill in &bundle.skills {
        assert!(
            converter.convert_skill(skill, &cfg).is_empty(),
            "OpenCode convert_skill should return empty"
        );
    }
    for hook in &bundle.hooks {
        assert!(
            converter.convert_hook(hook, &cfg).is_none(),
            "OpenCode convert_hook should return None"
        );
    }

    // generate_guidance returns empty
    assert!(
        converter.generate_guidance(&bundle, &cfg).is_empty(),
        "OpenCode generate_guidance should return empty"
    );
}

// ---------------------------------------------------------------------------
// 7. CI workspace includes memory-installer (MIG-04)
// ---------------------------------------------------------------------------

#[test]
fn ci_workspace_includes_installer() {
    // Find the workspace root Cargo.toml by using CARGO_MANIFEST_DIR to navigate up.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("should be able to find workspace root")
        .join("Cargo.toml");
    let workspace_toml = std::fs::read_to_string(&workspace_root)
        .unwrap_or_else(|e| panic!("should read {workspace_root:?}: {e}"));
    assert!(
        workspace_toml.contains("crates/memory-installer"),
        "Cargo.toml workspace members should include crates/memory-installer (MIG-04)"
    );
}
