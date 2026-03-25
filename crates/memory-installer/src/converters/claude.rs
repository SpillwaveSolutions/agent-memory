use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::json;

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

/// Marketplace identifier for agent-memory in Claude Code registry.
const MARKETPLACE_ID: &str = "agent-memory";
/// Plugin name as registered in Claude Code.
const PLUGIN_NAME: &str = "memory-query";
/// Combined registry key: {plugin}@{marketplace}.
const PLUGIN_REGISTRY_KEY: &str = "memory-query@agent-memory";
/// Git URL for the agent-memory marketplace source.
const MARKETPLACE_GIT_URL: &str = "https://github.com/SpillwaveSolutions/agent-memory.git";

/// Resolve the user home directory.
fn home_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~").as_ref()))
}

/// Read a JSON file, returning an empty object on any error (missing, corrupt, etc.).
fn read_json_or_empty(path: &Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| json!({}))
}

/// Read the plugin version from `{source_root}/plugins/memory-query-plugin/.claude-plugin/plugin.json`.
///
/// Falls back to `"1.0.0"` if the file is missing or malformed.
fn read_plugin_version(source_root: &Path) -> String {
    let plugin_json = source_root.join("plugins/memory-query-plugin/.claude-plugin/plugin.json");
    std::fs::read_to_string(&plugin_json)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "1.0.0".to_string())
}

