// chat — multi-turn conversation with codebase context

use crate::backends::{ollama::OllamaBackend, Backend};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// A stateful chat session
pub struct ChatSession {
    backend: Box<dyn Backend>,
    history: Vec<ChatMessage>,
    system: String,
    debounce: Duration,
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

impl ChatSession {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self::with_debounce(backend, Duration::from_millis(300))
    }

    pub fn with_debounce(backend: Box<dyn Backend>, debounce: Duration) -> Self {
        Self {
            backend,
            history: Vec::new(),
            system: "You are Idep, an intelligent coding assistant. \
                     You have deep knowledge of the current codebase. \
                     Respond concisely. Prefer code over prose."
                .into(),
            debounce,
            cancel_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Inject codebase context (called by the indexer)
    pub fn set_context(&mut self, context: &str) {
        self.system = format!(
            "You are Idep, an intelligent coding assistant.\n\
             Codebase context:\n{context}\n\n\
             Respond concisely. Prefer code over prose."
        );
    }

    /// Send a user message and return the response
    /// Delegates to send_streaming with a no-op callback to avoid code duplication
    pub async fn send(&mut self, message: &str) -> Result<String> {
        self.send_streaming(message, |_| {}).await
    }

    /// Send a user message and stream tokens to a callback if the backend supports it
    /// Implements proper debounce: cancels pending requests when new ones arrive
    pub async fn send_streaming<F>(&mut self, message: &str, mut on_token: F) -> Result<String>
    where
        F: FnMut(&str) + Send,
    {
        self.history.push(ChatMessage::user(message));

        let prompt = self.build_prompt();

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
                return Ok(String::new());
            }
        }

        // Try Ollama streaming first
        if let Some(ollama) = self.backend.as_any().downcast_ref::<OllamaBackend>() {
            let result = ollama
                .stream_completion(&prompt, 2048, |tok| on_token(tok))
                .await?;
            self.history.push(ChatMessage::assistant(&result));
            return Ok(result);
        }

        // Fallback: non-streaming backends, emit once at end
        let response = self.backend.complete(&prompt, 2048).await?;
        on_token(&response);
        self.history.push(ChatMessage::assistant(&response));
        Ok(response)
    }

    /// Clear conversation but keep system prompt
    pub fn reset(&mut self) {
        self.history.clear();
    }

    /// Send a message with context block prepended
    pub async fn send_with_context(&mut self, message: &str, context: &str) -> Result<String> {
        self.send_streaming_with_context(message, context, |_| {})
            .await
    }

    /// Send a message with context and stream tokens to callback
    pub async fn send_streaming_with_context<F>(
        &mut self,
        message: &str,
        context: &str,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str) + Send,
    {
        // Build prompt with context
        let prompt_with_context = self.build_prompt_with_context(message, context);

        self.history.push(ChatMessage::user(message));

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
                return Ok(String::new());
            }
        }

        // Try Ollama streaming first
        if let Some(ollama) = self.backend.as_any().downcast_ref::<OllamaBackend>() {
            let result = ollama
                .stream_completion(&prompt_with_context, 2048, |tok| on_token(tok))
                .await?;
            self.history.push(ChatMessage::assistant(&result));
            Ok(result)
        } else {
            // Fallback: non-streaming backends, emit once at end
            let response = self.backend.complete(&prompt_with_context, 2048).await?;
            on_token(&response);
            self.history.push(ChatMessage::assistant(&response));
            Ok(response)
        }
    }

    /// Export conversation history to JSON
    pub fn export(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.history)?)
    }

    /// Build a prompt string with context block
    fn build_prompt_with_context(&self, message: &str, context: &str) -> String {
        let mut prompt = format!("System: {}\n\n", self.system);

        if !context.is_empty() {
            prompt.push_str("Context:\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }

        for msg in &self.history {
            let role = match msg.role {
                Role::User => "Human",
                Role::Assistant => "Assistant",
                Role::System => "System",
            };
            prompt.push_str(&format!("{}: {}\n", role, msg.content));
        }
        prompt.push_str(&format!("Human: {}\nAssistant: ", message));
        prompt
    }

    /// Build a simple prompt string from history
    /// (backends that support native message arrays would override this)
    fn build_prompt(&self) -> String {
        let mut prompt = format!("System: {}\n\n", self.system);
        for msg in &self.history {
            let role = match msg.role {
                Role::User => "Human",
                Role::Assistant => "Assistant",
                Role::System => "System",
            };
            prompt.push_str(&format!("{}: {}\n", role, msg.content));
        }
        prompt.push_str("Assistant: ");
        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::mock::MockBackend;

    #[test]
    fn context_injection() {
        let backend = Box::new(MockBackend::new());
        let session = ChatSession::new(backend);

        let context =
            "Current file: main.rs\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";

        // This would test that context is properly injected
        // For now, we'll test the prompt building
        let prompt = session.build_prompt_with_context("What does this do?", context);

        assert!(prompt.contains("Context:"));
        assert!(prompt.contains("Current file: main.rs"));
        assert!(prompt.contains("What does this do?"));
        assert!(prompt.contains("Human: What does this do?"));
        assert!(prompt.contains("Assistant: "));
    }

    #[test]
    fn export_history() {
        let backend = Box::new(MockBackend::new());
        let mut session = ChatSession::new(backend);

        session.history.push(ChatMessage::user("Hello"));
        session.history.push(ChatMessage::assistant("Hi there!"));

        let exported = session.export().unwrap();
        assert!(exported.contains("Hello"));
        assert!(exported.contains("Hi there!"));
    }
}
