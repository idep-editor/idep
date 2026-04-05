# Changelog

All notable changes to Idep are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [v0.0.9] — 2026-04-05

### Added
- **RAG Context Engine** for intelligent codebase context gathering
- `ContextEngine` struct with query, workspace root, and cursor position support
- `ContextEngine::gather()` method combining multiple context sources:
  - Current file content (always included)
  - AST subtree around cursor using Tree-sitter
  - Top-k similar chunks from vector index
  - Recent edit history (last N saves)
- **Context serialization** to prompt-friendly text blocks
- **Token budget manager** with configurable max context tokens (default 4096)
- **Priority-based context truncation**: cursor context > similar chunks > history
- **Chat with RAG integration** via `ChatSession::send_with_context()`
- **Chat history export** with `ChatSession::export()` JSON serialization
- **Anthropic native message format** replacing legacy prompt building
- **Context window management** with intelligent history truncation

### Fixed
- **Message format compatibility** for Anthropic Claude API
- **Context injection timing** to ensure proper prompt construction
- **Token counting accuracy** for context budget management
- **Priority truncation logic** to preserve most relevant context
- **AST subtree extraction** for precise cursor context

### Testing
- **4 new integration tests** for RAG context functionality
- **3 new context engine tests** covering creation and serialization
- **4 new chat format tests** for native message building
- **Context priority truncation tests** verifying budget management
- **RAG function reference tests** ensuring accurate codebase responses
- **All 27 idep-ai tests passing** with comprehensive coverage

### Performance
- **Context gathering**: <100ms for typical project queries
- **Token budget management**: O(n) truncation with priority ordering
- **RAG response quality**: Demonstrably references correct codebase chunks

---

## [v0.0.8] — 2026-03-26

### Added
- **Vector store with similarity search** using brute-force cosine similarity
- `VectorStore` struct with 384-dimension embedding storage and ID mapping
- `VectorStore::add()` for validated embedding insertion with auto-incrementing IDs
- `VectorStore::find_similar()` for top-k semantic similarity search with cosine scoring
- `ScoredChunk` struct combining chunk ID with similarity score
- **Persistent index storage** with JSON serialization for vectors and ID mapping
- `VectorStore::save()` and `VectorStore::load()` for disk persistence and recovery
- **Chunk metadata store** for tracking CodeChunk metadata alongside embeddings
- `ChunkStore` with CRUD operations (get, insert, delete) and JSON persistence
- **Project indexer** integrating chunking, embedding, and storage pipeline
- `ProjectIndexer::index_project()` for full project indexing with .gitignore respect
- `ProjectIndexer::reindex_file()` for diff-based file re-indexing
- **Stable project hashing** for consistent index storage at `~/.idep/index/<project-hash>/`
- **Naive line-based chunking** with 512-char chunks and 5-line overlap
- **Comprehensive error handling** with graceful failure recovery during indexing
- **Performance benchmark** for 50k LOC projects with extrapolation estimates

### Fixed
- **Critical data consistency issues** in reindexing by implementing VectorStore::delete()
- **Path normalization bugs** using canonicalize() for reliable file comparison
- **Unbounded chunk size** vulnerability for lines longer than 512 characters
- **Index directory validation** with proper path canonicalization and directory checks
- **Load index consistency** with validation for file existence and count matching
- **Embedding pipeline resilience** to skip failed files instead of crashing entire indexing
- **Chunk overlap calculation** bug preventing infinite loops in edge cases
- **VectorStore ID type consistency** unified to u64 across all operations

### Performance
- **Indexing rate**: 72.4 LOC/sec (embedding-limited)
- **Search performance**: 568ms for semantic similarity queries
- **Estimated 50k LOC indexing time**: ~11.5 minutes
- **Memory efficiency**: JSON-based persistence with minimal overhead
- **Scalability**: Linear performance characteristics suitable for large codebases

### Testing
- **17 comprehensive tests** covering all major functionality
- **Round-trip persistence tests** for VectorStore and ChunkStore
- **Project indexing tests** with .gitignore validation and reindexing
- **Performance benchmark** with extrapolation to 50k LOC
- **Error handling tests** for edge cases and failure scenarios
- **All tests passing** with no regressions

