# Phase 47: Claude & OpenCode Converters - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning
**Source:** Derived from Phase 46 implementation, v2.7 requirements, and codebase analysis

<domain>
## Phase Boundary

Implement the Claude and OpenCode converters within the existing `memory-installer` crate. Phase 46 created the foundation: `RuntimeConverter` trait, `ClaudeConverter`/`OpenCodeConverter` stubs, tool mapping tables, plugin parser, writer with dry-run support, and managed-section markers. This phase fills in the stub methods so that `memory-installer install --agent claude` and `memory-installer install --agent opencode` produce correct output.

</domain>

<decisions>
## Implementation Decisions

### Claude converter (CLAUDE-01, CLAUDE-02)
- Near pass-through: copy commands, agents, skills with minimal transformation
- Path rewriting only: replace `~/.claude/` references in file content with `~/.config/agent-memory/`
- Frontmatter and body pass through unchanged (canonical source IS Claude format)
- `convert_command`: reconstruct frontmatter YAML + body, emit to `commands/<name>.md`
- `convert_agent`: reconstruct frontmatter YAML + body, emit to `agents/<name>.md`
- `convert_skill`: copy SKILL.md + additional_files into `skills/<name>/` directory
- `convert_hook`: pass-through (hooks are already Claude format) — but hooks are deferred to Phase 49, so return None for now
- `generate_guidance`: empty for Claude (no extra config needed)
- Target dir structure mirrors canonical: `.claude/plugins/memory-plugin/{commands,agents,skills}/`

### OpenCode converter (OC-01 through OC-06)
- **OC-01 — Flat naming**: `commands/` → `command/` (singular), same for agents → `agent/`
- **OC-02 — Tools object format**: Convert agent frontmatter `allowed-tools:` array to `tools:` object with `{tool_name: true}` entries
- **OC-03 — Tool name mapping**: Use `tool_maps::map_tool(Runtime::OpenCode, name)` for each tool reference in frontmatter
- **OC-04 — Color hex normalization**: Convert named colors (e.g., "blue", "green") to hex values (#0000FF, etc.) in agent frontmatter `color:` field
- **OC-05 — Path rewriting**: Replace `~/.claude/` with `~/.config/opencode/` in all file content
- **OC-06 — opencode.json permissions**: `generate_guidance` produces managed section for `opencode.json` with `read` permissions for installed skill paths
- `convert_hook`: return None (hooks deferred to Phase 49)

### Frontmatter reconstruction
- Use `serde_json::Value` (already parsed by Phase 46 parser) to manipulate frontmatter fields
- Serialize back to YAML using a simple key-value emitter (avoid pulling in serde_yaml since gray_matter already handles parsing)
- Alternative: use `serde_yaml` if it's simpler — it's deprecated but still functional, or write a minimal YAML serializer for the flat frontmatter we have
- Frontmatter fields are flat key-value with occasional arrays — no deeply nested structures

### Color name to hex mapping
- Maintain a small static lookup table in the OpenCode converter (or shared utility)
- Common CSS color names: blue→#0000FF, green→#008000, red→#FF0000, etc.
- Unknown color names pass through unchanged (warn via tracing)

### opencode.json managed section
- Use `writer::merge_managed_section` with JSON variant (MANAGED_JSON_KEY/MANAGED_JSON_VALUE)
- The managed section adds read permissions for each installed skill path
- Format: `"permissions": { "read": ["path1", "path2"] }` within the managed object

### Testing strategy
- Unit tests for each converter method with known input → expected output
- Test path rewriting with various `~/.claude/` patterns in content
- Test OpenCode tool name mapping via frontmatter transformation
- Test color hex normalization with known and unknown color names
- Test flat naming (commands/ → command/) in output paths
- Integration test: parse canonical plugin → convert → verify output structure

### Claude's Discretion
- Whether to use `serde_yaml` for serialization or write a minimal YAML emitter
- Exact format of the opencode.json permissions block
- Whether color map lives in opencode.rs or a shared utility module
- Error handling for malformed frontmatter fields (e.g., non-string color values)

</decisions>

<specifics>
## Specific Ideas

- The Phase 46 stubs in `converters/claude.rs` and `converters/opencode.rs` return `Vec::new()` / `None` — fill these in
- The `PluginCommand`, `PluginAgent`, `PluginSkill` types already have `frontmatter: serde_json::Value` — manipulate this directly
- The tool_maps.rs `map_tool(Runtime::OpenCode, ...)` function already maps AskUserQuestion→question, all tools→lowercase
- The writer.rs `write_files` and `merge_managed_section` functions are already tested and ready to use
- For YAML serialization, consider the `gray_matter` crate's capabilities or a simple format! macro for flat frontmatter

</specifics>

<deferred>
## Deferred Ideas

- Hook conversion for both Claude and OpenCode — Phase 49
- Gemini converter — Phase 48
- Codex/Copilot converters — Phase 49
- E2E install tests with temp directories — Phase 50
- --uninstall command — v2.8
- Interactive mode — v2.8

</deferred>

---

*Phase: 47-claude-opencode-converters*
*Context gathered: 2026-03-17 from Phase 46 implementation and v2.7 requirements*
