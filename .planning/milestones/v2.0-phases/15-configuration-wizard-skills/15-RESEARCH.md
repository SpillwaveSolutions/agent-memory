# Phase 15: Configuration Wizard Skills - Research

**Researched:** 2026-02-01
**Domain:** Claude Code Skills with AskUserQuestion Interactive Wizards
**Confidence:** HIGH

## Summary

This phase creates three interactive configuration wizard skills (`/memory-storage`, `/memory-llm`, `/memory-agents`) that extend the existing `/memory-setup` plugin. The skills use Claude Code's `AskUserQuestion` tool to guide users through advanced configuration scenarios with multi-step question flows, state detection, and conditional skip logic.

Research confirms that the existing `memory-setup` skill provides a robust pattern to follow. The AskUserQuestion tool is well-documented and supports single/multi-select questions with headers, labels, and descriptions. Skills are defined in SKILL.md files with YAML frontmatter and markdown instructions.

**Primary recommendation:** Follow the existing memory-setup skill structure exactly, using AskUserQuestion for interactive prompts with proper state detection before each question to skip already-configured options.

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| Claude Code Skills | Current | Interactive wizard implementation | Native Claude Code capability |
| SKILL.md | - | Skill definition format | Standard Claude Code skill format |
| AskUserQuestion | - | Interactive user prompts | Built-in tool for multi-option selection |
| Bash | - | State detection and config generation | Standard for file/system checks |

### Supporting

| Component | Purpose | When to Use |
|-----------|---------|-------------|
| Write tool | Generate config files | After gathering user choices |
| Read tool | Load existing config | State detection phase |
| Glob tool | Find configuration files | State detection phase |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| AskUserQuestion | Multiple single-line prompts | AskUserQuestion better for structured options with descriptions |
| SKILL.md | Command files only | Skills provide richer context and reference docs |
| Bash state detection | gRPC calls | Bash is simpler, gRPC requires daemon running |

## Architecture Patterns

### Recommended Project Structure

```
plugins/memory-setup-plugin/skills/
├── memory-setup/          # Existing (Phase 9)
│   ├── SKILL.md
│   └── references/
├── memory-storage/        # NEW - Phase 15
│   ├── SKILL.md
│   └── references/
│       ├── retention-policies.md
│       ├── gdpr-compliance.md
│       └── archive-strategies.md
├── memory-llm/            # NEW - Phase 15
│   ├── SKILL.md
│   └── references/
│       ├── provider-comparison.md
│       ├── model-selection.md
│       ├── cost-estimation.md
│       └── custom-endpoints.md
└── memory-agents/         # NEW - Phase 15
    ├── SKILL.md
    └── references/
        ├── storage-strategies.md
        ├── team-setup.md
        └── agent-identifiers.md
```

### Pattern 1: SKILL.md Frontmatter Structure

**What:** Standard YAML frontmatter for skill metadata
**When to use:** Every skill file

**Example:**
```yaml
---
name: memory-storage
description: |
  This skill should be used when the user asks to "configure storage",
  "set up retention policies", "configure GDPR mode", "tune memory performance",
  or "change storage path". Provides interactive wizard for storage configuration.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---
```

Source: Verified from existing `/plugins/memory-setup-plugin/skills/memory-setup/SKILL.md`

### Pattern 2: AskUserQuestion Tool Usage

**What:** Structured questions with header, options, and descriptions
**When to use:** Each wizard step requiring user choice

**Example:**
```typescript
{
  questions: [
    {
      question: "How long should conversation data be retained?",
      header: "Retention",  // max 12 chars
      multiSelect: false,
      options: [
        {
          label: "Forever (Recommended)",
          description: "Keep all data permanently for maximum historical context"
        },
        {
          label: "90 days",
          description: "Quarter retention, good balance of history and storage"
        },
        {
          label: "30 days",
          description: "One month retention, lower storage usage"
        }
      ]
    }
  ]
}
```

Source: Context7 - Claude Code documentation `/anthropics/claude-code`

### Pattern 3: State Detection Before Questions

**What:** Check existing configuration before asking questions
**When to use:** At start of each wizard and before each step

**Example:**
```bash
# State detection commands
grep -A5 '\[storage\]' ~/.config/memory-daemon/config.toml 2>/dev/null | grep path
grep retention ~/.config/memory-daemon/config.toml 2>/dev/null
ls ~/.memory-archive 2>/dev/null
```

Source: Existing `memory-setup/SKILL.md` State Detection section

### Pattern 4: Flag Modes (--minimal, --advanced, --fresh)

**What:** Three execution modes for wizard behavior
**When to use:** Command invocation

| Flag | Behavior |
|------|----------|
| (none) | Standard mode - skip completed steps, show core options |
| `--minimal` | Use defaults for everything possible, minimal questions |
| `--advanced` | Show all options including expert settings |
| `--fresh` | Ignore existing config, ask all questions, backup before overwrite |

