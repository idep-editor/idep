// idep-ai — AI layer for the Idep editor
//
// Three concerns, cleanly separated:
//   - backends  : HTTP clients for Anthropic, HuggingFace, Ollama, OpenAI-compat
//   - completion: inline LSP completion (FIM-aware, streaming)
//   - chat      : multi-turn conversation panel, context-aware
//   - indexer   : codebase RAG — tree-sitter chunking + embeddings

pub mod backends;
pub mod chat;
pub mod completion;
pub mod indexer;

/// Re-export the top-level types callers need
pub use backends::{Backend, BackendConfig};
pub use chat::{ChatMessage, ChatSession};
pub use completion::{CompletionRequest, CompletionResponse};
pub use indexer::Indexer;
