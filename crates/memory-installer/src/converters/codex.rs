use std::path::PathBuf;

use crate::converter::RuntimeConverter;
use crate::tool_maps::map_tool;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill, Runtime,
};

use super::helpers::{reconstruct_md, rewrite_paths};

/// Path prefix in canonical source to replace.
const CODEX_PATH_FROM: &str = "~/.claude/";
/// Replacement path prefix for agent-memory storage.
const CODEX_PATH_TO: &str = "~/.config/agent-memory/";

/// Determine sandbox permission level for a given agent.
///
/// `setup-troubleshooter` needs write access to modify config files;
/// all other agents default to read-only.
fn sandbox_for_agent(name: &str) -> &'static str {
    match name {
        "setup-troubleshooter" => "workspace-write",
        _ => "read-only",
    }
}

pub struct CodexConverter;

impl RuntimeConverter for CodexConverter {
    fn name(&self) -> &str {
        "codex"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join(".codex"),
            InstallScope::Global => {
                let config_dir = directories::BaseDirs::new()
                    .map(|b| b.config_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.config").as_ref()));
                config_dir.join("codex")
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

        let body = rewrite_paths(&cmd.body, CODEX_PATH_FROM, CODEX_PATH_TO);
        let content = reconstruct_md(&serde_json::Value::Object(fm), &body);

        vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }]
    }

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&agent.name);

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
        if let Some(allowed) = agent.frontmatter.get("allowed-tools").and_then(|v| v.as_array()) {
            for tool_val in allowed {
                if let Some(tool_name) = tool_val.as_str() {
                    // Skip MCP tools
                    if tool_name.starts_with("mcp__") {
                        continue;
                    }
                    if let Some(mapped) = map_tool(Runtime::Codex, tool_name) {
                        tools.push(mapped.to_string());
                    }
                }
            }
        }
        // Deduplicate (Codex maps Write and Edit both to "edit", etc.)
        tools.sort();
        tools.dedup();

        let body = rewrite_paths(&agent.body, CODEX_PATH_FROM, CODEX_PATH_TO);
        let sandbox = sandbox_for_agent(&agent.name);

        let mut full_body = body;
        if !tools.is_empty() {
            full_body.push_str("\n\n## Tools\n\n");
            for tool in &tools {
                full_body.push_str(&format!("- {tool}\n"));
            }
        }
        full_body.push_str(&format!("\n## Sandbox\n\n**Recommended sandbox:** `{sandbox}`\n"));

        let content = reconstruct_md(&serde_json::Value::Object(fm), &full_body);

        vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }]
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        let target_dir = self.target_dir(&cfg.scope);
        let skill_dir = target_dir.join("skills").join(&skill.name);

        let body = rewrite_paths(&skill.body, CODEX_PATH_FROM, CODEX_PATH_TO);
        let content = reconstruct_md(&skill.frontmatter, &body);

        let mut files = vec![ConvertedFile {
            target_path: skill_dir.join("SKILL.md"),
            content,
        }];

        for additional in &skill.additional_files {
            let rewritten = rewrite_paths(&additional.content, CODEX_PATH_FROM, CODEX_PATH_TO);
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
        // Hooks deferred to Phase 49
        None
    }

    fn generate_guidance(
        &self,
        bundle: &PluginBundle,
        cfg: &InstallConfig,
    ) -> Vec<ConvertedFile> {
        let target = self.target_dir(&cfg.scope);
        let mut md = String::new();

        md.push_str("# Agent Memory\n\n");
        md.push_str("Memory plugin for cross-session conversation recall.\n\n");
        md.push_str("## Available Skills\n\n");

        for cmd in &bundle.commands {
            let desc = cmd
                .frontmatter
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description");
            md.push_str(&format!("- **{}**: {}\n", cmd.name, desc));
        }

        md.push_str("\n## Agents\n\n");
        for agent in &bundle.agents {
            let desc = agent
                .frontmatter
                .get("description")
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
        let converter = CodexConverter;
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
            PathBuf::from("/project/.codex/skills/memory-search/SKILL.md")
        );
        // Verify YAML frontmatter contains name and description
        assert!(files[0].content.contains("name: memory-search"));
        assert!(files[0].content.contains("description: Search past conversations"));
        // Verify path rewriting
        assert!(files[0].content.contains("~/.config/agent-memory/data"));
        assert!(!files[0].content.contains("~/.claude/data"));
    }

    #[test]
    fn agent_to_skill() {
        let converter = CodexConverter;
        let cfg = test_config();
        let agent = PluginAgent {
            name: "memory-navigator".to_string(),
            frontmatter: serde_json::json!({
                "description": "Navigate memory",
                "allowed-tools": ["Read", "Bash", "mcp__memory"]
            }),
            body: "Navigate through ~/.claude/skills for lookup".to_string(),
            source_path: PathBuf::from("agents/memory-navigator.md"),
        };

        let files = converter.convert_agent(&agent, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.codex/skills/memory-navigator/SKILL.md")
        );
        // Verify orchestration content
        assert!(files[0].content.contains("name: memory-navigator"));
        assert!(files[0].content.contains("description: Navigate memory"));
        // Verify tools section -- mcp__ excluded, Read->read, Bash->execute
        assert!(files[0].content.contains("## Tools"));
        assert!(files[0].content.contains("- execute"));
        assert!(files[0].content.contains("- read"));
        assert!(!files[0].content.contains("mcp__"));
        // Verify sandbox section
        assert!(files[0].content.contains("## Sandbox"));
        assert!(files[0].content.contains("**Recommended sandbox:** `read-only`"));
        // Verify path rewriting
        assert!(files[0].content.contains("~/.config/agent-memory/skills"));
    }

    #[test]
    fn agents_md_generation() {
        let converter = CodexConverter;
        let cfg = test_config();
        let bundle = PluginBundle {
            commands: vec![PluginCommand {
                name: "memory-search".to_string(),
                frontmatter: serde_json::json!({"description": "Search conversations"}),
                body: String::new(),
                source_path: PathBuf::from("cmd.md"),
            }],
            agents: vec![
                PluginAgent {
                    name: "memory-navigator".to_string(),
                    frontmatter: serde_json::json!({"description": "Navigate memory"}),
                    body: String::new(),
                    source_path: PathBuf::from("agent1.md"),
                },
                PluginAgent {
                    name: "setup-troubleshooter".to_string(),
                    frontmatter: serde_json::json!({"description": "Troubleshoot setup"}),
                    body: String::new(),
                    source_path: PathBuf::from("agent2.md"),
                },
            ],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].target_path,
            PathBuf::from("/project/.codex/AGENTS.md")
        );

        let content = &files[0].content;
        // Header
        assert!(content.contains("# Agent Memory"));
        // Skills list
        assert!(content.contains("## Available Skills"));
        assert!(content.contains("- **memory-search**: Search conversations"));
        // Agents section
        assert!(content.contains("## Agents"));
        assert!(content.contains("### memory-navigator"));
        assert!(content.contains("Navigate memory"));
        assert!(content.contains("**Recommended sandbox:** `read-only`"));
        assert!(content.contains("### setup-troubleshooter"));
        assert!(content.contains("Troubleshoot setup"));
        assert!(content.contains("**Recommended sandbox:** `workspace-write`"));
    }

    #[test]
    fn sandbox_mapping() {
        assert_eq!(sandbox_for_agent("setup-troubleshooter"), "workspace-write");
        assert_eq!(sandbox_for_agent("memory-navigator"), "read-only");
        assert_eq!(sandbox_for_agent("anything-else"), "read-only");
        assert_eq!(sandbox_for_agent(""), "read-only");
    }

    #[test]
    fn convert_skill_produces_skill_md_and_additional_files() {
        let converter = CodexConverter;
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
            PathBuf::from("/project/.codex/skills/memory-query/SKILL.md")
        );
        assert!(files[0].content.contains("~/.config/agent-memory/data"));

        // Additional file
        assert_eq!(
            files[1].target_path,
            PathBuf::from("/project/.codex/skills/memory-query/rules/search.md")
        );
        assert!(files[1].content.contains("~/.config/agent-memory/db"));
        assert!(!files[1].content.contains("~/.claude/db"));
    }

    #[test]
    fn convert_hook_returns_none() {
        let converter = CodexConverter;
        let cfg = test_config();
        let hook = HookDefinition {
            name: "test-hook".to_string(),
            frontmatter: serde_json::Value::Null,
            body: String::new(),
            source_path: PathBuf::from("hooks/test.md"),
        };
        assert!(converter.convert_hook(&hook, &cfg).is_none());
    }
}
