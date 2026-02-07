# Custom Endpoints

Configure agent-memory to use custom LLM endpoints for Azure OpenAI, LocalAI, LM Studio, and other OpenAI-compatible APIs.

## When to Use Custom Endpoints

- **Azure OpenAI** - Enterprise deployments with Azure
- **LocalAI** - Self-hosted OpenAI-compatible server
- **LM Studio** - Desktop app for local LLM serving
- **Proxy servers** - API proxies for rate limiting or caching
- **Private deployments** - On-premise LLM hosting

## Basic Configuration

```toml
[summarizer]
provider = "openai"  # Use OpenAI-compatible protocol
api_endpoint = "https://your-custom-endpoint/v1"
model = "your-model-name"
api_key = "your-api-key"  # Or use environment variable
```

## Azure OpenAI

### Prerequisites

1. Azure subscription with OpenAI service enabled
2. Deployed model in Azure OpenAI Studio
3. API key and endpoint from Azure portal

### Configuration

```toml
[summarizer]
provider = "openai"
api_endpoint = "https://your-resource.openai.azure.com/openai/deployments/your-deployment"
model = "gpt-4o-mini"  # Your deployment name
api_version = "2024-02-01"
```

### Environment Variables

```bash
export AZURE_OPENAI_API_KEY="your-azure-key"
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
```

### Full Example

```toml
[summarizer]
provider = "openai"
api_endpoint = "https://mycompany.openai.azure.com/openai/deployments/gpt4o-mini"
model = "gpt4o-mini"
api_version = "2024-02-01"
# api_key loaded from AZURE_OPENAI_API_KEY
```

## LocalAI

Run OpenAI-compatible API locally with any model.

### Setup

```bash
# Install LocalAI
docker run -p 8080:8080 localai/localai:latest

# Or with specific model
docker run -p 8080:8080 -v ./models:/models \
  localai/localai:latest --models-path /models
```

### Configuration

```toml
[summarizer]
provider = "openai"
api_endpoint = "http://localhost:8080/v1"
model = "gpt-3.5-turbo"  # Or your loaded model name
# No api_key needed for local
```

### Testing

```bash
# Verify LocalAI is running
curl http://localhost:8080/v1/models

# Test completion
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"test"}]}'
```

## LM Studio

Desktop application for running local LLMs with OpenAI-compatible server.

### Setup

1. Download LM Studio from https://lmstudio.ai
2. Load a model (e.g., Llama, Mistral)
3. Start local server (default port 1234)

### Configuration

```toml
[summarizer]
provider = "openai"
api_endpoint = "http://localhost:1234/v1"
model = "local-model"  # LM Studio uses any model name
# No api_key needed
```

### Notes

- LM Studio must be running when daemon starts
- Model must be loaded in LM Studio
- Server runs on port 1234 by default

## Proxy Servers

For caching, rate limiting, or request modification.

### Configuration

```toml
[summarizer]
provider = "openai"
api_endpoint = "https://your-proxy-server/v1"
model = "gpt-4o-mini"
# Proxy may require authentication
api_key = "your-proxy-key"
```

### Common Proxy Features

- Request caching to reduce API calls
- Rate limit management
- Request/response logging
- Cost tracking

## Ollama with Remote Server

Run Ollama on a separate server.

### Server Setup

```bash
# On server (allow remote connections)
OLLAMA_HOST=0.0.0.0 ollama serve
```

### Configuration

```toml
[summarizer]
provider = "ollama"
api_endpoint = "http://your-server:11434"
model = "llama3.2:3b"
```

## Testing Custom Endpoints

Before applying configuration:

```bash
# Test endpoint availability
curl -s -o /dev/null -w "%{http_code}" $API_ENDPOINT/models

# Test completion
curl -X POST "$API_ENDPOINT/chat/completions" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "your-model",
    "messages": [{"role": "user", "content": "test"}],
    "max_tokens": 5
  }'
```

## Troubleshooting

### Connection Refused

```
Error: Connection refused to custom endpoint
```

Fix:
1. Verify endpoint URL is correct
2. Check if service is running
3. Verify firewall/network allows connection
4. Check if HTTPS is required

### Authentication Failed

```
Error: 401 Unauthorized
```

Fix:
1. Verify API key is correct
2. Check key is set in environment or config
3. Verify key has proper permissions

### Model Not Found

```
Error: Model 'your-model' not found
```

Fix:
1. List available models: `curl $API_ENDPOINT/models`
2. Use exact model name from list
3. For Azure, use deployment name not model name

### SSL/TLS Errors

```
Error: SSL certificate verify failed
```

Fix:
```toml
[summarizer]
# For self-signed certificates (not recommended for production)
ssl_verify = false
```

Or properly install certificates.

## Security Considerations

1. **Use HTTPS** for remote endpoints
2. **Rotate API keys** regularly
3. **Use environment variables** for secrets, not config files
4. **Firewall restrictions** for internal endpoints
5. **Monitor usage** for anomalies
