use std::path::PathBuf;

use serde_json::json;

use crate::converter::RuntimeConverter;
use crate::tool_maps::map_tool;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill, Runtime,
};

use super::helpers::{reconstruct_md, rewrite_paths};

/// Map a named CSS/terminal color to its hex equivalent.
///
/// Returns `Some("#RRGGBB")` for known named colors, or `None` if the input
/// is already hex or unrecognized.
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
        _ => None,
    }
}

/// Build the `tools` object for an OpenCode agent from its `allowed-tools` array.
///
/// - MCP tools (`mcp__*`) pass through unchanged.
/// - Known Claude tools are mapped via `map_tool(Runtime::OpenCode, ...)`.
/// - Unknown tools are silently skipped.
fn build_opencode_tools(agent: &PluginAgent) -> serde_json::Map<String, serde_json::Value> {
    let mut tools = serde_json::Map::new();

    if let Some(arr) = agent
        .frontmatter
        .get("allowed-tools")
        .and_then(|v| v.as_array())
    {
        for item in arr {
            if let Some(name) = item.as_str() {
                if name.starts_with("mcp__") {
                    tools.insert(name.to_string(), json!(true));
                } else if let Some(mapped) = map_tool(Runtime::OpenCode, name) {
                    tools.insert(mapped.to_string(), json!(true));
                }
                // Unknown tools silently skipped
            }
        }
    }

    tools
}

/// Apply ordered path rewrites for OpenCode.
///
/// CRITICAL: `~/.claude/plugins/` must be rewritten BEFORE `~/.claude/`
/// to avoid double-rewriting (the longer match consumes the prefix first).
fn opencode_rewrite_paths(content: &str) -> String {
    // First: longer match
    let pass1 = rewrite_paths(content, "~/.claude/plugins/", "~/.config/opencode/");
    // Second: shorter match (catches remaining ~/.claude/ references)
    rewrite_paths(&pass1, "~/.claude/", "~/.config/opencode/")
}

pub struct OpenCodeConverter;

