// HuggingFace Inference API backend

use super::Backend;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{header::RETRY_AFTER, Client, Response, StatusCode};
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

pub struct HuggingFaceBackend {
    client: Client,
    api_token: String,
    model: String,
    endpoint: String,
}

impl HuggingFaceBackend {
    pub fn new(api_token: String, model: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint
            .unwrap_or_else(|| format!("https://api-inference.huggingface.co/models/{model}"));
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
            api_token,
            model,
            endpoint,
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

#[async_trait]
impl Backend for HuggingFaceBackend {
    fn name(&self) -> &str {
        "huggingface"
    }

    fn info(&self) -> super::BackendInfo {
        super::BackendInfo {
            name: "huggingface",
            version: None,
            endpoint: self.endpoint.clone(),
            cloud_dependent: true,
            requires_auth: true,
        }
    }

    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        debug!("HuggingFace complete: model={}", self.model);

        use serde_json::json;
        let body = json!({
            "inputs": prompt,
            "parameters": {
                "max_new_tokens": max_tokens,
                "return_full_text": false,
            }
        });

        let request = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_token)
            .json(&body);

        let response = self.send_with_retry(request).await?.error_for_status()?;

        let result: serde_json::Value = response.json().await?;
        if let Some(text) = result[0]["generated_text"].as_str() {
            Ok(text.to_string())
        } else {
            Ok(String::new())
        }
    }
}

fn parse_retry_after(resp: &Response) -> Option<Duration> {
    resp.headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
}
