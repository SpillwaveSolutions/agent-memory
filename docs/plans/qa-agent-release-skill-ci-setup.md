# Plan: QA Agent, Release Skill, and CI/CD Setup for agent-memory

## Summary

Set up comprehensive Rust QA infrastructure for the agent-memory project:
1. Install external skills for Rust testing and cargo assistance
2. Create local `qa-rust-agent` in `.claude/agents/`
3. Create local `releasing-rust` skill in `.claude/skills/`
4. Create GitHub Actions CI workflow for PR checks
5. Create GitHub Actions release workflow for multi-platform builds

All agents and skills are **project-local** (stored in `.claude/`), not global plugins.

---

## Phase 1: Install External Skills

### 1.1 Install rust-testing skill
```bash
skilz install attunehq/hurry/rust-testing -p --agent claude
```

### 1.2 Install rust-cargo-assistant skill
```bash
skilz install CuriousLearner/devkit/rust-cargo-assistant -b --agent claude
```

**Note**: If these skills don't exist in the marketplace, create local equivalents in Phase 2.

---

## Phase 2: Create Local QA Agent

### File: `.claude/agents/qa-rust-agent.md`

Create a Rust-specific QA agent that:
- Triggers after any `.rs` file changes
- Runs `cargo check`, `cargo clippy`, `cargo test`
- Validates documentation builds
- Enforces workspace lint rules (unwrap_used, expect_used, panic, todo = deny)
- Reports per-crate test results
- Blocks completion on test failures

**Key Commands the Agent Will Execute:**
```bash
cargo check --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test -p memory-daemon --test integration_test -- --test-threads=1
cargo doc --no-deps --all-features
```

---

## Phase 3: Create Local Skills

### 3.1 Create `rust-testing` skill (if external not available)

**Directory**: `.claude/skills/rust-testing/`

**Files to create:**
- `SKILL.md` - Testing patterns, commands, fixtures
- `.skilz-manifest.yaml` - Local skill metadata

**Content covers:**
- `cargo test` command variations
- Async test patterns with tokio
- Property-based testing with proptest
- Test fixtures with tempfile
- Integration test harness patterns
- Coverage with cargo-tarpaulin

### 3.2 Create `releasing-rust` skill

**Directory**: `.claude/skills/releasing-rust/`

**Files to create:**
- `SKILL.md` - Release workflow, cross-compilation, versioning

**Content covers:**

#### Cross-Compilation Targets
| Platform | Target Triple | Runner |
|----------|---------------|--------|
| Linux x86_64 | x86_64-unknown-linux-gnu | ubuntu-latest |
| Linux ARM64 | aarch64-unknown-linux-gnu | ubuntu-latest + cross |
| macOS Intel | x86_64-apple-darwin | macos-13 |
| macOS Apple Silicon | aarch64-apple-darwin | macos-14 |
| Windows x86_64 | x86_64-pc-windows-msvc | windows-latest |

#### Artifact Naming Convention
```
memory-daemon-{version}-{platform}-{arch}.{ext}

Examples:
  memory-daemon-0.2.0-linux-x86_64.tar.gz
  memory-daemon-0.2.0-macos-aarch64.tar.gz
  memory-daemon-0.2.0-windows-x86_64.zip
```

#### Version Management
- Use `cargo set-version` (cargo-edit)
- Semantic versioning (MAJOR.MINOR.PATCH)
- Changelog generation with git-cliff

#### Integration with other skills
- References `rust-cargo-assistant` for dependency management
- References `git-cli` for tagging
- References `mastering-github-cli` for release creation

---

## Phase 4: Create GitHub CI Workflow

### File: `.github/workflows/ci.yml`

**Jobs:**
| Job | Purpose | Runs On |
|-----|---------|---------|
| `fmt` | Format check | ubuntu-latest |
| `clippy` | Lint check | ubuntu-latest |
| `test` | Run tests | ubuntu-latest, macos-latest |
| `build` | Build binaries | ubuntu-latest, macos-latest |
| `doc` | Documentation check | ubuntu-latest |

