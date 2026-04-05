// indexer.rs — Project-level indexing coordinator

use anyhow::Result;
use idep_ai::indexer::{ChunkKind, CodeChunk};
use std::path::{Path, PathBuf};

use crate::chunk_store::ChunkStore;
use crate::pipeline::EmbedPipeline;
use crate::vector_store::VectorStore;

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
        self.vector_store = VectorStore::new()?;
        self.chunk_store = ChunkStore::new();

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
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to chunk {}: {}", path.display(), e);
                    }
                }
            }
        }

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
        let normalized_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return Ok(0);
            }
        };

        let chunks_to_remove: Vec<u64> = self
            .chunk_store
            .ids()
            .filter(|&id| {
                if let Some(chunk) = self.chunk_store.get(id) {
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
            self.vector_store.delete(*chunk_id)?;
        }

        if self.is_source_file(path) {
            let chunks = self.chunk_file(path)?;
            let embedded_chunks = self.embed_pipeline.run(chunks)?;

            for embedded_chunk in embedded_chunks {
                let chunk_id = self.chunk_store.insert(embedded_chunk.chunk.clone())?;
                self.vector_store
                    .add(&chunk_id.to_string(), &embedded_chunk.embedding)?;
            }
        }

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
    pub fn is_source_file(&self, path: &Path) -> bool {
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
    pub fn naive_chunk(&self, path: &Path, content: &str) -> Vec<CodeChunk> {
        let lines: Vec<&str> = content.lines().collect();
        let chunk_size = 512;
        let overlap = 5;
        let mut chunks = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let mut chunk_content = String::new();
            let mut char_count = 0;
            let mut end_line = i;

            for (line_idx, line) in lines[i..].iter().enumerate() {
                if line.len() > chunk_size && line_idx == 0 {
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
            let lines_added = end_line - i;
            if lines_added <= overlap {
                i = end_line + 1;
            } else {
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
mod tests {
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

        let rust_file = project_path.join("src").join("main.rs");
        fs::create_dir_all(rust_file.parent().unwrap()).expect("Failed to create src dir");
        let mut file = fs::File::create(&rust_file).expect("Failed to create main.rs");
        writeln!(file, "fn main() {{\n    println!(\"Hello, world!\");\n}}")
            .expect("Failed to write main.rs");

        let python_file = project_path.join("script.py");
        let mut file = fs::File::create(&python_file).expect("Failed to create script.py");
        writeln!(file, "def hello():\n    print(\"Hello, Python!\")")
            .expect("Failed to write script.py");

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let chunk_count = indexer.index_project().expect("Failed to index project");

        assert!(chunk_count > 0);
        assert!(!indexer.is_empty());
        assert_eq!(indexer.len(), chunk_count);

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

        let gitignore_path = project_path.join(".gitignore");
        let mut file = fs::File::create(&gitignore_path).expect("Failed to create .gitignore");
        writeln!(file, "target/").expect("Failed to write .gitignore");

        let src_file = project_path.join("src").join("main.rs");
        fs::create_dir_all(src_file.parent().unwrap()).expect("Failed to create src dir");
        let mut file = fs::File::create(&src_file).expect("Failed to create main.rs");
        writeln!(file, "fn main() {{}}").expect("Failed to write main.rs");

        let target_file = project_path.join("target").join("debug").join("main");
        fs::create_dir_all(target_file.parent().unwrap()).expect("Failed to create target dir");
        let mut file = fs::File::create(&target_file).expect("Failed to create target file");
        writeln!(file, "binary content").expect("Failed to write target file");

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let chunk_count = indexer.index_project().expect("Failed to index project");

        assert!(!indexer.is_empty());
        assert!(chunk_count < 10);
    }

    #[test]
    fn project_indexer_save_and_load() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let rust_file = project_path.join("main.rs");
        let mut file = fs::File::create(&rust_file).expect("Failed to create main.rs");
        writeln!(file, "fn test() {{\n    println!(\"test\");\n}}")
            .expect("Failed to write main.rs");

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let original_count = indexer.index_project().expect("Failed to index project");
        assert!(original_count > 0);

        let mut new_indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        new_indexer.load_index().expect("Failed to load index");

        assert_eq!(new_indexer.len(), original_count);
        assert!(!new_indexer.is_empty());
    }

    #[test]
    fn project_indexer_reindex_nonexistent_file() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");

        let nonexistent_file = project_path.join("nonexistent.rs");
        let result = indexer.reindex_file(&nonexistent_file);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn project_indexer_reindex_file() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let rust_file = project_path.join("main.rs");
        let mut file = fs::File::create(&rust_file).expect("Failed to create main.rs");
        writeln!(file, "fn original() {{\n    println!(\"original\");\n}}")
            .expect("Failed to write main.rs");

        let mut indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");
        let original_count = indexer.index_project().expect("Failed to index project");
        assert!(original_count > 0);

        let mut file = fs::File::create(&rust_file).expect("Failed to recreate main.rs");
        writeln!(
            file,
            "fn modified() {{\n    println!(\"modified\");\n    fn nested() {{}}\n}}"
        )
        .expect("Failed to write modified main.rs");

        let _removed_count = indexer
            .reindex_file(&rust_file)
            .expect("Failed to re-index file");

        assert!(!indexer.is_empty());
    }

    #[test]
    fn naive_chunking_overlap_doesnt_skip_content() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let project_path = temp_dir.path().join("test_project");
        fs::create_dir(&project_path).expect("Failed to create project dir");

        let indexer = ProjectIndexer::new(&project_path).expect("Failed to create indexer");

        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!(
                "This is line number {} with some additional text to make it longer\n",
                i
            ));
        }
        let path = project_path.join("test.txt");

        let chunks = indexer.naive_chunk(&path, &content);

        let mut covered_lines = std::collections::HashSet::new();
        for chunk in &chunks {
            for line_num in chunk.start_line..=chunk.end_line {
                covered_lines.insert(line_num);
            }
        }

        for line in 1..=100 {
            assert!(
                covered_lines.contains(&line),
                "Line {} was not covered",
                line
            );
        }

        assert!(
            chunks.len() >= 2,
            "Should have at least 2 chunks, got {}",
            chunks.len()
        );

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

        assert!(indexer.is_source_file(Path::new("test.rs")));
        assert!(indexer.is_source_file(Path::new("test.ts")));
        assert!(indexer.is_source_file(Path::new("test.py")));
        assert!(indexer.is_source_file(Path::new("test.md")));

        assert!(!indexer.is_source_file(Path::new("test.txt")));
        assert!(!indexer.is_source_file(Path::new("test.bin")));
        assert!(!indexer.is_source_file(Path::new("test")));
    }
}
