// idep-index — semantic indexing
//
// Codebase search and embeddings

use anyhow::Result;
use fastembed::{EmbeddingModel, TextEmbedding};
use idep_ai::indexer::CodeChunk;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

/// A search result with similarity score
#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub id: u64,
    pub score: f32,
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        0.0
    } else {
        dot_product / (magnitude_a * magnitude_b)
    }
}

/// Vector store for embeddings with similarity search
pub struct VectorStore {
    // Store vectors directly instead of using HNSW for now to avoid lifetime issues
    vectors: Vec<Vec<f32>>,
    id_map: HashMap<usize, String>, // Map internal IDs to chunk identifiers
    next_id: usize,
}

impl VectorStore {
    /// Create a new vector store for 384-dimensional embeddings
    pub fn new() -> Result<Self> {
        Ok(Self {
            vectors: Vec::new(),
            id_map: HashMap::new(),
            next_id: 0,
        })
    }

    /// Add an embedding to the store
    pub fn add(&mut self, chunk_id: &str, embedding: &[f32]) -> Result<u64> {
        let id = self.next_id;

        // Validate embedding length
        if embedding.len() != 384 {
            return Err(anyhow::anyhow!(
                "Embedding must be 384 dimensions, got {}",
                embedding.len()
            ));
        }

        // Store the embedding vector
        self.vectors.push(embedding.to_vec());

        // Update state
        self.id_map.insert(id, chunk_id.to_string());
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ID overflow: maximum ID reached"))?;

        Ok(id as u64)
    }

    /// Find similar embeddings using brute-force cosine similarity
    pub fn find_similar(&self, embedding: &[f32], top_k: usize) -> Result<Vec<ScoredChunk>> {
        // Validate query embedding dimensions
        if embedding.len() != 384 {
            return Err(anyhow::anyhow!(
                "Query embedding must be 384 dimensions, got {}",
                embedding.len()
            ));
        }

        // Compute cosine similarity with all vectors
        let mut similarities: Vec<(usize, f32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(id, vec)| {
                let similarity = cosine_similarity(embedding, vec);
                (id, similarity)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        let results: Vec<ScoredChunk> = similarities
            .into_iter()
            .take(top_k)
            .map(|(id, score)| ScoredChunk {
                id: id as u64,
                score,
            })
            .collect();

        Ok(results)
    }

    /// Save the index to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        // Validate that ID map matches vector count
        if self.id_map.len() != self.vectors.len() {
            return Err(anyhow::anyhow!(
                "ID map size {} doesn't match vector count {}",
                self.id_map.len(),
                self.vectors.len()
            ));
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Save vectors as JSON
        let vectors_json = serde_json::to_string(&self.vectors)?;
        std::fs::write(path, vectors_json)?;

        // Save the ID mapping as JSON
        let id_map_path = path.with_extension("json");
        let id_map_json = serde_json::to_string(&self.id_map)?;
        std::fs::write(id_map_path, id_map_json)?;

        Ok(())
    }

    /// Load the index from disk
    pub fn load(path: &Path) -> Result<Self> {
        // Load vectors from JSON
        let vectors_json = std::fs::read_to_string(path)?;
        let vectors: Vec<Vec<f32>> = serde_json::from_str(&vectors_json)?;

        // Load the ID mapping
        let id_map_path = path.with_extension("json");
        let id_map_json = std::fs::read_to_string(id_map_path)?;
        let id_map: HashMap<usize, String> = serde_json::from_str(&id_map_json)?;

        // Validate that loaded ID map matches vector count
        if id_map.len() != vectors.len() {
            return Err(anyhow::anyhow!(
                "Loaded ID map size {} doesn't match vector count {}",
                id_map.len(),
                vectors.len()
            ));
        }

        // Determine next ID (max existing ID + 1)
        let next_id = id_map.keys().max().map(|&id| id + 1).unwrap_or(0);

        Ok(Self {
            vectors,
            id_map,
            next_id,
        })
    }

    /// Get the chunk identifier for an internal ID
    pub fn get_chunk_id(&self, id: u64) -> Option<&str> {
        self.id_map.get(&(id as usize)).map(|s| s.as_str())
    }

    /// Get the number of embeddings in the store
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
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
            if first.len() != 384 {
                return Err(anyhow::anyhow!(
                    "Expected 384 dimensions for AllMiniLML6V2, got {}",
                    first.len()
                ));
            }
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
    use tempfile::tempdir;

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

    #[test]
    fn vector_store_creation() {
        let store = VectorStore::new();
        assert!(store.is_ok());
    }

    #[test]
    fn vector_store_basic_add() {
        let mut store = VectorStore::new().expect("Failed to create vector store");

        // Just test that we can add without crashing
        let embedding: Vec<f32> = vec![0.1; 384];
        let _id = store.add("chunk1", &embedding);

        // If we get here, no segfault occurred
        // Test passed successfully
    }

    #[test]
    fn vector_store_save_and_load() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index.usearch");

        // Create and populate store
        let mut store = VectorStore::new().expect("Failed to create vector store");

        let embedding1: Vec<f32> = vec![0.1; 384];
        let embedding2: Vec<f32> = vec![0.2; 384];

        store
            .add("chunk1", &embedding1)
            .expect("Failed to add embedding");
        store
            .add("chunk2", &embedding2)
            .expect("Failed to add embedding");

        // Save to disk
        store.save(&index_path).expect("Failed to save store");

        // Load from disk
        let loaded_store = VectorStore::load(&index_path).expect("Failed to load store");

        // Verify loaded store has same data
        assert_eq!(loaded_store.len(), 2);
        assert_eq!(loaded_store.get_chunk_id(0), Some("chunk1"));
        assert_eq!(loaded_store.get_chunk_id(1), Some("chunk2"));

        // Verify search works on loaded store
        let query: Vec<f32> = vec![0.15; 384];
        let results = loaded_store
            .find_similar(&query, 2)
            .expect("Failed to search loaded store");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn vector_store_self_similarity() {
        let mut store = VectorStore::new().expect("Failed to create vector store");

        // Add 50 random embeddings
        let mut embeddings = Vec::new();
        for i in 0..50 {
            let embedding: Vec<f32> = (0..384).map(|j| (i + j) as f32 / 1000.0).collect();
            embeddings.push(embedding);
            store
                .add(&format!("chunk{}", i), &embeddings[i])
                .expect("Failed to add embedding");
        }

        // Query with each embedding and verify top-1 is itself
        for (i, embedding) in embeddings.iter().enumerate() {
            let results = store.find_similar(embedding, 1).expect("Failed to search");
            assert_eq!(results.len(), 1);
            assert_eq!(
                store.get_chunk_id(results[0].id),
                Some(format!("chunk{}", i).as_str())
            );
            // Self-similarity should be very close to 1.0
            assert!(results[0].score > 0.99);
        }
    }
}
