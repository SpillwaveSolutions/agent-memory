---
phase: 46-installer-crate-foundation
verified: 2026-03-17T20:13:14Z
status: gaps_found
score: 6/7 must-haves verified
gaps:
  - truth: "Unmapped tool names produce a tracing::warn log (not silent drops)"
    status: failed
    reason: "map_tool() returns None silently for unknown tool names. No tracing::warn is emitted anywhere in map_tool or in callers. The VALIDATION.md planned test tool_maps::test_unmapped_warns does not exist. The converter stubs never call map_tool so the warning path has no trigger site."
    artifacts:
      - path: "crates/memory-installer/src/tool_maps.rs"
        issue: "map_tool wildcard arm `_ => None` has no tracing::warn call; silent drop"
    missing:
      - "Add tracing::warn!(\"unmapped tool '{}' for {:?} runtime -- skipping\", claude_name, runtime) in the `_ => None` arm of map_tool, before returning None"
      - "Add test verifying the warn is emitted (requires tracing-test or log capture)"
---

# Phase 46: Installer Crate Foundation Verification Report

**Phase Goal:** memory-installer crate with CLI, plugin parser, converter trait, tool maps -- the foundation all converter phases (47-49) build on
**Verified:** 2026-03-17T20:13:14Z
**Status:** gaps_found
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | memory-installer crate compiles as a standalone binary with no tokio dependency | VERIFIED | `cargo build -p memory-installer` succeeds; Cargo.toml has no tokio dep; no tokio in dependency metadata |
| 2 | Running memory-installer --help prints usage with install subcommand and all flags | VERIFIED | `cargo run -- --help` shows install subcommand; `--agent`, `--project`, `--global`, `--dir`, `--dry-run`, `--source` all present |
| 3 | All 6 converter stubs implement RuntimeConverter trait and compile cleanly | VERIFIED | All 6 .rs files exist with 9 RuntimeConverter method signatures each; `cargo test` passes all 47 tests; `cargo clippy -D warnings` clean |
| 4 | select_converter(runtime) returns a Box<dyn RuntimeConverter> for all 6 runtimes | VERIFIED | converters/mod.rs has exhaustive match; converter.rs tests verify name() for all 6 runtimes |
| 5 | Parser returns a PluginBundle with 6 commands, 2 agents, 13 skills | VERIFIED | Tests `test_parse_sources_command_count`, `test_parse_sources_agent_count`, `test_parse_sources_skill_count` all pass |
| 6 | map_tool covers all 11 tools for all 6 runtimes; Task returns None for Gemini | VERIFIED | Static match table in tool_maps.rs has explicit arm for all 66 (6x11) combinations; exhaustive tests pass |
| 7 | Unmapped tool names produce a tracing::warn log (not silent drops) | FAILED | `map_tool` wildcard arm `_ => None` has no tracing::warn; no converter calls map_tool; planned test `test_unmapped_warns` does not exist |

