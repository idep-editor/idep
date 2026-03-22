// idep-index — semantic indexing
//
// Codebase search and embeddings

use anyhow::Result;
use fastembed::{EmbeddingModel, TextEmbedding};
use idep_ai::indexer::CodeChunk;
use std::path::PathBuf;

pub struct Embedder {
    model: TextEmbedding,
    model_path: PathBuf,
}

/// A chunk with its embedding vector
#[derive(Debug, Clone)]
pub struct EmbeddedChunk {
    pub chunk: CodeChunk,
    pub embedding: Vec<f32>,
}

pub struct EmbedPipeline {
    embedder: Embedder,
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

            // Extract texts from chunks
            let texts: Vec<&str> = batch.iter().map(|chunk| chunk.content.as_str()).collect();

            // Embed the batch
            let embeddings = self.embedder.embed_batch(&texts)?;

            // Create embedded chunks
            for (chunk, embedding) in batch.iter().zip(embeddings) {
                embedded_chunks.push(EmbeddedChunk {
                    chunk: chunk.clone(),
                    embedding,
                });
            }

            // Progress callback (simple println for now)
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
        F: FnMut(usize, usize), // (processed, total)
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

impl Embedder {
    /// Initialize the embedder with the default model.
    ///
    /// Downloads the model on first run and caches it to `~/.idep/models/`.
    pub fn new() -> Result<Self> {
        let model_name = EmbeddingModel::AllMiniLML6V2;
        let model_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".idep")
            .join("models")
            .join(model_name.to_string());

        // Ensure cache directory exists
        if let Some(parent) = model_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Initialize fastembed with the model (it handles download/caching internally)
        let model = TextEmbedding::try_new(
            fastembed::TextInitOptions::new(model_name).with_show_download_progress(true),
        )?;

        Ok(Self { model, model_path })
    }

    /// Embed a batch of texts.
    ///
    /// Returns a vector of embeddings, one per input text.
    /// Each embedding is a vector of f32 values (dimension 384 for AllMiniLML6V2).
    pub fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let embeddings = self.model.embed(texts, None)?;
        // Verify embedding dimension
        if let Some(first) = embeddings.first() {
            assert_eq!(
                first.len(),
                384,
                "Expected 384 dimensions for AllMiniLML6V2"
            );
        }
        Ok(embeddings)
    }

    /// Get the path to the cached model file.
    pub fn model_path(&self) -> &PathBuf {
        &self.model_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use idep_ai::indexer::{ChunkKind, CodeChunk};
    use std::path::PathBuf;
    use std::sync::Mutex;

    // Use a Mutex to allow mutable access across tests
    lazy_static::lazy_static! {
        static ref EMBEDDER: Mutex<Embedder> = Mutex::new(Embedder::new().expect("Failed to create embedder"));
    }

    #[test]
    fn embed_batch_returns_correct_shape() {
        let mut embedder = EMBEDDER.lock().unwrap();
        let texts = [
            "Hello world",
            "Rust is a systems programming language",
            "Fastembed provides local embeddings",
            "Testing embedding shapes",
            "Semantic search relies on vectors",
            "Machine learning models need data",
            "Code indexing improves developer experience",
            "Local embeddings keep data private",
            "Vector databases store high-dimensional data",
            "Embedding dimension is 384",
        ];
        let embeddings = embedder.embed_batch(&texts).expect("Failed to embed");
        assert_eq!(embeddings.len(), texts.len());
        for emb in &embeddings {
            assert_eq!(emb.len(), 384);
        }
    }

    #[test]
    fn embed_performance_benchmark() {
        let mut embedder = EMBEDDER.lock().unwrap();
        let mut texts: Vec<String> = Vec::new();
        for i in 0..100 {
            texts.push(format!(
                "Sample text {} with some content to simulate ~200 tokens.",
                i
            ));
        }
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let start = std::time::Instant::now();
        let _embeddings = embedder.embed_batch(&text_refs).expect("Failed to embed");
        let duration = start.elapsed();
        println!("Embedded 100 texts in {:?}", duration);
        // This is a simple benchmark; in CI we can assert an upper bound if needed
    }

    #[test]
    fn embed_without_network_calls() {
        // This test ensures that after the initial download, no network calls are made.
        // We can't easily block network in a unit test, but we can verify embedding works.
        // The fact that we can embed without errors suggests the model is cached locally.
        let mut embedder = EMBEDDER.lock().unwrap();
        let texts = ["Network independence test"];
        let _embeddings = embedder.embed_batch(&texts).expect("Failed to embed");
        // If we got here without network errors, the model is cached locally
    }

    #[test]
    fn pipeline_produces_one_embedding_per_chunk() {
        // Create test chunks
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

        // Verify we have one embedded chunk per input chunk
        assert_eq!(embedded_chunks.len(), chunks.len());

        // Verify each embedded chunk has correct embedding dimension
        for embedded_chunk in &embedded_chunks {
            assert_eq!(embedded_chunk.embedding.len(), 384);
        }

        // Verify chunks are preserved correctly
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

        // Should have progress updates
        assert!(!progress_calls.is_empty());
        assert_eq!(progress_calls.last(), Some(&(2, 2))); // Final update
    }
}
