# Project Research Summary

**Project:** agent-memory v2.7 — Multi-Runtime Installer (memory-installer crate)
**Domain:** Rust CLI tool — plugin format converter and installer
**Researched:** 2026-03-16
**Confidence:** HIGH

## Executive Summary

The v2.7 milestone adds a standalone `memory-installer` binary crate to the existing 14-crate agent-memory Rust workspace. Its purpose is to read the canonical Claude plugin source tree (YAML frontmatter Markdown files) and convert it into runtime-specific installations for six targets: Claude, OpenCode, Gemini, Codex, Copilot, and generic skills. This replaces five manually-maintained adapter directories that have already diverged in format. The product pattern is well-understood — the GSD installer (1600+ LOC Node.js) solves an identical problem and was directly analyzed as the reference implementation. A Rust binary is strictly better here than Node.js because it ships as a single cross-compiled binary alongside `memory-daemon`, requires no runtime dependency on the target machine, and integrates naturally with the existing CI/CD release pipeline.

The recommended approach is a `RuntimeConverter` trait-per-runtime architecture with a centralized `tool_maps.rs` for name mapping, `gray_matter` 0.3.x for frontmatter parsing (the only actively maintained frontmatter crate after `serde_yaml` was deprecated March 2024), and `walkdir` for directory traversal. The converter pipeline is: parse canonical source into a `PluginBundle`, dispatch to the appropriate runtime converter, collect `Vec<ConvertedFile>`, then write or dry-run. Each runtime converter is one file in `converters/`; adding a new runtime is one new file and one line in `mod.rs`. The `--dry-run` flag (missing from the GSD reference implementation) is a high-value, low-cost addition implementable as a write-interceptor on the output stage.

The primary risks are data-loss risks, not complexity risks. Three pitfalls are critical: (1) overwriting shared config files (Gemini `settings.json`, OpenCode `opencode.json`) instead of merging — the existing manual adapter README already warns this is destructive; (2) silently dropping unmapped tools from `allowed-tools:` arrays, leaving installed commands degraded with no error; and (3) hook format divergence causing capture to silently stop working after install (Gemini uses PascalCase event names + `command:` field; Copilot uses camelCase + `bash:` field). The parse-merge-write pattern for shared config files, a per-converter tool audit with warnings, and table-driven hook event name mapping are the mitigations. All three must be addressed before the first runtime converter ships.

---

## Key Findings

### Recommended Stack

The installer requires exactly two new external dependencies added to the workspace: `gray_matter = "0.3"` (YAML frontmatter parsing) and `walkdir = "2.5"` (recursive directory traversal). Everything else — `clap`, `serde`, `serde_json`, `toml`, `anyhow`, `thiserror`, `directories`, `shellexpand` — is already in the workspace. The critical avoidance is `serde_yaml`, which was officially deprecated in March 2024 (crates.io version is `0.9.34+deprecated`) and has no future maintenance. `gray_matter` 0.3.2 wraps `yaml-rust2` internally and handles the `---` delimiter splitting that a bare YAML parser would require manual implementation for. No async runtime (`tokio`) is needed — the installer is a one-shot synchronous filesystem tool. See `.planning/research/STACK.md` for full details.

**Core technologies:**
- `gray_matter` 0.3.2: YAML frontmatter parsing — only actively maintained frontmatter crate; uses `yaml-rust2` internally; released July 2025
- `walkdir` 2.5.0: recursive directory traversal — standard crate (218M downloads); handles recursion, symlink policies, iterator errors
- `clap` 4.5 (workspace): CLI interface — already in workspace; derive macros; shared across all workspace binaries
- `toml` 0.8 (workspace): Gemini TOML serialization — already in workspace; upgrading to 1.0.6 is safe but deferrable
- `serde_json` (workspace): OpenCode and Gemini JSON config read-merge-write — already in workspace
- `shellexpand` 3.1 (workspace): tilde expansion for install target paths — already in workspace via memory-daemon

### Expected Features

The feature scope is well-defined by comparison to the GSD reference implementation. The installer has 13 must-have P1 features all required for the v2.7 milestone. The GSD installer gap analysis identified `--dry-run` as a missing feature that has high value and low implementation cost: implement as a flag on the converter output stage rather than per-converter. See `.planning/research/FEATURES.md` for full feature details and the dependency graph.

