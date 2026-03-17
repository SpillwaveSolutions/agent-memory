use std::path::PathBuf;

use crate::converter::RuntimeConverter;
use crate::types::{
    ConvertedFile, HookDefinition, InstallConfig, InstallScope, PluginAgent, PluginBundle,
    PluginCommand, PluginSkill,
};

pub struct ClaudeConverter;

#[allow(unused_variables)]
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
        Vec::new()
    }

    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        Vec::new()
    }

    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        Vec::new()
    }

    fn convert_hook(&self, hook: &HookDefinition, cfg: &InstallConfig) -> Option<ConvertedFile> {
        None
    }

    fn generate_guidance(&self, bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile> {
        Vec::new()
    }
}
