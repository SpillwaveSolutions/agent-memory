use std::path::PathBuf;

use serde_json::json;

use crate::converter::RuntimeConverter;
use crate::tool_maps::map_tool;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill, Runtime, MANAGED_JSON_KEY, MANAGED_JSON_VALUE,
};

use super::helpers::{escape_shell_vars, reconstruct_md, rewrite_paths};

/// Path prefix in canonical source to replace.
const GEMINI_PATH_FROM: &str = "~/.claude/";
/// Replacement path prefix for agent-memory storage.
const GEMINI_PATH_TO: &str = "~/.config/agent-memory/";

pub struct GeminiConverter;

impl RuntimeConverter for GeminiConverter {
    fn name(&self) -> &str {
        "gemini"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join(".gemini"),
            InstallScope::Global => {
                let home = directories::BaseDirs::new()
                    .map(|b| b.home_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~").as_ref()));
                home.join(".gemini")
            }
            InstallScope::Custom(dir) => dir.clone(),
        }
    }

    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);

        let desc = cmd
            .frontmatter
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(&cmd.name);

        let body = escape_shell_vars(&cmd.body);
        let body = rewrite_paths(&body, GEMINI_PATH_FROM, GEMINI_PATH_TO);

        let mut table = toml::map::Map::new();
        table.insert(
            "description".to_string(),
            toml::Value::String(desc.to_string()),
        );
        table.insert("prompt".to_string(), toml::Value::String(body));

        let content =
            toml::to_string_pretty(&toml::Value::Table(table)).unwrap_or_default();

        vec![ConvertedFile {
            target_path: target_dir.join("commands").join(format!("{}.toml", cmd.name)),
            content,
        }]
    }

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&agent.name);

        // Build SKILL.md frontmatter -- strip color and skills fields (GEM-04)
        let mut fm = serde_json::Map::new();
        fm.insert(
            "name".to_string(),
            serde_json::Value::String(agent.name.clone()),
        );
        if let Some(desc) = agent.frontmatter.get("description") {
            fm.insert("description".to_string(), desc.clone());
        }

        // Build tools list: exclude MCP (mcp__*) and map through tool_maps (GEM-02, GEM-03)
        let tools = build_gemini_tools(agent);

        let body = escape_shell_vars(&agent.body);
        let mut body = rewrite_paths(&body, GEMINI_PATH_FROM, GEMINI_PATH_TO);

        // Append tools section if any tools remain after filtering
        if !tools.is_empty() {
            body.push_str("\n\n## Tools\n\n");
            for tool in &tools {
                body.push_str(&format!("- {tool}\n"));
            }
        }

        let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

        vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }]
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&skill.name);

        let body = escape_shell_vars(&skill.body);
        let body = rewrite_paths(&body, GEMINI_PATH_FROM, GEMINI_PATH_TO);
        let content = reconstruct_md(&skill.frontmatter, &body);

        let mut files = vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }];

        for additional in &skill.additional_files {
            let rewritten = escape_shell_vars(&additional.content);
            let rewritten = rewrite_paths(&rewritten, GEMINI_PATH_FROM, GEMINI_PATH_TO);
            files.push(ConvertedFile {
                target_path: skill_dir.join(&additional.relative_path),
                content: rewritten,
            });
        }

        files
    }

    fn convert_hook(&self, _hook: &HookDefinition, _cfg: &InstallConfig) -> Option<ConvertedFile> {
        // Hooks deferred to Phase 49
        None
    }

    fn generate_guidance(
        &self,
        _bundle: &PluginBundle,
        cfg: &InstallConfig,
    ) -> Vec<ConvertedFile> {
        let target = self.target_dir(&cfg.scope);

        let cmd = "$HOME/.gemini/hooks/memory-capture.sh";

        let settings = json!({
            "_comment": [
                "This file is managed by memory-installer.",
                "Manual edits to the hooks section will be overwritten on next install."
            ],
            MANAGED_JSON_KEY: MANAGED_JSON_VALUE,
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "name": "memory-capture-session-start",
                        "type": "command",
                        "command": cmd,
                        "timeout": 5000,
                        "description": "Capture session start into agent-memory"
                    }]
                }],
                "SessionEnd": [{
                    "hooks": [{
                        "name": "memory-capture-session-end",
                        "type": "command",
                        "command": cmd,
                        "timeout": 5000,
                        "description": "Capture session end into agent-memory"
                    }]
                }],
                "BeforeAgent": [{
                    "hooks": [{
                        "name": "memory-capture-user-prompt",
                        "type": "command",
                        "command": cmd,
                        "timeout": 5000,
                        "description": "Capture user prompts into agent-memory"
                    }]
                }],
                "AfterAgent": [{
                    "hooks": [{
                        "name": "memory-capture-assistant-response",
                        "type": "command",
                        "command": cmd,
                        "timeout": 5000,
                        "description": "Capture assistant responses into agent-memory"
                    }]
                }],
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
            }
        });

        let content = serde_json::to_string_pretty(&settings).unwrap_or_default();

        vec![ConvertedFile {
            target_path: target.join("settings.json"),
            content,
        }]
    }
}

