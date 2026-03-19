// indexer — codebase-aware RAG engine
//
// Pipeline:
//   1. Walk project files
//   2. Chunk via tree-sitter (AST-aware, not naive line splits)
//   3. Embed chunks (fastembed-rs, local, no network)
//   4. Store in usearch (in-process vector index)
//   5. Query: given cursor context, retrieve top-K relevant chunks

use anyhow::Result;
use std::path::{Path, PathBuf};

mod ast;
use ast::{AstChunker, Chunk};

/// A chunk of source code with its provenance
#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub file: PathBuf,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: ChunkKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ChunkKind {
    Function,
    Struct,
    Impl,
    Trait,
    Module,
    Other,
}

/// Query result from the index
#[derive(Debug)]
pub struct IndexResult {
    pub chunk: CodeChunk,
    pub score: f32,
}

/// The codebase indexer
///
/// NOTE: Embedding and vector store integration (fastembed-rs + usearch)
/// will be added in Phase 2. This skeleton defines the interface.
pub struct Indexer {
    root: PathBuf,
    chunks: Vec<CodeChunk>,
}

impl Indexer {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            chunks: Vec::new(),
        }
    }

    /// Walk the project and index all source files
    pub async fn index(&mut self) -> Result<usize> {
        self.chunks.clear();
        let chunks = walk_and_chunk(&self.root)?;
        let count = chunks.len();
        self.chunks = chunks;
        tracing::info!("Indexed {} chunks from {}", count, self.root.display());
        Ok(count)
    }

    /// Re-index a single file (called on save)
    pub async fn reindex_file(&mut self, path: &Path) -> Result<()> {
        self.chunks.retain(|c| c.file != path);
        let new_chunks = chunk_file(path)?;
        self.chunks.extend(new_chunks);
        Ok(())
    }

    /// Retrieve top-K chunks relevant to the query string
    ///
    /// Phase 1: naive keyword matching
    /// Phase 2: replace with vector similarity (fastembed-rs + usearch)
    pub fn query(&self, query: &str, top_k: usize) -> Vec<&CodeChunk> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(&CodeChunk, usize)> = self
            .chunks
            .iter()
            .map(|c| {
                let score = c
                    .content
                    .to_lowercase()
                    .split_whitespace()
                    .filter(|w| query_lower.contains(w))
                    .count();
                (c, score)
            })
            .filter(|(_, s)| *s > 0)
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().take(top_k).map(|(c, _)| c).collect()
    }

    /// Format top-K results as a context string for the chat session
    pub fn context_for(&self, query: &str, top_k: usize) -> String {
        let results = self.query(query, top_k);
        if results.is_empty() {
            return String::new();
        }

        results
            .iter()
            .map(|c| {
                format!(
                    "// {} (lines {}–{})\n{}",
                    c.file.display(),
                    c.start_line,
                    c.end_line,
                    c.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Walk a directory and chunk all Rust/TS/Python source files
fn walk_and_chunk(root: &Path) -> Result<Vec<CodeChunk>> {
    let mut chunks = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if is_source_file(path) {
            if let Ok(file_chunks) = chunk_file(path) {
                chunks.extend(file_chunks);
            }
        }
    }
    Ok(chunks)
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "toml" | "md")
    )
}

fn chunk_file(path: &Path) -> Result<Vec<CodeChunk>> {
    let content = std::fs::read_to_string(path)?;

    // Try AST-aware chunking; if unsupported language or parse fails, fall back to naive.
    let ast_chunks = AstChunker::new()
        .chunk(path, &content)
        .unwrap_or_else(|_| Vec::new());
    let chunks = if ast_chunks.is_empty() {
        naive_chunk(path, &content)
    } else {
        ast_chunks
            .into_iter()
            .map(|c| code_chunk_from_ast(path, &content, &c))
            .collect()
    };

    Ok(chunks)
}

/// Naive line-based chunking (fallback)
fn naive_chunk(path: &Path, content: &str) -> Vec<CodeChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let chunk_size = 40;
    let overlap = 5;
    let mut chunks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let end = (i + chunk_size).min(lines.len());
        chunks.push(CodeChunk {
            file: path.to_path_buf(),
            content: lines[i..end].join("\n"),
            start_line: i + 1,
            end_line: end,
            kind: ChunkKind::Other,
            name: None,
        });
        if end == lines.len() {
            break;
        }
        i += chunk_size - overlap;
    }

    chunks
}

fn code_chunk_from_ast(path: &Path, source: &str, chunk: &Chunk) -> CodeChunk {
    let line_offsets = line_offsets(source);
    let start_line = byte_to_line(chunk.start_byte, &line_offsets);
    let end_line = byte_to_line(chunk.end_byte, &line_offsets);
    let kind = match chunk.kind.as_str() {
        "function" => ChunkKind::Function,
        "struct" => ChunkKind::Struct,
        "impl" => ChunkKind::Impl,
        "trait" => ChunkKind::Trait,
        _ => ChunkKind::Other,
    };

    CodeChunk {
        file: path.to_path_buf(),
        content: chunk.text.clone(),
        start_line,
        end_line,
        kind,
        name: chunk.name.clone(),
    }
}

fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut acc = 0;
    offsets.push(0);
    for line in source.lines() {
        acc += line.len() + 1; // include newline
        offsets.push(acc);
    }
    offsets
}

fn byte_to_line(byte: usize, offsets: &[usize]) -> usize {
    match offsets.binary_search(&byte) {
        Ok(idx) => idx + 1,
        Err(idx) => idx, // idx is insertion point; line numbers are 1-based
    }
}