**Must have (table stakes):**
- Runtime selection (`--agent claude|opencode|gemini|codex|copilot|skills`) — core value proposition
- Scope selection (`--project|--global`) with correct target directories per runtime
- Tool name mapping for all 5 non-Claude runtimes via static tables in `tool_maps.rs`
- Frontmatter conversion per runtime: YAML to TOML for Gemini; `allowed-tools` array to `tools` object for OpenCode; strip `color:` for Gemini
- Path rewriting in all installed content (`~/.claude/` to runtime-appropriate path)
- Clean orphan removal (delete managed dirs before fresh copy to prevent stale file accumulation)
- Idempotent re-install (running twice leaves identical result)
- `--dry-run` flag (print planned writes without executing) — GSD gap; implement via write-interceptor on `ConvertedFile` output
- `--all` flag (install for every runtime in sequence)
- Managed-section markers in shared config files (Codex `config.toml`, Gemini `settings.json`) — required for safe uninstall
- Uninstall (`--uninstall`) using managed-section markers
- Hook conversion per runtime (Phase 49) — highest-risk item in the milestone
- Codex skill adapter header injection and `config.toml` agent registration with sandbox mode

**Should have (competitive):**
- Env var config dir override (`CODEX_HOME`, `OPENCODE_CONFIG_DIR`, `XDG_CONFIG_HOME` priority chain)
- `--config-dir` explicit path override for power users and non-default setups
- Cross-reference rewriting (`/memory:search` to `/memory-search` for non-namespace runtimes)
- Interactive mode when no flags provided (post-MVP; does not block v2.7)

**Defer (v2+):**
- Plugin validation subcommand (`memory-installer validate`)
- JSON output mode (`--output json`) for machine-readable install reports
- Multi-plugin support (install multiple canonical sources)

### Architecture Approach

The architecture is a single new `memory-installer` crate with a `RuntimeConverter` trait dispatched at runtime via `Box<dyn RuntimeConverter>`. The crate is structurally isolated from `memory-daemon` — no shared Rust crate code, no gRPC, no async, no RocksDB. The canonical plugin source is read from the filesystem at install time (not embedded via `include_str!`), which allows plugin iteration without binary rebuilds. The build order has a hard sequential dependency in Phase 46 (crate scaffolding, parser, and converter trait define `PluginBundle` and `RuntimeConverter` that all subsequent phases depend on), then Phases 47 and 48 can proceed in parallel, then Phase 49 requires both, then Phase 50 for integration. See `.planning/research/ARCHITECTURE.md` for full component diagrams and per-runtime converter specifics.

**Major components:**
1. `parser.rs` — walk canonical plugin dir with `walkdir`, parse YAML frontmatter with `gray_matter`, build `PluginBundle { commands, agents, skills, hooks }`
2. `converters/` (6 files: claude, opencode, gemini, codex, copilot, skills) — each implements `RuntimeConverter` trait; `CodexConverter` delegates to `SkillsConverter` for structural transformation then adds `AGENTS.md`
3. `tool_maps.rs` — static tool name mapping tables (11 tools across 4 non-trivial runtimes); centralized to prevent per-converter drift
4. `hooks.rs` — per-runtime hook format conversion; table-driven event name mapping (PascalCase vs. camelCase; `command:` vs. `bash:` field divergence)
5. `types.rs` — `PluginBundle`, `ConvertedFile`, `InstallScope` enum; stable contract imported by `e2e-tests` without pulling in `walkdir`
6. `main.rs` — clap CLI entry point; dispatch to `select_converter()`, collect `Vec<ConvertedFile>`, write or dry-run

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for full analysis with codebase evidence. All pitfalls are verified from existing adapter READMEs, actual hook config files, and known Copilot CLI bugs that generated scripts must preserve workarounds for.

1. **Clobbering user customizations on reinstall** — Use read-merge-write for shared config files (Gemini `settings.json`, OpenCode `opencode.json`). Never overwrite. The Gemini adapter README already documents this with a `jq` merge command. This policy must be established in Phase 46 before any converter is written, not retrofitted later.

