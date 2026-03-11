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
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

/// LSP completion handler — bridges textDocument/completion requests to CompletionEngine
pub struct CompletionHandler {
    engine: Arc<CompletionEngine>,
    debounce: Duration,
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
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
            cancel_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Handle a textDocument/completion request
    /// Returns LSP CompletionItem with the generated text
    /// Implements proper debounce: cancels pending requests when new ones arrive
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

        // Cancel any pending request from the previous call
        {
            let mut token_guard = self.cancel_token.lock().await;
            if let Some(token) = token_guard.take() {
                token.cancel();
            }
        }

        // Create a new cancellation token for this request
        let token = CancellationToken::new();
        let token_clone = token.clone();
        {
            let mut token_guard = self.cancel_token.lock().await;
            *token_guard = Some(token_clone);
        }

        // Wait for debounce duration or cancellation
        tokio::select! {
            _ = sleep(self.debounce) => {},
            _ = token.cancelled() => {
                // Request was cancelled by a newer one; return early
                return Ok(None);
            }
        }

        let resp = self.engine.complete(req).await?;

        // Truncate label to first line for LSP menu rendering
        let label = resp.text.lines().next().unwrap_or("").to_string();

        // Convert to LSP CompletionItem
        let item = CompletionItem {
            label,
            insert_text: Some(resp.text),
            kind: Some(lsp_types::CompletionItemKind::SNIPPET),
            ..Default::default()
        };

        Ok(Some(CompletionResponse::Array(vec![item])))
    }
}
