# AGENTS.md

## Cursor Cloud specific instructions

This is a Rust project (`sorcy`) that provides a small CLI for dependency source URL discovery.

### Environment

- **Rust 1.83.0** and **Cargo 1.83.0** are available system-wide (backward-compatible with the 1.82.0 edition used by the project).
- The project uses `Cargo.toml` as the dependency manifest.
- Dependency versions are pinned where needed to stay compatible with Rust 1.82.0.

### Running / Testing

- Build: `cargo build`
- Run CLI: `cargo run -- .`
- Run tests: `cargo test`
