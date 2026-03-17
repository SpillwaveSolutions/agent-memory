# Architecture Research

**Domain:** Multi-runtime plugin installer for Rust workspace (v2.7)
**Researched:** 2026-03-16
**Confidence:** HIGH

---

## Context

This document covers **v2.7 architecture** for the multi-runtime installer. The v2.6 architecture (episodic memory, ranking, lifecycle) is archived in `v2.6-ARCHITECTURE.md` for reference.

v2.7 adds a new `memory-installer` crate to the existing 14-crate agent-memory Rust workspace. The installer converts the canonical Claude plugin source into runtime-specific installations for Claude, OpenCode, Gemini, Codex, Copilot, and generic skill targets — replacing manually-maintained adapter directories.

**Evidence base:** Existing workspace code (`memory-daemon/src/clod.rs`, `cli.rs`, `commands.rs`), canonical plugin source (`plugins/memory-query-plugin/`, `plugins/memory-setup-plugin/`), v2.7 implementation plan, GSD frontmatter parsing patterns from `frontmatter.cjs`.

---

## System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                    Canonical Source (input)                       │
│  plugins/memory-plugin/                                           │
│  ┌──────────────┐  ┌────────────┐  ┌──────────────┐  ┌────────┐ │
│  │  commands/   │  │  agents/   │  │   skills/    │  │ hooks/ │ │
│  │ (6 .md files)│  │ (2 agents) │  │ (13 SKILL.md)│  │ YAML   │ │
│  └──────┬───────┘  └─────┬──────┘  └──────┬───────┘  └───┬────┘ │
└─────────┼────────────────┼────────────────┼──────────────┼──────┘
          │                │                │              │
          ▼                ▼                ▼              ▼
┌──────────────────────────────────────────────────────────────────┐
│              memory-installer crate (crates/memory-installer/)    │
│                                                                   │
│  ┌────────────┐  ┌───────────────┐  ┌────────────────────────┐  │
│  │  parser.rs │  │ converter.rs  │  │     tool_maps.rs       │  │
│  │            │  │ (trait def)   │  │ (Claude→runtime tables)│  │
│  │ parse_dir()│  │               │  │                        │  │
│  │ →PluginBun-│  │RuntimeConver- │  │ CLAUDE_TO_OPENCODE     │  │
│  │  dle       │  │  ter trait    │  │ CLAUDE_TO_GEMINI       │  │
│  └─────┬──────┘  └───────┬───────┘  │ CLAUDE_TO_CODEX       │  │
│        │                 │          └───────────┬────────────┘  │
│        │                 ▼                      │               │
│  ┌────────────────────────────────────────────┐ │               │
│  │              converters/                   │◄┘               │
│  │  ┌──────┐ ┌──────────┐ ┌────────┐ ┌─────┐ │                 │
│  │  │claude│ │opencode  │ │ gemini │ │codex│ │                 │
│  │  │  .rs │ │  .rs     │ │  .rs   │ │ .rs │ │                 │
│  │  └──────┘ └──────────┘ └────────┘ └─────┘ │                 │
│  │  ┌─────────┐  ┌──────────┐                 │                 │
│  │  │copilot  │  │skills.rs │                 │                 │
│  │  │  .rs    │  │(generic) │                 │                 │
│  │  └─────────┘  └──────────┘                 │                 │
│  └────────────────────────────────────────────┘                 │
│                                                                   │
│  ┌────────────────┐  ┌───────────────────────────────────────┐  │
│  │   hooks.rs     │  │              CLI entry                │  │
│  │ (per-runtime   │  │  memory-installer install             │  │
│  │  hook formats) │  │  --agent <runtime> [--project|global] │  │
│  └────────────────┘  │  [--dir path] [--dry-run]             │  │
│                       └───────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────────────────────────────────┐
│                     Install Targets (output)                      │
│  ┌─────────┐ ┌──────────┐ ┌────────┐ ┌───────┐ ┌─────────────┐ │
│  │ Claude  │ │ OpenCode │ │ Gemini │ │ Codex │ │  Generic    │ │
│  │~/.claude│ │~/.config/│ │~/.gem- │ │~/.cod-│ │  <--dir>    │ │
│  │/plugins/│ │opencode/ │ │ ini/   │ │ ex/   │ │             │ │
│  └─────────┘ └──────────┘ └────────┘ └───────┘ └─────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Location |
|-----------|----------------|----------|
| `parser.rs` | Walk canonical plugin dir, parse YAML frontmatter, build `PluginBundle` | `crates/memory-installer/src/parser.rs` |
| `types.rs` | `PluginBundle`, `PluginCommand`, `PluginAgent`, `PluginSkill`, `HookDefinition`, `ConvertedFile`, `InstallScope` | `crates/memory-installer/src/types.rs` |
| `converter.rs` | `RuntimeConverter` trait definition | `crates/memory-installer/src/converter.rs` |
| `tool_maps.rs` | Static tool name mapping tables per runtime | `crates/memory-installer/src/tool_maps.rs` |
| `converters/claude.rs` | Pass-through: copy canonical source, rewrite storage paths | `crates/memory-installer/src/converters/claude.rs` |
| `converters/opencode.rs` | Flatten commands, tools object, PascalCase→lowercase, path rewrite | `crates/memory-installer/src/converters/opencode.rs` |
| `converters/gemini.rs` | YAML→TOML, tool snake_case, strip color/skills, escape `${VAR}`, strip HTML | `crates/memory-installer/src/converters/gemini.rs` |
| `converters/codex.rs` | Commands→skill dirs, agents→orchestration skills, AGENTS.md generation | `crates/memory-installer/src/converters/codex.rs` |
| `converters/copilot.rs` | Skills copy, `.agent.md` generation, `.github/` hook format | `crates/memory-installer/src/converters/copilot.rs` |
| `converters/skills.rs` | Generic base: commands→skill dirs, agents→skill dirs, no runtime transforms | `crates/memory-installer/src/converters/skills.rs` |
| `hooks.rs` | Per-runtime hook format conversions | `crates/memory-installer/src/hooks.rs` |
| `main.rs` | Standalone `memory-installer` binary entry with clap CLI | `crates/memory-installer/src/main.rs` |

