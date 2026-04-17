# Idep

> **I**ntegrated **D**evelopment **E**nvironment, **P**owered  
> *idep* (Balinese) — Thought. Mind. Consciousness.

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org)
[![CI](https://github.com/idep-editor/idep/actions/workflows/ci.yml/badge.svg)](https://github.com/idep-editor/idep/actions/workflows/ci.yml)
[![Build status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/idep-editor/idep/actions)
[![Sponsor](https://img.shields.io/github/sponsors/idep-editor)](https://github.com/sponsors/idep-editor)

> ⚠️ **Alpha — no release binary yet.**  
> v0.1.0: Terminal editor functional. v0.2.0 will add syntax highlighting.  
> Follow along or contribute — see [TODO.md](TODO.md) for what's next.

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
| License | Proprietary | GPL-3 / AGPL-3 | Proprietary | **Apache 2.0** |
| AI paradigm | Inline assist | Inline assist | **Agent orchestration** | **Precise completion + RAG** |
| AI backends | BYOK (cloud-locked models) | Multi-provider | Gemini-first + Claude/GPT | **Any — no middleman** |
| Codebase RAG | Local index, cloud inference | ❌ | ❌ | **✅ fully in-process** |
| Cloud dependency | Moderate | Low | **Hard (Google account)** | **None** |
| RAM floor | 4–8GB | ~4GB | 16GB recommended | **~2GB target** |
| WSL2 / Linux | Good | Good | Okay | **First-class** |
| Self-hostable | Enterprise only | Partial | ❌ | **✅ fully** |
| Open source | ❌ | GPL-3 / AGPL-3 | ❌ | **Apache 2.0** |

---

## Positioning

The IDE market is splitting in two directions:

**Upward** — tools like Google Antigravity are becoming agent orchestration platforms.
You delegate features to autonomous agents. The IDE becomes Mission Control.
Powerful for high-level product work. Requires cloud. Requires trust. Requires Google.

**Downward** — Idep goes the other way. Native runtime, local inference, in-process RAG.
You remain the thinker. The tool disappears. Your codebase never leaves your machine.

Idep is for developers who want thought-level control, not agent-level delegation.
Your codebase index runs in-process — not just locally, but never touching a network
path even for embedding. That's a sharper claim than any other tool in this table.

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

### Prerequisites

**Ubuntu / Debian:**
```bash
sudo apt-get install libssl-dev pkg-config
```

**Fedora / RHEL:**
```bash
sudo dnf install openssl-devel pkgconfig
```

**macOS:** OpenSSL and pkg-config are usually pre-installed.

### Terminal Requirements

**Windows (WSL2):**
- Windows Terminal 1.18+ (required for mouse support and truecolor)
- Windows 10 version 1903+ (build 18362+)

**Linux:**
- Any terminal with 256-color or truecolor support
- Mouse support requires terminal with X10 or SGR 1006 mouse protocol

**macOS:**
- Terminal.app (basic support) or iTerm2 3.4+ (recommended)

### Build

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

## Examples

### Ollama Smoketest

Quick validation that the Ollama backend and FIM completion pipeline work end-to-end.

```bash
# Start Ollama (if not already running)
ollama serve

# In another terminal, run the smoketest
bash example/ollama-smoketest.md
```

This validates:
- ✅ Ollama connectivity and model availability
- ✅ FIM token format (DeepSeek, StarCoder, CodeLlama)
- ✅ Stop-sequence handling (prevents generation past function boundary)
- ✅ Streaming token collection
- ✅ Deterministic completion (`temperature: 0`)

See [example/ollama-smoketest.md](example/ollama-smoketest.md) for detailed steps and troubleshooting.

---

## Status

| Component | Status |
|---|---|
| `idep-ai` backends | ✅ Complete (v0.0.1) |
| `idep-ai` completion | ✅ Complete (v0.0.2) |
| `idep-ai` chat | ✅ Complete (v0.0.2) |
| `idep-core` buffer | ✅ Complete (v0.0.2) |
| `idep-core` workspace | ✅ Complete (v0.0.2) |
| `idep-lsp` bridge | ✅ Complete (v0.0.2) |
| `idep-tui` editor | ✅ Complete (v0.1.0) |

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
*[idep.dev](https://idep.dev)*
