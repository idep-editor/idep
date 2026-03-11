# Idep — Development TODO

> Track: https://github.com/idep-editor/idep  
> Domain: idep.dev  
> Tagline: *Think in code.*

---

## 🔴 Phase 0 — Repo Hygiene (Do First)

- [x] Confirm `cargo check --all` passes clean on main
- [x] Add `LICENSE` file (Apache 2.0)
- [x] Add `CONTRIBUTING.md`
- [x] Add `CHANGELOG.md` (start at `v0.1.0-dev`)
- [x] Update `.gitignore` — remove `Cargo.lock` exclusion (binaries should commit lockfile)
- [x] Add `rust-toolchain.toml` to pin Rust version
- [x] Register `github.com/idep-editor/website` repo (placeholder for idep.dev)
- [ ] File DJKI trademark — Class 42 (software) ← per Defensive Branding Framework

---

## 🟡 Phase 1 — idep-ai: Make It Work (Weeks 1–2)

### Config schema (do before wiring backends)

The canonical config path is `~/.config/idep/config.toml` (XDG Base Dir spec).
All backends are selected and configured through the `[ai]` table.

```toml
[ai]
backend  = "ollama"          # ollama | anthropic | huggingface | openai
model    = "codellama:13b"
endpoint = "http://localhost:11434"   # optional — ollama / openai-compat only

[ai.auth]
api_key = "..."              # optional — anthropic / huggingface / openai only
```

- [x] Define and document config schema in `config.example.toml` (committed to repo)
- [x] Implement `Config` struct with serde deserialization from TOML
  - [x] `[ai].backend` — enum: `ollama | anthropic | huggingface | openai`
  - [x] `[ai].model` — string
  - [x] `[ai].endpoint` — optional string (URL override for ollama / openai-compat)
  - [x] `[ai.auth].api_key` — optional string (env var fallback: `IDEP_API_KEY`)
- [x] Resolve config path: XDG `~/.config/idep/config.toml` with fallback to `~/.idep/config.toml`
- [x] Add `config.example.toml` to repo root with all four backend examples

### Backends
- [x] Write integration test for `OllamaBackend` (requires local Ollama)
- [ ] Write unit test for `AnthropicBackend` (mock HTTP server)
- [ ] Write unit test for `HuggingFaceBackend` (mock HTTP server)
- [ ] Write unit test for `OpenAiCompatBackend` (mock HTTP server)
- [ ] Add retry logic with exponential backoff to all backends
- [ ] Add timeout configuration per backend
- [ ] Handle rate limit errors (429) gracefully with retry-after

