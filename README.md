# Idep

> **I**ntegrated **D**evelopment **E**nvironment, **P**owered  
> *idep* (Balinese) — Thought. Mind. Consciousness.

A lightweight, Rust-native AI-powered IDE.  
Built for developers who want speed, local-first AI, and full control —
not another cloud platform that thinks for you.

```
Think in code. Own your tools.
```

---

## Why Idep?

| | Windsurf / Cursor | Zed | Google Antigravity | **Idep** |
|---|---|---|---|---|
| Runtime | Electron (VS Code fork) | Native (GPUI) | Electron (VS Code fork) | **Native (Rust)** |
| License | Proprietary | AGPL-3 | Proprietary | **Apache 2.0** |
| AI paradigm | Inline assist | Inline assist | **Agent orchestration** | **Precise completion + RAG** |
| AI backends | BYOK (cloud-locked models) | Anthropic, OpenAI, Ollama | Gemini-first + Claude/GPT | **Any — no middleman** |
| Codebase RAG | Cloud-indexed | ❌ | ❌ | **✅ local, in-process** |
| Cloud dependency | Moderate | Low | **Hard (Google account)** | **None** |
| RAM floor | ~8GB+ | ~4GB | 16GB recommended | **~2GB target** |
| WSL2 / Linux | Good | Good | Okay | **First-class** |
| Self-hostable | Enterprise only | Partial | ❌ | **✅ fully** |
| Open source | ❌ | AGPL-3 | ❌ | **Apache 2.0** |

---

## Positioning

The IDE market is splitting in two directions:

**Upward** — tools like Google Antigravity are becoming agent orchestration platforms.
You delegate features to autonomous agents. The IDE becomes Mission Control.
Powerful for high-level product work. Requires cloud. Requires trust. Requires Google.

**Downward** — Idep goes the other way. Native runtime, local inference, in-process RAG.
You remain the thinker. The tool disappears. Your codebase never leaves your machine.

Idep is for developers who want thought-level control, not agent-level delegation.

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
mkdir -p ~/.config/idep
cp config.example.toml ~/.config/idep/config.toml
```

### Config reference (`~/.config/idep/config.toml`)

Switch AI backends by changing a single line. No restart required.

**Ollama (local, no API key)**
```toml
[ai]
backend  = "ollama"
model    = "codellama:13b"
endpoint = "http://localhost:11434"
```

**Anthropic**
```toml
[ai]
backend = "anthropic"
model   = "claude-haiku-4-5-20251001"

[ai.auth]
api_key = "sk-ant-..."
```

**HuggingFace**
```toml
[ai]
backend = "huggingface"
model   = "bigcode/starcoder2-15b"

[ai.auth]
api_key = "hf_..."
```

**Any OpenAI-compatible endpoint** (GPT-4o, Groq, Together, LM Studio…)
```toml
[ai]
backend  = "openai"
model    = "gpt-4o-mini"
endpoint = "https://api.groq.com/openai/v1"

[ai.auth]
api_key = "gsk_..."
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
