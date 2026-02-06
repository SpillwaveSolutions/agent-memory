# Model Selection Guide

Choose the right model for your agent-memory summarization needs.

## Quick Recommendations

| Use Case | Recommended Model | Reason |
|----------|-------------------|--------|
| Most users | gpt-4o-mini | Best price/performance |
| Quality-focused | claude-3-5-sonnet | Highest quality summaries |
| Privacy-focused | llama3.2:3b | Local, no data sharing |
| Budget-conscious | gpt-4o-mini | Lowest cost |
| Offline needed | mistral (Ollama) | Works without internet |

## OpenAI Models

### gpt-4o-mini (Recommended)

**Best for most users**

| Attribute | Value |
|-----------|-------|
| Input Cost | $0.15 per 1M tokens |
| Output Cost | $0.60 per 1M tokens |
| Context Window | 128,000 tokens |
| Speed | Very fast |
| Quality | High |

```toml
[summarizer]
provider = "openai"
model = "gpt-4o-mini"
```

### gpt-4o

**Highest quality OpenAI model**

| Attribute | Value |
|-----------|-------|
| Input Cost | $2.50 per 1M tokens |
| Output Cost | $10.00 per 1M tokens |
| Context Window | 128,000 tokens |
| Speed | Fast |
| Quality | Highest |

Use when:
- Summary quality is critical
- Processing complex technical content
- Budget is not a concern

### gpt-4-turbo

**Previous generation, legacy support**

| Attribute | Value |
|-----------|-------|
| Input Cost | $10.00 per 1M tokens |
| Output Cost | $30.00 per 1M tokens |
| Context Window | 128,000 tokens |
| Speed | Medium |
| Quality | Very high |

Not recommended for new deployments. Use gpt-4o instead.

## Anthropic Models

### claude-3-5-haiku-latest (Recommended)

**Fast and cost-effective Claude model**

| Attribute | Value |
|-----------|-------|
| Input Cost | $0.25 per 1M tokens |
| Output Cost | $1.25 per 1M tokens |
| Context Window | 200,000 tokens |
| Speed | Fast |
| Quality | High |

```toml
[summarizer]
provider = "anthropic"
model = "claude-3-5-haiku-latest"
```

### claude-3-5-sonnet-latest

**Best quality Claude model**

| Attribute | Value |
|-----------|-------|
| Input Cost | $3.00 per 1M tokens |
| Output Cost | $15.00 per 1M tokens |
| Context Window | 200,000 tokens |
| Speed | Medium |
| Quality | Highest |

Use when:
- Nuanced summarization needed
- Complex technical content
- Long context processing

## Ollama Models

Discover available models:
```bash
# List installed models
ollama list

# Search available models
ollama search

# Pull a new model
ollama pull llama3.2:3b
```

### llama3.2:3b

**Compact, fast, good for basic summarization**

| Attribute | Value |
|-----------|-------|
| Cost | Free (local) |
| RAM Required | 4GB |
| Speed | Fast |
| Quality | Good |

```toml
[summarizer]
provider = "ollama"
model = "llama3.2:3b"
```

### mistral

**Balanced quality and speed**

| Attribute | Value |
|-----------|-------|
| Cost | Free (local) |
| RAM Required | 8GB |
| Speed | Medium |
| Quality | Better |

### llama3.1:8b

**Best quality for local models**

| Attribute | Value |
|-----------|-------|
| Cost | Free (local) |
| RAM Required | 16GB |
| Speed | Slow |
| Quality | Best (local) |

### phi

**Microsoft's efficient small model**

| Attribute | Value |
|-----------|-------|
| Cost | Free (local) |
| RAM Required | 4GB |
| Speed | Very fast |
| Quality | Moderate |

## Model Discovery Commands

### OpenAI
```bash
# List available models
curl -s -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models | jq -r '.data[].id' | grep gpt
```

### Anthropic
```bash
# Current models (check docs for latest)
# claude-3-5-sonnet-latest
# claude-3-5-haiku-latest
```

### Ollama
```bash
# List local models
curl -s http://localhost:11434/api/tags | jq -r '.models[].name'

# Pull new model
ollama pull <model-name>
```

## Quality vs Cost Tradeoff

```
Quality
  ^
  |   claude-3-5-sonnet
  |       o
  |
  |   gpt-4o
  |     o        claude-3-5-haiku
  |                  o
  |   gpt-4o-mini
  |       o        llama3.1:8b
  |                    o
  |              mistral
  |                o
  |         llama3.2:3b
  |             o
  |-------------------------> Cost
       $0.15  $1   $3   $10
```

## Context Window Considerations

| Model | Context | Typical Summary Input |
|-------|---------|----------------------|
| gpt-4o-mini | 128k | 2-4k tokens |
| claude-3-5-haiku | 200k | 2-4k tokens |
| llama3.2:3b | 8k | 2-4k tokens |

For agent-memory summarization, context window is rarely a limiting factor as summaries typically process 2-4k tokens of conversation at a time.

## Testing Models

Test model quality before committing:

```bash
# Quick test with memory-daemon
memory-daemon admin test-summary \
  --input "Your test conversation..." \
  --model gpt-4o-mini

# Compare models
for model in gpt-4o-mini gpt-4o; do
  echo "=== $model ==="
  memory-daemon admin test-summary --model $model
done
```
