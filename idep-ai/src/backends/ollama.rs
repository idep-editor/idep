// Ollama backend — local inference, no API key needed

use super::Backend;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::debug;

pub struct OllamaBackend {
    client: Client,
    url: String,
    model: String,
}

impl OllamaBackend {
    pub fn new(url: String, model: String) -> Self {
        Self {
            client: Client::new(),
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

    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        debug!("Ollama complete: model={}", self.model);

        let body = json!({
            "model":  self.model,
            "prompt": prompt,
            "stream": true,
            "options": {
                "num_predict": max_tokens,
            }
        });

        let response = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

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
