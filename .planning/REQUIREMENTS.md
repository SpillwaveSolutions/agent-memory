# Requirements: Agent Memory v2.7

**Defined:** 2026-03-16
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v2.7 Requirements

Requirements for the Multi-Runtime Portability milestone. Each maps to roadmap phases.

### Canonical Source (CANON)

- [x] **CANON-01**: Canonical plugin source comprises both `memory-query-plugin/` and `memory-setup-plugin/` directories (reinterpreted: installer reads from both dirs, no merge per Phase 45 CONTEXT.md decision)
- [ ] **CANON-02**: Canonical hook definitions in YAML format capture all event types across runtimes *(deferred to Phase 49 per Phase 45 CONTEXT.md decision)*
- [x] **CANON-03**: All 6 commands, 2 agents, 13 skills consolidated with no content loss

### Installer Infrastructure (INST)

- [x] **INST-01**: Standalone `memory-installer` binary with clap CLI accepting `--agent <runtime>`, `--project`/`--global`, `--dir <path>`, `--dry-run`
- [x] **INST-02**: Plugin parser extracts commands, agents, skills with YAML frontmatter from canonical source directory
- [x] **INST-03**: `RuntimeConverter` trait with `convert_command`, `convert_agent`, `convert_skill`, `convert_hook`, `generate_guidance`, `target_dir` methods
- [x] **INST-04**: Centralized tool mapping tables in `tool_maps.rs` covering all 11 tool names across 6 runtimes
- [x] **INST-05**: Managed-section markers in shared config files enabling safe merge, upgrade, and uninstall
- [x] **INST-06**: `--dry-run` mode shows what would be installed without writing files
- [x] **INST-07**: Unmapped tool names produce warnings (not silent drops)

### Claude Converter (CLAUDE)

- [x] **CLAUDE-01**: Claude converter copies canonical source with minimal transformation (path rewriting only)
- [x] **CLAUDE-02**: Storage paths rewritten to runtime-neutral `~/.config/agent-memory/`

### OpenCode Converter (OC)

- [ ] **OC-01**: Commands flattened from `commands/memory-search.md` to `command/memory-search.md`
- [ ] **OC-02**: Agent frontmatter converts `allowed-tools:` array to `tools:` object with `tool: true` entries
- [ ] **OC-03**: Tool names converted to lowercase with special mappings (AskUserQuestion→question, etc.)
- [ ] **OC-04**: Color names normalized to hex values
- [ ] **OC-05**: Paths rewritten from `~/.claude/` to `~/.config/opencode/`
- [ ] **OC-06**: Auto-configure `opencode.json` read permissions for installed skill paths

### Gemini Converter (GEM)

- [x] **GEM-01**: Command frontmatter converted from YAML to TOML format
- [x] **GEM-02**: Agent `allowed-tools:` converted to `tools:` array with Gemini snake_case names
- [x] **GEM-03**: MCP and Task tools excluded from converted output (auto-discovered by Gemini)
- [x] **GEM-04**: `color:` and `skills:` fields stripped from agent frontmatter
- [x] **GEM-05**: Shell variable `${VAR}` escaped to `$VAR` (Gemini template engine conflict)
- [x] **GEM-06**: Hook definitions merged into `.gemini/settings.json` using managed-section markers

### Codex Converter (CDX)

- [x] **CDX-01**: Commands converted to Codex skill directories (each command becomes a SKILL.md)
- [x] **CDX-02**: Agents converted to orchestration skill directories
- [x] **CDX-03**: `AGENTS.md` generated from agent metadata for project-level Codex guidance
- [x] **CDX-04**: Sandbox permissions mapped per agent (workspace-write vs read-only)

### Copilot Converter (COP)

- [x] **COP-01**: Commands converted to Copilot skill format under `.github/skills/`
- [x] **COP-02**: Agents converted to `.agent.md` format with Copilot tool names
- [x] **COP-03**: Hook definitions converted to `.github/hooks/` JSON format with shell scripts

### Generic Skills Converter (SKL)

