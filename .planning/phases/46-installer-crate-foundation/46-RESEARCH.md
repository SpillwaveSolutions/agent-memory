# Phase 46: Installer Crate Foundation - Research

**Researched:** 2026-03-17
**Domain:** Rust CLI crate scaffolding — plugin parser, converter trait, tool maps, clap CLI
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Standalone `memory-installer` binary — NOT a subcommand of memory-daemon
- Zero coupling to daemon (no gRPC, no RocksDB, no tokio)
- Synchronous only — pure file I/O, no async needed
- Ships as part of the cross-compiled release (same CI pipeline as memory-daemon)
- `gray_matter` 0.3.x for YAML frontmatter parsing (serde_yaml is deprecated)
- `walkdir` 2.5 for directory traversal
- `clap` (workspace), `toml` (workspace), `serde`+`serde_json` (workspace), `shellexpand` (workspace), `directories` (workspace), `anyhow`+`thiserror` (workspace)
- Only 2 NEW external dependencies: gray_matter, walkdir
- Parser reads `plugins/installer-sources.json` to discover sources, then each marketplace.json
- PluginBundle: commands, agents, skills, hooks
- RuntimeConverter trait with convert_command, convert_agent, convert_skill, convert_hook, generate_guidance, target_dir
- Converters are stateless — all config passed via InstallConfig struct
- Tool maps in tool_maps.rs — static lookup, unmapped tools warn (not silently drop)
- CLI: `memory-installer install --agent <runtime> [--project|--global] [--dir <path>] [--dry-run]`
- Managed-section markers: `# --- MANAGED BY memory-installer (DO NOT EDIT) ---` / `# --- END MANAGED ---`
- JSON variant: `"__managed_by": "memory-installer"` for JSON config injection
- Dry-run: write-interceptor pattern on output stage
- Marker format is a compatibility contract — decided in Phase 46, never changed

### Claude's Discretion

- Exact module layout within crates/memory-installer/src/
- Error types and error handling patterns
- Whether to use a trait object (dyn RuntimeConverter) or an enum dispatch
- Test file organization
- Whether PluginBundle fields use owned Strings or borrowed &str

### Deferred Ideas (OUT OF SCOPE)

- Actual converter implementations — Phases 47-49
- Hook conversion pipeline — Phase 49
- E2E testing of installs — Phase 50
- --uninstall command — v2.8
- --all flag — v2.8
- Interactive mode — v2.8
- Version tracking with upgrade detection — v2.8
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INST-01 | Standalone `memory-installer` binary with clap CLI accepting `--agent <runtime>`, `--project`/`--global`, `--dir <path>`, `--dry-run` | Clap derive pattern from existing memory-daemon/cli.rs; standalone binary per memory-ingest precedent |
| INST-02 | Plugin parser extracts commands, agents, skills with YAML frontmatter from canonical source directory | gray_matter 0.3.x parse API documented below; marketplace.json discovery path confirmed from Phase 45 artifacts |
| INST-03 | `RuntimeConverter` trait with `convert_command`, `convert_agent`, `convert_skill`, `convert_hook`, `generate_guidance`, `target_dir` methods | Trait design verified against all 6 planned converter impls; return types chosen to handle Codex multi-file output |
| INST-04 | Centralized tool mapping tables in `tool_maps.rs` covering all 11 tool names across 6 runtimes | Full 11x6 mapping table in v2.7 plan; static BTreeMap/match lookup pattern researched |
| INST-05 | Managed-section markers in shared config files enabling safe merge, upgrade, and uninstall | Marker format and three-case merge logic derived from GSD installer reference; format locked in this phase |
| INST-06 | `--dry-run` mode shows what would be installed without writing files | Write-interceptor on ConvertedFile stage — no per-converter changes needed; pattern validated |
| INST-07 | Unmapped tool names produce warnings (not silent drops) | `map_tool` returns `Option<String>`; caller logs warning via tracing::warn! when None |
</phase_requirements>

---

## Summary

Phase 46 scaffolds the entire `memory-installer` crate: workspace integration, binary entry point, types, plugin parser, converter trait, and tool mapping tables. It produces no converter implementations — those follow in Phases 47-49. The output of this phase is the foundation contract that all downstream phases build on.

The canonical plugin source is already in place from Phase 45: two directories (`plugins/memory-query-plugin/`, `plugins/memory-setup-plugin/`) each with a `.claude-plugin/marketplace.json` manifest, a `commands/` directory of `.md` files with YAML frontmatter, an `agents/` directory, and a `skills/` tree. The installer-sources.json discovery manifest was also created in Phase 45.

The key architectural decisions are already locked: standalone binary (like memory-ingest, not memory-daemon), synchronous I/O, gray_matter for frontmatter parsing, trait-based converter dispatch, and centralized tool maps. Phase 46's job is to implement these decisions as working Rust code with tests, not to make new architectural choices.

