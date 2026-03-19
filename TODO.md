# Idep — Development TODO

> Track: https://github.com/idep-editor/idep
> Domain: idep.dev
> Tagline: *Think in code. Own your tools.*

---

## Version Map

| Phase | Range | Theme |
|---|---|---|
| **0.0.x** | 0.0.1 – 0.0.9 | Headless core |
| **0.1.x** | 0.1.0 – 0.1.5 | Terminal UI |
| **0.2.x** | 0.2.0 – 0.2.6 | GUI renderer |
| **0.3.x** | 0.3.0 – 0.3.4 | Plugin system |
| **0.4.x** | 0.4.0 – 0.4.4 | Config UX + distribution |
| **0.5.x** | 0.5.0 – 0.5.5 | Performance + stability |
| **0.6.x** | 0.6.0 – 0.6.4 | Security + trust |
| **0.7.x** | 0.7.0 – 0.7.5 | Documentation |
| **0.8.x** | 0.8.0 – 0.8.4 | Community infrastructure |
| **0.9.x** | 0.9.0 – 0.9.4 | Release candidates |
| **1.0.0** | — | Stable |

---

## Phase 0.0.x — Headless Core

> One subsystem per version. No UI. Every layer tested before the next begins.

---

### ✅ v0.0.1 — Repo Foundation
> **Gate:** `cargo build --all` passes · CI green

#### Repo hygiene
- [x] `cargo check --all` passes clean on main
- [x] `LICENSE` (Apache 2.0)
- [x] `CONTRIBUTING.md`
- [x] `CHANGELOG.md` (start at `v0.1.0-dev`)
- [x] `.gitignore` — `Cargo.lock` included (binary crate)
- [x] `rust-toolchain.toml` — Rust version pinned
- [x] `SUSTAINABILITY.md`
- [x] `SECURITY.md`

#### Workspace structure
- [x] `Cargo.toml` workspace with all member crates declared
- [x] `idep-core` crate skeleton
- [x] `idep-ai` crate skeleton
- [x] `idep-lsp` crate skeleton
- [x] `idep-plugin` crate skeleton
- [x] `idep-index` crate skeleton
- [x] All crates compile with empty `lib.rs`

#### Config schema
- [x] `config.example.toml` written and documented
- [x] `Config` struct with serde TOML deserialization
  - [x] `[ai].backend` — enum: `ollama | anthropic | huggingface | openai`
  - [x] `[ai].model` — string
  - [x] `[ai].endpoint` — optional URL override
  - [x] `[ai.auth].api_key` — optional, env var fallback `IDEP_API_KEY`
- [x] XDG config resolution: `~/.config/idep/config.toml` → `~/.idep/config.toml`
- [x] Unit test: config loads from file
- [x] Unit test: config falls back to env var for api_key
- [x] Unit test: missing optional fields use defaults

#### CI
- [x] GitHub Actions: `cargo check --all`
- [x] GitHub Actions: `cargo test --all`
- [x] GitHub Actions: `cargo clippy --all -- -D warnings`
- [x] GitHub Actions: `cargo fmt --all -- --check`
- [x] CI runs on push to `main` and on PRs

#### Website
- [x] `github.com/idep-editor/website` created
- [x] Deployed to `idep-website.vercel.app`
- [x] Landing page: tagline, comparison table, backend config switcher

---

### ✅ v0.0.2 — AI Backends + Buffer + Workspace
> **Gate:** Ollama completions working end-to-end

#### `idep-ai` — Backends
- [x] `Backend` trait defined: `send()`, `stream()`, `health_check()`
- [x] `OllamaBackend` implemented
  - [x] `/api/generate` POST
  - [x] `/api/tags` health check
  - [x] Streaming token callback
  - [x] Integration test (requires local Ollama)
- [x] `AnthropicBackend` implemented
  - [x] `/v1/messages` POST
  - [x] SSE streaming
  - [x] Unit test with mock HTTP server
- [x] `HuggingFaceBackend` implemented
  - [x] Inference API POST
  - [x] Unit test with mock HTTP server
- [x] `OpenAiCompatBackend` implemented
  - [x] `/v1/chat/completions` POST
  - [x] SSE streaming
  - [x] Unit test with mock HTTP server
- [x] Retry logic with exponential backoff — all backends
- [x] Configurable timeout per backend
- [x] 429 rate-limit handling with `Retry-After` respect

#### `idep-ai` — Completion
- [x] `CompletionEngine` struct
- [x] Wire to `llm-ls` LSP bridge
- [x] Debounce logic (configurable, default 300ms)
- [x] Stop-sequence handling
- [x] FIM token validation: DeepSeek · StarCoder · CodeLlama
- [x] Benchmark: keypress → first token latency logged

#### `idep-ai` — Chat
- [x] `ChatSession` struct: message history, context window
- [x] Streaming token callback restored to `send()`
- [x] Debounce wired through to chat context

#### `idep-core` — Buffer
- [x] `Buffer::insert(pos, text)`
- [x] `Buffer::delete(range)`
- [x] `Buffer::lines() -> impl Iterator`
- [x] `Buffer::to_string()`
- [x] Cursor position tracking
- [x] Unit tests for all buffer operations

#### `idep-core` — Workspace
- [x] `Workspace::open_file(path) -> Buffer`
- [x] `Workspace::save_file(path, buffer)`
- [x] File watcher (`notify` crate) → trigger reindex on save
- [x] Unit tests for workspace operations

#### WSL2 — File system
- [x] Verify `notify` inotify events fire correctly on native Linux paths (`~/`, `/home/...`)
- [x] Verify `notify` behavior on `/mnt/c/...` (DrvFs) paths — document known limitations
- [x] Path normalization: `WindowsPath` ↔ `LinuxPath` translation utility
- [x] Unit test: open file via `/mnt/c` path, save, verify watcher fires

---

### ✅ v0.0.3 — LSP Client Lifecycle (done)
> **Gate:** `initialize` → `initialized` → `shutdown` handshake completes cleanly against `rust-analyzer`

#### `idep-lsp` — Process management
- [x] Spawn language server subprocess (`Command` + stdio pipes)
- [x] Capture stdout/stderr separately
- [x] Graceful shutdown: send `shutdown` request, wait for response, send `exit` notification
- [x] Force-kill if shutdown times out (configurable, default 5s)
- [x] Restart policy: exponential backoff, max 3 retries
- [x] Unit test: mock LSP server, verify lifecycle sequence

#### `idep-lsp` — JSON-RPC transport
- [x] `JsonRpcTransport` struct: read/write over stdio
- [x] Content-Length header framing (LSP wire format)
- [x] Async read loop: deserialize incoming messages
- [x] Outgoing message queue: serialize + write
- [x] Request ID tracking: match responses to pending requests
- [x] Notification dispatch: fire-and-forget incoming notifications
- [x] Unit test: round-trip a request/response pair
- [x] Unit test: handle malformed message gracefully

#### `idep-lsp` — `initialize` handshake
- [x] Build `InitializeParams` with client capabilities
- [x] Send `initialize` request
- [x] Receive and store `InitializeResult` (server capabilities)
- [x] Send `initialized` notification
- [x] Store negotiated capabilities for downstream use
- [x] Integration test: full handshake with `rust-analyzer`

#### WSL2 — LSP path handling
- [x] URI normalization: convert `file:///mnt/c/...` URIs to WSL-native paths before sending to LSP server
- [x] URI normalization: convert LSP server responses back to idep-internal paths
- [x] Unit test: round-trip path conversion for Windows-style and Linux-style paths
- [x] Integration test: `rust-analyzer` started from WSL2, resolves definition across path boundary

#### CI gate
- [x] Integration test runs `rust-analyzer` in CI (install via `rustup component add rust-analyzer`)
- [x] Test: initialize → shutdown sequence passes

---

### ✅ v0.0.4 — LSP Completions in Buffer
> **Gate:** `textDocument/completion` request round-trips and results land in buffer

#### `idep-lsp` — Document sync
- [x] `textDocument/didOpen` — send on buffer open
- [x] `textDocument/didChange` — send on buffer mutation (incremental sync)
- [x] `textDocument/didSave` — send on file save
- [x] `textDocument/didClose` — send on buffer close
- [x] Unit test: verify correct notification sequence on open → edit → save → close

#### `idep-lsp` — Completions
- [x] `textDocument/completion` request builder
- [x] `CompletionParams`: URI, position, context
- [x] Parse `CompletionList` / `CompletionItem[]` response
- [x] Filter and rank completion items
- [x] Bridge completion results to `idep-core` buffer
- [x] Unit test: mock server returns completions, verify items parsed
- [x] Integration test: get completions from `rust-analyzer` on a real `.rs` file

#### `idep-core` — Completion insertion
- [x] `Buffer::apply_completion(item)` — insert text at cursor
- [x] Handle `insertText` vs `textEdit` completion item kinds
- [x] Unit test: completion applied correctly at various cursor positions

---

### 🔴 v0.0.5 — LSP Diagnostics, Hover, Goto
> **Gate:** Diagnostics received, hover response parsed, goto-definition resolves a location

#### `idep-lsp` — Diagnostics
- [x] Handle `textDocument/publishDiagnostics` notification
- [x] Parse `Diagnostic[]`: range, severity, message, code
- [x] Store diagnostics per document URI
- [x] Expose `get_diagnostics(uri) -> Vec<Diagnostic>` API
- [x] Clear stale diagnostics on `didClose`
- [x] Unit test: mock server publishes diagnostics, verify stored correctly
- [ ] Integration test: open file with intentional error, verify diagnostic received

#### `idep-lsp` — Hover
- [x] `textDocument/hover` request builder
- [x] Parse `Hover` response: `MarkupContent` (plaintext or markdown)
- [x] Return `Option<String>` hover text
- [x] Unit test: mock hover response parsed correctly
- [ ] Integration test: hover over a Rust symbol, get type info

#### `idep-lsp` — Goto Definition
- [x] `textDocument/definition` request builder
- [x] Parse `Location` / `LocationLink[]` response
- [x] Return `Vec<Location>`: URI + range
- [x] Unit test: mock response parsed correctly
- [ ] Integration test: goto definition resolves to correct file + line

---

### 🔴 v0.0.6 — Tree-sitter AST Chunking
> **Gate:** Rust, TypeScript, and Python source files chunked into labeled AST nodes with correct spans

#### `idep-ai` — Tree-sitter integration
- [ ] Add `tree-sitter` + language grammars to `idep-ai` deps
  - [ ] `tree-sitter-rust`
  - [ ] `tree-sitter-typescript`
  - [ ] `tree-sitter-python`
- [ ] `AstChunker` struct: takes source text + language, returns `Vec<Chunk>`
- [ ] `Chunk` type: `{ kind, name, start_byte, end_byte, text }`

#### Rust chunking
- [ ] Extract `fn` items (name, signature, body)
- [ ] Extract `impl` blocks (type name, methods)
- [ ] Extract `struct` definitions
- [ ] Extract `trait` definitions
- [ ] Extract `enum` definitions
- [ ] Unit test: chunk a real Rust file, verify expected nodes extracted

#### TypeScript chunking
- [ ] Extract `function` declarations
- [ ] Extract `class` declarations
- [ ] Extract `interface` declarations
- [ ] Extract `type` alias declarations
- [ ] Unit test: chunk a real TS file, verify expected nodes extracted

#### Python chunking
- [ ] Extract `def` functions
- [ ] Extract `class` definitions
- [ ] Unit test: chunk a real Python file, verify expected nodes extracted

#### General
- [ ] Graceful fallback: unknown language → naive line chunking (configurable chunk size)
- [ ] Respect max chunk size (configurable, default 512 tokens estimated)
- [ ] Split oversized nodes at logical boundaries
- [ ] Unit test: oversized function split correctly

---

### 🔴 v0.0.7 — Local Embeddings Pipeline
> **Gate:** `fastembed-rs` produces embeddings for a batch of chunks, no network calls, latency benchmarked

#### `idep-index` — Embedder
- [ ] Add `fastembed` crate to `idep-index` deps
- [ ] `Embedder` struct: wraps fastembed model
- [ ] Model download on first run → cached to `~/.idep/models/`
- [ ] `Embedder::embed_batch(texts: &[&str]) -> Vec<Vec<f32>>`
- [ ] Embedding dimension asserted (e.g. 384 for `all-MiniLM-L6-v2`)
- [ ] Unit test: embed a batch of 10 strings, verify shape
- [ ] Benchmark: time to embed 100 chunks of ~200 tokens each
- [ ] Verify: no outbound network calls during embed (network blocked in test)

