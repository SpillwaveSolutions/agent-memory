//! Centralized tool name mapping tables for all AI runtimes.
//!
//! Maps Claude PascalCase tool names to runtime-specific equivalents.
//! Returns `Option<&'static str>` -- `None` means the tool is excluded for that runtime.
//!
//! **MCP tools (`mcp__*`):** Callers must check `tool_name.starts_with("mcp__")` before
//! calling `map_tool`. MCP tools pass through unchanged for Claude/OpenCode and are
//! excluded (None) for Gemini/Codex/Copilot. This keeps `map_tool` simple with a
//! static return type.

use crate::types::Runtime;

/// Map a Claude tool name to the equivalent name for a target runtime.
///
/// Returns `Some(mapped_name)` if the tool is supported, or `None` if the
/// tool is excluded for that runtime (e.g., `Task` on Gemini) or is unknown.
///
/// # Panics
///
/// Does not panic. Unknown tool names return `None`.
pub fn map_tool(runtime: Runtime, claude_name: &str) -> Option<&'static str> {
    match (runtime, claude_name) {
        // Claude: pass-through (canonical names)
        (Runtime::Claude, "Read") => Some("Read"),
        (Runtime::Claude, "Write") => Some("Write"),
        (Runtime::Claude, "Edit") => Some("Edit"),
        (Runtime::Claude, "Bash") => Some("Bash"),
        (Runtime::Claude, "Grep") => Some("Grep"),
        (Runtime::Claude, "Glob") => Some("Glob"),
        (Runtime::Claude, "WebSearch") => Some("WebSearch"),
        (Runtime::Claude, "WebFetch") => Some("WebFetch"),
        (Runtime::Claude, "TodoWrite") => Some("TodoWrite"),
        (Runtime::Claude, "AskUserQuestion") => Some("AskUserQuestion"),
        (Runtime::Claude, "Task") => Some("Task"),

        // Skills: same as Claude (pass-through for generic runtimes)
        (Runtime::Skills, "Read") => Some("Read"),
        (Runtime::Skills, "Write") => Some("Write"),
        (Runtime::Skills, "Edit") => Some("Edit"),
        (Runtime::Skills, "Bash") => Some("Bash"),
        (Runtime::Skills, "Grep") => Some("Grep"),
        (Runtime::Skills, "Glob") => Some("Glob"),
        (Runtime::Skills, "WebSearch") => Some("WebSearch"),
        (Runtime::Skills, "WebFetch") => Some("WebFetch"),
        (Runtime::Skills, "TodoWrite") => Some("TodoWrite"),
        (Runtime::Skills, "AskUserQuestion") => Some("AskUserQuestion"),
        (Runtime::Skills, "Task") => Some("Task"),

        // OpenCode: lowercase equivalents
        (Runtime::OpenCode, "Read") => Some("read"),
        (Runtime::OpenCode, "Write") => Some("write"),
        (Runtime::OpenCode, "Edit") => Some("edit"),
        (Runtime::OpenCode, "Bash") => Some("bash"),
        (Runtime::OpenCode, "Grep") => Some("grep"),
        (Runtime::OpenCode, "Glob") => Some("glob"),
        (Runtime::OpenCode, "WebSearch") => Some("websearch"),
        (Runtime::OpenCode, "WebFetch") => Some("webfetch"),
        (Runtime::OpenCode, "TodoWrite") => Some("todowrite"),
        (Runtime::OpenCode, "AskUserQuestion") => Some("question"),
        (Runtime::OpenCode, "Task") => Some("task"),

        // Gemini: snake_case / Gemini-specific names; Task excluded
        (Runtime::Gemini, "Read") => Some("read_file"),
        (Runtime::Gemini, "Write") => Some("write_file"),
        (Runtime::Gemini, "Edit") => Some("replace"),
        (Runtime::Gemini, "Bash") => Some("run_shell_command"),
        (Runtime::Gemini, "Grep") => Some("search_file_content"),
        (Runtime::Gemini, "Glob") => Some("glob"),
        (Runtime::Gemini, "WebSearch") => Some("google_web_search"),
        (Runtime::Gemini, "WebFetch") => Some("web_fetch"),
        (Runtime::Gemini, "TodoWrite") => Some("write_todos"),
        (Runtime::Gemini, "AskUserQuestion") => Some("ask_user"),
        (Runtime::Gemini, "Task") => None, // Gemini auto-discovers; excluded

        // Codex: simplified names
        (Runtime::Codex, "Read") => Some("read"),
        (Runtime::Codex, "Write") => Some("edit"),
        (Runtime::Codex, "Edit") => Some("edit"),
        (Runtime::Codex, "Bash") => Some("execute"),
        (Runtime::Codex, "Grep") => Some("search"),
        (Runtime::Codex, "Glob") => Some("search"),
        (Runtime::Codex, "WebSearch") => Some("web"),
        (Runtime::Codex, "WebFetch") => Some("web"),
        (Runtime::Codex, "TodoWrite") => Some("todo"),
        (Runtime::Codex, "AskUserQuestion") => Some("ask_user"),
        (Runtime::Codex, "Task") => Some("agent"),

        // Copilot: same mappings as Codex
        (Runtime::Copilot, "Read") => Some("read"),
        (Runtime::Copilot, "Write") => Some("edit"),
        (Runtime::Copilot, "Edit") => Some("edit"),
        (Runtime::Copilot, "Bash") => Some("execute"),
        (Runtime::Copilot, "Grep") => Some("search"),
        (Runtime::Copilot, "Glob") => Some("search"),
        (Runtime::Copilot, "WebSearch") => Some("web"),
        (Runtime::Copilot, "WebFetch") => Some("web"),
        (Runtime::Copilot, "TodoWrite") => Some("todo"),
        (Runtime::Copilot, "AskUserQuestion") => Some("ask_user"),
        (Runtime::Copilot, "Task") => Some("agent"),

        // Unknown tool name for any runtime
        _ => {
            tracing::warn!("unmapped tool '{}' for {:?} — skipping", claude_name, runtime);
            None
        }
    }
}

