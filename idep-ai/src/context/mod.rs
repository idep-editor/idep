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

/// Cursor position in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

/// Context gathered from various sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Current file content (always included)
    pub current_file: Option<CurrentFileContext>,
    /// AST subtree around cursor
    pub ast_context: Option<AstContext>,
    /// Top-k similar chunks from vector index
    pub similar_chunks: Vec<SimilarChunk>,
    /// Recent edit history
    pub edit_history: Vec<EditHistoryItem>,
    /// Token usage information
    pub token_usage: TokenUsage,
}

/// Current file context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentFileContext {
    pub file_path: PathBuf,
    pub content: String,
    pub cursor_position: Position,
    /// Lines around cursor (context window)
    pub nearby_lines: Vec<String>,
    /// Language of the file
    pub language: String,
}

/// AST context around cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstContext {
    /// The AST node containing the cursor
    pub current_node: AstNode,
    /// Parent nodes (for context)
    pub parent_nodes: Vec<AstNode>,
    /// Child nodes (for detailed context)
    pub child_nodes: Vec<AstNode>,
}

/// AST node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub node_type: String,
    pub text: String,
    pub start_position: Position,
    pub end_position: Position,
    pub name: Option<String>,
}

/// Similar chunk from vector search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarChunk {
    pub chunk: CodeChunk,
    pub similarity_score: f32,
    pub relevance_rank: usize,
}

/// Edit history item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditHistoryItem {
    pub file_path: PathBuf,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub change_type: ChangeType,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Delete,
    Modify,
}

/// Token usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub current_file_tokens: usize,
    pub ast_context_tokens: usize,
    pub similar_chunks_tokens: usize,
    pub edit_history_tokens: usize,
}

/// Configuration for context gathering
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum context tokens (default 4096)
    pub max_tokens: usize,
    /// Number of similar chunks to retrieve (default 5)
    pub max_similar_chunks: usize,
    /// Number of lines around cursor to include (default 10)
    pub cursor_context_lines: usize,
    /// Number of recent edits to include (default 3)
    pub max_edit_history: usize,
    /// Priority order for truncation
    pub priority_order: Vec<ContextSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextSource {
    CurrentFile,
    AstContext,
    SimilarChunks,
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

/// Context engine that gathers and manages RAG context
pub struct ContextEngine {
    config: ContextConfig,
    // In a real implementation, these would be actual integrations
    // For now, we'll define the interface
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextEngine {
    /// Create a new context engine with default configuration
    pub fn new() -> Self {
        Self::with_config(ContextConfig::default())
    }

    /// Create a new context engine with custom configuration
    pub fn with_config(config: ContextConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Gather context for a query and cursor position
    pub fn gather(
        &self,
        _query: &str,
        _workspace_root: &Path,
        _cursor_file: &Path,
        _cursor_pos: Position,
    ) -> Result<Context> {
        // TODO: Implement context gathering
        // This method should:
        // 1. Load current file content around cursor
        // 2. Parse AST around cursor using Tree-sitter
        // 3. Query vector index for similar chunks
        // 4. Load recent edit history
        // 5. Apply token budget management

        Err(anyhow::anyhow!(
            "ContextEngine::gather() is not yet implemented"
        ))
    }

    /// Serialize context into a prompt-friendly text block
    pub fn serialize_context(&self, context: &Context) -> Result<String> {
        let mut output = String::new();

        // Add current file context
        if let Some(current_file) = &context.current_file {
            output.push_str("## Current File Context\n");
            output.push_str(&format!("File: {}\n", current_file.file_path.display()));
            output.push_str(&format!("Language: {}\n", current_file.language));
            output.push_str("Content around cursor:\n");
            for (i, line) in current_file.nearby_lines.iter().enumerate() {
                // Calculate line number: cursor_line (0-indexed) - offset + i + 1 (for 1-indexed display)
                let offset = current_file.nearby_lines.len() / 2;
                let line_num = current_file.cursor_position.line.saturating_sub(offset) + i + 1;
                output.push_str(&format!("{}: {}\n", line_num, line));
            }
            output.push('\n');
        }

        // Add AST context
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

        // Add similar chunks
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

        // Add edit history
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

        // Add token usage info
        output.push_str(&format!(
            "Context uses {}/{} tokens\n",
            context.token_usage.total_tokens, context.token_usage.max_tokens
        ));

        Ok(output)
    }

    /// Apply token budget management to fit context within limit
    pub fn apply_token_budget(&self, _context: &mut Context) -> Result<()> {
        // TODO: Implement token counting and truncation
        // This would:
        // 1. Count tokens in each section
        // 2. If over budget, truncate lower priority sections first
        // 3. Update token_usage with final counts

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
