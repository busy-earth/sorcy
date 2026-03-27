# AGENTS.md

## Cursor Cloud specific instructions

This is a Rust project (`sorcy`) that provides a small CLI for dependency source URL discovery.

### Environment

- Rust and Cargo versions are managed by mise. Do not hardcode versions.
- To bootstrap the environment: `./bin/mise install`
- To run CI: `./bin/mise run ci`
- To update all versions (toolchain + crates): `./bin/mise run update`
- The project uses `Cargo.toml` as the dependency manifest.

### Gotchas

- On first use, mise requires trusting the config: `~/.local/bin/mise trust /workspace/mise.toml`
- After `./bin/mise install`, activate mise in the current shell so `cargo`/`rustc` are on PATH: `eval "$(/home/ubuntu/.local/bin/mise activate bash)"`
- No external services (databases, Docker, etc.) are required. All default tests use an in-process mock HTTP server.

### Running / Testing

- Build: `cargo build`
- Run CLI: `cargo run -- .`
- Run tests: `cargo test`