**Triggers:**
- Push to `main`
- Pull requests targeting `main`

**Dependencies installed:**
- `protobuf-compiler` (proto compilation)
- `libclang-dev` (RocksDB, usearch bindgen)

**Caching:**
- Use `Swatinem/rust-cache@v2` with shared keys

---

## Phase 5: Create GitHub Release Workflow

### File: `.github/workflows/release.yml`

**Triggers:**
- Push tags matching `v[0-9]+.[0-9]+.[0-9]+`
- Manual workflow_dispatch with version input

**Build Matrix:**
```yaml
matrix:
  include:
    - target: x86_64-unknown-linux-gnu
      os: ubuntu-latest
      name: linux-x86_64
    - target: aarch64-unknown-linux-gnu
      os: ubuntu-latest
      name: linux-aarch64
      cross: true
    - target: x86_64-apple-darwin
      os: macos-13
      name: macos-x86_64
    - target: aarch64-apple-darwin
      os: macos-14
      name: macos-aarch64
    - target: x86_64-pc-windows-msvc
      os: windows-latest
      name: windows-x86_64
```

**Outputs:**
- Platform-specific archives (tar.gz for Unix, zip for Windows)
- SHA256 checksums file
- GitHub Release with all artifacts

---

## Phase 6: Create Supporting Configuration Files

### 6.1 `rust-toolchain.toml`
```toml
[toolchain]
channel = "1.83"
components = ["rustfmt", "clippy"]
```

### 6.2 `.rustfmt.toml`
```toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

### 6.3 `clippy.toml`
```toml
cognitive-complexity-threshold = 25
too-many-lines-threshold = 200
too-many-arguments-threshold = 7
```

---

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `.claude/agents/qa-rust-agent.md` | Create | QA enforcement agent |
| `.claude/skills/rust-testing/SKILL.md` | Create | Testing patterns skill |
| `.claude/skills/rust-testing/.skilz-manifest.yaml` | Create | Skill metadata |
| `.claude/skills/releasing-rust/SKILL.md` | Create | Release workflow skill |
| `.claude/skills/releasing-rust/.skilz-manifest.yaml` | Create | Skill metadata |
| `.github/workflows/ci.yml` | Create | PR checks workflow |
| `.github/workflows/release.yml` | Create | Multi-platform release workflow |
| `rust-toolchain.toml` | Create | Pin Rust version |
| `.rustfmt.toml` | Create | Format configuration |
| `clippy.toml` | Create | Lint configuration |
| `CLAUDE.md` | Update | Document new agents/skills |
| `.claude/settings.local.json` | Update | Add new permissions |

---

## Verification Plan

### After Implementation:

1. **Verify skill installation:**
   ```bash
   ls -la .claude/skills/
   ```

2. **Test CI workflow locally:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

3. **Test release build:**
   ```bash
   cargo build --release --bin memory-daemon
   ```

4. **Create test PR to verify CI:**
   ```bash
   git checkout -b test/ci-workflow
   git add .github/
   git commit -m "ci: add PR checks workflow"
   gh pr create --title "CI: Add PR checks" --body "Test CI workflow"
   ```

5. **Verify PR checks run:**
   ```bash
   gh pr checks
   ```

---

## Execution Order

1. Install external skills (or note if unavailable)
2. Create `.claude/agents/` directory and `qa-rust-agent.md`
3. Create `.claude/skills/rust-testing/` with SKILL.md
4. Create `.claude/skills/releasing-rust/` with SKILL.md
5. Create `.github/workflows/` directory
6. Create `ci.yml` workflow
7. Create `release.yml` workflow
8. Create `rust-toolchain.toml`, `.rustfmt.toml`, `clippy.toml`
9. Update `CLAUDE.md` to document new infrastructure
10. Update `.claude/settings.local.json` with new permissions
11. Test locally with cargo commands
12. Create PR to test CI workflow
