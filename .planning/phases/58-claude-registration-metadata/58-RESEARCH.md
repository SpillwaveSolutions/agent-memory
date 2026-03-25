# Phase 58: Claude Code Registration + Plugin Metadata - Research

**Researched:** 2026-03-25
**Domain:** Claude Code plugin registry system, JSON merge operations, plugin metadata
**Confidence:** HIGH

## Summary

Phase 58 adds Claude Code runtime registration to the existing `ClaudeConverter`. Currently the converter does pass-through file conversion (commands, agents, skills with path rewriting) but does not register the plugin with Claude Code's discovery system. The phase requires writing 3 JSON registry files (`known_marketplaces.json`, `installed_plugins.json`, `settings.json`) and creating the missing `plugin.json` metadata file.

The reference implementation exists in codebase-mentor's Python `claude.py` and handles all edge cases (missing files, version cleanup, idempotent merge). The Rust implementation follows the same JSON structure and merge strategy. The OpenCode converter's `generate_guidance()` method already demonstrates the pattern of reading existing JSON, merging, and emitting as `ConvertedFile` -- but the Claude registration is more complex because it writes to **global** `~/.claude/` paths regardless of install scope, and must handle 3 separate files with different merge strategies.

**Primary recommendation:** Add a `register()` method to `ClaudeConverter` (not the trait) called from `generate_guidance()`, which reads/merges/emits the 3 registry JSON files. Create `plugin.json` as a new static file in the canonical plugin source.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Extend `ClaudeConverter::generate_guidance()` to emit the 3 registry files as `ConvertedFile` entries OR add a new post-install registration step in `main.rs` after `write_files()` completes
- Plugin metadata files (`.claude-plugin/plugin.json`, `marketplace.json`) are part of the canonical plugin source
- Three registry files: `known_marketplaces.json`, `installed_plugins.json`, `settings.json` with exact JSON structures specified in CONTEXT.md
- Plugin key format: `{plugin-name}@{marketplace-id}` (e.g., `memory-query@agent-memory`)
- Version read from `.claude-plugin/plugin.json` (single source of truth)
- Install path includes version: `~/.claude/plugins/cache/agent-memory/memory-query/1.0.0/`
- All 3 registry files must MERGE with existing content (not overwrite)
- JSON merge strategies: upsert marketplace by key, upsert plugin entry, set enabledPlugins to true preserving other settings
- Only `memory-query-plugin` for now (not `memory-setup-plugin`)

### Claude's Discretion
- Whether registration happens in `generate_guidance()` or as a post-install step
- Exact error handling for corrupt/missing registry files (create fresh vs fail)
- Whether to also handle `memory-setup-plugin` or just `memory-query-plugin`

### Deferred Ideas (OUT OF SCOPE)
- `memory-setup-plugin` registration (just do `memory-query-plugin` for now)
- Plugin marketplace publishing
- Automatic update checking
- `--for all` flag
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CREG-01 | Write `known_marketplaces.json` with git marketplace entry | Python reference shows exact JSON structure; merge = upsert by marketplace ID key |
| CREG-02 | Write `installed_plugins.json` with versioned plugin entry | Python reference shows version:2 format, plugin array per key, preserves installedAt on update |
| CREG-03 | Write `settings.json` with `enabledPlugins` entry | Python reference shows simple key=true insert into enabledPlugins map |
| CREG-04 | Plugin key format `{plugin-name}@{marketplace-id}` | Key is `memory-query@agent-memory`; plugin-name from plugin.json name field |
| CREG-05 | Version from plugin.json, install path includes version dir | Need to create plugin.json first (META-01); version extracted via serde |
| CREG-06 | Re-install idempotent; old version dirs cleaned up | Python reference iterates sibling dirs under cache/agent-memory/memory-query/, removes non-current |
| META-01 | `.claude-plugin/plugin.json` exists with name, version, description | Must CREATE this file -- it does not exist yet. Reference: codebase-mentor's plugin.json |
| META-02 | `.claude-plugin/marketplace.json` exists with marketplace metadata | Already exists at `plugins/memory-query-plugin/.claude-plugin/marketplace.json` -- DONE |
| META-03 | Version in plugin.json is single source of truth for install path versioning | Installer reads plugin.json at install time to determine cache path version component |
</phase_requirements>

