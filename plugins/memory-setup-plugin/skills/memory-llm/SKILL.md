---
name: memory-llm
description: |
  This skill should be used when the user asks to "configure LLM",
  "change summarizer provider", "test API connection", "estimate LLM costs",
  "discover models", or "tune summarization quality". Provides interactive wizard
  for LLM provider configuration with model discovery and API testing.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory LLM Skill

Configure LLM providers for agent-memory summarization including provider selection, model discovery, API testing, cost estimation, and quality tuning.

## When Not to Use

- Initial installation (use `/memory-setup` first)
- Querying past conversations (use memory-query plugin)
- Storage configuration (use `/memory-storage`)
- Multi-agent configuration (use `/memory-agents`)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `/memory-llm` | Interactive LLM wizard | `/memory-llm` |
| `/memory-llm --test` | Test current API key only | `/memory-llm --test` |
| `/memory-llm --discover` | List available models | `/memory-llm --discover` |
| `/memory-llm --estimate` | Show cost estimation | `/memory-llm --estimate` |
| `/memory-llm --advanced` | Show all options including quality tuning | `/memory-llm --advanced` |
| `/memory-llm --fresh` | Re-configure all options from scratch | `/memory-llm --fresh` |

## Question Flow

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

## State Detection

Before beginning configuration, detect current system state.

### Detection Commands

```bash
# Check API keys in environment
[ -n "$OPENAI_API_KEY" ] && echo "OPENAI: set" || echo "OPENAI: not set"
[ -n "$ANTHROPIC_API_KEY" ] && echo "ANTHROPIC: set" || echo "ANTHROPIC: not set"

# Check current summarizer config
grep -A10 '\[summarizer\]' ~/.config/memory-daemon/config.toml 2>/dev/null

# Test OpenAI connectivity
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models

# Test Anthropic connectivity
curl -s -o /dev/null -w "%{http_code}" \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  https://api.anthropic.com/v1/messages

# Check Ollama availability
curl -s http://localhost:11434/api/tags 2>/dev/null
```

### State Summary Format

```
Current LLM State
-----------------
Provider:        OpenAI
Model:           gpt-4o-mini
API Key:         OPENAI_API_KEY is set
Connection:      [check] Verified
Quality:         Balanced (temp=0.3, max_tokens=512)

Recommended:     Configuration complete, no changes needed
```

## Wizard Steps

### Step 1: Provider Selection

**Always ask unless --minimal with existing config**

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

### Step 2: Model Discovery

**Dynamic options based on selected provider**

```
question: "Which model should be used for summarization?"
header: "Model"
```

**OpenAI Models:**
```
options:
  - label: "gpt-4o-mini (Recommended)"
    description: "Fast and cost-effective at $0.15/1M input, $0.60/1M output tokens"
  - label: "gpt-4o"
    description: "Best quality at $2.50/1M input, $10/1M output tokens"
  - label: "gpt-4-turbo"
    description: "Previous generation at $10/1M input, $30/1M output tokens"
multiSelect: false
```

**Anthropic Models:**
```
options:
  - label: "claude-3-5-haiku-latest (Recommended)"
    description: "Fast and cost-effective at $0.25/1M input, $1.25/1M output tokens"
  - label: "claude-3-5-sonnet-latest"
    description: "Best quality at $3/1M input, $15/1M output tokens"
multiSelect: false
```

**Ollama Models (discovered dynamically):**
```bash
# Discover available models
curl -s http://localhost:11434/api/tags | jq -r '.models[].name'
```

```
options:
  - label: "llama3.2:3b"
    description: "Compact, fast, good for basic summarization"
  - label: "mistral"
    description: "Balanced quality and speed"
  - label: "phi"
    description: "Microsoft's small but capable model"
  - label: "[Other discovered models]"
    description: "Based on local Ollama installation"
multiSelect: false
```

### Step 3: API Key Configuration

**Skip if:** env var set for selected provider AND not `--fresh`

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

**If Enter new key selected:**

```
question: "Enter your API key (will be stored in config.toml):"
header: "Key"
type: password
validation: "Key must start with 'sk-' for OpenAI or 'sk-ant-' for Anthropic"
```

**Security note:** Recommend using environment variables over storing keys in config files.

### Step 4: Test Connection

**Always run to verify API access**

This is an action step, not a question. Run live API test:

```bash
# OpenAI test
curl -s -X POST https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"test"}],"max_tokens":5}' \
  | jq -r '.choices[0].message.content // .error.message'

# Anthropic test
curl -s -X POST https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-3-5-haiku-latest","max_tokens":10,"messages":[{"role":"user","content":"test"}]}' \
  | jq -r '.content[0].text // .error.message'

# Ollama test
curl -s -X POST http://localhost:11434/api/generate \
  -d '{"model":"llama3.2:3b","prompt":"test","stream":false}' \
  | jq -r '.response // .error'
```

**Display:**
```
Testing API connection...
  [check] Connected to OpenAI API
  [check] Model gpt-4o-mini available
  [check] Rate limit: 10,000 RPM
```

Or on failure:
```
Testing API connection...
  [x] Connection failed: Invalid API key

Options:
  1. Enter a different API key
  2. Skip and configure later
  3. Cancel setup
```

### Step 5: Cost Estimation

**Informational display, not a question**

```
Cost Estimation
---------------
Based on typical usage patterns:

| Usage Level | Events/Day | Summaries/Day | Monthly Cost |
|-------------|------------|---------------|--------------|
| Light       | 100        | ~5            | ~$0.01       |
| Medium      | 1,000      | ~50           | ~$0.05       |
| Heavy       | 5,000      | ~250          | ~$0.25       |

Your estimated usage: Medium (~$0.05/month with gpt-4o-mini)

Factors affecting cost:
  * Summary length (default: ~500 tokens)
  * Conversation volume
  * Model selection
```