### Completion
- [ ] Wire `CompletionEngine` → `llm-ls` LSP bridge
- [ ] Add debounce logic (configurable, default 300ms)
- [ ] Add stop-sequence handling (don't continue past function end)
- [ ] Test FIM tokens for DeepSeek, StarCoder, CodeLlama
- [ ] Benchmark: measure latency from keypress → first token

### Chat
- [ ] Upgrade `build_prompt()` to use native message arrays for Anthropic
- [ ] Add context window management (truncate history when approaching limit)
- [ ] Add `ChatSession::export()` — serialize conversation to JSON
- [ ] Add streaming token callback back to `send()` (removed in refactor)

### Indexer
- [ ] Replace naive line chunking with tree-sitter AST chunking
  - [ ] Rust: extract `fn`, `impl`, `struct`, `trait` nodes
  - [ ] TypeScript: extract `function`, `class`, `interface` nodes
  - [ ] Python: extract `def`, `class` nodes
- [ ] Integrate `fastembed-rs` for local embeddings (no network)
- [ ] Integrate `usearch` for in-process vector similarity search
- [ ] Implement incremental indexing (diff-based, not full re-walk)
- [ ] Respect `.gitignore` patterns during walk
- [ ] Persist index to `~/.idep/index/<project-hash>/` (survive restarts)

---

## 🟡 Phase 1 — idep-core: Buffer Basics (Weeks 2–3)

- [ ] Implement `Buffer::insert(pos, text)`
- [ ] Implement `Buffer::delete(range)`
- [ ] Implement `Buffer::lines() -> impl Iterator`
- [ ] Implement `Buffer::to_string()`
- [ ] Add cursor position tracking to `Buffer`
- [ ] Implement `Workspace::open_file(path) -> Buffer`
- [ ] Implement `Workspace::save_file(path, buffer)`
- [ ] Write unit tests for all buffer operations
- [ ] Add file watcher (notify crate) — trigger `Indexer::reindex_file` on save

---

## 🟡 Phase 1 — idep-lsp: Wire the Protocol (Weeks 2–4)

- [ ] Implement LSP client lifecycle: initialize → initialized → shutdown
- [ ] Spawn language server process (e.g. `rust-analyzer`, `typescript-language-server`)
- [ ] Handle `textDocument/didOpen`, `didChange`, `didSave`
- [ ] Handle `textDocument/completion` — bridge to `CompletionEngine`
- [ ] Handle `textDocument/hover`
- [ ] Handle `textDocument/definition`
- [ ] Handle `textDocument/publishDiagnostics`
- [ ] Add `llm-ls` as a virtual LSP for AI completions
- [ ] Write integration test: spawn `rust-analyzer`, get completions on a test file

---

## 🔵 Phase 2 — idep-plugin: WASM SDK (Month 2)

- [ ] Define plugin API surface (v1 — commit to stability)
  - [ ] `on_file_open(path, content)`
  - [ ] `on_file_save(path, content)`
  - [ ] `provide_completions(context) -> Vec<Completion>`
  - [ ] `register_command(name, handler)`
  - [ ] `open_panel(title, html_content)`
- [ ] Implement WASM host with `wasmtime`
- [ ] Write Rust plugin SDK (`idep-plugin` crate — targets `wasm32-unknown-unknown`)
- [ ] Write example plugin: `hello-world` (registers a command)
- [ ] Write example plugin: `word-count` (shows word count in status bar)
- [ ] Document plugin API
- [ ] Add TypeScript bindings for plugin SDK (for non-Rust plugin authors)

---

## 🔵 Phase 2 — idep-index: Upgrade Indexer (Month 2)

- [ ] Move `walk_and_chunk` from `idep-ai` → `idep-index`
- [ ] `fastembed-rs` embedding pipeline (batch processing)
- [ ] `usearch` vector index with persistence
- [ ] Expose query API: `find_similar(embedding, top_k) -> Vec<ScoredChunk>`
- [ ] Benchmark: index a 50k LOC Rust project, measure query latency

---

## 🔵 Phase 3 — Editor UI (Month 3)

> Decision point: egui (fast to implement) vs custom wgpu renderer (better long-term)

- [ ] Evaluate: spike egui-based editor view (1 week timebox)
- [ ] Evaluate: spike wgpu text renderer (1 week timebox)
- [ ] Decision: commit to one renderer
- [ ] Implement: basic text editing view (render buffer, cursor, selection)
- [ ] Implement: syntax highlighting (via tree-sitter highlight queries)
- [ ] Implement: file tree panel
- [ ] Implement: AI chat panel (streams tokens into UI)
- [ ] Implement: LSP diagnostic gutter (error/warning markers)
- [ ] Implement: inline completion ghost text

---

## 🔵 Phase 3 — Config & UX (Month 3)

- [ ] Implement config loader: `~/.config/idep/config.toml` → typed structs (schema defined in Phase 1)
- [ ] Implement config validation with clear error messages
- [ ] Add `idep --check-config` CLI command
- [ ] Add `idep --version` CLI command
- [ ] Add first-run wizard: detect Ollama, suggest model download
- [ ] Add keybinding system (load from `~/.config/idep/keybindings.toml`)
- [ ] Hot-reload config on file change (switch backends without restarting)

---

## 🌐 Website (Parallel track)

- [x] Create `github.com/idep-editor/website`
- [x] Deploy to `idep-website.vercel.app` (Astro + Tailwind, pending `idep.dev` DNS)
- [x] Page: Landing (tagline, why Idep, interactive backend config switcher)
- [ ] Page: Docs (getting started, full config reference)
- [ ] Page: Roadmap (public-facing version of this TODO)
- [ ] Set up `idep.dev` DNS → Vercel

---

## 📦 Release Checklist (Before v0.1.0-alpha)

- [ ] `cargo check --all` passes
- [ ] `cargo test --all` passes
- [ ] `cargo clippy --all -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] CI green on main
- [ ] README status table updated
- [ ] CHANGELOG updated
- [ ] GitHub Release created with binary artifacts (via `cargo-dist` or `release-plz`)
- [ ] Announce on: dev.to, Reddit r/rust, Hacker News (Show HN)

---

## 💡 Backlog / Future

- [ ] HyQAI integration — quantum-classical hybrid code suggestion (CiptaSel P1 angle)
- [ ] Tolvex/Idep integration — syntax support for Tolvex language
- [ ] Balinese developer community outreach (ASEAN first-mover)
- [ ] TeknoRakit educational edition — lightweight build for low-spec hardware
- [ ] Remote development mode (SSH into a server, edit locally)
- [ ] Collaborative editing (CRDT-based, using automerge-rs)
- [ ] Mobile companion app (view/edit via Idep's LSP over network)

---

## 🗓 Milestones

| Milestone | Target | Criteria |
|---|---|---|
| `v0.0.1` | Week 1 | `cargo build --all` passes, CI green |
| `v0.0.2` — AI works | Week 3 | Ollama completions working end-to-end |
| `v0.0.3` — LSP works | Week 5 | rust-analyzer completions in buffer |
| `v0.1.0-alpha` | Month 2 | Usable for basic Rust editing with AI |
| `v0.2.0-beta` | Month 4 | Plugin system, full UI, public announcement |
| `v1.0.0` | Month 6 | Stable API, docs complete, community |
