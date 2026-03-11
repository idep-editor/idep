# Ollama Smoke Test

Verify your Ollama setup works correctly with Idep before diving into development.

---

## Prerequisites

- [Ollama](https://ollama.com) installed and running
- A FIM-capable model pulled (see below)
- `curl` available in your shell

---

## 1. Install & Start Ollama

```bash
# Linux
curl -fsSL https://ollama.com/install.sh | sh

# macOS
brew install ollama

# Start the server
ollama serve
```

Verify it's up:

```bash
curl http://localhost:11434/api/tags
# Expected: {"models":[...]}
```

---

## 2. Pull a Model

Idep uses FIM (fill-in-the-middle) for completions. Use a model that supports it.

| Model | RAM | FIM format |
|---|---|---|
| `deepseek-coder:1.3b` | ~1.5GB | DeepSeek tokens |
| `deepseek-coder:6.7b` | ~4GB | DeepSeek tokens |
| `codellama:7b` | ~4GB | CodeLlama tokens |
| `starcoder2:3b` | ~2GB | StarCoder tokens |

```bash
ollama pull deepseek-coder:1.3b
```

---

## 3. Smoke Test — Plain Completion

Confirm the model responds at all:

```bash
curl -s http://localhost:11434/api/generate \
  -d '{"model":"deepseek-coder:1.3b","prompt":"fn add(a: i32, b: i32) -> i32 {","stream":false}' \
  | jq '.response'
```

Expected: some generated text (may be chat-style at this point — that's fine).

---

## 4. Smoke Test — FIM Completion

This is the mode Idep actually uses. Three flags are critical:

- `raw: true` — bypasses Ollama's chat template, preserves FIM tokens
- `temperature: 0` — deterministic output, no prose
- `stop` — model-specific stop sequences

```bash
curl -s http://localhost:11434/api/generate -d '{
  "model": "deepseek-coder:1.3b",
  "prompt": "<｜fim▁begin｜>fn add(a: i32, b: i32) -> i32 {\n<｜fim▁hole｜>\n}<｜fim▁end｜>",
  "stream": false,
  "raw": true,
  "options": {
    "temperature": 0,
    "stop": ["}\n", "<｜fim▁end｜>", "<｜end▁of▁sentence｜>"]
  }
}' | jq '.response'
```

Expected output:

```
"    a + b\n"
```

The model fills in only the function body and stops. If you see prose like "Here is a function that adds two integers...", `raw: true` is not being applied correctly.

---

## 5. Smoke Test — Streaming FIM

Same as above but with streaming enabled, which is how Idep delivers inline completions:

```bash
curl -s http://localhost:11434/api/generate -d '{
  "model": "deepseek-coder:1.3b",
  "prompt": "<｜fim▁begin｜>fn add(a: i32, b: i32) -> i32 {\n<｜fim▁hole｜>\n}<｜fim▁end｜>",
  "stream": true,
  "raw": true,
  "options": {
    "temperature": 0,
    "stop": ["}\n", "<｜fim▁end｜>", "<｜end▁of▁sentence｜>"]
  }
}'
```

You should see a stream of newline-delimited JSON, each with a `response` token, ending with `"done":true,"done_reason":"stop"`. If it ends with `done_reason: "length"`, your stop sequences aren't firing.

---

## 6. Configure Idep

Once the smoke test passes, set up your Idep config:

```bash
mkdir -p ~/.config/idep
cp config.example.toml ~/.config/idep/config.toml
```

Edit `~/.config/idep/config.toml`:

```toml
[ai]
backend  = "ollama"
model    = "deepseek-coder:1.3b"
endpoint = "http://localhost:11434"
```

---

## 7. Run the Idep Integration Test

```bash
# Fast mock-based test (CI-safe)
cargo test --test ollama_backend -- --nocapture

# Live test against real Ollama (requires Ollama running)
cargo test --test ollama_backend -- --nocapture --ignored
```

---

## FIM Token Reference

| Model family | `fim_begin` | `fim_hole` | `fim_end` | Stop sequences |
|---|---|---|---|---|
| DeepSeek Coder | `<｜fim▁begin｜>` | `<｜fim▁hole｜>` | `<｜fim▁end｜>` | `}\n`, `<｜fim▁end｜>`, `<｜end▁of▁sentence｜>` |
| CodeLlama | `<PRE>` | `<SUF>` | `<MID>` | `<EOT>` |
| StarCoder2 | `<fim_prefix>` | `<fim_suffix>` | `<fim_middle>` | `<|endoftext|>` |

---

## Troubleshooting

**Model responds in prose instead of code**
→ Add `"raw": true` to your request. Without it, Ollama wraps the prompt in a chat template that overrides FIM tokens.

**Completion doesn't stop at function boundary**
→ Check that model-specific stop sequences are being passed. See FIM Token Reference above.

**`ollama serve` port already in use**
→ Another Ollama instance is running. Kill it: `pkill ollama`

**WSL2: can't reach `localhost:11434` from Windows host**
→ Use `http://$(hostname -I | awk '{print $1}'):11434` as your endpoint.
