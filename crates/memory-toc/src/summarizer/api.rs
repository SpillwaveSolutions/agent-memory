//! API-based summarizer using OpenAI-compatible endpoints.

use async_trait::async_trait;
use backoff::{backoff::Backoff, ExponentialBackoff};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};

use memory_types::Event;

use super::{Summarizer, SummarizerError, Summary};

/// Configuration for API-based summarizer.
#[derive(Debug, Clone)]
pub struct ApiSummarizerConfig {
    /// API base URL (e.g., "https://api.openai.com/v1")
    pub base_url: String,

    /// Model to use (e.g., "gpt-4o-mini", "claude-3-haiku-20240307")
    pub model: String,

    /// API key
    pub api_key: SecretString,

    /// Request timeout
    pub timeout: Duration,

    /// Maximum retries on failure
    pub max_retries: u32,
}

impl ApiSummarizerConfig {
    /// Create config for OpenAI API.
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            model: model.into(),
            api_key: SecretString::from(api_key.into()),
            timeout: Duration::from_secs(60),
            max_retries: 3,
        }
    }

    /// Create config for Claude API.
    pub fn claude(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: "https://api.anthropic.com/v1".to_string(),
            model: model.into(),
            api_key: SecretString::from(api_key.into()),
            timeout: Duration::from_secs(60),
            max_retries: 3,
        }
    }
}

/// API-based summarizer implementation.
pub struct ApiSummarizer {
    client: Client,
    config: ApiSummarizerConfig,
}

impl ApiSummarizer {
    /// Create a new API summarizer.
    pub fn new(config: ApiSummarizerConfig) -> Result<Self, SummarizerError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| SummarizerError::ConfigError(e.to_string()))?;

        Ok(Self { client, config })
    }

    /// Build prompt for event summarization.
    fn build_events_prompt(&self, events: &[Event]) -> String {
        let events_text: String = events
            .iter()
            .map(|e| {
                let timestamp = e.timestamp.format("%Y-%m-%d %H:%M:%S");
                format!("[{}] {}: {}", timestamp, e.role, e.text)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"Summarize this conversation segment for a Table of Contents entry.

CONVERSATION:
{events_text}

Provide your response in JSON format:
{{
  "title": "Brief title (5-10 words)",
  "bullets": ["Key point 1", "Key point 2", "Key point 3"],
  "keywords": ["keyword1", "keyword2", "keyword3"]
}}

Guidelines:
- Title should capture the main topic or activity
- 3-5 bullet points summarizing key discussions or decisions
- 3-7 keywords for search/filtering
- Focus on what would help someone find this conversation later"#
        )
    }

    /// Build prompt for rollup summarization.
    fn build_rollup_prompt(&self, summaries: &[Summary]) -> String {
        let summaries_text: String = summaries
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let bullets = s.bullets.join("\n  - ");
                format!(
                    "### Summary {}\nTitle: {}\nBullets:\n  - {}\nKeywords: {}",
                    i + 1,
                    s.title,
                    bullets,
                    s.keywords.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"Create a higher-level summary by aggregating these child summaries.

CHILD SUMMARIES:
{summaries_text}

Provide your response in JSON format:
{{
  "title": "Brief title (5-10 words)",
  "bullets": ["Key point 1", "Key point 2", "Key point 3"],
  "keywords": ["keyword1", "keyword2", "keyword3"]
}}

Guidelines:
- Title should capture the overall theme
- 3-5 bullet points covering the most important topics across all children
- 3-7 keywords representing major themes
- Focus on themes and patterns, not individual details"#
        )
    }

    /// Call the API with retry logic.
    async fn call_api(&self, prompt: &str) -> Result<String, SummarizerError> {
        let mut backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(120)),
            ..Default::default()
        };

        let mut attempts = 0;

        loop {
            attempts += 1;
            debug!(attempt = attempts, "Calling summarization API");

            match self.make_request(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempts >= self.config.max_retries {
                        error!(error = %e, "Max retries exceeded");
                        return Err(e);
                    }

                    match backoff.next_backoff() {
                        Some(duration) => {
                            warn!(
                                error = %e,
                                retry_in_ms = duration.as_millis(),
                                "API call failed, retrying"
                            );
                            tokio::time::sleep(duration).await;
                        }
                        None => {
                            error!(error = %e, "Backoff exhausted");
                            return Err(e);
                        }
                    }
                }
            }
        }
    }

    /// Make a single API request.
    async fn make_request(&self, prompt: &str) -> Result<String, SummarizerError> {
        // Build request based on API type
        let is_anthropic = self.config.base_url.contains("anthropic");

        let response = if is_anthropic {
            self.make_anthropic_request(prompt).await?
        } else {
            self.make_openai_request(prompt).await?
        };

        Ok(response)
    }

    /// Make OpenAI-compatible API request.
    async fn make_openai_request(&self, prompt: &str) -> Result<String, SummarizerError> {
        #[derive(Serialize)]
        struct OpenAIRequest {
            model: String,
            messages: Vec<OpenAIMessage>,
            response_format: OpenAIResponseFormat,
        }

        #[derive(Serialize)]
        struct OpenAIMessage {
            role: String,
            content: String,
        }

        #[derive(Serialize)]
        struct OpenAIResponseFormat {
            #[serde(rename = "type")]
            format_type: String,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
        }

        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIMessageResponse,
        }

        #[derive(Deserialize)]
        struct OpenAIMessageResponse {
            content: String,
        }

        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            response_format: OpenAIResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| SummarizerError::ApiError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SummarizerError::RateLimitExceeded);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SummarizerError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let response_body: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| SummarizerError::ParseError(e.to_string()))?;

        response_body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| SummarizerError::ParseError("No choices in response".to_string()))
    }

    /// Make Anthropic API request.
    async fn make_anthropic_request(&self, prompt: &str) -> Result<String, SummarizerError> {
        #[derive(Serialize)]
        struct AnthropicRequest {
            model: String,
            max_tokens: u32,
            messages: Vec<AnthropicMessage>,
        }

        #[derive(Serialize)]
        struct AnthropicMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
        }

        #[derive(Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: 1024,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let url = format!("{}/messages", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", self.config.api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| SummarizerError::ApiError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SummarizerError::RateLimitExceeded);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SummarizerError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let response_body: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| SummarizerError::ParseError(e.to_string()))?;

        response_body
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| SummarizerError::ParseError("No content in response".to_string()))
    }

    /// Parse JSON response into Summary.
    fn parse_summary(&self, response: &str) -> Result<Summary, SummarizerError> {
        // Try to extract JSON from response (in case there's extra text)
        let json_str = extract_json(response);

        serde_json::from_str(&json_str).map_err(|e| {
            SummarizerError::ParseError(format!("Failed to parse summary JSON: {}", e))
        })
    }
}

