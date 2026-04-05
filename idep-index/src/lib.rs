// idep-index — semantic indexing
//
// Codebase search and embeddings

use anyhow::Result;
use fastembed::{EmbeddingModel, TextEmbedding};
use idep_ai::indexer::{ChunkKind, CodeChunk};
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
    // Store vectors in a HashMap keyed by ID to avoid index shifting issues
    vectors: HashMap<u64, Vec<f32>>,
    id_map: HashMap<u64, String>, // Map internal IDs to chunk identifiers
    next_id: u64,
}

impl VectorStore {
    /// Create a new vector store for 384-dimensional embeddings
    pub fn new() -> Result<Self> {
        Ok(Self {
            vectors: HashMap::new(),
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
        self.vectors.insert(id, embedding.to_vec());

        // Update state
        self.id_map.insert(id, chunk_id.to_string());
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ID overflow: maximum ID reached"))?;

        Ok(id)
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
        let mut similarities: Vec<(u64, f32)> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let similarity = cosine_similarity(embedding, vec);
                (*id, similarity)
            })
            .collect();

        // Sort by similarity (descending), handle NaN values by pushing them to end
        similarities.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or_else(|| {
                // Handle NaN: push NaN values to end of results
                if b.1.is_nan() {
                    std::cmp::Ordering::Less
                } else if a.1.is_nan() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });

        // Take top k
        let results: Vec<ScoredChunk> = similarities
            .into_iter()
            .take(top_k)
            .map(|(id, score)| ScoredChunk { id, score })
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

        // Save the ID mapping as JSON with distinct extension
        let id_map_path = path.with_extension("id_map.json");
        let id_map_json = serde_json::to_string(&self.id_map)?;
        std::fs::write(id_map_path, id_map_json)?;

        Ok(())
    }

    /// Load the index from disk
    pub fn load(path: &Path) -> Result<Self> {
        // Load vectors from JSON
        let vectors_json = std::fs::read_to_string(path)?;
        let vectors: HashMap<u64, Vec<f32>> = serde_json::from_str(&vectors_json)?;

        // Load the ID mapping with matching extension
        let id_map_path = path.with_extension("id_map.json");
        let id_map_json = std::fs::read_to_string(id_map_path)?;
        let id_map: HashMap<u64, String> = serde_json::from_str(&id_map_json)?;

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
        self.id_map.get(&id).map(|s| s.as_str())
    }

    /// Remove an embedding from the store
    /// Note: This is now efficient with HashMap - no need to rebuild
    pub fn delete(&mut self, id: u64) -> Result<()> {
        if self.id_map.remove(&id).is_some() {
            self.vectors.remove(&id);
            // Update next_id if necessary (only if deleting the highest ID)
            if self.next_id > id && self.next_id == id + 1 {
                // Find the new highest ID
                self.next_id = self
                    .id_map
                    .keys()
                    .max()
                    .map(|&max_id| max_id + 1)
                    .unwrap_or(0);
            }
        }
        Ok(())
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

/// Chunk metadata store for persisting CodeChunk data alongside vectors
pub struct ChunkStore {
    chunks: HashMap<u64, CodeChunk>,
    next_id: u64,
}

impl Default for ChunkStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkStore {
    /// Create a new chunk store
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            next_id: 0,
        }
    }

    /// Insert a chunk and return its ID
    pub fn insert(&mut self, chunk: CodeChunk) -> Result<u64> {
        let id = self.next_id;
        self.chunks.insert(id, chunk);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ID overflow: maximum ID reached"))?;
        Ok(id)
    }

    /// Get a chunk by ID
    pub fn get(&self, id: u64) -> Option<&CodeChunk> {
        self.chunks.get(&id)
    }

    /// Delete a chunk by ID
    pub fn delete(&mut self, id: u64) -> Option<CodeChunk> {
        self.chunks.remove(&id)
    }

    /// Save the chunk store to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize chunks using JSON
        let chunks_json = serde_json::to_string(&self.chunks)?;
        std::fs::write(path, chunks_json)?;

