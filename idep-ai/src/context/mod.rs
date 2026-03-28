// context — RAG context engine for codebase-aware chat
//
// Gathers relevant context from multiple sources:
//   1. Current file content and cursor context
//   2. AST subtree around cursor (Tree-sitter)
//   3. Top-k similar chunks from vector index
//   4. Recent edit history
//   5. Token budget management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::indexer::CodeChunk;

/// Cursor position in a file.
///
/// Both fields are **0-indexed**: line 0 is the first line of the file,
/// character 0 is the first column of a line. This matches the LSP
/// `Position` convention used throughout `idep-lsp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// 0-indexed line number from the start of the file.
    pub line: usize,
    /// 0-indexed character offset within the line (Unicode scalar values, not bytes).
    pub character: usize,
}

/// Context gathered from various sources for a single query.
///
/// Each field is `Option` or a `Vec` so that callers can build partial
/// contexts incrementally. The `token_usage` field is always present and
/// reflects the budget state at the time the context was serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Current file content and cursor surroundings (always included when available).
    pub current_file: Option<CurrentFileContext>,
    /// AST subtree around cursor, provided by Tree-sitter (v0.0.6+).
    pub ast_context: Option<AstContext>,
    /// Top-k similar chunks retrieved from the vector index (v0.0.8+).
    pub similar_chunks: Vec<SimilarChunk>,
    /// Recent edit history collected from the workspace file watcher (v0.1+).
    pub edit_history: Vec<EditHistoryItem>,
    /// Token budget breakdown for this context snapshot.
    pub token_usage: TokenUsage,
}

/// Context derived from the file that is currently open under the cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentFileContext {
    /// Absolute path to the file.
    pub file_path: PathBuf,
    /// Full file content at the time the context was gathered.
    pub content: String,
    /// Position of the editor cursor inside this file (0-indexed).
    pub cursor_position: Position,
    /// Lines centered on `cursor_position.line`, with a window of
    /// `ContextConfig::cursor_context_lines` lines total (half before,
    /// half after the cursor line). 0-indexed relative to the file.
    pub nearby_lines: Vec<String>,
    /// Detected language of the file (e.g. `"rust"`, `"python"`, `"typescript"`).
    pub language: String,
}

/// AST context extracted around the cursor by Tree-sitter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstContext {
    /// The innermost AST node that contains the cursor position.
    pub current_node: AstNode,
    /// Ancestor nodes from the current node up to the root (ordered nearest → root).
    pub parent_nodes: Vec<AstNode>,
    /// Direct children of the current node.
    pub child_nodes: Vec<AstNode>,
}

/// A single node from the Tree-sitter AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    /// Tree-sitter node kind string (e.g. `"function_item"`, `"struct_item"`).
    pub node_type: String,
    /// Source text spanned by this node.
    pub text: String,
    /// Start position of this node in the source file (0-indexed).
    pub start_position: Position,
    /// End position of this node in the source file (0-indexed).
    pub end_position: Position,
    /// Extracted name of the node, if applicable (e.g. function or struct name).
    pub name: Option<String>,
}

/// A code chunk returned by the vector index together with its relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarChunk {
    /// The source chunk with file path, content, and span information.
    pub chunk: CodeChunk,
    /// Cosine similarity score in the range `[0.0, 1.0]`; higher means more relevant.
    pub similarity_score: f32,
    /// 1-indexed rank among the top-k results (1 = best match).
    pub relevance_rank: usize,
}

/// A single entry in the recent-edit history collected from the workspace watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditHistoryItem {
    /// File that was modified.
    pub file_path: PathBuf,
    /// UTC timestamp of the change.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Kind of edit that occurred.
    pub change_type: ChangeType,
    /// Short human-readable description of the change (e.g. `"added fn parse_token"`).
    pub summary: String,
}

/// Kind of source-code edit recorded in the history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Delete,
    Modify,
}

