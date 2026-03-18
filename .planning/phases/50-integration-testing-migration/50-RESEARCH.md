# Phase 50: Integration Testing & Migration - Research

**Researched:** 2026-03-18
**Domain:** Rust integration testing, file structure verification, adapter archival
**Confidence:** HIGH

## Summary

Phase 50 is the final phase of v2.7. It needs three things: (1) E2E integration tests that exercise all 6 converters with a canonical test bundle and verify both file structure and content correctness, (2) archival of 3 old adapter directories with README stubs, and (3) CI verification that memory-installer is already covered.

The codebase is well-positioned for this. All 6 converters are implemented (though OpenCode is still a stub returning empty vectors). The `tempfile` crate is already a dev-dependency. There are 104+ unit tests in the crate covering individual converter methods, but no integration tests exist yet -- the `tests/` directory does not exist. The `select_converter` function and public types make it straightforward to write full-bundle E2E tests. CI already includes memory-installer via `--workspace` flags, so MIG-04 is satisfied.

**Primary recommendation:** Create `crates/memory-installer/tests/e2e_converters.rs` with a shared canonical `PluginBundle` fixture, then test each of the 5 implemented converters (skip OpenCode) for file paths, frontmatter format, and content correctness. Archive 3 adapter directories but preserve `plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` (required by `include_str!`).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- E2E tests should install to temp directories (use `tempdir` crate or `std::env::temp_dir`) for each of the 6 runtimes (Claude, Codex, Gemini, Copilot, OpenCode, Skills)
- Tests verify file structure: correct directories, file names, file extensions
- Tests verify frontmatter conversion: tool name mapping per runtime, YAML-to-TOML for Gemini, field transformations
- E2E tests should use a small canonical test bundle (PluginBundle with 1 command, 1 agent, 1 skill, 1 hook) to keep tests focused
- Tests belong in `crates/memory-installer/tests/` (integration tests) not in the unit test modules
- Old directories to archive: `plugins/memory-copilot-adapter/`, `plugins/memory-gemini-adapter/`, `plugins/memory-opencode-plugin/`
- Keep `plugins/memory-query-plugin/` and `plugins/memory-setup-plugin/` (still active)
- Archive means: replace contents with a single README.md stub pointing users to `memory-installer`
- Do NOT delete the directories -- keep them with README stubs for one release cycle
- The `plugins/installer-sources.json` file should remain
- CI already uses `--workspace` flags which automatically includes `memory-installer`

### Claude's Discretion
- Test helper structure and shared fixtures
- Whether to use `assert_cmd` or direct Rust function calls for E2E tests
- Exact wording of README archive stubs
- Whether to include `include_str!` hook script tests in E2E scope

### Deferred Ideas (OUT OF SCOPE)
- MIG-F01: Delete archived adapter directories after one release cycle (v2.8+)
- INST-F01: Interactive mode with runtime selection prompts
- INST-F02: `--uninstall` command
- INST-F03: `--all` flag for all runtimes
- INST-F04: Version tracking with upgrade detection
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MIG-01 | E2E tests verify install-to-temp-dir produces correct file structure per runtime | All 6 converters analyzed; file paths documented per runtime below; `tempfile` already in dev-deps; `select_converter` + `RuntimeConverter` trait make direct function calls easy |
| MIG-02 | E2E tests verify frontmatter conversion correctness (tool names, format, fields) | Tool map analysis complete; Gemini TOML format documented; Copilot camelCase hooks documented; Skills pass-through documented |
| MIG-03 | Old adapter directories archived with README stubs pointing to `memory-installer` | All 3 directories inventoried; `include_str!` dependency on copilot hook script identified and documented |
| MIG-04 | Installer added to workspace CI (build, clippy, test) | VERIFIED: memory-installer already in root Cargo.toml workspace members; CI uses `--workspace` for all 4 checks |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tempfile | workspace | Temp directory creation for E2E tests | Already in dev-dependencies; `TempDir` auto-cleans |
| serde_json | workspace | JSON assertions for hooks/settings output | Already in dependencies |
| toml | workspace | TOML parsing assertions for Gemini output | Already in dependencies |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| pretty_assertions | workspace | Better diff output on test failures | If already in workspace; not critical |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Direct function calls | `assert_cmd` (CLI binary testing) | Direct calls are simpler -- no binary spawn overhead, tests the library API directly which is what matters |
| `tempfile::TempDir` | `std::env::temp_dir()` | TempDir auto-cleans; manual temp requires cleanup code |
| `assert_fs` | `tempfile` + `std::fs` | assert_fs adds another dependency; tempfile is already available |

**Recommendation:** Use direct Rust function calls with `tempfile::TempDir`, not `assert_cmd`. The converters are library functions -- testing them directly is more precise and faster.

