use std::path::PathBuf;

/// Target AI runtime for plugin installation.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Claude,
    OpenCode,
    Gemini,
    Codex,
    Copilot,
    Skills,
}

/// Where plugins should be installed.
#[derive(Debug, Clone)]
pub enum InstallScope {
    /// Project-local directory (e.g., `./.claude/`).
    Project(PathBuf),
    /// Global user config directory (e.g., `~/.claude/`).
    Global,
    /// Explicit custom directory (required with `--agent skills`).
    Custom(PathBuf),
}

/// Configuration threaded through the converter pipeline.
#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Where to install (project, global, or custom directory).
    pub scope: InstallScope,
    /// If true, print what would be installed without writing files.
    pub dry_run: bool,
    /// Root directory containing the `plugins/` source tree.
    pub source_root: PathBuf,
}

/// A complete bundle of parsed plugin artifacts ready for conversion.
#[derive(Debug, Clone)]
pub struct PluginBundle {
    pub commands: Vec<PluginCommand>,
    pub agents: Vec<PluginAgent>,
    pub skills: Vec<PluginSkill>,
    pub hooks: Vec<HookDefinition>,
}

/// A parsed command definition from a `.md` file.
#[derive(Debug, Clone)]
pub struct PluginCommand {
    pub name: String,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub source_path: PathBuf,
}

/// A parsed agent definition from a `.md` file.
#[derive(Debug, Clone)]
pub struct PluginAgent {
    pub name: String,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub source_path: PathBuf,
}

/// A parsed skill definition from a skill directory.
#[derive(Debug, Clone)]
pub struct PluginSkill {
    pub name: String,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub source_path: PathBuf,
    pub additional_files: Vec<SkillFile>,
}

/// An additional file within a skill directory (references, scripts, etc.).
#[derive(Debug, Clone)]
pub struct SkillFile {
    pub relative_path: PathBuf,
    pub content: String,
}

/// A parsed hook definition from a `.md` file.
#[derive(Debug, Clone)]
pub struct HookDefinition {
    pub name: String,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub source_path: PathBuf,
}

/// A file produced by a converter, ready to be written to the target.
#[derive(Debug, Clone)]
pub struct ConvertedFile {
    pub target_path: PathBuf,
    pub content: String,
}

/// Managed-section begin marker.
/// THIS STRING IS A COMPATIBILITY CONTRACT -- never change it after first release.
pub const MANAGED_BEGIN: &str = "# --- MANAGED BY memory-installer (DO NOT EDIT) ---";

/// Managed-section end marker.
/// THIS STRING IS A COMPATIBILITY CONTRACT -- never change it after first release.
pub const MANAGED_END: &str = "# --- END MANAGED ---";

/// JSON key for managed-section identification in JSON config files.
pub const MANAGED_JSON_KEY: &str = "__managed_by";

/// JSON value for managed-section identification in JSON config files.
pub const MANAGED_JSON_VALUE: &str = "memory-installer";
