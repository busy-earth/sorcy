# AGENTS.md

## Cursor Cloud specific instructions

This is a Rust project (`sorcy`) that provides a small CLI for dependency source URL discovery.

### Environment

- Rust and Cargo versions are managed by mise. Do not hardcode versions.
- To bootstrap the environment: `./bin/mise install`
- To run CI: `./bin/mise run ci`
- To update all versions (toolchain + crates): `./bin/mise run update`
- The project uses `Cargo.toml` as the dependency manifest.

### Running / Testing

- Build: `cargo build`
- Run CLI: `cargo run -- .`
- Run tests: `cargo test`
