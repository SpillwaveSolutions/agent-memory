use std::path::PathBuf;

use serde_json::json;

use crate::converter::RuntimeConverter;
use crate::tool_maps::map_tool;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill, Runtime,
};

use super::helpers::{reconstruct_md, rewrite_paths};

/// Path prefix in canonical source to replace.
const COPILOT_PATH_FROM: &str = "~/.claude/";
/// Replacement path prefix for agent-memory storage.
const COPILOT_PATH_TO: &str = "~/.config/agent-memory/";

/// Embedded hook capture script from the canonical Copilot adapter.
const HOOK_CAPTURE_SCRIPT: &str = include_str!(
    "../../../../plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh"
);

pub struct CopilotConverter;

impl RuntimeConverter for CopilotConverter {
    fn name(&self) -> &str {
        "copilot"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join(".github"),
            InstallScope::Global => {
                let config_dir = directories::BaseDirs::new()
                    .map(|b| b.config_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.config").as_ref()));
                config_dir.join("github-copilot")
            }
            InstallScope::Custom(dir) => dir.clone(),
        }
    }

    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&cmd.name);

        let mut fm = serde_json::Map::new();
        fm.insert(
            "name".to_string(),
            serde_json::Value::String(cmd.name.clone()),
        );
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

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);

        let mut fm = serde_json::Map::new();
        fm.insert(
            "name".to_string(),
            serde_json::Value::String(agent.name.clone()),
        );
        if let Some(desc) = agent.frontmatter.get("description") {
            fm.insert("description".to_string(), desc.clone());
        }

        // Map tools from allowed-tools frontmatter
        let mut tools: Vec<String> = Vec::new();
        if let Some(allowed) = agent
            .frontmatter
            .get("allowed-tools")
            .and_then(|v| v.as_array())
        {
            for tool_val in allowed {
                if let Some(tool_name) = tool_val.as_str() {
                    // Skip MCP tools
                    if tool_name.starts_with("mcp__") {
                        continue;
                    }
                    if let Some(mapped) = map_tool(Runtime::Copilot, tool_name) {
                        tools.push(mapped.to_string());
                    }
                }
            }
        }
        // Deduplicate (Copilot maps Write and Edit both to "edit", etc.)
        tools.sort();
        tools.dedup();

        fm.insert(
            "tools".to_string(),
            serde_json::Value::Array(tools.iter().map(|t| json!(t)).collect()),
        );
        fm.insert("infer".to_string(), serde_json::Value::Bool(true));

        let body = rewrite_paths(&agent.body, COPILOT_PATH_FROM, COPILOT_PATH_TO);
        let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

        vec![ConvertedFile {
            target_path: target_dir
                .join("agents")
                .join(format!("{}.agent.md", agent.name)),
            content,
        }]
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&skill.name);

        let body = rewrite_paths(&skill.body, COPILOT_PATH_FROM, COPILOT_PATH_TO);
        let content = reconstruct_md(&skill.frontmatter, &body);

        let mut files = vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }];

        for additional in &skill.additional_files {
            let rewritten = rewrite_paths(&additional.content, COPILOT_PATH_FROM, COPILOT_PATH_TO);
            files.push(ConvertedFile {
                target_path: skill_dir.join(&additional.relative_path),
                content: rewritten,
            });
        }

        files
    }

    fn convert_hook(&self, _hook: &HookDefinition, _cfg: &InstallConfig) -> Option<ConvertedFile> {
        // Hooks are generated via generate_guidance, not per-hook conversion
        None
    }

    fn generate_guidance(&self, _bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target = self.target_dir(&cfg.scope);
        let script_path = ".github/hooks/scripts/memory-capture.sh";

        let hooks_json = generate_copilot_hooks_json(script_path);
        let content = serde_json::to_string_pretty(&hooks_json).unwrap_or_default();

        vec![
            ConvertedFile {
                target_path: target.join("hooks/memory-hooks.json"),
                content,
            },
            ConvertedFile {
                target_path: target.join("hooks/scripts/memory-capture.sh"),
                content: HOOK_CAPTURE_SCRIPT.to_string(),
            },
        ]
    }
}