**Score:** 6/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/memory-installer/Cargo.toml` | Crate definition with gray_matter and walkdir deps | VERIFIED | Both deps present; no tokio; follows workspace pattern |
| `crates/memory-installer/src/main.rs` | CLI entry point with clap derive parser | VERIFIED | Cli + Commands::Install with all 6 flags; scope validation; full pipeline wiring |
| `crates/memory-installer/src/lib.rs` | pub mod declarations for all 6 modules | VERIFIED | Declares types, converter, converters, parser, tool_maps, writer |
| `crates/memory-installer/src/types.rs` | PluginBundle, ConvertedFile, Runtime, InstallConfig, managed-section constants | VERIFIED | All types present; MANAGED_BEGIN/MANAGED_END constants with compatibility contract docs |
| `crates/memory-installer/src/converter.rs` | RuntimeConverter trait with 7 methods | VERIFIED | Trait has name, target_dir, convert_command, convert_agent, convert_skill, convert_hook, generate_guidance; 8 dispatch tests |
| `crates/memory-installer/src/converters/mod.rs` | select_converter dispatch table | VERIFIED | Exhaustive match for all 6 Runtime variants; pub use for all converter structs |
| `crates/memory-installer/src/parser.rs` | parse_sources() and parse_md_file() | VERIFIED | 232 lines; gray_matter generic parse::<Value>; two-level discovery; walkdir skill traversal; 7 tests |
| `crates/memory-installer/src/tool_maps.rs` | map_tool() for 11 tools x 6 runtimes | PARTIAL | Static match table present and correct; KNOWN_TOOLS const; 18 tests pass. Gap: no tracing::warn on unknown tools |
| `crates/memory-installer/src/writer.rs` | write_files(), merge_managed_section(), markers | VERIFIED | 346 lines; dry-run mode; 3-case merge logic; remove_managed_section; 14 tests |
| `crates/memory-installer/src/converters/claude.rs` | ClaudeConverter stub | VERIFIED | Unit struct; all 7 RuntimeConverter methods; correct target_dir paths |
| `crates/memory-installer/src/converters/opencode.rs` | OpenCodeConverter stub | VERIFIED | Unit struct; all 7 RuntimeConverter methods |
| `crates/memory-installer/src/converters/gemini.rs` | GeminiConverter stub | VERIFIED | Unit struct; all 7 RuntimeConverter methods |
| `crates/memory-installer/src/converters/codex.rs` | CodexConverter stub | VERIFIED | Unit struct; all 7 RuntimeConverter methods |
| `crates/memory-installer/src/converters/copilot.rs` | CopilotConverter stub | VERIFIED | Unit struct; all 7 RuntimeConverter methods |
| `crates/memory-installer/src/converters/skills.rs` | SkillsConverter stub | VERIFIED | Unit struct; all 7 RuntimeConverter methods |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/main.rs` | `src/lib.rs` | `use memory_installer::*` | WIRED | main.rs imports converters, parser, types, writer from memory_installer crate |
| `src/main.rs` | `src/parser.rs` | calls parse_sources | WIRED | `parser::parse_sources(&source_root)` on line 91 |
| `src/main.rs` | `src/converters/mod.rs` | calls select_converter | WIRED | `converters::select_converter(agent)` on line 119 |
| `src/main.rs` | `src/writer.rs` | calls write_files | WIRED | `writer::write_files(&all_files, dry_run)` on line 141 |
| `src/converters/mod.rs` | `src/converter.rs` | RuntimeConverter import | WIRED | `use crate::converter::RuntimeConverter` on line 15 |
| `src/parser.rs` | `plugins/installer-sources.json` | reads and deserializes | WIRED | `source_root.join("installer-sources.json")` on line 90 |
| `src/parser.rs` | `plugins/*/.claude-plugin/marketplace.json` | reads marketplace | WIRED | `source_dir.join(".claude-plugin/marketplace.json")` on line 104 |
| `src/writer.rs` | `src/types.rs` (MANAGED_BEGIN/END) | re-exports constants | WIRED | `pub use crate::types::{MANAGED_BEGIN, MANAGED_END, ...}` on line 18 |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| INST-01 | 46-01 | Standalone binary with clap CLI, all flags | SATISFIED | Binary builds; `--help` shows all 6 flags; scope validation exits with error when none provided |
| INST-02 | 46-02 | Plugin parser extracts commands/agents/skills with YAML frontmatter | SATISFIED | parse_sources() returns 6 commands, 2 agents, 13 skills; frontmatter as serde_json::Value; 7 passing integration tests |
| INST-03 | 46-01 | RuntimeConverter trait with 7 methods | SATISFIED | Trait defined in converter.rs; all 6 stubs implement it; select_converter dispatches correctly |
| INST-04 | 46-03 | Tool mapping tables for all 11 tools x 6 runtimes | SATISFIED | Static match covers all 66 combinations; KNOWN_TOOLS const; exhaustive tests pass |
| INST-05 | 46-03 | Managed-section markers enabling safe merge/upgrade/uninstall | SATISFIED | MANAGED_BEGIN/MANAGED_END constants; merge_managed_section 3-case logic; remove_managed_section; 8 writer tests |
| INST-06 | 46-03 | --dry-run shows what would be installed without writing | SATISFIED | write_files dry_run=true prints CREATE/OVERWRITE without touching filesystem; end-to-end `--dry-run` works |
| INST-07 | 46-03 | Unmapped tool names produce warnings, not silent drops | BLOCKED | map_tool returns None silently; no tracing::warn in map_tool or callers; planned test `test_unmapped_warns` absent |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/converters/claude.rs` | 31,35,39,43,47 | `Vec::new()` / `None` stubs | INFO | By design -- stub pattern, to be filled in phases 47-49 |
| `src/converters/opencode.rs` | all convert methods | empty stubs | INFO | By design -- stub pattern |
| `src/converters/gemini.rs` | all convert methods | empty stubs | INFO | By design -- stub pattern |
| `src/converters/codex.rs` | all convert methods | empty stubs | INFO | By design -- stub pattern |
| `src/converters/copilot.rs` | all convert methods | empty stubs | INFO | By design -- stub pattern |
| `src/converters/skills.rs` | all convert methods | empty stubs | INFO | By design -- stub pattern |
| `src/tool_maps.rs` | 102 | `_ => None` with no warn | WARNING | Blocks INST-07; runtime-silent tool drops when unknown tools are encountered in phases 47-49 |

All INFO stubs are intentional -- phases 47-49 fill in the conversion logic.

The WARNING on `tool_maps.rs` line 102 is the only blocking gap.

### Human Verification Required

None. All key behaviors are verified programmatically:
- Build: verified via `cargo build`
- Tests: 47/47 passing via `cargo test`
- CLI flags: verified via `--help` output
- Dry-run end-to-end: verified via `cargo run -- install --agent claude --project --dry-run --source plugins/`
- Clippy: clean with `-D warnings`
- Format: `cargo fmt --check` passes

### Gaps Summary

**One gap blocking full goal achievement:**

INST-07 ("Unmapped tool names produce warnings, not silent drops") is not implemented. The `map_tool` function in `tool_maps.rs` has a wildcard arm `_ => None` that silently discards unknown tool names. No `tracing::warn!` macro call exists anywhere in the tool mapping code path. The VALIDATION.md pre-planned a test named `tool_maps::test_unmapped_warns` for this requirement, but this test was never written.

The fix is small: add `tracing::warn!("unmapped tool '{}' for {:?} -- skipping", claude_name, runtime)` before the `None` return in the wildcard arm. This ensures that when phases 47-49 implement converters calling `map_tool` with tool names from plugin frontmatter, any typos or new tools in future plugin versions will log a visible warning rather than silently producing empty output.

All other phase 46 goals are fully achieved: the crate builds, 47 tests pass, all 7 RuntimeConverter methods are trait-defined and implemented in 6 stubs, the parser returns correct counts against real plugin directories, tool maps cover all 11 tools x 6 runtimes, the writer handles dry-run and managed-sections, and the main.rs pipeline runs end-to-end.

---

_Verified: 2026-03-17T20:13:14Z_
_Verifier: Claude (gsd-verifier)_