/// All 11 known Claude tool names, in canonical order.
pub const KNOWN_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "Edit",
    "Bash",
    "Grep",
    "Glob",
    "WebSearch",
    "WebFetch",
    "TodoWrite",
    "AskUserQuestion",
    "Task",
];

#[cfg(test)]
mod tests {
    use super::*;

    // --- Individual mapping tests ---

    #[test]
    fn opencode_read() {
        assert_eq!(map_tool(Runtime::OpenCode, "Read"), Some("read"));
    }

    #[test]
    fn opencode_write() {
        assert_eq!(map_tool(Runtime::OpenCode, "Write"), Some("write"));
    }

    #[test]
    fn opencode_ask_user_question() {
        assert_eq!(
            map_tool(Runtime::OpenCode, "AskUserQuestion"),
            Some("question")
        );
    }

    #[test]
    fn gemini_read() {
        assert_eq!(map_tool(Runtime::Gemini, "Read"), Some("read_file"));
    }

    #[test]
    fn gemini_bash() {
        assert_eq!(map_tool(Runtime::Gemini, "Bash"), Some("run_shell_command"));
    }

    #[test]
    fn gemini_task_excluded() {
        assert_eq!(map_tool(Runtime::Gemini, "Task"), None);
    }

    #[test]
    fn codex_write() {
        assert_eq!(map_tool(Runtime::Codex, "Write"), Some("edit"));
    }

    #[test]
    fn copilot_bash() {
        assert_eq!(map_tool(Runtime::Copilot, "Bash"), Some("execute"));
    }

    #[test]
    fn claude_read_passthrough() {
        assert_eq!(map_tool(Runtime::Claude, "Read"), Some("Read"));
    }

    #[test]
    fn skills_read_passthrough() {
        assert_eq!(map_tool(Runtime::Skills, "Read"), Some("Read"));
    }

    #[test]
    fn unknown_tool_returns_none() {
        assert_eq!(map_tool(Runtime::OpenCode, "UnknownTool"), None);
    }

    // --- Exhaustive coverage tests ---

    #[test]
    fn all_11_tools_return_some_for_opencode() {
        for tool in KNOWN_TOOLS {
            assert!(
                map_tool(Runtime::OpenCode, tool).is_some(),
                "OpenCode should map tool '{tool}'"
            );
        }
    }

    #[test]
    fn gemini_maps_10_returns_none_for_task() {
        let mut some_count = 0;
        let mut none_count = 0;
        for tool in KNOWN_TOOLS {
            match map_tool(Runtime::Gemini, tool) {
                Some(_) => some_count += 1,
                None => none_count += 1,
            }
        }
        assert_eq!(some_count, 10, "Gemini should map 10 tools");
        assert_eq!(none_count, 1, "Gemini should exclude 1 tool (Task)");
    }

    #[test]
    fn all_11_tools_return_some_for_claude() {
        for tool in KNOWN_TOOLS {
            assert!(
                map_tool(Runtime::Claude, tool).is_some(),
                "Claude should map tool '{tool}'"
            );
        }
    }

    #[test]
    fn all_11_tools_return_some_for_skills() {
        for tool in KNOWN_TOOLS {
            assert!(
                map_tool(Runtime::Skills, tool).is_some(),
                "Skills should map tool '{tool}'"
            );
        }
    }

    #[test]
    fn all_11_tools_return_some_for_codex() {
        for tool in KNOWN_TOOLS {
            assert!(
                map_tool(Runtime::Codex, tool).is_some(),
                "Codex should map tool '{tool}'"
            );
        }
    }

    #[test]
    fn all_11_tools_return_some_for_copilot() {
        for tool in KNOWN_TOOLS {
            assert!(
                map_tool(Runtime::Copilot, tool).is_some(),
                "Copilot should map tool '{tool}'"
            );
        }
    }

    #[test]
    fn unknown_tool_none_for_all_runtimes() {
        for runtime in [
            Runtime::Claude,
            Runtime::OpenCode,
            Runtime::Gemini,
            Runtime::Codex,
            Runtime::Copilot,
            Runtime::Skills,
        ] {
            assert_eq!(
                map_tool(runtime, "NonExistentTool"),
                None,
                "Unknown tool should return None for {:?}",
                runtime
            );
        }
    }
}