2. **Tool mapping gaps silently drop tools** — During conversion, audit every `allowed-tools:` entry against the runtime's tool map. Log a warning for unmapped tools (including MCP tools like `mcp__context7__*`). Default behavior must be `warn`, not `drop`. Missing this leaves installed commands functionally degraded with no indication.

3. **Hook format differences cause silent capture failure** — Gemini uses PascalCase event names (`AfterTool`) and `command:` field; Copilot uses camelCase (`postToolUse`) and `bash:` field. A hook registered with the wrong event name is valid JSON but never fires. Use a table-driven `HookDefinition` type with per-runtime event name mappings and test by parsing the generated config.

4. **YAML frontmatter edge cases break the parser** — Do not implement a custom YAML parser. The GSD `frontmatter.cjs` does not handle block scalars (`description: |`), colon-in-value, or regex special characters in `triggers:` arrays. Use `gray_matter` with `features = ["yaml"]` for all frontmatter parsing. Add a corpus test: parse all canonical source files and assert round-trip correctness.

5. **Premature adapter retirement** — Archive, do not delete. Keep stub directories with a deprecation README. Retire adapters one full release cycle after the installer ships (v2.8 retires what v2.7 replaces). The 144 bats tests likely reference adapter paths and must be migrated before retirement.

---

## Implications for Roadmap

Based on combined research, the build order has a hard sequential dependency through Phase 46, then two parallel paths that merge in Phase 49. The existing v2.7 plan phases (45-50) are architecturally sound. Preserve this structure.

### Phase 45: Canonical Source Consolidation

**Rationale:** All converters read the consolidated `plugins/memory-plugin/` directory. This phase produces no Rust code but is a gate — nothing can be meaningfully tested without a unified canonical source. Must complete before Phase 46 crate scaffolding begins.
**Delivers:** Unified canonical plugin source with merged commands, agents, skills, and hooks from the two existing plugins (`memory-query-plugin` and `memory-setup-plugin`)
**Addresses:** Eliminates ambiguity about which plugin directory is canonical; provides the test corpus for Phase 46 parser validation
**Avoids:** Pitfall 6 (premature retirement) — archive the old plugin directories here, do not delete

### Phase 46: Installer Crate Foundation (Parser, Converter Trait, Tool Maps)

**Rationale:** Defines `PluginBundle` and `RuntimeConverter` that Phases 47, 48, and 49 all depend on. Cannot parallelize any converter work until this is complete. This phase also establishes the owned-vs-shared file policy (merge vs. overwrite) that prevents Pitfall 1 and Pitfall 3. The managed-section marker format decided here is a compatibility contract — changing it post-release breaks uninstall for existing users.
**Delivers:** `memory-installer` crate skeleton; `parser.rs` using `gray_matter`; `types.rs`; `converter.rs` trait definition; `tool_maps.rs` with 11-tool mapping tables; workspace Cargo.toml additions (`gray_matter`, `walkdir`); corpus round-trip test for all canonical source files
**Uses:** `gray_matter` 0.3.2 (new), `walkdir` 2.5.0 (new), all other workspace deps
**Avoids:** Pitfall 5 (YAML edge cases) by using `gray_matter` from the start; Pitfall 2 (tool mapping gaps) by building the audit into the converter trait contract

### Phase 47: Claude and OpenCode Converters

**Rationale:** The Claude converter is pass-through (source and target formats are identical) — use it to validate the `RuntimeConverter` trait and `ConvertedFile` pipeline before writing complex transforms. OpenCode is the second simplest: flat commands, tools object, strip `name:` field. Both converters confirm that the Phase 46 `PluginBundle` correctly represents the canonical source. Implement `--dry-run` write-interceptor here, not per-converter.
**Delivers:** Working `claude.rs` and `opencode.rs` converters; `--dry-run` flag as write-interceptor on `Vec<ConvertedFile>` output; `--agent claude|opencode` CLI paths; `opencode.json` permissions generation; XDG env var priority resolution for OpenCode
**Implements:** RuntimeConverter Pattern 1 (trait dispatch); path rewriting; InstallScope enum
**Avoids:** OpenCode integration gotcha — strip `name:` field from converted frontmatter (OpenCode uses filename as command name; `name:` causes conflict)

### Phase 48: Gemini and Codex Converters