---

## Binary Architecture Decision: Standalone Crate, Not Subcommand

**Decision: New `memory-installer` standalone binary, not a subcommand of `memory-daemon`.**

**Rationale:**

1. `memory-daemon` already has a `Clod` subcommand compiled with heavy dependencies (tokio, tonic, RocksDB, HNSW, Candle). Adding `install-agent` would link all those libraries into a tool that just copies and transforms files.

2. The installer is a pure filesystem tool — no daemon process needed, no gRPC, no async runtime required. Forcing it into the daemon creates a spurious async context for a synchronous operation.

3. Standalone binary fits the existing split: `memory-daemon` serves the API, `memory-ingest` handles hook ingestion, `memory-installer` handles agent setup. Three binaries, three responsibilities.

4. The existing `memory-daemon clod convert` subcommand (in `crates/memory-daemon/src/clod.rs`) is a CLOD-to-adapter prototype built in v2.1. `memory-installer` supersedes it. The `Clod` CLI variant is retired in Phase 50.

5. If users want `memory-daemon install-agent` as a convenience alias, a thin `Commands::InstallAgent` variant can delegate via `std::process::Command` to `memory-installer` without coupling implementation to the daemon crate.

---

## Recommended Project Structure

```
crates/memory-installer/
├── Cargo.toml
└── src/
    ├── main.rs              # binary entry, clap CLI, dispatches to install()
    ├── lib.rs               # pub re-exports for integration tests
    ├── types.rs             # PluginBundle, PluginCommand, PluginAgent, PluginSkill,
    │                        #   HookDefinition, ConvertedFile, InstallScope
    ├── parser.rs            # parse_plugin_dir() → Result<PluginBundle>
    │                        #   uses walkdir + serde_yaml for frontmatter
    ├── converter.rs         # RuntimeConverter trait
    ├── tool_maps.rs         # static mapping tables per runtime
    ├── hooks.rs             # HookConverter, per-runtime hook format logic
    └── converters/
        ├── mod.rs           # re-exports all converters, select_converter()
        ├── claude.rs        # ClaudeConverter: pass-through with path rewrite
        ├── opencode.rs      # OpenCodeConverter: flatten, tools object, name mapping
        ├── gemini.rs        # GeminiConverter: TOML format, snake_case tools
        ├── codex.rs         # CodexConverter: commands→skills, AGENTS.md
        ├── copilot.rs       # CopilotConverter: .github/ format, .agent.md
        └── skills.rs        # SkillsConverter: generic base, delegate for Codex
```

