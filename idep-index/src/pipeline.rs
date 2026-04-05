// pipeline.rs — Embedding pipeline for batch processing

use anyhow::Result;
use idep_ai::indexer::CodeChunk;

use crate::embedder::{EmbeddedChunk, Embedder};

pub struct EmbedPipeline {
    pub embedder: Embedder,
    batch_size: usize,
}

impl EmbedPipeline {
    /// Create a new pipeline with default batch size
    pub fn new() -> Result<Self> {
        Ok(Self {
            embedder: Embedder::new()?,
            batch_size: 32,
        })
    }

    /// Create a new pipeline with custom batch size
    pub fn with_batch_size(batch_size: usize) -> Result<Self> {
        Ok(Self {
            embedder: Embedder::new()?,
            batch_size,
        })
    }

    /// Process chunks into embedded chunks
    pub fn run(&mut self, chunks: Vec<CodeChunk>) -> Result<Vec<EmbeddedChunk>> {
        let mut embedded_chunks = Vec::with_capacity(chunks.len());
        let total = chunks.len();

        for batch_start in (0..chunks.len()).step_by(self.batch_size) {
            let batch_end = (batch_start + self.batch_size).min(chunks.len());
            let batch = &chunks[batch_start..batch_end];

            let texts: Vec<&str> = batch.iter().map(|chunk| chunk.content.as_str()).collect();
            let embeddings = self.embedder.embed_batch(&texts)?;

            for (chunk, embedding) in batch.iter().zip(embeddings) {
                embedded_chunks.push(EmbeddedChunk {
                    chunk: chunk.clone(),
                    embedding,
                });
            }

            let processed = batch_end.min(total);
            println!("Embedded {}/{} chunks", processed, total);
        }

        Ok(embedded_chunks)
    }

    /// Process chunks with custom progress callback
    pub fn run_with_progress<F>(
        &mut self,
        chunks: Vec<CodeChunk>,
        mut progress: F,
    ) -> Result<Vec<EmbeddedChunk>>
    where
        F: FnMut(usize, usize),
    {
        let mut embedded_chunks = Vec::with_capacity(chunks.len());
        let total = chunks.len();

        for batch_start in (0..chunks.len()).step_by(self.batch_size) {
            let batch_end = (batch_start + self.batch_size).min(chunks.len());
            let batch = &chunks[batch_start..batch_end];

            let texts: Vec<&str> = batch.iter().map(|chunk| chunk.content.as_str()).collect();
            let embeddings = self.embedder.embed_batch(&texts)?;

            for (chunk, embedding) in batch.iter().zip(embeddings) {
                embedded_chunks.push(EmbeddedChunk {
                    chunk: chunk.clone(),
                    embedding,
                });
            }

            let processed = batch_end.min(total);
            progress(processed, total);
        }

        Ok(embedded_chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use idep_ai::indexer::{ChunkKind, CodeChunk};
    use std::path::PathBuf;

    #[test]
    fn pipeline_produces_one_embedding_per_chunk() {
        let chunks = vec![
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "fn hello() { println!(\"Hello\"); }".to_string(),
                start_line: 1,
                end_line: 1,
                kind: ChunkKind::Function,
                name: Some("hello".to_string()),
            },
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "struct Test { field: i32 }".to_string(),
                start_line: 2,
                end_line: 2,
                kind: ChunkKind::Struct,
                name: Some("Test".to_string()),
            },
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "impl Test { fn new() -> Self { Self { field: 0 } } }".to_string(),
                start_line: 3,
                end_line: 3,
                kind: ChunkKind::Impl,
                name: Some("Test".to_string()),
            },
        ];

        let mut pipeline = EmbedPipeline::new().expect("Failed to create pipeline");
        let embedded_chunks = pipeline
            .run(chunks.clone())
            .expect("Failed to run pipeline");

        assert_eq!(embedded_chunks.len(), chunks.len());

        for embedded_chunk in &embedded_chunks {
            assert_eq!(embedded_chunk.embedding.len(), 384);
        }

        for (i, embedded_chunk) in embedded_chunks.iter().enumerate() {
            assert_eq!(embedded_chunk.chunk.content, chunks[i].content);
            assert_eq!(embedded_chunk.chunk.kind, chunks[i].kind);
            assert_eq!(embedded_chunk.chunk.name, chunks[i].name);
        }
    }

    #[test]
    fn pipeline_with_custom_batch_size() {
        let chunks = vec![
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "fn test1() {}".to_string(),
                start_line: 1,
                end_line: 1,
                kind: ChunkKind::Function,
                name: Some("test1".to_string()),
            },
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "fn test2() {}".to_string(),
                start_line: 2,
                end_line: 2,
                kind: ChunkKind::Function,
                name: Some("test2".to_string()),
            },
        ];

        let mut pipeline = EmbedPipeline::with_batch_size(1).expect("Failed to create pipeline");
        let embedded_chunks = pipeline.run(chunks).expect("Failed to run pipeline");

        assert_eq!(embedded_chunks.len(), 2);

        for embedded_chunk in &embedded_chunks {
            assert_eq!(embedded_chunk.embedding.len(), 384);
        }
    }

    #[test]
    fn pipeline_with_progress_callback() {
        let chunks = vec![
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "fn test() {}".to_string(),
                start_line: 1,
                end_line: 1,
                kind: ChunkKind::Function,
                name: Some("test".to_string()),
            },
            CodeChunk {
                file: PathBuf::from("test.rs"),
                content: "struct Test {}".to_string(),
                start_line: 2,
                end_line: 2,
                kind: ChunkKind::Struct,
                name: Some("Test".to_string()),
            },
        ];

        let mut progress_calls = Vec::new();
        let mut pipeline = EmbedPipeline::new().expect("Failed to create pipeline");
        let _embedded_chunks = pipeline
            .run_with_progress(chunks, |processed, total| {
                progress_calls.push((processed, total));
            })
            .expect("Failed to run pipeline");

        assert!(!progress_calls.is_empty());
        assert_eq!(progress_calls.last(), Some(&(2, 2)));
    }
}
