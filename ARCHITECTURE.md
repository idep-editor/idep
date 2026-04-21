# Idep — Architecture

> *Contributor-level introduction. This is the map. The detailed guide lives
> at `docs/architecture.md` (scoped for v0.7.4) with step-by-step recipes
> for adding backends, languages, and plugin hooks.*

---

## Crate map

| Crate | Responsibility | Key public types |
|---|---|---|
| `idep-core` | Buffer, workspace, cursor, file I/O, undo/redo | `Buffer`, `Workspace`, `Position` |
| `idep-ai` | AI backends, completion, chat, context engine | `Backend`, `CompletionEngine`, `ChatSession`, `ContextEngine` |
| `idep-lsp` | LSP client lifecycle, JSON-RPC transport, document sync | `LspClient`, `DocumentManager`, `Diagnostic` |
| `idep-index` | Tree-sitter chunking, embeddings, vector store | `AstChunker`, `Embedder`, `VectorStore`, `Indexer` |
| `idep-plugin` | WASM plugin host, sandbox, plugin API | `PluginHost`, `Plugin` trait |
| `idep-tui` | Terminal UI, input handling, rendering (v0.1.x) | `App`, `Event`, `Renderer` |

Planned: `idep-gui` (v0.2.x, after renderer spike), `idep-plugin-sdk` (v0.3.2, separate crate for plugin authors).

---

## Dependency rules

These are enforced by convention today, by `cargo deny` later (v0.6.4):

1. **`idep-core` depends on nothing in the workspace.** It is the foundation.
   No `idep-ai`, no `idep-lsp`, no rendering. Buffer operations must be
   testable without any AI or LSP infrastructure.
2. **`idep-ai`, `idep-lsp`, `idep-index` each depend only on `idep-core`.**
   They do not depend on each other. Cross-cutting concerns (e.g. RAG context
   for chat) are composed at the `idep-tui` / `idep-gui` layer.
3. **`idep-plugin` depends only on `idep-core` types that are exposed to plugins.**
   No AI, no LSP, no index exposed via the plugin ABI unless explicitly
   designed for it.
4. **`idep-tui` and `idep-gui` are the composition layer.** They wire
   everything together and own the application lifecycle.

Violations get caught at PR review. This layering is what lets the TUI and
GUI share the engine without duplication.

---

## High-level dependency graph

```
               ┌────────────────────────────────────────────────┐
               │          idep-tui  (v0.1.x)                    │
               │          idep-gui  (v0.2.x, planned)           │
               └──────┬──────┬──────┬──────┬──────┬──────┬──────┘
                      │      │      │      │      │      │
                      ▼      ▼      ▼      ▼      ▼      ▼
                ┌──────┐ ┌──────┐ ┌────────┐ ┌────────┐ ┌──────────┐
                │ ai   │ │ lsp  │ │ index  │ │ plugin │ │ (others) │
                └──┬───┘ └──┬───┘ └───┬────┘ └───┬────┘ └────┬─────┘
                   │        │         │          │           │
                   └────────┴─────────┴──────────┴───────────┘
                                      │
                                      ▼
                                ┌───────────┐
                                │ idep-core │
                                └───────────┘
```

Arrow direction = "depends on". `idep-core` has no workspace-internal dependencies.

---

## Core data flows

### 1. Keystroke → buffer update → render

```
user keypress
  → idep-tui event loop
  → App dispatches action based on KeybindingMap (v0.4.3)
  → idep-core::Buffer mutation (insert/delete/cursor-move)
  → Buffer pushes entry to undo history
  → idep-tui re-renders visible range
```

Buffer is the single source of truth. The TUI never keeps its own copy of
text; it reads from `Buffer::lines()` on render. This is what lets the GUI
port (v0.2.x) reuse the same engine with no changes.

### 2. Keystroke in Insert mode → inline completion ghost text

```
user keypress in Insert mode
  → idep-tui debounce timer starts (400ms)
  → debounce fires
  → idep-ai::CompletionEngine builds FIM prompt from cursor context
  → CompletionEngine calls configured Backend (e.g. OllamaBackend)
  → backend streams tokens
  → completion truncated on stop sequence
  → result handed back to idep-tui
  → ghost text rendered inline
  → Tab accepts → Buffer::insert; Esc dismisses
```

If another keypress arrives mid-request, the in-flight token is cancelled.
All backends use the same `Backend` trait, so swapping providers is a
config change.

### 3. File save → incremental reindex

```
Buffer::save() called
  → idep-core workspace fires FileWatcher event (notify crate, 100ms debounce)
  → idep-index::Indexer::reindex_file(path)
  → old chunks for that file removed from VectorStore + ChunkStore
  → AstChunker re-parses file via tree-sitter
  → EmbedPipeline produces new embeddings (fastembed-rs, local)
  → VectorStore::add() for each new chunk
  → persisted to ~/.idep/index/<project-hash>/
```

No network call at any point in this flow after the initial fastembed model
download. This is the claim that `tests/network_audit.rs` (v0.6.0) will
verify continuously.