## Architecture Patterns

### Test File Structure
```
crates/memory-installer/
  tests/
    e2e_converters.rs     # Full-bundle conversion tests for all runtimes
```

A single integration test file is sufficient. Each runtime gets its own test function (or small group). A shared `canonical_bundle()` helper provides the test fixture.

### Pattern 1: Canonical Test Bundle Factory
**What:** A function returning a `PluginBundle` with exactly 1 command, 1 agent, 1 skill (with 1 additional file), and 1 hook.
**When to use:** Every E2E test calls this to get consistent input.
**Example:**
```rust
fn canonical_bundle() -> PluginBundle {
    PluginBundle {
        commands: vec![PluginCommand {
            name: "memory-search".to_string(),
            frontmatter: serde_json::json!({
                "description": "Search past conversations"
            }),
            body: "Search for things in ~/.claude/data".to_string(),
            source_path: PathBuf::from("commands/memory-search.md"),
        }],
        agents: vec![PluginAgent {
            name: "memory-navigator".to_string(),
            frontmatter: serde_json::json!({
                "description": "Navigate memory",
                "allowed-tools": ["Read", "Bash", "Grep", "mcp__memory", "Task"]
            }),
            body: "Navigate through ~/.claude/skills for lookup".to_string(),
            source_path: PathBuf::from("agents/memory-navigator.md"),
        }],
        skills: vec![PluginSkill {
            name: "memory-query".to_string(),
            frontmatter: serde_json::json!({"description": "Query skill"}),
            body: "Query ~/.claude/data".to_string(),
            source_path: PathBuf::from("skills/memory-query/SKILL.md"),
            additional_files: vec![SkillFile {
                relative_path: PathBuf::from("rules/search.md"),
                content: "Rule: use ~/.claude/db for searches".to_string(),
            }],
        }],
        hooks: vec![HookDefinition {
            name: "session-start".to_string(),
            frontmatter: serde_json::json!({"event": "session_start"}),
            body: "Hook body".to_string(),
            source_path: PathBuf::from("hooks/session-start.md"),
        }],
    }
}
```

### Pattern 2: Per-Runtime Conversion + Assertion
**What:** For each runtime, call all converter methods, collect files, verify paths and content.
**When to use:** Each test function follows this pattern.
**Example:**
```rust
#[test]
fn claude_full_bundle_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = InstallConfig {
        scope: InstallScope::Project(dir.path().to_path_buf()),
        dry_run: false,
        source_root: PathBuf::from("/unused"),
    };
    let bundle = canonical_bundle();
    let converter = select_converter(Runtime::Claude);

    let mut all_files: Vec<ConvertedFile> = Vec::new();
    for cmd in &bundle.commands {
        all_files.extend(converter.convert_command(cmd, &cfg));
    }
    for agent in &bundle.agents {
        all_files.extend(converter.convert_agent(agent, &cfg));
    }
    // ... skills, hooks, guidance
    all_files.extend(converter.generate_guidance(&bundle, &cfg));

    // Write to temp dir
    write_files(&all_files, false).unwrap();

    // Assert file existence
    assert!(dir.path().join(".claude/plugins/memory-plugin/commands/memory-search.md").exists());
    // Assert content
    let content = std::fs::read_to_string(...).unwrap();
    assert!(content.contains("~/.config/agent-memory/"));
}
```

### Anti-Patterns to Avoid
- **Testing converter output in isolation again:** Unit tests already cover individual methods. E2E tests should verify the full pipeline (bundle -> convert all -> write -> verify on disk).
- **Testing OpenCode as if implemented:** OpenCode converter is a stub returning empty Vecs. Test that it produces empty output, don't assert file creation.
- **Hardcoding paths without using converter's target_dir:** Always derive expected paths from the converter to avoid fragile tests.

## Expected Output Per Runtime

### Claude
| Artifact | Path Pattern | Format |
|----------|-------------|--------|
| Command | `.claude/plugins/memory-plugin/commands/{name}.md` | YAML frontmatter |
| Agent | `.claude/plugins/memory-plugin/agents/{name}.md` | YAML frontmatter |
| Skill | `.claude/plugins/memory-plugin/skills/{name}/SKILL.md` | YAML frontmatter |
| Guidance | (none) | No guidance files |

### Codex
| Artifact | Path Pattern | Format |
|----------|-------------|--------|
| Command | `.codex/skills/{name}/SKILL.md` | YAML frontmatter |
| Agent | `.codex/skills/{name}/SKILL.md` | YAML frontmatter + Tools + Sandbox sections |
| Skill | `.codex/skills/{name}/SKILL.md` + additional files | YAML frontmatter |
| Guidance | `.codex/AGENTS.md` | Markdown with skills list and agent descriptions |

