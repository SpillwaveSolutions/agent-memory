# API Testing

Test and verify LLM API connections before configuring agent-memory.

## Why Test API Connections?

Testing ensures:

- API key is valid and active
- Selected model is accessible
- Rate limits are sufficient
- Network connectivity is working

## Quick Test Commands

### OpenAI

```bash
# Test API key (list models)
curl -s -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models | jq '.data[0:3]'

# Test completion
curl -s https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Say hello"}],
    "max_tokens": 10
  }' | jq '.choices[0].message.content'
```

### Anthropic

```bash
# Test API key
curl -s https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-haiku-latest",
    "max_tokens": 10,
    "messages": [{"role": "user", "content": "Say hello"}]
  }' | jq '.content[0].text'
```

### Ollama

```bash
# Check if Ollama is running
curl -s http://localhost:11434/api/tags | jq '.models[].name'

# Test generation
curl -s http://localhost:11434/api/generate \
  -d '{
    "model": "llama3.2:3b",
    "prompt": "Say hello",
    "stream": false
  }' | jq '.response'
```

## Detailed Test Procedures

### OpenAI Full Test

```bash
#!/bin/bash
echo "=== OpenAI API Test ==="

# 1. Check API key format
if [[ ! "$OPENAI_API_KEY" =~ ^sk- ]]; then
  echo "[x] Invalid API key format (should start with 'sk-')"
  exit 1
fi
echo "[check] API key format OK"

# 2. Test authentication
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models)

if [ "$STATUS" != "200" ]; then
  echo "[x] Authentication failed (HTTP $STATUS)"
  exit 1
fi
echo "[check] Authentication OK"

# 3. Check model access
MODELS=$(curl -s -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models | jq -r '.data[].id' | grep gpt-4o-mini)

if [ -z "$MODELS" ]; then
  echo "[x] Model gpt-4o-mini not accessible"
  exit 1
fi
echo "[check] Model gpt-4o-mini accessible"

# 4. Test completion
RESPONSE=$(curl -s https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"Hi"}],"max_tokens":5}')

if echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
  echo "[x] Completion failed: $(echo $RESPONSE | jq -r '.error.message')"
  exit 1
fi
echo "[check] Completion OK"

echo ""
echo "=== All tests passed ==="
```

### Anthropic Full Test

```bash
#!/bin/bash
echo "=== Anthropic API Test ==="

# 1. Check API key format
if [[ ! "$ANTHROPIC_API_KEY" =~ ^sk-ant- ]]; then
  echo "[x] Invalid API key format (should start with 'sk-ant-')"
  exit 1
fi
echo "[check] API key format OK"

# 2. Test authentication with message
RESPONSE=$(curl -s https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-3-5-haiku-latest","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}')

if echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
  ERROR_TYPE=$(echo "$RESPONSE" | jq -r '.error.type')
  ERROR_MSG=$(echo "$RESPONSE" | jq -r '.error.message')
  echo "[x] API error: $ERROR_TYPE - $ERROR_MSG"
  exit 1
fi
echo "[check] Authentication OK"
echo "[check] Model accessible"
echo "[check] Completion OK"

echo ""
echo "=== All tests passed ==="
```

### Ollama Full Test

```bash
#!/bin/bash
echo "=== Ollama API Test ==="

# 1. Check if Ollama is running
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
  echo "[x] Ollama not running on localhost:11434"
  echo "    Start with: ollama serve"
  exit 1
fi
echo "[check] Ollama running"

# 2. Check model availability
MODEL="llama3.2:3b"
MODELS=$(curl -s http://localhost:11434/api/tags | jq -r '.models[].name')

if ! echo "$MODELS" | grep -q "$MODEL"; then
  echo "[!] Model $MODEL not found"
  echo "    Pull with: ollama pull $MODEL"
  echo "    Available models: $MODELS"
  exit 1
fi
echo "[check] Model $MODEL available"

# 3. Test generation
RESPONSE=$(curl -s http://localhost:11434/api/generate \
  -d "{\"model\":\"$MODEL\",\"prompt\":\"Hi\",\"stream\":false}")

if echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
  echo "[x] Generation failed: $(echo $RESPONSE | jq -r '.error')"
  exit 1
fi
echo "[check] Generation OK"

echo ""
echo "=== All tests passed ==="
```

## Common Error Codes

| Code | Provider | Meaning | Resolution |
|------|----------|---------|------------|
| 401 | OpenAI | Invalid API key | Verify key at platform.openai.com |
| 401 | Anthropic | Invalid API key | Verify key at console.anthropic.com |
| 403 | OpenAI | No billing/access | Add payment method |
| 404 | All | Model not found | Check model name spelling |
| 429 | All | Rate limited | Wait and retry |
| 500 | All | Server error | Try again later |
| 503 | All | Service unavailable | Try again later |

## Troubleshooting

### "Invalid API key"

```bash
# Check key is set
echo ${OPENAI_API_KEY:0:10}...  # Show first 10 chars

# Check for extra whitespace
echo "$OPENAI_API_KEY" | xxd | head -5
```

### "Model not found"

```bash
# List available OpenAI models
curl -s -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models | jq -r '.data[].id' | sort
```

### "Rate limited"

```bash
# Check rate limit headers
curl -s -I -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models | grep -i rate
```

### "Connection refused" (Ollama)

```bash
# Check if Ollama is running
pgrep -x ollama || echo "Ollama not running"

# Start Ollama
ollama serve &

# Check port
lsof -i :11434
```

## Integrated Test Command

Use the built-in test:

```bash
# Test current configuration
/memory-llm --test

# Expected output:
# Testing LLM connection...
#   [check] Provider: OpenAI
#   [check] API key: Valid
#   [check] Model: gpt-4o-mini accessible
#   [check] Completion: OK (latency: 234ms)
```
