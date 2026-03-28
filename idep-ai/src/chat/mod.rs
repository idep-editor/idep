// chat — multi-turn conversation with codebase context

use crate::backends::{ollama::OllamaBackend, Backend};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::debug;

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

    /// Send a user message and return the full response string.
    ///
    /// Delegates to [`send_streaming`] with a no-op token callback.
    pub async fn send(&mut self, message: &str) -> Result<String> {
        self.send_streaming(message, |_| {}).await
    }

    /// Send a user message and stream tokens to a callback if the backend supports it.
    ///
    /// Implements proper debounce: if a new call arrives within the debounce
    /// window the previous pending request is cancelled and an empty string is
    /// returned for that call.
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

    /// Send a message with a RAG context block prepended to the prompt.
    ///
    /// Delegates to [`send_streaming_with_context`] with a no-op token callback.
    pub async fn send_with_context(&mut self, message: &str, context: &str) -> Result<String> {
        self.send_streaming_with_context(message, context, |_| {})
            .await
    }

    /// Send a message with context and stream individual tokens to a callback.
    ///
    /// The `context` string is prepended to the prompt so that the model sees
    /// relevant codebase information before the user question. History is
    /// updated only after the debounce window passes — cancelled calls do not
    /// mutate history.
    pub async fn send_streaming_with_context<F>(
        &mut self,
        message: &str,
        context: &str,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str) + Send,
    {
        // Build prompt with context prepended as a clearly labelled block.
        let prompt_with_context = self.build_prompt_with_context(message, context);

        debug!(
            context_len = context.len(),
            history_len = self.history.len(),
            "Sending message with RAG context",
        );

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

        // Add user message to history only after debounce passes
        self.history.push(ChatMessage::user(message));

        // Try Ollama streaming first (for now, Anthropic uses the regular complete method)
        let result = if let Some(ollama) = self.backend.as_any().downcast_ref::<OllamaBackend>() {
            let result = ollama
                .stream_completion(&prompt_with_context, 2048, |tok| on_token(tok))
                .await?;
            result
        } else {
            // Fallback: non-streaming backends, emit once at end
            let response = self.backend.complete(&prompt_with_context, 2048).await?;
            on_token(&response);
            response
        };

        // Add assistant response to history
        self.history.push(ChatMessage::assistant(&result));
        Ok(result)
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

    /// Build a native message array with context embedded as the first user turn.
    ///
    /// **Legacy format.** This encodes the system prompt + context as a
    /// simulated user/assistant exchange at the start of the array, which is
    /// the workaround required by backends that do not accept a dedicated
    /// `system` parameter. For the Anthropic Messages API (claude-3+) prefer
    /// [`build_anthropic_request`], which places system content in the
    /// top-level `system` field.
    pub fn build_messages_with_context(
        &self,
        message: &str,
        context: &str,
    ) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();

        // Add system message
        if !self.system.is_empty() {
            messages.push(json!({
                "role": "user",
                "content": format!("System: {}\n\n{}",
                    if !context.is_empty() {
                        format!("Context:\n{}\n\n", context)
                    } else {
                        String::new()
                    },
                    self.system
                )
            }));
            messages.push(json!({
                "role": "assistant",
                "content": "Understood. I'll use this context and system information."
            }));
        } else if !context.is_empty() {
            // If no system message but have context, add it as first user message
            messages.push(json!({
                "role": "user",
                "content": format!("Context:\n{}", context)
            }));
            messages.push(json!({
                "role": "assistant",
                "content": "I understand the context. How can I help you?"
            }));
        }

        // Add conversation history
        for msg in &self.history {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => continue, // Skip system messages in native format
            };
            messages.push(json!({
                "role": role,
                "content": msg.content
            }));
        }

        // Add current message
        messages.push(json!({
            "role": "user",
            "content": message
        }));

        messages
    }

    /// Build native message array without context
    pub fn build_messages(&self) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();

        // Add system message
        if !self.system.is_empty() {
            messages.push(json!({
                "role": "user",
                "content": format!("System: {}", self.system)
            }));
            messages.push(json!({
                "role": "assistant",
                "content": "Understood. I'll follow these instructions."
            }));
        }

        // Add conversation history
        for msg in &self.history {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => continue, // Skip system messages in native format
            };
            messages.push(json!({
                "role": role,
                "content": msg.content
            }));
        }

        messages
    }

    /// Build native message array with context window management.
    ///
    /// Calls [`build_messages_with_context`] then enforces `max_tokens` by
    /// removing older history messages via [`truncate_messages_to_budget`].
    /// The system/context preamble and the most-recent messages are always
    /// retained; only middle-history messages are dropped.
    pub fn build_messages_with_window_management(
        &self,
        message: &str,
        context: &str,
        max_tokens: usize,
    ) -> Vec<serde_json::Value> {
        let messages = self.build_messages_with_context(message, context);

        // Count how many leading messages are pinned (system preamble + context preamble).
        let system_msg_count = if !self.system.is_empty() { 2 } else { 0 };
        let context_msg_count = if !context.is_empty() { 2 } else { 0 };
        let pinned = system_msg_count + context_msg_count;

        truncate_messages_to_budget(messages, pinned, max_tokens)
    }

    /// Build a proper Anthropic Messages API request payload.
    ///
    /// Returns `(system_prompt, messages)` where:
    /// - `system_prompt` is the combined system instruction and RAG context,
    ///   ready to be placed in the top-level `"system"` field of the request body.
    /// - `messages` contains only the user/assistant conversation history and
    ///   the current user message, with **no** system preamble embedded as a
    ///   fake user turn.
    ///
    /// This matches the [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
    /// which accepts `system` as a separate top-level string rather than as an
    /// ordinary message turn.  Use this instead of [`build_messages_with_context`]
    /// when calling `AnthropicBackend` directly.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let (system, messages) = session.build_anthropic_request("My question", &context);
    /// let body = serde_json::json!({
    ///     "model": "claude-opus-4-5",
    ///     "system": system,
    ///     "messages": messages,
    ///     "max_tokens": 2048,
    /// });
    /// ```
    pub fn build_anthropic_request(
        &self,
        message: &str,
        context: &str,
    ) -> (String, Vec<serde_json::Value>) {
        // Combine the base system instruction with the RAG context block.
        // Context is placed after the system prompt so the model sees behavioural
        // instructions before codebase information.
        let system_prompt = if context.is_empty() {
            self.system.clone()
        } else {
            format!("{}\n\n## Codebase Context\n\n{}", self.system, context)
        };

        // Build the messages array from conversation history + current message.
        // System content lives in system_prompt above — never embed it here.
        let mut messages: Vec<serde_json::Value> = self
            .history
            .iter()
            .filter_map(|msg| {
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => return None, // already in system_prompt
                };
                Some(json!({ "role": role, "content": msg.content }))
            })
            .collect();

        messages.push(json!({ "role": "user", "content": message }));

        (system_prompt, messages)
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

