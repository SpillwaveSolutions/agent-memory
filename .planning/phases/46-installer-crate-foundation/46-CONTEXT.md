# Phase 46: Installer Crate Foundation - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning
**Source:** Derived from v2.7 milestone conversations, research synthesis, and Phase 45 decisions

<domain>
## Phase Boundary

Create a new `memory-installer` Rust crate in the workspace with CLI, plugin parser, RuntimeConverter trait, centralized tool mapping tables, managed-section markers, and --dry-run support. This phase produces the foundation that all converter phases (47-49) build on. No actual converters are implemented here — only the trait, types, parser, and CLI scaffold.

</domain>

<decisions>
## Implementation Decisions

### Binary architecture
- Standalone `memory-installer` binary — NOT a subcommand of memory-daemon
- Zero coupling to daemon (no gRPC, no RocksDB, no tokio)
- Synchronous only — pure file I/O, no async needed
- Ships as part of the cross-compiled release (same CI pipeline as memory-daemon)
- Precedent: memory-ingest is already a separate workspace binary

### Dependencies
- `gray_matter` 0.3.x for YAML frontmatter parsing (serde_yaml is deprecated per Stack research)
- `walkdir` 2.5 for directory traversal
- `clap` (already in workspace) for CLI
- `toml` (already in workspace) for Gemini TOML generation (used in Phase 48)
- `serde` + `serde_json` (already in workspace) for types
- `shellexpand` (already in workspace) for ~ path expansion
- `directories` (already in workspace) for XDG paths
- `anyhow` + `thiserror` (already in workspace) for errors
- Only 2 NEW external dependencies: gray_matter, walkdir

### Plugin parser
- Reads `plugins/installer-sources.json` (created in Phase 45) to discover plugin source directories
- Walks each source directory using marketplace.json to find commands, agents, skills
- Extracts YAML frontmatter + markdown body from each .md file
- Returns a PluginBundle containing all parsed artifacts
- Must handle: frontmatter with arrays (allowed-tools, skills, triggers), multiline descriptions, special characters

### RuntimeConverter trait
- One impl per runtime (ClaudeConverter, OpenCodeConverter, GeminiConverter, CodexConverter, CopilotConverter, SkillsConverter)
- Methods: convert_command, convert_agent, convert_skill, convert_hook, generate_guidance, target_dir
- Each method returns ConvertedFile(s) with target path + content
- Converters are stateless — all config passed via InstallConfig struct

### Tool mapping tables
- Centralized in tool_maps.rs
- Static BTreeMap or match-based lookup: map_tool(runtime, claude_tool_name) -> Option<String>
- 11 Claude tools × 6 runtimes (see v2.7 plan for full table)
- Unmapped tools return None → caller logs warning (INST-07)
- MCP tools (mcp__*) pass through unchanged for Claude/OpenCode, excluded for Gemini

### CLI design
- `memory-installer install --agent <runtime> [--project|--global] [--dir <path>] [--dry-run]`
- Runtimes: claude, opencode, gemini, codex, copilot, skills
- `--project` installs to current directory (e.g., .claude/, .opencode/, .gemini/)
- `--global` installs to user config dir (e.g., ~/.claude/, ~/.config/opencode/, ~/.gemini/)
- `--dir <path>` for generic skills target (required with --agent skills)
- `--dry-run` shows what would be installed without writing files
- No interactive prompts (anti-feature per Features research)
- Exit codes: 0 success, 1 error

### Managed-section markers
- Format: `# --- MANAGED BY memory-installer (DO NOT EDIT) ---` / `# --- END MANAGED ---`
- Used when merging into shared config files (opencode.json, .gemini/settings.json)
- Installer replaces content between markers on upgrade, preserves content outside
- Marker format is a compatibility contract — decided in Phase 46, never changed
- JSON variant: `"__managed_by": "memory-installer"` field in managed objects

### Dry-run implementation
- Write-interceptor pattern on output stage, not per-converter
- All converters produce Vec<ConvertedFile> → dry-run prints paths + previews instead of writing
- Shows: target path, file size, whether it would overwrite existing

### Claude's Discretion
- Exact module layout within crates/memory-installer/src/
- Error types and error handling patterns
- Whether to use a trait object (dyn RuntimeConverter) or an enum dispatch
- Test file organization
- Whether PluginBundle fields use owned Strings or borrowed &str

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Cargo.toml` workspace already has clap, serde, serde_json, toml, anyhow, thiserror, shellexpand, directories
- `plugins/installer-sources.json` (Phase 45) provides discovery manifest
- `plugins/memory-query-plugin/.claude-plugin/marketplace.json` defines asset paths
- `plugins/memory-setup-plugin/.claude-plugin/marketplace.json` defines asset paths

### Established Patterns
- Workspace crates follow `crates/<name>/` layout with `src/main.rs` or `src/lib.rs`
- Binary crates: memory-daemon (complex, async), memory-ingest (simple, sync) — installer follows memory-ingest pattern
- All crates use `version.workspace = true` for version inheritance
- CLAUDE.md requires: cargo fmt, cargo clippy --workspace -D warnings, cargo test, cargo doc

### Integration Points
- Add `memory-installer` to workspace members in root Cargo.toml
- Add gray_matter and walkdir to [workspace.dependencies]
- Phase 47-49 converters will add files to crates/memory-installer/src/converters/
- CI pipeline (ci.yml) automatically picks up new workspace crates

</code_context>

<specifics>
## Specific Ideas

- Model the tool mapping table after GSD's approach (see /Users/richardhightower/src/get-shit-done/bin/install.js lines 475-530)
- The converter trait should be simple enough that adding a new runtime is one new file
- Parser should be robust against missing frontmatter (treat as empty metadata + full body)
- Consider the GSD article's side-by-side runtime comparison table as the reference for what each converter must handle

</specifics>

<deferred>
## Deferred Ideas

- Actual converter implementations — Phases 47-49
- Hook conversion pipeline — Phase 49
- E2E testing of installs — Phase 50
- --uninstall command — v2.8
- --all flag — v2.8
- Interactive mode — v2.8
- Version tracking with upgrade detection — v2.8

</deferred>

---

*Phase: 46-installer-crate-foundation*
*Context gathered: 2026-03-17 from milestone conversations and research*
