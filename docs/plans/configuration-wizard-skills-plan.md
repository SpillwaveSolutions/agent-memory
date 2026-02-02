# Configuration Wizard Skills Plan

## Overview

Create three new configuration wizard skills for agent-memory that use the AskUserQuestion interactive pattern to guide users through complex configuration scenarios.

**Location:** `plugins/memory-setup-plugin/skills/`

**Skills to Create:**
1. `memory-storage` - Storage, retention, and performance tuning
2. `memory-llm` - LLM provider deep configuration
3. `memory-agents` - Multi-agent and team configuration

---

## Feature/Config Coverage Matrix

This matrix ensures all memory-daemon configuration options are covered by the wizard skills.

### Coverage by Skill

| Config Section | Option | memory-setup | memory-storage | memory-llm | memory-agents | Gap? |
|---------------|--------|--------------|----------------|------------|---------------|------|
| **[storage]** | path | Basic | Deep | - | - | No |
| | write_buffer_size_mb | - | Advanced | - | - | No |
| | max_background_jobs | - | Advanced | - | - | No |
| **[server]** | host | Advanced | - | - | - | No |
| | port | Advanced | - | - | - | No |
| | timeout_secs | - | - | - | - | **Yes** |
| **[summarizer]** | provider | Basic | - | Deep | - | No |
| | model | Basic | - | Deep + Discovery | - | No |
| | api_key | Basic | - | Deep + Test | - | No |
| | api_endpoint | - | - | Deep | - | No |
| | max_tokens | - | - | Advanced | - | No |
| | temperature | - | - | Advanced | - | No |
| **[toc]** | segment_min_tokens | Advanced | - | - | - | No |
| | segment_max_tokens | Advanced | - | - | - | No |
| | time_gap_minutes | Advanced | - | - | - | No |
| | overlap_tokens | - | - | - | - | **Yes** |
| | overlap_minutes | - | - | - | - | **Yes** |
| **[rollup]** | min_age_hours | - | Advanced | - | - | No |
| | schedule | - | Advanced | - | - | No |
| **[logging]** | level | - | - | - | - | **Yes** |
| | format | - | - | - | - | **Yes** |
| | file | - | - | - | - | **Yes** |
| **[agents]** | mode | - | - | - | Deep | No (New) |
| | storage_strategy | - | - | - | Deep | No (New) |
| | agent_id | - | - | - | Deep | No (New) |
| | query_scope | - | - | - | Deep | No (New) |
| **[retention]** | policy | - | Deep | - | - | No (New) |
| | cleanup_schedule | - | Advanced | - | - | No (New) |
| | archive_strategy | - | Advanced | - | - | No (New) |
| | gdpr_mode | - | Deep | - | - | No (New) |

### Gap Resolution

| Gap | Resolution |
|-----|------------|
| server.timeout_secs | Add to memory-setup --advanced mode |
| toc.overlap_tokens | Add to memory-setup --advanced mode |
| toc.overlap_minutes | Add to memory-setup --advanced mode |
| logging.* | Create new skill: memory-logging OR add to memory-setup --advanced |

**Recommendation:** Add logging options to memory-setup --advanced rather than creating a 4th skill.

---

## Skill 1: memory-storage

### Purpose
Configure storage paths, data retention policies, cleanup schedules, and performance tuning.

### Commands

| Command | Purpose |
|---------|---------|
| `/memory-storage` | Interactive storage wizard |
| `/memory-storage --minimal` | Use defaults, minimal questions |
| `/memory-storage --advanced` | Show all options including cron and performance |

### Question Flow (6 Steps)