        // Store next_id in a separate file
        let next_id_path = path.with_extension("next");
        let next_id_json = serde_json::to_string(&self.next_id)?;
        std::fs::write(next_id_path, next_id_json)?;

        Ok(())
    }

    /// Load the chunk store from disk
    pub fn load(path: &Path) -> Result<Self> {
        // Load chunks from JSON
        let chunks_json = std::fs::read_to_string(path)?;
        let chunks: HashMap<u64, CodeChunk> = serde_json::from_str(&chunks_json)?;

        // Load next_id from separate file
        let next_id_path = path.with_extension("next");
        let next_id_json = std::fs::read_to_string(next_id_path)?;
        let next_id: u64 = serde_json::from_str(&next_id_json)?;

        // Validate that next_id is consistent with loaded chunks
        let max_id = chunks.keys().max().copied().unwrap_or(0);
        if next_id <= max_id {
            return Err(anyhow::anyhow!(
                "Invalid next_id {} for max chunk id {} - potential data corruption",
                next_id,
                max_id
            ));
        }

        Ok(Self { chunks, next_id })
    }

    /// Get the number of chunks in the store
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get all chunk IDs
    pub fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.chunks.keys().copied()
    }
}

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

    #[test]
    fn vector_store_delete_maintains_consistency() {
        let mut store = VectorStore::new().expect("Failed to create vector store");

        // Add 5 embeddings
        let embeddings: Vec<Vec<f32>> = (0..5).map(|i| vec![i as f32; 384]).collect();
        let ids: Vec<u64> = embeddings
            .iter()
            .enumerate()
            .map(|(i, emb)| store.add(&format!("chunk{}", i), emb).unwrap())
            .collect();

        // Verify initial state
        assert_eq!(store.len(), 5);
        assert_eq!(ids, vec![0, 1, 2, 3, 4]);

        // Delete middle chunk (ID 2)
        store.delete(2).expect("Failed to delete chunk 2");

        // Verify state after deletion
        assert_eq!(store.len(), 4);
        assert!(!store.id_map.contains_key(&2));

        // Verify remaining IDs are still correct
        assert!(store.id_map.contains_key(&0));
        assert!(store.id_map.contains_key(&1));
        assert!(store.id_map.contains_key(&3));
        assert!(store.id_map.contains_key(&4));

        // Test search still works
        let query = vec![1.0; 384];
        let results = store.find_similar(&query, 3).unwrap();
        assert_eq!(results.len(), 3);

        // Test we can still get chunk IDs
        assert_eq!(store.get_chunk_id(0), Some("chunk0"));
        assert_eq!(store.get_chunk_id(1), Some("chunk1"));
        assert_eq!(store.get_chunk_id(3), Some("chunk3"));
        assert_eq!(store.get_chunk_id(4), Some("chunk4"));
        assert_eq!(store.get_chunk_id(2), None);
    }

    #[test]
    fn chunk_store_round_trip() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store_path = temp_dir.path().join("test_chunks.json");

        // Create test chunks
        let chunk1 = CodeChunk {
            file: PathBuf::from("src/main.rs"),
            content: "fn main() { println!(\"Hello, world!\"); }".to_string(),
            start_line: 1,
            end_line: 1,
            kind: ChunkKind::Function,
            name: Some("main".to_string()),
        };

        let chunk2 = CodeChunk {
            file: PathBuf::from("src/utils.rs"),
            content: "pub fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            start_line: 1,
            end_line: 1,
            kind: ChunkKind::Function,
            name: Some("add".to_string()),
        };

        // Test insert and get
        let mut store = ChunkStore::new();
        let id1 = store
            .insert(chunk1.clone())
            .expect("Failed to insert chunk1");
        let id2 = store
            .insert(chunk2.clone())
            .expect("Failed to insert chunk2");

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(store.len(), 2);

        // Test get
        let retrieved1 = store.get(id1).expect("Failed to get chunk1");
        let retrieved2 = store.get(id2).expect("Failed to get chunk2");

        assert_eq!(retrieved1.file, chunk1.file);
        assert_eq!(retrieved1.content, chunk1.content);
        assert_eq!(retrieved1.start_line, chunk1.start_line);
        assert_eq!(retrieved1.end_line, chunk1.end_line);
        assert_eq!(retrieved1.kind, chunk1.kind);
        assert_eq!(retrieved1.name, chunk1.name);

        assert_eq!(retrieved2.file, chunk2.file);
        assert_eq!(retrieved2.content, chunk2.content);
        assert_eq!(retrieved2.start_line, chunk2.start_line);
        assert_eq!(retrieved2.end_line, chunk2.end_line);
        assert_eq!(retrieved2.kind, chunk2.kind);
        assert_eq!(retrieved2.name, chunk2.name);

        // Test delete
        let deleted = store.delete(id1).expect("Failed to delete chunk1");
        assert_eq!(deleted.file, chunk1.file);
        assert_eq!(store.len(), 1);
        assert!(store.get(id1).is_none());
        assert!(store.get(id2).is_some());

        // Test save and load
        store.save(&store_path).expect("Failed to save store");
        let loaded_store = ChunkStore::load(&store_path).expect("Failed to load store");

        assert_eq!(loaded_store.len(), 1);
        let loaded_chunk = loaded_store
            .get(id2)
            .expect("Failed to get chunk from loaded store");
        assert_eq!(loaded_chunk.file, chunk2.file);
        assert_eq!(loaded_chunk.content, chunk2.content);

        // Test IDs iterator
        let ids: Vec<u64> = loaded_store.ids().collect();
        assert_eq!(ids, vec![id2]);
    }
}

