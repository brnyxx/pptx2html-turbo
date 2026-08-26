# Contributing to pptx2html-turbo

Thank you for your interest in contributing! This guide will help you get started.

## Development Setup

```bash
# Clone the repository
git clone https://github.com/brnyxx/pptx2html-turbo.git
cd pptx2html-turbo

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace
```

### WASM Development

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM package
cd crates/pptx2html-wasm && wasm-pack build --target web

# Open the demo page
open demo/index.html
```

### Python Bindings

```bash
# Install maturin
pip install maturin

# Build and install in development mode
cd crates/pptx2html-py && maturin develop
```

## Making Changes

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-change`)
3. Make your changes
4. Run the full check suite:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

5. Commit with a conventional commit message:

```
feat: add support for X
fix: correct Y rendering
refactor: simplify Z pipeline
```

6. Submit a pull request

## Code Style

- Follow existing patterns in the codebase
- Use `log` crate for output, never `println!` in library code
- No `unsafe` or `unwrap()` in library code (ok in tests)
- Explicit imports only (no wildcard `use crate::*`)
- Early return pattern preferred
- All errors via `PptxError` enum with `thiserror`

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full pipeline diagram and module responsibilities.

### Adding Support for New PPTX Features

1. Find the ECMA-376 spec section for the element
2. Add model types in `crates/pptx2html-core/src/model/`
3. Add parser logic in `crates/pptx2html-core/src/parser/`
4. Add resolver logic if inheritance applies in `crates/pptx2html-core/src/resolver/`
5. Add HTML/CSS rendering in `crates/pptx2html-core/src/renderer/`
6. Add tests (unit + integration using `MinimalPptx` builder)

## Testing

- Unit tests go in source files (`#[cfg(test)]` module)
- Integration tests use the `MinimalPptx` builder to create valid PPTX ZIP archives in memory
- Run benchmarks with `cargo bench --package pptx2html-core`

## Reporting Issues

- Use the [issue tracker](https://github.com/brnyxx/pptx2html-turbo/issues)
- Include a minimal PPTX file that reproduces the issue if possible
- Describe expected vs actual output

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