/// Build Gemini tool names from an agent's `allowed-tools` frontmatter array.
///
/// Skips MCP tools (`mcp__*` prefix) and excludes tools that map to `None`
/// (e.g., `Task` for Gemini).
fn build_gemini_tools(agent: &PluginAgent) -> Vec<String> {
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
        // Skip MCP tools (GEM-03)
        if name.starts_with("mcp__") {
            continue;
        }
        // Map through tool_maps; None means excluded (e.g., Task)
        if let Some(mapped) = map_tool(Runtime::Gemini, name) {
            result.push(mapped.to_string());
        }
    }
    result
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

    #[test]
    fn command_to_toml_format() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "memory-search".to_string(),
            frontmatter: json!({"description": "Search past conversations"}),
            body: "Search for things in ~/.claude/data".to_string(),
            source_path: PathBuf::from("commands/memory-search.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.gemini/commands/memory-search.toml")
        );
        // Verify TOML format
        assert!(files[0].content.contains("description = "));
        assert!(files[0].content.contains("Search past conversations"));
        assert!(files[0].content.contains("prompt = "));
        // Verify path rewriting
        assert!(files[0].content.contains("~/.config/agent-memory/data"));
        assert!(!files[0].content.contains("~/.claude/data"));
    }

    #[test]
    fn agent_to_skill_directory() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "memory-navigator".to_string(),
            frontmatter: json!({
                "description": "Navigate memories",
                "color": "#0000FF",
                "skills": ["search", "recall"],
                "allowed-tools": ["Read", "Bash", "Grep"]
            }),
            body: "Navigator instructions for ~/.claude/skills".to_string(),
            source_path: PathBuf::from("agents/memory-navigator.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.gemini/skills/memory-navigator/SKILL.md")
        );
        // Verify color and skills are stripped (GEM-04)
        assert!(!files[0].content.contains("color:"));
        assert!(!files[0].content.contains("skills:"));
        assert!(!files[0].content.contains("#0000FF"));
        // Has name and description
        assert!(files[0].content.contains("name: memory-navigator"));
        assert!(files[0].content.contains("description: Navigate memories"));
        // Path rewriting
        assert!(files[0].content.contains("~/.config/agent-memory/skills"));
    }

    #[test]
    fn mcp_and_task_tools_excluded() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test-agent".to_string(),
            frontmatter: json!({
                "description": "Test",
                "allowed-tools": ["Read", "Task", "mcp__custom_tool", "Bash"]
            }),
            body: "Test body".to_string(),
            source_path: PathBuf::from("agents/test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        let content = &files[0].content;
        // Task should not appear (mapped to None for Gemini)
        assert!(!content.contains("Task"));
        // MCP tools should not appear
        assert!(!content.contains("mcp__"));
        // Read and Bash should be mapped to Gemini names
        assert!(content.contains("read_file"));
        assert!(content.contains("run_shell_command"));
    }

    #[test]
    fn skill_conversion_with_additional_files() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let skill = PluginSkill {
            name: "memory-query".to_string(),
            frontmatter: json!({"description": "Query skill"}),
            body: "Query ~/.claude/data with ${HOME}".to_string(),
            source_path: PathBuf::from("skills/memory-query/SKILL.md"),
            additional_files: vec![SkillFile {
                relative_path: PathBuf::from("rules/search.md"),
                content: "Rule: use ~/.claude/db for ${HOME} searches".to_string(),
            }],
        };

        let files = converter.convert_skill(&skill, &cfg);
        assert_eq!(files.len(), 2);

        // SKILL.md
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.gemini/skills/memory-query/SKILL.md")
        );
        assert!(files[0].content.contains("~/.config/agent-memory/data"));
        assert!(files[0].content.contains("$HOME"));
        assert!(!files[0].content.contains("${HOME}"));

        // Additional file
        assert_eq!(
            files[1].target_path,
            PathBuf::from("/project/.gemini/skills/memory-query/rules/search.md")
        );
        assert!(files[1].content.contains("~/.config/agent-memory/db"));
        assert!(!files[1].content.contains("${HOME}"));
    }

    #[test]
    fn convert_hook_returns_none() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let hook = HookDefinition {
            name: "test-hook".to_string(),
            frontmatter: serde_json::Value::Null,
            body: String::new(),
            source_path: PathBuf::from("hooks/test.md"),
        };
        assert!(converter.convert_hook(&hook, &cfg).is_none());
    }

    #[test]
    fn settings_json_generation() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.gemini/settings.json")
        );

        let parsed: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();

        // Managed marker
        assert_eq!(parsed[MANAGED_JSON_KEY], MANAGED_JSON_VALUE);

        // Comment array
        assert!(parsed["_comment"].is_array());

        // All 6 hook event types present
        let hooks = &parsed["hooks"];
        assert!(hooks["SessionStart"].is_array());
        assert!(hooks["SessionEnd"].is_array());
        assert!(hooks["BeforeAgent"].is_array());
        assert!(hooks["AfterAgent"].is_array());
        assert!(hooks["BeforeTool"].is_array());
        assert!(hooks["AfterTool"].is_array());

        // BeforeTool and AfterTool have matcher
        assert_eq!(hooks["BeforeTool"][0]["matcher"], "*");
        assert_eq!(hooks["AfterTool"][0]["matcher"], "*");

        // Hook command path
        let cmd_path = hooks["SessionStart"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(cmd_path.contains("memory-capture.sh"));
        assert!(cmd_path.starts_with("$HOME/.gemini/hooks/"));
    }

    #[test]
    fn shell_var_escaping_in_command() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "test".to_string(),
            frontmatter: json!({"description": "Test"}),
            body: "Use ${HOME}/path and ${USER}".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert!(files[0].content.contains("$HOME/path"));
        assert!(files[0].content.contains("$USER"));
        assert!(!files[0].content.contains("${HOME}"));
        assert!(!files[0].content.contains("${USER}"));
    }

    #[test]
    fn command_fallback_description() {
        let converter = GeminiConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "my-command".to_string(),
            frontmatter: serde_json::Value::Null,
            body: "Body content".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        // Falls back to command name as description
        assert!(files[0].content.contains("my-command"));
    }
}