- [x] **SKL-01**: `--agent skills --dir <path>` installs to user-specified directory
- [x] **SKL-02**: Commands become skill directories, agents become orchestration skills
- [x] **SKL-03**: No runtime-specific transforms beyond path rewriting

### Hook Conversion (HOOK)

- [x] **HOOK-01**: Canonical YAML hook definitions converted to per-runtime formats
- [x] **HOOK-02**: Hook event names mapped correctly per runtime (PascalCase/camelCase differences)
- [x] **HOOK-03**: Hook scripts generated with fail-open behavior and background execution

### Testing & Migration (MIG)

- [x] **MIG-01**: E2E tests verify install-to-temp-dir produces correct file structure per runtime
- [x] **MIG-02**: E2E tests verify frontmatter conversion correctness (tool names, format, fields)
- [ ] **MIG-03**: Old adapter directories archived with README stubs pointing to `memory-installer`
- [x] **MIG-04**: Installer added to workspace CI (build, clippy, test)

## Future Requirements (v2.8+)

- **MIG-F01**: Delete archived adapter directories after one release cycle
- **INST-F01**: Interactive mode with runtime selection prompts
- **INST-F02**: `--uninstall` command to remove installed files using managed markers
- **INST-F03**: `--all` flag to install all runtimes at once
- **INST-F04**: Version tracking with upgrade detection

## Out of Scope

| Feature | Reason |
|---------|--------|
| Interactive prompts for MVP | Breaks CI and agent-driven workflows; add post-MVP |
| Two-way sync (runtime→canonical) | One-way conversion is simpler and matches GSD pattern |
| Plugin marketplace integration | Claude marketplace is separate from installer |
| Hook format unification | Each runtime's hook mechanism is too different; convert per-runtime |
| Windows PowerShell hooks | Shell scripts with WSL sufficient for MVP; PS1 hooks deferred |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CANON-01 | Phase 45 | Complete |
| CANON-02 | Phase 49 | Pending |
| CANON-03 | Phase 45 | Complete |
| INST-01 | Phase 46 | Complete |
| INST-02 | Phase 46 | Complete |
| INST-03 | Phase 46 | Complete |
| INST-04 | Phase 46 | Complete |
| INST-05 | Phase 46 | Complete |
| INST-06 | Phase 46 | Complete |
| INST-07 | Phase 46 | Complete |
| CLAUDE-01 | Phase 47 | Complete |
| CLAUDE-02 | Phase 47 | Complete |
| OC-01 | Phase 47 | Pending |
| OC-02 | Phase 47 | Pending |
| OC-03 | Phase 47 | Pending |
| OC-04 | Phase 47 | Pending |
| OC-05 | Phase 47 | Pending |
| OC-06 | Phase 47 | Pending |
| GEM-01 | Phase 48 | Complete |
| GEM-02 | Phase 48 | Complete |
| GEM-03 | Phase 48 | Complete |
| GEM-04 | Phase 48 | Complete |
| GEM-05 | Phase 48 | Complete |
| GEM-06 | Phase 48 | Complete |
| CDX-01 | Phase 48 | Complete |
| CDX-02 | Phase 48 | Complete |
| CDX-03 | Phase 48 | Complete |
| CDX-04 | Phase 48 | Complete |
| COP-01 | Phase 49 | Complete |
| COP-02 | Phase 49 | Complete |
| COP-03 | Phase 49 | Complete |
| SKL-01 | Phase 49 | Complete |
| SKL-02 | Phase 49 | Complete |
| SKL-03 | Phase 49 | Complete |
| HOOK-01 | Phase 49 | Complete |
| HOOK-02 | Phase 49 | Complete |
| HOOK-03 | Phase 49 | Complete |
| MIG-01 | Phase 50 | Complete |
| MIG-02 | Phase 50 | Complete |
| MIG-03 | Phase 50 | Pending |
| MIG-04 | Phase 50 | Complete |

**Coverage:**
- v2.7 requirements: 41 total
- Mapped to phases: 41
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-16*
*Last updated: 2026-03-16 after research synthesis*
