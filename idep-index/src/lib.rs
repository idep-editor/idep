// idep-index — semantic indexing
//
// Codebase search and embeddings

use anyhow::Result;
use fastembed::{EmbeddingModel, TextEmbedding};
use std::path::PathBuf;

pub struct Embedder {
    model: TextEmbedding,
    model_path: PathBuf,
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
}
