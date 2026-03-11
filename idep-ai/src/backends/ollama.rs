// Ollama backend — local inference, no API key needed

use super::Backend;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{Client, Response, StatusCode};
use serde_json::json;
use std::any::Any;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

pub struct OllamaBackend {
    client: Client,
    url: String,
    model: String,
}

impl OllamaBackend {
    pub fn new(url: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
            url,
            model,
        }
    }
}

#[async_trait]
impl Backend for OllamaBackend {
    fn name(&self) -> &str {
        "ollama"
    }

    fn info(&self) -> super::BackendInfo {
        super::BackendInfo {
            name: "ollama",
            version: None,
            endpoint: self.url.clone(),
            cloud_dependent: false,
            requires_auth: false,
        }
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        debug!("Ollama complete: model={}", self.model);

        let body = json!({
            "model":  self.model,
            "prompt": prompt,
            "stream": true,
            "raw": true,
            "options": {
                "num_predict": max_tokens,
                "temperature": 0.0,
            }
        });

        let request = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&body);

        let response = self.send_with_retry(request).await?.error_for_status()?;

        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        let mut result = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(token) = event["response"].as_str() {
                        result.push_str(token);
                    }
                    if event["done"].as_bool().unwrap_or(false) {
                        break;
                    }
                }
            }
        }

        Ok(result)
    }
}

impl OllamaBackend {
    pub async fn stream_completion<F>(
        &self,
        prompt: &str,
        max_tokens: u32,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str) + Send,
    {
        debug!("Ollama stream: model={}", self.model);

        let body = json!({
            "model":  self.model,
            "prompt": prompt,
            "stream": true,
            "raw": true,
            "options": {
                "num_predict": max_tokens,
                "temperature": 0.0,
            }
        });

        let request = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&body);

        let response = self.send_with_retry(request).await?.error_for_status()?;

        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        let mut result = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(token) = event["response"].as_str() {
                        result.push_str(token);
                        on_token(token);
                    }
                    if event["done"].as_bool().unwrap_or(false) {
                        break;
                    }
                }
            }
        }

        Ok(result)
    }
}

impl OllamaBackend {
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
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
}
