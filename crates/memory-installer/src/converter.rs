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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converters::select_converter;
    use crate::types::Runtime;

    #[test]
    fn select_converter_returns_correct_name_for_claude() {
        let converter = select_converter(Runtime::Claude);
        assert_eq!(converter.name(), "claude");
    }

    #[test]
    fn select_converter_returns_correct_name_for_opencode() {
        let converter = select_converter(Runtime::OpenCode);
        assert_eq!(converter.name(), "opencode");
    }

    #[test]
    fn select_converter_returns_correct_name_for_gemini() {
        let converter = select_converter(Runtime::Gemini);
        assert_eq!(converter.name(), "gemini");
    }

    #[test]
    fn select_converter_returns_correct_name_for_codex() {
        let converter = select_converter(Runtime::Codex);
        assert_eq!(converter.name(), "codex");
    }

    #[test]
    fn select_converter_returns_correct_name_for_copilot() {
        let converter = select_converter(Runtime::Copilot);
        assert_eq!(converter.name(), "copilot");
    }

    #[test]
    fn select_converter_returns_correct_name_for_skills() {
        let converter = select_converter(Runtime::Skills);
        assert_eq!(converter.name(), "skills");
    }

    #[test]
    fn all_converters_return_empty_results_for_stubs() {
        let cfg = InstallConfig {
            scope: InstallScope::Project(PathBuf::from("/tmp/test")),
            dry_run: false,
            source_root: PathBuf::from("/tmp/src"),
        };
        let cmd = PluginCommand {
            name: "test-cmd".to_string(),
            frontmatter: serde_json::Value::Null,
            body: String::new(),
            source_path: PathBuf::from("test.md"),
        };

        for runtime in [
            Runtime::Claude,
            Runtime::OpenCode,
            Runtime::Gemini,
            Runtime::Codex,
            Runtime::Copilot,
            Runtime::Skills,
        ] {
            let converter = select_converter(runtime);
            assert!(
                converter.convert_command(&cmd, &cfg).is_empty(),
                "stub converter for {:?} should return empty Vec",
                runtime
            );
        }
    }

    #[test]
    fn target_dir_project_scope_contains_runtime_specific_path() {
        let root = PathBuf::from("/project");
        let scope = InstallScope::Project(root);

        let claude_dir = select_converter(Runtime::Claude).target_dir(&scope);
        assert!(claude_dir.to_str().unwrap().contains(".claude"));

        let opencode_dir = select_converter(Runtime::OpenCode).target_dir(&scope);
        assert!(opencode_dir.to_str().unwrap().contains(".opencode"));

        let gemini_dir = select_converter(Runtime::Gemini).target_dir(&scope);
        assert!(gemini_dir.to_str().unwrap().contains(".gemini"));
    }
}
