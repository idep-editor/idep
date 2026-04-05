// chunk_store.rs — Chunk metadata persistence

use anyhow::Result;
use idep_ai::indexer::CodeChunk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Chunk metadata store for persisting CodeChunk data alongside vectors
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let chunks_json = serde_json::to_string(&self.chunks)?;
        std::fs::write(path, chunks_json)?;

        let next_id_path = path.with_extension("next");
        let next_id_json = serde_json::to_string(&self.next_id)?;
        std::fs::write(next_id_path, next_id_json)?;

        Ok(())
    }

    /// Load the chunk store from disk
    pub fn load(path: &Path) -> Result<Self> {
        let chunks_json = std::fs::read_to_string(path)?;
        let chunks: HashMap<u64, CodeChunk> = serde_json::from_str(&chunks_json)?;

        let next_id_path = path.with_extension("next");
        let next_id_json = std::fs::read_to_string(next_id_path)?;
        let next_id: u64 = serde_json::from_str(&next_id_json)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use idep_ai::indexer::{ChunkKind, CodeChunk};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn chunk_store_round_trip() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store_path = temp_dir.path().join("test_chunks.json");

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

        let retrieved1 = store.get(id1).expect("Failed to get chunk1");
        let retrieved2 = store.get(id2).expect("Failed to get chunk2");

        assert_eq!(retrieved1.file, chunk1.file);
        assert_eq!(retrieved1.content, chunk1.content);
        assert_eq!(retrieved2.file, chunk2.file);
        assert_eq!(retrieved2.content, chunk2.content);

        let deleted = store.delete(id1).expect("Failed to delete chunk1");
        assert_eq!(deleted.file, chunk1.file);
        assert_eq!(store.len(), 1);
        assert!(store.get(id1).is_none());
        assert!(store.get(id2).is_some());

        store.save(&store_path).expect("Failed to save store");
        let loaded_store = ChunkStore::load(&store_path).expect("Failed to load store");

        assert_eq!(loaded_store.len(), 1);
        let loaded_chunk = loaded_store
            .get(id2)
            .expect("Failed to get chunk from loaded store");
        assert_eq!(loaded_chunk.file, chunk2.file);
        assert_eq!(loaded_chunk.content, chunk2.content);

        let ids: Vec<u64> = loaded_store.ids().collect();
        assert_eq!(ids, vec![id2]);
    }
}