**Primary recommendation:** Build in three waves — (1) crate scaffolding + CLI skeleton that compiles and passes pr-precheck, (2) types + parser with unit tests against real plugin files, (3) converter trait + tool maps with compile-time coverage. Each wave is independently verifiable.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `gray_matter` | 0.3.2 | Parse `---YAML---` frontmatter blocks from .md files | Only maintained frontmatter crate; uses yaml-rust2 internally; serde_yaml is deprecated |
| `walkdir` | 2.5.0 | Recursive directory traversal for plugin source trees | 218M downloads; standard choice for this purpose; handles symlinks and iterator errors |
| `clap` | 4.5 (workspace) | CLI: `install --agent <runtime> --project --dry-run` | Already in workspace; derive macros with automatic --help |
| `serde` + `serde_json` | 1.0 (workspace) | Types serialization + JSON config merging | Already in workspace |
| `anyhow` + `thiserror` | 1.0 / 2.0 (workspace) | Error handling through parser and converter pipeline | Workspace standard |
| `shellexpand` | 3.1 (workspace) | Expand `~/.claude/` → absolute paths | Already in workspace via memory-daemon |
| `directories` | 6.0 (workspace) | `~/.config/agent-memory/`, XDG path resolution | Already in workspace |
| `toml` | 0.8 (workspace) | Gemini TOML output (used in Phase 48 but dependency declared now) | Already in workspace |
| `tracing` | 0.1 (workspace) | `tracing::warn!` for unmapped tool names (INST-07) | Workspace standard for all binaries |

### Supporting (dev-dependencies only)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | 3.15 (workspace) | Temporary dirs for parser integration tests | Test-only |

### Installation

```toml
# crates/memory-installer/Cargo.toml
[dependencies]
gray_matter = { version = "0.3", features = ["yaml"] }
walkdir = "2.5"
clap = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
directories = { workspace = true }
shellexpand = "3.1"
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }

# workspace Cargo.toml additions:
# [workspace.dependencies]
# gray_matter = { version = "0.3", features = ["yaml"] }
# walkdir = "2.5"
#
# [workspace] members:
# "crates/memory-installer"
```

---

## Architecture Patterns

### Recommended Project Structure

```
crates/memory-installer/
├── Cargo.toml
└── src/
    ├── main.rs              # binary entry — clap parse → install()
    ├── lib.rs               # pub re-exports for integration tests
    ├── types.rs             # PluginBundle, PluginCommand, PluginAgent,
    │                        #   PluginSkill, HookDefinition, ConvertedFile,
    │                        #   InstallScope, InstallConfig, Runtime
    ├── parser.rs            # parse_sources() → Result<PluginBundle>
    │                        #   reads installer-sources.json + marketplace.json
    │                        #   uses walkdir + gray_matter
    ├── converter.rs         # RuntimeConverter trait definition
    ├── tool_maps.rs         # static per-runtime tool name tables
    ├── writer.rs            # write_files() / dry_run_report() write-interceptor
    └── converters/
        ├── mod.rs           # select_converter(runtime) → Box<dyn RuntimeConverter>
        ├── claude.rs        # stub (Phase 47)
        ├── opencode.rs      # stub (Phase 47)
        ├── gemini.rs        # stub (Phase 48)
        ├── codex.rs         # stub (Phase 48)
        ├── copilot.rs       # stub (Phase 49)
        └── skills.rs        # stub (Phase 49)
```

**Note on stubs:** Phase 46 creates all converter files as stubs that implement the trait but return empty `Vec<ConvertedFile>`. This allows the crate to compile completely, pass clippy, and have the dispatch table wired up before Phase 47 fills in real logic.

### Pattern 1: gray_matter Frontmatter Parsing

**What:** Parse YAML frontmatter from `.md` files using gray_matter 0.3.x.

**When to use:** Every `.md` file in commands/, agents/, skills/

**API (verified from crates.io + docs.rs):**

```rust
use gray_matter::Matter;
use gray_matter::engine::YAML;

let matter = Matter::<YAML>::new();
let parsed = matter.parse(&content);
// parsed.data: Option<gray_matter::Pod>  ← the YAML frontmatter
// parsed.content: String                 ← body after second ---
// parsed.excerpt: Option<String>         ← unused here

// Extract as serde Value:
if let Some(data) = parsed.data {
    let value: serde_json::Value = data.deserialize()?;
}
```

**Important:** `gray_matter::Pod` implements `Deserialize` via `serde`. Deserialize into `serde_json::Value` (not serde_yaml::Value) since gray_matter does not depend on serde_yaml. Store frontmatter as `serde_json::Value` throughout the crate — this avoids pulling in a YAML-specific value type.

