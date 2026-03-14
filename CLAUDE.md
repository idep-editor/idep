<!-- Last updated: main branch (v0.0.4 merged) — revisit when v0.0.5 lands -->

# Claude Review Context

Use this guide when reviewing pull requests for Idep.

## CI requirements
- Requires `ANTHROPIC_API_KEY` repo secret for Claude reviews via GitHub Actions
- All PRs must pass `cargo test`, `cargo clippy -D warnings`, and `cargo fmt`
- GitHub Action usage docs: https://github.com/anthropics/claude-code-action/blob/main/docs/usage.md

## Project snapshot

**Vision**: Idep is a code editor that thinks with you. AI-native, LSP-powered, built for developers who want to own their tools.

**Current phase**: 0.0.x (headless core) — building foundational subsystems before UI. v0.0.4 shipped with LSP completions in buffer.

**Architecture**:
- **idep-core**: Buffer (ropey), workspace (file I/O, watching), config (XDG + env)
- **idep-ai**: Completion engine with multiple backends (Anthropic, OpenAI, HuggingFace, Ollama), FIM support, debouncing
- **idep-lsp**: LSP client with document sync, completions, path normalization (WSL2-aware)
- **idep-index**: (Planned) Code indexing for semantic search
- **idep-plugin**: (Planned) Plugin system

**Tech stack**:
- Rust 2021 edition, tokio async runtime
- Text buffer: `ropey` (char-indexed rope)
- File watching: `notify` with debouncing
- LSP: Custom client with rust-analyzer integration tests
- AI: Direct API calls with streaming support

## Review focus

### Correctness & safety
- Avoid panics in prod paths (no `unwrap`/`expect` on I/O or network)
- Propagate errors with `anyhow::Result` or typed errors where present
- Respect async boundaries; no blocking in async paths

### Concurrency & IO
- For notify watchers, debounce where appropriate; avoid leaking threads
- Ensure file handles are closed; prefer `tokio::fs` in async contexts

### API contracts

**AI backends**:
- `Backend::as_any` — required for trait-object downcasting (Rust has no built-in downcast on dyn Trait); used exclusively for Ollama streaming; other backends can return `self` trivially
- Completion stop-sequence truncation must not overrun buffer
- Chat streaming should surface token callbacks safely
- FIM (Fill-In-Middle) tokens must match model-specific formats (CodeLlama, StarCoder, DeepSeek)

**LSP client**:
- `CompletionItem.text_edit` must delete the specified range before inserting. Never insert at cursor when textEdit is present—this causes doubled text
- Respect `CompletionItem.sort_text` for ranking; it's server-controlled ordering. Only fall back to label length when sort_text is absent
- Document versions must increment on each change notification

**Buffer operations**:
- All edits must update cursor position via `update_cursor`
- Index exclusively via char positions, never byte offsets (ropey panics on non-char boundaries)

### Testing
- Unit tests for buffer ops (insert/delete/lines), workspace open/save, and watchers
- Backend tests use `httpmock`; avoid real network
- Add regression tests for parsing/streaming and stop-sequences
- **LSP tests**: Mock servers for document sync and completion sequences; gate python3-dependent tests with availability checks
- **Integration tests**: rust-analyzer tests gated by `RUN_RA_INT=1` env var to avoid CI flake

### Style & docs
- Run `cargo fmt` and `cargo clippy -D warnings` before committing
- Favor small, composable functions; prefer early returns
- Document public APIs briefly; add comments only for non-obvious logic

## Stack specifics

### ropey
- Index exclusively via `char_idx` / line APIs; byte indexing into a `Rope` panics on non-char boundaries and silently corrupts cursor state
- Update cursor after all edits (insert/delete)

### notify
- Watch is edge-triggered; ensure callbacks are cheap
- Consider debounce if watcher fires on noisy directories

### Config
- XDG base dirs first, env-var fallback second
- Tests must cover both paths

### Completion & Chat
- Debounce (300ms default) applies to both `send` and `send_streaming`
- Tests should verify the timer resets on repeated calls, not just fires once
- Stop-sequences must truncate output cleanly without buffer overrun

### LSP (idep-lsp)
- **Path normalization**: WSL2 environments require converting `file:///C:/...` ↔ `file:///mnt/c/...`. Use `path::to_server_uri` for server-bound URIs
- **Document sync**: Send `didOpen`, `didChange`, `didSave`, `didClose` in correct sequence. Track versions per document
- **Completion ranking**: Use `BTreeMap` for deterministic deduplication by label. Sort by `sort_text` first (server intent), then label length, then lexicographic
- **textEdit handling**: Apply range deletion before insertion. Use `Buffer::apply_text_edit(range, new_text)` for proper replacement
- **Cursor positioning**: `update_cursor` must clamp to last character index (line_len - 1), accounting for trailing newlines. Avoid shadowing variables