```
State Detection
      |
      v
+------------------+
| Step 1: Storage  | <- Skip if path exists (unless --fresh)
| Path             |
+--------+---------+
         |
         v
+------------------+
| Step 2: Retention| <- Skip if policy configured
| Policy           |
+--------+---------+
         |
         v
+------------------+
| Step 3: Cleanup  | <- --advanced only
| Schedule         |
+--------+---------+
         |
         v
+------------------+
| Step 4: Archive  | <- --advanced only
| Strategy         |
+--------+---------+
         |
         v
+------------------+
| Step 5: GDPR     | <- Show if EU locale detected
| Mode             |
+--------+---------+
         |
         v
+------------------+
| Step 6: Perf     | <- --advanced only
| Tuning           |
+--------+---------+
         |
         v
    Execution
```

### Questions with AskUserQuestion Format

**Step 1: Storage Path**
```
question: "Where should agent-memory store conversation data?"
header: "Storage"
options:
  - label: "~/.memory-store (Recommended)"
    description: "Standard user location, works on all platforms"
  - label: "~/.local/share/agent-memory/db"
    description: "XDG-compliant location for Linux"
  - label: "Custom path"
    description: "Specify a custom storage location"
multiSelect: false
```

**Step 2: Retention Policy**
```
question: "How long should conversation data be retained?"
header: "Retention"
options:
  - label: "Forever (Recommended)"
    description: "Keep all data permanently for maximum historical context"
  - label: "90 days"
    description: "Quarter retention, good balance of history and storage"
  - label: "30 days"
    description: "One month retention, lower storage usage"
  - label: "7 days"
    description: "Short-term memory only, minimal storage"
multiSelect: false
```

**Step 3: Cleanup Schedule** (--advanced)
```
question: "When should automatic cleanup run?"
header: "Schedule"
options:
  - label: "Daily at 3 AM (Recommended)"
    description: "Runs during off-hours, catches expired data quickly"
  - label: "Weekly on Sunday"
    description: "Less frequent cleanup, lower system impact"
  - label: "Disabled"
    description: "Manual cleanup only with memory-daemon admin cleanup"
  - label: "Custom cron"
    description: "Specify a custom cron expression"
multiSelect: false
```

**Step 4: Archive Strategy** (--advanced)
```
question: "How should old data be archived before deletion?"
header: "Archive"
options:
  - label: "Compress to archive (Recommended)"
    description: "Saves space, data recoverable from ~/.memory-archive/"
  - label: "Export to JSON"
    description: "Human-readable backup before deletion"
  - label: "No archive"
    description: "Delete directly (irreversible)"
multiSelect: false
```

**Step 5: GDPR Mode**
```
question: "Enable GDPR-compliant deletion mode?"
header: "GDPR"
options:
  - label: "No (Recommended)"
    description: "Standard retention with tombstones"
  - label: "Yes"
    description: "Complete data removal, audit logging, export-before-delete"
multiSelect: false
```

**Step 6: Performance Tuning** (--advanced)
```
question: "Configure storage performance parameters?"
header: "Performance"
options:
  - label: "Balanced (Recommended)"
    description: "64MB write buffer, 4 background jobs - works for most users"
  - label: "Low memory"
    description: "16MB write buffer, 1 background job - for constrained systems"
  - label: "High performance"
    description: "128MB write buffer, 8 background jobs - for heavy usage"
  - label: "Custom"
    description: "Specify write_buffer_size_mb and max_background_jobs"
multiSelect: false
```

### State Detection

```bash
# Current storage path
grep -A5 '\[storage\]' ~/.config/memory-daemon/config.toml 2>/dev/null | grep path

# Storage usage
du -sh ~/.memory-store 2>/dev/null

# Available disk space
df -h ~/.memory-store 2>/dev/null | tail -1

# Retention configured?
grep retention ~/.config/memory-daemon/config.toml 2>/dev/null

# Archive exists?
ls ~/.memory-archive 2>/dev/null
```

### Config Changes

**New/updated sections in config.toml:**

```toml
[storage]
path = "~/.memory-store"
write_buffer_size_mb = 64
max_background_jobs = 4

[retention]
policy = "forever"  # or "days:30", "days:90", etc.
cleanup_schedule = "0 3 * * *"
archive_strategy = "compress"
archive_path = "~/.memory-archive"
gdpr_mode = false
```

