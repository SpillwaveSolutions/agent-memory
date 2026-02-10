//! CLOD (Cross-Language Operation Definition) parser and converter.
//!
//! CLOD is a TOML-based format for defining agent-memory commands in a
//! platform-neutral way. A single CLOD file can be converted to adapter-specific
//! files for Claude Code, OpenCode, Gemini CLI, and Copilot CLI.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// A complete CLOD definition parsed from a TOML file.
#[derive(Debug, Deserialize)]
pub struct ClodDefinition {
    pub command: ClodCommand,
    pub process: Option<ClodProcess>,
    pub output: Option<ClodOutput>,
    pub adapters: Option<ClodAdapters>,
}

/// The `[command]` section: identity and parameters.
#[derive(Debug, Deserialize)]
pub struct ClodCommand {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub parameters: Vec<ClodParameter>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// A single command parameter.
#[derive(Debug, Deserialize)]
pub struct ClodParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub position: Option<u32>,
    pub flag: Option<String>,
}

/// The `[process]` section: execution steps.
#[derive(Debug, Deserialize)]
pub struct ClodProcess {
    pub steps: Vec<String>,
}

/// The `[output]` section: formatting template.
#[derive(Debug, Deserialize)]
pub struct ClodOutput {
    pub format: String,
}

/// Per-adapter configuration in `[adapters]`.
#[derive(Debug, Deserialize, Default)]
pub struct ClodAdapters {
    pub claude: Option<AdapterConfig>,
    pub opencode: Option<AdapterConfig>,
    pub gemini: Option<AdapterConfig>,
    pub copilot: Option<AdapterConfig>,
}

/// Configuration for a single adapter target.
#[derive(Debug, Deserialize)]
pub struct AdapterConfig {
    pub directory: Option<String>,
    pub extension: Option<String>,
}

/// Parse a CLOD definition from a TOML file.
pub fn parse_clod(path: &Path) -> Result<ClodDefinition> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read CLOD file: {}", path.display()))?;
    let def: ClodDefinition =
        toml::from_str(&content).with_context(|| format!("Failed to parse CLOD file: {}", path.display()))?;

    // Validate required fields
    if def.command.name.is_empty() {
        anyhow::bail!("CLOD validation error: command.name is empty");
    }
    if def.command.description.is_empty() {
        anyhow::bail!("CLOD validation error: command.description is empty");
    }

    // Check for duplicate parameter names
    let mut seen = std::collections::HashSet::new();
    for param in &def.command.parameters {
        if !seen.insert(&param.name) {
            anyhow::bail!(
                "CLOD validation error: duplicate parameter name '{}'",
                param.name
            );
        }
    }

    Ok(def)
}

/// Generate a Claude Code command file (Markdown with YAML frontmatter).
pub fn generate_claude(def: &ClodDefinition, out_dir: &Path) -> Result<String> {
    let dir = adapter_dir(out_dir, &def.adapters, |a| &a.claude, "commands");
    fs::create_dir_all(&dir)?;

    let ext = adapter_ext(&def.adapters, |a| &a.claude, "md");
    let filename = format!("{}.{}", def.command.name, ext);
    let filepath = dir.join(&filename);

    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("name: {}\n", def.command.name));
    content.push_str(&format!(
        "description: {}\n",
        yaml_escape(&def.command.description)
    ));

    if !def.command.parameters.is_empty() {
        content.push_str("parameters:\n");
        for param in &def.command.parameters {
            content.push_str(&format!("  - name: {}\n", param.name));
            content.push_str(&format!(
                "    description: {}\n",
                yaml_escape(&param.description)
            ));
            content.push_str(&format!("    required: {}\n", param.required));
        }
    }

    content.push_str("---\n\n");
    content.push_str(&format!("{}\n", def.command.description));

    if let Some(process) = &def.process {
        content.push_str("\n## Process\n\n");
        for (i, step) in process.steps.iter().enumerate() {
            content.push_str(&format!("{}. {}\n", i + 1, step));
        }
    }

    fs::write(&filepath, &content)?;
    Ok(filepath.display().to_string())
}

/// Generate an OpenCode command file (Markdown with $ARGUMENTS).
pub fn generate_opencode(def: &ClodDefinition, out_dir: &Path) -> Result<String> {
    let dir = adapter_dir(out_dir, &def.adapters, |a| &a.opencode, "command");
    fs::create_dir_all(&dir)?;

    let ext = adapter_ext(&def.adapters, |a| &a.opencode, "md");
    let filename = format!("{}.{}", def.command.name, ext);
    let filepath = dir.join(&filename);

    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("name: {}\n", def.command.name));
    content.push_str(&format!(
        "description: {}\n",
        yaml_escape(&def.command.description)
    ));
    content.push_str("---\n\n");
    content.push_str(&format!("{}\n", def.command.description));
    content.push_str("\nArguments: $ARGUMENTS\n");

    if !def.command.parameters.is_empty() {
        content.push_str("\n## Parameters\n\n");
        for param in &def.command.parameters {
            let req = if param.required {
                "required"
            } else {
                "optional"
            };
            content.push_str(&format!("- **{}** ({}): {}\n", param.name, req, param.description));
        }
    }

    if let Some(process) = &def.process {
        content.push_str("\n## Process\n\n");
        for (i, step) in process.steps.iter().enumerate() {
            content.push_str(&format!("{}. {}\n", i + 1, step));
        }
    }

    fs::write(&filepath, &content)?;
    Ok(filepath.display().to_string())
}