**Graceful degradation:** If `parsed.data` is `None` (no frontmatter or malformed), log a warning and treat as empty metadata + full body. Do not fail hard — some skill files may legitimately have no frontmatter.

### Pattern 2: Discovery via installer-sources.json + marketplace.json

**What:** Two-level discovery: sources manifest → per-plugin manifest → asset list

**Data flow:**

```
plugins/installer-sources.json
  → sources[*].path = "./memory-query-plugin"
      → plugins/memory-query-plugin/.claude-plugin/marketplace.json
          → plugins[0].commands[*] = "./commands/memory-search.md"
          → plugins[0].agents[*]   = "./agents/memory-navigator.md"
          → plugins[0].skills[*]   = "./skills/memory-query"
```

**Implementation note:** The marketplace.json `skills` entries are directory paths, not files. Walk each skill directory to find `SKILL.md`. The `references/` and `scripts/` subdirectories should be captured as additional `ConvertedFile` entries within each `PluginSkill`.

### Pattern 3: RuntimeConverter Trait

**What:** One impl per runtime. CLI dispatches to correct impl via `Box<dyn RuntimeConverter>`.

**Trait signature (from CONTEXT.md + v2.7 plan):**

```rust
// Source: .planning/phases/46-installer-crate-foundation/46-CONTEXT.md
pub trait RuntimeConverter {
    fn name(&self) -> &str;
    fn target_dir(&self, scope: &InstallScope) -> PathBuf;
    fn convert_command(&self, cmd: &PluginCommand, cfg: &InstallConfig) -> Vec<ConvertedFile>;
    fn convert_agent(&self, agent: &PluginAgent, cfg: &InstallConfig) -> Vec<ConvertedFile>;
    fn convert_skill(&self, skill: &PluginSkill, cfg: &InstallConfig) -> Vec<ConvertedFile>;
    fn convert_hook(&self, hook: &HookDefinition, cfg: &InstallConfig) -> Option<ConvertedFile>;
    fn generate_guidance(&self, bundle: &PluginBundle, cfg: &InstallConfig) -> Vec<ConvertedFile>;
}
```

**Note on return types:** `convert_command` returns `Vec<ConvertedFile>` (not singular) because the Codex converter produces a skill directory (multiple files) from one command. This is confirmed by the Codex converter spec in v2.7 plan.

**Dispatch table in converters/mod.rs:**

```rust
pub fn select_converter(runtime: Runtime) -> Box<dyn RuntimeConverter> {
    match runtime {
        Runtime::Claude   => Box::new(ClaudeConverter),
        Runtime::OpenCode => Box::new(OpenCodeConverter),
        Runtime::Gemini   => Box::new(GeminiConverter),
        Runtime::Codex    => Box::new(CodexConverter),
        Runtime::Copilot  => Box::new(CopilotConverter),
        Runtime::Skills   => Box::new(SkillsConverter),
    }
}
```

### Pattern 4: Tool Maps as Static Match

**What:** Tool name lookup: Claude PascalCase → runtime-specific name. Returns `Option<&'static str>` — `None` triggers a warning.

**Full mapping table (from v2.7 plan — HIGH confidence):**

| Claude | OpenCode | Gemini | Codex | Copilot |
|--------|----------|--------|-------|---------|
| Read | read | read_file | read | read |
| Write | write | write_file | edit | edit |
| Edit | edit | replace | edit | edit |
| Bash | bash | run_shell_command | execute | execute |
| Grep | grep | search_file_content | search | search |
| Glob | glob | glob | search | search |
| WebSearch | websearch | google_web_search | web | web |
| WebFetch | webfetch | web_fetch | web | web |
| TodoWrite | todowrite | write_todos | todo | todo |
| AskUserQuestion | question | ask_user | ask_user | ask_user |
| Task | task | *(excluded — None)* | agent | agent |

**MCP tools (`mcp__*`):** Pass through unchanged for Claude and OpenCode; excluded (return None + warn) for Gemini/Codex/Copilot.

**Implementation:** A `match` expression is the cleanest Rust approach — no HashMap overhead, exhaustive coverage enforced at compile time for known tools, unknown tools fall through to `None`.

```rust
// Source: tool_maps.rs pattern
pub fn map_tool(runtime: Runtime, claude_name: &str) -> Option<&'static str> {
    // MCP tools pass through for Claude/OpenCode
    if claude_name.starts_with("mcp__") {
        return match runtime {
            Runtime::Claude | Runtime::OpenCode => Some(claude_name), // leaks lifetime...
            _ => None,
        };
        // NOTE: For MCP pass-through, caller must handle the owned string case separately
    }
    match (runtime, claude_name) {
        (Runtime::OpenCode, "Read") => Some("read"),
        (Runtime::OpenCode, "Write") => Some("write"),
        // ... etc.
        _ => None,
    }
}
```

