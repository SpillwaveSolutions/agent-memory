# Pitfalls Research: Multi-Runtime Installer

**Domain:** Multi-runtime plugin installer — converting canonical Claude plugin format to OpenCode, Gemini, Codex, Copilot, and generic skill runtimes. Adding an automated conversion system on top of existing manually-maintained adapters.
**Researched:** 2026-03-16
**Confidence:** HIGH (drawn from codebase analysis of actual adapter differences, GSD frontmatter parser internals, and observed format divergences in the existing manually-maintained adapters)

---

## Critical Pitfalls

Mistakes that cause data loss, user workflow breakage, or require architectural rework.

---

### Pitfall 1: Clobbering User Customizations on Reinstall

**What goes wrong:**
The installer runs `--agent gemini --global` and writes files to `~/.gemini/`. The user has manually edited `~/.gemini/settings.json` to add their own hooks, API keys, or custom settings. The installer overwrites the file and wipes their customizations.

**Why it happens:**
File-copy installers default to overwrite semantics because it is simpler to implement. The Gemini adapter's manual install instructions already warn about this explicitly: "IMPORTANT: Do NOT overwrite — merge hooks into existing settings." But automation forgets the warning and just writes.

**Evidence from existing adapters:**
The Gemini adapter's manual install instructions include a merge step using `jq`:
```bash
EXISTING=$(cat ~/.gemini/settings.json 2>/dev/null || echo '{}')
HOOKS=$(cat plugins/memory-gemini-adapter/.gemini/settings.json | jq '.hooks')
echo "$EXISTING" | jq --argjson hooks "$HOOKS" '.hooks = ((.hooks // {}) * $hooks)' > ~/.gemini/settings.json
```
This merge logic exists because a naive overwrite breaks existing user configurations. The installer must replicate this logic for every file that is not fully owned by agent-memory.

**Consequences:**
- User loses other hooks they rely on (CI integrations, other tools)
- User loses custom API key configurations in settings.json
- User reports the installer as destructive and avoids future upgrades
- Cross-runtime inconsistency: some runtimes blow up settings, others merge

**How to avoid:**
1. For files NOT fully owned by the installer (settings.json, plugin manifests that users may customize): use read-merge-write, not overwrite. Read the existing file, merge the agent-memory sections, write the result.
2. For files FULLY owned by the installer (converted command files, skill directories): overwrite is safe.
3. Implement `--dry-run` that prints the diff between what exists and what will be written.
4. Before any write, back up the target file to `<file>.agent-memory.bak`.
5. Track which files were installed by writing a manifest: `~/.config/agent-memory/installed-files.json`. Reinstall only touches files in the manifest.

**Warning signs:**
- Install command does not distinguish between "owned" and "shared" files
- No merge logic for settings.json, opencode.json, or similar shared config files
- Reinstall test does not start from "user has existing config"

**Phase to address:** Phase 46 (Installer Crate Foundation) — establish owned vs. shared file policy before any runtime converter is written. Phase 50 (Integration Testing) — include a reinstall test that verifies customization preservation.

---

### Pitfall 2: Tool Mapping Gaps — Unmapped Tools Silently Dropped

**What goes wrong:**
A Claude command uses a tool not in the mapping table (e.g., `mcp__context7__resolve-library-id`, `Agent`, or a custom MCP tool). The converter silently drops the `allowed-tools:` entry. The installed command runs but lacks expected capabilities.

**Why it happens:**
Tool mapping tables are defined statically at converter implementation time. The canonical Claude source can reference any tool available to Claude, including MCP tools, custom tools, and tools added after the converter was written. The converter's tool_maps.rs does not have a catch-all for unmapped tools.

**Evidence from the implementation plan:**
The plan's tool mapping table (Phase 46-03) lists 11 tool mappings but notes that `Task` is excluded for Gemini (`*(excluded)*`). This is an explicit gap. Any tool not in the table has undefined behavior.

**Consequences:**
- Skills and commands that rely on MCP tools (Context7, memory MCP server) are silently degraded
- `Agent` tool (sub-agent invocation) is not mapped for Gemini/Codex, breaking orchestrator skills
- Users report commands that "don't work" but the installer reported success
- Skills that reference excluded tools execute but cannot do anything useful

