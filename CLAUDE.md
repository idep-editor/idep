<!-- Last updated: v0.0.4 — revisit when Indexer or new LSP verbs land -->

# Claude Review Context

Use this guide when reviewing pull requests for Idep.

## CI requirements
- Requires `ANTHROPIC_API_KEY` repo secret for Claude reviews via GitHub Actions
- All PRs must pass `cargo test`, `cargo clippy -D warnings`, and `cargo fmt`
- GitHub Action usage docs: https://github.com/anthropics/claude-code-action/blob/main/docs/usage.md

## Project snapshot
- Language: Rust 2021
- Async runtime: tokio
- Text buffer: `ropey`
- File watching: `notify`
- AI backends: Anthropic, OpenAI-compatible, HuggingFace, Ollama
- LSP bridge: `idep-lsp` → CompletionEngine

## Review focus

### Correctness & safety
- Avoid panics in prod paths (no `unwrap`/`expect` on I/O or network)
- Propagate errors with `anyhow::Result` or typed errors where present
- Respect async boundaries; no blocking in async paths

### Concurrency & IO
- For notify watchers, debounce where appropriate; avoid leaking threads
- Ensure file handles are closed; prefer `tokio::fs` in async contexts

### API contracts
- `Backend::as_any` — required for trait-object downcasting (Rust has no built-in downcast on dyn Trait); used exclusively for Ollama streaming; other backends can return `self` trivially
- Completion stop-sequence truncation must not overrun buffer
- Chat streaming should surface token callbacks safely

### Testing
- Unit tests for buffer ops (insert/delete/lines), workspace open/save, and watchers
- Backend tests use `httpmock`; avoid real network
- Add regression tests for parsing/streaming and stop-sequences

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