### Step 6: Quality/Latency Tradeoffs

**Skip if:** `--minimal` mode OR not `--advanced`

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

**If Custom selected:**

```
question: "Enter temperature (0.0-1.0):"
header: "Temp"
type: number
validation: "0.0 <= value <= 1.0"
```

```
question: "Enter max tokens (128-2048):"
header: "Tokens"
type: number
validation: "128 <= value <= 2048"
```

### Step 7: Token Budget Optimization

**Skip if:** `--minimal` mode OR not `--advanced`

```
question: "Configure token budget optimization?"
header: "Budget"
options:
  - label: "Balanced (Recommended)"
    description: "Standard summarization, ~$0.02/month typical usage"
  - label: "Economical"
    description: "Shorter summaries, 50% cost reduction"
  - label: "Detailed"
    description: "Longer summaries, 2x cost but more context preserved"
  - label: "Custom"
    description: "Set specific token limits"
multiSelect: false
```

## Config Generation

After wizard completion, generate or update config.toml:

```bash
# Create or update summarizer section
cat >> ~/.config/memory-daemon/config.toml << 'EOF'

[summarizer]
provider = "openai"
model = "gpt-4o-mini"
# api_key loaded from OPENAI_API_KEY env var
# api_endpoint = "https://api.openai.com/v1"  # for custom endpoints
max_tokens = 512
temperature = 0.3
budget_mode = "balanced"
EOF
```

### Config Value Mapping

| Wizard Choice | Config Values |
|---------------|---------------|
| OpenAI | `provider = "openai"` |
| Anthropic | `provider = "anthropic"` |
| Ollama | `provider = "ollama"`, `api_endpoint = "http://localhost:11434"` |
| None | `provider = "none"` |
| Balanced | `temperature = 0.3`, `max_tokens = 512` |
| Deterministic | `temperature = 0.0`, `max_tokens = 512` |
| Creative | `temperature = 0.7`, `max_tokens = 512` |
| Economical | `max_tokens = 256`, `budget_mode = "economical"` |
| Detailed | `max_tokens = 1024`, `budget_mode = "detailed"` |

## Validation

Before applying configuration, validate:

```bash
# 1. API key format valid
echo "$OPENAI_API_KEY" | grep -E '^sk-[a-zA-Z0-9]{32,}$' && echo "[check] OpenAI key format OK" || echo "[x] Invalid key format"
echo "$ANTHROPIC_API_KEY" | grep -E '^sk-ant-[a-zA-Z0-9-]+$' && echo "[check] Anthropic key format OK" || echo "[x] Invalid key format"

# 2. Live API test successful (see Step 4)

# 3. Selected model available
# OpenAI
curl -s -H "Authorization: Bearer $OPENAI_API_KEY" https://api.openai.com/v1/models \
  | jq -r '.data[].id' | grep -q "gpt-4o-mini" && echo "[check] Model available"

# 4. Rate limits verified (from test response headers)
```

## Output Formatting

### Success Display

```
==================================================
 LLM Configuration Complete!
==================================================

[check] Provider: OpenAI
[check] Model: gpt-4o-mini
[check] API Key: Using OPENAI_API_KEY environment variable
[check] Connection: Verified
[check] Quality: Balanced (temp=0.3, max_tokens=512)
[check] Estimated cost: ~$0.05/month

Configuration written to ~/.config/memory-daemon/config.toml

Next steps:
  * Restart daemon: memory-daemon restart
  * Test summarization: memory-daemon admin test-summary
  * Configure storage: /memory-storage
```

### Partial Success Display

```
==================================================
 LLM Configuration Partially Complete
==================================================

[check] Provider: OpenAI
[check] Model: gpt-4o-mini
[!] API Key: Not verified (connection test skipped)
[!] Quality: Using defaults

What's missing:
  * API connection not tested

To verify configuration:
  /memory-llm --test
```

### Error Display

```
[x] LLM Configuration Failed
-----------------------------

Error: API connection failed - Invalid API key

To fix:
  1. Verify your API key at https://platform.openai.com/api-keys
  2. Set environment variable: export OPENAI_API_KEY="sk-..."
  3. Re-run: /memory-llm --fresh

Need help? Check: /memory-llm --test
```

## Mode Behaviors

### Default Mode (`/memory-llm`)

- Runs state detection
- Shows steps 1-5
- Uses defaults for advanced options
- Skips API key if env var set

### Test Mode (`/memory-llm --test`)

- Only runs connection test (Step 4)
- Shows current configuration
- Quick verification

### Discover Mode (`/memory-llm --discover`)

- Lists all available models for configured provider
- Shows pricing information
- No configuration changes

### Estimate Mode (`/memory-llm --estimate`)

- Shows cost estimation only (Step 5)
- Based on current or typical usage
- No configuration changes

### Advanced Mode (`/memory-llm --advanced`)

- Shows ALL seven steps
- Includes quality tuning and budget optimization
- Full control over parameters

### Fresh Mode (`/memory-llm --fresh`)

- Ignores existing configuration
- Asks all questions from scratch
- Useful when switching providers

## Reference Files

For detailed information, see:

- [Provider Comparison](references/provider-comparison.md) - Detailed provider comparison
- [Model Selection](references/model-selection.md) - Model options and recommendations
- [Cost Estimation](references/cost-estimation.md) - Detailed cost calculations
- [Custom Endpoints](references/custom-endpoints.md) - Azure OpenAI, LocalAI, etc.

## Related Skills

After LLM configuration, consider:

- `/memory-storage` - Configure storage and retention
- `/memory-agents` - Set up multi-agent configuration
- `/memory-setup` - Full installation wizard
- `/memory-status` - Check current system status