### Validation

1. Path exists or can be created
2. Write permissions verified
3. Minimum 100MB free disk space
4. Cron expression valid (if custom)
5. Archive path writable (if archiving enabled)

---

## Skill 2: memory-llm

### Purpose
Deep configuration for LLM providers including model discovery, cost estimation, API testing, and custom endpoints.

### Commands

| Command | Purpose |
|---------|---------|
| `/memory-llm` | Interactive LLM wizard |
| `/memory-llm --test` | Test current API key only |
| `/memory-llm --discover` | List available models |
| `/memory-llm --estimate` | Show cost estimation |

### Question Flow (7 Steps)

```
State Detection
      |
      v
+------------------+
| Step 1: Provider | <- Always ask (core decision)
+--------+---------+
         |
         v
+------------------+
| Step 2: Model    | <- Show discovered models
| Discovery        |
+--------+---------+
         |
         v
+------------------+
| Step 3: API Key  | <- Skip if env var set
+--------+---------+
         |
         v
+------------------+
| Step 4: Test     | <- Always run to verify
| Connection       |
+--------+---------+
         |
         v
+------------------+
| Step 5: Cost     | <- Informational, no question
| Estimation       |
+--------+---------+
         |
         v
+------------------+
| Step 6: Quality  | <- --advanced only
| Tradeoffs        |
+--------+---------+
         |
         v
+------------------+
| Step 7: Budget   | <- --advanced only
| Optimization     |
+--------+---------+
         |
         v
    Execution
```

### Questions with AskUserQuestion Format

**Step 1: Provider Selection**
```
question: "Which LLM provider should generate summaries?"
header: "Provider"
options:
  - label: "OpenAI (Recommended)"
    description: "GPT models - fast, reliable, good price/performance"
  - label: "Anthropic"
    description: "Claude models - high quality summaries"
  - label: "Ollama (Local)"
    description: "Private, runs on your machine, no API costs"
  - label: "None"
    description: "Disable summarization entirely"
multiSelect: false
```

**Step 2: Model Selection** (dynamic based on discovery)
```
question: "Which model should be used for summarization?"
header: "Model"
options:
  - label: "gpt-4o-mini (Recommended)"
    description: "Fast and cost-effective at $0.15/1M tokens"
  - label: "gpt-4o"
    description: "Best quality at $5/1M tokens"
  - label: "gpt-4-turbo"
    description: "Previous generation at $10/1M tokens"
multiSelect: false
```

**Step 3: API Key**
```
question: "How should the API key be configured?"
header: "API Key"
options:
  - label: "Use existing environment variable (Recommended)"
    description: "OPENAI_API_KEY is already set"
  - label: "Enter new key"
    description: "Provide a new API key"
  - label: "Test existing key"
    description: "Verify the current key works"
multiSelect: false
```

**Step 6: Quality/Latency Tradeoffs** (--advanced)
```
question: "Configure quality vs latency tradeoff?"
header: "Quality"
options:
  - label: "Balanced (Recommended)"
    description: "temperature=0.3, max_tokens=512 - good for most uses"
  - label: "Deterministic"
    description: "temperature=0.0 - consistent, reproducible summaries"
  - label: "Creative"
    description: "temperature=0.7 - more variation in summaries"
  - label: "Custom"
    description: "Specify temperature and max_tokens manually"
multiSelect: false
```

**Step 7: Token Budget** (--advanced)
```
question: "Configure token budget optimization?"
header: "Budget"
options:
  - label: "Balanced (Recommended)"
    description: "Standard summarization, ~$0.02/month typical usage"
  - label: "Economical"
    description: "Shorter summaries, lower cost"
  - label: "Detailed"
    description: "Longer summaries, higher cost"
  - label: "Custom"
    description: "Set specific token limits"
multiSelect: false
```

### State Detection

