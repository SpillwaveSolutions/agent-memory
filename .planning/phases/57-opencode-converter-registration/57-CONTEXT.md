# Phase 57: OpenCode Converter + Registration - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning
**Source:** Codebase-mentor reference + existing stub analysis

<domain>
## Phase Boundary

This phase replaces the stub `OpenCodeConverter` (all methods return `Vec::new()`) with a full implementation that produces correctly-formatted OpenCode files. Also adds runtime registration by writing `opencode.json` permissions. The converter follows the same `RuntimeConverter` trait pattern as the 5 working converters.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- Modify existing `crates/memory-installer/src/converters/opencode.rs` — replace stub with real implementation
- Follow patterns from `claude.rs`, `gemini.rs`, `codex.rs` converters (all in same directory)
- Reuse existing helpers: `value_to_yaml`, `reconstruct_md`, `rewrite_paths` from `converters/helpers.rs`
- Reuse existing tool mapping: `tool_maps::map_tool(Runtime::OpenCode, tool_name)` already has mappings

### Command Conversion (OC-01)
- Commands flattened: `commands/memory-search.md` → `command/memory-search.md` (singular directory)
- Same YAML frontmatter structure, just path change
- Rewrite internal paths from `~/.claude/` to `~/.config/opencode/`

### Agent Conversion (OC-02, OC-03, OC-04)
- `allowed-tools:` array → `tools:` object with `{ tool_name: true }` entries
- Tool names converted via `tool_maps::map_tool(Runtime::OpenCode, name)`:
  - AskUserQuestion → question
  - SkillTool → skill
  - TodoWrite → todowrite
  - WebFetch → webfetch
  - WebSearch → websearch
  - Others → lowercase
  - mcp__* → passed through unchanged
- `name:` field removed (OpenCode derives from filename)
- Color values: named colors → hex (`cyan` → `#00FFFF`, `blue` → `#0000FF`, etc.)
- `subagent_type: "general-purpose"` → `"general"`

### Path Rewriting (OC-05)
- `~/.claude/plugins/` → `~/.config/opencode/`
- `~/.claude/` → `~/.config/opencode/`
- `~/.config/agent-memory/` paths left unchanged (runtime-neutral)

### OpenCode Registration (OC-06, OREG-01..03)
- After file conversion, write/merge `opencode.json` with read permissions
- Permission format:
  ```json
  {
    "permission": {
      "read": { "~/.config/opencode/agent-memory/*": "allow" },
      "external_directory": { "~/.config/opencode/agent-memory/*": "allow" }
    }
  }
  ```
- Must MERGE with existing `opencode.json` (not overwrite)
- Glob patterns match installed skill/command directories

### Reference Implementation
- `/Users/richardhightower/clients/spillwave/src/codebase-mentor/ai_codebase_mentor/converters/opencode.py`
- Key patterns: tool name mapping, color hex conversion, frontmatter transformation, permission writing
- Agent-memory already has the Rust equivalent infrastructure — this phase fills in the OpenCode-specific logic

### Claude's Discretion
- Exact hex color map (named colors to hex values)
- Whether to include `description:` field in agent `tools:` object or just `true`
- Error handling for malformed frontmatter in canonical source

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing Stub (REPLACE)
- `crates/memory-installer/src/converters/opencode.rs` — current stub returning empty Vecs

### Working Converter Patterns (FOLLOW)
- `crates/memory-installer/src/converters/claude.rs` — pass-through with path rewriting
- `crates/memory-installer/src/converters/gemini.rs` — TOML conversion, tool mapping, settings merge
- `crates/memory-installer/src/converters/codex.rs` — command-to-skill, AGENTS.md generation
- `crates/memory-installer/src/converters/helpers.rs` — value_to_yaml, reconstruct_md, rewrite_paths

### Tool Mapping
- `crates/memory-installer/src/tool_maps.rs` — `map_tool(Runtime::OpenCode, name)` already defined

### Reference Implementation
- `/Users/richardhightower/clients/spillwave/src/codebase-mentor/ai_codebase_mentor/converters/opencode.py` — Python reference for OpenCode conversion patterns

### Types
- `crates/memory-installer/src/types.rs` — ConvertedFile, PluginCommand, PluginAgent, PluginSkill, InstallConfig
- `crates/memory-installer/src/converter.rs` — RuntimeConverter trait

</canonical_refs>

<specifics>
## Specific Ideas

- The existing E2E test `opencode_stub` in `tests/e2e_converters.rs` should be updated to verify real output
- Color map: cyan=#00FFFF, blue=#0000FF, green=#00FF00, red=#FF0000, yellow=#FFFF00, magenta=#FF00FF, white=#FFFFFF
- OpenCode's `opencode.json` is similar to Claude's `settings.json` — JSON merge pattern already exists in `managed_sections.rs`

</specifics>

<deferred>
## Deferred Ideas

- OpenCode hook support (hooks not yet standardized in OpenCode)
- Interactive mode for OpenCode installer
- Gemini/Codex/Copilot registration (separate phases)

</deferred>

---

*Phase: 57-opencode-converter-registration*
*Context gathered: 2026-03-25*