**Rationale:** Gemini is the most format-intensive conversion (YAML to TOML, `${VAR}` escaping, `settings.json` merge-not-overwrite). Codex introduces the multi-file output pattern (one command to one skill directory) and delegates structural transformation to `SkillsConverter`. Both are independent from Phase 47 and can run in parallel after Phase 46 completes.
**Delivers:** `gemini.rs` and `codex.rs` converters; managed-section markers in `settings.json`; `AGENTS.md` generation for Codex; `--agent gemini|codex` CLI paths; Codex sandbox mode lookup table
**Uses:** `toml` crate for Gemini TOML serialization; `serde_json` for `settings.json` read-merge-write
**Avoids:** Pitfall 3 (Gemini settings.json clobber) via merge-not-overwrite for both global and project scopes; strip `color:` and `skills:` fields; escape `${VAR}` to `$VAR`

### Phase 49: Copilot, Generic Skills, and Hook Conversion Pipeline

**Rationale:** The generic `SkillsConverter` is the structural base for `CodexConverter` — must be designed after seeing all converter patterns to extract shared logic correctly. The hook conversion pipeline is the highest-risk item in the milestone (Pitfall 7) and benefits from having all converter format patterns established first. Copilot's known quirks (Bug #991 `sessionStart` fires per-prompt; `toolArgs` double-parse) must be preserved in generated hook scripts.
**Delivers:** `copilot.rs`, `skills.rs`, `hooks.rs`; complete hook pipeline for all 5 runtimes with table-driven event name mapping; `--all` flag; complete `--agent` CLI surface; OS-specific hook script strategy (`.sh` for Unix; document WSL requirement for Windows or generate `.ps1` fallback)
**Avoids:** Pitfall 4 (Windows path separators in hook config — `%USERPROFILE%` vs `$HOME`); Pitfall 7 (hook event name divergence) via `HookDefinition` type with per-runtime event name tables

### Phase 50: Integration Testing and Migration

**Rationale:** Round-trip integration tests require all converters complete. Adapter retirement must be deliberate — stubs preserved, bats tests migrated, `memory-daemon clod convert` subcommand retired only after verifying no callers remain. This phase validates the entire pipeline end-to-end.
**Delivers:** Integration test suite (install to temp dir via `e2e-tests`, verify file structure per runtime); stub adapter directories with deprecation README; `memory-daemon Clod` subcommand retired; CI matrix updated to include `memory-installer`; Phase 50 migration guide in UPGRADING.md
**Avoids:** Pitfall 6 (premature adapter retirement) — archive not delete; verify 144 bats tests pass against new installer before removing old adapter paths

### Phase Ordering Rationale

- Phase 45 is a prerequisite with no Rust deliverables — natural first phase, no risk
- Phase 46 is a hard gate because `PluginBundle` and `RuntimeConverter` are required by all converters and the managed-section marker format is a compatibility contract
- Phases 47 and 48 are independent after Phase 46 and can run as parallel agent threads
- Phase 49 requires both 47 and 48 because `SkillsConverter` should extract patterns observed across all preceding converters
- Phase 50 requires all converters complete for meaningful integration testing
- The `--dry-run` write-interceptor should be implemented in Phase 47 (not per-converter) so it is available for testing all subsequent converters

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 49 (Hook Conversion Pipeline):** Highest-risk phase. Per-runtime hook invocation semantics are partially documented (OpenCode TypeScript plugin API event subscription shape was referenced from a cached plugin file at MEDIUM confidence). Verify OpenCode hook API against current OpenCode docs before Phase 49 planning begins.
- **Phase 50 (Windows integration testing):** The project cross-compiles for Windows but hook scripts are `.sh`. The decision to emit `.bat`/`.ps1` alternatives vs. document WSL as required must be made before Phase 49 writes hook templates. Consider requesting a `/gsd:research-phase` on Windows hook script strategy.

