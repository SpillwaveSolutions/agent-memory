# Phase 19: OpenCode Commands and Skills - Research

**Researched:** 2026-02-09
**Domain:** OpenCode plugin development (commands, skills, agents)
**Confidence:** HIGH

## Summary

Phase 19 ports the existing Claude Code memory-query-plugin to OpenCode format. The research confirms that OpenCode uses a compatible skill format with YAML frontmatter, supporting cross-platform skill portability. Commands differ slightly (using `$ARGUMENTS` for argument substitution), and agents can be defined with markdown frontmatter.

The porting task is straightforward because:
1. Skills use identical SKILL.md format with YAML frontmatter (same as Claude Code)
2. Commands need minor adaptation for `$ARGUMENTS` substitution
3. Agents use similar markdown+frontmatter format but with different fields

**Primary recommendation:** Port skills with minimal changes (same SKILL.md format works), adapt commands for `$ARGUMENTS`, and create agent definition with OpenCode-specific frontmatter. Create `.opencode/` directory structure within a new `plugins/memory-opencode-plugin/` folder.

## Standard Stack

### Core

| Component | Format | Purpose | Why Standard |
|-----------|--------|---------|--------------|
| SKILL.md | YAML frontmatter + markdown | Skill definition | OpenCode native format, Claude-compatible |
| Command .md | YAML frontmatter + markdown | Slash command definition | OpenCode native format |
| Agent .md | YAML frontmatter + markdown | Agent definition | OpenCode native format |
| opencode.json | JSON config | Plugin configuration | Optional, for TypeScript plugins |

### Supporting

| Component | Format | Purpose | When to Use |
|-----------|--------|---------|-------------|
| references/ | Markdown subdirectory | Extended skill documentation | When skill has command reference |
| AGENTS.md | Markdown | Project instructions | For global plugin instructions |
| package.json | JSON | NPM dependencies | Only for TypeScript plugins |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `.opencode/` directory | `.claude/` directory | OpenCode also reads `.claude/`, but native `.opencode/` is cleaner |
| SKILL.md format | TypeScript plugin | TS plugins need Bun runtime; SKILL.md is universal |
| Static commands | TypeScript hooks | Hooks need event handling; static commands are simpler for this use case |

## Architecture Patterns

### Recommended Project Structure

```
plugins/memory-opencode-plugin/
├── .opencode/
│   ├── command/                    # Slash commands
│   │   ├── memory-search.md
│   │   ├── memory-recent.md
│   │   └── memory-context.md
│   ├── skill/                      # Skills (folder per skill)
│   │   ├── memory-query/
│   │   │   ├── SKILL.md
│   │   │   └── references/
│   │   │       └── command-reference.md
│   │   ├── retrieval-policy/
│   │   │   ├── SKILL.md
│   │   │   └── references/
│   │   │       └── command-reference.md
│   │   ├── topic-graph/
│   │   │   ├── SKILL.md
│   │   │   └── references/
│   │   │       └── command-reference.md
│   │   ├── bm25-search/
│   │   │   ├── SKILL.md
│   │   │   └── references/
│   │   │       └── command-reference.md
│   │   └── vector-search/
│   │       ├── SKILL.md
│   │       └── references/
│   │           └── command-reference.md
│   └── agents/                     # Agent definitions
│       └── memory-navigator.md
├── README.md                       # Installation guide
└── .gitignore
```

### Pattern 1: OpenCode Skill Format

**What:** Skills use YAML frontmatter with `name`, `description`, and optional `metadata`.

**When to use:** For all skills (same format as Claude Code).

**Example:**
```yaml
---
name: memory-query
description: |
  Query past conversations from the agent-memory system. Use when asked to
  "recall what we discussed", "search conversation history", etc.
license: MIT
metadata:
  version: 2.0.0
  author: SpillwaveSolutions
---

# Memory Query Skill

[Markdown content follows...]
```

