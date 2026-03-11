# Security Policy

## Threat Model

Idep is designed with a local-first threat model:

- **Your code never leaves your machine** — the indexer runs in-process,
  embeddings are computed locally via `fastembed-rs`, vectors are stored
  in `~/.idep/index/`. No chunk of your codebase is sent to any server
  unless you explicitly configure a remote AI backend.

- **No telemetry** — Idep does not phone home. There is no analytics SDK,
  no crash reporter that sends data without your knowledge, no background process
  that checks in with idep.dev.

- **Backend traffic is explicit** — when you configure `backend = "anthropic"`,
  your prompts go to Anthropic's API over HTTPS. That's the only outbound
  traffic Idep initiates. You can inspect it with any network monitor.

- **Plugin sandbox** — plugins run inside a `wasmtime` WASM sandbox.
  They cannot make arbitrary syscalls or network requests without
  going through the plugin API, which you control.

## Reporting Vulnerabilities

Please do not report security vulnerabilities in public GitHub issues.

Email: security@idep.dev (monitored by the maintainer, @SHA888)

We aim to acknowledge reports within 48 hours and provide a fix timeline
within 7 days for critical issues.

## Supported Versions

Idep is in early development (pre-v1.0). Security fixes are applied to
`main` and the most recent release tag only.

| Version | Supported |
|---|---|
| `main` (dev) | ✅ |
| Latest release | ✅ |
| Older releases | ❌ |