### Gemini
| Artifact | Path Pattern | Format |
|----------|-------------|--------|
| Command | `.gemini/commands/{name}.toml` | TOML (description + prompt) |
| Agent | `.gemini/skills/{name}/SKILL.md` | YAML frontmatter (no color/skills fields) |
| Skill | `.gemini/skills/{name}/SKILL.md` + additional files | YAML frontmatter |
| Guidance | `.gemini/settings.json` | JSON with hooks and managed markers |

### Copilot
| Artifact | Path Pattern | Format |
|----------|-------------|--------|
| Command | `.github/skills/{name}/SKILL.md` | YAML frontmatter |
| Agent | `.github/agents/{name}.agent.md` | YAML frontmatter with tools array + infer: true |
| Skill | `.github/skills/{name}/SKILL.md` + additional files | YAML frontmatter |
| Guidance | `.github/hooks/memory-hooks.json` + `.github/hooks/scripts/memory-capture.sh` | JSON hooks + bash script |

### Skills
| Artifact | Path Pattern | Format |
|----------|-------------|--------|
| Command | `skills/{name}/SKILL.md` | YAML frontmatter |
| Agent | `skills/{name}/SKILL.md` | YAML frontmatter + Tools (Claude names, no remap) |
| Skill | `skills/{name}/SKILL.md` + additional files | YAML frontmatter |
| Guidance | (none) | No guidance files |

### OpenCode (STUB)
| Artifact | Path Pattern | Format |
|----------|-------------|--------|
| All | (none) | Returns empty Vec -- stub only |

## Key Verification Points Per Runtime

### Claude
- Path rewriting: `~/.claude/` -> `~/.config/agent-memory/`
- Frontmatter preserved as-is (pass-through)

### Codex
- Tool deduplication (Write + Edit both -> "edit", deduped)
- Sandbox: `setup-troubleshooter` gets `workspace-write`, others `read-only`
- AGENTS.md contains skills list and agent entries

### Gemini
- Commands produce TOML (not YAML)
- `${HOME}` escaped to `$HOME` (shell var escaping)
- Agent frontmatter strips `color:` and `skills:` fields
- `Task` tool excluded (maps to None)
- settings.json contains `__managed_by` marker and 6 hook events (PascalCase)

