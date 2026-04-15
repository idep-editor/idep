# Contributing to Idep

Thanks for helping build Idep! Please follow these guidelines to keep the codebase healthy.

## Prerequisites

### System dependencies

**Ubuntu / Debian:**
```bash
sudo apt-get install libssl-dev pkg-config
```

**Fedora / RHEL:**
```bash
sudo dnf install openssl-devel pkgconfig
```

**macOS:**
```bash
# OpenSSL and pkg-config are usually pre-installed
# If not: brew install openssl pkg-config
```

### Rust toolchain
- Rust stable (pinned via `rust-toolchain.toml`)
- `cargo fmt`, `cargo clippy`, `cargo test`
- `pre-commit` installed (`pip install pre-commit`), then run `pre-commit install`

## Development Workflow
1. Format and lint:
   ```bash
   cargo fmt --all
   cargo clippy --all --all-targets --all-features -- -D warnings
   ```
2. Test:
   ```bash
   cargo test --all
   ```
3. Run pre-commit locally before pushing:
   ```bash
   pre-commit run --all-files
   ```
4. Keep changes focused and include tests when adding or changing behavior.

## Pull Requests
- Describe the change and its motivation; link related issues if any.
- Note any breaking changes or follow-up work.
- Ensure CI is green and pre-commit hooks pass.

## Reporting Issues
- Provide reproduction steps, expected vs actual behavior, logs/errors, and environment details.
