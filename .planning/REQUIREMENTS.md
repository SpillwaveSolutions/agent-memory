# Requirements: Agent Memory v3.2

**Defined:** 2026-03-25
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v3.2 Requirements

Requirements for the Plugin Installer & OpenCode Converter milestone.

### OpenCode Converter (OC)

- [x] **OC-01**: Commands flattened from `commands/memory-search.md` to `command/memory-search.md`
- [x] **OC-02**: Agent frontmatter converts `allowed-tools:` array to `tools:` object with `tool: true` entries
- [x] **OC-03**: Tool names converted to lowercase with special mappings (AskUserQuestion→question, etc.)
- [x] **OC-04**: Color names normalized to hex values
- [x] **OC-05**: Paths rewritten from `~/.claude/` to `~/.config/opencode/`
- [x] **OC-06**: Auto-configure `opencode.json` read permissions for installed skill paths

### Claude Code Registration (CREG)

- [x] **CREG-01**: `memory-installer install --agent claude` writes `known_marketplaces.json` with git marketplace entry
- [x] **CREG-02**: `memory-installer install --agent claude` writes `installed_plugins.json` with versioned plugin entry
- [x] **CREG-03**: `memory-installer install --agent claude` writes `settings.json` with `enabledPlugins` entry
- [x] **CREG-04**: Plugin key format follows `{plugin-name}@{marketplace-id}` convention (e.g., `memory-query@agent-memory`)
- [x] **CREG-05**: Version read from `.claude-plugin/plugin.json` — install path includes version directory
- [x] **CREG-06**: Re-install updates in place (idempotent); old version directories cleaned up

### OpenCode Registration (OREG)

- [x] **OREG-01**: `memory-installer install --agent opencode` writes `opencode.json` with read permissions for installed paths
- [x] **OREG-02**: Permission entries use glob patterns matching installed skill/command directories
- [x] **OREG-03**: Existing `opencode.json` content preserved (merge, not overwrite)

### Uninstall (UNINST)

- [ ] **UNINST-01**: `memory-installer uninstall --agent claude` removes plugin from all 3 registry files and deletes installed files
- [ ] **UNINST-02**: `memory-installer uninstall --agent opencode` removes permission entries and deletes installed files
- [ ] **UNINST-03**: Uninstall is safe to call when not installed (no-op, no error)

### Status (STAT)

- [ ] **STAT-01**: `memory-installer status` shows installed runtimes, versions, and paths
- [ ] **STAT-02**: Status reports "not installed" for runtimes without registration

### Plugin Metadata (META)

- [x] **META-01**: `.claude-plugin/plugin.json` exists with name, version, description
- [x] **META-02**: `.claude-plugin/marketplace.json` exists with marketplace registration metadata
- [x] **META-03**: Version in plugin.json is the single source of truth for install path versioning

## Future Requirements (v3.3+)

- **CREG-F01**: `--for all` flag to install for all supported runtimes at once
- **UNINST-F01**: `--all` flag to uninstall from all runtimes
- **REG-F01**: Gemini, Codex, Copilot registration patterns (currently convert-only)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Python wrapper installer | Using Rust memory-installer directly (option 1) |
| Gemini/Codex/Copilot registration | Convert-only for now; registration deferred |
| Plugin marketplace publishing | Separate from local installation |
| Automatic update checking | Future enhancement |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| OC-01 | Phase 57 | Complete |
| OC-02 | Phase 57 | Complete |
| OC-03 | Phase 57 | Complete |
| OC-04 | Phase 57 | Complete |
| OC-05 | Phase 57 | Complete |
| OC-06 | Phase 57 | Complete |
| CREG-01 | Phase 58 | Complete |
| CREG-02 | Phase 58 | Complete |
| CREG-03 | Phase 58 | Complete |
| CREG-04 | Phase 58 | Complete |
| CREG-05 | Phase 58 | Complete |
| CREG-06 | Phase 58 | Complete |
| OREG-01 | Phase 57 | Complete |
| OREG-02 | Phase 57 | Complete |
| OREG-03 | Phase 57 | Complete |
| UNINST-01 | Phase 59 | Pending |
| UNINST-02 | Phase 59 | Pending |
| UNINST-03 | Phase 59 | Pending |
| STAT-01 | Phase 59 | Pending |
| STAT-02 | Phase 59 | Pending |
| META-01 | Phase 58 | Complete |
| META-02 | Phase 58 | Complete |
| META-03 | Phase 58 | Complete |

**Coverage:**
- v3.2 requirements: 23 total
- Mapped to phases: 23
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-25*