Phases with standard patterns (skip research-phase):
- **Phase 45:** Pure content migration — no code, well-understood task
- **Phase 46:** Standard Rust crate scaffolding; `gray_matter` and `walkdir` APIs are simple and documented; refer to existing workspace crate setups as templates
- **Phase 47:** Claude pass-through and OpenCode conversion formats documented in existing adapters; GSD reference installer covers both
- **Phase 48:** Gemini TOML format and Codex skills format documented in existing adapter READMEs and actual config files in the repo

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All dependencies verified on crates.io; `serde_yaml` deprecation confirmed at 0.9.34+deprecated; `gray_matter` 0.3.2 confirmed actively maintained (July 2025); workspace Cargo.toml verified directly |
| Features | HIGH | Based on direct source analysis of GSD reference installer (1600+ LOC) and v2.7 implementation plan; gap analysis between GSD and proposed Rust implementation is thorough; feature dependencies fully mapped |
| Architecture | HIGH | Based on existing workspace code (`clod.rs`, `cli.rs`), canonical plugin source with actual frontmatter, v2.7 authoritative plan, and GSD `frontmatter.cjs` internals; `RuntimeConverter` trait pattern is well-established |
| Pitfalls | HIGH | Drawn from codebase evidence — adapter READMEs with explicit warnings, actual hook config files showing format divergence, known Copilot CLI bugs (Bug #991, toolArgs double-parse), all confirmed in existing artifacts |

**Overall confidence:** HIGH

### Gaps to Address

- **OpenCode TypeScript plugin hook API shape:** The event subscription API for OpenCode hooks was referenced from a cached plugin file (MEDIUM confidence). Verify against current OpenCode documentation before Phase 49 implementation to confirm field names and event type enumeration.
- **Windows hook script strategy:** The v2.7 plan does not resolve whether to emit `.bat`/`.ps1` on Windows or document WSL as required. This decision must be made before Phase 49 writes hook script templates. Decide and document in Phase 46 policy as part of the `HookDefinition` type design.
- **Managed-section marker format:** The exact marker strings must be decided in Phase 46 before any install runs in production. Once markers are present in user config files, changing them requires a migration. Treat this as a compatibility contract and document the decision explicitly.
- **`toml` crate upgrade (0.8 to 1.0.6):** The workspace uses `toml` 0.8; latest stable is 1.0.6. Upgrading is safe but requires testing across all crates. Can defer to post-v2.7 if no blocking issues in Phase 48 Gemini TOML generation.

---

## Sources

### Primary (HIGH confidence)

- `/Users/richardhightower/src/get-shit-done/bin/install.js` — Direct source analysis of GSD reference installer (1600+ LOC Node.js); feature landscape, conversion patterns, managed-section marker approach
- `docs/plans/v2.7-multi-runtime-portability-plan.md` — Authoritative v2.7 milestone plan with tool mapping tables and phase definitions
- `plugins/memory-gemini-adapter/README.md` — Documents merge requirement for `settings.json`, project vs. global precedence model, manual `jq` merge command
- `plugins/memory-copilot-adapter/README.md` — Documents camelCase event names, `bash:` field format, Copilot Bug #991, toolArgs double-parse quirk
- `plugins/memory-gemini-adapter/.gemini/settings.json` — Actual Gemini hook config format (PascalCase event names, `command:` field)
- `plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json` — Actual Copilot hook format (camelCase event names, `bash:` field)
- `crates/memory-daemon/src/clod.rs` — Existing CLOD converter showing patterns to supersede in Phase 50
- `.planning/PROJECT.md` — Key architectural decisions and workspace constraints (cross-platform targets, binary architecture)
- crates.io API — `serde_yaml` deprecation confirmed (0.9.34+deprecated, March 2024); `gray_matter` 0.3.2 confirmed (July 2025); `walkdir` 2.5.0 (March 2024); `toml` 1.0.6 (March 2026)
- Workspace `Cargo.toml` — Verified existing: clap 4.5, toml 0.8, serde 1.0, serde_json 1.0, anyhow 1.0, thiserror 2.0, directories 6.0, shellexpand 3.1, tempfile 3.15

### Secondary (MEDIUM confidence)

- `/Users/richardhightower/.claude/get-shit-done/bin/lib/frontmatter.cjs` — GSD YAML parser internals; confirms block scalar limitation and known gaps; informs why `gray_matter` is required over custom parser
- `/Users/richardhightower/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.1/.opencode/plugins/superpowers.js` — OpenCode TypeScript plugin format reference for hook event subscription API shape

---
*Research completed: 2026-03-16*
*Synthesized by: gsd-synthesizer from STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
*Ready for roadmap: yes*