/// Per-section token budget breakdown for a [`Context`] snapshot.
///
/// `total_tokens` equals the sum of all section counts. Sections are listed
/// in **descending priority** order; when the budget is exceeded the
/// lowest-priority sections are truncated first:
///
/// 1. `current_file_tokens` — highest priority, truncated last
/// 2. `ast_context_tokens`
/// 3. `similar_chunks_tokens`
/// 4. `edit_history_tokens` — lowest priority, truncated first
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Total tokens consumed across all sections (sum of the section fields below).
    pub total_tokens: usize,
    /// Hard token limit sourced from [`ContextConfig::max_tokens`].
    pub max_tokens: usize,
    /// Tokens used by the current-file section (priority 1 — truncated last).
    pub current_file_tokens: usize,
    /// Tokens used by the AST context section (priority 2).
    pub ast_context_tokens: usize,
    /// Tokens used by the similar-chunks section (priority 3).
    pub similar_chunks_tokens: usize,
    /// Tokens used by the edit-history section (priority 4 — truncated first).
    pub edit_history_tokens: usize,
}

/// Configuration for context gathering and token budget management.
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum number of tokens allowed across all context sections (default: 4096).
    pub max_tokens: usize,
    /// Number of similar chunks to retrieve from the vector index (default: 5).
    pub max_similar_chunks: usize,
    /// Total number of lines to include around the cursor (default: 10).
    /// Half of this value is taken before the cursor line and half after.
    pub cursor_context_lines: usize,
    /// Maximum number of recent edit-history entries to include (default: 3).
    pub max_edit_history: usize,
    /// Truncation priority: sections are dropped from the end of this list first.
    pub priority_order: Vec<ContextSource>,
}

/// A named context section, used to express truncation priority order.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextSource {
    /// Content of the file currently open in the editor (highest priority).
    CurrentFile,
    /// AST subtree around the cursor (Tree-sitter, v0.0.6+).
    AstContext,
    /// Top-k semantically similar chunks from the vector index (v0.0.8+).
    SimilarChunks,
    /// Recent edits tracked by the workspace file watcher (v0.1+; lowest priority).
    EditHistory,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            max_similar_chunks: 5,
            cursor_context_lines: 10,
            max_edit_history: 3,
            priority_order: vec![
                ContextSource::CurrentFile,
                ContextSource::AstContext,
                ContextSource::SimilarChunks,
                ContextSource::EditHistory,
            ],
        }
    }
}

/// Context engine that gathers and manages RAG context for codebase-aware chat.
pub struct ContextEngine {
    config: ContextConfig,
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextEngine {
    /// Create a new context engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(ContextConfig::default())
    }

    /// Create a new context engine with custom configuration.
    pub fn with_config(config: ContextConfig) -> Self {
        Self { config }
    }

    /// Return the active configuration.
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Gather context for a query at the given cursor position.
    ///
    /// Collects context from multiple sources with priority ordering:
    ///
    /// 1. **Current file** — content and lines around `cursor_pos` (always included)
    /// 2. **AST subtree** — Tree-sitter node enclosing the cursor (v0.0.6)
    /// 3. **Similar chunks** — top-k results from the vector index (v0.0.8, `idep-index`)
    /// 4. **Edit history** — recent file saves from the workspace watcher (v0.1+)
    ///
    /// After collection, [`apply_token_budget`] is called to ensure the total
    /// token count stays within [`ContextConfig::max_tokens`].
    ///
    /// # Status
    ///
    /// **Not yet implemented.** Full implementation awaits:
    /// - v0.0.6: Tree-sitter AST chunking integration
    /// - v0.0.8: `idep-index` vector store integration
    pub fn gather(
        &self,
        _query: &str,
        _workspace_root: &Path,
        _cursor_file: &Path,
        _cursor_pos: Position,
    ) -> Result<Context> {
        Err(anyhow::anyhow!(
            "ContextEngine::gather() is not yet implemented — \
             awaits Tree-sitter (v0.0.6) and idep-index vector store (v0.0.8)"
        ))
    }