### 4. Chat message → RAG context → streaming response

```
user submits chat message
  → idep-ai::ContextEngine::gather(query, cursor_file, cursor_pos)
      ├─ current file content
      ├─ AST subtree around cursor (tree-sitter)
      ├─ top-k similar chunks from VectorStore (cosine similarity)
      └─ recent edit history (last N saves)
  → context serialized with priority-based token budget truncation
      (priority: cursor context > similar chunks > history)
  → ChatSession::send_with_context() builds native message array
  → configured Backend streams response tokens
  → tokens appended to chat panel as they arrive
```

The context engine never sends chunks to a third party for retrieval. Only
the composed prompt goes to the configured AI backend — which the user chose
(Ollama = stays local; Anthropic = user-explicit cloud call).

### 5. LSP: file open → diagnostics

```
idep-core::Workspace::open_file()
  → idep-lsp::LspClient.didOpen() notification
  → server parses file
  → server sends textDocument/publishDiagnostics notification
  → LspClient stores diagnostics per URI (with WSL path normalization)
  → idep-tui queries get_diagnostics(uri) on next render
  → gutter markers and status bar count updated
```

WSL path normalization (v0.0.3) is load-bearing: LSP servers use
`file:///` URIs that look different depending on whether they're running
inside WSL2 (native Linux paths) or on the Windows host. The client
round-trips them transparently.

---

## Key design decisions

### Why `idep-core::Buffer` is the single source of truth

Ratatui and wgpu both pressure you toward owning your own text model. We
resisted. If the Buffer is authoritative, both renderers read from it and
neither needs to re-implement editing, undo, or cursor math. This is what
lets TUI ship at v0.1.0 and GUI reuse the engine at v0.2.2.

### Why TUI before GUI

The TUI forces the engine API to be clean. You cannot hide a sloppy
architecture behind pretty rendering in a terminal. By the time we start
the GUI renderer (v0.2.0), the engine is already stress-tested by a real
user-facing application. The renderer spike approach (egui vs wgpu, hard
timebox) is the second layer of the same insurance.

### Why tree-sitter AST chunking for RAG

Naive line chunking splits functions across chunk boundaries. The retrieved
chunks are then incomplete and the LLM hallucinates the missing parts. AST
chunking keeps function bodies together. Fallback to line chunking is
explicit for unsupported languages — not the default path.

### Why `fastembed-rs` (local) over an embedding API

Because the claim is "code never leaves the machine, *ever*". Calling a
remote embedding API at index time would violate it silently — the user
would not see the call, but every function in their codebase would cross
the network. `fastembed-rs` runs ONNX locally; the only network call is the
initial model download, which is explicit and cacheable.

### Why WASM for plugins (not native dynamic linking or subprocess)

Dynamic linking via `libloading` crate means a buggy plugin crashes the
editor. Subprocess plugins mean IPC overhead on every hook call. WASM via
`wasmtime` gives us: memory isolation, fuel-based CPU limits, per-plugin
sandboxing, and a single-language-agnostic ABI (v0.3.3 adds TypeScript SDK
reusing the same host). The trade-off is plugin size and startup time,
which are acceptable for our hook frequency.

### Why plugin API freeze is deferred

The original plan froze API at v0.3.0. That's too early — no plugin has
stress-tested it yet. The revised plan (see TODO.md v0.3.0 and v0.3.4)
ships v0.3.0 API as **experimental** and freezes it at **v0.3.4** after
the three example plugins (Rust SDK) and one TypeScript plugin have
exercised it. One more cycle of real use before committing is cheap
insurance against an expensive v2 migration.

### Why `<2 GB` RAM as a product line in the sand

Mainstream IDEs assume 16 GB as a floor. Our primary target user is on 4–8
GB. Every design decision — in-process embeddings with the smallest usable
model, buffer as single source of truth, WASM plugins with bounded memory,
TUI as a fully-supported mode — traces back to this constraint. If we lose
this, we lose our moat. See `PRODUCT.md` decision filter.

---

## Where to go next

- **Adding a new AI backend?** → See v0.7.4 guide (`docs/architecture.md` §
  "Adding a new AI backend"). Short version: implement `Backend` trait in
  `idep-ai/src/backends/<name>.rs`, add config enum variant, write mock-HTTP
  unit test + integration test.
- **Adding a new language to the indexer?** → Add tree-sitter grammar to
  `idep-ai` deps, add language detection, extend `AstChunker`, add tests.
- **Adding a new LSP method?** → Add request/response types in `idep-lsp`,
  add builder and parser, wire into `DocumentManager`, add integration test
  against `rust-analyzer` or equivalent.
- **Adding a new plugin host function?** → **Freeze risk.** Do not add
  after v0.3.4 without a v2 API discussion. See plugin versioning policy.

---

*Version: aligned with Idep v0.1.0, 2026-04-21. Updates to architectural
boundaries must be announced in `CHANGELOG.md`.*
