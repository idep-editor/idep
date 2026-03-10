// completion — FIM-aware inline code completions
//
// Fill-in-Middle (FIM) sends the model both the code BEFORE
// and AFTER the cursor, producing more accurate completions.

use crate::backends::Backend;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// FIM token configuration per model family
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FimTokens {
    pub prefix: String,  // inserted before the code prefix
    pub suffix: String,  // inserted before the code suffix
    pub middle: String,  // inserted at the cursor — model fills here
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
}

/// The result of a completion
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub text: String,
}

/// Completion engine — wraps a backend with FIM prompt construction
pub struct CompletionEngine {
    backend:    Box<dyn Backend>,
    fim_tokens: FimTokens,
}

impl CompletionEngine {
    pub fn new(backend: Box<dyn Backend>, fim_tokens: FimTokens) -> Self {
        Self { backend, fim_tokens }
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

        let mut result = String::new();
        self.backend.complete(&prompt, req.max_tokens, |token| {
            result.push_str(&token);
        }).await?;

        Ok(CompletionResponse { text: result })
    }
}