#### `idep-index` — Pipeline wiring
- [ ] `EmbedPipeline::run(chunks: Vec<Chunk>) -> Vec<EmbeddedChunk>`
- [ ] `EmbeddedChunk`: original `Chunk` + `Vec<f32>` embedding
- [ ] Batch size configurable (default 32)
- [ ] Progress callback for large batches
- [ ] Unit test: pipeline produces one embedding per chunk

---

### 🔴 v0.0.8 — Vector Index + Query
> **Gate:** `usearch` index persists to disk, survives restart, `find_similar()` returns correct top-k results

#### `idep-index` — Vector store
- [ ] Add `usearch` crate to `idep-index` deps
- [ ] `VectorStore` struct: wraps usearch index
- [ ] `VectorStore::add(id, embedding)`
- [ ] `VectorStore::find_similar(embedding, top_k) -> Vec<ScoredChunk>`
- [ ] `ScoredChunk`: chunk ID + cosine similarity score
- [ ] `VectorStore::save(path)` — persist index to disk
- [ ] `VectorStore::load(path)` — restore index from disk
- [ ] Unit test: add 50 embeddings, query, verify top-1 is self
- [ ] Unit test: save → load → query returns same results

#### `idep-index` — Chunk metadata store
- [ ] Persist chunk metadata alongside vector index (serde + bincode or sled)
- [ ] `ChunkStore::get(id) -> Option<Chunk>`
- [ ] `ChunkStore::insert(chunk) -> id`
- [ ] `ChunkStore::delete(id)`
- [ ] Unit test: round-trip chunk metadata through store

#### `idep-index` — Project indexer
- [ ] `Indexer::index_project(root: &Path)`
  - [ ] Walk directory tree (respect `.gitignore` via `ignore` crate)
  - [ ] Detect language per file extension
  - [ ] Chunk each file via `AstChunker`
  - [ ] Embed chunks via `EmbedPipeline`
  - [ ] Store in `VectorStore` + `ChunkStore`
- [ ] `Indexer::reindex_file(path: &Path)` — diff-based, not full re-walk
  - [ ] Remove old chunks for this file
  - [ ] Re-chunk, re-embed, re-insert
- [ ] Index stored at `~/.idep/index/<project-hash>/`
- [ ] Unit test: index a small test project, verify chunk count
- [ ] Benchmark: index 50k LOC Rust project, measure total time

---

### 🔴 v0.0.9 — RAG Context Injection
> **Gate:** Chat responses demonstrably reference correct codebase chunks; verified by test

#### `idep-ai` — Context engine
- [ ] `ContextEngine` struct: takes query, workspace root, returns `Vec<Chunk>`
- [ ] `ContextEngine::gather(query: &str, cursor_file: &Path, cursor_pos: Position) -> Context`
  - [ ] Current file content (always included)
  - [ ] AST subtree around cursor (Tree-sitter)
  - [ ] Top-k similar chunks from vector index
  - [ ] Recent edit history (last N saves)
- [ ] Context serializer: format chunks into prompt-friendly text block
- [ ] Token budget manager: fit context within model's context window
  - [ ] Configurable max context tokens (default 4096)
  - [ ] Priority order: cursor context > similar chunks > history
  - [ ] Truncate lower-priority sections first

#### `idep-ai` — Chat with RAG
- [ ] `ChatSession::send_with_context(query, context)` — prepend context block to message
- [ ] `ChatSession::export()` — serialize history to JSON
- [ ] Unit test: context block injected correctly into prompt
- [ ] Integration test: ask question about a specific function → response references it

#### `idep-ai` — Anthropic native message format
- [ ] Replace `build_prompt()` with native `messages` array for Anthropic
- [ ] Context window management: truncate history near token limit
- [ ] Unit test: message array built correctly for multi-turn conversation

---

## Phase 0.1.x — Terminal UI

> Prove the engine through a thin, usable interface. Force the engine API to be clean.

---

### 🔴 v0.1.0 — Basic TUI Editor
> **Gate:** Open a file, move cursor, insert and delete text, save — all in terminal

#### `idep-tui` crate
- [ ] Add `idep-tui` to workspace
- [ ] Add `ratatui` + `crossterm` deps
- [ ] `App` struct: holds active buffer, cursor, mode (Normal/Insert)
- [ ] Main event loop: read input → dispatch → render
- [ ] Render: file content with line numbers
- [ ] Render: status bar (filename, mode, cursor position, modified flag)

#### Input handling
- [ ] Normal mode: `h/j/k/l` cursor movement
- [ ] Normal mode: `i` → Insert, `Esc` → Normal
- [ ] Normal mode: `w/b` word movement
- [ ] Normal mode: `0/$` line start/end
- [ ] Normal mode: `gg/G` file start/end
- [ ] Normal mode: `dd` delete line
- [ ] Normal mode: `u` undo, `Ctrl+r` redo
- [ ] Insert mode: printable character insertion
- [ ] Insert mode: `Backspace` / `Delete`
- [ ] Insert mode: `Enter` (newline)
- [ ] Insert mode: `Ctrl+s` save file
- [ ] Command mode: `:w` save, `:q` quit, `:wq` save+quit, `:q!` force quit

#### WSL2 — TUI platform verification
- [ ] Test: TUI renders correctly in Windows Terminal (WSL2 backend)
- [ ] Test: TUI renders correctly in Windows Terminal Preview
- [ ] Test: color output correct (256-color and truecolor)
- [ ] Test: Unicode / box-drawing characters render without artifacts
- [ ] Test: mouse support works in Windows Terminal
- [ ] Test: `Ctrl+C` / `Ctrl+Z` signals handled correctly across WSL2 boundary
- [ ] Document: minimum Windows Terminal version required

#### Buffer ↔ TUI bridge
- [ ] `idep-core::Buffer` used as single source of truth
- [ ] Undo/redo history in buffer (configurable depth, default 100)
- [ ] Unit test: undo/redo sequence correct after series of edits

---

### 🔴 v0.1.1 — Syntax Highlighting in TUI
> **Gate:** Rust file opens with correct token-level highlighting

- [ ] Wire `tree-sitter` highlight queries into TUI renderer
- [ ] `Highlighter` struct: takes buffer text + language, returns `Vec<HighlightedSpan>`
- [ ] `HighlightedSpan`: byte range + highlight name (e.g. `keyword`, `string`, `comment`)
- [ ] Color mapping: highlight name → `ratatui` `Style`
- [ ] Default theme: dark background, readable contrast
- [ ] Supported languages: Rust, TypeScript, Python, TOML, Markdown
- [ ] Graceful fallback: unknown language → no highlighting, no crash
- [ ] Unit test: Rust snippet produces expected highlight spans
- [ ] Performance: no perceptible lag on files up to 10k lines

---

### 🔴 v0.1.2 — LSP Diagnostics in TUI
> **Gate:** Save a Rust file with an error; diagnostic appears in status bar within 2 seconds

- [ ] Connect `idep-lsp` to TUI `App`
- [ ] Start LSP server on workspace open
- [ ] Send `didOpen` for initial file
- [ ] Send `didChange` on buffer mutation (debounced, 500ms)
- [ ] Send `didSave` on `:w`
- [ ] Receive `publishDiagnostics` → store on `App`
- [ ] Render: error/warning count in status bar
- [ ] Render: inline diagnostic markers at end of affected lines
- [ ] Render: diagnostic detail panel (toggle with configurable key, default `Space+d`)
- [ ] Integration test: open file with error, verify diagnostic displayed

---

### 🔴 v0.1.3 — Inline AI Completions in TUI
> **Gate:** Pause typing in Insert mode; ghost text suggestion appears; `Tab` accepts it

- [ ] Connect `idep-ai` `CompletionEngine` to TUI
- [ ] Trigger: debounce keypress in Insert mode (configurable, default 400ms)
- [ ] Fetch completion from configured backend (Ollama for offline default)
- [ ] Render ghost text: dimmed style, appended after cursor
- [ ] `Tab` → accept ghost text (insert into buffer)
- [ ] `Esc` / any other key → dismiss ghost text
- [ ] Cancel in-flight request if user resumes typing before response arrives
- [ ] Show spinner in status bar while fetching
- [ ] Unit test: ghost text rendered at correct position
- [ ] Integration test: completion fetched and accepted end-to-end

---

### 🔴 v0.1.4 — AI Chat Panel in TUI
> **Gate:** Ask a question about the open file; streaming response appears in split pane

- [ ] Split layout: editor pane (left/top) + chat pane (right/bottom)
- [ ] Toggle chat pane: configurable key (default `Space+c`)
- [ ] Chat input box at bottom of chat pane
- [ ] Send message: `Enter`
- [ ] Streaming response: tokens appear as they arrive (no waiting for full response)
- [ ] Scroll history: `j/k` or mouse scroll in chat pane
- [ ] Context injection: current file + cursor-adjacent AST chunk attached automatically
- [ ] Show token count of injected context in chat pane header
- [ ] Clear chat history: configurable key (default `Space+x`)
- [ ] Integration test: send question, verify streaming response received

---

### 🔴 v0.1.5 — File Tree + Multi-Buffer
> **Gate:** Navigate project files in tree panel; open multiple buffers; switch between them

- [ ] File tree panel (toggle: configurable key, default `Space+e`)
- [ ] Tree render: directory structure, current file highlighted
- [ ] Tree navigation: `j/k` move, `Enter` open file, `l` expand dir, `h` collapse dir
- [ ] Multi-buffer: `App` holds `Vec<Buffer>` with active index
- [ ] Buffer switcher: configurable key (default `Space+b`) → list open buffers
- [ ] Next/prev buffer: configurable keys (default `]b` / `[b`)
- [ ] Close buffer: configurable key (default `Space+q`) — prompt if unsaved
- [ ] Status bar: shows buffer index and total (e.g. `[2/4]`)
- [ ] Unit test: open 3 buffers, close middle one, verify remaining order correct

---

## Phase 0.2.x — GUI Renderer

> Spike both options before committing. One week per spike, hard timebox.

---

### 🔴 v0.2.0 — egui Spike
> **Gate:** Basic text editing view renders at 60fps; decision criteria documented

- [ ] `idep-gui` crate skeleton with `egui` + `eframe` deps
- [ ] Open file → render buffer content in egui text area
- [ ] Cursor visible and moves with arrow keys
- [ ] Character insertion and deletion works
- [ ] No perceptible frame drop on 5k line file
- [ ] Measure: frame time, memory use, startup time
- [ ] WSL2: verify renders via WSLg (Windows 11) without explicit `DISPLAY` config
- [ ] WSL2: verify renders via X11 forwarding (Windows 10 + VcXsrv / X410)
- [ ] WSL2: document minimum WSL version and WSLg availability requirement
- [ ] Write `docs/renderer-spike-egui.md`: findings, pros, cons, blockers, WSL2 notes

---

### 🔴 v0.2.1 — wgpu Text Renderer Spike
> **Gate:** Custom glyph rendering via wgpu at 60fps; decision criteria documented

- [ ] `wgpu` + `glyphon` (or `cosmic-text`) deps in `idep-gui`
- [ ] Window setup, swap chain, render loop
- [ ] Font loading (Geist Mono from local file)
- [ ] Render a page of monospace text with correct line spacing
- [ ] Cursor rectangle rendered at correct position
- [ ] Measure: frame time, memory use, startup time, implementation complexity
- [ ] Write `docs/renderer-spike-wgpu.md`: findings, pros, cons, blockers, WSL2 notes

#### WSL2 — wgpu considerations
- [ ] Verify wgpu Vulkan backend initializes under WSLg
- [ ] Verify wgpu DX12 backend (via WSL2 GPU paravirtualization) if Vulkan unavailable
- [ ] Measure: GPU memory overhead under WSL2 vs native Linux
- [ ] Document: minimum WSL2 kernel version for GPU support (`5.10.43.3+`)

---

### 🔴 v0.2.2 — Renderer Decision + Basic Editing View
> **Gate:** Chosen renderer committed; file opens, cursor moves, text edits work; decision documented

#### Decision
- [ ] Write `docs/renderer-decision.md`: comparison table from spikes, rationale, final choice
- [ ] Delete losing spike branch / archive code with note in `docs/renderer-decision.md`
- [ ] Update `Cargo.toml` to remove unused renderer deps

