// backends — unified interface over multiple AI inference providers
//
// All providers speak the same Backend trait.
// Callers never need to know which HTTP API is underneath.

pub mod anthropic;
pub mod huggingface;
pub mod ollama;
pub mod openai_compat;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A single streaming token from the backend
pub type Token = String;

/// Metadata returned by each backend for diagnostics and `idep --backends`.
#[derive(Debug, Clone)]
pub struct BackendInfo {
    pub name: &'static str,
    pub version: Option<String>, // e.g. model version if detectable
    pub endpoint: String,        // resolved endpoint URL
    pub cloud_dependent: bool,   // true if requires internet
    pub requires_auth: bool,     // true if API key required
}

/// Backend configuration loaded from ~/.idep/config.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum BackendConfig {
    Anthropic {
        api_key: String,
        model: String,
        #[serde(default = "default_max_tokens")]
        max_tokens: u32,
    },
    HuggingFace {
        api_token: String,
        model: String,
        endpoint: Option<String>, // custom inference endpoint
    },
    Ollama {
        #[serde(default = "default_ollama_url")]
        url: String,
        model: String,
    },
    OpenAiCompat {
        url: String,
        api_key: Option<String>,
        model: String,
    },
}

fn default_max_tokens() -> u32 {
    1024
}
fn default_ollama_url() -> String {
    "http://localhost:11434".into()
}

/// A backend is anything that can stream completions
#[async_trait]
pub trait Backend: Send + Sync {
    /// Complete a prompt and return the full response
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String>;

    /// Name for logging / debug
    fn name(&self) -> &str;

    /// Return static metadata about this backend configuration.
    fn info(&self) -> BackendInfo;
}

/// Construct the right backend from config
pub fn from_config(config: BackendConfig) -> Box<dyn Backend> {
    match config {
        BackendConfig::Anthropic {
            api_key,
            model,
            max_tokens,
        } => Box::new(anthropic::AnthropicBackend::new(api_key, model, max_tokens)),
        BackendConfig::HuggingFace {
            api_token,
            model,
            endpoint,
        } => Box::new(huggingface::HuggingFaceBackend::new(
            api_token, model, endpoint,
        )),
        BackendConfig::Ollama { url, model } => Box::new(ollama::OllamaBackend::new(url, model)),
        BackendConfig::OpenAiCompat {
            url,
            api_key,
            model,
        } => Box::new(openai_compat::OpenAiCompatBackend::new(url, api_key, model)),
    }
}
