# sorcy

`sorcy` is a Rust CLI that scans a repository for dependency manifests and returns
a single JSON list of:

- dependency name
- source code URL

The MVP is intentionally small and focused.

## MVP scope

The core has four parts:

1. **Scanner**: finds dependency files in the current repo.
2. **Parsers**: reads supported ecosystem formats.
3. **Normalizer**: converts everything into one record shape.
4. **Output**: returns final JSON list (`dependency`, `source_url`).

## Supported manifests

`sorcy` currently scans:

- Python:
  - `pyproject.toml`
  - `requirements*.txt`
- npm:
  - `package.json`
- Cargo:
  - `Cargo.toml`
- C/C++:
  - `vcpkg.json`
  - `vcpkg-configuration.json`
  - `conanfile.txt`
  - `conanfile.py`

## How source URLs are resolved

- **Python**: reads dependency names from repo manifests and queries PyPI package metadata (`project_urls`, `home_page`, `project_url`).
- **npm**: reads dependency names from `package.json` and queries npm registry metadata (`repository`, `homepage`).
- **Cargo**: reads dependency names from `Cargo.toml` and queries crates.io metadata (`repository`, `homepage`).
- **C/C++**: reads repo-local metadata only. If forge references are directly present (for example in `vcpkg-configuration.json` registries), `sorcy` returns them. It does not clone repos, build graphs, or parse source code in this MVP.

## Build and run

```bash
cargo build
```

Scan current directory:

```bash
cargo run -- .
```

Pretty JSON:

```bash
cargo run -- . --pretty
```

Write to file:

```bash
cargo run -- . --output sorcy-sources.json --pretty
```

## Output format

The output is JSON:

```json
[
  {
    "dependency": "requests",
    "source_url": "https://github.com/psf/requests"
  }
]
```

Only dependencies with resolved source URLs are returned.

## Test

```bash
cargo test
```

The integration tests include a full loop against a temporary test repo and mocked registry metadata.