#### Basic editing
- [ ] Port `idep-core::Buffer` as single source of truth (no re-implementation)
- [ ] Port TUI input handling to GUI action dispatch (reuse same action enum)
- [ ] Line numbers gutter: fixed-width column, correct width for file line count
- [ ] Cursor: blinking block (normal mode), beam (insert mode)
- [ ] Cursor moves correctly: `h/j/k/l`, arrow keys, `w/b`, `0/$`, `gg/G`
- [ ] Character insertion and deletion at cursor position
- [ ] Newline insertion: `Enter` splits line at cursor
- [ ] `Ctrl+s` save; prompt on unsaved close

#### Viewport
- [ ] Vertical scroll: cursor always visible (scroll follows cursor)
- [ ] Horizontal scroll: for lines wider than viewport
- [ ] Page up / page down: `Ctrl+u` / `Ctrl+d`
- [ ] Scroll position preserved on window resize
- [ ] Window resize recalculates layout without crash

#### Status bar
- [ ] Filename (relative to workspace root)
- [ ] Current mode (Normal / Insert)
- [ ] Cursor line:col
- [ ] Modified indicator (`[+]`)
- [ ] Backend name + model (e.g. `ollama/codellama:13b`)

#### Tests
- [ ] Unit test: cursor movement stays within buffer bounds
- [ ] Unit test: insert + delete round-trip produces original content
- [ ] Integration test: open file, edit, save, reopen → content persisted

---

### 🔴 v0.2.3 — Syntax Highlighting in GUI
> **Gate:** Rust file renders with correct highlighting; no frame drop on 10k line file

#### Highlight rendering
- [ ] Reuse `Highlighter` from `v0.1.1` — no duplication of tree-sitter logic
- [ ] Map highlight spans → colored glyph runs in chosen renderer
- [ ] Verify byte-range → glyph-index mapping is correct for multibyte UTF-8
- [ ] Verify highlights do not bleed across line boundaries
- [ ] Handle files with mixed CRLF/LF line endings without highlight offset errors

#### Theme system
- [ ] `Theme` struct: map of highlight name → RGBA color
- [ ] Load theme from `~/.config/idep/themes/<name>.toml`
- [ ] Built-in default dark theme (matches `--idep-bg` / `--idep-accent` brand colors)
- [ ] Built-in default light theme
- [ ] `[editor].theme` config key: name of active theme
- [ ] Graceful fallback: unknown theme name → default dark, log warning
- [ ] Hot-reload: file watcher on active theme file → re-render on change

#### Performance
- [ ] Incremental highlight: only re-run tree-sitter on changed region, not whole file
- [ ] Highlight computed off render thread; apply result on next frame
- [ ] Benchmark: highlight latency on 10k line Rust file < 16ms
- [ ] Unit test: highlight spans produce expected color sequence for Rust snippet
- [ ] Unit test: theme hot-reload applies without restart

---

### 🔴 v0.2.4 — LSP Diagnostics Gutter in GUI
> **Gate:** Error squiggle appears under affected code within 2s of save; hover shows message

#### Gutter
- [ ] Reuse `idep-lsp` diagnostics from `v0.1.2` — no re-implementation
- [ ] Gutter column: fixed-width strip left of line numbers
- [ ] Error icon (●) and warning icon (◆) rendered per affected line
- [ ] Multiple diagnostics on same line: show highest severity icon
- [ ] Gutter icon click → jump cursor to diagnostic start position
- [ ] Gutter renders correctly after scroll, resize, and buffer edit

#### Inline squiggle
- [ ] Squiggle underline rendered under affected byte range
- [ ] Error severity → red squiggle; warning → yellow; hint → grey
- [ ] Squiggle positions recalculated on buffer edit (shift ranges correctly)
- [ ] Squiggle not rendered on folded lines

#### Hover tooltip
- [ ] Mouse hover over squiggle range → show tooltip after 300ms delay
- [ ] Tooltip content: severity label + diagnostic message + optional code
- [ ] Tooltip dismisses on mouse-out or keypress
- [ ] Tooltip does not overflow viewport (flip above/below as needed)

#### Diagnostics panel
- [ ] Panel toggle: configurable key (default `Space+d`)
- [ ] Panel lists all diagnostics: file, line, col, severity, message
- [ ] Click row → open file, jump to line
- [ ] Sort by: severity (default), file, line
- [ ] Filter by: error only / warning only / all
- [ ] Badge in panel toggle button shows error count

#### Tests
- [ ] Integration test: introduce error, save, verify gutter icon appears within 2s
- [ ] Integration test: fix error, save, verify gutter icon disappears
- [ ] Unit test: diagnostic range shift correct after insert before affected line

---

### 🔴 v0.2.5 — AI Chat Panel in GUI
> **Gate:** Chat panel opens; streaming response renders token-by-token; context injection confirmed

#### Panel layout
- [ ] Chat panel widget: dockable right or bottom (user preference)
- [ ] Panel resizable via drag handle
- [ ] Panel toggle: configurable key (default `Space+c`)
- [ ] Panel persists open/closed state across sessions
- [ ] Panel width/height persisted in `~/.config/idep/ui-state.toml`

#### Message rendering
- [ ] Message history: scrollable list, newest at bottom
- [ ] User messages: right-aligned or visually distinct label
- [ ] Assistant messages: left-aligned with model name label
- [ ] Streaming render: tokens appended to last assistant message as they arrive
- [ ] Cursor/spinner shown at end of in-progress assistant message
- [ ] Markdown rendering in assistant messages:
  - [ ] Fenced code blocks with syntax highlighting
  - [ ] Inline `code` spans
  - [ ] Bold and italic text
  - [ ] Bullet and numbered lists
  - [ ] Horizontal rules

#### Input
- [ ] Input text field at bottom of panel
- [ ] `Enter` sends message; `Shift+Enter` inserts newline
- [ ] Input clears after send
- [ ] Input disabled while response is streaming
- [ ] `Esc` cancels in-flight request

#### Context injection
- [ ] Context indicator bar above input: shows which chunks are attached
- [ ] Click chunk label → show full chunk text in tooltip
- [ ] Token count shown: "Context: 1,240 / 4,096 tokens"
- [ ] Manual attach: drag file from file tree into chat input

#### History management
- [ ] Scroll to top loads earlier messages (virtual list, not full re-render)
- [ ] Clear chat history: configurable key (default `Space+x`) + confirmation prompt
- [ ] Export chat: save to `~/.idep/chats/<timestamp>.json`
- [ ] `ChatSession::export()` format: array of `{role, content, timestamp}`

#### Tests
- [ ] Integration test: send message, verify streaming response appears token-by-token
- [ ] Unit test: markdown code block rendered with correct highlight spans
- [ ] Unit test: context token count computed correctly

---

### 🔴 v0.2.6 — Inline Completion Ghost Text in GUI
> **Gate:** Ghost text appears after typing pause; `Tab` accepts; `Esc` dismisses; no flicker

#### Ghost text rendering
- [ ] Reuse `CompletionEngine` from `v0.1.3` — no re-implementation
- [ ] Ghost text rendered as dimmed glyph run inline after cursor position
- [ ] Ghost text color: 40% opacity of `--idep-fg-muted` (not full foreground)
- [ ] Ghost text rendered on same line as cursor; multi-line ghost text supported
- [ ] Ghost text does not shift existing text or affect layout
- [ ] Ghost text cleared immediately on any editing keystroke

#### Trigger and lifecycle
- [ ] Trigger: typing pauses for configurable duration (default 400ms)
- [ ] Trigger: only in insert/edit mode, not in read-only buffers
- [ ] In-flight request cancelled immediately if user resumes typing
- [ ] New request debounced from resumed typing pause
- [ ] No ghost text shown if completion is identical to existing text ahead of cursor
- [ ] Loading indicator: subtle spinner in gutter while request is in-flight

#### Accept / dismiss
- [ ] `Tab` → accept full ghost text (insert into buffer, clear ghost)
- [ ] `Ctrl+Right` → accept next word of ghost text only
- [ ] `Esc` → dismiss ghost text without inserting
- [ ] Any other key → dismiss ghost text, process key normally
- [ ] Accepted text added to undo history as single undo step

#### Tests
- [ ] Unit test: ghost text rendered at correct glyph position after multibyte characters
- [ ] Unit test: partial accept (`Ctrl+Right`) inserts correct word boundary
- [ ] Unit test: in-flight request cancelled on keystroke before response arrives
- [ ] Integration test: completion fetched, ghost text shown, `Tab` accepts end-to-end

---

## Phase 0.3.x — Plugin System

---

### 🔴 v0.3.0 — Plugin API Surface v1
> **Gate:** API surface reviewed, documented, and frozen — no breaking changes after this point

#### API trait definition
- [ ] Define `Plugin` trait in `idep-plugin/src/api.rs`
  - [ ] `fn on_file_open(path: &str, content: &str)`
  - [ ] `fn on_file_save(path: &str, content: &str)`
  - [ ] `fn on_file_close(path: &str)`
  - [ ] `fn on_cursor_move(path: &str, line: u32, col: u32)`
  - [ ] `fn provide_completions(ctx: CompletionContext) -> Vec<Completion>`
  - [ ] `fn register_command(name: &str, handler: fn())`
  - [ ] `fn open_panel(title: &str, html_content: &str)`
- [ ] All methods have default no-op implementations (plugins only override what they need)

#### Type definitions
- [ ] `CompletionContext`: `{ path, line, col, prefix, suffix, language }`
- [ ] `Completion`: `{ label, insert_text, detail, kind }`
- [ ] `CompletionKind`: enum `{ Function, Variable, Type, Keyword, Snippet }`
- [ ] `PluginMeta`: `{ name, version, description, author }` — read from `plugin.toml`
- [ ] All types `#[repr(C)]` or fully serialized across WASM boundary (no raw pointers)
- [ ] All types implement `serde::Serialize` + `serde::Deserialize`

#### Host API (functions plugins can call)
- [ ] `idep_log(level: LogLevel, msg: &str)` — write to idep log
- [ ] `idep_get_selection() -> Option<String>` — get current editor selection
- [ ] `idep_insert_text(text: &str)` — insert at cursor
- [ ] `idep_show_notification(msg: &str, level: NotifLevel)` — status bar notification
- [ ] `idep_read_config(key: &str) -> Option<String>` — read from plugin's own config namespace

#### Stability contract
- [ ] ABI stability review: all types verified safe across WASM boundary
- [ ] `#[deprecated]` path documented: how to signal future deprecation without breaking
- [ ] Semantic versioning policy written: what constitutes a breaking change
- [ ] Write `docs/plugin-api-v1.md`: full reference with every type and method
- [ ] Tag commit as `plugin-api-v1-frozen`
- [ ] CI check: any change to `idep-plugin/src/api.rs` after this tag requires explicit override

#### Inter-plugin messaging schema (reserved, not yet implemented)
- [ ] Reserve `[messaging]` section in `plugin.toml` manifest schema
  - [ ] `subscribes_to`: array of event topic strings (e.g. `["file.save", "completion.accepted"]`)
  - [ ] `publishes`: array of event topic strings (e.g. `["lint.result"]`)
- [ ] Schema validation: accept and store `[messaging]` fields, but do not wire a bus yet
- [ ] Document in `docs/plugin-api-v1.md`: "reserved for future inter-plugin event bus — fields are parsed and validated but not dispatched in v0.3.0"
- [ ] Unit test: `plugin.toml` with `[messaging]` section parses without error
- [ ] Unit test: `plugin.toml` without `[messaging]` section still parses (backward compat)

---

### 🔴 v0.3.1 — WASM Host
> **Gate:** Load a compiled `.wasm` plugin; call `on_file_open`; verify it executes sandboxed

#### Host setup
- [ ] Add `wasmtime` dep to `idep-plugin`
- [ ] `PluginHost` struct: manages a collection of loaded plugins
- [ ] `PluginHost::load(path: &Path) -> Result<PluginId>` — load single `.wasm`
- [ ] `PluginHost::unload(id: PluginId)` — drop instance, release memory
- [ ] `PluginHost::reload(id: PluginId)` — unload + re-load from same path
- [ ] Plugin registry: `HashMap<PluginId, PluginInstance>`

#### Sandbox enforcement
- [ ] Sandbox limits declared at instantiation: no filesystem, no network, no threading
- [ ] Memory limit per plugin (configurable, default 64MB) — `wasmtime::StoreLimits`
- [ ] CPU time limit per plugin call (configurable, default 1s) — fuel-based metering
- [ ] Verify: plugin cannot import `wasi:filesystem` or `wasi:sockets`
- [ ] Verify: plugin attempting file access → trap caught, plugin continues running