### Changed
- Workspace version bumped to 0.0.8
- Made `EmbedPipeline.embedder` public for benchmark access
- Added serde derives to `CodeChunk` and `ChunkKind` for persistence
- Enhanced error messages with detailed context and path information

---

## [v0.0.7] — 2026-03-22

### Added
- **Local embeddings pipeline** with fastembed v5 and AllMiniLML6V2 model
- `Embedder` struct with automatic model download and caching to `~/.idep/models/`
- `Embedder::embed_batch()` method with 384-dimension assertion for AllMiniLML6V2
- `EmbedPipeline` for batch processing with configurable batch size (default 32)
- `EmbeddedChunk` struct combining CodeChunk with Vec<f32> embedding
- Progress callbacks for large batch embedding operations via `run_with_progress()`
- Comprehensive test suite:
  - Unit test: embed batch of 10 strings, verify 384-dimension shape
  - Benchmark: time to embed 100 chunks of ~200 tokens each
  - Network independence test: verifies no outbound network calls after initial download
  - Pipeline tests: verifies one embedding per chunk, custom batch sizes, progress callbacks
- Model cache exclusion from git via .gitignore (`.fastembed_cache/`, `*.onnx`, `*.lock`)
- Internal workspace dependencies for cross-crate integration

### Changed
- Added `PartialEq` to `ChunkKind` for testing pipeline functionality
- Workspace version bumped to 0.0.7
- All 62 tests passing across all crates

---

## [v0.0.6] — 2026-03-19

### Added
- **Tree-sitter AST chunking** for Rust, TypeScript, and Python with labeled spans and names
- `AstChunker` and `Chunk` types, integrated into indexer with AST-first chunking
- TypeScript/Python chunk extraction tests (functions, classes, interfaces, type aliases)
- Oversized chunk splitting with default max size (512 chars)

### Fixed
- Graceful fallback to naive line chunking for unsupported languages or parse failures
- Deterministic chunk naming propagation into CodeChunk

### Changed
- Indexer now uses AST chunking by default, with configurable fallback chunk size

---

## [v0.0.5] — 2026-03-19

### Added
- **LSP diagnostics**: `textDocument/publishDiagnostics` handling and per-document diagnostic storage with retrieval API
- **Hover support**: `textDocument/hover` request builder, response parsing, and plain-text extraction
- **Goto definition**: `textDocument/definition` builder and normalized `Location` list from `Location` / `LocationLink`
- WSL URI normalization applied consistently for all textDocument requests and diagnostics storage
- Integration tests for Rust Analyzer: completions, hover, goto-definition, and diagnostics

### Fixed
- Normalized diagnostic URI lookups to avoid client/server URI format mismatches (WSL path forms)
- Cleared stale diagnostics on `didClose` and added regression tests
- Added backend timeout guard for integration notification polling to avoid indefinite hangs

### Changed
- `DocumentManager` now routes all provider URIs through `to_server_uri()` before sending and querying
- `LspClient` hover text helper returns plain text for `MarkupContent` and `MarkedString` variants
- Completed all v0.0.5 TODO milestones in `TODO.md`

---

## [v0.0.4] — 2026-03-18

### Added
- **LSP document synchronization**: `didOpen`, `didChange`, `didSave`, `didClose` notifications
- **LSP completions**: `textDocument/completion` request builder with full round-trip support
- `CompletionParams` construction with URI, position, and context
- Completion response parsing for `CompletionList` and `CompletionItem[]`
- `Buffer::apply_completion()` with `textEdit` range handling (delete range before insert)
- `Buffer::apply_text_edit()` for proper LSP range replacement
- Completion ranking: sort by `sort_text` (server intent), then label length, deterministic deduplication via `BTreeMap`
- `completion.rs` module for bridging LSP results to buffer
- rust-analyzer integration test for real completion requests
- Document sync test with Python mock server (gated by python3 availability)

### Fixed
- **textEdit range bug**: Completions now properly delete the specified range before inserting, preventing doubled text (e.g., "fn fo" + "fn foo" → "fn foo")
- Dead code shadow in `update_cursor` (removed variable re-declaration)
- Cursor positioning: clamp to last character index (line_len - 1), accounting for trailing newlines

