# Feature Research

**Domain:** Multi-runtime plugin installer (agent-memory v2.7)
**Researched:** 2026-03-16
**Confidence:** HIGH (based on GSD installer source analysis + v2.7 implementation plan)

## Context

This research focuses exclusively on the installer itself — not the plugin content being installed. The installer reads a canonical Claude plugin source tree and converts it into runtime-specific installations for Claude, OpenCode, Gemini, Codex, Copilot, and generic skill runtimes.

Reference: GSD installer at `/Users/richardhightower/src/get-shit-done/bin/install.js` (1600+ LOC Node.js) is the closest real-world reference for this exact problem. Key observations from source analysis:

- GSD uses `--claude|--opencode|--gemini|--codex|--copilot|--antigravity|--all` flags for runtime selection
- GSD uses `--global|-g` and `--local|-l` for scope selection
- GSD uses `--uninstall|-u` for removal
- GSD uses managed-section markers (e.g. `# GSD Agent Configuration — managed by get-shit-done installer`) to safely inject into shared config files (Codex `config.toml`, Copilot instructions)
- GSD handles three-case config merging: (1) new file, (2) existing with markers, (3) existing without markers
- GSD performs per-runtime frontmatter conversion, tool name mapping, path rewriting, and content attribution
- GSD does NOT have `--dry-run` — identified as a gap
- GSD does NOT have `--output json` — identified as a gap
- GSD handles XDG base directory compliance for OpenCode via 4-priority env var chain
- GSD handles WSL + Windows-native Node detection; Rust handles this naturally via `std::path::Path`

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Runtime selection (`--agent <runtime>`) | Every installer lets you choose what to install | LOW | `claude\|opencode\|gemini\|codex\|copilot\|skills`; already in v2.7 plan |
| Scope selection (`--project\|--global`) | Users need per-project vs system-wide control | LOW | Maps to different target dirs per runtime; already in v2.7 plan |
| Custom target dir (`--dir <path>`) | Power users and non-standard setups need override | LOW | `--config-dir` in GSD; needed for generic `skills` runtime and non-default setups |
| Tool name mapping per runtime | Core conversion — PascalCase Claude → snake_case Gemini etc. | MEDIUM | 5 separate static mapping tables; already defined in v2.7 plan `tool_maps.rs` |
| Frontmatter format conversion | Each runtime uses different YAML/TOML schemas | MEDIUM | YAML→TOML for Gemini; `allowed-tools` array → `tools` object for OpenCode; strip `color:` for Gemini |
| Path rewriting in all installed content | `~/.claude/` must become runtime-appropriate path | MEDIUM | GSD handles 4 regex patterns per runtime (`~/.claude/`, `$HOME/.claude/`, `./.claude/`, runtime-specific prefix); forward-slash enforcement for cross-platform |
| Idempotent re-install (upgrade) | Running installer twice should not break things | MEDIUM | GSD deletes then recreates managed dirs; merges managed sections in config files with markers; stale commands removed automatically |
| Uninstall (`--uninstall`) | Users need a clean removal path | MEDIUM | Requires managed-section markers written during install to safely identify what to remove |
| `--dry-run` mode | Preview what would be installed without writing files | LOW | GSD does NOT have this — identified gap; implement via write-interceptor on `ConvertedFile` output; high value, low cost |
| Help text (`--help`) | Every CLI tool needs this | LOW | Implicit in clap CLI; but needs good runtime/scope examples |
| Install-all shortcut (`--all`) | Install for every runtime at once | LOW | GSD supports `--all` flag; maps to running each converter in sequence |
| Env var config dir override | Runtimes use `CODEX_HOME`, `OPENCODE_CONFIG_DIR`, `XDG_CONFIG_HOME`, etc. | LOW | GSD implements 4-5 priority resolution per runtime; needed for non-default setups |
| Clean orphan removal | Reinstalling after removing a command must delete stale files | MEDIUM | GSD deletes managed subdirs before fresh copy; prevents stale `skill-name/` dirs from old versions |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Managed-section merging for shared config files | Installer must inject into `config.toml`, `settings.json`, `opencode.json` without destroying user content | HIGH | GSD implements three-case logic: (1) new file, (2) existing with markers, (3) existing without markers. Critical for Codex `config.toml` and Gemini `settings.json` |
| Marker-based owned-section tracking | Lets installer safely upgrade its sections without touching user config above/below markers | MEDIUM | GSD markers: `# GSD Agent Configuration — managed by...`. Enables safe uninstall. Markers must be decided before first release — format is a compatibility contract |
| Per-runtime hook format conversion | Hooks are the most runtime-specific artifact — Claude YAML vs Gemini JSON vs OpenCode TypeScript plugin | HIGH | GSD installs JS hook files with path templating; agent-memory needs shell script hooks. This is a key differentiator since most installers skip hooks entirely |
| Cross-reference rewriting in body text | `/memory:search` → `/memory-search` for runtimes without namespace syntax | LOW | GSD rewrites `/gsd:` → `/gsd-` for Copilot/Codex. Same pattern needed for memory commands |
| Codex skill adapter header injection | Codex lacks slash-commands; skills need an adapter header explaining `$skill-name` invocation pattern and tool translation | MEDIUM | GSD injects a `<codex_skill_adapter>` block translating AskUserQuestion → request_user_input, Task → spawn_agent. Memory installer needs equivalent memory-specific adapter guidance |
| Codex `config.toml` agent registration | Codex requires explicit `[agents.name]` entries with `sandbox_mode` and `config_file` | MEDIUM | GSD generates per-agent `.toml` files + writes `[agents.name]` config entries; sandbox mode from lookup table (`workspace-write` vs `read-only`) |
| XDG base directory compliance for OpenCode | OpenCode follows XDG spec — installer must respect `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG`, `XDG_CONFIG_HOME` env var priority chain | LOW | GSD implements 4-priority resolution. Missing this breaks OpenCode on Linux where XDG_CONFIG_HOME is non-default |
| Attribution/metadata stripping per runtime | Some runtimes reject fields others require (e.g., `color:` crashes Gemini validation, `name:` is redundant in OpenCode) | LOW | GSD strips `color:` for Gemini, `name:` for OpenCode (uses filename), adds `metadata.short-description` for Codex description truncation |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Backup/restore of user customizations | "Don't overwrite my changes" | Installer cannot distinguish user content vs managed content without markers; backup files pile up and are never restored | Use managed-section markers so installer only owns its section; user content above/below is never touched |
| Interactive prompts for runtime/scope when no flags given | GSD shows interactive readline prompts when run with no flags | Unusable in CI, scripts, agent-driven workflows; adds readline/terminal dependency | Require `--agent` and `--project\|--global` flags; add optional interactive mode in v1.x post-MVP as a convenience layer on top of the same flag-driven core |
| Version tracking file | "Write a `.memory-installer-version` file" | Cross-platform path fragility; users delete it; file becomes stale | Installer regenerates from canonical source on every run — idempotent by design means version tracking is not needed |
| Separate plugin validation subcommand | "Validate canonical source before installing" | High complexity; different runtimes accept different schemas | Phase 46 `parser.rs` naturally catches malformed frontmatter during parse; surface errors then, not as a separate validate step |
| Rollback on partial failure | "Undo if Gemini install fails after Claude succeeds" | Each runtime is independent; partial success is still useful | Atomic per-runtime install (delete then write); if one runtime fails, report error and continue others; user can re-run for failed runtime |
| GUI or TUI wizard mode | "Add interactive installation wizard" | Significant complexity for marginal gain; agent-driven users use CLI flags | Clear `--help` output and `--dry-run` preview covers the use case without the complexity |
| Checksum verification of installed files | "Verify installed files match canonical source" | Files are intentionally transformed so hash of source != hash of installed | Idempotent reinstall IS the verification mechanism — re-run and compare output |

