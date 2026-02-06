# Provider Comparison

Compare LLM providers for agent-memory summarization.

## Overview

| Provider | Cost | Quality | Latency | Privacy | Best For |
|----------|------|---------|---------|---------|----------|
| OpenAI | $$ | High | Fast | Cloud | Most users |
| Anthropic | $$$ | Highest | Medium | Cloud | Quality-focused |
| Ollama | Free | Variable | Slow | Local | Privacy-focused |
| None | Free | N/A | N/A | N/A | Minimal setup |

## OpenAI

**GPT models - fast, reliable, good price/performance**

### Pros
- Fastest response times
- Consistent quality
- Best price/performance ratio
- Wide model selection
- Excellent documentation

### Cons
- Data sent to OpenAI servers
- Requires API key
- Usage costs (though low)

### Configuration
```toml
[summarizer]
provider = "openai"
model = "gpt-4o-mini"
# Uses OPENAI_API_KEY environment variable
```

### API Key Setup
1. Visit https://platform.openai.com/api-keys
2. Create a new API key
3. Set environment variable:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```

### Models
| Model | Input Cost | Output Cost | Context | Recommendation |
|-------|------------|-------------|---------|----------------|
| gpt-4o-mini | $0.15/1M | $0.60/1M | 128k | Best value |
| gpt-4o | $2.50/1M | $10.00/1M | 128k | Highest quality |
| gpt-4-turbo | $10.00/1M | $30.00/1M | 128k | Legacy |

## Anthropic

**Claude models - highest quality summaries**

### Pros
- Excellent at nuanced summarization
- Better handling of technical content
- Constitutional AI safety approach
- Long context support

### Cons
- Higher costs than OpenAI
- Slightly slower response times
- Data sent to Anthropic servers

### Configuration
```toml
[summarizer]
provider = "anthropic"
model = "claude-3-5-haiku-latest"
# Uses ANTHROPIC_API_KEY environment variable
```

### API Key Setup
1. Visit https://console.anthropic.com/
2. Create a new API key
3. Set environment variable:
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

### Models
| Model | Input Cost | Output Cost | Context | Recommendation |
|-------|------------|-------------|---------|----------------|
| claude-3-5-haiku | $0.25/1M | $1.25/1M | 200k | Best value |
| claude-3-5-sonnet | $3.00/1M | $15.00/1M | 200k | Highest quality |

## Ollama (Local)

**Run models locally - complete privacy, no API costs**

### Pros
- Complete privacy - data never leaves your machine
- No API costs
- Works offline
- Many model choices

### Cons
- Requires local resources (RAM, CPU/GPU)
- Slower than cloud APIs
- Quality varies by model
- Setup more complex

### Prerequisites
1. Install Ollama: https://ollama.ai
2. Pull a model:
   ```bash
   ollama pull llama3.2:3b
   ```
3. Start Ollama:
   ```bash
   ollama serve
   ```

### Configuration
```toml
[summarizer]
provider = "ollama"
model = "llama3.2:3b"
api_endpoint = "http://localhost:11434"
```

### Recommended Models
| Model | RAM Required | Quality | Speed |
|-------|--------------|---------|-------|
| llama3.2:3b | 4GB | Good | Fast |
| mistral | 8GB | Better | Medium |
| llama3.1:8b | 16GB | Best | Slow |

## None (Disabled)

**Disable summarization entirely**

### When to Use
- Testing/development only
- TOC-only mode sufficient
- No API access available
- Minimal resource usage

### Configuration
```toml
[summarizer]
provider = "none"
```

### Impact
- No LLM-generated summaries
- Table of Contents still generated
- Faster event processing
- No API costs

## Decision Matrix

| If you need... | Choose |
|----------------|--------|
| Best price/performance | OpenAI |
| Highest quality | Anthropic |
| Complete privacy | Ollama |
| Minimal setup | None |
| Offline capability | Ollama |
| Fastest responses | OpenAI |
| Long context | Anthropic |

## API Key Security

### Best Practices
1. Use environment variables, not config files
2. Never commit API keys to git
3. Use separate keys for development/production
4. Rotate keys periodically
5. Monitor usage for anomalies

### Environment Setup
```bash
# Add to ~/.bashrc or ~/.zshrc
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Verification
```bash
# Check keys are set
echo "OpenAI: ${OPENAI_API_KEY:+configured}"
echo "Anthropic: ${ANTHROPIC_API_KEY:+configured}"
```