**How to avoid:**
1. During conversion, collect all tool names from `allowed-tools:` fields and check each against the runtime's tool map.
2. For unmapped tools: log a warning with the tool name, the command/skill it came from, and the target runtime.
3. Provide three behaviors configurable per-runtime: `drop` (silent), `warn` (log), `fail` (abort conversion). Default should be `warn`.
4. Include unmapped tool names in conversion output metadata so they can be inspected after install.
5. For MCP tools: generate a comment in the installed file noting which MCP tools were excluded.

**Warning signs:**
- Converter's convert_command does not iterate `allowed-tools:` entries before writing the converted file
- No test that installs a command with a non-standard tool and verifies the warning appears
- The tools object/array in the converted output has fewer entries than the canonical source

**Phase to address:** Phase 46-03 (Tool Maps) and Phase 47-02 (OpenCode Converter). Each converter must implement the tool audit.

---

### Pitfall 3: Gemini settings.json Clobbers Existing Hooks

**What goes wrong:**
The installer writes to `~/.gemini/settings.json`. The user already has project-level `.gemini/settings.json` with OTHER hooks (not agent-memory). The installer overwrites the project-level file, silently removing the other hooks.

**This is different from Pitfall 1** in that the issue is specifically with Gemini's precedence model: project settings take FULL precedence over global settings — they do NOT merge. So if the installer writes a project-level settings.json, it must contain ALL hooks the user wants, not just agent-memory's hooks.

**Evidence from existing adapter README:**
> "Important: If you have both global and project-level settings.json with hooks, the project-level hooks take full precedence for that project (they do NOT merge). Ensure your project-level settings include the memory-capture hooks if you want capture in that project."

**Consequences:**
- User's CI hooks, linting hooks, or other tools stop firing silently
- The user has no indication this happened — Gemini does not report missing hooks
- Debugging requires knowing Gemini's precedence model, which most users do not

**How to avoid:**
1. Global install (`--global`): merge agent-memory hooks into `~/.gemini/settings.json`
2. Project install (`--project`): read existing `.gemini/settings.json`, merge agent-memory sections, write back
3. NEVER overwrite Gemini settings files without first reading and merging
4. Add a `--list-hooks` diagnostic subcommand that shows which hooks are registered across global and project levels

**Warning signs:**
- Global and project install paths use the same write function without a merge flag
- No test for installing when a project-level settings.json with other hooks already exists

**Phase to address:** Phase 48-01 (Gemini Converter).

---

### Pitfall 4: Path Separator Disaster on Windows

**What goes wrong:**
The installer writes hook paths into settings.json or hooks.json as Unix paths (`$HOME/.gemini/hooks/memory-capture.sh`). On Windows, these paths use backslashes and the `$HOME` variable may not be set. The hooks reference a path that does not exist.

**Why it happens:**
Rust's `PathBuf` handles path separators correctly in the file system, but string interpolation for config file contents (not file system paths) bypasses PathBuf. When writing `"command": "$HOME/.gemini/hooks/memory-capture.sh"` into JSON, the string is literal and does not benefit from PathBuf normalization.

**Evidence from the PROJECT.md constraints:**
> Platforms: macOS, Linux, Windows (cross-compile)

The project explicitly targets Windows. Hook scripts (`.sh` files) do not run on Windows without WSL or Git Bash.

**Consequences:**
- All hook registrations produce paths that fail on Windows
- Shell scripts (`.sh`) are not executable on Windows directly
- `$HOME` is not a Windows environment variable (it is `%USERPROFILE%`)
- If the installer generates Windows-incompatible hook scripts, hooks silently fail

**How to avoid:**
1. Detect the target platform at install time (`std::env::consts::OS`)
2. On Windows: use `%USERPROFILE%` instead of `$HOME`, backslash separators in string paths, and generate `.bat` or `.ps1` hook scripts instead of `.sh`
3. For hook scripts: provide OS-specific templates (`.sh` for Unix, `.bat`/`.ps1` for Windows)
4. Test the installer on Windows as part of the Phase 50 integration test matrix
5. In the short term: document Windows as "WSL required for hook scripts" and emit a clear error on Windows rather than silently generating broken hooks

