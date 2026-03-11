# Development Guide

## Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. Hooks run:
- `cargo fmt` — code formatting
- `cargo clippy` — linting with warnings as errors
- `cargo test` — unit tests

### Setup

Install pre-commit:
```bash
pip install pre-commit
```

Install the git hooks:
```bash
pre-commit install
```

### Running Hooks Manually

Run all hooks on staged files:
```bash
pre-commit run --all-files
```

Run a specific hook:
```bash
pre-commit run cargo-fmt --all-files
pre-commit run cargo-clippy --all-files
pre-commit run cargo-test --all-files
```

### Skipping Hooks

To skip hooks on a commit (not recommended):
```bash
git commit --no-verify
```

## Building

```bash
cargo build --all
```

## Testing

```bash
cargo test --all
```

## Linting

```bash
cargo clippy --all --all-targets --all-features
```

## Formatting

```bash
cargo fmt --all
```
