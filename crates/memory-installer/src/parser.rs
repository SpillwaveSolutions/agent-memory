//! Plugin parser that reads installer-sources.json, walks plugin directories,
//! extracts YAML frontmatter + markdown bodies, and returns a complete PluginBundle.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use walkdir::WalkDir;

use crate::types::{
    HookDefinition, PluginAgent, PluginBundle, PluginCommand, PluginSkill, SkillFile,
};

// ---------------------------------------------------------------------------
// Internal serde structs for JSON manifests
// ---------------------------------------------------------------------------

/// Top-level structure of `installer-sources.json`.
#[derive(Debug, Deserialize)]
struct SourceManifest {
    #[allow(dead_code)]
    version: String,
    sources: Vec<SourceEntry>,
}

/// One entry in the `sources` array.
#[derive(Debug, Deserialize)]
struct SourceEntry {
    path: String,
    #[allow(dead_code)]
    description: String,
}

/// Top-level structure of a plugin's `marketplace.json`.
#[derive(Debug, Deserialize)]
struct MarketplaceManifest {
    plugins: Vec<MarketplacePlugin>,
}

/// One plugin entry inside `marketplace.json`.
#[derive(Debug, Deserialize)]
struct MarketplacePlugin {
    #[allow(dead_code)]
    name: String,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    agents: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a markdown file, extracting YAML frontmatter as `serde_json::Value` and
/// the body content as a `String`.
///
/// Uses `gray_matter` with the YAML engine. The generic `parse::<Value>` call
/// deserializes frontmatter directly into `serde_json::Value`, avoiding the
/// intermediate `Pod` type.
///
/// If the file has no frontmatter (no `---` delimiters), returns an empty
/// `Value::Object` and the full file content as the body.
pub fn parse_md_file(path: &Path) -> Result<(Value, String)> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let parsed = matter
        .parse::<Value>(&content)
        .map_err(|e| anyhow::anyhow!("gray_matter parse error in {}: {e}", path.display()))?;

    let frontmatter = parsed.data.unwrap_or_else(|| {
        tracing::warn!("no frontmatter in {} -- treating as empty", path.display());
        Value::Object(serde_json::Map::new())
    });

    Ok((frontmatter, parsed.content))
}

