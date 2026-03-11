// completion — FIM-aware inline code completions
//
// Fill-in-Middle (FIM) sends the model both the code BEFORE
// and AFTER the cursor, producing more accurate completions.

use crate::backends::Backend;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// FIM token configuration per model family
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FimTokens {
    pub prefix: String, // inserted before the code prefix
    pub suffix: String, // inserted before the code suffix
    pub middle: String, // inserted at the cursor — model fills here
}

impl FimTokens {
    /// DeepSeek Coder family
    pub fn deepseek() -> Self {
        Self {
            prefix: "<｜fim▁begin｜>".into(),
            suffix: "<｜fim▁end｜>".into(),
            middle: "<｜fim▁middle｜>".into(),
        }
    }

    /// StarCoder / StarCoder2 family
    pub fn starcoder() -> Self {
        Self {
            prefix: "<fim_prefix>".into(),
            suffix: "<fim_suffix>".into(),
            middle: "<fim_middle>".into(),
        }
    }

    /// CodeLlama family
    pub fn codellama() -> Self {
        Self {
            prefix: "▁<PRE>".into(),
            suffix: "▁<SUF>".into(),
            middle: "▁<MID>".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct EchoBackend;

    #[async_trait]
    impl Backend for EchoBackend {
        async fn complete(&self, prompt: &str, _max_tokens: u32) -> Result<String> {
            Ok(prompt.to_string())
        }

        fn name(&self) -> &str {
            "echo"
        }

        fn info(&self) -> crate::backends::BackendInfo {
            crate::backends::BackendInfo {
                name: "echo",
                version: None,
                endpoint: "local".into(),
                cloud_dependent: false,
                requires_auth: false,
            }
        }
    }

    async fn assert_fim_tokens(tokens: FimTokens) {
        let engine = CompletionEngine::new(Box::new(EchoBackend), tokens.clone());
        let req = CompletionRequest {
            prefix: "PRE".into(),
            suffix: "SUF".into(),
            language: "rust".into(),
            max_tokens: 32,
            stop_sequences: None,
        };

        let resp = engine.complete(req).await.unwrap();
        let expected = format!(
            "{}{}{}{}{}",
            tokens.prefix, "PRE", tokens.suffix, "SUF", tokens.middle
        );
        assert_eq!(resp.text, expected);
        assert!(!tokens.prefix.is_empty());
        assert!(!tokens.suffix.is_empty());
        assert!(!tokens.middle.is_empty());
    }

    #[tokio::test]
    async fn deepseek_fim_tokens_build_prompt() {
        assert_fim_tokens(FimTokens::deepseek()).await;
    }

    #[tokio::test]
    async fn starcoder_fim_tokens_build_prompt() {
        assert_fim_tokens(FimTokens::starcoder()).await;
    }

    #[tokio::test]
    async fn codellama_fim_tokens_build_prompt() {
        assert_fim_tokens(FimTokens::codellama()).await;
    }

    struct DelayedBackend;

    #[async_trait]
    impl Backend for DelayedBackend {
        async fn complete(&self, prompt: &str, _max_tokens: u32) -> Result<String> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(prompt.to_string())
        }

        fn name(&self) -> &str {
            "delayed"
        }

        fn info(&self) -> crate::backends::BackendInfo {
            crate::backends::BackendInfo {
                name: "delayed",
                version: None,
                endpoint: "local".into(),
                cloud_dependent: false,
                requires_auth: false,
            }
        }
    }

    #[tokio::test]
    async fn measures_latency_to_first_token() {
        let engine = CompletionEngine::new(Box::new(DelayedBackend), FimTokens::deepseek());
        let req = CompletionRequest {
            prefix: "A".into(),
            suffix: "B".into(),
            language: "rust".into(),
            max_tokens: 8,
            stop_sequences: None,
        };

        let (_resp, latency) = engine.complete_with_latency(req).await.unwrap();
        assert!(latency >= Duration::from_millis(50));
    }
}

/// A request for inline completion at cursor position
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// Code before the cursor
    pub prefix: String,
    /// Code after the cursor (for FIM)
    pub suffix: String,
    /// File language identifier (e.g. "rust", "typescript")
    pub language: String,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Optional stop sequences (truncate on first occurrence)
    pub stop_sequences: Option<Vec<String>>,
}

/// The result of a completion
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub text: String,
}

/// Completion engine — wraps a backend with FIM prompt construction
pub struct CompletionEngine {
    backend: Box<dyn Backend>,
    fim_tokens: FimTokens,
}

impl CompletionEngine {
    pub fn new(backend: Box<dyn Backend>, fim_tokens: FimTokens) -> Self {
        Self {
            backend,
            fim_tokens,
        }
    }

    /// Build the FIM prompt and run completion
    pub async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let prompt = format!(
            "{}{}{}{}{}",
            self.fim_tokens.prefix,
            req.prefix,
            self.fim_tokens.suffix,
            req.suffix,
            self.fim_tokens.middle,
        );

        let text = self.backend.complete(&prompt, req.max_tokens).await?;
        let truncated = if let Some(stops) = &req.stop_sequences {
            truncate_on_stop(&text, stops)
        } else {
            text
        };

        Ok(CompletionResponse { text: truncated })
    }

    /// Measure latency from call to first token (full response, since backend is non-streaming)
    pub async fn complete_with_latency(
        &self,
        req: CompletionRequest,
    ) -> Result<(CompletionResponse, Duration)> {
        let start = Instant::now();
        let resp = self.complete(req).await?;
        let latency = start.elapsed();
        Ok((resp, latency))
    }
}

fn truncate_on_stop(text: &str, stops: &[String]) -> String {
    let mut earliest = None;
    for stop in stops {
        if stop.is_empty() {
            continue;
        }
        if let Some(idx) = text.find(stop) {
            earliest = Some(earliest.map_or(idx, |cur: usize| cur.min(idx)));
        }
    }

    if let Some(idx) = earliest {
        text[..idx].to_string()
    } else {
        text.to_string()
    }
}