#### Host ↔ plugin interface
- [ ] Host exports `idep_log(msg_ptr, msg_len)` — plugin can write to idep log
- [ ] Host exports `idep_get_config(key_ptr, key_len, out_ptr, out_len) -> i32`
- [ ] All pointers validated before dereference (bounds check against plugin memory)
- [ ] Shared memory buffer: single allocation, passed by offset + length, not raw pointer

#### Lifecycle
- [ ] Call `on_file_open(path_ptr, path_len, content_ptr, content_len)` on buffer open
- [ ] Call `on_file_save(path_ptr, path_len, content_ptr, content_len)` on `:w`
- [ ] Plugin panic (WASM trap) → caught, logged with plugin name, plugin marked failed
- [ ] Failed plugin: skip future calls, do not crash host
- [ ] Unit test: load test WASM, call `on_file_open`, verify log output received
- [ ] Unit test: plugin that traps → host continues, other plugins unaffected

---

### 🔴 v0.3.2 — Rust Plugin SDK
> **Gate:** A Rust plugin compiles to WASM, loads in the host, and calls host functions correctly

#### `idep-plugin-sdk` crate
- [ ] New crate: `idep-plugin-sdk` (separate from `idep-plugin` host)
- [ ] `wasm32-unknown-unknown` target only (no std dependencies that require OS)
- [ ] `#[idep::plugin]` proc-macro: generates required WASM export symbols
- [ ] SDK types: `CompletionContext`, `Completion`, `FileEvent` — mirror host types exactly
- [ ] `idep::log(msg: &str)` — calls host `idep_log` import
- [ ] `idep::get_config(key: &str) -> Option<String>` — calls host `idep_get_config`

#### Trait surface
- [ ] `Plugin` trait with default no-op implementations for all hooks:
  - [ ] `fn on_file_open(&mut self, path: &str, content: &str)`
  - [ ] `fn on_file_save(&mut self, path: &str, content: &str)`
  - [ ] `fn provide_completions(&mut self, ctx: &CompletionContext) -> Vec<Completion>`
- [ ] `register_command(name: &str, handler: fn())` — registers in host command palette

#### Build verification
- [ ] `cargo build --target wasm32-unknown-unknown` produces valid `.wasm`
- [ ] Output `.wasm` size < 500KB for minimal plugin (no bloat check)
- [ ] `wasm-opt -O2` post-processing step documented in SDK readme

#### Example plugins
- [ ] `examples/hello-world/` — logs file path on open, registers `hello` command
- [ ] `examples/word-count/` — counts words on save, logs result
- [ ] `examples/custom-completion/` — provides static completion items
- [ ] All three: compile, load in host, execute correct callback without error

#### Tests
- [ ] Unit test: proc-macro generates correct WASM export symbol names
- [ ] Integration test: load each example plugin, trigger each hook, verify behavior

---

### 🔴 v0.3.3 — TypeScript Plugin SDK
> **Gate:** A TypeScript plugin compiles to WASM and loads in the host without error

#### Package
- [ ] `idep-plugin-ts` npm package (in `sdk/typescript/` directory)
- [ ] TypeScript type definitions mirroring Rust SDK types exactly:
  - [ ] `CompletionContext`, `Completion`, `FileEvent`
  - [ ] `Plugin` interface with all hook signatures
- [ ] `idep.log(msg: string): void` — binds to host `idep_log`
- [ ] `idep.getConfig(key: string): string | null` — binds to host `idep_get_config`
- [ ] `registerPlugin(plugin: Plugin): void` — entry point macro

#### Build toolchain
- [ ] Evaluate: `wasm-pack` + `wasm-bindgen` vs raw `AssemblyScript`
- [ ] Document choice with rationale in `docs/plugin-ts-sdk.md`
- [ ] `npm run build` produces `dist/plugin.wasm`
- [ ] `tsconfig.json` configured for WASM target
- [ ] `package.json` scripts: `build`, `test`, `example`

#### Example plugin
- [ ] `examples/hello-world-ts/` — logs file path on open
- [ ] `examples/word-count-ts/` — counts words on save
- [ ] Both compile and load in Rust host without error

#### Developer experience
- [ ] `docs/plugin-ts-quickstart.md` — install, write, build, load in 5 steps
- [ ] Source maps generated for debuggability
- [ ] TypeScript strict mode enabled (`"strict": true`)
- [ ] All types exported from package index

---

### 🔴 v0.3.4 — Plugin Authoring Docs + Discovery
> **Gate:** A new contributor can write, build, and load a plugin following the docs alone

- [ ] `docs/plugin-authoring-rust.md`: step-by-step guide
- [ ] `docs/plugin-authoring-typescript.md`: step-by-step guide
- [ ] Plugin manifest format: `plugin.toml` (name, version, entry point)
- [ ] Plugin loader: scan `~/.config/idep/plugins/` on startup
- [ ] CLI: `idep plugin install <path>` — copy to plugins dir
- [ ] CLI: `idep plugin list` — list loaded plugins
- [ ] CLI: `idep plugin disable <name>` — move to disabled dir

#### Plugin discoverability via RAG
- [ ] On plugin load: index `plugin.toml` metadata (name, version, description, hook list) into `idep-index` vector store
- [ ] On plugin unload: remove plugin metadata chunks from vector store
- [ ] `ContextEngine::gather()` includes plugin capability chunks when query matches plugin description
- [ ] Chat panel can answer "what plugins are installed?" and "can any plugin do X?" from indexed context
- [ ] Unit test: install plugin → query vector store → plugin metadata chunk returned
- [ ] Unit test: uninstall plugin → query vector store → plugin metadata chunk absent

---

## Phase 0.4.x — Config UX + Distribution

---

### 🔴 v0.4.0 — Config Validation
> **Gate:** Every invalid config variant produces a clear, actionable error message; zero panics on bad input

#### Structural validation
- [ ] Validate all required fields present — error names missing field explicitly
- [ ] Validate `backend` enum value is one of `ollama | anthropic | huggingface | openai`
- [ ] Validate `endpoint` is a valid URL if present (scheme must be `http` or `https`)
- [ ] Validate `model` is non-empty string
- [ ] Validate `api_key` present when backend requires it (`anthropic`, `huggingface`, `openai`)
- [ ] Validate `timeout_ms` > 0 if present
- [ ] Validate `debounce_ms` > 0 if present
- [ ] Validate `max_context_tokens` > 0 and ≤ 200000 if present

#### Error message quality
- [ ] Each error message includes: field path, what was wrong, how to fix it
- [ ] Example: `[ai].endpoint: "ftp://localhost" is not a valid HTTP/HTTPS URL. Use: "http://localhost:11434"`
- [ ] Unknown keys in config: warn (do not error) — user may be on older version
- [ ] All errors collected before returning — show all problems at once, not just first

#### Config loading
- [ ] `Config::load() -> Result<Config, Vec<ConfigError>>` — returns all errors
- [ ] `Config::load_from_str(toml: &str)` — for testing without filesystem
- [ ] `ConfigError` type: `{ field: String, message: String }`
- [ ] Display impl for `ConfigError`: human-readable single line

#### Tests
- [ ] Unit test: valid config for each backend loads without error
- [ ] Unit test: missing required `backend` → error names the field
- [ ] Unit test: invalid `endpoint` URL → error names the field + shows example
- [ ] Unit test: missing `api_key` for Anthropic backend → error explains requirement
- [ ] Unit test: multiple invalid fields → all errors returned, not just first
- [ ] Unit test: unknown key → warning logged, no error
- [ ] Unit test: `Config::load_from_str("")` → meaningful error, no panic

---

### 🔴 v0.4.1 — CLI Utilities
> **Gate:** All CLI flags work correctly; output is human-readable and machine-parseable where noted

#### `--version`
- [ ] Prints: `idep 0.x.y (commit abc1234, built 2025-01-01)`
- [ ] Commit hash injected at build time via `build.rs` + `VERGEN_GIT_SHA`
- [ ] Build date injected at build time
- [ ] `--version --json` prints JSON for scripting: `{ "version": "0.x.y", "commit": "...", "built": "..." }`

#### `--check-config`
- [ ] Loads config from default path, runs full validation
- [ ] Prints `OK` with resolved config path on success
- [ ] Prints all `ConfigError`s on failure, one per line, with field + message
- [ ] Exit code: 0 on success, 1 on failure
- [ ] `--check-config --verbose` prints fully resolved config (all fields, defaults shown)
- [ ] `--check-config --verbose` redacts `api_key` → `[REDACTED]`
- [ ] `--check-config --config <path>` loads from explicit path instead of default

#### `--help`
- [ ] Top-level usage summary
- [ ] All flags listed with descriptions
- [ ] Config file path shown (with `~` expansion)
- [ ] Link to `docs/getting-started.md`

#### `--list-keys`
- [ ] Lists all bindable actions with current key assignment
- [ ] Format: `action-name    <key>    # description`
- [ ] Groups by context: global / editor / chat / file-tree
- [ ] `--list-keys --json` prints machine-readable JSON

#### `--plugin-verify`
- [ ] `idep plugin verify <path>` — compute SHA256 of `.wasm` file, compare against `plugin.toml` `[integrity].sha256`
- [ ] `plugin.toml` schema: optional `[integrity]` section with `sha256` field
- [ ] `idep plugin install <path>` — if `[integrity].sha256` is present, verify before copying to plugins dir; fail on mismatch
- [ ] `idep plugin install <path> --skip-verify` — bypass integrity check (explicit opt-out)
- [ ] Unit test: matching hash → install succeeds
- [ ] Unit test: mismatched hash → install fails with clear error naming expected vs actual
- [ ] Unit test: missing `[integrity]` section → install proceeds with warning "no integrity hash provided"

#### Tests
- [ ] Unit test: `--version` output parses correctly
- [ ] Unit test: `--check-config` exits 0 on valid config, 1 on invalid
- [ ] Unit test: `--check-config --verbose` redacts api_key
- [ ] Integration test: run binary with each flag, verify exit code and output format

---

### 🔴 v0.4.2 — First-Run Wizard
> **Gate:** Fresh install with no config file runs wizard; exits with valid config written

- [ ] Detect missing config on startup
- [ ] TUI wizard: step 1 — choose backend (Ollama / Anthropic / HuggingFace / OpenAI-compat)
- [ ] Step 2 — detect Ollama running locally; if found, suggest default model
- [ ] Step 3 — prompt for API key if cloud backend selected
- [ ] Step 4 — write `~/.config/idep/config.toml`
- [ ] Step 5 — run `--check-config` and show result
- [ ] Skip wizard if `--no-wizard` flag passed
- [ ] Unit test: wizard produces valid config for each backend choice

#### WSL2 — First-run experience
- [ ] Detect if running inside WSL2 (`/proc/version` contains `microsoft`)
- [ ] Wizard: if WSL2 detected, offer WSL2-specific notes (GUI requires WSLg or X11 server)
- [ ] Docs: `docs/install-wsl2.md` — step-by-step WSL2 setup guide
  - [ ] Prerequisites: Windows Terminal, WSLg (Win11) or VcXsrv (Win10)
  - [ ] Install binary
  - [ ] Configure Ollama: note Ollama may run on Windows host, endpoint = `http://$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):11434`
  - [ ] Verify GPU passthrough for wgpu (if GUI chosen)

---

### 🔴 v0.4.3 — Keybinding System
> **Gate:** All default keybindings overridable via `keybindings.toml`; conflict detected; hot-reload works

#### Schema
- [ ] `~/.config/idep/keybindings.toml` format defined and documented
- [ ] Key combo syntax: `"Ctrl+s"`, `"Space+c"`, `"Alt+F4"`, `"g g"` (sequence)
- [ ] Context-aware bindings: `[editor]`, `[chat]`, `[file-tree]`, `[global]` sections
- [ ] Default bindings file shipped with binary, loaded as fallback

#### `KeybindingMap` implementation
- [ ] `KeybindingMap` struct: `HashMap<Context, HashMap<Action, KeyCombo>>`
- [ ] `KeybindingMap::lookup(context, key_event) -> Option<Action>`
- [ ] Key sequence support: track partial match state across keystrokes (e.g. `g` then `g`)
- [ ] Sequence timeout: configurable (default 500ms), partial match cleared on timeout
- [ ] All previously hardcoded keys replaced with `KeybindingMap::lookup` calls

#### Validation
- [ ] Conflict detection: same key in same context assigned to two actions → warn with both action names
- [ ] Unknown action name in user config → warn, skip that binding
- [ ] Invalid key combo syntax → warn with line number, skip that binding
- [ ] `idep --check-config` includes keybinding validation