## Standard Stack

### Core (already in workspace)
| Library | Purpose | Why Standard |
|---------|---------|--------------|
| `serde_json` | JSON parse/serialize for all 3 registry files | Already a workspace dep; `Value` API handles dynamic merge |
| `directories` | Resolve `~/.claude/` home path cross-platform | Already used by `ClaudeConverter::target_dir()` |
| `anyhow` | Error handling for file I/O | Already used throughout writer.rs |
| `chrono` | ISO-8601 timestamp generation for `lastUpdated`/`installedAt` | **NEW** -- need to add to workspace deps |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `chrono` for timestamps | `time` crate | `chrono` is more common; `time` is lighter but less ergonomic for ISO-8601 |
| `chrono` for timestamps | `std::time` + manual format | Error-prone; ISO-8601 with timezone offset is tricky to format manually |

**New dependency note:** `chrono` is needed for `Utc::now().to_rfc3339()` to produce ISO-8601 timestamps matching the Python reference (`datetime.now(timezone.utc).isoformat()`). Check if `chrono` is already in the workspace before adding.

**Version verification:**
```bash
# Verify chrono is needed (check if already in workspace)
cargo metadata --format-version 1 | grep chrono
# If not present, add with: cargo add chrono --features serde
```

## Architecture Patterns

### Registration Architecture Decision

**Recommendation: Use `generate_guidance()` (not a new trait method or post-install step).**

Rationale:
1. `generate_guidance()` already returns `Vec<ConvertedFile>` -- the registry files are just more `ConvertedFile` entries
2. The OpenCode converter already uses this pattern for `opencode.json` merge (read existing, merge, emit)
3. No trait changes needed -- other converters unaffected
4. The `write_files()` function handles directory creation and dry-run for free
5. Registration files flow through the same pipeline, getting dry-run support automatically

**However, there is a critical difference:** The 3 registry files live in `~/.claude/` (global), NOT in the converter's `target_dir()`. The OpenCode converter's `generate_guidance()` reads the existing file at `target_dir().join("opencode.json")` -- but the Claude registry files are always at `~/.claude/plugins/known_marketplaces.json` etc., regardless of whether `--project` or `--global` was passed.

