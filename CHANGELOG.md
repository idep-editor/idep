# Changelog

All notable changes to Idep are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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

---

## [Unreleased] — v0.0.2

### Planned
- `CompletionEngine` → `llm-ls` LSP bridge
- Ollama completions end-to-end
- `idep-core` buffer primitives
- `ChatSession` streaming and export