### Structure Rationale

- **`converters/` subdirectory:** Each runtime is one file. Adding a new runtime is one new file plus one line in `mod.rs`. No modification to other converters.
- **`tool_maps.rs` separate from converters:** Mappings are data, not logic. Tests can validate mapping tables independently. Each converter imports only the table it needs.
- **`types.rs` separate from `parser.rs`:** Types are stable; parser implementation evolves. Integration tests import types without pulling in `walkdir`.
- **`hooks.rs` separate module:** Hook conversion has its own complexity per runtime. Keeps converters focused on command/agent/skill transformation.
- **`lib.rs` with re-exports:** Enables `crates/e2e-tests` to import installer types for round-trip testing without binary invocation.

---

## Architectural Patterns

### Pattern 1: RuntimeConverter Trait

**What:** Each runtime implements one trait. The CLI dispatches to the correct impl at runtime via `Box<dyn RuntimeConverter>`.

**When to use:** Any time a new runtime target needs to be added. Implement the trait, register in `converters/mod.rs`, add to CLI dispatch.

**Trade-offs:** Trait objects add a small vtable overhead. For a filesystem tool that runs once, this is irrelevant. The ergonomics of adding a new runtime cleanly outweigh the theoretical cost.

**Trait definition:**

```rust
pub trait RuntimeConverter {
    fn name(&self) -> &str;
    fn target_dir(&self, scope: &InstallScope) -> PathBuf;
    fn convert_command(&self, cmd: &PluginCommand) -> Vec<ConvertedFile>;
    fn convert_agent(&self, agent: &PluginAgent) -> Vec<ConvertedFile>;
    fn convert_skill(&self, skill: &PluginSkill) -> Vec<ConvertedFile>;
    fn convert_hook(&self, hook: &HookDefinition) -> Option<ConvertedFile>;
    fn generate_guidance(&self, bundle: &PluginBundle) -> Vec<ConvertedFile>;
}
```

Note: `convert_command` and `convert_skill` return `Vec<ConvertedFile>` (not singular) because the Codex converter produces a skill directory (multiple files) from one command.

### Pattern 2: Frontmatter Parse → Transform → Serialize

**What:** Parse YAML frontmatter from canonical `.md` files, modify fields in a `BTreeMap<String, serde_yaml::Value>`, serialize back. The body (content after the second `---`) is preserved and optionally post-processed.

**When to use:** Every converter that transforms command or agent files.

**Evidence from existing code:** The GSD `frontmatter.cjs` uses the same parse/transform/serialize loop. The existing `clod.rs` in `memory-daemon` uses manual string building — the new `memory-installer` should use `serde_yaml` for roundtrip correctness instead.

**Implementation note:** Use `serde_yaml::Value` (not typed structs) for the frontmatter map to allow arbitrary field manipulation without defining structs for each runtime's format.

```rust
pub struct ParsedFile {
    pub frontmatter: BTreeMap<String, serde_yaml::Value>,
    pub body: String,
    pub source_path: PathBuf,
}

pub fn parse_md_file(path: &Path) -> Result<ParsedFile> {
    // Split on first two `---` delimiters
    // Parse YAML block with serde_yaml::from_str
    // Return body as-is
}

pub fn serialize_md_file(fm: &BTreeMap<String, serde_yaml::Value>, body: &str) -> String {
    // serde_yaml::to_string(&fm) + "---\n" + body
}
```

### Pattern 3: Tool Mapping as Static Tables

**What:** Tool name mappings are `HashMap<&str, &str>` initialized once (via `lazy_static!` or `OnceLock`). Each converter calls `tool_maps::map_tool(Runtime::Gemini, &tool_name)`.

**When to use:** Any converter that rewrites tool names in `allowed-tools:` frontmatter arrays or skill body text.

**Evidence:** The v2.7 plan provides the full mapping table (11 tools across 4 runtimes). This is stable data, not logic. Centralizing prevents per-converter drift.

**Mapping table (from v2.7 plan):**

