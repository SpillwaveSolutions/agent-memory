# Technology Stack: v2.7 Multi-Runtime Installer (memory-installer crate)

**Domain:** Rust CLI tool — plugin format converter / installer
**Researched:** 2026-03-16
**Confidence:** HIGH

## Executive Summary

The v2.7 milestone adds a new `memory-installer` Rust crate to the existing workspace. Its job is to read a canonical Claude plugin directory (YAML frontmatter + markdown), convert it to runtime-specific formats (OpenCode, Gemini, Codex, Copilot, generic skills), and write the output to the correct install directories.

**New dependencies required:** `gray_matter` (frontmatter parsing) and `walkdir` (directory traversal). Everything else is already in the workspace.

**Key finding: `serde_yaml` is deprecated (0.9.34+deprecated, March 2024).** Do not add it. Use `gray_matter` 0.3.x which uses `yaml-rust2` internally. The workspace already has `toml = "0.8"` (now 1.0.6 — consider upgrading), `clap = "4.5"`, `serde`, `serde_json`, `shellexpand`, `anyhow`, `thiserror`, and `directories`.

**Node.js vs Rust decision:** The GSD project uses a Node.js installer (install.js) because Node is ubiquitous in the GSD ecosystem. For agent-memory, a Rust binary is strictly better: it is part of the existing workspace build, ships as a single cross-compiled binary alongside `memory-daemon`, requires no Node.js runtime on the target machine, and integrates naturally with the existing CI/CD release pipeline. The installer logic (string manipulation, file I/O, TOML generation) is trivially expressible in Rust.

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `clap` | 4.5 (workspace) | CLI interface: `install-agent --agent <runtime> --project|--global --dry-run` | Already in workspace, derive macros, well-maintained |
| `gray_matter` | 0.3.2 | Parse `--- YAML ---` frontmatter from `.md` files | Only actively maintained frontmatter crate; uses yaml-rust2 internally; supports YAML, JSON, TOML; released July 2025 |
| `walkdir` | 2.5.0 | Recursive directory traversal for plugin source tree | Standard crate for this purpose; 218M downloads; last stable March 2024 |
| `toml` | 0.8 (workspace, consider 1.0.6) | Serialize Gemini TOML command files | Already in workspace; v1.0.6 is latest stable (March 2026) with TOML 1.1 spec |
| `serde` + `serde_json` | 1.0 (workspace) | Serialize/deserialize frontmatter values, write JSON config files | Already in workspace |
| `shellexpand` | 3.1 (already in memory-daemon) | Expand `~/.claude/` → absolute paths for install targets | Already pulled transitively; add to installer Cargo.toml |
| `directories` | 6.0 (workspace) | Cross-platform config/home directory resolution | Already in workspace; used by memory-daemon |
| `anyhow` + `thiserror` | 1.0 / 2.0 (workspace) | Error handling through converter pipeline | Workspace standard |

### Supporting Libraries (Already in Workspace — No New Adds)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_json` | 1.0 | Write OpenCode `opencode.json` permissions, Gemini `settings.json` | Merging JSON config files at install time |
| `tempfile` | 3.15 | Temporary dirs for integration tests | Test-only; already in workspace dev-dependencies |
| `dirs` | 5 | Alternative home-dir lookup if `directories` is too heavy | Already in workspace; `directories` preferred for full path resolution |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo test -p memory-installer` | Unit + integration tests for each converter | Standard workspace test runner |
| `cargo clippy --workspace` | Enforced via `task pr-precheck` | All warnings as errors; no exceptions |
| `cargo build --release` | Release binary alongside `memory-daemon` | Add `memory-installer` to release workflow targets |

---

## Installer Cargo.toml (Recommended)

```toml
[package]
name = "memory-installer"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "memory-installer"
path = "src/main.rs"

[dependencies]
# Frontmatter parsing — NEW dependency
gray_matter = { version = "0.3", features = ["yaml"] }

# Directory traversal — NEW dependency
walkdir = "2.5"

# Everything else from workspace
clap = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
directories = { workspace = true }
shellexpand = "3.1"

[dev-dependencies]
tempfile = { workspace = true }
```

**Workspace `Cargo.toml` additions:**

```toml
# Add to [workspace.dependencies]:
gray_matter = { version = "0.3", features = ["yaml"] }
walkdir = "2.5"