#### Hot-reload
- [ ] File watcher on `keybindings.toml` (reuse `notify` from workspace watcher)
- [ ] On change: reload, re-validate, apply if valid; log error and keep old map if invalid
- [ ] Hot-reload does not interrupt active key sequence

#### `idep --list-keys`
- [ ] Prints all registered actions, their context, current binding, and description
- [ ] Highlights any conflicts with `[CONFLICT]` marker
- [ ] `--json` flag for machine-readable output

#### Tests
- [ ] Unit test: key lookup returns correct action in correct context
- [ ] Unit test: sequence `g g` resolves only after second key
- [ ] Unit test: sequence timeout clears partial match
- [ ] Unit test: conflict between two actions on same key → both warned
- [ ] Unit test: hot-reload with valid file → new bindings take effect
- [ ] Unit test: hot-reload with invalid file → old bindings preserved, error logged

---

### 🔴 v0.4.4 — Release Pipeline
> **Gate:** Tagging `v*` on main triggers CI; binaries for all targets uploaded to GitHub Release with checksums

#### `cargo-dist` setup
- [ ] Add `cargo-dist` to workspace `Cargo.toml` `[workspace.metadata.dist]`
- [ ] Build targets declared:
  - [ ] `x86_64-unknown-linux-gnu`
  - [ ] `aarch64-unknown-linux-gnu`
  - [ ] `x86_64-apple-darwin`
  - [ ] `aarch64-apple-darwin` (Apple Silicon)
  - [ ] `x86_64-pc-windows-msvc`
- [ ] Install script generated: `install.sh` (Unix) and `install.ps1` (Windows)
- [ ] Archive format: `.tar.gz` for Unix, `.zip` for Windows

#### GitHub Actions release workflow
- [ ] Workflow triggers on push to tag matching `v[0-9]+.*`
- [ ] Workflow does NOT trigger on non-version tags
- [ ] Build matrix: one job per target
- [ ] Cross-compilation: `aarch64-unknown-linux-gnu` built on `ubuntu-latest` with `cross`
- [ ] All jobs must pass before release is published (fan-in gate)
- [ ] Release draft created first; published only after all artifacts attached

#### Artifacts
- [ ] Binary artifact per target attached to GitHub Release
- [ ] `SHA256SUMS` file listing checksum for each artifact
- [ ] `CHANGELOG.md` excerpt for current version auto-extracted and posted as release notes
- [ ] Release marked as `latest` only if version does not contain `-alpha`, `-beta`, or `-rc`

#### Verification
- [ ] Post-release check: download Linux x86_64 binary, run `idep --version`, verify output
- [ ] Post-release check: verify SHA256 checksum matches published `SHA256SUMS`
- [ ] Smoke test script: `scripts/verify-release.sh <version>`

#### Tests
- [ ] CI dry-run: `cargo dist plan` passes on PR without publishing
- [ ] Integration test: build script produces binary that runs `--version` on host platform

---

## Phase 0.5.x — Performance + Stability

---

### 🔴 v0.5.0 — Benchmark Suite
> **Gate:** Automated benchmark runs in CI; baseline numbers recorded in `docs/benchmarks.md`

- [ ] `benches/` directory with `criterion` benchmarks
- [ ] Benchmark: keypress → buffer updated latency
- [ ] Benchmark: keypress → completion request sent latency (debounce excluded)
- [ ] Benchmark: first token latency (Ollama local, measured from request sent)
- [ ] Benchmark: startup time (cold start to editor ready)
- [ ] Benchmark: `idep-lsp` round-trip latency (request sent → response parsed)
- [ ] CI: run benchmarks on each PR, fail if >20% regression vs baseline
- [ ] `docs/benchmarks.md`: record baseline numbers, hardware spec

---

### 🔴 v0.5.1 — Indexer Performance
> **Gate:** 50k LOC Rust project indexed in under 60 seconds; query latency under 50ms

- [ ] Benchmark: `Indexer::index_project()` on `idep` repo itself
- [ ] Benchmark: `Indexer::reindex_file()` on single file change
- [ ] Benchmark: `VectorStore::find_similar()` query latency
- [ ] Parallel chunking: use `rayon` for multi-core file walking
- [ ] Parallel embedding: batch size tuned for throughput
- [ ] Profile: identify top bottleneck, document in `docs/benchmarks.md`
- [ ] Target: full index of 50k LOC in <60s on 4-core machine
- [ ] Target: query latency <50ms at 10k chunks

#### WSL2 — Indexer filesystem performance
- [ ] Benchmark: index project on native Linux path (`~/`) under WSL2
- [ ] Benchmark: index project on DrvFs path (`/mnt/c/...`) under WSL2
- [ ] Document: DrvFs performance gap and recommendation (keep projects on Linux filesystem)
- [ ] Warning: emit warning if project root is on `/mnt/` path

#### Git ref watcher — reindex on branch switch
- [ ] Watch `.git/HEAD` via `notify` file watcher (same watcher infrastructure as workspace)
- [ ] On `.git/HEAD` change: trigger `Indexer::index_project()` full reindex
- [ ] Debounce: ignore rapid `.git/HEAD` writes during rebase (500ms window)
- [ ] Log: "Branch changed to `<ref>`, reindexing project"
- [ ] If `.git/HEAD` does not exist (not a git repo): skip, no error
- [ ] Unit test: simulate `.git/HEAD` write → verify `index_project()` called
- [ ] Unit test: non-git directory → watcher setup skipped gracefully
- [ ] Integration test: `git checkout` on test repo → vector store contains chunks from new branch, not old

---

### 🔴 v0.5.2 — Large File Performance
> **Gate:** 100k line file opens, scrolls, and edits without perceptible lag

- [ ] Test file: generate 100k line Rust file
- [ ] Measure: time to open and render first frame
- [ ] Measure: scroll FPS through entire file
- [ ] Measure: keypress-to-render latency mid-file
- [ ] Fix any identified bottlenecks (likely: naive re-highlight on every keystroke)
- [ ] Virtualized rendering: only render visible lines
- [ ] Incremental highlight: only re-highlight changed region
- [ ] Target: <100ms open, >30fps scroll, <16ms keypress-to-render

---

### 🔴 v0.5.3 — Memory Profiling
> **Gate:** Idle memory <500MB; active editing 10k line file <2GB; top allocators identified and documented

#### Profiling setup
- [ ] Profile with `heaptrack` on Linux (allocation-level tracking)
- [ ] Profile with `valgrind massif` as cross-check
- [ ] Establish three scenarios: idle (no file open), editing small file (1k lines), editing large file (10k lines)
- [ ] Run each scenario for 60 seconds, capture heap snapshot

#### Analysis
- [ ] Identify top 5 allocators by retained heap bytes
- [ ] Identify any unbounded caches (completion history, chat history, embedding cache)
- [ ] Identify any retained old completions or stale LSP responses
- [ ] Identify rope buffer overhead vs raw text size ratio

#### Fixes
- [ ] Bound completion history cache (configurable max, default 100 items)
- [ ] Bound chat message history (configurable max, default 50 messages)
- [ ] Bound embedding cache (LRU, configurable max entries, default 1000)
- [ ] LSP: drop parsed AST after diagnostics extracted — do not retain full parsed tree
- [ ] Verify: no memory growth over 30 minutes of idle (no leak)

#### CI integration
- [ ] `cargo test` includes a memory ceiling test: spawn editor, open 10k file, assert RSS < 2GB
- [ ] Nightly job: full `heaptrack` run, compare peak RSS vs baseline, fail if >10% regression

#### Documentation
- [ ] `docs/benchmarks.md`: memory section with profiling methodology
- [ ] Record baseline numbers: idle RSS, active RSS, peak RSS on large file
- [ ] Record hardware spec: CPU, RAM, OS used for measurement

---

### 🔴 v0.5.4 — LSP Stress Testing
> **Gate:** All edge cases pass 100 consecutive runs without crash or hang; no goroutine / thread leaks

#### Server lifecycle edge cases
- [ ] Test: LSP server crashes mid-session → client detects exit, triggers restart policy
- [ ] Test: LSP server restart succeeds → workspace re-initialized, diagnostics resume
- [ ] Test: LSP server never starts (binary not found) → clear error message, graceful degradation
- [ ] Test: LSP server takes >5s to initialize → timeout fires, error reported, retry attempted
- [ ] Test: shutdown request ignored by server → force-kill after timeout, no hang

#### Message edge cases
- [ ] Test: server sends malformed JSON → client logs error, skips message, continues
- [ ] Test: server sends response with unknown ID → logged and dropped
- [ ] Test: server sends notification for unknown method → logged and dropped
- [ ] Test: server sends truncated message (partial Content-Length body) → client waits, does not panic
- [ ] Test: server sends `null` as response body → handled, not unwrapped blindly

#### Concurrency edge cases
- [ ] Test: rapid file changes (50 edits/second) → no duplicate `didChange` sends, no deadlock
- [ ] Test: `didChange` sent before `initialize` completes → queued, sent after handshake
- [ ] Test: two `textDocument/completion` requests in flight simultaneously → both resolved correctly
- [ ] Test: `textDocument/definition` response arrives after buffer has changed → result discarded gracefully
- [ ] Test: `publishDiagnostics` arrives after buffer closed → stale diagnostics cleared, not applied

#### LSP feature edge cases
- [ ] Test: hover over whitespace → `null` response handled, no crash
- [ ] Test: hover over token with no type info → `null` handled, no crash
- [ ] Test: goto definition returns 0 results → empty vec returned, no crash
- [ ] Test: goto definition returns 100+ results → all parsed, no crash
- [ ] Test: completion list has 0 items → empty rendered, no crash
- [ ] Test: completion item missing `label` field → skipped, not panicked

#### Reliability gate
- [ ] All tests pass 100 consecutive runs without failure
- [ ] No thread count growth over 60 minutes of continuous use (verified with `/proc/PID/status`)

---

### 🔴 v0.5.5 — Fuzz Buffer Operations
> **Gate:** 1M iterations produce zero panics or UB; 10M in nightly CI; all crashes become regression tests

#### Fuzz target setup
- [ ] Add `cargo-fuzz` to workspace dev-dependencies
- [ ] `fuzz/fuzz_targets/buffer_ops.rs` fuzz target
- [ ] `fuzz/fuzz_targets/buffer_undo_redo.rs` fuzz target
- [ ] `fuzz/fuzz_targets/buffer_cursor.rs` fuzz target
- [ ] Structured fuzzing: define `FuzzOp` enum (`Insert { pos, text }`, `Delete { range }`, `MoveCursor { pos }`, `Undo`, `Redo`)
- [ ] Use `arbitrary` crate for `FuzzOp` derivation

#### Fuzz coverage
- [ ] Insert at: pos=0, pos=len, pos=len+1, pos=usize::MAX
- [ ] Insert: empty string, single char, 10k chars, string with null bytes, emoji (multibyte)
- [ ] Delete: zero-length range, range at end, overlapping range, range past end
- [ ] Cursor move: to pos=0, to pos=len, to pos=usize::MAX
- [ ] Undo with no history → no panic
- [ ] Redo with nothing to redo → no panic
- [ ] Mixed sequences: insert/delete/undo/redo in random order

#### Sanitizer coverage
- [ ] Run under AddressSanitizer (ASAN): detect out-of-bounds and use-after-free
- [ ] Run under UndefinedBehaviorSanitizer (UBSAN): detect integer overflow, misaligned access
- [ ] `RUSTFLAGS="-Z sanitizer=address"` fuzz run in CI

#### Regression harness
- [ ] Any crash found → minimized input saved to `fuzz/artifacts/`
- [ ] Minimized inputs added as regular unit tests (never deleted)
- [ ] `cargo test` runs all regression inputs before fuzzing

#### CI schedule
- [ ] Local: run 1M iterations as part of `cargo test --release` (fast fuzz mode)
- [ ] Nightly CI job: 10M iterations with ASAN enabled, fail on any finding

---

## Phase 0.6.x — Security + Trust

---

### 🔴 v0.6.0 — Network Surface Audit
> **Gate:** Zero outbound connections without explicit user action; verified by test with network blocked

#### Enumeration
- [ ] Static analysis: `grep -r "reqwest\|ureq\|hyper\|TcpStream\|UdpSocket"` — list all outbound call sites
- [ ] Document every call site: file, line, purpose, trigger condition
- [ ] Categorize: user-triggered (AI backend calls) vs automatic (telemetry, updates, model download)