**Design: Registry files are always written to `~/.claude/` paths.** The `generate_guidance()` method constructs absolute paths to `~/.claude/` for registry `ConvertedFile` entries. For `--project` scope, skip registration (project installs don't register globally).

### Key Implementation Pattern: Read-Merge-Write

All 3 registry files follow the same pattern (from Python reference):

```rust
fn merge_registry_json(path: &Path, mutate: impl FnOnce(&mut serde_json::Value)) -> ConvertedFile {
    // 1. Read existing file (or start with empty object)
    let mut data = if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 2. Apply mutation (upsert)
    mutate(&mut data);

    // 3. Return as ConvertedFile (write_files handles actual write)
    ConvertedFile {
        target_path: path.to_path_buf(),
        content: serde_json::to_string_pretty(&data).unwrap() + "\n",
    }
}
```

### Plugin.json Structure (to create)

Based on codebase-mentor reference:

```json
{
  "name": "memory-query",
  "version": "1.0.0",
  "description": "Intelligent memory retrieval with tier-aware routing, intent classification, and fallback chains",
  "keywords": ["memory", "retrieval", "agent-memory", "search", "context"],
  "author": {
    "name": "Spillwave Solutions",
    "url": "https://github.com/SpillwaveSolutions"
  },
  "license": "MIT",
  "homepage": "https://github.com/SpillwaveSolutions/agent-memory",
  "repository": "https://github.com/SpillwaveSolutions/agent-memory"
}
```

### Three Registry File Structures

**1. `~/.claude/plugins/known_marketplaces.json`**
```json
{
  "agent-memory": {
    "source": {"source": "git", "url": "https://github.com/SpillwaveSolutions/agent-memory.git"},
    "installLocation": "/Users/username/.claude/plugins/marketplaces/agent-memory",
    "lastUpdated": "2026-03-25T12:00:00+00:00"
  }
}
```
- Merge: upsert by marketplace ID key (`agent-memory`)
- `installLocation` uses absolute expanded path (not `~`)

**2. `~/.claude/plugins/installed_plugins.json`**
```json
{
  "version": 2,
  "plugins": {
    "memory-query@agent-memory": [
      {
        "scope": "user",
        "installPath": "/Users/username/.claude/plugins/cache/agent-memory/memory-query/1.0.0",
        "version": "1.0.0",
        "installedAt": "2026-03-25T12:00:00+00:00",
        "lastUpdated": "2026-03-25T12:00:00+00:00"
      }
    ]
  }
}
```
- Merge: upsert by plugin key; preserve `installedAt` from existing entry on update
- `installPath` uses absolute expanded path

**3. `~/.claude/settings.json`**
```json
{
  "enabledPlugins": {
    "memory-query@agent-memory": true
  }
}
```
- Merge: set `enabledPlugins.{key} = true`; preserve ALL other settings keys

### Recommended Project Structure

```
crates/memory-installer/src/
  converters/
    claude.rs          # Extended with registration logic
  types.rs             # No changes needed
  writer.rs            # No changes needed (write_files handles ConvertedFile)
  main.rs              # No changes needed (generate_guidance already called)

plugins/memory-query-plugin/
  .claude-plugin/
    plugin.json        # NEW (META-01)
    marketplace.json   # EXISTS (META-02 -- already done)
```

### Anti-Patterns to Avoid
- **Direct file writes in converter:** Do NOT bypass `write_files()` by writing directly in `generate_guidance()`. Emit as `ConvertedFile` to get dry-run support.
- **Overwriting registry files:** Always read-merge-write. Never truncate existing settings.json.
- **Hardcoded home path:** Use `directories::BaseDirs` for cross-platform home resolution (already used in `target_dir()`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ISO-8601 timestamps | Manual string formatting | `chrono::Utc::now().to_rfc3339()` | Timezone/offset formatting is error-prone |
| Home directory resolution | `env::var("HOME")` | `directories::BaseDirs::new()` | Cross-platform (already in crate) |
| JSON pretty-printing | Manual indentation | `serde_json::to_string_pretty()` | Already used in OpenCode converter |
| Path expansion (~) | String replacement | `shellexpand::tilde()` | Already a dependency |

**Key insight:** The read-merge-write pattern is deceptively complex because of concurrent access (another tool might modify settings.json) and corrupt file recovery. The Python reference handles this with try/except and fallback to empty dict -- replicate that in Rust with `.ok().unwrap_or_default()`.

## Common Pitfalls

### Pitfall 1: Registry Files at Wrong Path
**What goes wrong:** Writing registry files relative to `target_dir()` instead of `~/.claude/`
**Why it happens:** Other `ConvertedFile` entries use `target_dir()` for their paths
**How to avoid:** Registry file paths are ALWAYS absolute `~/.claude/plugins/known_marketplaces.json` etc. -- hardcode the `~/.claude/` prefix via `BaseDirs::home_dir()`
**Warning signs:** Registry files appearing inside `~/.claude/plugins/memory-plugin/` instead of `~/.claude/plugins/`

### Pitfall 2: installPath Uses Tilde Instead of Absolute Path
**What goes wrong:** Writing `~/.claude/plugins/cache/...` as a string in JSON instead of expanding to `/Users/username/.claude/plugins/cache/...`
**Why it happens:** Copy-pasting from docs that use `~` shorthand
**How to avoid:** The Python reference uses `str(Path.home() / ".claude" / ...)` which produces absolute paths. Do the same in Rust.
**Warning signs:** Claude Code can't find the plugin because it doesn't expand `~` in JSON values

### Pitfall 3: Destroying settings.json Content
**What goes wrong:** Overwriting `settings.json` with only `enabledPlugins`, losing user's other settings (theme, model preferences, etc.)
**Why it happens:** Creating a new JSON object instead of merging into existing
**How to avoid:** Always read existing file first, merge only the `enabledPlugins` key
**Warning signs:** User reports lost settings after install

### Pitfall 4: Missing `installedAt` Preservation on Re-install
**What goes wrong:** `installedAt` timestamp changes on every re-install
**Why it happens:** Not checking for existing entry before writing
**How to avoid:** Python reference preserves `installedAt` from existing entry: `existing[0].get("installedAt", now)`
**Warning signs:** `installedAt` always equals `lastUpdated`

### Pitfall 5: Old Version Directories Not Cleaned Up (CREG-06)
**What goes wrong:** Multiple version directories accumulate under `cache/agent-memory/memory-query/`
**Why it happens:** Only writing new version without checking siblings
**How to avoid:** Before writing new version, iterate sibling directories under the plugin cache dir and remove any that don't match current version
**Warning signs:** Multiple version directories (e.g., `1.0.0/`, `1.0.1/`, `1.1.0/`) under the plugin cache

### Pitfall 6: Project Scope Shouldn't Register Globally
**What goes wrong:** `--project` install writes to `~/.claude/settings.json`
**Why it happens:** Not gating registration on scope
**How to avoid:** Only perform registry writes for `InstallScope::Global`. The Python reference only calls `_register_plugin()` when `target == "global"`.

## Code Examples

### Reading Version from plugin.json

```rust
/// Read version from .claude-plugin/plugin.json in the source tree.
fn read_plugin_version(source_root: &Path) -> Option<String> {
    // source_root points to plugins/ dir; plugin.json is at
    // plugins/memory-query-plugin/.claude-plugin/plugin.json
    let manifest = source_root
        .join("memory-query-plugin")
        .join(".claude-plugin")
        .join("plugin.json");

    let content = std::fs::read_to_string(&manifest).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    data.get("version")?.as_str().map(|s| s.to_string())
}
```

### Building the Plugin Registry Key

```rust
const MARKETPLACE_ID: &str = "agent-memory";
const PLUGIN_NAME: &str = "memory-query";
const PLUGIN_REGISTRY_KEY: &str = "memory-query@agent-memory";
const MARKETPLACE_GIT_URL: &str = "https://github.com/SpillwaveSolutions/agent-memory.git";
```

### Known Marketplaces Merge

```rust
fn build_known_marketplaces(home: &Path, now: &str) -> ConvertedFile {
    let path = home.join(".claude/plugins/known_marketplaces.json");
    let mut data = read_json_or_empty(&path);

    let obj = data.as_object_mut().unwrap();
    obj.insert(MARKETPLACE_ID.to_string(), json!({
        "source": {"source": "git", "url": MARKETPLACE_GIT_URL},
        "installLocation": home.join(".claude/plugins/marketplaces").join(MARKETPLACE_ID)
            .to_string_lossy().to_string(),
        "lastUpdated": now,
    }));

    ConvertedFile {
        target_path: path,
        content: serde_json::to_string_pretty(&data).unwrap() + "\n",
    }
}
```

### Old Version Cleanup

```rust
/// Remove stale version directories under the plugin cache dir.
fn cleanup_old_versions(cache_plugin_dir: &Path, current_version: &str) {
    if !cache_plugin_dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(cache_plugin_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if name != current_version {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }
}
```

**Note on cleanup timing:** Cleanup must happen BEFORE `write_files()` writes the new version, or after. Since `generate_guidance()` runs before `write_files()`, cleanup could happen inside `generate_guidance()` as a side effect, OR be moved to a separate post-write step. The cleaner approach is to perform cleanup inside `generate_guidance()` since it already reads the filesystem.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual plugin install | Registry-based auto-discovery | Claude Code plugin system (2025) | 3 JSON files required for Claude to find plugins |
| No metadata files | `.claude-plugin/plugin.json` | Claude Code plugin format | Version, name, description needed |

**Current status:**
- `marketplace.json` already exists for both plugins
- `plugin.json` does NOT exist yet -- must be created (META-01)
- Claude converter does file conversion but no registration
- OpenCode converter already demonstrates the `generate_guidance()` merge pattern

## Open Questions

1. **chrono dependency**
   - What we know: ISO-8601 timestamps are required for `lastUpdated` and `installedAt`
   - What's unclear: Whether `chrono` is already in the workspace Cargo.toml
   - Recommendation: Check workspace deps; if not present, add `chrono` with `serde` feature. Alternative: use `time` crate if already present.

2. **Cleanup timing for old versions**
   - What we know: Python reference removes old versions before `copytree`
   - What's unclear: Whether to do cleanup inside `generate_guidance()` (side effect) or add a pre/post step
   - Recommendation: Perform cleanup inside `generate_guidance()` since it already reads filesystem state. This is a side effect but matches the pattern of the Python reference and keeps registration self-contained.

3. **`--project` scope behavior**
   - What we know: Python reference only registers for global installs
   - What's unclear: Should `--project` install still copy plugin files but skip registration?
   - Recommendation: Yes -- `--project` installs files to `./.claude/plugins/memory-plugin/` (existing behavior) but does NOT write the 3 registry files. Only `--global` triggers registration.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in) |
| Config file | Cargo.toml workspace |
| Quick run command | `cargo test -p memory-installer` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CREG-01 | known_marketplaces.json written with correct structure | unit | `cargo test -p memory-installer -- creg01` | Wave 0 |
| CREG-02 | installed_plugins.json written with versioned entry | unit | `cargo test -p memory-installer -- creg02` | Wave 0 |
| CREG-03 | settings.json written with enabledPlugins entry | unit | `cargo test -p memory-installer -- creg03` | Wave 0 |
| CREG-04 | Plugin key format memory-query@agent-memory | unit | `cargo test -p memory-installer -- creg04` | Wave 0 |
| CREG-05 | Version from plugin.json used in install path | unit | `cargo test -p memory-installer -- creg05` | Wave 0 |
| CREG-06 | Re-install cleans old version dirs | unit | `cargo test -p memory-installer -- creg06` | Wave 0 |
| META-01 | plugin.json exists with required fields | unit | `cargo test -p memory-installer -- meta01` | Wave 0 |
| META-02 | marketplace.json exists | unit (file check) | `test -f plugins/memory-query-plugin/.claude-plugin/marketplace.json` | Exists |
| META-03 | Version in plugin.json drives install path | unit | `cargo test -p memory-installer -- meta03` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`
- **Phase gate:** Full `task pr-precheck` before PR

### Wave 0 Gaps
- [ ] `plugins/memory-query-plugin/.claude-plugin/plugin.json` -- create static metadata file (META-01)
- [ ] Tests for all CREG-* requirements using `tempfile` for isolated registry file testing
- [ ] Verify `chrono` or equivalent timestamp crate is available

## Sources

### Primary (HIGH confidence)
- Python reference implementation: `/Users/richardhightower/clients/spillwave/src/codebase-mentor/ai_codebase_mentor/converters/claude.py` -- exact JSON structures, merge logic, version cleanup
- Existing codebase: `crates/memory-installer/src/converters/claude.rs` -- current converter implementation
- Existing codebase: `crates/memory-installer/src/converters/opencode.rs` -- `generate_guidance()` merge pattern reference
- Existing codebase: `crates/memory-installer/src/writer.rs` -- `write_files()` and `merge_managed_section()` infrastructure
- Existing codebase: `plugins/memory-query-plugin/.claude-plugin/marketplace.json` -- existing marketplace metadata
- Reference plugin.json: `/Users/richardhightower/clients/spillwave/src/codebase-mentor/plugins/codebase-wizard/.claude-plugin/plugin.json`

### Secondary (MEDIUM confidence)
- CONTEXT.md user decisions -- locked JSON structures for registry files

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all deps already in workspace except chrono
- Architecture: HIGH - Python reference provides exact pattern, OpenCode converter demonstrates Rust equivalent
- Pitfalls: HIGH - Python reference shows all edge cases; path expansion and merge issues well-documented

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable domain -- Claude Code plugin format unlikely to change)
