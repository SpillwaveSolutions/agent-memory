# Cost Estimation

Estimate and optimize LLM costs for agent-memory summarization.

## Cost Calculation Formula

```
Monthly Cost = (tokens_per_summary * summaries_per_month) / 1,000,000 * price_per_1M_tokens
```

### Typical Values

| Parameter | Typical Value | Notes |
|-----------|---------------|-------|
| Tokens per summary | 300-500 | Input + output combined |
| Summaries per day | 5-50 | Depends on conversation volume |
| Days per month | 30 | Standard |

## Usage Tiers

### gpt-4o-mini Costs

| Usage | Events/Day | Summaries/Day | Monthly Cost |
|-------|------------|---------------|--------------|
| Light | 100 | ~5 | $0.01 |
| Medium | 500 | ~25 | $0.03 |
| Heavy | 2,000 | ~100 | $0.10 |
| Team | 10,000 | ~500 | $0.50 |

### claude-3-5-haiku Costs

| Usage | Events/Day | Summaries/Day | Monthly Cost |
|-------|------------|---------------|--------------|
| Light | 100 | ~5 | $0.02 |
| Medium | 500 | ~25 | $0.05 |
| Heavy | 2,000 | ~100 | $0.20 |
| Team | 10,000 | ~500 | $1.00 |

### gpt-4o Costs (Premium)

| Usage | Events/Day | Summaries/Day | Monthly Cost |
|-------|------------|---------------|--------------|
| Light | 100 | ~5 | $0.15 |
| Medium | 500 | ~25 | $0.75 |
| Heavy | 2,000 | ~100 | $3.00 |
| Team | 10,000 | ~500 | $15.00 |

## Token Counting

### What Counts as Tokens

```
1 token ~ 4 characters (English)
1 token ~ 0.75 words (English)

Example:
  "The quick brown fox" = 4 words = ~5 tokens
```

### Summary Token Breakdown

| Component | Tokens |
|-----------|--------|
| Input (conversation context) | 200-400 |
| System prompt | ~50 |
| Output (summary) | 100-200 |
| **Total per summary** | **350-650** |

## Budget Optimization Modes

### Balanced (Default)

```toml
[summarizer]
max_tokens = 512
budget_mode = "balanced"
```

- Standard summary length
- Good context preservation
- Typical cost: baseline

### Economical

```toml
[summarizer]
max_tokens = 256
budget_mode = "economical"
```

- Shorter summaries
- Essential information only
- **50% cost reduction**

### Detailed

```toml
[summarizer]
max_tokens = 1024
budget_mode = "detailed"
```

- Longer, more detailed summaries
- Maximum context preservation
- **2x cost increase**

## Cost Monitoring

### Check Current Usage

```bash
# OpenAI usage
# Visit: https://platform.openai.com/usage

# Anthropic usage
# Visit: https://console.anthropic.com/usage
```

### Estimate from Event Count

```bash
# Get event count
EVENT_COUNT=$(memory-daemon admin stats | grep "total_events" | awk '{print $2}')

# Estimate summaries (1 summary per 20 events average)
SUMMARIES=$((EVENT_COUNT / 20))

# Estimate tokens (400 tokens per summary average)
TOKENS=$((SUMMARIES * 400))

# Estimate cost (gpt-4o-mini: $0.15/1M input + $0.60/1M output)
# Assuming 60% input, 40% output
INPUT_COST=$(echo "scale=4; $TOKENS * 0.6 / 1000000 * 0.15" | bc)
OUTPUT_COST=$(echo "scale=4; $TOKENS * 0.4 / 1000000 * 0.60" | bc)
TOTAL=$(echo "scale=4; $INPUT_COST + $OUTPUT_COST" | bc)

echo "Estimated cost: \$$TOTAL"
```

## Cost Comparison Table

| Model | Light ($) | Medium ($) | Heavy ($) |
|-------|-----------|------------|-----------|
| gpt-4o-mini | 0.01 | 0.03 | 0.10 |
| gpt-4o | 0.15 | 0.75 | 3.00 |
| claude-3-5-haiku | 0.02 | 0.05 | 0.20 |
| claude-3-5-sonnet | 0.20 | 1.00 | 4.00 |
| Ollama (local) | 0.00 | 0.00 | 0.00 |

## Cost Reduction Strategies

### 1. Use Economical Model

Switch to gpt-4o-mini or claude-3-5-haiku for significant savings.

### 2. Reduce Summary Frequency

```toml
[summarizer]
# Summarize less frequently
batch_size = 50  # Summarize every 50 events instead of 20
```

### 3. Shorter Summaries

```toml
[summarizer]
max_tokens = 256  # Default is 512
```

### 4. Use Local Models

For privacy AND cost savings:

```toml
[summarizer]
provider = "ollama"
model = "llama3.2:3b"
```

### 5. Disable for Low-Value Content

Consider not summarizing all events:

```toml
[summarizer]
# Skip events shorter than 100 characters
min_event_length = 100
```

## Annual Cost Projection

| Usage Level | Monthly | Annual |
|-------------|---------|--------|
| Light (gpt-4o-mini) | $0.01 | $0.12 |
| Medium (gpt-4o-mini) | $0.03 | $0.36 |
| Heavy (gpt-4o-mini) | $0.10 | $1.20 |
| Team (gpt-4o-mini) | $0.50 | $6.00 |

## Free Tier Considerations

### OpenAI
- No permanent free tier
- $5 initial credit for new accounts
- Pay-as-you-go after

### Anthropic
- No free tier
- Pay-as-you-go from start

### Ollama
- Completely free
- Local compute costs (electricity)
