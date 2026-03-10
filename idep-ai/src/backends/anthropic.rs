// Anthropic Claude backend — streaming via SSE

use super::Backend;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::debug;

pub struct AnthropicBackend {
    client:     Client,
    api_key:    String,
    model:      String,
    max_tokens: u32,
}

impl AnthropicBackend {
    pub fn new(api_key: String, model: String, max_tokens: u32) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            max_tokens,
        }
    }
}

#[async_trait]
impl Backend for AnthropicBackend {
    fn name(&self) -> &str { "anthropic" }

    async fn complete(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<String> {
        debug!("Anthropic complete: model={}", self.model);

        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens.min(self.max_tokens),
            "stream": true,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        // Stream SSE events
        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut result = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Parse SSE lines
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { break; }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = event
                            .pointer("/delta/text")
                            .and_then(|v| v.as_str())
                        {
                            result.push_str(text);
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}