### Changed
- Moved `rank_completions` from `LspClient` to `completion.rs` module (better organization)
- Completion ranking now respects `sort_text` field per LSP spec
- Updated Claude workflow to use `claude-code-action@v1` with custom prompt and model settings
- Expanded `CLAUDE.md` with full SDLC coverage, SemVer guidance, and deployment context

---

## [v0.0.3] — 2026-03-13

### Added
- LSP client lifecycle: spawn, shutdown, restart with backoff, stderr capture
- JSON-RPC transport over stdio with Content-Length framing, pending-response tracking, and notification broadcast
- Initialize handshake helper with stored `InitializeResult` and client capabilities
- WSL2 path normalization utilities (Windows ↔ `/mnt/<drive>`), plus round-trip tests and env-gated rust-analyzer integration scaffold
- CI installs `rust-analyzer` component; initialize→shutdown sequence test

### Changed
- rust-analyzer integration test now env-gated (`RUN_RA_INT=1`) instead of ignored

---

## [v0.0.2] — 2026-03-12

### Added
- `CompletionEngine` with FIM-aware prompt construction and stop-sequence truncation
- FIM token support: DeepSeek, StarCoder, CodeLlama variants with model-specific stop sequences
- DeepSeek stop sequences: `}\n`, `<｜fim▁end｜>`, `<｜end▁of▁sentence｜>`
- `FimTokens::for_model()` — auto-select FIM tokens based on model name
- Debounce logic with `CancellationToken` (configurable, default 300ms) in `CompletionHandler` and `ChatSession`
- Stop-sequence truncation via `truncate_on_stop()` (post-processing fallback)
- Streaming token callback on `ChatSession::send_streaming()`
- `Buffer::insert`, `Buffer::delete`, `Buffer::lines`, `Buffer::Display` trait
- Cursor position tracking in `Buffer`
- `Workspace::open_file`, `Workspace::save_file`
- File watcher with `notify-debouncer-mini` (100ms debounce)
- Unit tests for all buffer operations
- Live FIM completion integration test (`#[ignore]` — run with `cargo test -- --ignored`)
- `CompletionItem::label` truncation to first line for LSP menu rendering

### Fixed
- Ollama backend: `raw: true` to bypass chat template and preserve FIM tokens
- Ollama backend: `temperature: 0.0` for deterministic code completions
- `Buffer::lines()` strips trailing newlines (editor API convention)
- File watcher debounce prevents rapid-fire reindex on save
- Claude workflow: use `jq --rawfile` for prompt/diff to prevent injection

### Changed
- `ChatSession::send()` → `ChatSession::send_streaming()` with token callback
- `Buffer::to_string()` replaced with `Display` trait implementation

---

## [v0.0.1] — 2026-03-11

### Added
- Cargo workspace scaffold: `idep-core`, `idep-ai`, `idep-lsp`, `idep-plugin`, `idep-index`
- `LICENSE` — Apache 2.0
- `rust-toolchain.toml` — Rust edition pinned
- `CONTRIBUTING.md` — contributor guide
- `SECURITY.md` — local-first threat model, vulnerability reporting policy
- `SUSTAINABILITY.md` — contribution model and project pledge
- `config.example.toml` — full backend config reference (Ollama, Anthropic, HuggingFace, OpenAI-compat)
- `Config` struct with serde TOML deserialization and XDG path resolution
- `Backend` trait with `BackendInfo` struct and `info()` method for diagnostics
- Backend implementations: `OllamaBackend`, `AnthropicBackend`, `HuggingFaceBackend`, `OpenAiCompatBackend`
- Unit and integration tests for all four backends
- Retry logic with exponential backoff and rate-limit (429) handling — all backends
- Pre-commit hooks: `fmt`, `clippy`, `cargo test`
- CI workflow (GitHub Actions)

### Changed
- `README.md` — competitor table expanded with Google Antigravity column
- `README.md` — Positioning section added
- `README.md` — tagline updated to "Think in code. Own your tools."
- `README.md` — pre-alpha notice and status table added
- `TODO.md` — restructured by version milestone