#### Verification — automatic calls
- [ ] Verify: no telemetry or analytics calls anywhere in codebase
- [ ] Verify: no update check on startup
- [ ] Verify: no crash report upload
- [ ] Verify: model download (`fastembed-rs`) only triggered on first index, not on startup
- [ ] Verify: embeddings computed locally — `fastembed-rs` does not call any API

#### Verification — user-triggered calls
- [ ] All AI backend calls: only made when user explicitly sends a message or triggers completion
- [ ] All AI backend calls: use the endpoint from user config — no hardcoded fallback endpoint

#### Network-blocked test
- [ ] `tests/network_audit.rs`: starts idep with network interface blocked (via `seccomp` or mock)
- [ ] Test: startup completes without network access → no panic, no timeout hang
- [ ] Test: open file, trigger index → completes without network
- [ ] Test: send AI request with Ollama backend (local) → works without internet access
- [ ] Test: `--check-config` completes without network access

#### Documentation
- [ ] `SECURITY.md` updated: list all outbound call sites with verified "user-triggered only" claim
- [ ] Each claim in `SECURITY.md` linked to the test that verifies it

---

### 🔴 v0.6.1 — API Key Handling Audit
> **Gate:** API key never appears in any output, log, file, or backtrace under any condition

#### `ApiKey` type
- [ ] `ApiKey(String)` newtype — never `pub` inner field
- [ ] `Debug` impl: always outputs `ApiKey([REDACTED])`
- [ ] `Display` impl: always outputs `[REDACTED]`
- [ ] `Serialize` impl: outputs `"[REDACTED]"` (for JSON export safety)
- [ ] No `Clone` derived — must be explicit `.clone_secret()` method to force awareness
- [ ] `Zeroize` on drop: key bytes zeroed in memory when `ApiKey` dropped

#### Log audit
- [ ] Grep all `tracing::` / `log::` call sites for any variable that might carry key
- [ ] Verify: `reqwest::RequestBuilder` log level never set to TRACE (would log headers)
- [ ] Verify: HTTP client not configured with any debug logging middleware that logs headers
- [ ] Verify: error types derived from backend responses do not include request headers in display

#### Storage audit
- [ ] Verify: config written by wizard does not log key to stdout
- [ ] Verify: `ChatSession::export()` JSON does not include key
- [ ] Verify: crash dump / `std::panic::set_hook` output does not include config struct
- [ ] Verify: temp files created during request (if any) do not contain key
- [ ] Verify: key not included in `--check-config --verbose` output (shown as `[REDACTED]`)

#### Process audit
- [ ] Verify: key not passed via CLI argument (would appear in `ps aux`)
- [ ] Verify: key loaded from file or env var only, never hardcoded fallback

#### Tests
- [ ] Unit test: `format!("{:?}", api_key)` → `"ApiKey([REDACTED])"`
- [ ] Unit test: `serde_json::to_string(&api_key)` → `"\"[REDACTED]\""`
- [ ] Unit test: `ChatSession::export()` on session that used Anthropic → no key in output
- [ ] Unit test: `--check-config --verbose` output does not contain the actual key value

---

### 🔴 v0.6.2 — WASM Plugin Sandbox Audit
> **Gate:** Plugin cannot escape sandbox; all escape attempts caught and logged; host continues running

#### Filesystem escape attempts
- [ ] Plugin attempts `std::fs::read("/etc/passwd")` → WASM import not linked → trap at instantiation
- [ ] Plugin attempts `std::fs::write("/tmp/evil", b"x")` → same
- [ ] Plugin attempts `std::env::var("HOME")` → not available → returns `None` / empty
- [ ] Plugin attempts to open a socket directly → not linked → trap

#### Memory escape attempts
- [ ] Plugin reads outside its linear memory bounds → wasmtime trap, caught by host
- [ ] Plugin writes to host-side address → wasmtime linear memory isolation prevents it
- [ ] Plugin allocates 1GB → `StoreLimits` fires, allocation denied, trap caught

#### CPU escape attempts
- [ ] Plugin enters infinite loop → fuel exhausted after configured limit → trap caught
- [ ] Plugin calls `std::thread::sleep(Duration::MAX)` → not available → ignored

#### Host isolation
- [ ] Plugin cannot read other plugins' memory
- [ ] Plugin cannot call host functions not explicitly exported (verify via wasmtime linker)
- [ ] Malicious plugin manifest (path traversal in entry point) → rejected at load time

#### Recovery
- [ ] Plugin that traps on `on_file_open` → marked `PluginState::Failed`, skipped on future calls
- [ ] All other plugins continue working after one plugin fails
- [ ] Failed plugin logged with: plugin name, hook name, trap message
- [ ] `idep plugin list` shows failed plugins with failure reason

#### Documentation
- [ ] `docs/plugin-security.md`:
  - [ ] What the sandbox prevents (filesystem, network, threads, host memory)
  - [ ] What plugins CAN do (log, read config, provide completions, register commands)
  - [ ] How to report a sandbox escape vulnerability

---

### 🔴 v0.6.3 — `SECURITY.md` Update
> **Gate:** Every security claim backed by a named test; responsible disclosure process complete

#### Threat model update
- [ ] Re-read and revise threat model section to reflect current architecture
- [ ] Explicitly list: what idep protects against, and what it does not
- [ ] Out-of-scope: security of AI backend providers (Anthropic, HF, etc.)
- [ ] Out-of-scope: security of LSP servers (rust-analyzer, etc.)

#### Claim verification index
- [ ] Table: security claim → test name → pass/fail status
- [ ] Claim: "no outbound connections without user action" → linked to `tests/network_audit.rs`
- [ ] Claim: "API key never logged" → linked to unit tests in `v0.6.1`
- [ ] Claim: "plugin cannot access filesystem" → linked to sandbox audit tests in `v0.6.2`
- [ ] Claim: "plugin cannot access host memory" → linked to sandbox audit tests in `v0.6.2`
- [ ] Claim: "code never leaves machine for indexing" → linked to `tests/network_audit.rs`

#### Responsible disclosure
- [ ] Disclosure email address listed (security@idep.dev or GitHub security advisories)
- [ ] Expected response time stated (e.g. 48 hours for acknowledgement)
- [ ] CVE process described (will request CVE for confirmed vulnerabilities)
- [ ] Hall of fame: section for acknowledged reporters (opt-in)
- [ ] Out-of-scope list: what not to report (e.g. issues in third-party AI backends)

#### Review
- [ ] `SECURITY.md` reviewed by at least one person other than the author
- [ ] All links in `SECURITY.md` verified live

---

### 🔴 v0.6.4 — Dependency Audit
> **Gate:** `cargo audit` clean; `cargo deny` clean; no unexpected GPL deps; supply chain documented

#### Vulnerability audit
- [ ] Run `cargo audit` — resolve or explicitly accept all findings with documented rationale
- [ ] Add `cargo audit` to CI: fail on any new `RUSTSEC` advisory matching deps
- [ ] Nightly CI: re-run `cargo audit` automatically (new advisories published daily)
- [ ] `audit.toml`: document any ignored advisories with expiry date and rationale

#### License compliance
- [ ] Run `cargo deny check licenses`
- [ ] Policy: deny GPL-2.0, GPL-3.0, LGPL (incompatible with Apache 2.0 distribution)
- [ ] Policy: allow MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, CC0
- [ ] `deny.toml` committed to repo with full license policy documented
- [ ] All exceptions to policy documented with rationale

#### Supply chain
- [ ] Run `cargo deny check bans` — no duplicate versions of critical deps (e.g. two versions of `serde`)
- [ ] Run `cargo deny check sources` — all deps from crates.io or explicit git (no unknown registries)
- [ ] `Cargo.lock` committed and kept up to date
- [ ] Dependabot or Renovate configured to open PRs for dep updates

#### Documentation
- [ ] `docs/dependencies.md`: list direct deps with purpose, license, and maintainer health note
- [ ] Flag any deps with single maintainer or low activity — document mitigation (fork plan, alternative)

#### Tests
- [ ] CI: `cargo deny check` runs on every PR
- [ ] CI: `cargo audit` runs on every PR
- [ ] Nightly: full re-scan with latest advisory DB

---

## Phase 0.7.x — Documentation

> Written after APIs stabilize. Not before.

---

### 🔴 v0.7.0 — API Reference Docs
> **Gate:** `cargo doc` clean with `deny(missing_docs)`; deployed to `docs.idep.dev`; zero broken links

#### `#![deny(missing_docs)]` enforcement
- [ ] Add `#![deny(missing_docs)]` to: `idep-core`, `idep-ai`, `idep-lsp`, `idep-index`, `idep-plugin`
- [ ] All public structs documented: purpose, invariants
- [ ] All public traits documented: contract, implementor requirements
- [ ] All public functions documented: what it does, parameters, return value, panics, errors
- [ ] All public fields documented: meaning, valid range, default
- [ ] All public error variants documented: when this error occurs, how to handle it

#### Doc comment quality
- [ ] Every doc comment answers: "what does this do?" in one sentence
- [ ] Complex types include `# Examples` section with runnable code
- [ ] Fallible functions include `# Errors` section listing `Err` variants
- [ ] Functions that can panic include `# Panics` section
- [ ] Internal implementation notes use `// not ///` — not surfaced in docs

#### Cross-references
- [ ] `idep-ai` docs link to `idep-index` types where used
- [ ] `idep-lsp` docs link to `idep-core` buffer types
- [ ] All trait implementors listed (via `#[doc = "See also:"]` or `cargo doc` auto-linking)

#### Deployment
- [ ] `cargo doc --all --no-deps` runs clean (zero warnings)
- [ ] Docs deployed to `docs.idep.dev` via GitHub Pages on push to `main`
- [ ] CI: verify docs build on every PR (`cargo doc --all --no-deps 2>&1 | grep -c "^warning"` must be 0)
- [ ] No broken intra-doc links (`cargo doc` reports these as warnings → treated as errors)

#### Verification
- [ ] Manual review: spot-check 10 random public items for doc quality
- [ ] `docs.idep.dev` loads in browser, search works, all crates listed

---

### 🔴 v0.7.1 — Getting Started Guide
> **Gate:** An unfamiliar developer installs idep and gets first AI completion in under 15 minutes following the guide alone

#### Guide structure — `docs/getting-started.md`
- [ ] Section: Prerequisites
  - [ ] Rust toolchain (link to rustup)
  - [ ] For local AI: Ollama install + model download (`ollama pull codellama:13b`)
  - [ ] For cloud AI: obtaining an Anthropic / HuggingFace / OpenAI API key
  - [ ] WSL2 note: link to `docs/install-wsl2.md`
- [ ] Section: Install
  - [ ] From binary (GitHub Releases — preferred, no Rust needed)
  - [ ] From source (`cargo install idep`)
  - [ ] Verify: `idep --version` prints expected output
- [ ] Section: First-run wizard walkthrough
  - [ ] Step-by-step screenshots or terminal recordings
  - [ ] What each step does and why
  - [ ] How to re-run wizard if config is lost: `idep --wizard`
- [ ] Section: Open your first project
  - [ ] `idep /path/to/project`
  - [ ] What happens on first open (indexing, LSP start)
  - [ ] Expected startup time
- [ ] Section: Trigger first AI completion
  - [ ] Open a Rust file
  - [ ] Start typing a function
  - [ ] What ghost text looks like
  - [ ] How to accept (`Tab`) and dismiss (`Esc`)
- [ ] Section: Trigger first AI chat message
  - [ ] Open chat panel (`Space+c`)
  - [ ] Ask a question about the open file
  - [ ] What context is injected automatically
- [ ] Section: Next steps (links to config reference, keybindings, plugin docs)

#### Quality gate
- [ ] Guide reviewed by at least one developer who has never seen the project
- [ ] Reviewer completes install → first completion without asking any questions
- [ ] All terminal commands copy-pasteable (no ellipsis, no placeholder commands)
- [ ] No broken links (CI: `lychee docs/getting-started.md`)
- [ ] Estimated reading + follow-along time documented at top of guide

---

### 🔴 v0.7.2 — Full Config Reference
> **Gate:** Every config field documented with type, default, valid values, and example; CI verifies sync with `config.example.toml`

#### Document structure — `docs/config-reference.md`
- [ ] Section: `[ai]`
  - [ ] `backend`: enum, required, valid: `ollama | anthropic | huggingface | openai`
  - [ ] `model`: string, required, examples per backend
  - [ ] `endpoint`: URL string, optional, default per backend
  - [ ] `timeout_ms`: integer, optional, default 30000
  - [ ] `max_retries`: integer, optional, default 3
