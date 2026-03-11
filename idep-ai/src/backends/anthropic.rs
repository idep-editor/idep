// Anthropic Claude backend — streaming via SSE

use super::Backend;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{header::RETRY_AFTER, Client, Response, StatusCode};
use serde_json::json;
use std::{any::Any, time::Duration};
use tokio::time::sleep;
use tracing::debug;

pub struct AnthropicBackend {
    client: Client,
    api_key: String,
    model: String,
    max_tokens: u32,
    base_url: String,
}

impl AnthropicBackend {
    pub fn new(api_key: String, model: String, max_tokens: u32) -> Self {
        Self::new_with_base_url(
            api_key,
            model,
            max_tokens,
            "https://api.anthropic.com".into(),
        )
    }

    pub fn new_with_base_url(
        api_key: String,
        model: String,
        max_tokens: u32,
        base_url: String,
    ) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
            api_key,
            model,
            max_tokens,
            base_url,
        }
    }

    async fn send_with_retry(&self, request: reqwest::RequestBuilder) -> Result<Response> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 200;

        for attempt in 0..MAX_RETRIES {
            let resp = request
                .try_clone()
                .ok_or_else(|| anyhow!("retryable request missing clone"))?
                .send()
                .await;

            match resp {
                Ok(r) => {
                    if r.status().is_success() {
                        return Ok(r);
                    }

                    if r.status() == StatusCode::TOO_MANY_REQUESTS {
                        if let Some(delay) = parse_retry_after(&r) {
                            sleep(delay).await;
                            continue;
                        }
                    }

                    if r.status().is_server_error() && attempt + 1 < MAX_RETRIES {
                        sleep(Duration::from_millis(BASE_DELAY_MS * 2u64.pow(attempt))).await;
                        continue;
                    }

                    return Err(anyhow!("request failed with status {}", r.status()));
                }
                Err(_e) if attempt + 1 < MAX_RETRIES => {
                    sleep(Duration::from_millis(BASE_DELAY_MS * 2u64.pow(attempt))).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(anyhow!("exhausted retries"))
    }
}

fn parse_retry_after(resp: &Response) -> Option<Duration> {
    resp.headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[async_trait]
impl Backend for AnthropicBackend {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn info(&self) -> super::BackendInfo {
        super::BackendInfo {
            name: "anthropic",
            version: None,
            endpoint: self.base_url.clone(),
            cloud_dependent: true,
            requires_auth: true,
        }
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        debug!("Anthropic complete: model={}", self.model);

        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens.min(self.max_tokens),
            "stream": true,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let request = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await?.error_for_status()?;

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
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = event.pointer("/delta/text").and_then(|v| v.as_str()) {
                            result.push_str(text);
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}
