// idep-index — semantic indexing
//
// Codebase search and embeddings

pub mod chunk_store;
pub mod embedder;
pub mod indexer;
pub mod pipeline;
pub mod vector_store;

// Re-export main types for convenience
pub use chunk_store::ChunkStore;
pub use embedder::{EmbeddedChunk, Embedder};
pub use indexer::ProjectIndexer;
pub use pipeline::EmbedPipeline;
pub use vector_store::{cosine_similarity, ScoredChunk, VectorStore};
