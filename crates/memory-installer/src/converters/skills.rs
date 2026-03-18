use std::path::PathBuf;

use crate::converter::RuntimeConverter;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill,
};

use super::helpers::{reconstruct_md, rewrite_paths};

/// Path prefix in canonical source to replace.
const SKILLS_PATH_FROM: &str = "~/.claude/";
/// Replacement path prefix for agent-memory storage.
const SKILLS_PATH_TO: &str = "~/.config/agent-memory/";

pub struct SkillsConverter;

impl RuntimeConverter for SkillsConverter {
    fn name(&self) -> &str {
        "skills"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join("skills"),
            InstallScope::Global => {
                let config_dir = directories::BaseDirs::new()
                    .map(|b| b.config_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.config").as_ref()));
                config_dir.join("agent-memory/skills")
            }
            InstallScope::Custom(dir) => dir.clone(),
        }
    }

    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join(&cmd.name);

        let mut fm = serde_json::Map::new();
        fm.insert(
            "name".to_string(),
            serde_json::Value::String(cmd.name.clone()),
        );
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

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join(&agent.name);

        let mut fm = serde_json::Map::new();
        fm.insert(
            "name".to_string(),
            serde_json::Value::String(agent.name.clone()),
        );
        if let Some(desc) = agent.frontmatter.get("description") {
            fm.insert("description".to_string(), desc.clone());
        }

        // Collect tools using canonical Claude names (no remapping for generic skills)
        let mut tools: Vec<String> = Vec::new();
        if let Some(allowed) = agent.frontmatter.get("allowed-tools").and_then(|v| v.as_array()) {
            for tool_val in allowed {
                if let Some(tool_name) = tool_val.as_str() {
                    // Skip MCP tools
                    if tool_name.starts_with("mcp__") {
                        continue;
                    }
                    tools.push(tool_name.to_string());
                }
            }
        }

        let body = rewrite_paths(&agent.body, SKILLS_PATH_FROM, SKILLS_PATH_TO);

        let mut full_body = body;
        if !tools.is_empty() {
            full_body.push_str("\n\n## Tools\n\n");
            for tool in &tools {
                full_body.push_str(&format!("- {tool}\n"));
            }
        }

        let content = reconstruct_md(&serde_json::Value::Object(fm), &full_body);

        vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }]
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join(&skill.name);

        let body = rewrite_paths(&skill.body, SKILLS_PATH_FROM, SKILLS_PATH_TO);
        let content = reconstruct_md(&skill.frontmatter, &body);

        let mut files = vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }];

        for additional in &skill.additional_files {
            let rewritten = rewrite_paths(&additional.content, SKILLS_PATH_FROM, SKILLS_PATH_TO);
            files.push(ConvertedFile {
                target_path: skill_dir.join(&additional.relative_path),
                content: rewritten,
            });
        }

        files
    }

    fn convert_hook(
        &self,
        _hook: &HookDefinition,
        _cfg: &InstallConfig,
    ) -> Option<ConvertedFile> {
        // Generic skills have no hook system
        None
    }

    fn generate_guidance(
        &self,
        _bundle: &PluginBundle,
        _cfg: &InstallConfig,
    ) -> Vec<ConvertedFile> {
        // No runtime-specific config needed for generic skills
        Vec::new()
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

    #[test]
    fn command_to_skill() {
        let converter = SkillsConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "memory-search".to_string(),
            frontmatter: serde_json::json!({"description": "Search past conversations"}),
            body: "Search for things in ~/.claude/data".to_string(),
            source_path: PathBuf::from("commands/memory-search.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/skills/memory-search/SKILL.md")
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
    fn agent_to_orchestration_skill() {
        let converter = SkillsConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "memory-navigator".to_string(),
            frontmatter: serde_json::json!({
                "description": "Navigate memory",
                "allowed-tools": ["Read", "Bash", "Grep", "mcp__memory"]
            }),
            body: "Navigate through ~/.claude/skills for lookup".to_string(),
            source_path: PathBuf::from("agents/memory-navigator.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/skills/memory-navigator/SKILL.md")
        );
        // Verify orchestration content
        assert!(files[0].content.contains("name: memory-navigator"));
        assert!(files[0].content.contains("description: Navigate memory"));
        // Verify tools section -- mcp__ excluded, canonical Claude names (no remapping)
        assert!(files[0].content.contains("## Tools"));
        assert!(files[0].content.contains("- Read"));
        assert!(files[0].content.contains("- Bash"));
        assert!(files[0].content.contains("- Grep"));
        assert!(!files[0].content.contains("mcp__"));
        // Verify path rewriting
        assert!(files[0].content.contains("~/.config/agent-memory/skills"));
    }

    #[test]
    fn skill_with_additional_files() {
        let converter = SkillsConverter;
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

        // SKILL.md
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/skills/memory-query/SKILL.md")
        );
        assert!(files[0].content.contains("~/.config/agent-memory/data"));

        // Additional file
        assert_eq!(
            files[1].target_path,
            PathBuf::from("/project/skills/memory-query/rules/search.md")
        );
        assert!(files[1].content.contains("~/.config/agent-memory/db"));
        assert!(!files[1].content.contains("~/.claude/db"));
    }

    #[test]
    fn custom_dir_targeting() {
        let converter = SkillsConverter;
        let scope = InstallScope::Custom(PathBuf::from("/my/path"));
        let dir = converter.target_dir(&scope);
        assert_eq!(dir, PathBuf::from("/my/path"));
    }

    #[test]
    fn tool_names_passthrough() {
        let converter = SkillsConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "test-agent".to_string(),
            frontmatter: serde_json::json!({
                "description": "Test agent",
                "allowed-tools": ["Read", "Bash", "Grep", "Edit", "Write"]
            }),
            body: "Test body".to_string(),
            source_path: PathBuf::from("agents/test.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        // Tools should use canonical Claude names (no remapping)
        assert!(files[0].content.contains("- Read"));
        assert!(files[0].content.contains("- Bash"));
        assert!(files[0].content.contains("- Grep"));
        assert!(files[0].content.contains("- Edit"));
        assert!(files[0].content.contains("- Write"));
    }

    #[test]
    fn convert_hook_returns_none() {
        let converter = SkillsConverter;
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
    fn generate_guidance_returns_empty() {
        let converter = SkillsConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };
        assert!(converter.generate_guidance(&bundle, &cfg).is_empty());
    }
}