**MCP pass-through detail:** Since MCP tool names are dynamic strings, the return type `Option<&'static str>` cannot accommodate pass-through without cloning. Recommendation: return `Option<Cow<'static, str>>` or have the caller handle the `mcp__*` prefix check before calling `map_tool`. The latter is simpler for Phase 46; callers in Phase 47-48 can handle MCP detection.

### Pattern 5: InstallConfig as the Config Carrier

**What:** Single struct threading all install-time configuration through the converter pipeline. Converters are stateless — they receive config on every call.

```rust
pub struct InstallConfig {
    pub scope: InstallScope,
    pub dry_run: bool,
    pub source_root: PathBuf,  // where plugins/ directory lives
}

pub enum InstallScope {
    Project(PathBuf),    // ./.claude/ relative to project root
    Global,              // ~/.claude/ etc.
    Custom(PathBuf),     // --dir <path> (required for Skills)
}
```

### Pattern 6: Write-Interceptor for --dry-run (INST-06)

**What:** All converters produce `Vec<ConvertedFile>`. A single `write_files()` function in `writer.rs` either writes the files or prints the dry-run report — no per-converter changes needed.

```rust
pub struct ConvertedFile {
    pub target_path: PathBuf,
    pub content: String,
    pub overwrite_existing: bool,  // computed at write time
}

pub fn write_files(files: &[ConvertedFile], dry_run: bool) -> Result<()> {
    for f in files {
        let exists = f.target_path.exists();
        if dry_run {
            let action = if exists { "OVERWRITE" } else { "CREATE" };
            println!("[DRY-RUN] {} {}", action, f.target_path.display());
            println!("  {} bytes", f.content.len());
        } else {
            // create parent dirs, write content
        }
    }
    Ok(())
}
```

### Pattern 7: Managed-Section Markers (INST-05)

**What:** The installer injects sections into shared config files (opencode.json, .gemini/settings.json) using markers. Content between markers is owned by the installer; content outside is owned by the user.

**Marker strings (locked — compatibility contract):**

For text-based files (TOML, shell config):
```
# --- MANAGED BY memory-installer (DO NOT EDIT) ---
... managed content ...
# --- END MANAGED ---
```

For JSON files:
```json
{
  "__managed_by": "memory-installer",
  ... managed key-value pairs ...
}
```

**Three-case merge logic:**
1. File does not exist → create with markers wrapping managed content
2. File exists with markers → replace content between markers
3. File exists without markers → append markers + managed content at end

This logic lives in `writer.rs` as `merge_managed_section()`. Phase 46 implements the infrastructure; actual JSON/TOML merging happens in Phases 47-48 when converters need it.

### Pattern 8: CLI Structure with clap derive

**What:** Standalone binary with `install` subcommand.

```rust
// Source: derived from existing memory-daemon/src/cli.rs pattern
#[derive(Parser, Debug)]
#[command(name = "memory-installer")]
#[command(about = "Install memory-agent plugins for various AI runtimes")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install memory plugins for an AI runtime
    Install {
        /// Target runtime
        #[arg(long, value_enum)]
        agent: Runtime,

        /// Install to project directory (e.g., ./.claude/)
        #[arg(long, conflicts_with = "global")]
        project: bool,

        /// Install to global user directory (e.g., ~/.claude/)
        #[arg(long, conflicts_with = "project")]
        global: bool,

        /// Custom target directory (required with --agent skills)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Preview what would be installed without writing files
        #[arg(long)]
        dry_run: bool,

        /// Path to canonical source root (defaults to auto-discovery)
        #[arg(long)]
        source: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum Runtime {
    Claude,
    OpenCode,
    Gemini,
    Codex,
    Copilot,
    Skills,
}
```

### Anti-Patterns to Avoid

- **Using serde_yaml:** Officially deprecated (0.9.34+deprecated, March 2024). Use gray_matter which uses yaml-rust2 internally.
- **Frontmatter as typed structs per runtime:** Each converter needs different fields. Store frontmatter as `serde_json::Value` map for maximum flexibility.
- **One monolithic `convert_all(bundle, runtime)` function:** Makes Codex→Skills delegation impossible; prevents runtime-local tests. One struct per runtime.
- **Inline tool name strings in converter files:** Centralizing in tool_maps.rs prevents per-converter drift across 11 tools × 6 runtimes.
- **tokio dependency:** The installer is synchronous. `std::fs` is sufficient for file I/O. No async context needed.
- **`include_str!` for canonical source:** Rebuilding the binary on every skill edit is wrong. Read from filesystem at install time.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML frontmatter splitting | Manual `---` delimiter splitting | `gray_matter` 0.3.x | Handles edge cases: no frontmatter, TOML frontmatter, nested YAML, multiline values |
| Directory recursion | Manual `std::fs::read_dir` recursion | `walkdir` 2.5 | Handles symlinks, iterator errors, and depth control cleanly |
| Path expansion | Manual `~` replacement | `shellexpand::tilde()` | Cross-platform, handles `$HOME`, edge cases on Windows |
| Config dir resolution | Manual env var checks | `directories::BaseDirs` | XDG-compliant on Linux, macOS, Windows; handles non-standard `XDG_CONFIG_HOME` |