/// Generate a Gemini CLI command file (TOML with `[prompt]`).
pub fn generate_gemini(def: &ClodDefinition, out_dir: &Path) -> Result<String> {
    let dir = adapter_dir(out_dir, &def.adapters, |a| &a.gemini, "commands");
    fs::create_dir_all(&dir)?;

    let ext = adapter_ext(&def.adapters, |a| &a.gemini, "toml");
    let filename = format!("{}.{}", def.command.name, ext);
    let filepath = dir.join(&filename);

    let mut prompt_body = String::new();
    prompt_body.push_str(&format!("{}\n", def.command.description));
    prompt_body.push_str("\nArguments: {{args}}\n");

    if !def.command.parameters.is_empty() {
        prompt_body.push_str("\nParameters:\n");
        for param in &def.command.parameters {
            let req = if param.required {
                "required"
            } else {
                "optional"
            };
            prompt_body.push_str(&format!("- {} ({}): {}\n", param.name, req, param.description));
        }
    }

    if let Some(process) = &def.process {
        prompt_body.push_str("\nProcess:\n");
        for (i, step) in process.steps.iter().enumerate() {
            prompt_body.push_str(&format!("{}. {}\n", i + 1, step));
        }
    }

    // Build TOML content
    let mut content = String::new();
    content.push_str("[prompt]\n");
    content.push_str(&format!(
        "description = \"{}\"\n",
        def.command.description.replace('"', "\\\"")
    ));
    content.push_str(&format!(
        "command = \"\"\"\n{}\"\"\"\n",
        prompt_body
    ));

    fs::write(&filepath, &content)?;
    Ok(filepath.display().to_string())
}

/// Generate a Copilot CLI skill file (Markdown in skill directory).
pub fn generate_copilot(def: &ClodDefinition, out_dir: &Path) -> Result<String> {
    let dir = adapter_dir(out_dir, &def.adapters, |a| &a.copilot, "skills");
    let skill_dir = dir.join(&def.command.name);
    fs::create_dir_all(&skill_dir)?;

    let ext = adapter_ext(&def.adapters, |a| &a.copilot, "md");
    let filename = format!("SKILL.{}", ext);
    let filepath = skill_dir.join(&filename);

    let title = def
        .command
        .name
        .replace('-', " ")
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("name: {}\n", def.command.name));
    content.push_str(&format!(
        "description: {}\n",
        yaml_escape(&def.command.description)
    ));
    content.push_str("---\n\n");
    content.push_str(&format!("# {}\n\n", title));
    content.push_str(&format!("{}\n", def.command.description));

    if !def.command.parameters.is_empty() {
        content.push_str("\n## Parameters\n\n");
        for param in &def.command.parameters {
            let req = if param.required {
                "required"
            } else {
                "optional"
            };
            content.push_str(&format!("- **{}** ({}): {}\n", param.name, req, param.description));
        }
    }

    if let Some(process) = &def.process {
        content.push_str("\n## Process\n\n");
        for (i, step) in process.steps.iter().enumerate() {
            content.push_str(&format!("{}. {}\n", i + 1, step));
        }
    }

    fs::write(&filepath, &content)?;
    Ok(filepath.display().to_string())
}

/// Generate adapter files for all targets.
pub fn generate_all(def: &ClodDefinition, out_dir: &Path) -> Result<Vec<String>> {
    let files = vec![
        generate_claude(def, out_dir)?,
        generate_opencode(def, out_dir)?,
        generate_gemini(def, out_dir)?,
        generate_copilot(def, out_dir)?,
    ];
    Ok(files)
}

/// Resolve the output directory for an adapter.
fn adapter_dir(
    base: &Path,
    adapters: &Option<ClodAdapters>,
    selector: impl Fn(&ClodAdapters) -> &Option<AdapterConfig>,
    default_dir: &str,
) -> std::path::PathBuf {
    adapters
        .as_ref()
        .and_then(|a| selector(a).as_ref())
        .and_then(|c| c.directory.as_deref())
        .map(|d| base.join(d))
        .unwrap_or_else(|| base.join(default_dir))
}

/// Resolve the file extension for an adapter.
fn adapter_ext(
    adapters: &Option<ClodAdapters>,
    selector: impl Fn(&ClodAdapters) -> &Option<AdapterConfig>,
    default_ext: &str,
) -> String {
    adapters
        .as_ref()
        .and_then(|a| selector(a).as_ref())
        .and_then(|c| c.extension.as_deref())
        .unwrap_or(default_ext)
        .to_string()
}