#[allow(unused_variables)]
impl RuntimeConverter for OpenCodeConverter {
    fn name(&self) -> &str {
        "opencode"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join(".opencode"),
            InstallScope::Global => {
                let config_dir = directories::BaseDirs::new()
                    .map(|b| b.config_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.config").as_ref()));
                config_dir.join("opencode")
            }
            InstallScope::Custom(dir) => dir.clone(),
        }
    }

    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let body = opencode_rewrite_paths(&cmd.body);
        let content = reconstruct_md(&cmd.frontmatter, &body);
        vec![ConvertedFile {
            target_path: target_dir.join("command").join(format!("{}.md", cmd.name)),
            content,
        }]
    }

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);

        // Build new frontmatter, transforming fields as needed.
        let mut new_fm = serde_json::Map::new();

        if let Some(obj) = agent.frontmatter.as_object() {
            for (key, val) in obj {
                match key.as_str() {
                    // Skip name (OpenCode derives from filename) and allowed-tools (replaced by tools object)
                    "name" | "allowed-tools" => continue,
                    "color" => {
                        if let Some(color_str) = val.as_str() {
                            if let Some(hex) = color_to_hex(color_str) {
                                new_fm.insert(
                                    key.clone(),
                                    serde_json::Value::String(hex.to_string()),
                                );
                            } else {
                                // Already hex or unknown -- pass through
                                new_fm.insert(key.clone(), val.clone());
                            }
                        } else {
                            new_fm.insert(key.clone(), val.clone());
                        }
                    }
                    "subagent_type" => {
                        if val.as_str() == Some("general-purpose") {
                            new_fm.insert(
                                key.clone(),
                                serde_json::Value::String("general".to_string()),
                            );
                        } else {
                            new_fm.insert(key.clone(), val.clone());
                        }
                    }
                    _ => {
                        new_fm.insert(key.clone(), val.clone());
                    }
                }
            }
        }

        // Insert tools object
        let tools = build_opencode_tools(agent);
        if !tools.is_empty() {
            new_fm.insert("tools".to_string(), serde_json::Value::Object(tools));
        }

        let fm_value = serde_json::Value::Object(new_fm);
        let body = opencode_rewrite_paths(&agent.body);
        let content = reconstruct_md(&fm_value, &body);

        vec![ConvertedFile {
            target_path: target_dir.join("agent").join(format!("{}.md", agent.name)),
            content,
        }]
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skill").join(&skill.name);

        let body = opencode_rewrite_paths(&skill.body);
        let content = reconstruct_md(&skill.frontmatter, &body);

        let mut files = vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }];

        for additional in &skill.additional_files {
            let rewritten = opencode_rewrite_paths(&additional.content);
            files.push(ConvertedFile {
                target_path: skill_dir.join(&additional.relative_path),
                content: rewritten,
            });
        }

        files
    }

    fn convert_hook(&self, _hook: &HookDefinition, _cfg: &InstallConfig) -> Option<ConvertedFile> {
        // Hooks deferred per CONTEXT.md
        None
    }

    fn generate_guidance(&self, _bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target = self.target_dir(&cfg.scope);
        let json_path = target.join("opencode.json");

        let perm_path = match &cfg.scope {
            InstallScope::Global => "~/.config/opencode/agent-memory/*".to_string(),
            InstallScope::Project(_) => ".opencode/agent-memory/*".to_string(),
            InstallScope::Custom(_) => return Vec::new(),
        };

        // Read existing file if present, merge into it
        let mut root = if json_path.exists() {
            std::fs::read_to_string(&json_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .unwrap_or_else(|| json!({}))
        } else {
            json!({})
        };

        // Ensure permission.read.{perm_path} = "allow"
        let permission = root
            .as_object_mut()
            .unwrap()
            .entry("permission")
            .or_insert_with(|| json!({}));

        let read_section = permission
            .as_object_mut()
            .unwrap()
            .entry("read")
            .or_insert_with(|| json!({}));
        read_section
            .as_object_mut()
            .unwrap()
            .insert(perm_path.clone(), json!("allow"));

        let ext_dir = permission
            .as_object_mut()
            .unwrap()
            .entry("external_directory")
            .or_insert_with(|| json!({}));
        ext_dir
            .as_object_mut()
            .unwrap()
            .insert(perm_path, json!("allow"));

        let content = serde_json::to_string_pretty(&root).unwrap() + "\n";

        vec![ConvertedFile {
            target_path: json_path,
            content,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SkillFile;
    use std::path::PathBuf;

    fn test_config() -> InstallConfig {
        InstallConfig {
            scope: InstallScope::Project(PathBuf::from("/project")),
            dry_run: false,
            source_root: PathBuf::from("/src"),
        }
    }

    // -- OC-01: convert_command --

    #[test]
    fn convert_command_uses_singular_command_directory() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "memory-search".to_string(),
            frontmatter: serde_json::json!({"description": "Search memories"}),
            body: "Search body".to_string(),
            source_path: PathBuf::from("commands/memory-search.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.opencode/command/memory-search.md")
        );
    }

    #[test]
    fn convert_command_rewrites_claude_plugins_paths() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "test".to_string(),
            frontmatter: serde_json::Value::Null,
            body: "Load from ~/.claude/plugins/foo and ~/.claude/data".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert!(files[0].content.contains("~/.config/opencode/foo"));
        assert!(files[0].content.contains("~/.config/opencode/data"));
        assert!(!files[0].content.contains("~/.claude/"));
    }

    #[test]
    fn convert_command_preserves_frontmatter() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "test".to_string(),
            frontmatter: serde_json::json!({"description": "Search memories"}),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert!(files[0].content.contains("description: Search memories"));
    }

    // -- OC-02: convert_agent tools object --

    #[test]
    fn convert_agent_produces_tools_object() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "navigator".to_string(),
            frontmatter: serde_json::json!({
                "description": "Nav agent",
                "allowed-tools": ["Read", "Bash"]
            }),
            body: "Navigate things".to_string(),
            source_path: PathBuf::from("agents/nav.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        let content = &files[0].content;
        assert!(content.contains("tools:"), "should have tools: section");
        assert!(content.contains("read: true"), "Read should map to read");
        assert!(content.contains("bash: true"), "Bash should map to bash");
    }

    #[test]
    fn convert_agent_removes_allowed_tools_key() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "description": "test",
                "allowed-tools": ["Read"]
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            !files[0].content.contains("allowed-tools"),
            "allowed-tools should be removed"
        );
    }

    #[test]
    fn convert_agent_removes_name_key() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "name": "my-agent",
                "description": "test"
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            !files[0].content.contains("name:"),
            "name: should be removed from frontmatter"
        );
    }

    // -- OC-03: tool mapping --

    #[test]
    fn convert_agent_maps_ask_user_question_to_question() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "allowed-tools": ["AskUserQuestion"]
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            files[0].content.contains("question: true"),
            "AskUserQuestion should map to question"
        );
    }

    #[test]
    fn convert_agent_mcp_tools_pass_through() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "allowed-tools": ["mcp__memory"]
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            files[0].content.contains("mcp__memory: true"),
            "MCP tools should pass through unchanged"
        );
    }

    #[test]
    fn convert_agent_unknown_tools_skipped() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "allowed-tools": ["UnknownTool", "Read"]
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            !files[0].content.contains("UnknownTool"),
            "unknown tools should be skipped"
        );
        assert!(
            files[0].content.contains("read: true"),
            "known tools should still be mapped"
        );
    }

    // -- OC-04: color hex conversion --

    #[test]
    fn color_to_hex_cyan() {
        assert_eq!(color_to_hex("cyan"), Some("#00FFFF"));
    }

    #[test]
    fn color_to_hex_blue() {
        assert_eq!(color_to_hex("blue"), Some("#0000FF"));
    }

    #[test]
    fn color_to_hex_already_hex_returns_none() {
        assert_eq!(color_to_hex("#FF0000"), None);
    }

    #[test]
    fn convert_agent_color_name_to_hex() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "color": "cyan"
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        // #00FFFF needs quoting in YAML, so value_to_yaml quotes it
        assert!(
            files[0].content.contains("\"#00FFFF\""),
            "cyan should become #00FFFF (quoted)"
        );
    }

    #[test]
    fn convert_agent_color_hex_passthrough() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "color": "#FF0000"
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            files[0].content.contains("\"#FF0000\""),
            "hex color should pass through (quoted)"
        );
    }

    // -- OC-05: path rewriting --

    #[test]
    fn path_rewriting_plugins_longer_match_first() {
        // ~/.claude/plugins/ -> ~/.config/opencode/ (strips plugins/ prefix)
        let result = opencode_rewrite_paths("~/.claude/plugins/memory-plugin/skills");
        assert_eq!(result, "~/.config/opencode/memory-plugin/skills");
    }

    #[test]
    fn path_rewriting_claude_data() {
        let result = opencode_rewrite_paths("~/.claude/data");
        assert_eq!(result, "~/.config/opencode/data");
    }

    #[test]
    fn path_rewriting_agent_memory_unchanged() {
        let result = opencode_rewrite_paths("~/.config/agent-memory/data");
        assert_eq!(result, "~/.config/agent-memory/data");
    }

    // -- OC-06: generate_guidance --

    #[test]
    fn generate_guidance_produces_opencode_json() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert_eq!(files.len(), 1);
        assert!(
            files[0]
                .target_path
                .to_string_lossy()
                .contains("opencode.json"),
            "should produce opencode.json"
        );
    }

    #[test]
    fn generate_guidance_has_permission_keys() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        let json: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();
        assert!(
            json["permission"]["read"].is_object(),
            "should have permission.read"
        );
        assert!(
            json["permission"]["external_directory"].is_object(),
            "should have permission.external_directory"
        );
    }

    #[test]
    fn generate_guidance_global_scope_uses_config_glob() {
        let converter = OpenCodeConverter;
        let cfg = InstallConfig {
            scope: InstallScope::Global,
            dry_run: false,
            source_root: PathBuf::from("/src"),
        };
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        let json: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();
        assert!(
            json["permission"]["read"]
                .get("~/.config/opencode/agent-memory/*")
                .is_some(),
            "global scope should use ~/.config/opencode/agent-memory/* glob"
        );
    }

    #[test]
    fn generate_guidance_project_scope_uses_dotdir_glob() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        let json: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();
        assert!(
            json["permission"]["read"]
                .get(".opencode/agent-memory/*")
                .is_some(),
            "project scope should use .opencode/agent-memory/* glob"
        );
    }

    // -- OREG-02: glob pattern ends with agent-memory/* --

    #[test]
    fn generate_guidance_glob_ends_with_agent_memory() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert!(
            files[0].content.contains("agent-memory/*"),
            "permission glob should end with agent-memory/*"
        );
    }

    // -- OREG-03: JSON merge --

    #[test]
    fn generate_guidance_merges_with_existing() {
        let converter = OpenCodeConverter;
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg = InstallConfig {
            scope: InstallScope::Project(tmp.path().to_path_buf()),
            dry_run: false,
            source_root: PathBuf::from("/src"),
        };

        // Pre-create opencode.json with existing content
        let opencode_dir = tmp.path().join(".opencode");
        std::fs::create_dir_all(&opencode_dir).unwrap();
        std::fs::write(opencode_dir.join("opencode.json"), r#"{"theme": "dark"}"#).unwrap();

        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        let json: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();

        // Existing content preserved
        assert_eq!(
            json["theme"].as_str(),
            Some("dark"),
            "existing theme key should be preserved"
        );
        // New permission entries added
        assert!(
            json["permission"]["read"].is_object(),
            "permission.read should be added"
        );
    }

    // -- convert_skill --

    #[test]
    fn convert_skill_uses_singular_skill_directory() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let skill = PluginSkill {
            name: "memory-query".to_string(),
            frontmatter: serde_json::json!({"description": "Query skill"}),
            body: "Query body".to_string(),
            source_path: PathBuf::from("skills/memory-query/SKILL.md"),
            additional_files: vec![],
        };

        let files = converter.convert_skill(&skill, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.opencode/skill/memory-query/SKILL.md")
        );
    }

    #[test]
    fn convert_skill_rewrites_paths_in_body_and_additional_files() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let skill = PluginSkill {
            name: "memory-query".to_string(),
            frontmatter: serde_json::json!({"description": "Query skill"}),
            body: "Query ~/.claude/data".to_string(),
            source_path: PathBuf::from("skills/memory-query/SKILL.md"),
            additional_files: vec![SkillFile {
                relative_path: PathBuf::from("rules/search.md"),
                content: "Rule: use ~/.claude/db for searches".to_string(),
            }],
        };

        let files = converter.convert_skill(&skill, &cfg);
        assert_eq!(files.len(), 2);
        assert!(files[0].content.contains("~/.config/opencode/data"));
        assert!(files[1].content.contains("~/.config/opencode/db"));
        assert!(!files[0].content.contains("~/.claude/"));
        assert!(!files[1].content.contains("~/.claude/"));
    }

    // -- convert_hook --

    #[test]
    fn convert_hook_returns_none() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let hook = HookDefinition {
            name: "test-hook".to_string(),
            frontmatter: serde_json::Value::Null,
            body: String::new(),
            source_path: PathBuf::from("hooks/test.md"),
        };
        assert!(converter.convert_hook(&hook, &cfg).is_none());
    }

    // -- subagent_type normalization --

    #[test]
    fn convert_agent_normalizes_subagent_type_general_purpose() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test".to_string(),
            frontmatter: serde_json::json!({
                "subagent_type": "general-purpose"
            }),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert!(
            files[0].content.contains("subagent_type: general"),
            "general-purpose should be normalized to general"
        );
        assert!(
            !files[0].content.contains("general-purpose"),
            "general-purpose should not appear in output"
        );
    }

    // -- target_dir --

    #[test]
    fn target_dir_project_scope() {
        let converter = OpenCodeConverter;
        let dir = converter.target_dir(&InstallScope::Project(PathBuf::from("/myproject")));
        assert_eq!(dir, PathBuf::from("/myproject/.opencode"));
    }

    // -- agent target uses singular agent/ directory --

    #[test]
    fn convert_agent_uses_singular_agent_directory() {
        let converter = OpenCodeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "navigator".to_string(),
            frontmatter: serde_json::json!({"description": "test"}),
            body: "body".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.opencode/agent/navigator.md")
        );
    }

    // -- Custom scope returns empty guidance --

    #[test]
    fn generate_guidance_custom_scope_returns_empty() {
        let converter = OpenCodeConverter;
        let cfg = InstallConfig {
            scope: InstallScope::Custom(PathBuf::from("/custom")),
            dry_run: false,
            source_root: PathBuf::from("/src"),
        };
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert!(files.is_empty(), "Custom scope should return empty");
    }
}