| Claude | OpenCode | Gemini | Codex/Copilot |
|--------|----------|--------|---------------|
| Read | read | read_file | read |
| Write | write | write_file | edit |
| Edit | edit | replace | edit |
| Bash | bash | run_shell_command | execute |
| Grep | grep | search_file_content | search |
| Glob | glob | glob | search |
| WebSearch | websearch | google_web_search | web |
| WebFetch | webfetch | web_fetch | web |
| TodoWrite | todowrite | write_todos | todo |
| AskUserQuestion | question | ask_user | ask_user |
| Task | task | *(excluded)* | agent |

### Pattern 4: Canonical Source as Filesystem, Not Embedded Binary

**What:** The canonical plugin source (`plugins/memory-plugin/`) is read from the filesystem at install time, not embedded via `include_str!`. The installer discovers the source directory via:
1. `--source <path>` CLI flag (explicit override)
2. Heuristic search: walk up from `$cwd` looking for `plugins/memory-plugin/`
3. Installed location: `~/.local/share/agent-memory/plugins/memory-plugin/` (post-cargo-install)

**Why filesystem over `include_str!`:**
- The canonical source evolves (new commands, updated skills). Rebuilding the binary for every skill edit is wrong.
- `include_str!` only works for files known at compile time. A canonical directory with an arbitrary number of files requires `include_dir` crate or a tar bundle — both add complexity for no benefit.
- The installer is a developer/admin tool, not an end-user appliance. Filesystem access is expected.

**Caveat:** A `--embedded` flag could be added post-v2.7 using an `include_dir!`-bundled snapshot for standalone distribution. This is not needed for the initial milestone.

### Pattern 5: InstallScope as Value Type, Not State

**What:** `InstallScope` is an enum passed as an argument to `target_dir()`, not stored in the converter struct. Converters are stateless and testable without setup.

```rust
pub enum InstallScope {
    Project(PathBuf),    // ./.claude/plugins/ relative to project root
    Global,              // ~/.claude/plugins/ etc.
    Custom(PathBuf),     // --dir <path> (required for Generic/Skills converter)
}
```

---

## Data Flow

### Install Flow

```
CLI parse
    ↓
  --agent <runtime>, --project|--global, --source <path>, --dry-run
    ↓
parse_plugin_dir(source_path)
    → walk commands/, agents/, skills/, hooks/
    → parse_md_file() for each .md
    → return PluginBundle { commands, agents, skills, hooks }
    ↓
select_converter(runtime) → Box<dyn RuntimeConverter>
    ↓
for each cmd in bundle.commands:
    converter.convert_command(&cmd) → Vec<ConvertedFile>
for each agent in bundle.agents:
    converter.convert_agent(&agent) → Vec<ConvertedFile>
for each skill in bundle.skills:
    converter.convert_skill(&skill) → Vec<ConvertedFile>
for each hook in bundle.hooks:
    converter.convert_hook(&hook) → Option<ConvertedFile>
converter.generate_guidance(&bundle) → Vec<ConvertedFile>
    ↓
if dry_run:
    print file paths and diffs only
else:
    write_files(all_converted_files, target_dir)
    ↓
print install summary (N files installed to <path>)
```

### Frontmatter Transform Flow (per-converter)

```
ParsedFile { frontmatter: BTreeMap<String, Value>, body: String }
    ↓
1. Clone frontmatter into mutable copy
2. map tool names in allowed-tools array (via tool_maps)
3. rename/remove/add runtime-specific fields
4. post-process body (escape ${VAR}, strip HTML, etc.)
    ↓
ConvertedFile { path: PathBuf, content: String }
```

### Hook Conversion Flow

```
HookDefinition { event_types, script_path, ... }
    ↓
HookConverter::for_runtime(runtime)
    ↓
Claude:   → .claude/hooks/<name>.yaml
OpenCode: → .opencode/plugin/index.ts (TypeScript event listener injection)
Gemini:   → .gemini/settings.json merge (JSON)
Copilot:  → .github/hooks/<name>.json
Codex:    → guidance text only (no hook support)
Generic:  → shell wrapper script
```

---

## Integration Points with Existing Workspace

### New: What Gets Added