**Key insight:** The installer's complexity is in the conversion logic, not in infrastructure. Keep infrastructure thin.

---

## Common Pitfalls

### Pitfall 1: gray_matter Returns Pod, Not serde_yaml::Value

**What goes wrong:** Developers assume gray_matter returns a serde_yaml::Value and try to pattern-match on YAML variants.

**Why it happens:** gray_matter has its own `Pod` type (a YAML-like value enum). It does NOT use serde_yaml internally — it uses yaml-rust2.

**How to avoid:** Deserialize `Pod` into `serde_json::Value` immediately after parsing:
```rust
let value: serde_json::Value = parsed.data
    .ok_or_else(|| anyhow!("missing frontmatter"))?
    .deserialize()
    .context("failed to deserialize frontmatter")?;
```
Store all frontmatter as `serde_json::Value` throughout the crate.

**Warning signs:** Compilation errors about Pod not implementing expected YAML traits.

### Pitfall 2: Skills Directory Structure vs File Structure

**What goes wrong:** Parser treats every path in marketplace.json as a file path. Skills entries are directory paths — the parser must walk the directory for SKILL.md.

**Why it happens:** Commands and agents are single `.md` files. Skills are directories with SKILL.md + optional references/ and scripts/ subdirectories.

**How to avoid:** In `parse_skill()`, check if the path is a directory. If so, look for `SKILL.md` within it and recurse into `references/` and `scripts/` for additional files.

**Warning signs:** `File not found` errors for skill paths that are clearly directory names.

### Pitfall 3: Missing --project and --global Both Absent

**What goes wrong:** User runs `memory-installer install --agent claude` without specifying scope. Binary panics or silently installs to the wrong place.

**Why it happens:** clap does not require at least one of a pair by default.

**How to avoid:** Use `#[arg(required_unless_present = "global")]` or validate in the command handler:
```rust
if !args.project && !args.global && args.dir.is_none() {
    eprintln!("error: one of --project, --global, or --dir is required");
    std::process::exit(1);
}
```
Also: `--agent skills` requires `--dir` — validate this explicitly.

**Warning signs:** Tests pass but manual invocation produces confusing behavior.

### Pitfall 4: Marker Format Changed After First Release

**What goes wrong:** Phase 46 uses one marker string; later a developer "improves" it. Existing installs have the old markers; the installer no longer recognizes them for upgrade/uninstall.

**Why it happens:** Markers look like arbitrary strings before you understand they are a compatibility contract.

**How to avoid:** Define markers as constants in Phase 46 with a doc comment explaining the compatibility contract:
```rust
/// Managed-section begin marker.
/// THIS STRING IS A COMPATIBILITY CONTRACT — never change it after first release.
pub const MANAGED_BEGIN: &str = "# --- MANAGED BY memory-installer (DO NOT EDIT) ---";
pub const MANAGED_END: &str = "# --- END MANAGED ---";
pub const MANAGED_JSON_KEY: &str = "__managed_by";
pub const MANAGED_JSON_VALUE: &str = "memory-installer";
```

**Warning signs:** Tests for upgrade/uninstall fail after seemingly innocent refactoring.

### Pitfall 5: Unmapped Tools Silently Dropped (INST-07)

**What goes wrong:** A new tool is added to the canonical source, or a tool name typo exists. The converter silently produces output missing the tool mapping. No error, no warning.

**Why it happens:** `map_tool()` returns `None` and callers propagate None by filtering it out.

**How to avoid:** Callers of `map_tool()` MUST log a warning when `None` is returned:
```rust
let mapped = tool_maps::map_tool(runtime, tool_name)
    .map(|s| s.to_string())
    .unwrap_or_else(|| {
        tracing::warn!("unmapped tool '{}' for runtime {:?} — keeping original", tool_name, runtime);
        tool_name.to_string()
    });
```
Whether to keep the original name or drop it is a per-converter decision; the warning is mandatory.

### Pitfall 6: Workspace Dependency Not Added for gray_matter/walkdir