    /// Serialize context into a prompt-friendly text block suitable for
    /// prepending to a chat message.
    ///
    /// Sections are emitted in priority order:
    /// 1. Current file content around cursor
    /// 2. AST node enclosing the cursor
    /// 3. Relevant code chunks from the vector index
    /// 4. Recent edit history
    ///
    /// A token-usage summary line is always appended at the end.
    pub fn serialize_context(&self, context: &Context) -> Result<String> {
        tracing::debug!(
            has_current_file = context.current_file.is_some(),
            has_ast = context.ast_context.is_some(),
            similar_chunks = context.similar_chunks.len(),
            edit_history = context.edit_history.len(),
            tokens_used = context.token_usage.total_tokens,
            tokens_max = context.token_usage.max_tokens,
            "Serializing context for prompt injection",
        );

        let mut output = String::new();

        // --- Current file ---
        if let Some(current_file) = &context.current_file {
            output.push_str("## Current File Context\n");
            output.push_str(&format!("File: {}\n", current_file.file_path.display()));
            output.push_str(&format!("Language: {}\n", current_file.language));
            output.push_str("Content around cursor:\n");

            // Calculate 1-indexed display line numbers for the context window.
            // nearby_lines is centered on cursor_position.line; offset gives the
            // distance from the cursor to the first line in the window.
            let offset = current_file.nearby_lines.len() / 2;
            for (i, line) in current_file.nearby_lines.iter().enumerate() {
                let line_num = current_file.cursor_position.line.saturating_sub(offset) + i + 1;
                output.push_str(&format!("{}: {}\n", line_num, line));
            }
            output.push('\n');
        }

        // --- AST context ---
        if let Some(ast_context) = &context.ast_context {
            output.push_str("## Code Structure Context\n");
            output.push_str(&format!(
                "Current node: {}\n",
                ast_context.current_node.node_type
            ));
            if let Some(name) = &ast_context.current_node.name {
                output.push_str(&format!("Name: {}\n", name));
            }
            output.push_str(&format!("Content:\n{}\n\n", ast_context.current_node.text));
        }

        // --- Similar chunks ---
        if !context.similar_chunks.is_empty() {
            output.push_str("## Relevant Code Chunks\n");
            for (i, chunk) in context.similar_chunks.iter().enumerate() {
                output.push_str(&format!(
                    "{}. {} ({}:{})\n",
                    i + 1,
                    chunk.chunk.name.as_deref().unwrap_or("unnamed"),
                    chunk.chunk.file.display(),
                    chunk.chunk.start_line
                ));
                output.push_str("```\n");
                output.push_str(&chunk.chunk.content);
                output.push_str("\n```\n\n");
            }
        }

        // --- Edit history ---
        if !context.edit_history.is_empty() {
            output.push_str("## Recent Changes\n");
            for edit in &context.edit_history {
                output.push_str(&format!(
                    "{} - {}: {}\n",
                    edit.timestamp.format("%Y-%m-%d %H:%M"),
                    edit.file_path.display(),
                    edit.summary
                ));
            }
            output.push('\n');
        }

        // --- Token usage summary ---
        output.push_str(&format!(
            "Context uses {}/{} tokens\n",
            context.token_usage.total_tokens, context.token_usage.max_tokens
        ));

        Ok(output)
    }

    /// Trim context sections so that `context.token_usage.total_tokens` fits
    /// within [`ContextConfig::max_tokens`].
    ///
    /// Sections are dropped in reverse priority order (edit history first,
    /// current file last) as defined by [`ContextConfig::priority_order`].
    ///
    /// # Status
    ///
    /// **Stub — not yet implemented.** Token-budget enforcement for the
    /// native message format is currently handled by
    /// [`crate::chat::ChatSession::build_messages_with_window_management`].
    /// This method will be fully implemented alongside [`gather`] once the
    /// Tree-sitter and vector-index integrations land.
    pub fn apply_token_budget(&self, _context: &mut Context) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_engine_creation() {
        let engine = ContextEngine::new();
        assert_eq!(engine.config.max_tokens, 4096);
        assert_eq!(engine.config.max_similar_chunks, 5);
    }

    #[test]
    fn context_engine_custom_config() {
        let config = ContextConfig {
            max_tokens: 2048,
            max_similar_chunks: 3,
            ..Default::default()
        };
        let engine = ContextEngine::with_config(config);
        assert_eq!(engine.config.max_tokens, 2048);
        assert_eq!(engine.config.max_similar_chunks, 3);
    }

    #[test]
    fn serialize_empty_context() {
        let engine = ContextEngine::new();
        let context = Context {
            current_file: None,
            ast_context: None,
            similar_chunks: Vec::new(),
            edit_history: Vec::new(),
            token_usage: TokenUsage {
                total_tokens: 0,
                max_tokens: 4096,
                current_file_tokens: 0,
                ast_context_tokens: 0,
                similar_chunks_tokens: 0,
                edit_history_tokens: 0,
            },
        };

        let serialized = engine.serialize_context(&context).unwrap();
        assert!(serialized.contains("Context uses 0/4096 tokens"));
    }
}
