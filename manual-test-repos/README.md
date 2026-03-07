# Manual test repos for sorcy

These repos are intentionally tiny and mirror the integration-test style:
- create a sample dependency manifest
- run `sorcy`
- verify output JSON contains `dependency` + `source_url`

## Repos included

- `python-demo/pyproject.toml`
- `npm-demo/package.json`
- `cargo-demo/Cargo.toml`
- `cpp-demo/vcpkg-configuration.json`

## Quick run commands

From the project root (`/workspace`):

```bash
cargo run -- manual-test-repos/python-demo --pretty
cargo run -- manual-test-repos/npm-demo --pretty
cargo run -- manual-test-repos/cargo-demo --pretty
cargo run -- manual-test-repos/cpp-demo --pretty
```

## Where output appears

- By default: output is printed to your terminal (stdout).
- If you want a file: add `--output <path>`.

Example:

```bash
cargo run -- manual-test-repos/python-demo --pretty --output manual-test-repos/python-demo/output.json
```

Then open:

`manual-test-repos/python-demo/output.json`