```bash
# API keys set?
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI: set" || echo "OPENAI: not set"
[ -n "$ANTHROPIC_API_KEY" ] && echo "ANTHROPIC: set" || echo "ANTHROPIC: not set"

# Current config
grep -A10 '\[summarizer\]' ~/.config/memory-daemon/config.toml 2>/dev/null

# Test connectivity
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models

# Check Ollama
curl -s http://localhost:11434/api/tags 2>/dev/null
```

### Config Changes

```toml
[summarizer]
provider = "openai"
model = "gpt-4o-mini"
# api_key loaded from OPENAI_API_KEY env var
# api_endpoint = "https://api.openai.com/v1"  # for custom endpoints
max_tokens = 512
temperature = 0.3
budget_mode = "balanced"
```

### Validation

1. API key format valid (sk- prefix for OpenAI, sk-ant- for Anthropic)
2. Live API test successful
3. Selected model available
4. Rate limits verified

---

## Skill 3: memory-agents

### Purpose
Configure multi-agent memory settings including store isolation, agent tagging, and cross-agent permissions.

### Commands

| Command | Purpose |
|---------|---------|
| `/memory-agents` | Interactive multi-agent wizard |
| `/memory-agents --single` | Configure for single user mode |
| `/memory-agents --team` | Configure for team use |

### Question Flow (6 Steps)

```
State Detection
      |
      v
+------------------+
| Step 1: Usage    | <- Always ask (core decision)
| Mode             |
+--------+---------+
         |
         v
+------------------+
| Step 2: Storage  | <- Skip if single user
| Strategy         |
+--------+---------+
         |
         v
+------------------+
| Step 3: Agent    | <- Always ask
| Identifier       |
+--------+---------+
         |
         v
+------------------+
| Step 4: Query    | <- Skip if single user or separate stores
| Scope            |
+--------+---------+
         |
         v
+------------------+
| Step 5: Storage  | <- --advanced, if separate stores
| Organization     |
+--------+---------+
         |
         v
+------------------+
| Step 6: Team     | <- If team mode selected
| Settings         |
+--------+---------+
         |
         v
    Execution
```

### Questions with AskUserQuestion Format

**Step 1: Usage Mode**
```
question: "How will agent-memory be used?"
header: "Mode"
options:
  - label: "Single user (Recommended)"
    description: "One person, one agent (Claude Code)"
  - label: "Single user, multiple agents"
    description: "One person using Claude Code, Cursor, etc."
  - label: "Team mode"
    description: "Multiple users sharing memory on a team"
multiSelect: false
```

**Step 2: Storage Strategy**
```
question: "How should agent data be stored?"
header: "Storage"
options:
  - label: "Unified store with tags (Recommended)"
    description: "Single database, agents identified by tag, easy cross-query"
  - label: "Separate stores per agent"
    description: "Complete isolation, cannot query across agents"
multiSelect: false
```

**Step 3: Agent Identifier**
```
question: "Choose your agent identifier (tags all events from this instance):"
header: "Agent ID"
options:
  - label: "claude-code (Recommended)"
    description: "Standard identifier for Claude Code"
  - label: "claude-code-{hostname}"
    description: "Unique per machine for multi-machine setups"
  - label: "{username}-claude"
    description: "User-specific for shared machines"
  - label: "Custom"
    description: "Specify a custom identifier"
multiSelect: false
```

**Step 4: Cross-Agent Query Permissions**
```
question: "What data should queries return?"
header: "Query Scope"
options:
  - label: "Own events only (Recommended)"
    description: "Query only this agent's data"
  - label: "All agents"
    description: "Query all agents' data (read-only)"
  - label: "Specified agents"
    description: "Query specific agents' data"
multiSelect: false
```

### State Detection

```bash
# Current multi-agent config
grep -A5 'agents' ~/.config/memory-daemon/config.toml 2>/dev/null

# Current agent_id
grep 'agent_id' ~/.config/memory-daemon/config.toml 2>/dev/null

# Detect other agents
ls ~/.memory-store/agents/ 2>/dev/null

# Hostname and user
hostname
whoami
```

