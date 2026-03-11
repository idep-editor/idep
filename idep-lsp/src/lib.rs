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

/// LSP completion handler — bridges textDocument/completion requests to CompletionEngine
pub struct CompletionHandler {
    engine: Arc<CompletionEngine>,
}

impl CompletionHandler {
    /// Create a new completion handler with the given backend and FIM tokens
    pub fn new(backend: Box<dyn Backend>, fim_tokens: idep_ai::completion::FimTokens) -> Self {
        let engine = CompletionEngine::new(backend, fim_tokens);
        Self {
            engine: Arc::new(engine),
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
    ) -> Result<Option<CompletionResponse>> {
        let req = CompletionRequest {
            prefix,
            suffix,
            language,
            max_tokens,
        };

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
