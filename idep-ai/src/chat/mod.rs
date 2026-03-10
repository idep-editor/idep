// chat — multi-turn conversation with codebase context

use crate::backends::Backend;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role:    Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into() }
    }
}

/// A stateful chat session
pub struct ChatSession {
    backend:  Box<dyn Backend>,
    history:  Vec<ChatMessage>,
    system:   String,
}

impl ChatSession {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            history: Vec::new(),
            system: "You are Idep, an intelligent coding assistant. \
                     You have deep knowledge of the current codebase. \
                     Respond concisely. Prefer code over prose.".into(),
        }
    }

    /// Inject codebase context (called by the indexer)
    pub fn set_context(&mut self, context: &str) {
        self.system = format!(
            "You are Idep, an intelligent coding assistant.\n\
             Codebase context:\n{}\n\n\
             Respond concisely. Prefer code over prose.",
            context
        );
    }

    /// Send a user message, stream the response via callback
    pub async fn send(
        &mut self,
        message: &str,
        on_token: impl FnMut(String) + Send,
    ) -> Result<String> {
        self.history.push(ChatMessage::user(message));

        // Build prompt from full history
        let prompt = self.build_prompt();

        let mut full_response = String::new();
        let mut cb = on_token;
        self.backend.complete(&prompt, 2048, |token| {
            full_response.push_str(&token);
            cb(token);
        }).await?;

        self.history.push(ChatMessage::assistant(&full_response));
        Ok(full_response)
    }

    /// Clear conversation but keep system prompt
    pub fn reset(&mut self) {
        self.history.clear();
    }

    /// Build a simple prompt string from history
    /// (backends that support native message arrays would override this)
    fn build_prompt(&self) -> String {
        let mut prompt = format!("System: {}\n\n", self.system);
        for msg in &self.history {
            let role = match msg.role {
                Role::User      => "Human",
                Role::Assistant => "Assistant",
                Role::System    => "System",
            };
            prompt.push_str(&format!("{}: {}\n", role, msg.content));
        }
        prompt.push_str("Assistant: ");
        prompt
    }
}
