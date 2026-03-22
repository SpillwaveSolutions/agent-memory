# Phase 45: Canonical Source Consolidation - Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Prepare the canonical plugin source for the installer. The installer will read from the existing `memory-query-plugin/` and `memory-setup-plugin/` directories — no merge needed. Hook canonicalization is deferred to Phase 49. Hand-written adapters remain in place until Phase 50 verifies installer output.

</domain>

<decisions>
## Implementation Decisions

### Plugin structure
- Keep `memory-query-plugin/` and `memory-setup-plugin/` as separate directories
- Installer reads from both — no merge into single `plugins/memory-plugin/`
- Zero migration risk — existing plugins continue working as Claude plugins
- Installer parser must support reading from multiple plugin source directories

### Hook handling
- Skip hook canonicalization in Phase 45 — that's Phase 49 work
- Phase 45 only consolidates commands, agents, and skills
- Existing hook implementations stay in the adapter directories

### Adapter retirement
- Leave hand-written adapters (copilot, gemini, opencode) in place
- Don't touch them until Phase 50 verifies installer produces equivalent output
- Then archive with README stubs pointing to the installer

### Claude's Discretion
- Whether to add a manifest file listing both plugin source directories for the installer
- Plugin.json format for the consolidated canonical reference
- Any cleanup of SKILL.md files or reference docs during consolidation

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `plugins/memory-query-plugin/.claude-plugin/plugin.json` — existing manifest format
- `plugins/memory-setup-plugin/.claude-plugin/plugin.json` — existing manifest format
- All 6 commands already have YAML frontmatter in Claude format
- All 13 skills have SKILL.md with consistent structure

### Established Patterns
- Commands use `---` YAML frontmatter with `name`, `description` fields
- Skills use SKILL.md with `name`, `description`, `version` frontmatter
- Agents use YAML frontmatter with `description`, `allowed-tools`, `color`
- Reference docs live in `references/` subdirs under each skill

### Integration Points
- Phase 46 parser must discover these files by walking plugin directories
- Phase 47-49 converters read the parsed output from Phase 46

</code_context>

<specifics>
## Specific Ideas

- The installer reads from both plugin dirs, treating them as equal sources
- No need for a single merged directory — the parser handles multi-source
- This simplifies Phase 45 to just ensuring the canonical source is clean and well-structured

</specifics>

<deferred>
## Deferred Ideas

- Hook canonicalization — Phase 49
- Adapter archival — Phase 50
- Single merged plugin directory — not needed (installer handles multi-source)

</deferred>

---

*Phase: 45-canonical-source-consolidation*
*Context gathered: 2026-03-16*