| Item | Type | Description |
|------|------|-------------|
| `crates/memory-installer/` | New crate | All installer source code |
| `plugins/memory-plugin/` | New directory | Consolidated canonical source (merged query + setup plugins) |
| `Cargo.toml` workspace `members` | Modified | Add `crates/memory-installer` |
| `.github/workflows/ci.yml` | Modified | Add `memory-installer` to clippy/test/doc matrix |

### Modified: Existing Components Touched

| Item | Change | Phase |
|------|--------|-------|
| `crates/memory-daemon/src/clod.rs` | Deprecated → deleted | Phase 50 |
| `crates/memory-daemon/src/cli.rs` | Remove `Clod` Commands variant | Phase 50 |
| `plugins/memory-query-plugin/` | Content merged into `plugins/memory-plugin/`, archived | Phase 45 |
| `plugins/memory-setup-plugin/` | Content merged into `plugins/memory-plugin/`, archived | Phase 45 |
| `plugins/memory-copilot-adapter/` | Archived (replaced by installer output) | Phase 50 |
| `plugins/memory-gemini-adapter/` | Archived (replaced by installer output) | Phase 50 |
| `plugins/memory-opencode-plugin/` | Archived (replaced by installer output) | Phase 50 |

### No Change Required

| Item | Why |
|------|-----|
| `crates/memory-daemon/` (runtime behavior) | Installer is a build-time/install-time tool, not runtime |
| `crates/memory-storage/` | Installer does not touch RocksDB |
| `crates/memory-service/` | Installer does not use gRPC |
| `proto/` | No new RPC definitions needed |
| `crates/e2e-tests/` (existing tests) | Existing E2E tests unchanged; installer tests added separately |

### Internal Boundary: memory-installer vs memory-daemon

The installer has NO runtime dependency on `memory-daemon`. It is a standalone filesystem tool. The relationship is:

- `memory-installer` reads `plugins/memory-plugin/` (source of truth at install time)
- `memory-daemon` serves the backend that installed plugins connect to
- The two binaries share no Rust crate code at runtime

The only shared concern is the path convention for `~/.config/agent-memory/` (runtime-neutral storage). Recommendation: extract the path constant into `memory-types` as a `const` to prevent drift between installer and daemon.

---

## Per-Runtime Converter Specifics

### Claude Converter (Pass-Through)

Minimal transforms. Source and target formats are identical — Claude plugin is the canonical format.

- Copy `commands/*.md` preserving frontmatter as-is
- Copy `agents/*.md` preserving frontmatter as-is
- Copy `skills/*/SKILL.md` and `skills/*/references/` recursively
- Copy `hooks/` preserving YAML format
- Rewrite storage path references from relative to `~/.config/agent-memory/`
- Target: `~/.claude/plugins/memory-plugin/` (global) or `.claude/plugins/memory-plugin/` (project)

### OpenCode Converter

Key transforms derived from v2.7 plan and GSD superpowers reference implementation:

- **Directory:** `commands/` → `command/` (flat, no namespace subdirectory)
- **Tool names:** `allowed-tools: [Read, Write, Bash]` → `tools: {read: true, write: true, bash: true}` (YAML object with boolean values, not array)
- **Command names:** `/memory:search` → `/memory-search` (namespace colon → hyphen)
- **Colors:** Named CSS colors → hex values (OpenCode requires hex)
- **Paths:** `~/.claude/` → `~/.config/opencode/`
- **Strip `name:` field** from command frontmatter (OpenCode infers name from filename)
- **Permissions:** Generate `opencode.json` with `files.read = true` and tool permissions
- **Skills:** Copy to `~/.config/opencode/skills/` maintaining SKILL.md structure
- Target: `~/.config/opencode/` (global) or `.opencode/` (project)

### Gemini Converter

Source format: YAML frontmatter Markdown. Target format: TOML with `[prompt]` section.

- **Format transform:** Entire command becomes a `.toml` file with `[prompt]` block
- **Tool names:** PascalCase → `snake_case` Gemini names via `CLAUDE_TO_GEMINI` map
- **Strip fields:** `color:`, `skills:` (no equivalent in Gemini format)
- **Escape:** `${VAR}` → `$VAR` (Gemini template engine conflicts with `${}` syntax)
- **Strip HTML:** `<sub>`, `</sub>` and other inline HTML tags
- **Agent transform:** `allowed-tools:` list → `tools:` array, exclude `Task` tool entirely
- **Hooks:** JSON merge into `.gemini/settings.json` (not separate files)
- Target: `~/.gemini/` (global) or `.gemini/` (project)

