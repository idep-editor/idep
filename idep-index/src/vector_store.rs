// vector_store.rs — Vector storage with similarity search

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredChunk {
    pub id: u64,
    pub score: f32,
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStore {
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

        if embedding.len() != 384 {
            return Err(anyhow::anyhow!(
                "Embedding must be 384 dimensions, got {}",
                embedding.len()
            ));
        }

        self.vectors.insert(id, embedding.to_vec());
        self.id_map.insert(id, chunk_id.to_string());
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ID overflow: maximum ID reached"))?;

        Ok(id)
    }

    /// Find similar embeddings using brute-force cosine similarity
    pub fn find_similar(&self, embedding: &[f32], top_k: usize) -> Result<Vec<ScoredChunk>> {
        if embedding.len() != 384 {
            return Err(anyhow::anyhow!(
                "Query embedding must be 384 dimensions, got {}",
                embedding.len()
            ));
        }

        let mut similarities: Vec<(u64, f32)> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let similarity = cosine_similarity(embedding, vec);
                (*id, similarity)
            })
            .collect();

        similarities.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or_else(|| {
                if b.1.is_nan() {
                    std::cmp::Ordering::Less
                } else if a.1.is_nan() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });

        let results: Vec<ScoredChunk> = similarities
            .into_iter()
            .take(top_k)
            .map(|(id, score)| ScoredChunk { id, score })
            .collect();

        Ok(results)
    }

    /// Save the index to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        if self.id_map.len() != self.vectors.len() {
            return Err(anyhow::anyhow!(
                "ID map size {} doesn't match vector count {}",
                self.id_map.len(),
                self.vectors.len()
            ));
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let vectors_json = serde_json::to_string(&self.vectors)?;
        std::fs::write(path, vectors_json)?;

        let id_map_path = path.with_extension("id_map.json");
        let id_map_json = serde_json::to_string(&self.id_map)?;
        std::fs::write(id_map_path, id_map_json)?;

        Ok(())
    }

    /// Load the index from disk
    pub fn load(path: &Path) -> Result<Self> {
        let vectors_json = std::fs::read_to_string(path)?;
        let vectors: HashMap<u64, Vec<f32>> = serde_json::from_str(&vectors_json)?;

        let id_map_path = path.with_extension("id_map.json");
        let id_map_json = std::fs::read_to_string(id_map_path)?;
        let id_map: HashMap<u64, String> = serde_json::from_str(&id_map_json)?;

        if id_map.len() != vectors.len() {
            return Err(anyhow::anyhow!(
                "Loaded ID map size {} doesn't match vector count {}",
                id_map.len(),
                vectors.len()
            ));
        }

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
    pub fn delete(&mut self, id: u64) -> Result<()> {
        if self.id_map.remove(&id).is_some() {
            self.vectors.remove(&id);
            if self.next_id > id && self.next_id == id + 1 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn vector_store_creation() {
        let store = VectorStore::new();
        assert!(store.is_ok());
    }

    #[test]
    fn vector_store_basic_add() {
        let mut store = VectorStore::new().expect("Failed to create vector store");
        let embedding: Vec<f32> = vec![0.1; 384];
        let _id = store.add("chunk1", &embedding);
    }

    #[test]
    fn vector_store_save_and_load() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test_index.json");

        let mut store = VectorStore::new().expect("Failed to create vector store");
        let embedding1: Vec<f32> = vec![0.1; 384];
        let embedding2: Vec<f32> = vec![0.2; 384];

        store
            .add("chunk1", &embedding1)
            .expect("Failed to add embedding");
        store
            .add("chunk2", &embedding2)
            .expect("Failed to add embedding");

        store.save(&index_path).expect("Failed to save store");

        let loaded_store = VectorStore::load(&index_path).expect("Failed to load store");
        assert_eq!(loaded_store.len(), 2);
        assert_eq!(loaded_store.get_chunk_id(0), Some("chunk1"));
        assert_eq!(loaded_store.get_chunk_id(1), Some("chunk2"));

        let query: Vec<f32> = vec![0.15; 384];
        let results = loaded_store
            .find_similar(&query, 2)
            .expect("Failed to search loaded store");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn vector_store_self_similarity() {
        let mut store = VectorStore::new().expect("Failed to create vector store");

        let mut embeddings = Vec::new();
        for i in 0..50 {
            let embedding: Vec<f32> = (0..384).map(|j| (i + j) as f32 / 1000.0).collect();
            embeddings.push(embedding);
            store
                .add(&format!("chunk{}", i), &embeddings[i])
                .expect("Failed to add embedding");
        }

        for (i, embedding) in embeddings.iter().enumerate() {
            let results = store.find_similar(embedding, 1).expect("Failed to search");
            assert_eq!(results.len(), 1);
            assert_eq!(
                store.get_chunk_id(results[0].id),
                Some(format!("chunk{}", i).as_str())
            );
            assert!(results[0].score > 0.99);
        }
    }

    #[test]
    fn vector_store_delete_maintains_consistency() {
        let mut store = VectorStore::new().expect("Failed to create vector store");

        let embeddings: Vec<Vec<f32>> = (0..5).map(|i| vec![i as f32; 384]).collect();
        let ids: Vec<u64> = embeddings
            .iter()
            .enumerate()
            .map(|(i, emb)| store.add(&format!("chunk{}", i), emb).unwrap())
            .collect();

        assert_eq!(store.len(), 5);
        assert_eq!(ids, vec![0, 1, 2, 3, 4]);

        store.delete(2).expect("Failed to delete chunk 2");

        assert_eq!(store.len(), 4);
        assert!(!store.id_map.contains_key(&2));
        assert!(store.id_map.contains_key(&0));
        assert!(store.id_map.contains_key(&1));
        assert!(store.id_map.contains_key(&3));
        assert!(store.id_map.contains_key(&4));

        let query = vec![1.0; 384];
        let results = store.find_similar(&query, 3).unwrap();
        assert_eq!(results.len(), 3);

        assert_eq!(store.get_chunk_id(0), Some("chunk0"));
        assert_eq!(store.get_chunk_id(1), Some("chunk1"));
        assert_eq!(store.get_chunk_id(3), Some("chunk3"));
        assert_eq!(store.get_chunk_id(4), Some("chunk4"));
        assert_eq!(store.get_chunk_id(2), None);
    }
}