**What goes wrong:** `crates/memory-installer/Cargo.toml` adds `gray_matter` and `walkdir` directly without adding them to `[workspace.dependencies]` first. This works but creates inconsistency with workspace conventions.

**Why it happens:** It is easy to forget the two-step: workspace.dependencies first, then `{ workspace = true }` in the crate.

**How to avoid:** Add both to `[workspace.dependencies]` in root `Cargo.toml` first, then use `gray_matter = { workspace = true }` in the crate.

---

## Code Examples

### Parsing a Command File

```rust
// Source: gray_matter 0.3.2 docs.rs + .planning/research/STACK.md
use gray_matter::{Matter, engine::YAML};
use serde_json::Value;
use anyhow::{Context, Result};

pub fn parse_md_file(path: &std::path::Path) -> Result<(Value, String)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);

    let frontmatter: Value = match parsed.data {
        Some(pod) => pod.deserialize()
            .with_context(|| format!("deserializing frontmatter in {}", path.display()))?,
        None => {
            tracing::warn!("no frontmatter in {} — treating as empty", path.display());
            Value::Object(serde_json::Map::new())
        }
    };

    Ok((frontmatter, parsed.content))
}
```

### Walking a Plugin Source Directory

```rust
// Source: walkdir 2.5 + marketplace.json discovery pattern
use walkdir::WalkDir;

pub fn list_skill_files(skill_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(skill_dir).follow_links(false) {
        let entry = entry.with_context(|| format!("walking {}", skill_dir.display()))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_owned());
        }
    }
    Ok(files)
}
```

### Dry-Run Report

```rust
// Source: CONTEXT.md dry-run specification
pub fn report_dry_run(files: &[ConvertedFile]) {
    println!("[DRY-RUN] Would install {} files:", files.len());
    for f in files {
        let status = if f.target_path.exists() { "OVERWRITE" } else { "  CREATE" };
        println!("  [{}] {}", status, f.target_path.display());
        println!("         {} bytes", f.content.len());
    }
}
```

### Tool Map Lookup with Warning

```rust
// Source: CONTEXT.md + v2.7 plan tool mapping table
pub fn map_tool_with_warning(runtime: Runtime, claude_name: &str) -> String {
    match tool_maps::map_tool(runtime, claude_name) {
        Some(mapped) => mapped.to_string(),
        None => {
            tracing::warn!(
                tool = claude_name,
                ?runtime,
                "tool has no mapping — keeping original name"
            );
            claude_name.to_string()
        }
    }
}
```

### Managed-Section Merge

```rust
// Source: CONTEXT.md + GSD installer three-case pattern
use std::path::Path;
use anyhow::Result;

pub fn merge_managed_section(file_path: &Path, managed_content: &str) -> Result<()> {
    const BEGIN: &str = "# --- MANAGED BY memory-installer (DO NOT EDIT) ---";
    const END: &str = "# --- END MANAGED ---";

    let managed_block = format!("{}\n{}\n{}\n", BEGIN, managed_content, END);

    if !file_path.exists() {
        // Case 1: new file
        std::fs::write(file_path, &managed_block)?;
        return Ok(());
    }

    let existing = std::fs::read_to_string(file_path)?;

    if let (Some(start), Some(end_pos)) = (existing.find(BEGIN), existing.find(END)) {
        // Case 2: file has markers — replace between them
        let before = &existing[..start];
        let after = &existing[end_pos + END.len()..];
        std::fs::write(file_path, format!("{}{}{}", before, managed_block, after))?;
    } else {
        // Case 3: file exists without markers — append
        let updated = format!("{}\n{}", existing.trim_end(), managed_block);
        std::fs::write(file_path, updated)?;
    }

    Ok(())
}
```

---

## Canonical Source Frontmatter Reference

Observed frontmatter fields across all current canonical plugin files (HIGH confidence — read directly from source):

**Command files (commands/*.md):**
```yaml
---
name: memory-search         # string — command name
description: "..."          # string or block scalar
parameters:                 # array of parameter objects
  - name: topic
    description: "..."
    required: true
    type: flag              # optional
    default: 7              # optional
skills:                     # array of skill names
  - memory-query
---
```

**Agent files (agents/*.md):**
```yaml
---
name: memory-navigator      # string
description: "..."          # string
triggers:                   # array of trigger objects
  - pattern: "..."
    type: message_pattern
skills:                     # array of skill names
  - memory-query
  - topic-graph
---
```

**Skill files (skills/*/SKILL.md):**
```yaml
---
name: memory-query          # string
description: |              # block scalar (multiline)
  Query past conversations...
license: MIT                # optional
metadata:                   # nested object
  version: 2.0.0
  author: SpillwaveSolutions
---
```