### Codex Converter

Codex has no hook support. Each command becomes a skill directory.

- **Commands → Skills:** `commands/memory-search.md` → `.codex/skills/memory-search/SKILL.md`
- **Agents → Orchestration Skills:** Agent becomes a large SKILL.md with full capability description
- **AGENTS.md generation:** One `AGENTS.md` at target root listing all installed agents/skills
- **Sandbox permissions:** Map `allowed-tools:` to Codex sandbox level (`workspace-write` vs `read-only`)
- **Tool names:** Via `CLAUDE_TO_CODEX` map
- **Delegate to SkillsConverter** for the commands→skills structural transformation, then extend with AGENTS.md
- Target: `~/.codex/skills/` (global) or `.codex/skills/` (project)

### Generic Skills Converter

Base implementation used directly by `--agent skills` and as a delegate by `CodexConverter`.

- Commands → skill directories with `SKILL.md`
- Agents → orchestration skill directories with `SKILL.md`
- Skills → copied directly (no structural change)
- No runtime-specific field transforms beyond path rewriting
- Target: user-specified `--dir <path>` (required, no default)

---

## Build Order

v2.7 phases must be built in this order due to dependency constraints:

**Dependency graph:**

```
Phase 45 (canonical source consolidation — no Rust code)
    ↓
Phase 46 (crate scaffolding + parser + converter trait)
    ↓                ↓
Phase 47          Phase 48
(claude+opencode) (gemini+codex)
    ↓                ↓
         Phase 49 (generic skills + hook pipeline)
              ↓
         Phase 50 (integration testing + migration)
```

**Rationale per phase:**

1. **Phase 45 first:** Canonical source consolidation produces no Rust code but creates the input all converters read. Nothing else can be validated without it.

2. **Phase 46 next:** Crate scaffolding, parser, and converter trait. Defines `PluginBundle` and `RuntimeConverter` that Phases 47, 48, 49 all depend on. Cannot parallelize earlier.

3. **Phases 47 and 48 in parallel:** Claude and OpenCode converters (47) are independent from Gemini and Codex converters (48) after Phase 46 completes. Separate agents can work these in parallel.

4. **Phase 49 after 47 and 48:** The generic `SkillsConverter` is the structural base for `CodexConverter`. Hook pipeline design benefits from seeing all converter patterns first. Requires phases 47 + 48 to be complete to extract any shared patterns.

5. **Phase 50 last:** Integration testing requires all converters complete. Archive old adapters only after tests confirm installer produces equivalent output.

---

## Scaling Considerations

| Concern | Current Scale | At Scale |
|---------|---------------|----------|
| Canonical source size | 6 commands + 13 skills = ~19 files | 50+ commands: `--filter` flag to install subset |
| New runtime support | 6 runtimes | One new `impl RuntimeConverter` file, no existing code changes |
| Converter test surface | 19 files × 6 runtimes = ~114 output files | Generate test fixtures from reference installs |
| Plugin discovery | Heuristic walk-up from cwd | Extend to `~/.local/share/agent-memory/plugins/` for installed binaries |

---

## Anti-Patterns

### Anti-Pattern 1: Put Install Logic in memory-daemon

**What people do:** Add `memory-daemon install-agent` as a subcommand because it is convenient.

**Why it's wrong:** Requires compiling tokio, tonic, RocksDB, HNSW, Candle into a tool that copies and transforms files. The existing `memory-daemon clod convert` subcommand is the cautionary example — it is a TOML-to-Markdown prototype that predates the full installer and needs to be retired, not expanded.

**Do this instead:** Standalone `memory-installer` binary. If users want `memory-daemon install-agent` as a convenience, add a thin `Commands::InstallAgent` variant that delegates via `std::process::Command` — without pulling installer source into the daemon crate.

### Anti-Pattern 2: Embed Canonical Source via include_str!

**What people do:** Use `include_str!("../../plugins/memory-plugin/commands/memory-search.md")` so the binary works without the source tree.

**Why it's wrong:** Every skill edit requires a binary rebuild. The `include_dir!` macro alternative bundles a complete static snapshot — this works but breaks the plugin iteration cycle during development.

