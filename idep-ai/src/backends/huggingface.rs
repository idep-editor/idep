// HuggingFace Inference API backend

use super::Backend;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
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
            client: Client::new(),
            api_token,
            model,
            endpoint,
        }
    }
}

#[async_trait]
impl Backend for HuggingFaceBackend {
    fn name(&self) -> &str {
        "huggingface"
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

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let result: serde_json::Value = response.json().await?;
        if let Some(text) = result[0]["generated_text"].as_str() {
            Ok(text.to_string())
        } else {
            Ok(String::new())
        }
    }
}