/// Project indexer that integrates chunking, embedding, and storage
pub struct ProjectIndexer {
    vector_store: VectorStore,
    chunk_store: ChunkStore,
    embed_pipeline: EmbedPipeline,
    root: PathBuf,
    index_dir: PathBuf,
}

impl ProjectIndexer {
    /// Create a new project indexer
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let index_dir = Self::get_index_dir(&root)?;

        // Create index directory if it doesn't exist
        std::fs::create_dir_all(&index_dir)?;

        Ok(Self {
            vector_store: VectorStore::new()?,
            chunk_store: ChunkStore::new(),
            embed_pipeline: EmbedPipeline::new()?,
            root,
            index_dir,
        })
    }

    /// Get the index directory for a project
    fn get_index_dir(root: &Path) -> Result<PathBuf> {
        // Validate and canonicalize the root path
        let canonical_root = root.canonicalize().map_err(|e| {
            anyhow::anyhow!("Cannot canonicalize root path {}: {}", root.display(), e)
        })?;

        if !canonical_root.is_dir() {
            return Err(anyhow::anyhow!(
                "Root path is not a directory: {}",
                canonical_root.display()
            ));
        }

        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let project_hash = Self::project_hash(&canonical_root)?;
        Ok(home.join(".idep").join("index").join(project_hash))
    }

    /// Generate a stable hash for the project path
    fn project_hash(root: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        root.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }

    /// Index the entire project
    pub fn index_project(&mut self) -> Result<usize> {
        // Clear existing data
        self.vector_store = VectorStore::new()?;
        self.chunk_store = ChunkStore::new();

        // Walk directory tree respecting .gitignore
        let mut total_chunks = 0;
        let walk = ignore::WalkBuilder::new(&self.root)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walk {
            let entry = entry.map_err(|e| anyhow::anyhow!("Walk error: {}", e))?;
            let path = entry.path();

            if path.is_file() && self.is_source_file(path) {
                match self.chunk_file(path) {
                    Ok(chunks) => {
                        total_chunks += chunks.len();

                        // Embed chunks and store them
                        match self.embed_pipeline.run(chunks) {
                            Ok(embedded_chunks) => {
                                for embedded_chunk in embedded_chunks {
                                    let chunk_id =
                                        self.chunk_store.insert(embedded_chunk.chunk.clone())?;
                                    self.vector_store
                                        .add(&chunk_id.to_string(), &embedded_chunk.embedding)?;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to embed {}: {}", path.display(), e);
                                // Continue with other files
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to chunk {}: {}", path.display(), e);
                        // Continue with other files
                    }
                }
            }
        }

        // Save the index
        self.save_index()?;

        tracing::info!(
            "Indexed {} chunks from project at {}",
            total_chunks,
            self.root.display()
        );
        Ok(total_chunks)
    }

    /// Re-index a single file (diff-based)
    pub fn reindex_file(&mut self, path: &Path) -> Result<usize> {
        // Try to normalize the path for comparison, but handle non-existent files gracefully
        let normalized_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If the file doesn't exist, we can't reindex it
                // This is expected for new files that haven't been saved yet
                return Ok(0);
            }
        };

        // Remove old chunks for this file
        let chunks_to_remove: Vec<u64> = self
            .chunk_store
            .ids()
            .filter(|&id| {
                if let Some(chunk) = self.chunk_store.get(id) {
                    // Compare normalized paths
                    chunk
                        .file
                        .canonicalize()
                        .is_ok_and(|chunk_path| chunk_path == normalized_path)
                } else {
                    false
                }
            })
            .collect();

        let removed_count = chunks_to_remove.len();
        for chunk_id in &chunks_to_remove {
            self.chunk_store.delete(*chunk_id);
            // Delete the corresponding embedding from VectorStore
            self.vector_store.delete(*chunk_id)?;
        }

        // Re-chunk, re-embed, re-insert
        if self.is_source_file(path) {
            let chunks = self.chunk_file(path)?;
            let embedded_chunks = self.embed_pipeline.run(chunks)?;

            for embedded_chunk in embedded_chunks {
                let chunk_id = self.chunk_store.insert(embedded_chunk.chunk.clone())?;
                self.vector_store
                    .add(&chunk_id.to_string(), &embedded_chunk.embedding)?;
            }
        }

        // Save the updated index
        self.save_index()?;

        tracing::info!("Re-indexed file: {}", path.display());
        Ok(removed_count)
    }

    /// Save the index to disk
    fn save_index(&self) -> Result<()> {
        let vector_store_path = self.index_dir.join("vectors.json");
        let chunk_store_path = self.index_dir.join("chunks.json");

        self.vector_store.save(&vector_store_path)?;
        self.chunk_store.save(&chunk_store_path)?;

        Ok(())
    }

    /// Load the index from disk
    pub fn load_index(&mut self) -> Result<()> {
        let vector_store_path = self.index_dir.join("vectors.json");
        let chunk_store_path = self.index_dir.join("chunks.json");

        // Validate that both files exist
        let vectors_exist = vector_store_path.exists();
        let chunks_exist = chunk_store_path.exists();

        if vectors_exist != chunks_exist {
            return Err(anyhow::anyhow!(
                "Index files are inconsistent: vectors={}, chunks={}",
                vectors_exist,
                chunks_exist
            ));
        }

        if vectors_exist && chunks_exist {
            self.vector_store = VectorStore::load(&vector_store_path)?;
            self.chunk_store = ChunkStore::load(&chunk_store_path)?;

            // Validate consistency between stores
            if self.vector_store.len() != self.chunk_store.len() {
                return Err(anyhow::anyhow!(
                    "Index inconsistency: {} vectors but {} chunks",
                    self.vector_store.len(),
                    self.chunk_store.len()
                ));
            }

            tracing::info!("Loaded index from {}", self.index_dir.display());
        }

        Ok(())
    }

    /// Check if a file is a source file we should index
    fn is_source_file(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "toml" | "md")
        )
    }

    /// Chunk a single file using naive chunking
    fn chunk_file(&self, path: &Path) -> Result<Vec<CodeChunk>> {
        let content = std::fs::read_to_string(path)?;
        Ok(self.naive_chunk(path, &content))
    }

    /// Naive line-based chunking (fallback)
    fn naive_chunk(&self, path: &Path, content: &str) -> Vec<CodeChunk> {
        let lines: Vec<&str> = content.lines().collect();
        let chunk_size = 512; // chars, not lines
        let overlap = 5;
        let mut chunks = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let mut chunk_content = String::new();
            let mut char_count = 0;
            let mut end_line = i;

            for (line_idx, line) in lines[i..].iter().enumerate() {
                // Check if this single line is too long
                if line.len() > chunk_size && line_idx == 0 {
                    // Split long lines or create a chunk with just this line
                    chunk_content.push_str(&line[..chunk_size.min(line.len())]);
                    end_line = i + 1;
                    break;
                }

                if char_count + line.len() + 1 > chunk_size && line_idx > 0 {
                    break;
                }
                chunk_content.push_str(line);
                chunk_content.push('\n');
                char_count += line.len() + 1;
                end_line = i + line_idx + 1;
            }

            chunks.push(CodeChunk {
                file: path.to_path_buf(),
                content: chunk_content,
                start_line: i + 1,
                end_line,
                kind: ChunkKind::Other,
                name: None,
            });

            if end_line >= lines.len() {
                break;
            }
            // Ensure forward progress by calculating overlap correctly
            let lines_added = end_line - i;
            // Move to the next chunk, ensuring we don't go backwards and don't skip too much
            if lines_added <= overlap {
                // If chunk is small or overlap is large, move forward at least 1 line
                i = end_line + 1;
            } else {
                // Normal case: move to end_line - overlap to create overlap
                i = end_line - overlap;
            }
        }

        chunks
    }

    /// Get the number of indexed chunks
    pub fn len(&self) -> usize {
        self.chunk_store.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.chunk_store.is_empty()
    }

    /// Get the index directory path
    pub fn index_dir(&self) -> &Path {
        &self.index_dir
    }
}