**Do this instead:** Read from filesystem at install time. Use `--source <path>` to override discovery. For distribution, ship the binary and the `plugins/memory-plugin/` directory together as a release artifact (natural for a workspace build).

### Anti-Pattern 3: One Monolithic Converter Function

**What people do:** Write a single `convert_all(bundle, runtime)` function with a large `match runtime` block.

**Why it's wrong:** Adding a new runtime requires editing the monolithic function. Tests for one runtime can break another through shared state. The `SkillsConverter` + `CodexConverter` delegation relationship (Codex calls Skills as a sub-converter) cannot work with a monolithic function.

**Do this instead:** One struct per runtime implementing `RuntimeConverter`. `CodexConverter` holds a `SkillsConverter` field and calls its `convert_command`/`convert_skill` methods, then adds AGENTS.md generation on top.

### Anti-Pattern 4: Inline Tool Name Strings in Each Converter

**What people do:** Write `"read_file"` directly in `gemini.rs`, `"read"` directly in `opencode.rs`.

**Why it's wrong:** When a runtime updates its tool names, finding all occurrences is error-prone. 11 tools × 6 runtimes = 66 string literals scattered across files.

**Do this instead:** Centralize in `tool_maps.rs` as a static lookup. Each converter calls `map_tool(Runtime::Gemini, &tool_name)` which returns the mapped name or the original if not found.

---

## Integration Points Summary

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Claude runtime | Write files to `~/.claude/plugins/` or `.claude/plugins/` | Standard YAML frontmatter Markdown format |
| OpenCode runtime | Write files to `~/.config/opencode/` or `.opencode/` | Flat commands, tools object, opencode.json permissions |
| Gemini runtime | Write TOML files + merge `settings.json` | TOML `[prompt]` format, JSON hook merge |
| Codex runtime | Write skill dirs + AGENTS.md | No hook support |
| Copilot runtime | Write skills + `.github/` hooks | `.agent.md` guidance file |
| Generic (any skill runtime) | Write to user-specified `--dir` | Pure SKILL.md directories |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `memory-installer` ↔ filesystem | Direct file I/O via `std::fs` | No gRPC, no async required |
| `memory-installer` ↔ `memory-types` | Shared path const for `~/.config/agent-memory/` | Add const to `memory-types` to prevent drift |
| `memory-installer` ↔ `memory-daemon` | None at runtime | Installer runs once at setup time; daemon runs continuously |
| `crates/e2e-tests` ↔ `memory-installer` | Import `memory-installer` as library (via `lib.rs`) | Round-trip tests: install to temp dir, verify file structure |

---

## Sources

- `/Users/richardhightower/clients/spillwave/src/agent-memory/crates/memory-daemon/src/clod.rs` — Existing CLOD converter (patterns to supersede, HIGH confidence)
- `/Users/richardhightower/clients/spillwave/src/agent-memory/crates/memory-daemon/src/cli.rs` — Existing Commands enum showing current subcommand pattern (HIGH confidence)
- `/Users/richardhightower/clients/spillwave/src/agent-memory/plugins/memory-query-plugin/` — Canonical Claude source format with actual frontmatter structure (HIGH confidence)
- `/Users/richardhightower/clients/spillwave/src/agent-memory/plugins/memory-setup-plugin/` — Second canonical plugin, agent and skill patterns (HIGH confidence)
- `/Users/richardhightower/clients/spillwave/src/agent-memory/docs/plans/v2.7-multi-runtime-portability-plan.md` — Authoritative milestone plan with tool mapping tables (HIGH confidence)
- `/Users/richardhightower/clients/spillwave/src/agent-memory/.planning/PROJECT.md` — Key decisions and workspace constraints (HIGH confidence)
- `/Users/richardhightower/.claude/get-shit-done/bin/lib/frontmatter.cjs` — GSD parse/transform/serialize pattern for YAML frontmatter (MEDIUM confidence — JavaScript reference, not Rust)
- `/Users/richardhightower/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.1/.opencode/plugins/superpowers.js` — OpenCode plugin format reference (MEDIUM confidence)

---

*Architecture research for: multi-runtime plugin installer (agent-memory v2.7)*
*Researched: 2026-03-16*