---

## Feature Dependencies

```
[Runtime Selection (--agent)]
    └──requires──> [Tool map for that runtime]   (tool_maps.rs)
                       └──requires──> [Frontmatter Parser]   (parser.rs)
                                          └──requires──> [Directory Walker]   (walkdir)

[Uninstall (--uninstall)]
    └──requires──> [Managed-section markers]   (written during install; must exist first)

[Managed-section merging for shared config files]
    └──requires──> [Marker strategy decided before v2.7 ships]

[Codex config.toml registration]
    └──requires──> [Agent TOML generator]
                       └──requires──> [Sandbox mode lookup table]

[Hook conversion pipeline]
    └──requires──> [Per-runtime hook format knowledge]
    └──enhances──> [Managed-section merging]   (settings.json / hooks config injection)

[--dry-run mode]
    └──enhances──> [All converters]   (converters produce ConvertedFile structs; dry-run prints instead of writes)

[--all flag]
    └──requires──> [Each individual runtime converter]

[Clean orphan removal]
    └──requires──> [Converter knows its output dir prefix]
```

### Dependency Notes

- **Managed-section markers must be decided before first release:** Once markers are in the wild, changing the format breaks uninstall for existing users. Decide marker strings in Phase 46 before any production installs.
- **Frontmatter parser is foundational:** Every converter depends on it. Invest in robustness here — multiline values, quoted strings, YAML arrays vs inline, serde_yaml is better than regex.
- **Dry-run via write-interceptor:** Implement as a flag on the converter output stage, not per-converter. Each converter returns `Vec<ConvertedFile>`; dry-run mode prints path + content summary instead of writing.
- **Uninstall requires markers:** Without managed-section markers in shared config files (Codex `config.toml`, Gemini `settings.json`), uninstall cannot safely remove injected sections without corrupting user config.
- **Hook conversion depends on per-runtime format knowledge:** Claude YAML, Gemini JSON settings merge, OpenCode TypeScript plugin, Copilot JSON hooks, Codex script-based. All 5 formats must be understood before Phase 49.