**Key observation:** Current canonical source does NOT use `allowed-tools:` or `color:` fields. These are fields from other Claude plugins (like GSD superpowers). The converter implementations (Phases 47-48) must handle them for correctness when encountering non-memory plugins, but Phase 46 parser just captures whatever fields exist without special-casing.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `serde_yaml` for YAML parsing | `gray_matter` with `yaml-rust2` | March 2024 (serde_yaml deprecated) | Must not use serde_yaml; gray_matter is the correct choice |
| Hand-rolled frontmatter splitting | `gray_matter` library | 2023+ | Handles edge cases in delimiter detection and multiline values |
| `clap` 3.x builder API | `clap` 4.x derive API | 2022 | Derive macros via `#[derive(Parser)]` — workspace already on 4.5 |

**Deprecated/outdated:**
- `serde_yaml` 0.9.34: Author-deprecated in March 2024; no future maintenance. The crates.io listing shows `+deprecated` in the version string.
- `yaml-rust` (original): Abandoned; replaced by `yaml-rust2` community fork.

---

## Open Questions

1. **MCP tool pass-through for map_tool() return type**
   - What we know: MCP tools (`mcp__*`) should pass through unchanged for Claude/OpenCode. `Option<&'static str>` cannot hold dynamic strings.
   - What's unclear: Best return type — `Option<Cow<'static, str>>` vs caller-side MCP check before calling map_tool.
   - Recommendation: Have callers check `tool_name.starts_with("mcp__")` before calling `map_tool()`. This keeps the map function simple with a static return type. Document this as the pattern in converter.rs doc comments.

2. **PluginBundle fields: owned Strings or borrowed &str**
   - What we know: CONTEXT.md marks this as Claude's discretion. The bundle lives for the duration of the install operation.
   - What's unclear: Whether borrowed lifetimes would complicate test code or the converter trait.
   - Recommendation: Use owned `String` fields in all types. The installer processes at most ~20 files; allocations are irrelevant. Borrowed lifetimes would complicate the `Box<dyn RuntimeConverter>` dispatch.

3. **Trait objects vs enum dispatch**
   - What we know: CONTEXT.md marks this as Claude's discretion. Six known runtimes.
   - What's unclear: Whether enum dispatch would be measurably better for this use case.
   - Recommendation: Use `Box<dyn RuntimeConverter>`. Adding a new runtime is one new file + one match arm in `select_converter()`. The number of dispatches per run is tiny (one per bundle item per runtime). Enum dispatch would require modifying the central enum for each new runtime — the trait object approach is cleaner for extensibility.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + cargo test |
| Config file | none — standard Rust test runner |
| Quick run command | `cargo test -p memory-installer` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INST-01 | CLI parses all flags without panicking | unit | `cargo test -p memory-installer cli` | ❌ Wave 0 |
| INST-01 | `--agent skills` without `--dir` produces error | unit | `cargo test -p memory-installer cli::test_skills_requires_dir` | ❌ Wave 0 |
| INST-01 | `--dry-run` flag threads through to writer | unit | `cargo test -p memory-installer cli::test_dry_run_flag` | ❌ Wave 0 |
| INST-02 | Parser reads installer-sources.json and walks both plugin dirs | integration | `cargo test -p memory-installer parser::test_parse_sources` | ❌ Wave 0 |
| INST-02 | Parser extracts correct frontmatter fields from command .md | unit | `cargo test -p memory-installer parser::test_parse_command` | ❌ Wave 0 |
| INST-02 | Parser handles missing frontmatter gracefully | unit | `cargo test -p memory-installer parser::test_parse_no_frontmatter` | ❌ Wave 0 |
| INST-02 | Parser extracts skill directory files correctly | unit | `cargo test -p memory-installer parser::test_parse_skill_dir` | ❌ Wave 0 |
| INST-03 | All 6 converter stubs implement RuntimeConverter (compile check) | compile | `cargo build -p memory-installer` | ❌ Wave 0 |
| INST-04 | All 11 tools mapped for opencode runtime | unit | `cargo test -p memory-installer tool_maps::test_opencode_map` | ❌ Wave 0 |
| INST-04 | All 11 tools mapped for gemini runtime | unit | `cargo test -p memory-installer tool_maps::test_gemini_map` | ❌ Wave 0 |
| INST-04 | Task tool maps to None for gemini | unit | `cargo test -p memory-installer tool_maps::test_task_excluded_gemini` | ❌ Wave 0 |
| INST-05 | Managed-section merge: new file creates with markers | unit | `cargo test -p memory-installer writer::test_merge_new_file` | ❌ Wave 0 |
| INST-05 | Managed-section merge: existing file with markers replaces section | unit | `cargo test -p memory-installer writer::test_merge_existing_markers` | ❌ Wave 0 |
| INST-05 | Managed-section merge: existing file without markers appends | unit | `cargo test -p memory-installer writer::test_merge_no_markers` | ❌ Wave 0 |
| INST-06 | Dry-run prints file paths without writing | unit | `cargo test -p memory-installer writer::test_dry_run_no_write` | ❌ Wave 0 |
| INST-07 | Unknown tool name produces tracing::warn output | unit | `cargo test -p memory-installer tool_maps::test_unmapped_warns` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo test --workspace --all-features`
- **Phase gate:** `task pr-precheck` (format + clippy + test + doc) green before `/gsd:verify-work`