- [ ] Section: `[ai.auth]`
  - [ ] `api_key`: string, optional, env var fallback `IDEP_API_KEY`
  - [ ] Security note: never commit this to version control
- [ ] Section: `[ai.completion]`
  - [ ] `debounce_ms`: integer, optional, default 400
  - [ ] `max_tokens`: integer, optional, default 128
  - [ ] `stop_sequences`: array of strings, optional
  - [ ] `fim_style`: enum, optional: `deepseek | starcoder | codellama | none`
- [ ] Section: `[ai.chat]`
  - [ ] `max_context_tokens`: integer, optional, default 4096
  - [ ] `history_depth`: integer, optional, default 20 (messages kept)
  - [ ] `system_prompt`: string, optional
- [ ] Section: `[editor]`
  - [ ] `theme`: string, optional, default `dark`
  - [ ] `font_size`: integer, optional, default 14
  - [ ] `tab_width`: integer, optional, default 4
  - [ ] `insert_spaces`: bool, optional, default true
  - [ ] `line_numbers`: bool, optional, default true
  - [ ] `wrap_lines`: bool, optional, default false
- [ ] Section: `[lsp]`
  - [ ] `rust_analyzer`: path or `auto`, optional
  - [ ] `typescript_language_server`: path or `auto`, optional
  - [ ] `diagnostic_debounce_ms`: integer, optional, default 500
- [ ] Section: `[index]`
  - [ ] `model`: string, optional, default `all-MiniLM-L6-v2`
  - [ ] `chunk_size_tokens`: integer, optional, default 512
  - [ ] `top_k`: integer, optional, default 5
  - [ ] `index_on_save`: bool, optional, default true
  - [ ] `exclude_patterns`: array of glob strings, optional

#### CI sync check
- [ ] CI script: parse `config.example.toml` keys, parse `docs/config-reference.md` headings, assert all keys documented
- [ ] CI fails if `config.example.toml` adds a key not present in reference doc
- [ ] CI fails if reference doc documents a key not in `config.example.toml`

---

### 🔴 v0.7.3 — Plugin Authoring Guide
> **Gate:** An external developer writes, builds, and loads a working plugin following the guide alone — without asking any questions

#### Rust guide — `docs/plugin-authoring-rust.md`
- [ ] Section: Prerequisites (Rust, `wasm32-unknown-unknown` target, `idep-plugin-sdk`)
- [ ] Section: Create a new plugin (`cargo new --lib my-plugin`)
- [ ] Section: Add `idep-plugin-sdk` dependency
- [ ] Section: Implement the `Plugin` trait — annotated `hello-world` walkthrough
- [ ] Section: Build for WASM (`cargo build --target wasm32-unknown-unknown --release`)
- [ ] Section: Create `plugin.toml` manifest
- [ ] Section: Install plugin (`idep plugin install ./target/.../<n>.wasm`)
- [ ] Section: Verify plugin loaded (`idep plugin list`)
- [ ] Section: Logging from a plugin (`idep_log`)
- [ ] Section: Providing completions (annotated `provide_completions` example)
- [ ] Section: Registering a command
- [ ] Section: Common mistakes and how to fix them

#### TypeScript guide — `docs/plugin-authoring-typescript.md`
- [ ] Section: Prerequisites (Node.js, `wasm-pack` or `AssemblyScript`)
- [ ] Section: Create a new plugin project
- [ ] Section: Install `idep-plugin-ts` package
- [ ] Section: Implement the plugin interface — annotated `hello-world` walkthrough
- [ ] Section: Build to WASM
- [ ] Section: Install and verify

#### API reference — `docs/plugin-api-reference.md`
- [ ] Every trait method documented: purpose, parameters, return type, example
- [ ] Every host function documented: `idep_log`, `idep_insert_text`, etc.
- [ ] Every type documented: fields, valid values, examples
- [ ] Stability level marked per item: `stable | experimental | deprecated`

#### Quality gate
- [ ] Both guides reviewed by at least one external developer
- [ ] Reviewer builds and loads a plugin without asking questions
- [ ] All code snippets compile without modification
- [ ] No broken links in any guide

---

### 🔴 v0.7.4 — Architecture Guide
> **Gate:** A new contributor reads the guide and correctly identifies where to make a change for 3 given scenarios

#### Document structure — `docs/architecture.md`
- [ ] Section: Crate map
  - [ ] Table: crate name, responsibility, key public types
  - [ ] Dependency graph diagram (Mermaid or SVG): which crates depend on which
  - [ ] Rule: `idep-core` must never depend on `idep-ai` or `idep-lsp`
- [ ] Section: Data flows (each as a numbered step-by-step + diagram)
  - [ ] Keypress → buffer update → re-render
  - [ ] Keypress → completion debounce → AI request → ghost text render
  - [ ] File save → `notify` event → `Indexer::reindex_file` → vector store update
  - [ ] Chat message → `ContextEngine::gather` → API request → streaming render
  - [ ] LSP: file open → `didOpen` → `publishDiagnostics` → gutter render
- [ ] Section: Key design decisions
  - [ ] Why `idep-core` buffer is the single source of truth (not a retained-mode model)
  - [ ] Why tree-sitter chunking over naive line splitting for RAG
  - [ ] Why `fastembed-rs` (local) over an embedding API call
  - [ ] Why WASM for plugins (not dynamic linking or subprocess)
  - [ ] Why TUI before GUI (API contract validation)
  - [ ] Why the renderer spike approach (not committing early)
- [ ] Section: Adding a new AI backend (step-by-step recipe)
- [ ] Section: Adding a new language to the indexer (step-by-step recipe)
- [ ] Section: Adding a new LSP method (step-by-step recipe)
- [ ] Section: Adding a new plugin host function (step-by-step recipe)

#### Quality gate
- [ ] Guide reviewed by contributor who did not write any of the code
- [ ] Reviewer answers: "where do I add a new backend?" without looking at source
- [ ] Reviewer answers: "where does ghost text get rendered?" without looking at source
- [ ] Reviewer answers: "where does context get assembled before a chat request?" without looking at source
- [ ] All diagrams render correctly in GitHub Markdown preview

---

### 🔴 v0.7.5 — Competitive Comparison Doc
> **Gate:** Published; all claims verifiable; reviewed by external reader; benchmark page live on idep.dev

#### Comparison documents
- [ ] `docs/why-not-antigravity.md`
  - [ ] Feature table: agent orchestration vs precise completion — explain the difference
  - [ ] RAM: measured side-by-side, same project
  - [ ] Cloud dependency: document what Google Antigravity requires vs idep
  - [ ] Honest trade-off section: when Antigravity is the better choice
- [ ] `docs/why-not-cursor-windsurf.md`
  - [ ] License: proprietary vs Apache 2.0 — what this means for the user
  - [ ] AI backend lock-in: BYOK cloud-only vs any endpoint
  - [ ] Electron vs native: measured startup time and RAM
  - [ ] Honest trade-off section: when Cursor/Windsurf is the better choice
- [ ] `docs/why-not-zed.md`
  - [ ] AGPL vs Apache 2.0: implications for downstream use
  - [ ] RAG: Zed has none, idep has in-process RAG — explain what this means in practice
  - [ ] GPU rendering: compare GPUI vs idep's chosen renderer
  - [ ] Honest trade-off section: when Zed is the better choice

#### Benchmark methodology
- [ ] `docs/benchmarks.md` benchmark section:
  - [ ] Hardware spec used for all measurements (CPU, RAM, OS)
  - [ ] Measurement tool and method for each metric
  - [ ] How to reproduce measurements (reproducible script in `scripts/benchmark-comparison.sh`)