/// Parse all plugin sources from the given source root directory.
///
/// Reads `{source_root}/installer-sources.json` to discover plugin directories,
/// then walks each directory using its `marketplace.json` to find commands,
/// agents, and skills. Returns a complete [`PluginBundle`].
pub fn parse_sources(source_root: &Path) -> Result<PluginBundle> {
    let manifest_path = source_root.join("installer-sources.json");
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: SourceManifest = serde_json::from_str(&manifest_text)
        .with_context(|| format!("deserializing {}", manifest_path.display()))?;

    let mut commands = Vec::new();
    let mut agents = Vec::new();
    let mut skills = Vec::new();
    let hooks: Vec<HookDefinition> = Vec::new(); // Hooks deferred to Phase 49

    for source in &manifest.sources {
        let source_dir = source_root.join(&source.path);

        let marketplace_path = source_dir.join(".claude-plugin/marketplace.json");
        let marketplace_text = std::fs::read_to_string(&marketplace_path)
            .with_context(|| format!("reading {}", marketplace_path.display()))?;
        let marketplace: MarketplaceManifest = serde_json::from_str(&marketplace_text)
            .with_context(|| format!("deserializing {}", marketplace_path.display()))?;

        for plugin in &marketplace.plugins {
            // Parse commands
            for cmd_path in &plugin.commands {
                let full_path = source_dir.join(cmd_path);
                let (frontmatter, body) = parse_md_file(&full_path)?;
                let name = frontmatter
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| name_from_path(&full_path));
                commands.push(PluginCommand {
                    name,
                    frontmatter,
                    body,
                    source_path: full_path,
                });
            }

            // Parse agents
            for agent_path in &plugin.agents {
                let full_path = source_dir.join(agent_path);
                let (frontmatter, body) = parse_md_file(&full_path)?;
                let name = frontmatter
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| name_from_path(&full_path));
                agents.push(PluginAgent {
                    name,
                    frontmatter,
                    body,
                    source_path: full_path,
                });
            }

            // Parse skills (directories, not files)
            for skill_path in &plugin.skills {
                let skill_dir = source_dir.join(skill_path);
                let skill_md = skill_dir.join("SKILL.md");

                let (frontmatter, body) = if skill_md.exists() {
                    parse_md_file(&skill_md)?
                } else {
                    tracing::warn!(
                        "SKILL.md not found in {} -- using empty frontmatter",
                        skill_dir.display()
                    );
                    (Value::Object(serde_json::Map::new()), String::new())
                };

                let name = frontmatter
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| name_from_path(&skill_dir));

                // Collect additional files (everything except SKILL.md)
                let additional_files = collect_additional_files(&skill_dir)?;

                skills.push(PluginSkill {
                    name,
                    frontmatter,
                    body,
                    source_path: skill_dir,
                    additional_files,
                });
            }
        }
    }

    Ok(PluginBundle {
        commands,
        agents,
        skills,
        hooks,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a name from a file path by taking the stem (filename without extension).
fn name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Walk a skill directory and collect all files except `SKILL.md` as [`SkillFile`] entries.
/// Each entry stores a path relative to `skill_dir` and the file content.
fn collect_additional_files(skill_dir: &Path) -> Result<Vec<SkillFile>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(skill_dir).follow_links(false) {
        let entry = entry.with_context(|| format!("walking skill dir {}", skill_dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        // Skip SKILL.md itself
        if entry.file_name() == "SKILL.md" {
            continue;
        }
        let abs_path = entry.path();
        let relative_path = abs_path
            .strip_prefix(skill_dir)
            .with_context(|| {
                format!(
                    "stripping prefix {} from {}",
                    skill_dir.display(),
                    abs_path.display()
                )
            })?
            .to_path_buf();
        let content = std::fs::read_to_string(abs_path)
            .with_context(|| format!("reading skill file {}", abs_path.display()))?;
        files.push(SkillFile {
            relative_path,
            content,
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn plugins_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("plugins")
    }

    #[test]
    fn test_parse_md_file_with_frontmatter() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "---\nname: test-command\ndescription: A test\n---\n\n# Body\n\nSome content."
        )
        .unwrap();

        let (fm, body) = parse_md_file(file.path()).unwrap();
        assert_eq!(fm["name"], "test-command");
        assert_eq!(fm["description"], "A test");
        assert!(body.contains("# Body"));
        assert!(body.contains("Some content."));
    }

    #[test]
    fn test_parse_md_file_no_frontmatter() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# Just a heading\n\nNo frontmatter here.").unwrap();

        let (fm, body) = parse_md_file(file.path()).unwrap();
        assert!(fm.is_object());
        assert_eq!(fm.as_object().unwrap().len(), 0);
        assert!(body.contains("# Just a heading"));
    }

    #[test]
    fn test_parse_sources_command_count() {
        let plugins = plugins_dir();
        let bundle = parse_sources(&plugins).unwrap();
        assert_eq!(
            bundle.commands.len(),
            6,
            "Expected 6 commands, got {}: {:?}",
            bundle.commands.len(),
            bundle.commands.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_sources_agent_count() {
        let plugins = plugins_dir();
        let bundle = parse_sources(&plugins).unwrap();
        assert_eq!(
            bundle.agents.len(),
            2,
            "Expected 2 agents, got {}: {:?}",
            bundle.agents.len(),
            bundle.agents.iter().map(|a| &a.name).collect::<Vec<_>>()
        );

        let agent_names: Vec<&str> = bundle.agents.iter().map(|a| a.name.as_str()).collect();
        assert!(agent_names.contains(&"memory-navigator"));
        assert!(agent_names.contains(&"setup-troubleshooter"));
    }

    #[test]
    fn test_parse_sources_skill_count() {
        let plugins = plugins_dir();
        let bundle = parse_sources(&plugins).unwrap();
        assert_eq!(
            bundle.skills.len(),
            13,
            "Expected 13 skills, got {}: {:?}",
            bundle.skills.len(),
            bundle.skills.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_command_frontmatter_fields() {
        let plugins = plugins_dir();
        let bundle = parse_sources(&plugins).unwrap();

        let search_cmd = bundle
            .commands
            .iter()
            .find(|c| c.name == "memory-search")
            .expect("memory-search command should exist");

        assert!(
            search_cmd.frontmatter.get("name").is_some(),
            "frontmatter should have 'name' field"
        );
        assert!(
            search_cmd.frontmatter.get("description").is_some(),
            "frontmatter should have 'description' field"
        );
        assert!(
            search_cmd.frontmatter.get("parameters").is_some(),
            "frontmatter should have 'parameters' field"
        );
        assert!(
            search_cmd.frontmatter["parameters"].is_array(),
            "'parameters' should be an array"
        );
        assert!(
            search_cmd.frontmatter.get("skills").is_some(),
            "frontmatter should have 'skills' field"
        );
    }

    #[test]
    fn test_skill_additional_files() {
        let plugins = plugins_dir();
        let bundle = parse_sources(&plugins).unwrap();

        // memory-query skill has a references/ directory with command-reference.md
        let query_skill = bundle
            .skills
            .iter()
            .find(|s| s.name == "memory-query")
            .expect("memory-query skill should exist");

        assert!(
            !query_skill.additional_files.is_empty(),
            "memory-query skill should have additional files from references/"
        );

        let ref_paths: Vec<&str> = query_skill
            .additional_files
            .iter()
            .map(|f| f.relative_path.to_str().unwrap())
            .collect();
        assert!(
            ref_paths.iter().any(|p| p.contains("command-reference")),
            "should contain command-reference.md, got: {:?}",
            ref_paths
        );
    }
}