### Wave 0 Gaps

All test files are new — none exist yet:

- [ ] `crates/memory-installer/src/` — entire crate does not exist yet
- [ ] `crates/memory-installer/src/main.rs` — CLI entry
- [ ] `crates/memory-installer/src/lib.rs` — re-exports
- [ ] `crates/memory-installer/src/types.rs` — PluginBundle, ConvertedFile, etc.
- [ ] `crates/memory-installer/src/parser.rs` — with inline `#[cfg(test)]` unit tests
- [ ] `crates/memory-installer/src/converter.rs` — trait definition
- [ ] `crates/memory-installer/src/tool_maps.rs` — with inline `#[cfg(test)]` unit tests
- [ ] `crates/memory-installer/src/writer.rs` — with inline `#[cfg(test)]` unit tests
- [ ] `crates/memory-installer/src/converters/mod.rs` — dispatch table
- [ ] `crates/memory-installer/src/converters/claude.rs` — stub
- [ ] `crates/memory-installer/src/converters/opencode.rs` — stub
- [ ] `crates/memory-installer/src/converters/gemini.rs` — stub
- [ ] `crates/memory-installer/src/converters/codex.rs` — stub
- [ ] `crates/memory-installer/src/converters/copilot.rs` — stub
- [ ] `crates/memory-installer/src/converters/skills.rs` — stub
- [ ] `crates/memory-installer/Cargo.toml` — package definition with gray_matter + walkdir
- [ ] Framework install: `gray_matter = { workspace = true }` + `walkdir = "2.5"` in root Cargo.toml

---

## Sources

### Primary (HIGH confidence)

- `.planning/phases/46-installer-crate-foundation/46-CONTEXT.md` — locked decisions, phase boundary, implementation specifics
- `.planning/research/STACK.md` — gray_matter/walkdir versions, dependency audit, serde_yaml deprecation evidence
- `.planning/research/ARCHITECTURE.md` — RuntimeConverter trait, data flow, component responsibilities
- `.planning/research/FEATURES.md` — dry-run pattern, managed-section three-case logic, GSD installer reference
- `docs/plans/v2.7-multi-runtime-portability-plan.md` — authoritative tool mapping table, phase plan
- `plugins/installer-sources.json` — discovery manifest structure (read directly)
- `plugins/memory-query-plugin/.claude-plugin/marketplace.json` — asset path format (read directly)
- `plugins/memory-setup-plugin/.claude-plugin/marketplace.json` — asset path format (read directly)
- `plugins/memory-query-plugin/commands/memory-search.md` — actual command frontmatter (read directly)
- `plugins/memory-query-plugin/commands/memory-recent.md` — parameter defaults in frontmatter (read directly)
- `plugins/memory-query-plugin/agents/memory-navigator.md` — agent frontmatter with triggers array (read directly)
- `plugins/memory-setup-plugin/agents/setup-troubleshooter.md` — agent frontmatter with multi-pattern triggers (read directly)
- `plugins/memory-query-plugin/skills/memory-query/SKILL.md` — skill frontmatter with metadata object (read directly)
- `crates/memory-daemon/src/cli.rs` — clap derive pattern to replicate (read directly)
- `crates/memory-ingest/Cargo.toml` — simple standalone binary pattern (read directly)
- `Cargo.toml` — workspace dependencies available to installer (read directly)
- `.planning/config.json` — nyquist_validation: true confirmed (read directly)

### Secondary (MEDIUM confidence)

- crates.io: `gray_matter` 0.3.2 — latest July 2025, `features = ["yaml"]` required, uses yaml-rust2 internally
- crates.io: `walkdir` 2.5.0 — latest March 2024, 218M downloads
- crates.io: `serde_yaml` 0.9.34+deprecated — confirmed deprecated March 2024

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions confirmed from workspace + crates.io; gray_matter deprecation evidence is authoritative
- Architecture: HIGH — RuntimeConverter trait, type names, and data flow locked in CONTEXT.md; backed by ARCHITECTURE.md and v2.7 plan
- Pitfalls: HIGH — gray_matter Pod type, skill directory structure, marker format: all derived from direct inspection of source files and CONTEXT.md

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable ecosystem; gray_matter and walkdir APIs are unlikely to change)