/// Extract JSON object from text (handles markdown code blocks).
fn extract_json(text: &str) -> String {
    // Check for markdown code block
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start + 7..].find("```") {
            return text[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Check for plain code block
    if let Some(start) = text.find("```") {
        if let Some(end) = text[start + 3..].find("```") {
            return text[start + 3..start + 3 + end].trim().to_string();
        }
    }

    // Find first { and last }
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        return text[start..=end].to_string();
    }

    text.to_string()
}

#[async_trait]
impl Summarizer for ApiSummarizer {
    async fn summarize_events(&self, events: &[Event]) -> Result<Summary, SummarizerError> {
        if events.is_empty() {
            return Err(SummarizerError::NoEvents);
        }

        let prompt = self.build_events_prompt(events);
        let response = self.call_api(&prompt).await?;
        self.parse_summary(&response)
    }

    async fn summarize_children(&self, summaries: &[Summary]) -> Result<Summary, SummarizerError> {
        if summaries.is_empty() {
            return Err(SummarizerError::NoEvents);
        }

        let prompt = self.build_rollup_prompt(summaries);
        let response = self.call_api(&prompt).await?;
        self.parse_summary(&response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_plain() {
        let text = r#"{"title": "Test", "bullets": [], "keywords": []}"#;
        let json = extract_json(text);
        assert_eq!(json, text);
    }

    #[test]
    fn test_extract_json_code_block() {
        let text = r#"Here's the summary:
```json
{"title": "Test", "bullets": [], "keywords": []}
```"#;
        let json = extract_json(text);
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_extract_json_with_prefix() {
        let text = r#"Sure! Here's your summary: {"title": "Test", "bullets": [], "keywords": []}"#;
        let json = extract_json(text);
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_openai_config() {
        let config = ApiSummarizerConfig::openai("test-key", "gpt-4o-mini");
        assert!(config.base_url.contains("openai"));
        assert_eq!(config.model, "gpt-4o-mini");
    }

    #[test]
    fn test_claude_config() {
        let config = ApiSummarizerConfig::claude("test-key", "claude-3-haiku-20240307");
        assert!(config.base_url.contains("anthropic"));
        assert_eq!(config.model, "claude-3-haiku-20240307");
    }
}