**Warning signs:**
- Hook path generation uses string concatenation with `/` separators instead of `PathBuf`
- No Windows CI job in the test matrix for the installer
- Hook script templates are `.sh` only

**Phase to address:** Phase 49-02 (Hook Conversion Pipeline). Windows handling must be addressed here, not retrofitted.

---

### Pitfall 5: YAML Frontmatter Edge Cases Break the Parser

**What goes wrong:**
The canonical Claude plugin source uses YAML frontmatter. The `parser.rs` extracts frontmatter using a simple regex and custom parser. YAML has edge cases that break simple parsers:

1. **Multiline strings** (`description: |` or `description: >`) — the GSD frontmatter parser does NOT handle block scalars. A multiline description in a skill's frontmatter is parsed as an empty object.
2. **Colon in values** (`description: "Use this for foo: bar"`) — the simple line parser splits on `:` and truncates the value.
3. **Special characters** in trigger patterns (`pattern: "what (did|were) we"`) — parentheses and pipe characters in patterns are valid YAML but break naive parsers.
4. **Quoted vs. unquoted values** — `name: memory-query` vs. `name: "memory-query"` should parse identically but may not.

**Evidence from the GSD frontmatter parser:**
The `frontmatter.cjs` parser (examined directly) uses a custom line-by-line parser, NOT a full YAML library. It has a known limitation: `value === '[' ? [] : {}` handles inline arrays but not block scalars. The comment "Key with no value or opening bracket — could be nested object or array" reveals this is a simplified parser.

The canonical source already has multiline descriptions in agent frontmatter:
```yaml
description: Autonomous agent for intelligent memory retrieval with tier-aware routing, intent classification, and automatic fallback chains
```
This is a long string on one line, which works. But the GSD templates use block scalars (`description: |`), which would fail.

**Consequences:**
- Descriptions are silently truncated or converted to empty strings/objects
- Trigger patterns with special characters are parsed incorrectly
- Converted files have wrong or missing descriptions, making the installed plugin non-functional
- The parser silently succeeds but produces garbage — no error is raised

