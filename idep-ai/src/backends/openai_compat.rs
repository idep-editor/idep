// OpenAI-compatible backend
// Works with: LiteLLM, LocalAI, Together.ai, any /v1/chat/completions endpoint

use super::Backend;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::debug;

pub struct OpenAiCompatBackend {
    client: Client,
    url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiCompatBackend {
    pub fn new(url: String, api_key: Option<String>, model: String) -> Self {
        Self {
            client: Client::new(),
            url,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl Backend for OpenAiCompatBackend {
    fn name(&self) -> &str {
        "openai_compat"
    }

    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        debug!(
            "OpenAI-compat complete: model={} url={}",
            self.model, self.url
        );

        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "stream": true,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let mut req = self
            .client
            .post(format!("{}/v1/chat/completions", self.url))
            .json(&body);

        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.send().await?.error_for_status()?;

        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut result = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = event
                            .pointer("/choices/0/delta/content")
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
