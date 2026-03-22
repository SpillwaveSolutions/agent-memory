# Phase 50: Integration Testing & Migration - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning
**Source:** Conversation context from Phase 45-49 execution

<domain>
## Phase Boundary

This phase delivers E2E tests proving all 6 converters produce correct output, archives the old adapter directories, and confirms CI coverage. It is the final phase of the v2.7 Multi-Runtime Portability milestone.

</domain>

<decisions>
## Implementation Decisions

### E2E Test Approach
- E2E tests should install to temp directories (use `tempdir` crate or `std::env::temp_dir`) for each of the 6 runtimes (Claude, Codex, Gemini, Copilot, OpenCode, Skills)
- Tests verify file structure: correct directories, file names, file extensions
- Tests verify frontmatter conversion: tool name mapping per runtime, YAML-to-TOML for Gemini, field transformations
- E2E tests should use a small canonical test bundle (PluginBundle with 1 command, 1 agent, 1 skill, 1 hook) to keep tests focused
- Tests belong in `crates/memory-installer/tests/` (integration tests) not in the unit test modules

### Adapter Archival
- Old directories to archive: `plugins/memory-copilot-adapter/`, `plugins/memory-gemini-adapter/`, `plugins/memory-opencode-plugin/`
- Keep `plugins/memory-query-plugin/` and `plugins/memory-setup-plugin/` (still active)
- Archive means: replace contents with a single README.md stub pointing users to `memory-installer`
- Do NOT delete the directories — keep them with README stubs for one release cycle (MIG-F01 handles deletion later)
- The `plugins/installer-sources.json` file should remain (it's the canonical source manifest for memory-installer)

### CI Integration
- CI already uses `--workspace` flags (fmt, clippy, test, build, doc) which automatically includes `memory-installer`
- MIG-04 is likely already satisfied — verify by confirming memory-installer appears in workspace members in root Cargo.toml
- If additional CI steps are needed (e.g., dedicated E2E test step for installer), add them

### Claude's Discretion
- Test helper structure and shared fixtures
- Whether to use `assert_cmd` or direct Rust function calls for E2E tests
- Exact wording of README archive stubs
- Whether to include `include_str!` hook script tests in E2E scope

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Converter Implementations (reference for expected output)
- `crates/memory-installer/src/converters/claude.rs` — Claude converter (YAML frontmatter, .claude/ paths)
- `crates/memory-installer/src/converters/codex.rs` — Codex converter (AGENTS.md + skill dirs, .codex/ paths)
- `crates/memory-installer/src/converters/gemini.rs` — Gemini converter (TOML frontmatter, GEMINI.md, hooks)
- `crates/memory-installer/src/converters/copilot.rs` — Copilot converter (.github/ paths, camelCase hooks)
- `crates/memory-installer/src/converters/opencode.rs` — OpenCode converter (AGENTS.md, .opencode/ paths)
- `crates/memory-installer/src/converters/skills.rs` — Skills converter (generic skill dirs, no tool remapping)

### Infrastructure
- `crates/memory-installer/src/converter.rs` — RuntimeConverter trait and convert_bundle orchestration
- `crates/memory-installer/src/types.rs` — PluginBundle, ConvertedFile, InstallScope, Runtime types
- `crates/memory-installer/src/tool_maps.rs` — Per-runtime tool name mappings

### Old Adapters (to be archived)
- `plugins/memory-copilot-adapter/` — Old Copilot adapter (replaced by CopilotConverter)
- `plugins/memory-gemini-adapter/` — Old Gemini adapter (replaced by GeminiConverter)
- `plugins/memory-opencode-plugin/` — Old OpenCode plugin (replaced by OpenCodeConverter)

### CI
- `.github/workflows/ci.yml` — Workspace CI (already includes memory-installer via --workspace)

</canonical_refs>

<specifics>
## Specific Ideas

- Phase 49 execution produced 104 passing unit tests in memory-installer — E2E tests should complement these with full-bundle conversion tests
- CopilotConverter uses `include_str!` for hook scripts from `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` — archival must preserve this file or the converter will break at compile time
- GeminiConverter produces TOML frontmatter (not YAML) — E2E tests should verify this format difference
- SkillsConverter uses pass-through tool names (no remapping) — E2E should verify tools are NOT remapped
- The `convert_bundle` function in `converter.rs` orchestrates all converters — E2E tests can call this directly

</specifics>

<deferred>
## Deferred Ideas

- MIG-F01: Delete archived adapter directories after one release cycle (v2.8+)
- INST-F01: Interactive mode with runtime selection prompts
- INST-F02: `--uninstall` command
- INST-F03: `--all` flag for all runtimes
- INST-F04: Version tracking with upgrade detection

</deferred>

---

*Phase: 50-integration-testing-migration*
*Context gathered: 2026-03-18 via conversation context*
