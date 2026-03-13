# Changelog

All notable changes to Idep are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased] — v0.0.4

### Planned
- LSP server integration (`textDocument/completion`)
- End-to-end LSP completion flow
- Buffer sync (`didOpen`/`didChange`/`didSave`) end-to-end

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