**How to avoid:**
1. Use `serde_yaml` (already in the plan's tech stack) for all frontmatter parsing. Do NOT implement a custom YAML parser.
2. Test the parser against the actual canonical source files before implementing converters. Run: parse every `.md` file in `plugins/memory-plugin/` and verify round-trip correctness.
3. Pay special attention to: the `triggers:` array in agent files (contains regex patterns with special chars), `description:` fields with colons and quotes, `parameters:` objects.
4. Add a corpus test: parse all canonical source files and assert `parse → serialize → parse` produces identical output.

**Warning signs:**
- `parser.rs` implements custom YAML parsing instead of using `serde_yaml`
- No test that round-trips a file with a multiline description
- No test that parses the `memory-navigator.md` agent file (which has complex trigger patterns)

**Phase to address:** Phase 46-02 (Plugin Parser). This is foundational — wrong parsing propagates to all converters.

---

### Pitfall 6: Breaking Existing Adapters During Migration (Premature Retirement)

**What goes wrong:**
Phase 50 retires the manually-maintained adapter directories. Users who have installed the Gemini adapter globally (files in `~/.gemini/`) continue to work, but users who reference the adapter directories from the repository (symlinks, relative paths, plugin installs from the repo path) break when the directories are archived.

**Why it happens:**
The Copilot README explicitly mentions:
```bash
/plugin install /path/to/plugins/memory-copilot-adapter
```
If `plugins/memory-copilot-adapter/` is archived or removed, this command fails. Users who installed via symlink (`ln -s`) also break.

**Consequences:**
- Users who have not yet migrated to the new installer lose functionality immediately when they pull the latest code
- CI tests that reference the adapter paths by directory fail
- The bats test suite (144 tests) likely has hardcoded references to the adapter directories
- Rolling back is painful if the old adapter files were deleted

**How to avoid:**
1. Archive, do NOT delete. Move `plugins/memory-copilot-adapter/` to `plugins/archived/memory-copilot-adapter/` with a deprecation notice.
2. Keep a stub `plugins/memory-copilot-adapter/` with only a README that says "Retired — use `memory-daemon install-agent --agent copilot`".
3. Retire adapters only AFTER the new installer has been running for one full release cycle (v2.8 retires what v2.7 replaces).
4. Update the bats tests to install via the new installer before running CLI tests, rather than referencing adapter directories.
5. Add a migration notice to the main README and UPGRADING.md before retirement.

**Warning signs:**
- Phase 50 deletes adapter directories rather than archiving them
- No migration guide between old adapter and new installer
- Bats tests still reference `plugins/memory-copilot-adapter/` after Phase 50

**Phase to address:** Phase 50-02 (Migration and Documentation). The retirement sequence must be deliberate.

---

### Pitfall 7: Hook Format Differences Cause Silent Capture Failure

**What goes wrong:**
Different runtimes have fundamentally different hook invocation formats, and the converter generates hooks that register but do not fire correctly:

- **Claude**: YAML hooks in `.claude/hooks.yaml`, event names are PascalCase (`PostToolUse`)
- **Gemini**: JSON in `settings.json` with array-of-objects structure, event names are PascalCase (`AfterTool`) but different from Claude
- **Copilot**: Standalone JSON file `memory-hooks.json`, event names are camelCase (`postToolUse`), `bash:` key for script path
- **OpenCode**: TypeScript plugin, hooks via plugin API
- **Codex**: No hook support (acknowledged in v2.4)

The hook conversion pipeline must know not just the file format but the event name mapping, invocation format, and path syntax for each runtime.

**Evidence from existing adapters:**
| Runtime | Hook Config Location | Event Name Format | Script Field |
|---------|---------------------|-------------------|--------------|
| Gemini | `settings.json` .hooks object | PascalCase (`SessionStart`) | `command:` |
| Copilot | `memory-hooks.json` standalone | camelCase (`sessionStart`) | `bash:` |

These differ in two ways: event name casing AND the field name for the script path. A converter that gets either wrong produces hooks that are registered (the file is valid JSON) but never fire (the event name does not match).

**Consequences:**
- Memory capture silently stops working after install via the new installer
- Users can verify the file was written but cannot debug why events are not captured
- The installed plugin "works" for commands/queries but not for capture, which is the primary value proposition

**How to avoid:**
1. Define a `HookDefinition` type in `hooks.rs` that maps canonical event names to per-runtime equivalents.
2. Write tests that install hooks for each runtime and verify the generated JSON matches the expected format exactly (field names, event names, path syntax).
3. Include a `verify_hooks` subcommand that reads the installed hooks config and reports whether the hook scripts are registered with the correct event names.
4. The conversion of hook event names must be table-driven and exhaustive, not ad hoc.

**Warning signs:**
- Hook conversion code has hardcoded event name strings without a mapping table
- No test that parses the generated hook config file and verifies event names
- The `hooks.rs` implementation treats all runtimes with the same JSON structure

**Phase to address:** Phase 49-02 (Hook Conversion Pipeline). This phase is the highest-risk in the milestone.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcode tool name mappings as constants | Fast to implement | Adding a new tool requires code change and recompile | Never — put in config files loaded at runtime |
| Use `fs::copy` for all converted files | Simplest write path | Overwrites user customizations, breaks Gemini settings merge | Never for shared config files (settings.json, plugin manifests) |
| Skip Windows path handling for now | Saves 1-2 days | Windows users get broken hook registrations with no error | Only if Windows explicitly scoped to Phase 50+ |
| Retire old adapters in same PR as new installer | Clean repo | Users have zero transition period, CI breaks | Never — keep adapters for one release cycle |
| Single `ConvertedFile` struct with string content | Simple generic type | Cannot represent binary files, loses type information about what the file IS | Acceptable for Phase 46, refactor by Phase 50 |
| Generate TOML with string formatting instead of `toml` crate | No extra dependency | TOML serialization has edge cases (special chars, multi-line) | Never — use the `toml` crate |

---

## Integration Gotchas

Common mistakes when connecting to the existing adapter system.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Gemini settings.json | Overwrite the file | Read → deserialize → merge hooks section → serialize → write |
| Copilot plugin install | Generate global path in hooks.json | Copilot hooks are project-local; paths must be project-relative |
| OpenCode commands | Keep `name:` field in frontmatter | OpenCode uses filename as command name; `name:` field causes duplicate or conflict |
| Gemini commands | Keep `color:` and `skills:` fields | Gemini TOML ignores these but their presence may cause parse errors in strict mode |
| Gemini template strings | Keep `${VAR}` syntax from Claude source | Gemini template engine treats `${VAR}` as variable substitution; must escape to `$VAR` or literal |
| Codex converter | Assume commands map 1:1 to skills | Codex skills have a different activation model; commands need AGENTS.md for orchestration |
| Claude pass-through | Copy .claude-plugin/plugin.json as-is | Path rewrites for storage dirs (`~/.claude/`) are still needed even in pass-through mode |

---

## Performance Traps

Patterns that work during development but fail in the field.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Walking entire plugin directory for every file | Works with 13 skills | O(n²) behavior as plugin grows | At 100+ skills/commands |
| Parsing all YAML frontmatter eagerly | Fast for test suite | Slow startup for dry-run of large plugin | At 50+ files; acceptable tradeoff |
| Generating TOML from serde_yaml::Value directly | Works for simple scalars | TOML and YAML type systems differ; booleans, integers, multiline strings need conversion | First time a field contains a multiline string |
| Holding all ConvertedFile contents in memory before writing | Simple implementation | High memory use for large skill directories with reference files | At 50MB+ of skill content; unlikely but possible |

---

## Security Mistakes

Domain-specific security issues for the installer.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Writing hook scripts with world-writable permissions | Any local process can modify the hook handler to intercept tool calls | Always `chmod 755` hook scripts, never `chmod 777` |
| Embedding absolute paths to `memory-ingest` in hook scripts at install time | Binary path is hardcoded; if the binary moves, hooks silently fail | Use `MEMORY_INGEST_PATH` env var override pattern (already exists in adapters) |
| Not validating the canonical source directory before conversion | A corrupted or adversarial plugin source could inject arbitrary shell code into hook scripts | Validate that hook script templates contain only expected patterns before writing |
| Writing converted files to arbitrary paths via `--dir` | `--agent skills --dir /etc/` would write files to system directories | Validate `--dir` path is within the user's home directory or a project subdirectory |

---

## UX Pitfalls

Common user experience mistakes for the installer.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Install succeeds but hooks are not firing (silent failure) | User thinks memory capture is working; no events are ingested | Print "Hooks registered. Verify with: memory-daemon install-agent --verify --agent gemini" |
| Installer overwrites files without telling the user what changed | User cannot review or reject changes | Always show a summary of files that will be written before writing them (like `terraform plan`) |
| `--dry-run` output is not actionable | User sees a list of files but cannot tell what would change | Dry-run should diff current files against what will be written |
| Reinstall on upgrade silently reverts user's edits to installed skills | User who customized a skill loses their changes | Track a content hash of installed files; warn if the user has modified them |
| No `--uninstall` command | User has to manually hunt down all installed files | Implement uninstall using the installed-files manifest |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Claude pass-through converter:** Often missing storage path rewrites (`~/.claude/` → `~/.config/agent-memory/`) — verify by checking if any path in the converted output contains a development machine path
- [ ] **OpenCode converter:** Often missing `name:` field stripping — verify by checking converted command files do not contain `name:` in frontmatter
- [ ] **Gemini converter:** Often missing `${VAR}` → `$VAR` escaping — verify by searching converted TOML files for `${`
- [ ] **Hook conversion:** Often missing executable permission on hook scripts — verify with `ls -la` after install
- [ ] **Global vs. project install:** Often installs to project dir when `--global` was intended — verify by checking the actual write target path in test output
- [ ] **Tool mapping audit:** Often appears complete but has one unmapped tool — verify by installing a command that references every tool in the mapping table and checking the output
- [ ] **Gemini settings.json merge:** Often appears to work but actually overwrites because merge test was run against an empty settings.json — test against a non-empty settings.json with existing hooks

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| User customizations overwritten | HIGH | Restore from `.agent-memory.bak` backup files if the installer created them; otherwise manual reconstruction |
| Broken adapter retirement (symlinks fail) | MEDIUM | Create stub directories pointing to new installer; update UPGRADING.md |
| Silent hook non-firing | LOW | Run `memory-daemon install-agent --verify --agent <runtime>` to diagnose; reinstall hooks |
| Tool mapping gap discovered post-release | LOW | Add to mapping table and republish; existing installs need reinstall to pick up new mappings |
| YAML parse failure on edge-case frontmatter | MEDIUM | Switch to serde_yaml for the failing field; all converted files need regeneration |
| Wrong event names in hook config | MEDIUM | Regenerate hook config with corrected event names; requires reinstall |
| Windows path breakage | HIGH | Generate OS-specific hook scripts; requires reinstall on Windows |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Clobbering user customizations (Pitfall 1) | Phase 46 (Installer Foundation) — establish owned vs. shared file policy | Reinstall test starting from user-modified state |
| Tool mapping gaps (Pitfall 2) | Phase 46-03 (Tool Maps) — tool audit in every converter | Test: install command with non-standard tool, verify warning logged |
| Gemini settings.json clobber (Pitfall 3) | Phase 48-01 (Gemini Converter) | Test: install with existing settings.json containing other hooks; verify other hooks preserved |
| Windows path separators (Pitfall 4) | Phase 49-02 (Hook Conversion Pipeline) | CI test on Windows runner; verify path strings in hook config |
| YAML frontmatter edge cases (Pitfall 5) | Phase 46-02 (Plugin Parser) — use serde_yaml | Corpus test: parse all canonical source files, round-trip assertion |
| Premature adapter retirement (Pitfall 6) | Phase 50-02 (Migration) — archive not delete | Verify old adapter directories exist as stubs post-migration |
| Hook format differences (Pitfall 7) | Phase 49-02 (Hook Conversion Pipeline) | Test: generated hook config for each runtime parsed and event names verified |

---

## Sources

### Codebase Evidence (HIGH confidence)
- `plugins/memory-gemini-adapter/README.md` — documents merge requirement for settings.json, project vs. global precedence model
- `plugins/memory-copilot-adapter/README.md` — documents camelCase event names, `bash:` field format, no-global-hooks limitation
- `plugins/memory-gemini-adapter/.gemini/settings.json` — shows actual Gemini hook config format with PascalCase event names, `command:` field
- `plugins/memory-copilot-adapter/.github/hooks/memory-hooks.json` — shows Copilot hook format with camelCase event names, `bash:` field
- `plugins/memory-opencode-plugin/.opencode/command/` — shows OpenCode flat command format without `name:` field
- `/Users/richardhightower/.claude/get-shit-done/bin/lib/frontmatter.cjs` — GSD YAML parser does not support block scalars; risk for any parser built on similar approach

### Implementation Plan Evidence (HIGH confidence)
- `docs/plans/v2.7-multi-runtime-portability-plan.md` — tool mapping table (Phase 46-03) shows known gaps (`Task` excluded for Gemini)
- v2.7 plan Phase 48-01 — explicit note: `Strip: color:, skills: fields` and `Escape: ${VAR} → $VAR`
- v2.7 plan Phase 47-02 — explicit note: `Strip name: field from frontmatter (OpenCode uses filename)`

### Project History Evidence (HIGH confidence)
- `.planning/PROJECT.md` — Codex adapter was added in v2.4 without hook support (constraint: no hooks)
- MEMORY.md — "Copilot CLI does not support global hooks (Issue #1157)"
- v2.4 decision record: `Codex adapter (no hooks) — Codex lacks hook support; skip hook-dependent tests`

### Known Risks Flagged in Existing Code (HIGH confidence)
- Copilot README: "sessionStart fires per-prompt (Bug #991)" — known runtime quirk that a hook converter must preserve workarounds for
- Copilot README: "toolArgs parsing errors — Copilot CLI sends toolArgs as a JSON-encoded string, not a JSON object" — hook script double-parse logic must be preserved in generated scripts

---
*Pitfalls research for: multi-runtime plugin installer (v2.7)*
*Researched: 2026-03-16*