---

## MVP Definition

### Launch With (v1 — Phases 45-50 as planned)

Minimum viable product for v2.7 milestone.

- [ ] Runtime selection (`--agent claude|opencode|gemini|codex|copilot|skills`) — core value
- [ ] Scope selection (`--project|--global`) with correct target dirs per runtime
- [ ] Tool name mapping for all 5 runtimes (static maps from v2.7 plan)
- [ ] Frontmatter conversion per runtime (YAML→TOML for Gemini, `allowed-tools`→`tools` for OpenCode, strip `color:` for Gemini)
- [ ] Path rewriting in all installed content (Claude paths → runtime-appropriate)
- [ ] Clean orphan removal (delete managed dirs before fresh copy)
- [ ] Idempotent re-install (running twice leaves same result)
- [ ] `--dry-run` flag (print planned writes without executing) — low complexity, high value, GSD gap
- [ ] `--all` flag (install for every runtime in sequence)
- [ ] Managed-section markers in shared config files (Codex `config.toml`, Gemini `settings.json`)
- [ ] Uninstall (`--uninstall`) using managed-section markers
- [ ] Hook conversion per runtime (Phase 49)
- [ ] Codex skill adapter header injection
- [ ] Codex `config.toml` agent registration with sandbox mode

### Add After Validation (v1.x)

Features to add once core converters are working.

