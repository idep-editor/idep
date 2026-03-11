# Idep — Development TODO

> Track: https://github.com/idep-editor/idep  
> Domain: idep.dev  
> Tagline: *Think in code. Own your tools.*

---

## Version Map

| Version | Status | Target | Criteria |
|---|---|---|---|
| `v0.0.1` | ✅ Shipped | Week 1 | `cargo build --all` passes, CI green |
| `v0.0.2` | 🟡 In progress | Week 3 | Ollama completions working end-to-end |
| `v0.0.3` | 🔴 Planned | Week 5 | rust-analyzer completions in buffer |
| `v0.1.0-alpha` | 🔴 Planned | Month 2 | Usable for basic Rust editing with AI |
| `v0.2.0-beta` | 🔴 Planned | Month 4 | Plugin system, full UI, public announcement |
| `v1.0.0` | 🔴 Planned | Month 6 | Stable API, docs complete, community |

---

## ✅ v0.0.1 — Repo Foundation
> `cargo build --all` passes · CI green · **Shipped Week 1**

### Repo hygiene
- [x] `cargo check --all` passes clean on main
- [x] `LICENSE` file (Apache 2.0)
- [x] `CONTRIBUTING.md`
- [x] `CHANGELOG.md` (start at `v0.1.0-dev`)
- [x] `.gitignore` — `Cargo.lock` included (binaries commit lockfile)
- [x] `rust-toolchain.toml` — Rust version pinned
- [x] `SUSTAINABILITY.md` — contribution model, what Idep will never do
- [x] `SECURITY.md` — threat model, local-first security guarantees

### Config schema
- [x] Config schema defined and documented in `config.example.toml`
- [x] `Config` struct with serde TOML deserialization
  - [x] `[ai].backend` — enum: `ollama | anthropic | huggingface | openai`
  - [x] `[ai].model` — string
  - [x] `[ai].endpoint` — optional URL override
  - [x] `[ai.auth].api_key` — optional, env var fallback `IDEP_API_KEY`
- [x] XDG config resolution: `~/.config/idep/config.toml` → `~/.idep/config.toml`

### AI backends — unit coverage
- [x] `OllamaBackend` — integration test (requires local Ollama)
- [x] `AnthropicBackend` — unit test (mock HTTP server)
- [x] `HuggingFaceBackend` — unit test (mock HTTP server)
- [x] `OpenAiCompatBackend` — unit test (mock HTTP server)
- [x] Retry logic with exponential backoff — all backends
- [x] Timeout configuration per backend
- [x] Rate limit (429) handling with retry-after

### Website
- [x] `github.com/idep-editor/website` created
- [x] Deployed to `idep-website.vercel.app`
- [x] Landing page: tagline, comparison table, backend config switcher

### Pending (no version gate)
- [ ] File DJKI trademark — Class 42 (software) ← per Defensive Branding Framework

---

## 🟡 v0.0.2 — AI Works End-to-End
> Ollama completions working · **Target: Week 3**  
> 📣 GitHub Sponsors goes live (quiet, no announcement)