### Config Changes

**New section in config.toml:**

```toml
[agents]
mode = "single"  # single, multi, team
storage_strategy = "unified"  # unified, separate
agent_id = "claude-code"
query_scope = "own"  # own, all, or comma-separated list

[team]
name = "default"
storage_path = "~/.memory-store/team/"
shared = false
```

### Validation

1. Agent ID valid (no spaces, 3-50 chars)
2. Agent ID unique in unified store
3. Storage path writable
4. Team path accessible (if shared)

---

## Implementation Tasks

### Phase 1: File Structure

Create skill directories:
```
plugins/memory-setup-plugin/skills/
├── memory-setup/          # Existing
├── memory-storage/
│   ├── SKILL.md
│   └── references/
│       ├── retention-policies.md
│       ├── gdpr-compliance.md
│       └── archive-strategies.md
├── memory-llm/
│   ├── SKILL.md
│   └── references/
│       ├── provider-comparison.md
│       ├── model-selection.md
│       ├── cost-estimation.md
│       └── custom-endpoints.md
└── memory-agents/
    ├── SKILL.md
    └── references/
        ├── storage-strategies.md
        ├── team-setup.md
        └── agent-identifiers.md
```

### Phase 2: SKILL.md Creation

For each skill:
1. Create SKILL.md with YAML frontmatter
2. Define commands table
3. Document question flow
4. Add state detection commands
5. Document config changes
6. Add output formatting (success/partial/error)
7. Add cross-skill navigation hints

### Phase 3: Reference Documentation

Create reference files for each skill with detailed explanations of options.

### Phase 4: Plugin Integration

Update `marketplace.json` to include new skills:
```json
{
  "plugins": [{
    "skills": [
      "./skills/memory-setup",
      "./skills/memory-storage",
      "./skills/memory-llm",
      "./skills/memory-agents"
    ]
  }]
}
```

### Phase 5: Update memory-setup

Add missing advanced options:
- server.timeout_secs
- toc.overlap_tokens
- toc.overlap_minutes
- logging.level, format, file

---

## Verification

After implementation, verify:

1. **Skill Discovery**
   ```bash
   # Skills should appear in Claude Code
   /memory-storage --help
   /memory-llm --help
   /memory-agents --help
   ```

2. **Question Flow**
   - Run each skill with no config
   - Run each skill with existing config (should skip)
   - Run each skill with --fresh (should ask all)
   - Run each skill with --advanced (should show extra options)

3. **Config Generation**
   - Verify config.toml updated correctly
   - Verify new sections added properly
   - Verify existing sections preserved

4. **Cross-Skill Navigation**
   - Each skill suggests related skills at completion
   - No circular dependencies

5. **Coverage Check**
   - All config options from coverage matrix are addressable
   - No gaps remain

---

## Files to Create

| File | Purpose |
|------|---------|
| `plugins/memory-setup-plugin/skills/memory-storage/SKILL.md` | Storage wizard skill |
| `plugins/memory-setup-plugin/skills/memory-llm/SKILL.md` | LLM wizard skill |
| `plugins/memory-setup-plugin/skills/memory-agents/SKILL.md` | Multi-agent wizard skill |
| `plugins/memory-setup-plugin/skills/memory-storage/references/*.md` | Storage reference docs |
| `plugins/memory-setup-plugin/skills/memory-llm/references/*.md` | LLM reference docs |
| `plugins/memory-setup-plugin/skills/memory-agents/references/*.md` | Agent reference docs |

## Files to Modify

| File | Change |
|------|--------|
| `plugins/memory-setup-plugin/.claude-plugin/marketplace.json` | Add new skill paths |
| `plugins/memory-setup-plugin/skills/memory-setup/SKILL.md` | Add missing advanced options |
| `plugins/memory-setup-plugin/skills/memory-setup/references/wizard-questions.md` | Add missing questions |
