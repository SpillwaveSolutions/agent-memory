use std::path::PathBuf;

use crate::converter::RuntimeConverter;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill,
};

use super::helpers::{reconstruct_md, rewrite_paths};

/// Path prefix in canonical source to replace.
const CLAUDE_PATH_FROM: &str = "~/.claude/";
/// Replacement path prefix for agent-memory storage.
const CLAUDE_PATH_TO: &str = "~/.config/agent-memory/";

pub struct ClaudeConverter;

impl RuntimeConverter for ClaudeConverter {
    fn name(&self) -> &str {
        "claude"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join(".claude/plugins/memory-plugin"),
            InstallScope::Global => {
                let home = directories::BaseDirs::new()
                    .map(|b| b.home_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~").as_ref()));
                home.join(".claude/plugins/memory-plugin")
            }
            InstallScope::Custom(dir) => dir.clone(),
        }
    }

    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let body = rewrite_paths(&cmd.body, CLAUDE_PATH_FROM, CLAUDE_PATH_TO);
        let content = reconstruct_md(&cmd.frontmatter, &body);
        vec![ConvertedFile {
            target_path: target_dir.join("commands").join(format!("{}.md", cmd.name)),
            content,
        }]
    }

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let body = rewrite_paths(&agent.body, CLAUDE_PATH_FROM, CLAUDE_PATH_TO);
        let content = reconstruct_md(&agent.frontmatter, &body);
        vec![ConvertedFile {
            target_path: target_dir.join("agents").join(format!("{}.md", agent.name)),
            content,
        }]
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&skill.name);

        let body = rewrite_paths(&skill.body, CLAUDE_PATH_FROM, CLAUDE_PATH_TO);
        let content = reconstruct_md(&skill.frontmatter, &body);

        let mut files = vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }];

        for additional in &skill.additional_files {
            let rewritten = rewrite_paths(&additional.content, CLAUDE_PATH_FROM, CLAUDE_PATH_TO);
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
        _cfg: &InstallConfig,
    ) -> Vec<ConvertedFile> {
        // No extra config needed for Claude runtime
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
    fn convert_command_produces_correct_path_and_content() {
        let converter = ClaudeConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "search".to_string(),
            frontmatter: serde_json::json!({"description": "Search memories"}),
            body: "Search for things in ~/.claude/data".to_string(),
            source_path: PathBuf::from("commands/search.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.claude/plugins/memory-plugin/commands/search.md")
        );
        assert!(files[0].content.contains("description: Search memories"));
        assert!(files[0].content.contains("~/.config/agent-memory/data"));
        assert!(!files[0].content.contains("~/.claude/data"));
    }

    #[test]
    fn convert_agent_produces_correct_path_and_content() {
        let converter = ClaudeConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "navigator".to_string(),
            frontmatter: serde_json::json!({"description": "Memory navigator"}),
            body: "Uses ~/.claude/skills for lookup".to_string(),
            source_path: PathBuf::from("agents/navigator.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.claude/plugins/memory-plugin/agents/navigator.md")
        );
        assert!(files[0].content.contains("~/.config/agent-memory/skills"));
        assert!(!files[0].content.contains("~/.claude/skills"));
    }

    #[test]
    fn convert_skill_produces_skill_md_and_additional_files() {
        let converter = ClaudeConverter;
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
            PathBuf::from("/project/.claude/plugins/memory-plugin/skills/memory-query/SKILL.md")
        );
        assert!(files[0].content.contains("~/.config/agent-memory/data"));

        // Additional file
        assert_eq!(
            files[1].target_path,
            PathBuf::from(
                "/project/.claude/plugins/memory-plugin/skills/memory-query/rules/search.md"
            )
        );
        assert!(files[1].content.contains("~/.config/agent-memory/db"));
        assert!(!files[1].content.contains("~/.claude/db"));
    }

    #[test]
    fn convert_hook_returns_none() {
        let converter = ClaudeConverter;
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
        let converter = ClaudeConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };
        assert!(converter.generate_guidance(&bundle, &cfg).is_empty());
    }

    #[test]
    fn path_rewriting_replaces_claude_with_agent_memory() {
        let converter = ClaudeConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "test".to_string(),
            frontmatter: serde_json::Value::Null,
            body: "Path ~/.claude/foo and ~/.claude/bar here".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("~/.config/agent-memory/foo"));
        assert!(files[0].content.contains("~/.config/agent-memory/bar"));
        assert!(!files[0].content.contains("~/.claude/"));
    }

    #[test]
    fn convert_command_with_no_frontmatter_returns_body_only() {
        let converter = ClaudeConverter;
        let cfg = test_config();
        let cmd = PluginCommand {
            name: "simple".to_string(),
            frontmatter: serde_json::Value::Null,
            body: "Just body content".to_string(),
            source_path: PathBuf::from("test.md"),
        };

        let files = converter.convert_command(&cmd, &cfg);
        assert_eq!(files[0].content, "Just body content");
    }
}