# Add to [workspace] members:
"crates/memory-installer"
```

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `gray_matter` 0.3.2 | `serde_yaml` 0.9.34 | `serde_yaml` is **deprecated** (March 2024, marked `+deprecated`). No future maintenance. |
| `gray_matter` 0.3.2 | `yaml-rust2` 0.11.0 directly | `gray_matter` wraps yaml-rust2 and handles the `---` delimiter splitting. Rolling custom frontmatter splitting adds brittle code with no benefit. |
| `gray_matter` 0.3.2 | `serde_yml` 0.0.12 | `serde_yml` is a fork of the deprecated serde_yaml with serde integration. Last release August 2024. Lower community trust than yaml-rust2 directly. Also does not handle frontmatter delimiter extraction. |
| `gray_matter` 0.3.2 | `frontmatter-gen` | Smaller community (not on lib.rs top results), less proven. gray_matter has 97K recent downloads vs unknown. |
| Rust binary (`memory-installer`) | Node.js script (like GSD `install.js`) | Node.js requires the user to have Node installed. The agent-memory ecosystem is Rust-first with cross-compiled release binaries for macOS/Linux/Windows. A Rust binary integrates with existing CI/release pipeline. The GSD installer uses Node because GSD targets developers who definitely have Node; agent-memory targets general users who should only need the downloaded binary. |
| `toml` (existing) | Hand-written TOML serialization | `toml` 0.8/1.0 is already in the workspace and handles all Gemini TOML generation correctly. |
| `walkdir` 2.5 | `std::fs::read_dir` recursively | `walkdir` handles recursion, symlink policies, and iterator errors cleanly. `read_dir` requires manual recursion. `walkdir` is the standard choice with 218M downloads. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `serde_yaml` | Officially deprecated March 2024 (version is `0.9.34+deprecated`). Author (dtolnay) stopped maintaining it. | `gray_matter` with `features = ["yaml"]` which uses `yaml-rust2` internally |
| `yaml-rust` (original) | Unmaintained; abandoned. | `yaml-rust2` (the maintained fork) — but prefer `gray_matter` to avoid manual frontmatter splitting |
| Tokio async runtime | The installer is a one-shot CLI; async adds no value over synchronous I/O. File copies and string transforms do not benefit from async. | Synchronous `std::fs` — no `tokio` dependency in this crate |
| `tonic` / gRPC in installer | The installer does not communicate with the daemon. It is a standalone file conversion tool. | None needed |
| `regex` crate | Tool name mapping (Claude→OpenCode→Gemini) is a static lookup table (`BTreeMap<&str, &str>`). Regex adds complexity with no gain. | `HashMap` / `BTreeMap` for tool name maps |
| `tera` or `handlebars` templating | Templates in this crate are trivial string substitutions (path rewriting). A full template engine is overkill. | String replacement via `.replace()` or `format!()` |

---

## Stack Patterns by Variant

**For frontmatter extraction from `.md` files:**
- Use `gray_matter::Matter::<gray_matter::engine::YAML>::new().parse(&content)`
- Returns `ParsedEntity { data: Option<Pod>, content: String, excerpt: Option<String> }`
- `data` is the parsed YAML as a `gray_matter::Pod` (serde-compatible value type)
- `content` is the markdown body after the `---` delimiters

**For TOML output (Gemini converter):**
- Deserialize frontmatter fields into a typed struct
- Serialize with `toml::to_string_pretty()`
- Use `toml::Value` for dynamic/unknown fields rather than typed structs

**For JSON output (OpenCode `opencode.json`, Gemini `settings.json`):**
- Read existing file with `serde_json::from_str()` (if it exists)
- Merge changes into `serde_json::Value`
- Write back with `serde_json::to_string_pretty()`

**For path resolution:**
- Use `directories::ProjectDirs` to find `~/.config/agent-memory/`
- Use `shellexpand::tilde()` for `~/.claude/`, `~/.gemini/`, etc. in plugin paths
- The `--project` flag resolves to the current working directory

**For dry-run support:**
- Thread a `dry_run: bool` flag through the converter trait
- Log what would be written with `tracing::info!()` instead of writing

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `gray_matter` 0.3.x | `yaml-rust2` ^0.10 (pulled transitively) | gray_matter 0.3.2 requires yaml-rust2 0.10+; yaml-rust2 0.11.0 is available but gray_matter pins ^0.10 |
| `toml` 0.8 (workspace) | Existing memory-daemon | Workspace already pins 0.8; upgrading to 1.0.6 is safe but requires testing across all crates that use toml. Can defer. |
| `walkdir` 2.5 | No Rust edition constraint | Pure library, no async, compatible with edition 2021 |
| `clap` 4.5 (workspace) | All existing binaries | Shared across memory-daemon, memory-client; no conflict adding to memory-installer |

---

## What NOT to Add (Dependency Hygiene)

This is a file-manipulation CLI crate. It should have minimal dependencies.

| Anti-Pattern | Reason | What to Do Instead |
|--------------|--------|-------------------|
| `tokio` | No async I/O needed; one-shot CLI | Use `std::fs` synchronously |
| `tonic` | Installer does not talk to the daemon | Omit entirely |
| `candle-core` / ML crates | No inference in installer | Omit entirely |
| `rocksdb` | No database access | Omit entirely |
| `tantivy` | No search index | Omit entirely |
| `reqwest` | No HTTP requests | Omit entirely |
| Memory workspace crates (`memory-types`, `memory-storage`, etc.) | Installer has no shared types with the daemon | Omit entirely — installer is a standalone tool |

The installer crate should have exactly **2 new external dependencies** (`gray_matter`, `walkdir`). Everything else comes from the workspace or `std`.

---

## Sources

- crates.io API: `serde_yaml` — confirmed deprecated at 0.9.34+deprecated (March 2024)
- crates.io API: `gray_matter` — latest 0.3.2, released July 10 2025, actively maintained
- crates.io API: `walkdir` — latest 2.5.0, released March 1 2024, 218M downloads
- crates.io API: `yaml-rust2` — latest 0.11.0, released December 16 2025
- crates.io API: `toml` — latest 1.0.6+spec-1.1.0, released March 6 2026
- crates.io API: `shellexpand` — latest 3.1.2, released February 23 2026
- docs.rs: gray_matter 0.3.2 — confirmed uses yaml-rust2 internally; `features = ["yaml"]` required
- Rust Forum: serde-yaml deprecation thread — confirmed community migration to yaml-rust2 and serde_yml
- Workspace `Cargo.toml` — verified existing: clap 4.5, toml 0.8, serde 1.0, serde_json 1.0, anyhow 1.0, thiserror 2.0, directories 6.0, tempfile 3.15
- `crates/memory-daemon/Cargo.toml` — verified shellexpand 3.1 already in workspace (transitively available)

---
*Stack research for: v2.7 memory-installer crate — multi-runtime plugin format converter*
*Researched: 2026-03-16*