Source: Existing `memory-setup/references/wizard-questions.md`

### Pattern 5: Output Formatting Consistency

**What:** Standard visual formatting for wizard output
**When to use:** All wizard steps and status messages

**Status Indicators:**
| Symbol | Meaning |
|--------|---------|
| `[check]` | Success/Complete |
| `[x]` | Missing/Failed |
| `[!]` | Warning |
| `[?]` | Unknown |
| `[>]` | In Progress |

**Step Headers:**
```
Step N of 6: Step Name
----------------------
[Question or action content]
```

Source: Existing `memory-setup/SKILL.md` Output Formatting section

### Anti-Patterns to Avoid

- **Asking already-answered questions:** Always detect state first and skip configured options
- **Hardcoded paths:** Use platform detection for correct paths (macOS/Linux/Windows)
- **Missing backup on --fresh:** Always backup before overwriting config
- **Unclear option descriptions:** Each option must have clear, actionable description
- **Overly long headers:** AskUserQuestion headers max 12 characters
- **Too many options:** Keep to 2-4 options per question when possible

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Interactive prompts | Custom text parsing | AskUserQuestion tool | Built-in, structured, reliable |
| Config file generation | String concatenation | Heredoc with Write tool | Cleaner, proper escaping |
| Platform detection | Manual path checks | Existing platform-specifics.md patterns | Already solved |
| API key validation | Regex only | Live API test calls | Format valid but key might be revoked |
| Cron validation | Custom parser | Use memory-daemon's built-in validation | Already implemented |

**Key insight:** The existing memory-setup skill has solved most UX patterns. Follow those patterns rather than inventing new ones.

## Common Pitfalls

### Pitfall 1: Forgetting State Detection

**What goes wrong:** Wizard asks about already-configured options
**Why it happens:** Jumping to questions without checking config first
**How to avoid:** Every question block must start with state detection
**Warning signs:** User complains about redundant questions

### Pitfall 2: Breaking Existing Config

**What goes wrong:** --fresh flag overwrites config without backup
**Why it happens:** Missing backup step in execution flow
**How to avoid:** Always create `.bak` file before overwriting
**Warning signs:** User loses previous configuration

### Pitfall 3: Platform Path Confusion

**What goes wrong:** Wrong paths on different platforms
**Why it happens:** Hardcoded macOS paths used on Linux/Windows
**How to avoid:** Use platform detection at start, reference platform-specifics.md
**Warning signs:** "file not found" errors on non-macOS

### Pitfall 4: Incomplete Config Coverage

**What goes wrong:** Some config options not addressable through any wizard
**Why it happens:** Options added to daemon but not to wizard skills
**How to avoid:** Maintain coverage matrix (exists in docs/plans/configuration-wizard-skills-plan.md)
**Warning signs:** Users must manually edit config.toml for some options

### Pitfall 5: AskUserQuestion Header Too Long

**What goes wrong:** Headers get truncated or display incorrectly
**Why it happens:** Headers longer than 12 characters
**How to avoid:** Keep headers short: "Storage", "Retention", "Provider"
**Warning signs:** Truncated text in Claude Code UI

### Pitfall 6: Missing Validation After Execution

**What goes wrong:** Config written but not verified as working
**Why it happens:** Wizard ends immediately after writing config
**How to avoid:** Add verification step with daemon restart if needed
**Warning signs:** Config written but daemon doesn't use new values

## Code Examples

Verified patterns from existing implementation:

### State Detection Flow

```bash
# 1. Check if config file exists
CONFIG_PATH="~/.config/memory-daemon/config.toml"
ls $CONFIG_PATH 2>/dev/null && echo "CONFIG_EXISTS" || echo "NO_CONFIG"

# 2. Check specific section
grep -A5 '\[retention\]' $CONFIG_PATH 2>/dev/null

# 3. Check environment variables
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI_KEY_SET" || echo "OPENAI_KEY_MISSING"

# 4. Check storage usage
du -sh ~/.memory-store 2>/dev/null
df -h ~/.memory-store 2>/dev/null | tail -1
```

Source: `memory-setup/SKILL.md` State Detection section

### Config File Generation

```bash
# Create config directory
mkdir -p ~/.config/memory-daemon

# Generate config.toml with heredoc
cat > ~/.config/memory-daemon/config.toml << 'EOF'
[storage]
path = "~/.memory-store"
write_buffer_size_mb = 64
max_background_jobs = 4

[retention]
policy = "forever"
cleanup_schedule = "0 3 * * *"
archive_strategy = "compress"
gdpr_mode = false
EOF
```

Source: `memory-setup/references/configuration-options.md`

### Success Display Format

```
==================================================
 Storage Configuration Complete!
==================================================

[check] Storage path: ~/.memory-store (2.3 GB used)
[check] Retention policy: 90 days
[check] Cleanup schedule: Daily at 3 AM
[check] GDPR mode: Disabled

Next steps:
  * Run /memory-llm to configure summarization
  * Run /memory-agents for multi-agent setup
```

