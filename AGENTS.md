# AGENTS.md

Agent-specific instructions for AI coding assistants working in this repository.

## Git Workflow Requirements

**CRITICAL: Never commit directly to `main` branch.**

1. Always create a feature branch before making changes
2. Use branch naming convention: `feature/<descriptive-name>`
3. All changes must go through Pull Requests
4. Never push directly to main

### Branch Workflow

```bash
# Create feature branch
git checkout -b feature/my-feature

# Make changes and commit
git add <files>
git commit -m "type(scope): message"

# Push and create PR
git push -u origin feature/my-feature
gh pr create --title "..." --body "..."
```

## Commit Message Format

Use conventional commits:
- `feat(scope): description` - New feature
- `fix(scope): description` - Bug fix
- `docs(scope): description` - Documentation
- `refactor(scope): description` - Code refactoring
- `test(scope): description` - Test additions

## Project Context

This is the agent-memory project - a local, append-only conversational memory system with:
- TOC-based agentic navigation
- gRPC service architecture
- Rust workspace with multiple crates
- GSD (Get Shit Done) workflow for planning

## Plan Storage

**IMPORTANT: All phase plans and RFCs must be stored in `docs/plans/`.**

- Phase plans: `docs/plans/phase-<N>-<name>-plan.md`
- RFCs: `docs/plans/<name>-rfc.md`
- Research docs: `docs/plans/<name>-research.md`

Do NOT leave plans only in `~/.claude/plans/` - always copy the final plan to `docs/plans/`.

## Before Starting Work

1. Check current branch: `git branch --show-current`
2. If on main, create feature branch first
3. Read `.planning/STATE.md` for current project status
4. Review relevant PLAN.md files before implementation