/// Build the Copilot hooks JSON with the exact format from the canonical adapter.
///
/// Structure: `{ version: 1, hooks: { <event>: [{ type, bash, timeoutSec, comment }] } }`
fn generate_copilot_hooks_json(script_path: &str) -> serde_json::Value {
    json!({
        "version": 1,
        "hooks": {
            "sessionStart": [{
                "type": "command",
                "bash": format!("{script_path} sessionStart"),
                "timeoutSec": 10,
                "comment": "Capture session start into agent-memory with synthesized session ID"
            }],
            "sessionEnd": [{
                "type": "command",
                "bash": format!("{script_path} sessionEnd"),
                "timeoutSec": 10,
                "comment": "Capture session end into agent-memory and clean up session temp file"
            }],
            "userPromptSubmitted": [{
                "type": "command",
                "bash": format!("{script_path} userPromptSubmitted"),
                "timeoutSec": 10,
                "comment": "Capture user prompts into agent-memory"
            }],
            "preToolUse": [{
                "type": "command",
                "bash": format!("{script_path} preToolUse"),
                "timeoutSec": 10,
                "comment": "Capture tool invocations into agent-memory"
            }],
            "postToolUse": [{
                "type": "command",
                "bash": format!("{script_path} postToolUse"),
                "timeoutSec": 10,
                "comment": "Capture tool results into agent-memory"
            }]
        }
    })
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
    fn command_to_skill() {
        let converter = CopilotConverter;
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
            PathBuf::from("/project/.github/skills/memory-search/SKILL.md")
        );
        // Verify YAML frontmatter contains name and description
        assert!(files[0].content.contains("name: memory-search"));
        assert!(files[0]
            .content
            .contains("description: Search past conversations"));
        // Verify path rewriting
        assert!(files[0].content.contains("~/.config/agent-memory/data"));
        assert!(!files[0].content.contains("~/.claude/data"));
    }

    #[test]
    fn agent_to_agent_md() {
        let converter = CopilotConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "memory-navigator".to_string(),
            frontmatter: json!({
                "description": "Navigate memory",
                "allowed-tools": ["Read", "Bash", "mcp__memory", "Write", "Edit"]
            }),
            body: "Navigate through ~/.claude/skills for lookup".to_string(),
            source_path: PathBuf::from("agents/memory-navigator.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.github/agents/memory-navigator.agent.md")
        );
        let content = &files[0].content;
        // Verify frontmatter
        assert!(content.contains("name: memory-navigator"));
        assert!(content.contains("description: Navigate memory"));
        assert!(content.contains("infer: true"));
        // Verify tools array -- mcp__ excluded, Write+Edit both map to "edit" (deduped)
        assert!(content.contains("tools:"));
        assert!(content.contains("- edit"));
        assert!(content.contains("- execute"));
        assert!(content.contains("- read"));
        assert!(!content.contains("mcp__"));
        // Verify path rewriting
        assert!(content.contains("~/.config/agent-memory/skills"));
    }

    #[test]
    fn skill_with_additional_files() {
        let converter = CopilotConverter;
        let cfg = test_config();
        let skill = PluginSkill {
            name: "memory-query".to_string(),
            frontmatter: json!({"description": "Query skill"}),
            body: "Query ~/.claude/data".to_string(),
            source_path: PathBuf::from("skills/memory-query/SKILL.md"),
            additional_files: vec![SkillFile {
                relative_path: PathBuf::from("rules/search.md"),
                content: "Rule: use ~/.claude/db for searches".to_string(),
            }],
        };

        let files = converter.convert_skill(&skill, &cfg);
        assert_eq!(files.len(), 2);

        // SKILL.md
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.github/skills/memory-query/SKILL.md")
        );
        assert!(files[0].content.contains("~/.config/agent-memory/data"));

        // Additional file
        assert_eq!(
            files[1].target_path,
            PathBuf::from("/project/.github/skills/memory-query/rules/search.md")
        );
        assert!(files[1].content.contains("~/.config/agent-memory/db"));
        assert!(!files[1].content.contains("~/.claude/db"));
    }

    #[test]
    fn target_dir_project_scope() {
        let converter = CopilotConverter;
        let scope = InstallScope::Project(PathBuf::from("/project"));
        let dir = converter.target_dir(&scope);
        assert_eq!(dir, PathBuf::from("/project/.github"));
    }

    #[test]
    fn convert_hook_returns_none() {
        let converter = CopilotConverter;
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
    fn hooks_json_generation() {
        let converter = CopilotConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert_eq!(files.len(), 2);

        // First file: hooks JSON
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.github/hooks/memory-hooks.json")
        );

        let parsed: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();
        assert_eq!(parsed["version"], 1);

        // All 5 camelCase events present
        let hooks = &parsed["hooks"];
        assert!(hooks["sessionStart"].is_array());
        assert!(hooks["sessionEnd"].is_array());
        assert!(hooks["userPromptSubmitted"].is_array());
        assert!(hooks["preToolUse"].is_array());
        assert!(hooks["postToolUse"].is_array());
    }

    #[test]
    fn hooks_json_field_names() {
        let converter = CopilotConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        let parsed: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();

        // Verify Copilot-specific field names (not Gemini field names)
        let entry = &parsed["hooks"]["sessionStart"][0];
        assert!(entry.get("bash").is_some(), "must have 'bash' field");
        assert!(
            entry.get("timeoutSec").is_some(),
            "must have 'timeoutSec' field"
        );
        assert!(entry.get("comment").is_some(), "must have 'comment' field");
        // Must NOT have Gemini field names
        assert!(
            entry.get("command").is_none(),
            "must not have 'command' field"
        );
        assert!(
            entry.get("timeout").is_none(),
            "must not have 'timeout' field"
        );
        assert!(
            entry.get("description").is_none(),
            "must not have 'description' field"
        );
        assert!(entry.get("name").is_none(), "must not have 'name' field");
    }

    #[test]
    fn hook_script_content() {
        let converter = CopilotConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert_eq!(files.len(), 2);

        // Second file: script
        assert_eq!(
            files[1].target_path,
            PathBuf::from("/project/.github/hooks/scripts/memory-capture.sh")
        );
        let script = &files[1].content;
        assert!(!script.is_empty());
        // Fail-open markers
        assert!(script.contains("trap"));
        assert!(script.contains("exit 0"));
    }

    #[test]
    fn hook_script_path_relative() {
        let converter = CopilotConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        let parsed: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();

        // Verify bash field uses relative .github/hooks/scripts/ path
        let bash = parsed["hooks"]["sessionStart"][0]["bash"].as_str().unwrap();
        assert!(
            bash.starts_with(".github/hooks/scripts/"),
            "bash field should use relative path, got: {bash}"
        );
        assert!(
            !bash.contains("$HOME"),
            "bash field should not contain $HOME"
        );
    }
}