Source: `memory-setup/SKILL.md` Output Formatting section

### Reference File Structure

```markdown
# Retention Policies

## Policy Options

| Policy | Description | Storage Impact |
|--------|-------------|----------------|
| forever | Keep all data permanently | Grows unbounded |
| days:N | Delete data older than N days | Bounded growth |

## Cleanup Schedule

Cleanup runs on a cron schedule...
```

Source: Pattern from existing `references/` files in memory-setup

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual config.toml editing | Interactive wizards | Phase 9 (2026-01-31) | UX improvement |
| Single monolithic wizard | Specialized skill per domain | Phase 15 | Better modularity |
| No validation | Live API testing | Phase 9 | Catch errors early |

**Deprecated/outdated:**
- Direct config editing: Still possible but wizards preferred for UX

## Open Questions

Things that couldn't be fully resolved:

1. **GDPR Mode Implementation Details**
   - What we know: GDPR mode flag exists in plan, enables complete data removal
   - What's unclear: Exact implementation in memory-daemon (may not exist yet)
   - Recommendation: Document as "coming feature" if not implemented, or verify with codebase search

2. **Multi-Agent Storage Strategy**
   - What we know: Plan documents unified vs separate storage
   - What's unclear: Whether memory-daemon currently supports `[agents]` config section
   - Recommendation: Verify with codebase grep, may need daemon updates

3. **Budget Optimization for LLM**
   - What we know: Plan mentions cost estimation and budget modes
   - What's unclear: How memory-daemon tracks/enforces token budgets
   - Recommendation: May be advisory only (show estimates, don't enforce)

## Sources

### Primary (HIGH confidence)

- Context7 `/anthropics/claude-code` - AskUserQuestion tool documentation
- Context7 `/websites/code_claude` - Skill and command patterns
- Existing `plugins/memory-setup-plugin/skills/memory-setup/SKILL.md` - Proven patterns
- Existing `plugins/memory-setup-plugin/skills/memory-setup/references/` - Reference doc patterns
- `docs/plans/configuration-wizard-skills-plan.md` - Detailed question flows

### Secondary (MEDIUM confidence)

- Context7 `/affaan-m/everything-claude-code` - Additional skill patterns
- `.planning/ROADMAP.md` - Phase dependencies and success criteria

### Tertiary (LOW confidence)

- N/A - All critical patterns verified with primary sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - AskUserQuestion and SKILL.md patterns well-documented
- Architecture: HIGH - Follows existing memory-setup patterns exactly
- Pitfalls: HIGH - Based on actual Claude Code documentation and existing implementation

**Research date:** 2026-02-01
**Valid until:** 2026-03-01 (30 days - stable pattern)

## Implementation Recommendations

Based on research, the implementation should:

1. **Create three skill directories** following exact structure of memory-setup
2. **Reuse marketplace.json pattern** - add new skills to existing plugin
3. **Follow question flows** from `docs/plans/configuration-wizard-skills-plan.md`
4. **Create reference docs** for each skill with detailed option explanations
5. **Update memory-setup** to add gap options (timeout_secs, overlap_*, logging.*)
6. **Verify coverage** with matrix at end of phase

### Skills Summary

| Skill | Config Sections | Key Questions |
|-------|-----------------|---------------|
| `/memory-storage` | `[storage]`, `[retention]`, `[rollup]` | Path, retention, cleanup, GDPR, performance |
| `/memory-llm` | `[summarizer]` | Provider, model, API key, quality, budget |
| `/memory-agents` | `[agents]`, `[team]` | Mode, storage strategy, agent ID, query scope |

### Files to Create

| File | Purpose |
|------|---------|
| `skills/memory-storage/SKILL.md` | Storage wizard skill |
| `skills/memory-storage/references/retention-policies.md` | Retention reference |
| `skills/memory-storage/references/gdpr-compliance.md` | GDPR reference |
| `skills/memory-storage/references/archive-strategies.md` | Archive reference |
| `skills/memory-llm/SKILL.md` | LLM wizard skill |
| `skills/memory-llm/references/provider-comparison.md` | Provider reference |
| `skills/memory-llm/references/model-selection.md` | Model reference |
| `skills/memory-llm/references/cost-estimation.md` | Cost reference |
| `skills/memory-llm/references/custom-endpoints.md` | Endpoint reference |
| `skills/memory-agents/SKILL.md` | Agent wizard skill |
| `skills/memory-agents/references/storage-strategies.md` | Storage reference |
| `skills/memory-agents/references/team-setup.md` | Team reference |
| `skills/memory-agents/references/agent-identifiers.md` | ID reference |

### Files to Modify

| File | Change |
|------|--------|
| `.claude-plugin/marketplace.json` | Add new skill paths |
| `skills/memory-setup/SKILL.md` | Add missing advanced options |
| `skills/memory-setup/references/wizard-questions.md` | Add missing questions |