/// Remove old version directories under `cache_plugin_dir` that don't match `current_version`.
///
/// No-op if the directory doesn't exist. Errors are silently ignored.
fn cleanup_old_versions(cache_plugin_dir: &Path, current_version: &str) {
    let entries = match std::fs::read_dir(cache_plugin_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if name != current_version {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
    }
}

/// Build the `known_marketplaces.json` registry file.
///
/// Upserts the `agent-memory` marketplace entry with git source and install location.
fn build_known_marketplaces(home: &Path, now: &str) -> ConvertedFile {
    let json_path = home.join(".claude/plugins/known_marketplaces.json");
    let mut root = read_json_or_empty(&json_path);

    let obj = root.as_object_mut().unwrap();
    obj.insert(
        MARKETPLACE_ID.to_string(),
        json!({
            "source": {
                "source": "git",
                "url": MARKETPLACE_GIT_URL
            },
            "installLocation": home.join(".claude/plugins/marketplaces").join(MARKETPLACE_ID).to_string_lossy().to_string(),
            "lastUpdated": now
        }),
    );

    let content = serde_json::to_string_pretty(&root).unwrap() + "\n";
    ConvertedFile {
        target_path: json_path,
        content,
    }
}

/// Build the `installed_plugins.json` registry file.
///
/// Ensures `version: 2` at top level, upserts the plugin entry preserving `installedAt`
/// from any existing entry.
fn build_installed_plugins(home: &Path, now: &str, version: &str) -> ConvertedFile {
    let json_path = home.join(".claude/plugins/installed_plugins.json");
    let mut root = read_json_or_empty(&json_path);

    let obj = root.as_object_mut().unwrap();
    obj.insert("version".to_string(), json!(2));

    // Preserve installedAt from existing entry if present.
    let installed_at = obj
        .get("plugins")
        .and_then(|p| p.get(PLUGIN_REGISTRY_KEY))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.first())
        .and_then(|entry| entry.get("installedAt"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| now.to_string());

    let install_path = home
        .join(".claude/plugins/cache")
        .join(MARKETPLACE_ID)
        .join(PLUGIN_NAME)
        .join(version);

    let plugins = obj.entry("plugins").or_insert_with(|| json!({}));
    plugins.as_object_mut().unwrap().insert(
        PLUGIN_REGISTRY_KEY.to_string(),
        json!([{
            "scope": "user",
            "installPath": install_path.to_string_lossy().to_string(),
            "version": version,
            "installedAt": installed_at,
            "lastUpdated": now
        }]),
    );

    let content = serde_json::to_string_pretty(&root).unwrap() + "\n";
    ConvertedFile {
        target_path: json_path,
        content,
    }
}

/// Build the `settings.json` registry file.
///
/// Sets `enabledPlugins.{PLUGIN_REGISTRY_KEY} = true`, preserving all other keys.
fn build_settings(home: &Path) -> ConvertedFile {
    let json_path = home.join(".claude/settings.json");
    let mut root = read_json_or_empty(&json_path);

    let obj = root.as_object_mut().unwrap();
    let enabled = obj.entry("enabledPlugins").or_insert_with(|| json!({}));
    enabled
        .as_object_mut()
        .unwrap()
        .insert(PLUGIN_REGISTRY_KEY.to_string(), json!(true));

    let content = serde_json::to_string_pretty(&root).unwrap() + "\n";
    ConvertedFile {
        target_path: json_path,
        content,
    }
}

pub struct ClaudeConverter;

impl RuntimeConverter for ClaudeConverter {
    fn name(&self) -> &str {
        "claude"
    }

    fn target_dir(&self, scope: &InstallScope) -> PathBuf {
        match scope {
            InstallScope::Project(root) => root.join(".claude/plugins/memory-plugin"),
            InstallScope::Global => {
                let home = home_dir();
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

    fn generate_guidance(&self, _bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        // Only register for Global scope
        if !matches!(cfg.scope, InstallScope::Global) {
            return Vec::new();
        }

        let home = home_dir();
        let now = Utc::now().to_rfc3339();
        let version = read_plugin_version(&cfg.source_root);

        // Cleanup old version directories before writing new
        let cache_plugin_dir = home
            .join(".claude/plugins/cache")
            .join(MARKETPLACE_ID)
            .join(PLUGIN_NAME);
        cleanup_old_versions(&cache_plugin_dir, &version);

        vec![
            build_known_marketplaces(&home, &now),
            build_installed_plugins(&home, &now, &version),
            build_settings(&home),
        ]
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

    /// Create a tempdir with a plugin.json at the expected relative path.
    fn setup_source_root(tmp: &Path, version: &str) {
        let plugin_dir = tmp.join("plugins/memory-query-plugin/.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&json!({
                "name": "memory-query",
                "version": version,
                "description": "test plugin"
            }))
            .unwrap(),
        )
        .unwrap();
    }

    /// Create the .claude directory structure in a fake home dir.
    fn setup_home(home: &Path) {
        std::fs::create_dir_all(home.join(".claude/plugins")).unwrap();
    }

    // ----------------------------------------------------------------
    // Existing tests (preserved from before Phase 58)
    // ----------------------------------------------------------------

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

    // ----------------------------------------------------------------
    // Phase 58 registration tests
    // ----------------------------------------------------------------

    // CREG-01: known_marketplaces.json structure
    #[test]
    fn test_creg01_known_marketplaces_structure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        let now = "2026-03-25T12:00:00+00:00";
        let file = build_known_marketplaces(home, now);

        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();
        let entry = &json[MARKETPLACE_ID];
        assert_eq!(entry["source"]["source"].as_str(), Some("git"));
        assert_eq!(entry["source"]["url"].as_str(), Some(MARKETPLACE_GIT_URL));
        assert_eq!(entry["lastUpdated"].as_str(), Some(now));

        // installLocation should be an absolute path
        let install_loc = entry["installLocation"].as_str().unwrap();
        assert!(
            install_loc.contains("marketplaces/agent-memory"),
            "installLocation should contain marketplaces/agent-memory, got: {install_loc}"
        );
    }

    // CREG-02: installed_plugins.json structure
    #[test]
    fn test_creg02_installed_plugins_structure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        let now = "2026-03-25T12:00:00+00:00";
        let file = build_installed_plugins(home, now, "1.0.0");

        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();
        assert_eq!(json["version"].as_i64(), Some(2));

        let plugins = &json["plugins"][PLUGIN_REGISTRY_KEY];
        assert!(plugins.is_array());
        let entry = &plugins[0];
        assert_eq!(entry["scope"].as_str(), Some("user"));
        assert_eq!(entry["version"].as_str(), Some("1.0.0"));
        assert_eq!(entry["installedAt"].as_str(), Some(now));
        assert_eq!(entry["lastUpdated"].as_str(), Some(now));

        let install_path = entry["installPath"].as_str().unwrap();
        assert!(
            install_path.contains("cache/agent-memory/memory-query/1.0.0"),
            "installPath should contain versioned cache path, got: {install_path}"
        );
    }

    // CREG-03: settings.json enabledPlugins
    #[test]
    fn test_creg03_settings_enabled_plugins() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        let file = build_settings(home);

        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();
        assert_eq!(
            json["enabledPlugins"][PLUGIN_REGISTRY_KEY].as_bool(),
            Some(true)
        );
    }

    // CREG-04: Plugin key is exactly "memory-query@agent-memory"
    #[test]
    fn test_creg04_plugin_key_format() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        let now = "2026-03-25T12:00:00+00:00";

        let installed = build_installed_plugins(home, now, "1.0.0");
        let installed_json: serde_json::Value = serde_json::from_str(&installed.content).unwrap();
        assert!(
            installed_json["plugins"]
                .get("memory-query@agent-memory")
                .is_some(),
            "installed_plugins should use exact key memory-query@agent-memory"
        );

        let settings = build_settings(home);
        let settings_json: serde_json::Value = serde_json::from_str(&settings.content).unwrap();
        assert!(
            settings_json["enabledPlugins"]
                .get("memory-query@agent-memory")
                .is_some(),
            "settings should use exact key memory-query@agent-memory"
        );
    }

    // CREG-05: Version read from plugin.json
    #[test]
    fn test_creg05_version_from_plugin_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        setup_source_root(tmp.path(), "2.3.4");

        let version = read_plugin_version(tmp.path());
        assert_eq!(version, "2.3.4");

        let home_tmp = tempfile::TempDir::new().unwrap();
        let home = home_tmp.path();
        setup_home(home);

        let now = "2026-03-25T12:00:00+00:00";
        let file = build_installed_plugins(home, now, &version);
        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();

        let install_path = json["plugins"][PLUGIN_REGISTRY_KEY][0]["installPath"]
            .as_str()
            .unwrap();
        assert!(
            install_path.contains("2.3.4"),
            "install path should contain version 2.3.4, got: {install_path}"
        );
    }

    // CREG-06: Idempotent reinstall preserves installedAt
    #[test]
    fn test_creg06_idempotent_reinstall_preserves_installed_at() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        let original_installed_at = "2026-01-01T00:00:00+00:00";
        let initial_content = json!({
            "version": 2,
            "plugins": {
                PLUGIN_REGISTRY_KEY: [{
                    "scope": "user",
                    "installPath": "/old/path",
                    "version": "0.9.0",
                    "installedAt": original_installed_at,
                    "lastUpdated": "2026-01-01T00:00:00+00:00"
                }]
            }
        });

        let json_path = home.join(".claude/plugins/installed_plugins.json");
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&initial_content).unwrap(),
        )
        .unwrap();

        let now = "2026-03-25T12:00:00+00:00";
        let file = build_installed_plugins(home, now, "1.0.0");
        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();

        let entry = &json["plugins"][PLUGIN_REGISTRY_KEY][0];
        assert_eq!(
            entry["installedAt"].as_str(),
            Some(original_installed_at),
            "installedAt should be preserved from original entry"
        );
        assert_eq!(
            entry["lastUpdated"].as_str(),
            Some(now),
            "lastUpdated should be updated"
        );
    }

    // CREG-06: Cleanup old version directories
    #[test]
    fn test_creg06_cleanup_old_versions() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache/agent-memory/memory-query");
        std::fs::create_dir_all(cache_dir.join("1.0.0")).unwrap();
        std::fs::create_dir_all(cache_dir.join("1.1.0")).unwrap();

        cleanup_old_versions(&cache_dir, "2.0.0");

        assert!(!cache_dir.join("1.0.0").exists(), "1.0.0 should be removed");
        assert!(!cache_dir.join("1.1.0").exists(), "1.1.0 should be removed");
    }

    // META-03: Version from plugin.json drives install path
    #[test]
    fn test_meta03_version_drives_install_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        setup_source_root(tmp.path(), "3.5.7");

        let version = read_plugin_version(tmp.path());
        assert_eq!(version, "3.5.7");

        let home_tmp = tempfile::TempDir::new().unwrap();
        let home = home_tmp.path();
        setup_home(home);

        let now = "2026-03-25T12:00:00+00:00";
        let file = build_installed_plugins(home, now, &version);
        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();

        let install_path = json["plugins"][PLUGIN_REGISTRY_KEY][0]["installPath"]
            .as_str()
            .unwrap();
        assert!(
            install_path.ends_with("3.5.7"),
            "install path should end with version, got: {install_path}"
        );
    }

    // Project scope returns empty
    #[test]
    fn test_project_scope_returns_empty() {
        let converter = ClaudeConverter;
        let cfg = InstallConfig {
            scope: InstallScope::Project(PathBuf::from("/project")),
            dry_run: false,
            source_root: PathBuf::from("/src"),
        };
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };
        assert!(
            converter.generate_guidance(&bundle, &cfg).is_empty(),
            "Project scope should return empty"
        );
    }

    // Custom scope returns empty
    #[test]
    fn test_custom_scope_returns_empty() {
        let converter = ClaudeConverter;
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
        assert!(
            converter.generate_guidance(&bundle, &cfg).is_empty(),
            "Custom scope should return empty"
        );
    }

    // Settings preserves existing keys
    #[test]
    fn test_settings_preserves_existing_keys() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        let existing = json!({
            "theme": "dark",
            "model": "claude-opus-4-20250514"
        });
        std::fs::write(
            home.join(".claude/settings.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        let file = build_settings(home);
        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();

        assert_eq!(json["theme"].as_str(), Some("dark"), "theme preserved");
        assert_eq!(
            json["model"].as_str(),
            Some("claude-opus-4-20250514"),
            "model preserved"
        );
        assert_eq!(
            json["enabledPlugins"][PLUGIN_REGISTRY_KEY].as_bool(),
            Some(true),
            "plugin enabled"
        );
    }

    // Corrupt JSON falls back to empty object
    #[test]
    fn test_corrupt_json_creates_fresh() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path();
        setup_home(home);

        // Write corrupt JSON
        std::fs::write(home.join(".claude/settings.json"), "{{not valid json}}").unwrap();

        let file = build_settings(home);
        let json: serde_json::Value = serde_json::from_str(&file.content).unwrap();

        // Should still have enabledPlugins set properly
        assert_eq!(
            json["enabledPlugins"][PLUGIN_REGISTRY_KEY].as_bool(),
            Some(true),
            "should gracefully create fresh settings with plugin enabled"
        );
    }

    // read_plugin_version fallback
    #[test]
    fn test_read_plugin_version_missing_falls_back() {
        let tmp = tempfile::TempDir::new().unwrap();
        // No plugin.json created
        let version = read_plugin_version(tmp.path());
        assert_eq!(version, "1.0.0", "should fall back to 1.0.0");
    }

    // Global scope generates 3 files
    #[test]
    fn test_global_scope_generates_three_files() {
        let converter = ClaudeConverter;
        let tmp = tempfile::TempDir::new().unwrap();
        setup_source_root(tmp.path(), "1.0.0");

        let cfg = InstallConfig {
            scope: InstallScope::Global,
            dry_run: false,
            source_root: tmp.path().to_path_buf(),
        };
        let bundle = PluginBundle {
            commands: vec![],
            agents: vec![],
            skills: vec![],
            hooks: vec![],
        };

        let files = converter.generate_guidance(&bundle, &cfg);
        assert_eq!(
            files.len(),
            3,
            "Global scope should produce 3 registry files"
        );

        // Verify filenames
        let paths: Vec<String> = files
            .iter()
            .map(|f| {
                f.target_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(paths.contains(&"known_marketplaces.json".to_string()));
        assert!(paths.contains(&"installed_plugins.json".to_string()));
        assert!(paths.contains(&"settings.json".to_string()));
    }
}
