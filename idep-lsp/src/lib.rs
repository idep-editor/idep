// idep-lsp — LSP server bridge
//
// Language server protocol orchestration and client management

use anyhow::Result;
use idep_ai::{
    backends::Backend,
    completion::{CompletionEngine, CompletionRequest},
};
use lsp_types::{CompletionItem, CompletionResponse};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// LSP completion handler — bridges textDocument/completion requests to CompletionEngine
pub struct CompletionHandler {
    engine: Arc<CompletionEngine>,
    debounce: Duration,
}

impl CompletionHandler {
    /// Create a new completion handler with the given backend and FIM tokens
    pub fn new(backend: Box<dyn Backend>, fim_tokens: idep_ai::completion::FimTokens) -> Self {
        Self::with_debounce(backend, fim_tokens, Duration::from_millis(300))
    }

    /// Create a handler with a custom debounce duration
    pub fn with_debounce(
        backend: Box<dyn Backend>,
        fim_tokens: idep_ai::completion::FimTokens,
        debounce: Duration,
    ) -> Self {
        let engine = CompletionEngine::new(backend, fim_tokens);
        Self {
            engine: Arc::new(engine),
            debounce,
        }
    }

    /// Handle a textDocument/completion request
    /// Returns LSP CompletionItem with the generated text
    pub async fn handle(
        &self,
        prefix: String,
        suffix: String,
        language: String,
        max_tokens: u32,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<Option<CompletionResponse>> {
        let req = CompletionRequest {
            prefix,
            suffix,
            language,
            max_tokens,
            stop_sequences,
        };

        // Debounce to avoid flooding the backend on rapid keypresses
        sleep(self.debounce).await;

        let resp = self.engine.complete(req).await?;

        // Convert to LSP CompletionItem
        let item = CompletionItem {
            label: resp.text.clone(),
            insert_text: Some(resp.text),
            kind: Some(lsp_types::CompletionItemKind::SNIPPET),
            ..Default::default()
        };

        Ok(Some(CompletionResponse::Array(vec![item])))
    }
}
