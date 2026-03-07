# sorcy

`sorcy` is a Rust CLI that scans a repository for dependency manifests and returns
a single JSON list of:

- dependency name
- source code URL

The MVP is intentionally small and focused.

## Install (quick)

`sorcy` now includes a simple installer flow similar to uv's one-command style,
adapted for this MVP.

macOS / Linux:

```bash
curl -LsSf https://raw.githubusercontent.com/busy-earth/sorcy/main/install.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://raw.githubusercontent.com/busy-earth/sorcy/main/install.ps1 | iex"
```

Notes:

- This installer uses `cargo install --git ...` under the hood.
- You need Rust/Cargo installed first.
- Optional env vars:
  - `SORCY_VERSION` (install a specific git tag)
  - `SORCY_REPO_URL` (install from a fork)

## MVP scope

The core has four parts:

1. **Scanner**: finds dependency files in the current repo.
2. **Parsers**: reads supported ecosystem formats.
3. **Normalizer**: converts everything into one record shape.
4. **Output**: returns final JSON list (`dependency`, `source_url`).

## Scalable architecture (uv-inspired, right-sized)

To keep growth clean for polyglot support, `sorcy` uses small module boundaries:

- `scan` finds files
- `parse` contains per-ecosystem parsers behind a shared parser trait
- `resolve` contains resolvers behind a shared resolver trait
- `lib` orchestrates scan → parse → resolve → output

This keeps new ecosystems additive: add parser/resolver implementations without
rewriting core orchestration.

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

Override network behavior from CLI:

```bash
cargo run -- . --http-timeout-seconds 20 --http-retries 5 --http-retry-backoff-ms 200
```

## Settings precedence (uv-style, small)

`sorcy` resolves settings in this order:

1. CLI arguments (highest)
2. Environment variables
3. Built-in defaults

Supported environment variables:

- `SORCY_PYPI_BASE_URL`
- `SORCY_NPM_BASE_URL`
- `SORCY_CRATES_BASE_URL`
- `SORCY_HTTP_TIMEOUT_SECONDS`
- `SORCY_HTTP_RETRIES`
- `SORCY_HTTP_RETRY_BACKOFF_MS`

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
