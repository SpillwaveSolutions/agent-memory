use std::path::PathBuf;

use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill,
};

/// Trait for converting canonical Claude-format plugins to a specific runtime's format.
///
/// Each runtime (Claude, OpenCode, Gemini, Codex, Copilot, Skills) implements this trait.
/// Converters are stateless -- all configuration is passed via [`InstallConfig`].
pub trait RuntimeConverter {
    /// Human-readable name for this runtime (e.g., "claude", "opencode").
    fn name(&self) -> &str;

    /// Target directory for this runtime given the install scope.
    fn target_dir(&self, scope: &InstallScope) -> PathBuf;

    /// Convert a single command definition to this runtime's format.
    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile>;

    /// Convert a single agent definition to this runtime's format.
    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile>;

    /// Convert a single skill definition to this runtime's format.
    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile>;

    /// Convert a single hook definition to this runtime's format.
    fn convert_hook(&self, hook: &HookDefinition, cfg: &InstallConfig) -> Option<ConvertedFile>;

    /// Generate any runtime-specific guidance or configuration files.
    fn generate_guidance(
        &self,
        bundle: &PluginBundle,
        cfg: &InstallConfig,
    ) -> Vec<ConvertedFile>;
}
