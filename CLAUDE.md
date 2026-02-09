# CLAUDE.md

Project-specific instructions for Claude Code when working in this repository.

## Git Workflow

**IMPORTANT: Never commit directly to `main` branch.**

- Always create a feature branch for new work
- Branch naming: `feature/<phase-or-feature-name>` (e.g., `feature/phase-11-bm25-planning`)
- Create PRs for all changes
- Merge to main only through approved PRs

## PR Pre-Push Validation

**CRITICAL: Always run `task pr-precheck` before pushing a PR.**

This validates format, clippy, tests, and docs match CI expectations:

```bash
# Run BEFORE git push for PRs
task pr-precheck
```

If `task` is not available, run manually:

```bash
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --workspace --all-features && \
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features
```

**Never push a PR without passing pr-precheck first.**

## Project Structure

This is an agent-memory system built in Rust with:
- `crates/` - Rust workspace crates
- `proto/` - gRPC protocol definitions
- `.planning/` - GSD workflow planning files
- `skills/` - Claude Code plugin skills

## Build Commands

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Check with clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build specific crate
cargo build -p memory-daemon

# Full QA check (format + clippy + test + doc)
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --workspace --all-features && \
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features
```

## Local Skills

Project-specific skills in `.claude/skills/`:

| Skill | Purpose |
|-------|---------|
| `modern-rust-expert` | Rust 2024 patterns, clippy compliance, functional-but-pragmatic philosophy |
| `rust-testing` | Test patterns, assertions, parameterized tests |
| `rust-cargo-assistant` | Cargo commands and dependency management |
| `releasing-rust` | Cross-platform release workflow, versioning, artifact naming |

## Local Agents

Project-specific agents in `.claude/agents/`:

| Agent | Purpose |
|-------|---------|
| `qa-rust-agent` | Enforces code quality after Rust file changes (format, clippy, test, doc) |

## CI/CD Workflows

GitHub Actions workflows in `.github/workflows/`:

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | Push to main, PRs | Format, clippy, test, build, doc checks |
| `release.yml` | Tags `v*.*.*`, manual | Multi-platform release builds |

### Release Process

```bash
# Bump version
cargo set-version 0.2.0

# Commit and tag
git add -A && git commit -m "chore: release v0.2.0"
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin main --tags
```

The release workflow automatically builds for:
- Linux x86_64 / ARM64
- macOS Intel / Apple Silicon
- Windows x86_64

## GSD Workflow

This project uses the Get Shit Done (GSD) workflow:
- Planning files in `.planning/`
- ROADMAP.md tracks phases
- STATE.md tracks current position
- PLAN.md files define executable tasks

## Plan Storage

**IMPORTANT: All phase plans and RFCs must be stored in `docs/plans/`.**

- Phase plans: `docs/plans/phase-<N>-<name>-plan.md`
- RFCs: `docs/plans/<name>-rfc.md`
- Research docs: `docs/plans/<name>-research.md`

Do NOT leave plans only in `~/.claude/plans/` - always copy the final plan to `docs/plans/`.

## Key Decisions

See `.planning/PROJECT.md` for architectural decisions and requirements.