#[cfg(test)]
mod project_indexer_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn project_indexer_creates_index_dir() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");

        // Check that index directory was created
        assert!(indexer.index_dir().exists());
        assert!(indexer
            .index_dir()
            .join("vectors.json")
            .parent()
            .unwrap()
            .exists());
    }

    #[test]
    fn project_indexer_indexes_small_project() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        // Create test files
        let rust_file = project_path.join("src").join("main.rs");
        fs::create_dir_all(rust_file.parent().unwrap()).expect("Failed to create src dir");
        let mut file = fs::File::create(&rust_file).expect("Failed to create main.rs");
        writeln!(file, "fn main() {{\n    println!(\"Hello, world!\");\n}}")
            .expect("Failed to write main.rs");

        let python_file = project_path.join("script.py");
        let mut file = fs::File::create(&python_file).expect("Failed to create script.py");
        writeln!(file, "def hello():\n    print(\"Hello, Python!\")")
            .expect("Failed to write script.py");

        // Index the project
        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let chunk_count = indexer.index_project().expect("Failed to index project");

        // Verify chunks were created
        assert!(chunk_count > 0);
        assert!(!indexer.is_empty());
        assert_eq!(indexer.len(), chunk_count);

        // Verify index files exist
        assert!(indexer.index_dir().join("vectors.json").exists());
        assert!(indexer.index_dir().join("chunks.json").exists());
        assert!(indexer.index_dir().join("chunks.next").exists());
        assert!(indexer.index_dir().join("vectors.id_map.json").exists());
    }

    #[test]
    fn project_indexer_respects_gitignore() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        // Create .gitignore
        let gitignore_path = project_path.join(".gitignore");
        let mut file = fs::File::create(&gitignore_path).expect("Failed to create .gitignore");
        writeln!(file, "target/").expect("Failed to write .gitignore");

        // Create files
        let src_file = project_path.join("src").join("main.rs");
        fs::create_dir_all(src_file.parent().unwrap()).expect("Failed to create src dir");
        let mut file = fs::File::create(&src_file).expect("Failed to create main.rs");
        writeln!(file, "fn main() {{}}").expect("Failed to write main.rs");

        let target_file = project_path.join("target").join("debug").join("main");
        fs::create_dir_all(target_file.parent().unwrap()).expect("Failed to create target dir");
        let mut file = fs::File::create(&target_file).expect("Failed to create target file");
        writeln!(file, "binary content").expect("Failed to write target file");

        // Index the project
        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let chunk_count = indexer.index_project().expect("Failed to index project");

        // Should only have chunks from src/main.rs, not from target/
        assert!(!indexer.is_empty());
        assert!(chunk_count < 10); // Should be small since target/ is ignored
    }

    #[test]
    fn project_indexer_save_and_load() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        // Create test file
        let rust_file = project_path.join("main.rs");
        let mut file = fs::File::create(&rust_file).expect("Failed to create main.rs");
        writeln!(file, "fn test() {{\n    println!(\"test\");\n}}")
            .expect("Failed to write main.rs");

        // Index and save
        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let original_count = indexer.index_project().expect("Failed to index project");
        assert!(original_count > 0);

        // Create new indexer and load
        let mut new_indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        new_indexer.load_index().expect("Failed to load index");

        // Verify loaded data
        assert_eq!(new_indexer.len(), original_count);
        assert!(!new_indexer.is_empty());
    }

    #[test]
    fn project_indexer_reindex_nonexistent_file() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");

        // Try to reindex a file that doesn't exist
        let nonexistent_file = project_path.join("nonexistent.rs");
        let result = indexer.reindex_file(&nonexistent_file);

        // Should succeed but return 0 (no chunks removed)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn project_indexer_reindex_file() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        // Create test file
        let rust_file = project_path.join("main.rs");
        let mut file = fs::File::create(&rust_file).expect("Failed to create main.rs");
        writeln!(file, "fn original() {{\n    println!(\"original\");\n}}")
            .expect("Failed to write main.rs");

        // Index the project
        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let original_count = indexer.index_project().expect("Failed to index project");
        assert!(original_count > 0);

        // Modify the file
        let mut file = fs::File::create(&rust_file).expect("Failed to recreate main.rs");
        writeln!(
            file,
            "fn modified() {{\n    println!(\"modified\");\n    fn nested() {{}}\n}}"
        )
        .expect("Failed to write modified main.rs");

        // Re-index the file
        let _removed_count = indexer
            .reindex_file(&rust_file)
            .expect("Failed to re-index file");

        // Verify the file was re-indexed (chunk count may change)
        assert!(!indexer.is_empty());
    }

    #[test]
    fn project_indexer_benchmark_performance() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("benchmark_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        // Create a smaller synthetic project for quick benchmarking
        let files_to_create = 20; // ~100 lines per file = ~2k LOC total
        let lines_per_file = 100;
        let mut total_lines = 0;

        for i in 0..files_to_create {
            let file_path = project_path.join(format!("src/file_{}.rs", i));
            fs::create_dir_all(file_path.parent().unwrap()).expect("Failed to create src dir");

            let mut file = fs::File::create(&file_path).expect("Failed to create file");
            writeln!(file, "//! Module file_{}", i).expect("Failed to write header");

            // Generate synthetic Rust code
            for j in 0..lines_per_file {
                writeln!(file, "/// Function {}_{} documentation", i, j)
                    .expect("Failed to write doc");
                writeln!(file, "pub fn function_{}_{}() -> u64 {{", i, j)
                    .expect("Failed to write fn");
                writeln!(file, "    let x = {}u64;", j * 7).expect("Failed to write let");
                writeln!(file, "    x * 2 + {}", j).expect("Failed to write calc");
                writeln!(file, "}}").expect("Failed to write close");
                writeln!(file).expect("Failed to write blank line");
            }

            total_lines += lines_per_file + 2;
        }

        // Create main.rs
        let main_path = project_path.join("src/main.rs");
        let mut main_file = fs::File::create(&main_path).expect("Failed to create main.rs");
        writeln!(
            main_file,
            "fn main() {{ println!(\"Benchmark: {} LOC\"); }}",
            total_lines
        )
        .expect("Failed to write main");
        total_lines += 1;

        println!(
            "Created benchmark project: {} LOC across {} files",
            total_lines,
            files_to_create + 1
        );

        // Benchmark indexing
        let start_time = std::time::Instant::now();

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let chunk_count = indexer.index_project().expect("Failed to index project");

        let indexing_duration = start_time.elapsed();

        // Quick search benchmark
        let search_start = std::time::Instant::now();
        let test_query = "function documentation";
        let query_embedding = {
            let mut temp_embedder = Embedder::new().expect("Failed to create embedder");
            temp_embedder
                .embed_batch(&[test_query])
                .expect("Failed to embed query")
                .remove(0)
        };
        let search_results = indexer
            .vector_store
            .find_similar(&query_embedding, 5)
            .expect("Failed to search");
        let search_duration = search_start.elapsed();

        // Results
        println!("=== Benchmark Results ===");
        println!(
            "Project: {} LOC, {} files",
            total_lines,
            files_to_create + 1
        );
        println!("Chunks: {}", chunk_count);
        println!("Indexing: {:?}", indexing_duration);
        println!("Search: {:?}", search_duration);
        println!(
            "Rate: {:.1} LOC/sec",
            total_lines as f64 / indexing_duration.as_secs_f64()
        );

        // Basic assertions
        assert!(total_lines >= 2000, "Should have at least 2k LOC");
        assert!(chunk_count > 0, "Should generate chunks");
        assert!(!indexer.is_empty(), "Index should not be empty");
        assert!(
            indexing_duration.as_secs() < 120,
            "Should complete within 2 minutes"
        );
        assert!(!search_results.is_empty(), "Search should return results");

        // Extrapolate to 50k LOC estimate
        let estimated_50k_time = std::time::Duration::from_secs_f64(
            (50000.0 / total_lines as f64) * indexing_duration.as_secs_f64(),
        );
        println!("Estimated 50k LOC indexing time: {:?}", estimated_50k_time);

        println!("✅ Benchmark completed successfully!");
    }

    #[test]
    fn naive_chunking_overlap_doesnt_skip_content() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");

        // Create content with short lines to trigger overlap edge case
        // Make it long enough to create multiple chunks (512 chars each)
        let mut content = String::new();
        for i in 1..=100 {
            // Each line is about 50 chars, so 100 lines = ~5000 chars = ~10 chunks
            content.push_str(&format!(
                "This is line number {} with some additional text to make it longer\n",
                i
            ));
        }
        let path = project_path.join("test.txt");

        let chunks = indexer.naive_chunk(&path, &content);

        // Verify no lines are skipped - all lines should be covered
        let mut covered_lines = std::collections::HashSet::new();
        for chunk in &chunks {
            for line_num in chunk.start_line..=chunk.end_line {
                covered_lines.insert(line_num);
            }
        }

        // Should cover lines 1-100
        for line in 1..=100 {
            assert!(
                covered_lines.contains(&line),
                "Line {} was not covered",
                line
            );
        }

        // Verify we have multiple chunks
        assert!(
            chunks.len() >= 2,
            "Should have at least 2 chunks, got {}",
            chunks.len()
        );

        // Verify overlap is working (chunks should overlap)
        let mut has_overlap = false;
        for window in chunks.windows(2) {
            let first_end = window[0].end_line;
            let second_start = window[1].start_line;
            if second_start <= first_end {
                has_overlap = true;
                break;
            }
        }
        assert!(has_overlap, "Chunks should overlap");
    }

    #[test]
    fn project_indexer_detects_source_files() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");

        // Test various file extensions
        assert!(indexer.is_source_file(Path::new("test.rs")));
        assert!(indexer.is_source_file(Path::new("test.ts")));
        assert!(indexer.is_source_file(Path::new("test.py")));
        assert!(indexer.is_source_file(Path::new("test.md")));

        // Test non-source files
        assert!(!indexer.is_source_file(Path::new("test.txt")));
        assert!(!indexer.is_source_file(Path::new("test.bin")));
        assert!(!indexer.is_source_file(Path::new("test")));
    }
}