/// Escape a string for YAML frontmatter values.
fn yaml_escape(s: &str) -> String {
    if s.contains(':') || s.contains('"') || s.contains('\'') || s.contains('#') || s.contains('{') || s.contains('}') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_clod_toml() -> &'static str {
        r#"
[command]
name = "memory-search"
description = "Search past conversations"
version = "1.0.0"

[[command.parameters]]
name = "query"
description = "Search query"
required = true
position = 0

[[command.parameters]]
name = "agent"
description = "Filter by agent"
required = false
flag = "--agent"

[process]
steps = [
    "Parse the query",
    "Run search",
]

[output]
format = "Results: {results}"
"#
    }

    fn minimal_clod_toml() -> &'static str {
        r#"
[command]
name = "memory-recent"
description = "Show recent activity"
"#
    }

    #[test]
    fn test_parse_clod_valid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.toml");
        fs::write(&path, sample_clod_toml()).unwrap();

        let def = parse_clod(&path).unwrap();
        assert_eq!(def.command.name, "memory-search");
        assert_eq!(def.command.version, "1.0.0");
        assert_eq!(def.command.parameters.len(), 2);
        assert!(def.process.is_some());
        assert!(def.output.is_some());
    }

    #[test]
    fn test_parse_clod_minimal() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.toml");
        fs::write(&path, minimal_clod_toml()).unwrap();

        let def = parse_clod(&path).unwrap();
        assert_eq!(def.command.name, "memory-recent");
        assert_eq!(def.command.version, "1.0.0"); // default
        assert!(def.command.parameters.is_empty());
        assert!(def.process.is_none());
    }

    #[test]
    fn test_parse_clod_empty_name_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.toml");
        fs::write(
            &path,
            r#"
[command]
name = ""
description = "test"
"#,
        )
        .unwrap();

        let result = parse_clod(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name is empty"));
    }

    #[test]
    fn test_parse_clod_duplicate_params_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.toml");
        fs::write(
            &path,
            r#"
[command]
name = "test"
description = "test"

[[command.parameters]]
name = "query"
description = "a"
required = true

[[command.parameters]]
name = "query"
description = "b"
required = false
"#,
        )
        .unwrap();

        let result = parse_clod(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn test_generate_claude() {
        let dir = TempDir::new().unwrap();
        let clod_path = dir.path().join("test.toml");
        fs::write(&clod_path, sample_clod_toml()).unwrap();

        let def = parse_clod(&clod_path).unwrap();
        let out = dir.path().join("out");
        let result = generate_claude(&def, &out).unwrap();

        assert!(result.contains("memory-search.md"));
        let content = fs::read_to_string(&result).unwrap();
        assert!(content.contains("name: memory-search"));
        assert!(content.contains("parameters:"));
        assert!(content.contains("## Process"));
    }

    #[test]
    fn test_generate_opencode() {
        let dir = TempDir::new().unwrap();
        let clod_path = dir.path().join("test.toml");
        fs::write(&clod_path, sample_clod_toml()).unwrap();

        let def = parse_clod(&clod_path).unwrap();
        let out = dir.path().join("out");
        let result = generate_opencode(&def, &out).unwrap();

        assert!(result.contains("memory-search.md"));
        let content = fs::read_to_string(&result).unwrap();
        assert!(content.contains("$ARGUMENTS"));
        assert!(content.contains("## Parameters"));
    }

    #[test]
    fn test_generate_gemini() {
        let dir = TempDir::new().unwrap();
        let clod_path = dir.path().join("test.toml");
        fs::write(&clod_path, sample_clod_toml()).unwrap();

        let def = parse_clod(&clod_path).unwrap();
        let out = dir.path().join("out");
        let result = generate_gemini(&def, &out).unwrap();

        assert!(result.contains("memory-search.toml"));
        let content = fs::read_to_string(&result).unwrap();
        assert!(content.contains("[prompt]"));
        assert!(content.contains("{{args}}"));
    }

    #[test]
    fn test_generate_copilot() {
        let dir = TempDir::new().unwrap();
        let clod_path = dir.path().join("test.toml");
        fs::write(&clod_path, sample_clod_toml()).unwrap();

        let def = parse_clod(&clod_path).unwrap();
        let out = dir.path().join("out");
        let result = generate_copilot(&def, &out).unwrap();

        assert!(result.contains("SKILL.md"));
        let content = fs::read_to_string(&result).unwrap();
        assert!(content.contains("# Memory Search"));
        assert!(content.contains("## Parameters"));
    }

    #[test]
    fn test_generate_all() {
        let dir = TempDir::new().unwrap();
        let clod_path = dir.path().join("test.toml");
        fs::write(&clod_path, sample_clod_toml()).unwrap();

        let def = parse_clod(&clod_path).unwrap();
        let out = dir.path().join("out");
        let files = generate_all(&def, &out).unwrap();

        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_yaml_escape() {
        assert_eq!(yaml_escape("simple text"), "simple text");
        assert_eq!(
            yaml_escape("text: with colon"),
            "\"text: with colon\""
        );
        assert_eq!(
            yaml_escape("text with \"quotes\""),
            "\"text with \\\"quotes\\\"\""
        );
    }
}
