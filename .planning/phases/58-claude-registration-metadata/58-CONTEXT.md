# Phase 58: Claude Code Registration + Plugin Metadata - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning
**Source:** Codebase-mentor reference implementation analysis

<domain>
## Phase Boundary

This phase adds runtime registration to the existing Claude converter. After `memory-installer install --agent claude` converts files, it now also registers the plugin with Claude Code's discovery system by writing 3 JSON registry files. Also creates `.claude-plugin/plugin.json` and `marketplace.json` metadata files. The existing `ClaudeConverter` file conversion is unchanged — this adds the registration step on top.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- Extend `ClaudeConverter::generate_guidance()` to emit the 3 registry files as `ConvertedFile` entries
- OR add a new post-install registration step in `main.rs` after `write_files()` completes
- Plugin metadata files (`.claude-plugin/plugin.json`, `marketplace.json`) are part of the canonical plugin source — they exist in the source tree already

### Three Registry Files (Claude Code)

**1. `~/.claude/known_marketplaces.json`**
```json
{
  "agent-memory": {
    "source": {"source": "git", "url": "https://github.com/SpillwaveSolutions/agent-memory.git"},
    "installLocation": "~/.claude/plugins/marketplaces/agent-memory",
    "lastUpdated": "ISO-8601 timestamp"
  }
}
```

**2. `~/.claude/installed_plugins.json`**
```json
{
  "version": 2,
  "plugins": {
    "memory-query@agent-memory": [
      {
        "scope": "user",
        "installPath": "~/.claude/plugins/cache/agent-memory/memory-query/1.0.0",
        "version": "1.0.0",
        "installedAt": "ISO-8601 timestamp",
        "lastUpdated": "ISO-8601 timestamp"
      }
    ]
  }
}
```

**3. `~/.claude/settings.json`**
```json
{
  "enabledPlugins": {
    "memory-query@agent-memory": true
  }
}
```

### Plugin Key Format (CREG-04)
- Format: `{plugin-name}@{marketplace-id}`
- Example: `memory-query@agent-memory`
- `plugin-name` comes from `.claude-plugin/plugin.json` `name` field
- `marketplace-id` is the marketplace directory name (e.g., `agent-memory`)

### Version Management (CREG-05)
- Version read from `.claude-plugin/plugin.json` (single source of truth)
- Install path includes version: `~/.claude/plugins/cache/agent-memory/memory-query/1.0.0/`
- On re-install, old version directories cleaned up (CREG-06)

### JSON Merge Strategy
- All 3 registry files must MERGE with existing content (not overwrite)
- `known_marketplaces.json`: upsert marketplace entry by key
- `installed_plugins.json`: upsert plugin entry in `plugins` map; clean up old versions
- `settings.json`: set `enabledPlugins.{key}` to `true`; preserve all other settings

### Plugin Metadata Files (META-01..03)
- `.claude-plugin/plugin.json`: name, version, description, author, license
- `.claude-plugin/marketplace.json`: marketplace listing with plugins array
- These already exist in the canonical plugin source (`memory-query-plugin/.claude-plugin/`)
- The installer copies them as-is (Claude converter does pass-through)
- Version in `plugin.json` is the single source of truth for install path versioning

### Reference Implementation
- `/Users/richardhightower/clients/spillwave/src/codebase-mentor/ai_codebase_mentor/converters/claude.py`
- Key patterns: registry file read-merge-write, version extraction, install path construction, old version cleanup

### Claude's Discretion
- Whether registration happens in `generate_guidance()` or as a post-install step
- Exact error handling for corrupt/missing registry files (create fresh vs fail)
- Whether to also handle `memory-setup-plugin` or just `memory-query-plugin`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Reference Implementation (Python)
- `/Users/richardhightower/clients/spillwave/src/codebase-mentor/ai_codebase_mentor/converters/claude.py` — Python reference for all 3 registry file writes

### Existing Claude Converter
- `crates/memory-installer/src/converters/claude.rs` — existing pass-through converter (works, just no registration)

### Existing Plugin Metadata
- `memory-query-plugin/.claude-plugin/plugin.json` — canonical plugin metadata (version source of truth)
- `memory-query-plugin/.claude-plugin/marketplace.json` — marketplace listing

### Installer Infrastructure
- `crates/memory-installer/src/main.rs` — install pipeline (parse → convert → write)
- `crates/memory-installer/src/types.rs` — ConvertedFile, InstallConfig, InstallScope
- `crates/memory-installer/src/writer.rs` — write_files() function

</canonical_refs>

<specifics>
## Specific Ideas

- The `generate_guidance()` method on `ClaudeConverter` currently returns empty Vec — this is where registration files can be emitted
- Alternatively, a new `register()` trait method could be added to `RuntimeConverter`
- The codebase-mentor Python code handles all edge cases (missing files, version cleanup, idempotent merge)
- Registry files live in `~/.claude/` (global scope) regardless of `--project` vs `--global` flag

</specifics>

<deferred>
## Deferred Ideas

- `memory-setup-plugin` registration (just do `memory-query-plugin` for now)
- Plugin marketplace publishing
- Automatic update checking
- `--for all` flag

</deferred>

---

*Phase: 58-claude-registration-metadata*
*Context gathered: 2026-03-25*
