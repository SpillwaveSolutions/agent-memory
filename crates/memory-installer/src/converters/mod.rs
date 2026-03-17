pub mod claude;
pub mod codex;
pub mod copilot;
pub mod gemini;
pub mod opencode;
pub mod skills;

pub use claude::ClaudeConverter;
pub use codex::CodexConverter;
pub use copilot::CopilotConverter;
pub use gemini::GeminiConverter;
pub use opencode::OpenCodeConverter;
pub use skills::SkillsConverter;

use crate::converter::RuntimeConverter;
use crate::types::Runtime;

/// Select the appropriate converter for the given runtime.
pub fn select_converter(runtime: Runtime) -> Box<dyn RuntimeConverter> {
    match runtime {
        Runtime::Claude => Box::new(ClaudeConverter),
        Runtime::OpenCode => Box::new(OpenCodeConverter),
        Runtime::Gemini => Box::new(GeminiConverter),
        Runtime::Codex => Box::new(CodexConverter),
        Runtime::Copilot => Box::new(CopilotConverter),
        Runtime::Skills => Box::new(SkillsConverter),
    }
}
