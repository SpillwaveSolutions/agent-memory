//! Plugin parser that reads installer-sources.json, walks plugin directories,
//! extracts YAML frontmatter + markdown bodies, and returns a complete PluginBundle.

use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use crate::types::PluginBundle;

/// Parse a markdown file, extracting YAML frontmatter as `serde_json::Value` and
/// the body content as a `String`.
///
/// If the file has no frontmatter (no `---` delimiters), returns an empty
/// `Value::Object` and the full file content as the body.
pub fn parse_md_file(_path: &Path) -> Result<(Value, String)> {
    unimplemented!("parse_md_file not yet implemented")
}

/// Parse all plugin sources from the given source root directory.
///
/// Reads `{source_root}/installer-sources.json` to discover plugin directories,
/// then walks each directory using its `marketplace.json` to find commands,
/// agents, and skills.
pub fn parse_sources(_source_root: &Path) -> Result<PluginBundle> {
    unimplemented!("parse_sources not yet implemented")
}

/// Extract a name from a file path by taking the stem (filename without extension).
fn name_from_path(_path: &Path) -> String {
    unimplemented!("name_from_path not yet implemented")
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
