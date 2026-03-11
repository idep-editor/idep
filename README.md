# Idep

> **I**ntegrated **D**evelopment **E**nvironment, **P**owered  
> *idep* (Balinese) — Thought. Mind. Consciousness.

A lightweight, Rust-native AI-powered IDE.  
Built for developers who want speed, local-first AI, and full control.

```
Think in code.
```

---

## Why Idep?

| | Windsurf / Cursor | Zed | **Idep** |
|---|---|---|---|
| Runtime | Electron (VS Code fork) | Native (GPUI) | **Native (Rust)** |
| License | Proprietary | GPL-3 | **Apache 2.0** |
| AI backends | BYOK for individuals; proprietary models cloud-only | Anthropic, OpenAI, Ollama | **Any backend, any plan — no middleman** |
| Codebase RAG | Client-indexed, cloud-embedded | ❌ | **✅ fully in-process — embeddings never leave your machine** |
| WSL2 / Linux | Good | Good | **First-class** |
| Self-hostable | Enterprise plans only | Partial | **✅ fully — any plan** |

---

## Architecture

```
idep/
├── idep-core       — editor engine (buffer, workspace, LSP orchestration)
├── idep-ai         — AI layer (completions, chat, codebase indexer)
│   ├── backends/   — Anthropic · HuggingFace · Ollama · OpenAI-compat
│   ├── completion/ — FIM-aware inline completions
│   ├── chat/       — multi-turn conversation, context-aware
│   └── indexer/    — codebase RAG (tree-sitter + embeddings)
├── idep-lsp        — LSP client
├── idep-plugin     — WASM plugin SDK
└── idep-index      — vector index (fastembed-rs + usearch)
```

---

## Getting Started

```bash
# Clone
git clone https://github.com/idep-editor/idep
cd idep

# Build
cargo build

# Configure — copy and edit
cp config.example.toml ~/.idep/config.toml
```

### Minimal config (`~/.idep/config.toml`)

```toml
# Use local Ollama (no API key needed)
[ai]
provider = "ollama"
model    = "deepseek-coder:1.3b"
url      = "http://localhost:11434"

# Or Anthropic
# [ai]
# provider = "anthropic"
# api_key  = "sk-ant-..."
# model    = "claude-sonnet-4-20250514"
```

---

## Status

| Component | Status |
|---|---|
| `idep-ai` backends | 🟡 In progress |
| `idep-ai` completion | 🟡 In progress |
| `idep-ai` chat | 🟡 In progress |
| `idep-ai` indexer | 🔴 Planned (Phase 2) |
| `idep-core` | 🔴 Planned |
| `idep-plugin` | 🔴 Planned |

---

## Contributing

Idep is in early development. Issues and PRs welcome.  
See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Join the community on Discord: **https://discord.gg/PAKTq7YsMK**

---

## Sustainability

Idep is Apache 2.0, contributor-funded, and will never gate editor features
behind a subscription. See [SUSTAINABILITY.md](SUSTAINABILITY.md) for how the
project funds contributors and what it will never do to get there.

---

## License

Apache 2.0 — see [LICENSE](LICENSE)

---

*Built in Bali 🌴 by [@SHA888](https://github.com/SHA888)*  
*idep.dev*