#### Benchmark measurements
- [ ] RAM: idle memory for each editor (same empty project open)
- [ ] RAM: active memory editing a 10k line Rust file
- [ ] Startup time: cold start to editor ready (5 runs, median)
- [ ] First-token latency: Ollama `codellama:13b`, same prompt, 5 runs, median
- [ ] Index build time: idep vs "no index" (Cursor/Zed don't have equivalent — document this clearly)

#### Quality gates
- [ ] All claims are either: measured (with methodology), cited (with source), or clearly marked as opinion
- [ ] No FUD: do not attribute claims to competitors without source
- [ ] Reviewed by at least one developer who uses one of the compared tools
- [ ] Benchmark page live on `idep.dev/compare`

---

## Phase 0.8.x — Community Infrastructure

---

### 🔴 v0.8.0 — Contribution Infrastructure
> **Gate:** A first-time contributor submits a working PR without asking any questions

#### `CONTRIBUTING.md`
- [ ] Section: Dev environment setup
  - [ ] Required tools: Rust, `rust-analyzer`, `cargo-nextest`, `cargo-audit`, `cargo-deny`
  - [ ] Optional tools: `heaptrack`, `cargo-fuzz`, `wasm-pack`
  - [ ] Clone → `cargo build --all` → expected output
  - [ ] How to run all tests: `cargo nextest run --all`
  - [ ] How to run a single crate's tests: `cargo nextest run -p idep-ai`
  - [ ] How to run clippy: `cargo clippy --all -- -D warnings`
  - [ ] How to run fmt check: `cargo fmt --all -- --check`
  - [ ] How to run `cargo audit`
- [ ] Section: Project structure overview (link to `docs/architecture.md`)
- [ ] Section: Code style guide
  - [ ] Error handling: `thiserror` for library errors, `anyhow` in binaries
  - [ ] Logging: `tracing` crate, log levels defined
  - [ ] No `unwrap()` in library code — use `?` or explicit error types
  - [ ] Public API must have doc comments
  - [ ] Tests alongside code in `#[cfg(test)]` modules, not separate files
- [ ] Section: PR checklist
  - [ ] Tests pass (`cargo nextest run --all`)
  - [ ] No new clippy warnings
  - [ ] `cargo fmt` applied
  - [ ] Doc comments added for new public items
  - [ ] `CHANGELOG.md` entry added under `[Unreleased]`
- [ ] Section: How to report a bug (link to issue template)
- [ ] Section: How to propose a feature (link to issue template + discussion first)
- [ ] Section: Commit message format (`<type>(<scope>): <description>`)

#### GitHub issue and PR templates
- [ ] `.github/ISSUE_TEMPLATE/bug_report.md`: reproduction steps, expected vs actual, platform info
- [ ] `.github/ISSUE_TEMPLATE/feature_request.md`: problem statement, proposed solution, alternatives
- [ ] `.github/ISSUE_TEMPLATE/docs_issue.md`: which doc, what's wrong, suggested fix
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`: checklist matching `CONTRIBUTING.md` PR checklist

#### Labels and triage
- [ ] Label set created: `bug`, `enhancement`, `docs`, `good first issue`, `help wanted`, `blocked`, `breaking-change`
- [ ] `good first issue` applied to 5+ existing issues with clear scope
- [ ] `help wanted` applied to issues where outside contribution is welcome
- [ ] Stale bot configured: warn after 30 days, close after 60 days of inactivity

---

### 🔴 v0.8.1 — Discord Server
> **Gate:** Server live; new member can find help and start contributing within 10 minutes

#### Server structure
- [ ] Discord server created with idep branding (icon, banner)
- [ ] Channels created:
  - [ ] `#announcements` — read-only, major releases and news
  - [ ] `#general` — open discussion
  - [ ] `#dev` — technical discussion, PRs, architecture
  - [ ] `#help` — usage questions, troubleshooting
  - [ ] `#plugins` — plugin development and showcases
  - [ ] `#showcase` — share projects built with idep
  - [ ] `#off-topic` — community chat
- [ ] Roles: `contributor` (anyone with merged PR), `maintainer`
- [ ] Role-assignment bot or manual process documented

#### Onboarding
- [ ] `#welcome` channel or bot DM: links to docs, contributing guide, GitHub
- [ ] Pinned message in `#help`: how to ask a good question, what info to include
- [ ] Pinned message in `#dev`: link to `docs/architecture.md` and `CONTRIBUTING.md`
- [ ] Bot: auto-post GitHub release notes to `#announcements`

#### Community standards
- [ ] Code of conduct posted in server (link to `CODE_OF_CONDUCT.md` in repo)
- [ ] Moderation policy documented (in server or repo)
- [ ] Link added to `README.md`, website footer, and `CONTRIBUTING.md`

#### Verification
- [ ] Test: new member joins, reads onboarding, finds `#help` and asks a question
- [ ] Test: GitHub release triggers announcement bot post

---

### 🔴 v0.8.2 — Open Collective
> **Gate:** Open Collective page live; funds usage policy public; first sponsor tier functional

#### Open Collective setup
- [ ] Open Collective account created under `idep` organization (not personal)
- [ ] Project description written: what idep is, why it's worth funding
- [ ] Logo and banner uploaded

#### Sponsor tiers
- [ ] Tier: Individual Supporter — any amount, name in README
- [ ] Tier: Project Sponsor — $10+/month, logo in README (small)
- [ ] Tier: Company Sponsor — $100+/month, logo on website (prominent)
- [ ] Tier descriptions written: what sponsors get, what it funds

#### Funds usage policy
- [ ] `SUSTAINABILITY.md` updated with:
  - [ ] What funds are used for: contributor bounties, infra (CI, domain), tooling
  - [ ] What funds are never used for: gating features, cloud lock-in, ads
  - [ ] Bounty process: how tasks get bounties, how contributors claim them
  - [ ] Financial transparency: link to Open Collective public ledger
- [ ] Policy reviewed for clarity before publishing

#### Integration
- [ ] Badge added to `README.md` — active link (not placeholder)
- [ ] Link added to website footer
- [ ] Link added to GitHub Sponsors fallback (if applicable)
- [ ] First bounty posted on a `good first issue`

---

### 🔴 v0.8.3 — First External Contributor
> **Gate:** At least one PR from a contributor outside the core team is reviewed, merged, and celebrated

#### Preparation
- [ ] Verify `CONTRIBUTING.md` is complete (v0.8.0 gate passed)
- [ ] Identify 5+ issues suitable for first contribution:
  - [ ] Scope is clearly defined (not open-ended)
  - [ ] Does not require deep architecture knowledge
  - [ ] Has a clear acceptance criterion
  - [ ] Estimated effort: 1–4 hours
- [ ] Label identified issues `good first issue` + `help wanted`
- [ ] Add comment to each issue: "This is a good first issue. Here's a hint: ..."

#### Outreach to attract contributor
- [ ] Post in Discord `#dev`: "looking for first contributors, here are some good first issues"
- [ ] Post on relevant forums (Reddit `r/rust`, Rust users forum) with issue links
- [ ] Respond to questions about issues within 24 hours

#### Review process
- [ ] Review PR within 48 hours of submission
- [ ] Give constructive, specific feedback (not "LGTM" rubber-stamps)
- [ ] If changes needed: explain why, not just what
- [ ] Merge when ready — do not let it stall

#### Celebration
- [ ] Publicly thank contributor by name in `CHANGELOG.md` release notes
- [ ] Post in Discord `#announcements`: "Welcome our first contributor, [name]!"
- [ ] Add contributor to `CONTRIBUTORS.md` (create file if needed)
- [ ] Assign `contributor` Discord role

---

### 🔴 v0.8.4 — ASEAN / Indonesian Developer Outreach
> **Gate:** Announcement posted on all channels; Indonesian-language resources live; at least one community response

#### Content creation
- [ ] Announcement post written in English — leads with ~2GB RAM floor (vs Cursor 4–8GB, Antigravity 16GB), then covers: what idep is, why local-first matters, how to contribute
- [ ] Indonesian-language version translated and reviewed by native speaker
- [ ] `README.md` includes Indonesian translation section (`## Tentang idep`)
- [ ] `docs/getting-started-id.md` — Indonesian-language getting started guide
- [ ] Blog post: "Membangun IDE dengan Rust dan AI Lokal" (Building an IDE with Rust and Local AI)

#### Distribution channels
- [ ] dev.to post (English)
- [ ] Hacker News Show HN post (English)
- [ ] Reddit `r/rust` post
- [ ] Reddit `r/programming` post
- [ ] Indonesian developer communities:
  - [ ] `rust-id` Discord / Telegram
  - [ ] Dicoding community (largest Indonesian dev learning platform)
  - [ ] Komunitas Developer Indonesia (Facebook group)
  - [ ] Tech in Asia community

#### Low-spec hardware documentation
- [ ] `docs/low-spec-setup.md`:
  - [ ] Minimum spec: 2-core CPU, 4GB RAM, no dedicated GPU
  - [ ] Recommended Ollama model for low-spec: `codellama:7b` or `deepseek-coder:1.3b`
  - [ ] Performance expectations on low-spec hardware (measured)
  - [ ] Tips: disable GUI renderer, use TUI mode for lower memory footprint
- [ ] Verified: TUI mode runs on 4GB RAM machine without swapping
- [ ] Benchmark: measure actual peak RSS on 4GB RAM machine running TUI + Ollama `deepseek-coder:1.3b`
- [ ] Include benchmark numbers in announcement post and `docs/low-spec-setup.md`

#### Community response tracking
- [ ] Monitor: GitHub stars, issues, Discord joins, Reddit upvotes for 2 weeks post-announcement
- [ ] Respond to all comments within 48 hours
- [ ] Track: first Indonesian contributor (name in `CONTRIBUTORS.md`)

---

## Phase 0.9.x — Release Candidates

> Nothing new. Only hardening, verification, and announcement preparation.

---

### 🔴 v0.9.0 — RC1: All Tests Green
> **Gate:** Full test suite passes on all target platforms

- [ ] `cargo test --all` passes on Linux x86_64
- [ ] `cargo test --all` passes on Linux aarch64
- [ ] `cargo test --all` passes on macOS (Intel + Apple Silicon)
- [ ] `cargo test --all` passes on Windows / WSL2
- [ ] `cargo clippy --all -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo audit` clean
- [ ] `cargo deny check` clean

---

### 🔴 v0.9.1 — RC2: Bug Triage
> **Gate:** All P0/P1 bugs fixed; all issues triaged; regression tests added

#### Triage process
- [ ] All open GitHub issues reviewed by at least two people
- [ ] Every issue assigned a priority label: `P0` / `P1` / `P2` / `P3`
  - [ ] P0: crash, data loss, corruption, security issue
  - [ ] P1: major feature broken, no workaround
  - [ ] P2: significant issue, workaround exists
  - [ ] P3: minor issue, cosmetic, nice-to-have
- [ ] Every issue assigned to a milestone (`v1.0.0` or `backlog`)
- [ ] Stale / duplicate issues closed with comment

#### P0 fixes — required before RC3
- [ ] All P0 issues resolved and closed
- [ ] Regression test added for each P0 fix (test that would have caught the bug)
- [ ] `cargo test --all` passes after each P0 fix on a clean branch

#### P1 fixes — required before RC3
- [ ] All P1 issues resolved and closed
- [ ] Regression test added for each P1 fix
- [ ] `cargo test --all` passes after all P1 fixes

#### P2 / P3 handling
- [ ] All P2 issues documented in release notes under "Known Issues"
- [ ] All P3 issues moved to `backlog` milestone
- [ ] Release notes section written: "Known Issues in v1.0.0"

#### Sign-off
- [ ] Triage sign-off: confirm zero open P0/P1 issues in `v1.0.0` milestone
- [ ] Final `cargo test --all` on main branch: passes clean

---

### 🔴 v0.9.2 — RC3: Binary Artifacts Verified
> **Gate:** Every artifact smoke-tested on real hardware or VM for its target platform; no install failures

#### Download verification
- [ ] Download each artifact from GitHub Release page (not from local build)
- [ ] Verify SHA256 checksum for each artifact against published `SHA256SUMS` file
- [ ] Verify: install script `install.sh` runs cleanly on fresh Ubuntu 22.04 VM
- [ ] Verify: install script `install.sh` runs cleanly on fresh macOS 14 (Apple Silicon)
- [ ] Verify: `install.ps1` runs cleanly on fresh Windows 11 + WSL2

#### Smoke test per target
- [ ] Linux x86_64: `--version`, open `.rs` file, trigger completion, get diagnostic
- [ ] Linux aarch64: `--version`, open `.rs` file, trigger completion, get diagnostic
- [ ] macOS Intel: `--version`, open `.rs` file, trigger completion, get diagnostic
- [ ] macOS Apple Silicon: `--version`, open `.rs` file, trigger completion, get diagnostic
- [ ] Windows WSL2: `--version`, open `.rs` file, trigger completion, get diagnostic

#### Edge cases
- [ ] Fresh machine with no Rust installed → wizard detects missing Ollama, shows clear message
- [ ] Machine with Ollama installed → wizard detects it and pre-fills config
- [ ] Config file from older version → migration or clear error, not silent failure
- [ ] Binary on read-only filesystem → graceful error on config write, not panic

#### Sign-off
- [ ] Each platform smoke-tested by a human, not just CI
- [ ] Results recorded: tester name, platform, OS version, date, pass/fail notes

---

### 🔴 v0.9.3 — RC4: Docs and Changelog Final Review
> **Gate:** All docs accurate, CHANGELOG complete, zero broken links, README reflects v1.0.0 state

#### Docs accuracy review
- [ ] `docs/getting-started.md` — follow guide on fresh machine, verify every step works
- [ ] `docs/config-reference.md` — verify every field matches current code
- [ ] `docs/architecture.md` — verify all crate names, data flows, and diagrams are current
- [ ] `docs/plugin-api-reference.md` — verify every method signature matches `api.rs`
- [ ] `docs/plugin-authoring-rust.md` — build hello-world plugin following guide, verify it works
- [ ] `docs/plugin-authoring-typescript.md` — build hello-world plugin following guide
- [ ] `docs/keybindings.md` — verify all listed keys match `KeybindingMap` defaults
- [ ] `docs/benchmarks.md` — verify numbers are from current build, not stale
- [ ] `docs/why-not-*.md` — verify competitive claims are still accurate
- [ ] `SECURITY.md` — verify all claims are backed by current tests
- [ ] `SUSTAINABILITY.md` — verify funding model and links are current
- [ ] `CONTRIBUTING.md` — follow setup guide on fresh machine, verify it works

#### CHANGELOG completeness
- [ ] Entry for every version from `v0.0.1` to `v0.9.3` present
- [ ] Each entry has: date, version, brief summary, notable changes
- [ ] `[Unreleased]` section present and ready for `v1.0.0` entry
- [ ] No placeholder text ("TODO", "fix stuff", etc.)

#### Link audit
- [ ] Run `lychee --no-progress docs/ README.md CONTRIBUTING.md SECURITY.md SUSTAINABILITY.md`
- [ ] Zero broken links
- [ ] Add `lychee` link check to CI (run weekly, not on every PR)

#### README accuracy
- [ ] Status table reflects current component state (all completed items marked ✅)
- [ ] No references to features that don't exist yet
- [ ] Install instructions match `docs/getting-started.md`
- [ ] Discord invite link works
- [ ] Open Collective badge links to live page

#### Config sync check
- [ ] `config.example.toml` keys match `docs/config-reference.md` exactly (CI script passes)
- [ ] All default values in `config.example.toml` match defaults in Rust structs

---

### 🔴 v0.9.4 — RC5: Announcement Preparation
> **Gate:** Announcement assets ready; community notified of impending 1.0

- [ ] Blog post drafted: "Announcing idep v1.0"
- [ ] HN Show post drafted
- [ ] dev.to post drafted
- [ ] Reddit posts drafted (`r/rust`, `r/programming`)
- [ ] Indonesian community post drafted
- [ ] Discord `#announcements` post drafted
- [ ] `idep.dev` landing page updated for 1.0 launch
- [ ] OG image updated
- [ ] Community notified: "1.0 is coming in X days"

---

## 🎯 v1.0.0 — Stable

> This is a declaration, not a sprint. All the work is done in 0.9.x.

- [ ] All v0.9.x gates passed
- [ ] GitHub Release created with tag `v1.0.0`
- [ ] All announcement posts published simultaneously
- [ ] `plugin-api-v1` declared frozen in docs
- [ ] `CHANGELOG.md` entry for v1.0.0 written
- [ ] Celebrate 🌴

---

## 💡 Backlog (no version assigned)

- [ ] File DJKI trademark — Class 42 (software) ← Defensive Branding Framework
- [ ] Tolvex language syntax support
- [ ] Low-spec hardware build target (resource-constrained environments)
- [ ] Remote development mode (SSH)
- [ ] Collaborative editing (CRDT via `automerge-rs`)
- [ ] Mobile companion app (LSP over network)
- [ ] Windows native binary (non-WSL2)
- [ ] `cargo install idep` via crates.io
- [ ] Homebrew formula (macOS)
- [ ] `.deb` / `.rpm` packages