/// Trim a message list so that the estimated token count stays within `max_tokens`.
///
/// The first `pinned` messages (system preamble, context preamble) are always
/// kept intact. Older history messages beyond that boundary are dropped from
/// the *oldest first* until the total fits within the budget. The most-recent
/// messages are retained where possible.
///
/// # Token estimation
///
/// Uses a rough approximation of **4 characters ≈ 1 token**, which is a
/// reasonable heuristic for English prose and code. For production use,
/// replace `estimate_tokens` with a proper tokenizer such as `tiktoken-rs`.
fn truncate_messages_to_budget(
    messages: Vec<serde_json::Value>,
    pinned: usize,
    max_tokens: usize,
) -> Vec<serde_json::Value> {
    // Rough token estimation: ~4 chars per token.
    // Replace with a proper tokenizer (e.g. tiktoken-rs) for production use.
    let estimate_tokens = |text: &str| text.len() / 4;

    let total_tokens: usize = messages
        .iter()
        .map(|m| m.get("content").and_then(|v| v.as_str()).unwrap_or(""))
        .map(estimate_tokens)
        .sum();

    if total_tokens <= max_tokens {
        return messages;
    }

    let mut kept: Vec<serde_json::Value> = Vec::new();
    let mut used_tokens = 0;

    // Always retain the pinned preamble messages (system + context).
    for msg in messages.iter().take(pinned) {
        kept.push(msg.clone());
        if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
            used_tokens += estimate_tokens(content);
        }
    }

    // Walk the remaining messages from newest to oldest, keeping those that fit.
    for msg in messages.iter().skip(pinned).rev() {
        let msg_tokens = msg
            .get("content")
            .and_then(|v| v.as_str())
            .map(estimate_tokens)
            .unwrap_or(0);

        if used_tokens + msg_tokens <= max_tokens {
            // Insert after the pinned block so order is preserved.
            kept.insert(pinned, msg.clone());
            used_tokens += msg_tokens;
        }
    }

    kept
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

#[cfg(test)]
mod integration_test;

#[cfg(test)]
mod native_format_test;