**Source:** [OpenCode Skills Documentation](https://opencode.ai/docs/skills/)

**Key constraints:**
- Name: 1-64 chars, lowercase alphanumeric with hyphens
- Name regex: `^[a-z0-9]+(-[a-z0-9]+)*$`
- Description: 1-1024 characters
- Directory name must match skill name

### Pattern 2: OpenCode Command Format

**What:** Commands use YAML frontmatter with `description`, optional `agent`, and `$ARGUMENTS` substitution.

**When to use:** For all slash commands.

**Example:**
```yaml
---
description: Search past conversations by topic or keyword
agent: memory-navigator
---

# Memory Search

Search past conversations by topic or keyword using hierarchical TOC navigation.

## Process

1. Parse arguments from $ARGUMENTS
   - First argument is topic/keyword
   - Optional --period flag for time filtering

2. Check daemon status
   ```bash
   memory-daemon status
   ```

[Rest of process...]
```

**Source:** [OpenCode Commands Documentation](https://opencode.ai/docs/commands/)

**Key differences from Claude Code:**
- No `parameters:` array in frontmatter
- Use `$ARGUMENTS` for all arguments (vs Claude's structured parameters)
- Use `$1`, `$2`, etc. for positional args
- File name (minus .md) becomes command name

### Pattern 3: OpenCode Agent Format

**What:** Agents defined with YAML frontmatter specifying behavior, tools, and permissions.

**When to use:** For the memory-navigator agent.

**Example:**
```yaml
---
description: Autonomous agent for intelligent memory retrieval with tier-aware routing
mode: subagent
tools:
  read: true
  bash: true
  write: false
  edit: false
permission:
  bash:
    "memory-daemon *": allow
    "*": deny
---

# Memory Navigator Agent

[Agent instructions...]
```

**Source:** [OpenCode Agents Documentation](https://opencode.ai/docs/agents/)

**Key fields:**
- `description`: Required, explains agent purpose
- `mode`: `primary` | `subagent` | `all`
- `model`: Override default model
- `tools`: Enable/disable specific tools
- `permission`: Granular access control

**Note:** OpenCode agents don't have a `triggers:` field like Claude Code. Trigger patterns are implicit in the skill/agent descriptions or handled via `@mention` invocation.

### Pattern 4: Argument Handling Conversion

**What:** Convert Claude Code parameter arrays to OpenCode `$ARGUMENTS` handling.

**Claude Code format:**
```yaml
parameters:
  - name: topic
    description: Topic to search for
    required: true
  - name: period
    description: Time period
    required: false
```

**OpenCode format:**
```markdown
## Arguments

Parse arguments from `$ARGUMENTS`:
- `$1`: Topic to search for (required)
- `--period <value>`: Time period filter (optional)

Example: `/memory-search authentication --period "last week"`
→ $ARGUMENTS = "authentication --period last week"
→ $1 = "authentication"
```

### Anti-Patterns to Avoid

- **Don't use `triggers:` field:** OpenCode agents don't support this; rely on skill descriptions
- **Don't use TypeScript plugins unnecessarily:** SKILL.md format is sufficient for this phase
- **Don't put SKILL.md in root of skill directory:** Must be exactly `SKILL.md` (capitals)
- **Don't use underscores in skill names:** Must be lowercase alphanumeric with hyphens only

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Argument parsing | Custom parser in prompt | `$ARGUMENTS`, `$1`, `$2` | OpenCode handles substitution |
| Skill discovery | Manual loading instructions | SKILL.md in standard paths | OpenCode auto-discovers |
| Agent invocation | Custom invocation logic | `mode: subagent` + `@mention` | OpenCode handles routing |
| Command registration | Plugin manifest | File in `.opencode/command/` | Auto-registered by filename |

**Key insight:** OpenCode's file-based discovery eliminates the need for manifest files. Skills and commands are discovered automatically from standard paths.

## Common Pitfalls

### Pitfall 1: Trigger Pattern Compatibility

**What goes wrong:** Claude Code agents use `triggers:` for automatic activation; OpenCode doesn't.

**Why it happens:** Different activation paradigms between platforms.

**How to avoid:**
- Document trigger patterns in the agent description
- Use skill description to guide when the agent should be invoked
- Rely on `@memory-navigator` for explicit invocation

**Warning signs:** Agent never activates automatically in OpenCode.

### Pitfall 2: Parameter vs Arguments Mismatch

**What goes wrong:** Using Claude Code's `parameters:` array in OpenCode commands.

**Why it happens:** Copy-paste without adaptation.

**How to avoid:**
- Replace `parameters:` with inline `$ARGUMENTS` documentation
- Document positional args (`$1`, `$2`) in the process section

**Warning signs:** OpenCode ignores parameter definitions; users don't know how to pass args.

### Pitfall 3: Skill Name Case Sensitivity

**What goes wrong:** Using uppercase or mixed-case skill names.

**Why it happens:** Copying from other formats or natural naming.

**How to avoid:**
- Always use lowercase with hyphens
- Validate with regex: `^[a-z0-9]+(-[a-z0-9]+)*$`

**Warning signs:** Skill not discovered by OpenCode.

### Pitfall 4: Missing Description Field

**What goes wrong:** Skills without proper description aren't selectable by agents.

**Why it happens:** Description seems optional but is required for agent discovery.

**How to avoid:**
- Always include detailed description (1-1024 chars)
- Make description specific enough for agents to know when to use

**Warning signs:** Agent never loads the skill.

## Code Examples

### OpenCode Command (memory-search.md)

```yaml
---
description: Search past conversations by topic or keyword
---

# Memory Search

Search past conversations by topic or keyword using hierarchical TOC navigation.

## Usage

```
/memory-search <topic>
/memory-search <topic> --period "last week"
/memory-search authentication
/memory-search "database migration" --period january
```

## Arguments

Parse from `$ARGUMENTS`:
- **First argument**: Topic or keyword to search (required)
- **--period <value>**: Time period filter (optional)

## Process

1. **Check daemon status**
   ```bash
   memory-daemon status
   ```

2. **Get TOC root** to find available time periods
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 root
   ```

3. **Navigate to relevant period** based on --period argument
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"
   ```

4. **Search node summaries** for matching keywords

5. **Present results** with grip IDs for drill-down

## Output Format

```markdown
## Memory Search: [topic]

### [Time Period]
**Summary:** [matching bullet points]

**Excerpts:**
- "[excerpt text]" `grip:ID`
  _Source: [timestamp]_

---
Expand any excerpt: `/memory-context grip:ID`
```
```

### OpenCode Skill (SKILL.md)

```yaml
---
name: memory-query
description: |
  Query past conversations from the agent-memory system. Use when asked to
  "recall what we discussed", "search conversation history", "find previous session",
  "what did we talk about last week", or "get context from earlier". Provides
  tier-aware retrieval with automatic fallback chains, intent-based routing,
  and full explainability.
license: MIT
metadata:
  version: 2.0.0
  author: SpillwaveSolutions
---

# Memory Query Skill

[Full skill content same as Claude Code version]
```

### OpenCode Agent (memory-navigator.md)

```yaml
---
description: Autonomous agent for intelligent memory retrieval with tier-aware routing, intent classification, and automatic fallback chains
mode: subagent
tools:
  read: true
  bash: true
  write: false
  edit: false
permission:
  bash:
    "memory-daemon *": allow
    "grep *": allow
    "*": deny
---

# Memory Navigator Agent

Autonomous agent for intelligent memory retrieval with tier-aware routing,
intent classification, and automatic fallback chains. Handles complex queries
across multiple time periods with full explainability.

## When to Use

Invoke this agent (@memory-navigator) for complex queries that benefit from
intelligent routing:

- **Explore intent**: "What topics have we discussed recently?"
- **Answer intent**: "What have we discussed about authentication?"
- **Locate intent**: "Find the exact error message from JWT code"
- **Time-boxed intent**: "What happened yesterday?"

## Skills Used

- memory-query (core retrieval)
- topic-graph (Tier 1 exploration)
- bm25-search (keyword teleport)
- vector-search (semantic teleport)

[Rest of agent content...]
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Claude-only plugins | Cross-platform skills | 2025 | Skills portable across OpenCode, Claude Code, Cursor |
| Manifest-based discovery | File-based discovery | 2025 | No registration needed; auto-discovered |
| Hook-based events | Plugin lifecycle hooks | 2026 | TypeScript plugins can capture events |

**Deprecated/outdated:**
- Custom skill loaders: OpenCode has native skill discovery
- Claude-specific trigger patterns: Use agent descriptions instead

## OpenCode-Specific Considerations

### Skill Location Discovery

OpenCode searches these paths (in order):
1. `.opencode/skills/*/SKILL.md` (project)
2. `.claude/skills/*/SKILL.md` (Claude Code compatibility)
3. `.agents/skills/*/SKILL.md` (agents.md compatibility)
4. `~/.config/opencode/skills/*/SKILL.md` (global)
5. `~/.claude/skills/*/SKILL.md` (global Claude compatibility)
6. `~/.agents/skills/*/SKILL.md` (global agents.md compatibility)

**Recommendation:** Use `.opencode/skill/` for native OpenCode plugins.

### Command Location

OpenCode loads commands from:
1. `.opencode/commands/` (project)
2. `~/.config/opencode/commands/` (global)

Filename becomes command name (e.g., `memory-search.md` → `/memory-search`).

### Agent Location

OpenCode loads agents from:
1. `.opencode/agents/` (project)
2. `~/.config/opencode/agents/` (global)

Filename becomes agent name (e.g., `memory-navigator.md` → `@memory-navigator`).

## Open Questions

### Question 1: Agent Trigger Pattern Alternative

**What we know:** Claude Code uses `triggers:` for automatic agent activation.
**What's unclear:** How to achieve similar auto-activation in OpenCode without triggers.
**Recommendation:** Document patterns in agent description; rely on `@mention` invocation; investigate if OpenCode has a similar mechanism in future versions.

### Question 2: Event Capture (Deferred to Phase 20)

**What we know:** Phase 19 focuses on commands/skills/agents only.
**What's unclear:** How OpenCode plugin hooks work for event capture.
**Recommendation:** Defer to Phase 20; document as out of scope for Phase 19.

### Question 3: Skills Portability

**What we know:** Same SKILL.md format works in both Claude Code and OpenCode.
**What's unclear:** Whether to maintain one source and symlink, or duplicate.
**Recommendation:** Create OpenCode-specific copies in `.opencode/skill/`; consider symlinks as future optimization.

## README Requirements

The plugin README must document:

1. **Prerequisites:** memory-daemon installation and running
2. **Installation:**
   - Clone/copy to `~/.config/opencode/skills/memory-query-plugin` for global
   - Or symlink to project `.opencode/` for per-project
3. **Commands:** Usage examples for all three commands
4. **Agent Invocation:** How to use `@memory-navigator`
5. **Skills:** What each skill does and when used
6. **Tier Explanation:** Retrieval capability tiers

## Sources

### Primary (HIGH confidence)
- [OpenCode Skills Documentation](https://opencode.ai/docs/skills/) - Skill format, discovery paths
- [OpenCode Commands Documentation](https://opencode.ai/docs/commands/) - Command format, $ARGUMENTS
- [OpenCode Agents Documentation](https://opencode.ai/docs/agents/) - Agent format, modes
- [OpenCode Config Documentation](https://opencode.ai/docs/config/) - opencode.json options
- [OpenCode Plugins Documentation](https://opencode.ai/docs/plugins/) - TypeScript plugin structure

### Secondary (MEDIUM confidence)
- [OpenCode Rules Documentation](https://opencode.ai/docs/rules/) - AGENTS.md format
- Existing Claude Code plugin in `plugins/memory-query-plugin/` - Source format reference

### Tertiary (LOW confidence)
- WebSearch results for community patterns - Needs validation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Official documentation verified
- Architecture: HIGH - Matches Claude Code plugin structure
- Pitfalls: MEDIUM - Based on format differences, not production testing
- Trigger patterns: LOW - No clear equivalent found in OpenCode

**Research date:** 2026-02-09
**Valid until:** 2026-03-09 (30 days - stable format)
