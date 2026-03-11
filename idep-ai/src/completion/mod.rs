// completion — FIM-aware inline code completions
//
// Fill-in-Middle (FIM) sends the model both the code BEFORE
// and AFTER the cursor, producing more accurate completions.

use crate::backends::Backend;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// FIM token configuration per model family
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FimTokens {
    pub prefix: String,              // inserted before the code prefix
    pub suffix: String,              // inserted before the code suffix
    pub middle: String,              // inserted at the cursor — model fills here
    pub stop_sequences: Vec<String>, // model-specific stop tokens for Ollama
}

impl FimTokens {
    /// DeepSeek Coder family
    pub fn deepseek() -> Self {
        Self {
            prefix: "<｜fim▁begin｜>".into(),
            suffix: "<｜fim▁end｜>".into(),
            middle: "<｜fim▁middle｜>".into(),
            stop_sequences: vec![
                "}\n".into(),
                "<｜fim▁end｜>".into(),
                "<｜end▁of▁sentence｜>".into(),
            ],
        }
    }

    /// StarCoder / StarCoder2 family
    pub fn starcoder() -> Self {
        Self {
            prefix: "<fim_prefix>".into(),
            suffix: "<fim_suffix>".into(),
            middle: "<fim_middle>".into(),
            stop_sequences: vec!["}\n".into()],
        }
    }

    /// CodeLlama family
    pub fn codellama() -> Self {
        Self {
            prefix: "▁<PRE>".into(),
            suffix: "▁<SUF>".into(),
            middle: "▁<MID>".into(),
            stop_sequences: vec!["}\n".into()],
        }
    }

    /// Select FIM tokens based on model name
    pub fn for_model(model: &str) -> Self {
        if model.contains("deepseek") {
            Self::deepseek()
        } else if model.contains("starcoder") {
            Self::starcoder()
        } else if model.contains("codellama") {
            Self::codellama()
        } else {
            // Default to DeepSeek for unknown models
            Self::deepseek()
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

        fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
            self
        }
    }

    async fn assert_fim_tokens(tokens: FimTokens) {
        // Create tokens without stop sequences for testing
        let test_tokens = FimTokens {
            prefix: tokens.prefix.clone(),
            suffix: tokens.suffix.clone(),
            middle: tokens.middle.clone(),
            stop_sequences: vec![], // Disable stop sequences for this test
        };
        let engine = CompletionEngine::new(Box::new(EchoBackend), test_tokens);
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

    #[test]
    fn for_model_selects_correct_tokens() {
        assert_eq!(
            FimTokens::for_model("deepseek-coder:1.3b"),
            FimTokens::deepseek()
        );
        assert_eq!(
            FimTokens::for_model("starcoder2:7b"),
            FimTokens::starcoder()
        );
        assert_eq!(
            FimTokens::for_model("codellama:13b"),
            FimTokens::codellama()
        );
        // Unknown models default to DeepSeek
        assert_eq!(FimTokens::for_model("mistral:7b"), FimTokens::deepseek());
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

        fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
            self
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

    #[tokio::test]
    #[ignore] // run with: cargo test -- --ignored
    async fn live_fim_completion() {
        // Requires live Ollama at http://localhost:11434 with deepseek-coder:1.3b
        use crate::backends::ollama::OllamaBackend;

        let backend = Box::new(OllamaBackend::new(
            "http://localhost:11434".into(),
            "deepseek-coder:1.3b".into(),
        ));
        let engine = CompletionEngine::new(backend, FimTokens::deepseek());

        let req = CompletionRequest {
            prefix: "fn add(a: i32, b: i32) -> i32 {\n".into(),
            suffix: "\n}".into(),
            language: "rust".into(),
            max_tokens: 32,
            stop_sequences: Some(vec!["}".into()]),
        };

        let resp = engine
            .complete(req)
            .await
            .expect("live Ollama should respond");
        // Verify response is non-empty (FIM is working; language varies by model)
        assert!(!resp.text.is_empty(), "completion should not be empty");
        // Verify stop-sequence was respected (no closing brace in response)
        assert!(
            !resp.text.contains("}"),
            "completion should stop before closing brace, got: {}",
            resp.text
        );
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

        // Merge FIM model stop sequences with request stop sequences
        let mut all_stops = self.fim_tokens.stop_sequences.clone();
        if let Some(req_stops) = &req.stop_sequences {
            all_stops.extend(req_stops.clone());
        }

        let truncated = if !all_stops.is_empty() {
            truncate_on_stop(&text, &all_stops)
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
