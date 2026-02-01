# CLAUDE.md

Project-specific instructions for Claude Code when working in this repository.

## Git Workflow

**IMPORTANT: Never commit directly to `main` branch.**

- Always create a feature branch for new work
- Branch naming: `feature/<phase-or-feature-name>` (e.g., `feature/phase-11-bm25-planning`)
- Create PRs for all changes
- Merge to main only through approved PRs

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
cargo clippy -- -D warnings

# Build specific crate
cargo build -p memory-daemon
```

## GSD Workflow

This project uses the Get Shit Done (GSD) workflow:
- Planning files in `.planning/`
- ROADMAP.md tracks phases
- STATE.md tracks current position
- PLAN.md files define executable tasks

## Key Decisions

See `.planning/PROJECT.md` for architectural decisions and requirements.