- [ ] Interactive mode (when no flags provided, prompt for runtime and scope) — add after CLI is stable; don't block v2.7
- [ ] `--config-dir` override for all runtimes (XDG compliance for OpenCode, `CODEX_HOME` for Codex, etc.) — needed by power users; straightforward to add
- [ ] Cross-reference rewriting (`/memory:search` → `/memory-search` for non-namespace runtimes) — add when testing per-runtime CLI output

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] Plugin validation subcommand (`memory-installer validate`) — useful but not blocking launch
- [ ] JSON output mode (`--output json`) for machine-readable install report — add if agents need to parse installer output programmatically
- [ ] Multi-plugin support (install multiple canonical sources) — defer until a second plugin exists

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Runtime + scope selection | HIGH | LOW | P1 |
| Tool name mapping (all 5 runtimes) | HIGH | MEDIUM | P1 |
| Frontmatter conversion per runtime | HIGH | MEDIUM | P1 |
| Path rewriting in content | HIGH | LOW | P1 |
| Clean orphan removal | HIGH | LOW | P1 |
| Idempotent re-install | HIGH | MEDIUM | P1 |
| Dry-run mode | HIGH | LOW | P1 |
| Managed-section markers + merging | HIGH | MEDIUM | P1 |
| Uninstall | HIGH | MEDIUM | P1 |
| Hook conversion pipeline | HIGH | HIGH | P1 |
| Codex skill adapter injection | MEDIUM | MEDIUM | P1 |
| Codex config.toml registration | MEDIUM | MEDIUM | P1 |
| `--all` flag | MEDIUM | LOW | P1 |
| Env var config dir override | MEDIUM | LOW | P2 |
| `--config-dir` explicit override | MEDIUM | LOW | P2 |
| Cross-reference rewriting | LOW | LOW | P2 |
| Interactive mode (no-flag fallback) | MEDIUM | MEDIUM | P2 |
| JSON output mode | LOW | LOW | P3 |
| Plugin validation subcommand | LOW | MEDIUM | P3 |

**Priority key:**
- P1: Must have for v2.7 launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | GSD installer (JS reference) | Our approach (Rust) |
|---------|-------------------------------|---------------------|
| Dry-run | NOT IMPLEMENTED — gap | Implement via `ConvertedFile` write-interceptor; print path + content preview |
| Uninstall | `--uninstall` with marker-based section removal | Same pattern, markers embedded in generated config sections |
| Managed-section merging | Three-case logic per shared config file | Same logic in Rust with `std::fs` + string matching |
| Tool name mapping | Hardcoded JS maps per runtime | `tool_maps.rs` with static `HashMap` or `match`; same approach |
| Frontmatter parsing | Regex-based (fragile for multiline values) | `serde_yaml` for robust parsing — improvement over GSD |
| Path rewriting | 4 regex replacements per runtime | Same regex strategy; compile patterns once |
| Hook conversion | JS hook files copied with path templating | Shell scripts + per-runtime config injection (YAML/JSON/settings.json) |
| Codex adapter header | `<codex_skill_adapter>` injected into every SKILL.md | Same pattern with memory-specific translation guidance |
| Codex config.toml | Per-agent TOML files + `[agents.name]` registration | Same approach |
| XDG compliance for OpenCode | 4-priority env var resolution | Same logic; Rust has no XDG library needed — simple env var checks |
| WSL detection | Explicit WSL + Windows-native Node detection + exit 1 | Not needed; Rust `std::path::Path` cross-platform handles this; forward-slash enforcement for hook script paths |
| Attribution processing | Per-runtime `Co-Authored-By` strip/replace | Not needed for agent-memory; no commit attribution in plugin content |
| `--all` flag | Supported | Supported |
| Interactive mode | Readline prompts when no flags given | Post-MVP; v2.7 requires explicit flags |

---

## Sources

- GSD installer source: `/Users/richardhightower/src/get-shit-done/bin/install.js` — direct source analysis (HIGH confidence)
- v2.7 implementation plan: `docs/plans/v2.7-multi-runtime-portability-plan.md` — project-owned (HIGH confidence)
- Project context: `.planning/PROJECT.md` — project-owned (HIGH confidence)

---
*Feature research for: multi-runtime plugin installer (agent-memory v2.7)*
*Researched: 2026-03-16*
