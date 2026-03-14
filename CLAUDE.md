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

## SDLC & Development workflow

### Version control & branching
- **Main branch**: `main` — stable, always green CI, protected
- **Feature branches**: `v0.0.x-feature-name` (e.g., `v0.0.4-lsp-completions-in-buffer`)
- **Branch protection**: Require PR review, CI pass, no force-push to main
- **Commit messages**: Conventional commits preferred (e.g., `feat:`, `fix:`, `docs:`)

### Pull request process
1. **Create PR** from feature branch to main
2. **CI checks** run automatically (fmt, clippy, test)
3. **Code review** by maintainer or @claude
4. **Address feedback** via additional commits
5. **Squash merge** to main after approval
6. **Delete branch** after merge

### Release cycle

**Versioning** (follows [Semantic Versioning](https://semver.org/)):
- **0.0.x** (pre-alpha): Breaking changes allowed, no stability guarantees
- **0.x.y** (alpha/beta): Minor breaking changes possible, documented in CHANGELOG
- **1.0.0+** (stable): MAJOR.MINOR.PATCH
  - MAJOR: Breaking API changes
  - MINOR: New features, backward compatible
  - PATCH: Bug fixes, backward compatible

**Release preparation**:
1. Update `Cargo.toml` workspace version
2. Update `CHANGELOG.md` with release notes (group by Added/Changed/Fixed/Removed)
3. Run full test suite locally: `cargo test --all`
4. Create PR with version bump (title: `Release v0.0.x`)

**Release execution**:
1. Merge version bump PR to main
2. Tag release: `git tag v0.0.x && git push origin v0.0.x`
3. GitHub Actions builds and publishes artifacts
4. Create GitHub Release with notes from CHANGELOG
5. (Future) Publish to crates.io: `cargo publish -p idep`

### Quality gates

**Pre-commit hooks** (via `.pre-commit-config.yaml`):
- `cargo fmt` — auto-format (blocking)
- `cargo clippy -D warnings` — zero warnings policy (blocking)
- `cargo test` — all tests must pass (blocking)

**CI pipeline** (`.github/workflows/ci.yml`):
- Runs on: all PRs, pushes to main, tags
- Jobs: check, fmt, clippy, test
- Installs rust-analyzer for integration tests
- Caches dependencies for speed

**Environment variables for tests**:
- `RUN_RA_INT=1` — enable rust-analyzer integration tests
- `RUN_WSL_RA_TEST=1` — enable WSL rust-analyzer tests
- `ANTHROPIC_API_KEY` — for live AI completion tests (gated)

### Issue tracking
- **GitHub Issues** for bugs, features, questions
- **Labels**: `bug`, `enhancement`, `documentation`, `good first issue`
- **Milestones**: One per version (e.g., `v0.0.5`)
- **Projects**: Track progress on major features (e.g., "LSP Integration")

### Documentation
- **README.md**: Project overview, quick start, links
- **CONTRIBUTING.md**: How to contribute, code style, PR process
- **CHANGELOG.md**: Release notes, breaking changes
- **CLAUDE.md**: This file — review context for AI and humans
- **TODO.md**: Development roadmap, version gates
- **Inline docs**: Public APIs documented with `///` comments

## Review focus

### Correctness & safety
- Avoid panics in prod paths (no `unwrap`/`expect` on I/O or network)
- Propagate errors with `anyhow::Result` or typed errors where present
- Respect async boundaries; no blocking in async paths
- Check error paths: does the code handle file not found, network timeout, malformed JSON?

### Concurrency & IO
- For notify watchers, debounce where appropriate; avoid leaking threads
- Ensure file handles are closed; prefer `tokio::fs` in async contexts
- No blocking I/O in async functions (use `tokio::task::spawn_blocking` if needed)

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

### Testing strategy

**Unit tests** (fast, no external deps):
- Buffer ops: insert/delete/lines, cursor positioning
- Workspace: open/save, path normalization
- Config: XDG resolution, env fallback
- Completion ranking: sort_text, deduplication

**Integration tests** (gated by env vars):
- `RUN_RA_INT=1` — rust-analyzer LSP handshake, completions
- `RUN_WSL_RA_TEST=1` — WSL path normalization with rust-analyzer
- Python-dependent tests check for `python3` availability before running

**Backend tests** (use `httpmock`):
- Mock HTTP responses, no real network calls
- Test streaming, stop-sequences, error handling
- Verify FIM token formats per model

**Regression tests**:
- Add tests for every bug fix (e.g., textEdit range handling, cursor shadowing)
- Test edge cases: empty buffers, malformed JSON, network timeouts

### Code style

**Formatting & linting**:
- `cargo fmt` — enforced by pre-commit hook
- `cargo clippy -D warnings` — zero warnings policy
- No `#[allow]` attributes without documented justification

**Code organization**:
- Small, composable functions (prefer <50 lines)
- Early returns over deep nesting
- Public APIs get doc comments; internal logic only when non-obvious
- Group related functionality in modules (e.g., `completion.rs` for ranking/bridging)

**Error handling**:
- Use `anyhow::Result` for application errors
- Use typed errors (`thiserror`) for library APIs
- Context on errors: `.context("Failed to parse completion response")`
- Never `unwrap`/`expect` on I/O, network, or user input

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

## Deployment & distribution

**Current state**: Pre-alpha (0.0.x) — not yet ready for end users. Building core subsystems.

**Future distribution** (0.4.x+):
- Binary releases via GitHub Releases
- Package managers: Homebrew (macOS), apt/snap (Linux), Scoop/Chocolatey (Windows)
- Build from source: `cargo install idep` (when published to crates.io)

**Platform support**:
- Primary: Linux (native), macOS (native)
- Secondary: Windows (native), WSL2 (with path normalization)
- CI: GitHub Actions on ubuntu-latest

**Dependencies**:
- Rust toolchain (specified in `rust-toolchain.toml`)
- Optional: `rust-analyzer` for LSP integration tests (installed via `rustup component add rust-analyzer`)
- Optional: `python3` for document sync mock tests (CI may skip if unavailable)

**Configuration**:
- Config file: `~/.config/idep/config.toml` (XDG) or `$IDEP_CONFIG_PATH`
- API keys: Environment variables (`ANTHROPIC_API_KEY`, etc.) or config file
- See `config.example.toml` for reference