### `idep-ai` — Completion
- [x] Wire `CompletionEngine` → `llm-ls` LSP bridge
- [x] Debounce logic (configurable, default 300ms)
- [x] Stop-sequence handling (don't continue past function end)
- [x] FIM token validation: DeepSeek · StarCoder · CodeLlama
- [x] Benchmark: keypress → first token latency

### `idep-ai` — Chat (Ollama-only scope)
- [x] Streaming token callback restored to `send()` — required for Ollama completions
- [x] Debounce wired through to chat context (configurable, default 300ms)

### `idep-core` — Buffer basics
- [x] `Buffer::insert(pos, text)`
- [x] `Buffer::delete(range)`
- [x] `Buffer::lines() -> impl Iterator`
- [x] `Buffer::to_string()`
- [x] Cursor position tracking
- [ ] `Workspace::open_file(path) -> Buffer`
- [ ] `Workspace::save_file(path, buffer)`
- [x] Unit tests for all buffer operations
- [ ] File watcher (`notify` crate) → trigger `Indexer::reindex_file` on save

### Website — v0.0.2
- [ ] GitHub Sponsors badge: update from "Live at v0.0.2" → active link
- [ ] README status table updated

---

## 🔴 v0.0.3 — LSP Works
> rust-analyzer completions in buffer · **Target: Week 5**

### `idep-lsp`
- [ ] LSP client lifecycle: `initialize` → `initialized` → `shutdown`
- [ ] Spawn language server (`rust-analyzer`, `typescript-language-server`)
- [ ] `textDocument/didOpen`, `didChange`, `didSave`
- [ ] `textDocument/completion` → bridge to `CompletionEngine`
- [ ] `textDocument/hover`
- [ ] `textDocument/definition`
- [ ] `textDocument/publishDiagnostics`
- [ ] `llm-ls` wired as virtual LSP for AI completions
- [ ] Integration test: spawn `rust-analyzer`, get completions on test file

### `idep-ai` — Indexer
- [ ] tree-sitter AST chunking (replace naive line chunking)
  - [ ] Rust: `fn`, `impl`, `struct`, `trait`
  - [ ] TypeScript: `function`, `class`, `interface`
  - [ ] Python: `def`, `class`
- [ ] `fastembed-rs` — local embeddings, no network
- [ ] `usearch` — in-process vector similarity search
- [ ] Incremental indexing (diff-based, not full re-walk)
- [ ] Respect `.gitignore` patterns during walk
- [ ] Persist index to `~/.idep/index/<project-hash>/`

---

## 🔴 v0.1.0-alpha — Usable for Rust Editing
> Basic Rust editing with AI · **Target: Month 2**

### `idep-ai` — Chat (all backends)
- [ ] Native message arrays for Anthropic (replace `build_prompt()`)
- [ ] Context window management (truncate history near limit)
- [ ] `ChatSession::export()` — serialize to JSON

### Release gate
- [ ] `cargo check --all` passes
- [ ] `cargo test --all` passes
- [ ] `cargo clippy --all -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] CI green on main
- [ ] CHANGELOG updated
- [ ] GitHub Release with binary artifacts (`cargo-dist` or `release-plz`)

### `idep-index` — Upgrade indexer
- [ ] Move `walk_and_chunk` from `idep-ai` → `idep-index`
- [ ] `fastembed-rs` batch embedding pipeline
- [ ] `usearch` with persistence
- [ ] Query API: `find_similar(embedding, top_k) -> Vec<ScoredChunk>`
- [ ] Benchmark: index 50k LOC Rust project, measure query latency

### Website — v0.1.0-alpha
- [ ] Page: Docs (getting started, full config reference)
- [ ] Set up `idep.dev` DNS → Cloudflare Pages

---

## 🔴 v0.2.0-beta — Public Launch
> Plugin system · Full UI · **Target: Month 4**  
> 📣 Open Collective goes live · Announce: dev.to · Reddit r/rust · Hacker News (Show HN)

### `idep-plugin` — WASM SDK
- [ ] Plugin API surface v1 (commit to stability)
  - [ ] `on_file_open(path, content)`
  - [ ] `on_file_save(path, content)`
  - [ ] `provide_completions(context) -> Vec<Completion>`
  - [ ] `register_command(name, handler)`
  - [ ] `open_panel(title, html_content)`
- [ ] WASM host with `wasmtime`
- [ ] Rust plugin SDK (`idep-plugin` → `wasm32-unknown-unknown`)
- [ ] Example plugin: `hello-world`
- [ ] Example plugin: `word-count`
- [ ] Plugin API docs
- [ ] TypeScript bindings for plugin SDK

### Editor UI
> Spike both renderers before committing — 1 week each

- [ ] Spike: egui-based editor view (1-week timebox)
- [ ] Spike: wgpu text renderer (1-week timebox)
- [ ] Decision: commit to one renderer
- [ ] Basic text editing view (buffer, cursor, selection)
- [ ] Syntax highlighting (tree-sitter highlight queries)
- [ ] File tree panel
- [ ] AI chat panel (streams tokens)
- [ ] LSP diagnostic gutter (error/warning markers)
- [ ] Inline completion ghost text

### Config & UX
- [ ] Config validation with clear error messages
- [ ] `idep --check-config` CLI command
- [ ] `idep --version` CLI command
- [ ] First-run wizard: detect Ollama, suggest model download
- [ ] Keybinding system (`~/.config/idep/keybindings.toml`)
- [ ] Hot-reload config on file change

### Website — v0.2.0-beta
- [ ] Page: Roadmap (public-facing version of this TODO)
- [ ] Open Collective badge: update from "Live at v0.2.0-beta" → active link
- [ ] Competitive benchmark page: Idep vs Antigravity (RAM, startup time, latency)

---

## 🔴 v1.0.0 — Stable
> Stable API · Docs complete · Community · **Target: Month 6**

- [ ] All public API surfaces documented
- [ ] `idep-plugin` v1 API frozen
- [ ] Contribution guide complete
- [ ] Community Discord active
- [ ] "Why not Antigravity?" comparison doc published
- [ ] Balinese / ASEAN developer community outreach

---

## 💡 Backlog (no version assigned)

- [ ] HyQAI integration — quantum-classical hybrid suggestions (CiptaSel)
- [ ] Tolvex language syntax support
- [ ] TeknoRakit educational edition — low-spec hardware build
- [ ] Remote development mode (SSH)
- [ ] Collaborative editing (CRDT via `automerge-rs`)
- [ ] Mobile companion app (LSP over network)