### Copilot
- Agent files named `{name}.agent.md`
- Frontmatter includes `infer: true` and `tools:` array
- Hooks JSON has camelCase event names (sessionStart, not SessionStart)
- Hook entries use `bash`/`timeoutSec`/`comment` fields (not Gemini's `command`/`timeout`/`description`)
- memory-capture.sh script is non-empty and contains `trap` + `exit 0`

### Skills
- Tool names are canonical Claude names (Read, Bash, Grep -- NOT remapped)
- MCP tools excluded
- No guidance files generated

### OpenCode
- All methods return empty Vecs (stub test)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directory lifecycle | Manual mkdir/cleanup | `tempfile::TempDir` | Auto-cleanup on drop; already in dev-deps |
| TOML parsing for assertions | String matching | `toml::from_str` | Structural verification, not brittle string matching |
| JSON parsing for assertions | String matching | `serde_json::from_str` | Same reason |
| YAML frontmatter extraction | Regex parsing | `gray_matter` (already in deps) | Handles edge cases; already used by the crate |

## Common Pitfalls

### Pitfall 1: include_str! Compile Failure After Archival
**What goes wrong:** Archiving `plugins/memory-copilot-adapter/` deletes `memory-capture.sh`, breaking `include_str!` in `copilot.rs` which resolves at compile time.
**Why it happens:** `include_str!` path is `../../../../plugins/memory-copilot-adapter/.github/hooks/scripts/memory-capture.sh` -- relative from source file.
**How to avoid:** When archiving the copilot adapter, preserve the `.github/hooks/scripts/memory-capture.sh` file. Only replace top-level README.md and other non-essential files with the archive stub.
**Warning signs:** `cargo build` fails with "file not found" error pointing at the include_str! macro.

### Pitfall 2: OpenCode Stub Tests Asserting Non-Empty Output
**What goes wrong:** Writing tests that expect OpenCode to produce files, then failing because it's a stub.
**Why it happens:** CONTEXT.md says "test all 6 runtimes" but OpenCode is not implemented yet (OC-01 through OC-06 are Pending in REQUIREMENTS.md).
**How to avoid:** For OpenCode, test that the converter name is correct and output is empty. Do not assert file creation.
**Warning signs:** Test expects files that don't exist.

### Pitfall 3: Brittle Path Assertions on Windows
**What goes wrong:** Tests use forward-slash string comparisons that fail on Windows.
**Why it happens:** `PathBuf` uses OS-native separators.
**How to avoid:** Compare `PathBuf` values, not string representations. Or use `.ends_with()` on path components.
**Warning signs:** Tests pass on macOS/Linux, fail on Windows CI.

### Pitfall 4: Archiving .gitignore Files
**What goes wrong:** Each adapter directory has a `.gitignore`. If removed, git may start tracking generated artifacts.
**Why it happens:** Archival replaces all files with just a README stub.
**How to avoid:** Either keep the `.gitignore` or ensure the README stub is the only file and no generated content exists.

## Adapter Archival Details

### plugins/memory-copilot-adapter/ (17 files)
**Must preserve:**
- `.github/hooks/scripts/memory-capture.sh` -- required by `include_str!` in `copilot.rs`

**Can replace/remove:**
- `README.md` (replace with archive stub)
- `plugin.json`
- `.gitignore`
- `.github/agents/memory-navigator.agent.md`
- `.github/hooks/memory-hooks.json`
- `.github/skills/**` (6 skill directories with SKILL.md + references)

**Recommended approach:** Delete everything except `.github/hooks/scripts/memory-capture.sh`, add archive `README.md` at root.

### plugins/memory-gemini-adapter/ (18 files)
**Can replace/remove (all):**
- `README.md` (replace with archive stub)
- `.gitignore`
- `.gemini/**` (settings.json, hooks, commands, skills)

**Recommended approach:** Delete everything, add archive `README.md`.

### plugins/memory-opencode-plugin/ (17 files)
**Can replace/remove (all):**
- `README.md` (replace with archive stub)
- `.gitignore`
- `.opencode/**` (commands, agents, skills, plugin)

**Recommended approach:** Delete everything, add archive `README.md`.

## CI Integration Status

**MIG-04 is already satisfied.** Evidence:

1. Root `Cargo.toml` workspace members includes `"crates/memory-installer"`
2. CI `ci.yml` uses `--workspace` for:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test --workspace --all-features --exclude e2e-tests`
   - `cargo build --release --workspace`
   - `cargo doc --no-deps --workspace --all-features`

Integration tests in `crates/memory-installer/tests/` will be automatically picked up by `cargo test --workspace`. No CI changes needed.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework (cargo test) |
| Config file | `crates/memory-installer/Cargo.toml` (dev-dependencies: tempfile) |
| Quick run command | `cargo test -p memory-installer` |
| Full suite command | `cargo test --workspace --all-features --exclude e2e-tests` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MIG-01 | File structure correct per runtime | integration | `cargo test -p memory-installer --test e2e_converters` | No -- Wave 0 |
| MIG-02 | Frontmatter conversion correct | integration | `cargo test -p memory-installer --test e2e_converters` | No -- Wave 0 |
| MIG-03 | Old adapters archived with README stubs | manual verification | `ls plugins/memory-{copilot-adapter,gemini-adapter,opencode-plugin}/README.md` | No -- Wave 0 |
| MIG-04 | Installer in workspace CI | manual verification | `grep memory-installer Cargo.toml` | Already satisfied |

### Sampling Rate
- **Per task commit:** `cargo test -p memory-installer`
- **Per wave merge:** `cargo test --workspace --all-features --exclude e2e-tests`
- **Phase gate:** Full suite green + `task pr-precheck`

### Wave 0 Gaps
- [ ] `crates/memory-installer/tests/e2e_converters.rs` -- covers MIG-01, MIG-02
- [ ] Archive README stubs for 3 adapter directories -- covers MIG-03

## Sources

### Primary (HIGH confidence)
- `crates/memory-installer/src/converters/*.rs` -- all 6 converter implementations read in full
- `crates/memory-installer/src/converter.rs` -- RuntimeConverter trait and select_converter
- `crates/memory-installer/src/types.rs` -- all type definitions
- `crates/memory-installer/src/writer.rs` -- write_files function
- `crates/memory-installer/Cargo.toml` -- tempfile already in dev-deps
- Root `Cargo.toml` -- memory-installer confirmed in workspace members
- `.github/workflows/ci.yml` -- all CI jobs use --workspace

### Secondary (MEDIUM confidence)
- File listings of 3 adapter directories -- confirmed via filesystem

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all dependencies already present, no new crates needed
- Architecture: HIGH -- test patterns derived from existing unit test patterns in codebase
- Pitfalls: HIGH -- include_str! dependency verified in source code, OpenCode stub confirmed
- Archival: HIGH -- all files inventoried, include_str! path verified

**Research date:** 2026-03-18
**Valid until:** 2026-04-18 (stable -- no external dependency changes expected)
